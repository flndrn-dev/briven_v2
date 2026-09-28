use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rustls::RootCertStore;
use serde::{Deserialize, Serialize};
use tokio_postgres::config::{Host, SslMode};
use tokio_postgres::{Client, NoTls, error::SqlState};

#[derive(Clone)]
pub enum Catalog {
    Local(PathBuf),
    Postgres(String),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub main_branch_id: String,
    #[serde(default = "local_organization")]
    pub organization_id: String,
    pub state: ProjectState,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    Provisioning,
    Ready,
    Failed,
}

impl ProjectState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "provisioning" => Ok(Self::Provisioning),
            "ready" => Ok(Self::Ready),
            "failed" => Ok(Self::Failed),
            _ => bail!("unknown project state in catalog"),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Registry {
    projects: Vec<Project>,
}

#[derive(Debug)]
pub enum CreateError {
    Duplicate,
    Other(anyhow::Error),
}

fn local_organization() -> String {
    "local".to_string()
}

impl Catalog {
    pub async fn from_env(repo: PathBuf) -> Result<Self> {
        let Some(dsn) = std::env::var("BRIVEN_CONTROL_DATABASE_URL").ok() else {
            return Ok(Self::Local(repo));
        };
        validate_catalog_dsn(&dsn)?;
        let mut client = pg_client(&dsn).await?;
        migrate(&mut client).await?;
        Ok(Self::Postgres(dsn))
    }

    pub async fn load(&self, organization: &str) -> Result<Vec<Project>> {
        match self {
            Self::Local(repo) => Ok(load_local(repo)?
                .projects
                .into_iter()
                .filter(|project| project.organization_id == organization)
                .collect()),
            Self::Postgres(dsn) => {
                let client = pg_client(dsn).await?;
                let rows = client
                    .query(
                        "SELECT id, name, main_branch_id, organization_id, state
                         FROM briven_control.projects WHERE organization_id = $1 ORDER BY name",
                        &[&organization],
                    )
                    .await?;
                rows.into_iter()
                    .map(|row| {
                        let state: String = row.get(4);
                        Ok(Project {
                            id: row.get(0),
                            name: row.get(1),
                            main_branch_id: row.get(2),
                            organization_id: row.get(3),
                            state: ProjectState::parse(&state)?,
                        })
                    })
                    .collect()
            }
        }
    }

    pub async fn insert(&self, project: &Project) -> std::result::Result<(), CreateError> {
        match self {
            Self::Local(repo) => {
                let mut registry = load_local(repo).map_err(CreateError::Other)?;
                if registry.projects.iter().any(|existing| {
                    existing.organization_id == project.organization_id
                        && existing.name == project.name
                }) {
                    return Err(CreateError::Duplicate);
                }
                registry.projects.push(project.clone());
                save_local(repo, &registry).map_err(CreateError::Other)
            }
            Self::Postgres(dsn) => {
                let client = pg_client(dsn).await.map_err(CreateError::Other)?;
                client
                    .execute(
                        "INSERT INTO briven_control.projects
                         (id, organization_id, name, main_branch_id, state)
                         VALUES ($1, $2, $3, $4, $5)",
                        &[
                            &project.id,
                            &project.organization_id,
                            &project.name,
                            &project.main_branch_id,
                            &project.state.as_str(),
                        ],
                    )
                    .await
                    .map(|_| ())
                    .map_err(|error| {
                        if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
                            CreateError::Duplicate
                        } else {
                            CreateError::Other(error.into())
                        }
                    })
            }
        }
    }

    pub async fn set_state(&self, organization: &str, id: &str, state: ProjectState) -> Result<()> {
        match self {
            Self::Local(repo) => {
                let mut registry = load_local(repo)?;
                let project = registry
                    .projects
                    .iter_mut()
                    .find(|project| project.organization_id == organization && project.id == id)
                    .context("project missing from local catalog")?;
                project.state = state;
                save_local(repo, &registry)
            }
            Self::Postgres(dsn) => {
                let client = pg_client(dsn).await?;
                let count = client
                    .execute(
                        "UPDATE briven_control.projects SET state = $3
                         WHERE organization_id = $1 AND id = $2",
                        &[&organization, &id, &state.as_str()],
                    )
                    .await?;
                if count != 1 {
                    bail!("project missing from PostgreSQL catalog");
                }
                Ok(())
            }
        }
    }
}

