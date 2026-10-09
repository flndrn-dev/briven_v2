//! Independently versioned additive lifecycle state; old project catalogs stay readable.
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::catalog::{Catalog, pg_client};
use super::lifecycle::Record;

#[derive(Clone)]
pub(super) enum Store {
    Local(PathBuf),
    Postgres(String),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    version: u32,
    records: Vec<Record>,
}

impl Store {
    pub async fn open(catalog: &Catalog) -> Result<Self> {
        match catalog {
            Catalog::Local(repo) => Ok(Self::Local(repo.join("compute-lifecycle-v1.json"))),
            Catalog::Postgres(dsn) => {
                let mut client = pg_client(dsn).await?;
                let tx = client.transaction().await?;
                tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&0x427269764c6966_i64])
                    .await?;
                tx.batch_execute(
                    "CREATE SCHEMA IF NOT EXISTS briven_compute_lifecycle;
                    CREATE TABLE IF NOT EXISTS briven_compute_lifecycle.schema_migrations (
                    version integer PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now());",
                )
                .await?;
                let versions = tx
                    .query(
                        "SELECT version FROM briven_compute_lifecycle.schema_migrations",
                        &[],
                    )
                    .await?;
                if versions.iter().any(|row| row.get::<_, i32>(0) != 1) {
                    bail!("compute lifecycle schema is newer than this API supports");
                }
                if versions.is_empty() {
                    tx.batch_execute(
                        "CREATE TABLE briven_compute_lifecycle.endpoints (
                        endpoint_id text PRIMARY KEY CHECK (endpoint_id ~ '^ep-[a-z0-9-]{1,93}$'),
                        project_id text NOT NULL REFERENCES briven_control.projects(id),
                        timeline_id text NOT NULL CHECK (timeline_id ~ '^[0-9a-f]{32}$'),
                        generation bigint NOT NULL CHECK (generation >= 0),
                        record text NOT NULL CHECK (jsonb_typeof(record::jsonb) = 'object'),
                        updated_at timestamptz NOT NULL DEFAULT now());
                        INSERT INTO briven_compute_lifecycle.schema_migrations(version) VALUES(1);",
                    )
                    .await?;
                }
                tx.commit().await?;
                Ok(Self::Postgres(dsn.clone()))
            }
        }
    }

    pub async fn list(&self) -> Result<Vec<Record>> {
        let records = match self {
            Self::Local(path) => {
                if !path.exists() {
                    return Ok(Vec::new());
                }
                let registry: Registry = serde_json::from_slice(&std::fs::read(path)?)?;
                if registry.version != 1 {
                    bail!("unsupported compute lifecycle version");
                }
                registry.records
            }
            Self::Postgres(dsn) => {
                let client = pg_client(dsn).await?;
                let rows = client
                    .query(
                        "SELECT endpoint_id, project_id, timeline_id, generation, record
                    FROM briven_compute_lifecycle.endpoints ORDER BY endpoint_id LIMIT 4097",
                        &[],
                    )
                    .await?;
                let mut records = Vec::with_capacity(rows.len());
                for row in rows {
                    let record: Record = serde_json::from_str(row.get::<_, &str>(4))?;
                    if !record.matches(row.get(0), row.get(1), row.get(2))
                        || record.generation != row.get::<_, i64>(3) as u64
                    {
                        bail!("compute lifecycle storage binding changed");
                    }
                    records.push(record);
                }
                records
            }
        };
        if records.len() > 4096 {
            bail!("compute lifecycle inventory exceeds supported limit");
        }
        let mut seen = std::collections::HashSet::new();
        for record in &records {
            record.validate()?;
            if !seen.insert(&record.endpoint_id) {
                bail!("duplicate lifecycle endpoint");
            }
        }
        Ok(records)
    }

    pub async fn get(&self, endpoint: &str) -> Result<Option<Record>> {
        if let Self::Postgres(dsn) = self {
            let client = pg_client(dsn).await?;
            let Some(row) = client
                .query_opt(
                    "SELECT endpoint_id, project_id, timeline_id, generation, record
                 FROM briven_compute_lifecycle.endpoints WHERE endpoint_id=$1",
                    &[&endpoint],
                )
                .await?
            else {
                return Ok(None);
            };
            let record: Record = serde_json::from_str(row.get::<_, &str>(4))?;
            record.validate()?;
            if !record.matches(row.get(0), row.get(1), row.get(2))
                || record.generation != row.get::<_, i64>(3) as u64
            {
                bail!("compute lifecycle storage binding changed");
            }
            return Ok(Some(record));
        }
        Ok(self
            .list()
            .await?
            .into_iter()
            .find(|record| record.endpoint_id == endpoint))
    }

    /// Initial insert uses None; every later update must own the exact generation.
    /// Return false for a stale claimant without modifying either record.
    pub async fn save(&self, record: &Record, previous_generation: Option<u64>) -> Result<bool> {
        record.validate()?;
        let expected = match previous_generation {
            Some(previous) => previous
                .checked_add(1)
                .context("lifecycle generation overflow")?,
            None => 0,
        };
        if record.generation != expected {
            bail!("invalid lifecycle generation transition");
        }
        match self {
            Self::Local(path) => {
                let mut records = self.list().await?;
                let index = records
                    .iter()
                    .position(|value| value.endpoint_id == record.endpoint_id);
                match (index, previous_generation) {
                    (None, None) if records.len() < 4096 => records.push(record.clone()),
                    (Some(index), Some(previous))
                        if records[index].generation == previous
                            && records[index].matches(
                                &record.endpoint_id,
                                &record.project_id,
                                &record.timeline_id,
                            ) =>
                    {
                        records[index] = record.clone()
                    }
                    _ => return Ok(false),
                }
                let temporary = path.with_extension("json.tmp");
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create(true).truncate(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let mut file = options.open(&temporary)?;
                file.write_all(&serde_json::to_vec(&Registry {
                    version: 1,
                    records,
                })?)?;
                file.sync_all()?;
                std::fs::rename(&temporary, path)?;
                std::fs::File::open(path.parent().context("lifecycle parent missing")?)?
                    .sync_all()?;
                Ok(true)
            }
            Self::Postgres(dsn) => {
                let client = pg_client(dsn).await?;
                let generation = record.generation as i64;
                let body = serde_json::to_string(record)?;
                let count = if let Some(previous) = previous_generation {
                    client.execute("UPDATE briven_compute_lifecycle.endpoints
                        SET generation=$4, record=$5, updated_at=now()
                        WHERE endpoint_id=$1 AND project_id=$2 AND timeline_id=$3 AND generation=$6",
                        &[&record.endpoint_id, &record.project_id, &record.timeline_id, &generation, &body,
                          &(previous as i64)]).await?
                } else {
                    client.execute("INSERT INTO briven_compute_lifecycle.endpoints
                        (endpoint_id,project_id,timeline_id,generation,record) VALUES($1,$2,$3,$4,$5)
                        ON CONFLICT(endpoint_id) DO NOTHING",
                        &[&record.endpoint_id, &record.project_id, &record.timeline_id, &generation, &body]).await?
                };
                Ok(count == 1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn postgres_claim_is_atomic_reopen_retains_sleep_and_old_catalog_stays_readable() {
        let Ok(dsn) = std::env::var("BRIVEN_TEST_DISPOSABLE_LIFECYCLE_URL") else {
            return;
        };
        let parsed: tokio_postgres::Config = dsn.parse().unwrap();
        assert!(
            parsed
                .get_dbname()
                .unwrap()
                .starts_with("briven_lifecycle_fixture_")
        );
        let mut client = pg_client(&dsn).await.unwrap();
        super::super::catalog::migrate(&mut client).await.unwrap();
        let catalog = Catalog::Postgres(dsn.clone());
        let project = super::super::catalog::Project {
            id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            name: "lifecycle-fixture".into(),
            organization_id: "org_lifecycle_fixture".into(),
            main_branch_id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            state: super::super::catalog::ProjectState::Ready,
        };
        catalog.insert(&project).await.unwrap();
        let store = Store::open(&catalog).await.unwrap();
        let mut record = Record::new(
            "ep-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &project.id,
            &project.main_branch_id,
            60,
            1000,
        )
        .unwrap();
        assert!(store.save(&record, None).await.unwrap());
        record.generation = 1;
        record.phase = super::super::lifecycle::Phase::Sleeping;
        let (a, b) = tokio::join!(store.save(&record, Some(0)), store.save(&record, Some(0)));
        assert_ne!(a.unwrap(), b.unwrap());
        let reopened = Store::open(&catalog).await.unwrap();
        let saved = reopened.get(&record.endpoint_id).await.unwrap().unwrap();
        assert_eq!(saved.phase, super::super::lifecycle::Phase::Sleeping);
        assert_eq!(saved.generation, 1);
        record.generation = 2;
        record.timeline_id = "cccccccccccccccccccccccccccccccc".into();
        assert!(!reopened.save(&record, Some(1)).await.unwrap());
        // The unchanged older project migrator ignores the independently
        // versioned additive lifecycle schema and still sees the same project.
        super::super::catalog::migrate(&mut client).await.unwrap();
        assert_eq!(
            catalog.load("org_lifecycle_fixture").await.unwrap().len(),
            1
        );
        let versions: Vec<i32> = client
            .query("SELECT version FROM briven_control.schema_migrations", &[])
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get(0))
            .collect();
        assert_eq!(versions, vec![1]);
        let records: i64 = client
            .query_one(
                "SELECT count(*) FROM briven_compute_lifecycle.endpoints",
                &[],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(records, 1);
    }

    #[tokio::test]
    async fn local_reopen_retains_phase_and_stale_claimant_cannot_finish_or_rebind() {
        let directory = std::env::temp_dir().join(format!(
            "briven-lifecycle-{}",
            utils::id::TenantId::generate()
        ));
        std::fs::create_dir(&directory).unwrap();
        let store = Store::open(&Catalog::Local(directory.clone()))
            .await
            .unwrap();
        let mut record = Record::new(
            "ep-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            300,
            1000,
        )
        .unwrap();
        assert!(store.save(&record, None).await.unwrap());
        assert!(!store.save(&record, None).await.unwrap());
        record.generation = 1;
        record.phase = super::super::lifecycle::Phase::Stopping;
        assert!(store.save(&record, Some(0)).await.unwrap());
        let reopened = Store::open(&Catalog::Local(directory.clone()))
            .await
            .unwrap();
        assert_eq!(
            reopened
                .get(&record.endpoint_id)
                .await
                .unwrap()
                .unwrap()
                .phase,
            record.phase
        );
        assert!(!reopened.save(&record, Some(0)).await.unwrap());
        record.generation = 2;
        record.project_id = "cccccccccccccccccccccccccccccccc".into();
        assert!(!reopened.save(&record, Some(1)).await.unwrap());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
