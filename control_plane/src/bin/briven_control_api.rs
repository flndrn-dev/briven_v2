//! Briven product API for a single local engine environment.
//! This adapter uses signed customer identity from the website API.
//! Hosted operation still needs secure database roles and a compute scheduler.

use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use axum::extract::{Path as RoutePath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use control_plane::endpoint::{ComputeControlPlane, EndpointStatus};
use control_plane::local_env::{LocalEnv, base_path};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_postgres::NoTls;
use utils::id::{TenantId, TimelineId};

#[path = "../briven_control_api/auth.rs"]
mod auth;
#[path = "../briven_control_api/catalog.rs"]
mod catalog;
#[path = "../briven_control_api/customer_credential.rs"]
mod customer_credential;
#[path = "../briven_control_api/proxy_control.rs"]
mod proxy_control;

use auth::{AuthConfig, Principal};
use catalog::{Catalog, CreateError, Project, ProjectState};

#[derive(Clone)]
struct AppState {
    repo: PathBuf,
    cli: PathBuf,
    auth: Arc<AuthConfig>,
    proxy_auth: Arc<proxy_control::ProxyAuth>,
    catalog: Catalog,
    gate: Arc<Mutex<()>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateProject {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateBranch {
    name: String,
    parent: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Branch {
    name: String,
    id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Compute {
    id: String,
    branch: String,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Connection {
    uri: String,
    environment: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
}

#[derive(Serialize)]
struct ApiErrorBody {
    error: &'static str,
}

struct ApiError(StatusCode, &'static str);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ApiErrorBody { error: self.1 })).into_response()
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

fn internal(error: impl std::fmt::Display) -> ApiError {
    eprintln!("Briven control API: {error}");
    ApiError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "control API operation failed",
    )
}

fn engine(error: impl std::fmt::Display) -> ApiError {
    eprintln!("Briven engine: {error}");
    ApiError(
        StatusCode::BAD_GATEWAY,
        "local database engine operation failed",
    )
}

fn authorize(headers: &HeaderMap, auth: &AuthConfig, write: bool) -> ApiResult<Principal> {
    let principal = authenticated(headers, auth)?;
    if !principal.is_customer() || (write && !principal.can_write()) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "insufficient organization role",
        ));
    }
    Ok(principal)
}

fn authenticated(headers: &HeaderMap, auth: &AuthConfig) -> ApiResult<Principal> {
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let principal = supplied
        .and_then(|token| auth.principal_for(token))
        .ok_or(ApiError(
            StatusCode::UNAUTHORIZED,
            "authentication required",
        ))?;
    Ok(principal)
}

fn valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=48).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        && bytes[bytes.len() - 1] != b'-'
}

fn project<'a>(projects: &'a [Project], id: &str) -> ApiResult<&'a Project> {
    projects
        .iter()
        .find(|project| project.id == id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "project not found"))
}

async fn run_cli(state: &AppState, args: &[&str]) -> ApiResult<()> {
    let output = Command::new(&state.cli)
        .kill_on_drop(true)
        .args(args)
        .env("BRIVEN_REPO_DIR", &state.repo)
        .output()
        .await
        .map_err(engine)?;
    if !output.status.success() {
        return Err(engine(String::from_utf8_lossy(&output.stderr)));
    }
    Ok(())
}

async fn create_endpoint(
    state: &AppState,
    tenant_id: &str,
    branch: &str,
    endpoint_id: &str,
) -> ApiResult<()> {
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let plane = ComputeControlPlane::load(env).map_err(engine)?;
    let occupied = plane
        .endpoints
        .values()
        .flat_map(|endpoint| {
            [
                endpoint.pg_address.port(),
                endpoint.external_http_address.port(),
                endpoint.internal_http_address.port(),
            ]
        })
        .collect::<std::collections::HashSet<_>>();
    for pg_port in (55440u16..60000).step_by(3) {
        let external_port = pg_port + 1;
        let internal_port = pg_port + 2;
        if [pg_port, external_port, internal_port]
            .iter()
            .any(|port| occupied.contains(port))
        {
            continue;
        }
        if let (Ok(_pg), Ok(_external), Ok(_internal)) = (
            TcpListener::bind(("127.0.0.1", pg_port)),
            TcpListener::bind(("0.0.0.0", external_port)),
            TcpListener::bind(("127.0.0.1", internal_port)),
        ) {
            return run_cli(
                state,
                &[
                    "endpoint",
                    "create",
                    endpoint_id,
                    "--tenant-id",
                    tenant_id,
                    "--branch-name",
                    branch,
                    "--pg-port",
                    &pg_port.to_string(),
                    "--external-http-port",
                    &external_port.to_string(),
                    "--internal-http-port",
                    &internal_port.to_string(),
                ],
            )
            .await;
        }
    }
    Err(ApiError(
        StatusCode::SERVICE_UNAVAILABLE,
        "no compute ports available",
    ))
}