const MIGRATIONS: &[(i32, &str)] = &[(1, include_str!("migrations/0001_projects.sql"))];

async fn migrate(client: &mut Client) -> Result<()> {
    let transaction = client.transaction().await?;
    // Serialize startup migrations across API replicas using the same catalog.
    transaction
        .query_one("SELECT pg_advisory_xact_lock($1)", &[&0x42726976656e_i64])
        .await?;
    transaction
        .batch_execute(
            "CREATE SCHEMA IF NOT EXISTS briven_control;
             CREATE TABLE IF NOT EXISTS briven_control.schema_migrations (
               version integer PRIMARY KEY,
               applied_at timestamptz NOT NULL DEFAULT now()
             );",
        )
        .await?;
    let rows = transaction
        .query(
            "SELECT version FROM briven_control.schema_migrations ORDER BY version",
            &[],
        )
        .await?;
    let applied = rows
        .iter()
        .map(|row| row.get::<_, i32>(0))
        .collect::<std::collections::HashSet<_>>();
    if applied
        .iter()
        .any(|version| !MIGRATIONS.iter().any(|(known, _)| known == version))
    {
        bail!("control catalog has a migration newer than this API supports");
    }
    for (version, sql) in MIGRATIONS {
        if !applied.contains(version) {
            transaction.batch_execute(sql).await?;
            transaction
                .execute(
                    "INSERT INTO briven_control.schema_migrations (version) VALUES ($1)",
                    &[version],
                )
                .await?;
        }
    }
    transaction.commit().await?;
    Ok(())
}

fn registry_path(repo: &Path) -> PathBuf {
    repo.join("product-projects.json")
}

fn load_local(repo: &Path) -> Result<Registry> {
    let path = registry_path(repo);
    if !path.exists() {
        return Ok(Registry::default());
    }
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn save_local(repo: &Path, registry: &Registry) -> Result<()> {
    let path = registry_path(repo);
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, serde_json::to_vec_pretty(registry)?)?;
    std::fs::rename(temp, path)?;
    Ok(())
}

fn validate_catalog_dsn(dsn: &str) -> Result<()> {
    let config: tokio_postgres::Config = dsn.parse()?;
    if config.get_hosts().is_empty() {
        bail!("Briven control catalog DSN must include a PostgreSQL host");
    }
    Ok(())
}

async fn pg_client(dsn: &str) -> Result<Client> {
    let mut config: tokio_postgres::Config = dsn.parse()?;
    let remote = config.get_hosts().iter().any(|host| match host {
        Host::Tcp(name) => !is_loopback_host(name),
        Host::Unix(_) => false,
    });

    if remote {
        // Require TLS so tokio-postgres cannot silently fall back to a
        // plaintext connection. The Rustls connector validates the server
        // certificate chain and hostname against the system trust store.
        config.ssl_mode(SslMode::Require);
        let loaded = rustls_native_certs::load_native_certs();
        let mut roots = RootCertStore::empty();
        let (added, rejected) = roots.add_parsable_certificates(loaded.certs);
        if added == 0 {
            bail!(
                "system TLS trust store contains no usable certificates (rejected {rejected}; load errors: {:?})",
                loaded.errors
            );
        }
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
        let (client, connection) = config.connect(tls).await?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Briven catalog TLS connection ended: {error}");
            }
        });
        Ok(client)
    } else {
        let (client, connection) = config.connect(NoTls).await?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("Briven catalog connection ended: {error}");
            }
        });
        Ok(client)
    }
}

fn is_loopback_host(name: &str) -> bool {
    name.eq_ignore_ascii_case("localhost") || name == "127.0.0.1" || name == "::1"
}

#[cfg(test)]
mod tests {
    use super::{
        Catalog, Project, ProjectState, Registry, migrate, pg_client, validate_catalog_dsn,
    };
    use utils::id::TenantId;

