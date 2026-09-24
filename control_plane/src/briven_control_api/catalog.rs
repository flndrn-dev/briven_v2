use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio_postgres::config::Host;
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
        validate_local_dsn(&dsn)?;
        let client = pg_client(&dsn).await?;
        client
            .batch_execute(
                "CREATE SCHEMA IF NOT EXISTS briven_control;
                 CREATE TABLE IF NOT EXISTS briven_control.projects (
                   id text PRIMARY KEY,
                   organization_id text NOT NULL,
                   name text NOT NULL,
                   main_branch_id text NOT NULL,
                   state text NOT NULL CHECK (state IN ('provisioning', 'ready', 'failed')),
                   UNIQUE (organization_id, name)
                 );",
            )
            .await?;
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

fn validate_local_dsn(dsn: &str) -> Result<()> {
    let config: tokio_postgres::Config = dsn.parse()?;
    for host in config.get_hosts() {
        match host {
            Host::Tcp(name) if name == "localhost" || name == "127.0.0.1" || name == "::1" => {}
            Host::Unix(_) => {}
            Host::Tcp(_) => {
                bail!("Briven control catalog must use local PostgreSQL until TLS is configured")
            }
        }
    }
    Ok(())
}

async fn pg_client(dsn: &str) -> Result<Client> {
    let (client, connection) = tokio_postgres::connect(dsn, NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("Briven catalog connection ended: {error}");
        }
    });
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::{Catalog, Project, ProjectState, Registry, validate_local_dsn};
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
    fn remote_plaintext_catalog_is_rejected() {
        assert!(validate_local_dsn("postgresql://localhost/briven").is_ok());
        assert!(validate_local_dsn("postgresql://db.example.com/briven").is_err());
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