async fn can_query(uri: &str) -> bool {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (client, connection) =
            tokio_postgres::connect(&format!("{uri}?sslmode=disable"), NoTls).await?;
        let task = tokio::spawn(async move {
            let _ = connection.await;
        });
        let result = client.simple_query("SELECT 1").await;
        task.abort();
        result.map(|_| ())
    })
    .await
    .is_ok_and(|result| result.is_ok())
}

async fn enable_vector(uri: &str) -> ApiResult<()> {
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let (client, connection) =
            tokio_postgres::connect(&format!("{uri}?sslmode=disable"), NoTls).await?;
        let task = tokio::spawn(async move {
            let _ = connection.await;
        });
        let result = client
            .batch_execute("CREATE EXTENSION IF NOT EXISTS vector; SELECT '[1,2,3]'::vector(3);")
            .await;
        task.abort();
        result
    })
    .await;
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(engine(error)),
        Err(error) => Err(engine(error)),
    }
}

async fn apply_customer_role(admin_uri: &str, statements: &[String]) -> ApiResult<()> {
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let (client, connection) =
            tokio_postgres::connect(&format!("{admin_uri}?sslmode=disable"), NoTls).await?;
        let task = tokio::spawn(async move {
            let _ = connection.await;
        });
        // Apply role rotation and metadata grants atomically. In particular,
        // untrusted roles never see the temporary grant of metadata tables.
        let outcome = client
            .simple_query(&format!("BEGIN; {}; COMMIT;", statements.join("; ")))
            .await
            .map(|_| ());
        task.abort();
        outcome
    })
    .await;
    match result {
        Ok(Ok(())) => Ok(()),
        _ => {
            eprintln!("Briven control API: customer role apply failed");
            Err(ApiError(
                StatusCode::BAD_GATEWAY,
                "local database engine operation failed",
            ))
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Project>>> {
    let principal = authorize(&headers, &state.auth, false)?;
    let organization = principal.organization.as_str();
    let _guard = state.gate.lock().await;
    Ok(Json(
        state.catalog.load(organization).await.map_err(internal)?,
    ))
}

#[derive(Debug, PartialEq, Eq)]
enum ExistingName {
    Create,
    Resume,
    Conflict,
}

/// A repeated create for the same organization and name must keep one tenant.
/// A finished project stays taken. An interrupted or failed create is resumed.
fn existing_name(state: Option<ProjectState>) -> ExistingName {
    match state {
        None => ExistingName::Create,
        Some(ProjectState::Ready) => ExistingName::Conflict,
        Some(ProjectState::Provisioning | ProjectState::Failed) => ExistingName::Resume,
    }
}

fn load_plane(state: &AppState) -> ApiResult<ComputeControlPlane> {
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    ComputeControlPlane::load(env).map_err(engine)
}

async fn ensure_main_compute(
    state: &AppState,
    tenant_id: &str,
    timeline_id: &str,
) -> ApiResult<()> {
    // Tenant and timeline creation already accept the same ids again.
    run_cli(
        state,
        &[
            "tenant",
            "create",
            "--tenant-id",
            tenant_id,
            "--timeline-id",
            timeline_id,
        ],
    )
    .await?;
    let endpoint_id = format!("ep-{tenant_id}");
    let mut plane = load_plane(state)?;
    if !plane.endpoints.contains_key(&endpoint_id) {
        create_endpoint(state, tenant_id, "main", &endpoint_id).await?;
        plane = load_plane(state)?;
    }
    let already_running = {
        let endpoint = plane.endpoints.get(&endpoint_id).ok_or(ApiError(
            StatusCode::BAD_GATEWAY,
            "created compute is unavailable",
        ))?;
        // Starting a compute that is already running is an error, so only start
        // one that stopped or never finished starting.
        endpoint.status() == EndpointStatus::Running
    };
    if !already_running {
        run_cli(state, &["endpoint", "start", &endpoint_id]).await?;
        plane = load_plane(state)?;
    }
    let endpoint = plane.endpoints.get(&endpoint_id).ok_or(ApiError(
        StatusCode::BAD_GATEWAY,
        "created compute is unavailable",
    ))?;
    let uri = endpoint.connstr("cloud_admin", "postgres");
    if !can_query(&uri).await {
        return Err(ApiError(
            StatusCode::BAD_GATEWAY,
            "created compute is unavailable",
        ));
    }
    if state.auth.uses_customer_identity() {
        enable_vector(&uri).await?;
    }
    Ok(())
}

async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateProject>,
) -> ApiResult<(StatusCode, Json<Project>)> {
    let principal = authorize(&headers, &state.auth, true)?;
    let organization = principal.organization.as_str();
    if !valid_name(&input.name) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid project name"));
    }
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    let existing = projects.iter().find(|project| project.name == input.name);
    let mut resumed = false;
    let mut created = if let Some(existing) = existing {
        match existing_name(Some(existing.state)) {
            ExistingName::Conflict => {
                return Err(ApiError(
                    StatusCode::CONFLICT,
                    "project name already exists",
                ));
            }
            ExistingName::Resume => {
                resumed = true;
                existing.clone()
            }
            ExistingName::Create => {
                unreachable!("a saved project is either ready or still unfinished")
            }
        }
    } else {
        let tenant_id = TenantId::generate().to_string();
        let timeline_id = TimelineId::generate().to_string();
        let created = Project {
            id: tenant_id,
            organization_id: organization.to_string(),
            name: input.name,
            main_branch_id: timeline_id,
            state: ProjectState::Provisioning,
        };
        match state.catalog.insert(&created).await {
            Ok(()) => {}
            Err(CreateError::Duplicate) => {
                return Err(ApiError(
                    StatusCode::CONFLICT,
                    "project name already exists",
                ));
            }
            Err(CreateError::Other(error)) => return Err(internal(error)),
        }
        created
    };
    if resumed && !matches!(created.state, ProjectState::Provisioning) {
        created.state = ProjectState::Provisioning;
        state
            .catalog
            .set_state(organization, &created.id, created.state)
            .await
            .map_err(internal)?;
    }

    let tenant_id = created.id.clone();
    let timeline_id = created.main_branch_id.clone();
    let result = ensure_main_compute(&state, &tenant_id, &timeline_id).await;
    created.state = if result.is_ok() {
        ProjectState::Ready
    } else {
        ProjectState::Failed
    };
    state
        .catalog
        .set_state(organization, &tenant_id, created.state)
        .await
        .map_err(internal)?;
    result?;
    let status = if resumed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(created)))
}