    #[test]
    fn legacy_local_projects_belong_to_local_organization() {
        let registry: Registry = serde_json::from_str(
            r#"{"projects":[{"id":"a","name":"old","mainBranchId":"b","state":"ready"}]}"#,
        )
        .unwrap();
        assert_eq!(registry.projects[0].organization_id, "local");
        assert!(matches!(registry.projects[0].state, ProjectState::Ready));
    }

    #[test]
    fn hosted_catalog_dsn_is_accepted_for_verified_tls_connections() {
        assert!(validate_catalog_dsn("postgresql://localhost/briven").is_ok());
        assert!(validate_catalog_dsn("postgresql://db.example.com/briven").is_ok());
        assert!(validate_catalog_dsn("not a PostgreSQL connection string").is_err());
    }

    #[test]
    fn loopback_host_detection_does_not_classify_remote_hosts_as_local() {
        assert!(super::is_loopback_host("localhost"));
        assert!(super::is_loopback_host("127.0.0.1"));
        assert!(super::is_loopback_host("::1"));
        assert!(!super::is_loopback_host("db.example.com"));
    }

    #[tokio::test]
    async fn migrations_preserve_legacy_projects_and_are_idempotent() {
        let Ok(dsn) = std::env::var("BRIVEN_TEST_DISPOSABLE_CATALOG_URL") else {
            return;
        };
        let mut client = pg_client(&dsn).await.unwrap();
        client
            .batch_execute(
                "CREATE SCHEMA briven_control;
                 CREATE TABLE briven_control.projects (
                   id text PRIMARY KEY, organization_id text NOT NULL,
                   name text NOT NULL, main_branch_id text NOT NULL,
                   state text NOT NULL CHECK (state IN ('provisioning', 'ready', 'failed')),
                   UNIQUE (organization_id, name)
                 );
                 INSERT INTO briven_control.projects VALUES
                   ('old-id', 'org-a', 'old-project', 'main-id', 'ready');",
            )
            .await
            .unwrap();
        migrate(&mut client).await.unwrap();
        migrate(&mut client).await.unwrap();
        let count: i64 = client
            .query_one("SELECT count(*) FROM briven_control.projects", &[])
            .await
            .unwrap()
            .get(0);
        let versions: i64 = client
            .query_one("SELECT count(*) FROM briven_control.schema_migrations", &[])
            .await
            .unwrap()
            .get(0);
        assert_eq!(count, 1);
        assert_eq!(versions, 1);
    }

    #[tokio::test]
    async fn local_catalog_scopes_names_and_state_by_organization() {
        let directory =
            std::env::temp_dir().join(format!("briven-catalog-{}", TenantId::generate()));
        std::fs::create_dir(&directory).unwrap();
        let catalog = Catalog::Local(directory.clone());
        let alpha = Project {
            id: "alpha-id".to_string(),
            name: "same-name".to_string(),
            main_branch_id: "alpha-main".to_string(),
            organization_id: "alpha".to_string(),
            state: ProjectState::Provisioning,
        };
        let beta = Project {
            id: "beta-id".to_string(),
            main_branch_id: "beta-main".to_string(),
            organization_id: "beta".to_string(),
            ..alpha.clone()
        };
        catalog.insert(&alpha).await.unwrap();
        catalog.insert(&beta).await.unwrap();
        assert!(matches!(
            catalog.insert(&alpha).await,
            Err(super::CreateError::Duplicate)
        ));
        assert_eq!(catalog.load("alpha").await.unwrap().len(), 1);
        assert_eq!(catalog.load("beta").await.unwrap().len(), 1);
        assert!(catalog.load("other").await.unwrap().is_empty());
        catalog
            .set_state("alpha", &alpha.id, ProjectState::Ready)
            .await
            .unwrap();
        assert!(matches!(
            catalog.load("alpha").await.unwrap()[0].state,
            ProjectState::Ready
        ));
        assert!(matches!(
            catalog.load("beta").await.unwrap()[0].state,
            ProjectState::Provisioning
        ));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