async fn get_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath(id): RoutePath<String>,
) -> ApiResult<Json<Project>> {
    let principal = authorize(&headers, &state.auth, false)?;
    let organization = principal.organization.as_str();
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    Ok(Json(project(&projects, &id)?.clone()))
}

async fn list_branches(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath(id): RoutePath<String>,
) -> ApiResult<Json<Vec<Branch>>> {
    let principal = authorize(&headers, &state.auth, false)?;
    let organization = principal.organization.as_str();
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    project(&projects, &id)?;
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let tenant_id = id.parse::<TenantId>().map_err(internal)?;
    let mut branches = env
        .timeline_name_mappings()
        .into_iter()
        .filter(|(key, _)| key.tenant_id == tenant_id)
        .map(|(key, name)| Branch {
            name,
            id: key.timeline_id.to_string(),
        })
        .collect::<Vec<_>>();
    branches.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(branches))
}

async fn create_branch(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath(id): RoutePath<String>,
    Json(input): Json<CreateBranch>,
) -> ApiResult<(StatusCode, Json<Branch>)> {
    let principal = authorize(&headers, &state.auth, true)?;
    let organization = principal.organization.as_str();
    if !valid_name(&input.name) || !valid_name(input.parent.as_deref().unwrap_or("main")) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid branch name"));
    }
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    let existing = project(&projects, &id)?;
    if !matches!(existing.state, ProjectState::Ready) {
        return Err(ApiError(StatusCode::CONFLICT, "project is not ready"));
    }
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let tenant_id = id.parse::<TenantId>().map_err(internal)?;
    if env.get_branch_timeline_id(&input.name, tenant_id).is_some() {
        return Err(ApiError(StatusCode::CONFLICT, "branch name already exists"));
    }
    let parent = input.parent.as_deref().unwrap_or("main");
    if env.get_branch_timeline_id(parent, tenant_id).is_none() {
        return Err(ApiError(StatusCode::NOT_FOUND, "parent branch not found"));
    }
    let timeline_id = TimelineId::generate().to_string();
    run_cli(
        &state,
        &[
            "timeline",
            "branch",
            "--tenant-id",
            &id,
            "--timeline-id",
            &timeline_id,
            "--branch-name",
            &input.name,
            "--ancestor-branch-name",
            parent,
        ],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Branch {
            name: input.name,
            id: timeline_id,
        }),
    ))
}

async fn list_computes(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath(id): RoutePath<String>,
) -> ApiResult<Json<Vec<Compute>>> {
    let principal = authorize(&headers, &state.auth, false)?;
    let organization = principal.organization.as_str();
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    project(&projects, &id)?;
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let names = env.timeline_name_mappings();
    let plane = ComputeControlPlane::load(env).map_err(engine)?;
    let mut computes = Vec::new();
    for (endpoint_id, endpoint) in &plane.endpoints {
        if endpoint.tenant_id.to_string() != id {
            continue;
        }
        let status = endpoint.status();
        let status = if status == EndpointStatus::Running
            && !can_query(&endpoint.connstr("cloud_admin", "postgres")).await
        {
            "unavailable".to_string()
        } else {
            status.to_string()
        };
        computes.push(Compute {
            id: endpoint_id.clone(),
            branch: names
                .iter()
                .find(|(key, _)| {
                    key.tenant_id == endpoint.tenant_id && key.timeline_id == endpoint.timeline_id
                })
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            status,
        });
    }
    computes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Json(computes))
}

async fn create_compute(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath((id, branch)): RoutePath<(String, String)>,
) -> ApiResult<(StatusCode, Json<Compute>)> {
    let principal = authorize(&headers, &state.auth, true)?;
    let organization = principal.organization.as_str();
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    let existing = project(&projects, &id)?;
    if !matches!(existing.state, ProjectState::Ready) {
        return Err(ApiError(StatusCode::CONFLICT, "project is not ready"));
    }
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let tenant_id = id.parse::<TenantId>().map_err(internal)?;
    let timeline_id = env
        .get_branch_timeline_id(&branch, tenant_id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "branch not found"))?;
    let plane = ComputeControlPlane::load(env).map_err(engine)?;
    if plane
        .endpoints
        .values()
        .any(|endpoint| endpoint.tenant_id == tenant_id && endpoint.timeline_id == timeline_id)
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "branch compute already exists",
        ));
    }
    let endpoint_id = format!("ep-{id}-{branch}");
    create_endpoint(&state, &id, &branch, &endpoint_id).await?;
    run_cli(&state, &["endpoint", "start", &endpoint_id]).await?;
    let plane = ComputeControlPlane::load(LocalEnv::load_config(&state.repo).map_err(engine)?)
        .map_err(engine)?;
    let endpoint = plane.endpoints.get(&endpoint_id).ok_or(ApiError(
        StatusCode::BAD_GATEWAY,
        "created compute is unavailable",
    ))?;
    if !can_query(&endpoint.connstr("cloud_admin", "postgres")).await {
        return Err(ApiError(
            StatusCode::BAD_GATEWAY,
            "created compute is unavailable",
        ));
    }
    if state.auth.uses_customer_identity() {
        enable_vector(&endpoint.connstr("cloud_admin", "postgres")).await?;
    }
    Ok((
        StatusCode::CREATED,
        Json(Compute {
            id: endpoint_id,
            branch,
            status: "running".to_string(),
        }),
    ))
}

async fn get_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath((id, branch)): RoutePath<(String, String)>,
) -> ApiResult<Json<Connection>> {
    issue_connection(state, headers, id, branch, None).await
}

async fn get_service_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath((id, branch)): RoutePath<(String, String)>,
) -> ApiResult<Json<Connection>> {
    if !state.auth.uses_customer_identity() {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "signed service identity required",
        ));
    }
    let principal = authenticated(&headers, &state.auth)?;
    let kind = principal.service_for(&id).ok_or(ApiError(
        StatusCode::FORBIDDEN,
        "project service scope required",
    ))?;
    issue_connection(state, headers, id, branch, Some(kind)).await
}

async fn issue_connection(
    state: AppState,
    headers: HeaderMap,
    id: String,
    branch: String,
    service_kind: Option<customer_credential::ServiceKind>,
) -> ApiResult<Json<Connection>> {
    if state.auth.uses_customer_identity() {
        let principal = if let Some(kind) = service_kind {
            let principal = authenticated(&headers, &state.auth)?;
            if principal.service_for(&id) != Some(kind) {
                return Err(ApiError(
                    StatusCode::FORBIDDEN,
                    "project service scope required",
                ));
            }
            principal
        } else {
            authorize(&headers, &state.auth, true)?
        };
        let host = std::env::var("BRIVEN_CUSTOMER_PROXY_HOST").unwrap_or_default();
        let port = std::env::var("BRIVEN_CUSTOMER_PROXY_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(5432);
        let proxy_host_set = customer_credential::valid_proxy_host(&host) && port != 0;
        match customer_credential::connection_gate(
            true,
            principal.can_write(),
            proxy_host_set,
            true,
        ) {
            customer_credential::ConnectionGate::Forbidden => {
                return Err(ApiError(
                    StatusCode::FORBIDDEN,
                    "insufficient organization role",
                ));
            }
            customer_credential::ConnectionGate::Unavailable => {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "customer database credentials are not available yet",
                ));
            }
            customer_credential::ConnectionGate::NotReady => {
                return Err(ApiError(StatusCode::CONFLICT, "project is not ready"));
            }
            customer_credential::ConnectionGate::Issue => {}
        }
        let organization = principal.organization.as_str();
        let _guard = state.gate.lock().await;
        let projects = state.catalog.load(organization).await.map_err(internal)?;
        let found = project(&projects, &id)?;
        match customer_credential::connection_gate(
            true,
            principal.can_write(),
            proxy_host_set,
            matches!(found.state, ProjectState::Ready),
        ) {
            customer_credential::ConnectionGate::NotReady => {
                return Err(ApiError(StatusCode::CONFLICT, "project is not ready"));
            }
            customer_credential::ConnectionGate::Issue => {}
            customer_credential::ConnectionGate::Forbidden
            | customer_credential::ConnectionGate::Unavailable => {
                return Err(ApiError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "control API operation failed",
                ));
            }
        }
        let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
        let tenant_id = id.parse::<TenantId>().map_err(internal)?;
        let timeline_id = env
            .get_branch_timeline_id(&branch, tenant_id)
            .ok_or(ApiError(StatusCode::NOT_FOUND, "branch not found"))?;
        let plane = ComputeControlPlane::load(env).map_err(engine)?;
        let endpoint = plane
            .endpoints
            .iter()
            .find(|(_, endpoint)| {
                endpoint.tenant_id == tenant_id && endpoint.timeline_id == timeline_id
            })
            .ok_or(ApiError(
                StatusCode::CONFLICT,
                "branch has no running compute",
            ))?;
        let (endpoint_id, endpoint) = endpoint;
        let endpoint = if endpoint.status() != EndpointStatus::Running {
            proxy_control::endpoint(&state, endpoint_id).await?
        } else {
            Arc::clone(endpoint)
        };
        let admin_uri = endpoint.connstr("cloud_admin", "postgres");
        if !can_query(&admin_uri).await {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "branch compute is unavailable",
            ));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(internal)?
            .as_secs();
        let password = customer_credential::random_customer_password();
        let credential = match service_kind {
            Some(kind) => {
                customer_credential::issue_service_credential(&id, kind, "postgres", &password, now)
            }
            None => customer_credential::issue_customer_credential(&id, "postgres", &password, now),
        }
        .ok_or(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "control API operation failed",
        ))?;
        if !customer_credential::credential_opens_project(&credential.role, &id) {
            return Err(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "control API operation failed",
            ));
        }
        let statements =
            customer_credential::customer_role_statements(&credential).ok_or(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "control API operation failed",
            ))?;
        apply_customer_role(&admin_uri, &statements).await?;
        let uri = customer_credential::customer_uri(
            &credential.role,
            &credential.password,
            &host,
            port,
            "postgres",
        )
        .and_then(|uri| customer_credential::with_endpoint_hint(&uri, endpoint_id))
        .filter(|uri| customer_credential::publishable_customer_uri(uri))
        .ok_or(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "control API operation failed",
        ))?;
        return Ok(Json(Connection {
            uri,
            environment: if service_kind.is_some() {
                "service"
            } else {
                "customer"
            },
            expires_at: Some(credential.valid_until),
        }));
    }

    let principal = authorize(&headers, &state.auth, false)?;
    let organization = principal.organization.as_str();
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    project(&projects, &id)?;
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let tenant_id = id.parse::<TenantId>().map_err(internal)?;
    let timeline_id = env
        .get_branch_timeline_id(&branch, tenant_id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "branch not found"))?;
    let plane = ComputeControlPlane::load(env).map_err(engine)?;
    let endpoint = plane
        .endpoints
        .values()
        .find(|endpoint| {
            endpoint.tenant_id == tenant_id
                && endpoint.timeline_id == timeline_id
                && endpoint.status() == EndpointStatus::Running
        })
        .ok_or(ApiError(
            StatusCode::CONFLICT,
            "branch has no running compute",
        ))?;
    if !can_query(&endpoint.connstr("cloud_admin", "postgres")).await {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "branch compute is unavailable",
        ));
    }
    Ok(Json(Connection {
        uri: endpoint.connstr("cloud_admin", "postgres"),
        environment: "local",
        expires_at: None,
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    let auth = Arc::new(AuthConfig::from_env()?);
    if !auth.uses_customer_identity() && std::env::var("BRIVEN_ENV").as_deref() != Ok("development")
    {
        bail!("static control keys are development-only; configure BRIVEN_CONTROL_IDENTITY_SECRET");
    }
    let repo = base_path();
    LocalEnv::load_config(&repo).context("initialize the local engine with briven_local init")?;
    let cli = std::env::var_os("BRIVEN_LOCAL_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .with_file_name("briven_local")
        });
    if !cli.is_file() {
        bail!("briven_local binary not found; build it or set BRIVEN_LOCAL_BIN");
    }
    let address: SocketAddr = std::env::var("BRIVEN_CONTROL_API_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()
        .context("invalid BRIVEN_CONTROL_API_BIND")?;
    if !address.ip().is_loopback() && !auth.uses_customer_identity() {
        bail!("non-loopback control API requires signed customer identity");
    }
    let catalog = Catalog::from_env(repo.clone()).await?;
    if auth.uses_customer_identity() && matches!(catalog, Catalog::Local(_)) {
        bail!("signed customer mode requires BRIVEN_CONTROL_DATABASE_URL");
    }
    let state = AppState {
        repo,
        cli,
        auth,
        proxy_auth: Arc::new(proxy_control::ProxyAuth::from_env()?),
        catalog,
        gate: Arc::new(Mutex::new(())),
    };
    let app = Router::new()
        .route("/healthz", get(health))
        .route(
            "/proxy/get_endpoint_access_control",
            get(proxy_control::access_control),
        )
        .route("/proxy/wake_compute", get(proxy_control::wake_compute))
        .route("/v1/projects", get(list_projects).post(create_project))
        .route("/v1/projects/{id}", get(get_project))
        .route(
            "/v1/projects/{id}/branches",
            get(list_branches).post(create_branch),
        )
        .route("/v1/projects/{id}/computes", get(list_computes))
        .route(
            "/v1/projects/{id}/branches/{branch}/computes",
            post(create_compute),
        )
        .route(
            "/v1/projects/{id}/branches/{branch}/connection",
            get(get_connection),
        )
        .route(
            "/v1/projects/{id}/branches/{branch}/service-connection",
            get(get_service_connection),
        )
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Briven local control API listening on {address}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ExistingName, ProjectState, existing_name, valid_name};

    #[test]
    fn names_cannot_escape_cli_or_collide_with_paths() {
        for name in [
            "",
            "../main",
            "Main",
            "main_2",
            "-main",
            "main-",
            "main name",
            "école",
        ] {
            assert!(!valid_name(name), "accepted {name}");
        }
        assert!(valid_name("customer-db-2"));
    }

    #[test]
    fn retrying_project_creation_reuses_one_tenant() {
        assert_eq!(existing_name(None), ExistingName::Create);
        assert_eq!(
            existing_name(Some(ProjectState::Provisioning)),
            ExistingName::Resume
        );
        assert_eq!(
            existing_name(Some(ProjectState::Failed)),
            ExistingName::Resume
        );
        assert_eq!(
            existing_name(Some(ProjectState::Ready)),
            ExistingName::Conflict
        );
    }
}
