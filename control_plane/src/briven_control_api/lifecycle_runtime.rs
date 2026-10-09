//! Bounded orchestration of the fixed lifecycle policy under the engine mutation gate.
use std::collections::HashSet;

use super::lifecycle::{
    Action, Activity, DEFAULT_IDLE_SECONDS, Failure, Observation, Phase, Record,
};
use super::lifecycle_process::{Presence, Processes};
use super::*;

#[derive(Default)]
pub(super) struct Config {
    enabled: bool,
    endpoints: HashSet<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let enabled = std::env::var("BRIVEN_COMPUTE_LIFECYCLE_ENABLED").ok();
        let targets = std::env::var("BRIVEN_COMPUTE_LIFECYCLE_ENDPOINTS_JSON")
            .unwrap_or_else(|_| "[]".to_string());
        Self::parse(enabled.as_deref(), &targets)
    }

    fn parse(enabled: Option<&str>, targets: &str) -> Result<Self> {
        let enabled = match enabled {
            None | Some("false") => false,
            Some("true") => true,
            _ => bail!("BRIVEN_COMPUTE_LIFECYCLE_ENABLED must be true or false"),
        };
        let endpoints: Vec<String> = serde_json::from_str(targets)?;
        if endpoints.len() > 32 || endpoints.iter().any(|id| !lifecycle::valid_endpoint(id)) {
            bail!("invalid isolated lifecycle endpoint allowlist");
        }
        let allowed: HashSet<_> = endpoints.iter().cloned().collect();
        if endpoints.len() != allowed.len() {
            bail!("duplicate lifecycle enrollment target");
        }
        Ok(Self {
            enabled,
            endpoints: allowed,
        })
    }
    fn allows(&self, endpoint: &str) -> bool {
        self.enabled && self.endpoints.contains(endpoint)
    }
}

fn now() -> ApiResult<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .map_err(internal)
}

async fn persist(state: &AppState, record: &mut Record) -> ApiResult<()> {
    let previous = record.generation;
    record.generation = previous.checked_add(1).ok_or(ApiError(
        StatusCode::SERVICE_UNAVAILABLE,
        "compute state unavailable",
    ))?;
    record.changed_at = now()?;
    if !state
        .lifecycle_store
        .save(record, Some(previous))
        .await
        .map_err(internal)?
    {
        return Err(ApiError(StatusCode::CONFLICT, "compute lifecycle changed"));
    }
    Ok(())
}

pub(super) async fn activity(uri: &str) -> Option<Activity> {
    let separator = if uri.contains('?') { '&' } else { '?' };
    tokio::time::timeout(Duration::from_secs(5),async {
        let (client,connection)=tokio_postgres::connect(&format!("{uri}{separator}sslmode=disable&application_name=briven_lifecycle_inspection"),NoTls).await?;
        let task=scopeguard::guard(tokio::spawn(async move { let _=connection.await; }),|task|task.abort());
        let row=client.query_one("SELECT COUNT(*) FILTER(WHERE backend_type='client backend' AND pid<>pg_backend_pid()),
            (SELECT COUNT(*) FROM pg_prepared_xacts) FROM pg_stat_activity",&[]).await;
        task.abort();
        row.map(|row|Activity {clients:row.get(0),prepared:row.get(1)})
    }).await.ok()?.ok()
}

fn binding(record: &Record, endpoint: &control_plane::endpoint::Endpoint) -> bool {
    record.matches(
        endpoint.id(),
        &endpoint.tenant_id.to_string(),
        &endpoint.timeline_id.to_string(),
    )
}

async fn smart_stop(
    state: &AppState,
    record: &mut Record,
    endpoint: &control_plane::endpoint::Endpoint,
) -> ApiResult<()> {
    if record.phase != Phase::Stopping {
        record.transition(Phase::Stopping, now()?);
        record.stop_requested = false;
        record.failure = None;
        persist(state, record).await?;
    }
    // PostgreSQL may already have exited while compute_ctl is finishing; no
    // second signal or start is safe until both are gone.
    if Processes::inspect(endpoint).postgres != Presence::Alive {
        return Ok(());
    }
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        run_cli(
            state,
            &[
                "endpoint",
                "stop",
                &record.endpoint_id,
                "--mode",
                "smart",
                "--no-wait",
            ],
        ),
    )
    .await;
    record.stop_requested = true;
    record.failure = if matches!(result, Ok(Ok(()))) {
        None
    } else {
        Some(Failure::SleepRequestFailed)
    };
    persist(state, record).await
}

/// Caller holds the same engine mutation gate used by scheduler and endpoints.
pub(super) async fn demand(
    state: &AppState,
    endpoint: Arc<control_plane::endpoint::Endpoint>,
) -> ApiResult<Arc<control_plane::endpoint::Endpoint>> {
    let Some(mut record) = state
        .lifecycle_store
        .get(endpoint.id())
        .await
        .map_err(internal)?
    else {
        let status = endpoint.status();
        if status == EndpointStatus::Running {
            if control_plane::compute_resource::enabled().map_err(internal)?
                && Processes::inspect(&endpoint).observed(&endpoint, true) != Observation::Ready
            {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "compute resource binding unavailable",
                ));
            }
            return Ok(endpoint);
        }
        let observed = Processes::inspect(&endpoint).observed(&endpoint, false);
        if status != EndpointStatus::Stopped || observed != Observation::Absent {
            let time = now()?;
            let mut failure = Record::new(
                endpoint.id(),
                &endpoint.tenant_id.to_string(),
                &endpoint.timeline_id.to_string(),
                DEFAULT_IDLE_SECONDS,
                time,
            )
            .map_err(internal)?;
            failure.enabled = false;
            failure.phase = Phase::Failed;
            failure.failure = Some(Failure::UnexpectedExit);
            if !state
                .lifecycle_store
                .save(&failure, None)
                .await
                .map_err(internal)?
            {
                return Err(ApiError(StatusCode::CONFLICT, "compute lifecycle changed"));
            }
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "unexpected compute state requires review",
            ));
        }
        {
            control_plane::compute_resource::admit_start(&endpoint.tenant_id.to_string()).map_err(
                |_| {
                    ApiError(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "compute capacity unavailable",
                    )
                },
            )?;
            tokio::time::timeout(
                Duration::from_secs(90),
                run_cli(state, &["endpoint", "start", endpoint.id()]),
            )
            .await
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "compute resume timed out"))??;
        }
        return Ok(endpoint);
    };
    if !binding(&record, &endpoint) {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "compute binding unavailable",
        ));
    }
    let time = now()?;
    let processes = Processes::inspect(&endpoint);
    let sql_ready = processes.postgres == Presence::Alive
        && processes.controller == Presence::Alive
        && can_query(&endpoint.connstr("cloud_admin", "postgres")).await;
    let observation = processes.observed(&endpoint, sql_ready);
    record.last_demand = time;
    let action = record.demand(time, observation);
    match action {
        Action::Running => {
            record.transition(Phase::Running, now()?);
            record.failure = None;
            record.stop_requested = false;
            persist(state, &mut record).await?;
            Ok(endpoint)
        }
        Action::Wake => {
            control_plane::compute_resource::admit_start(&record.project_id).map_err(|_| {
                ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "compute capacity unavailable",
                )
            })?;
            record.transition(Phase::Starting, now()?);
            record.failure = None;
            record.stop_requested = false;
            record.starts = record.starts.checked_add(1).ok_or(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "compute state unavailable",
            ))?;
            persist(state, &mut record).await?;
            let outcome = tokio::time::timeout(
                Duration::from_secs(90),
                run_cli(state, &["endpoint", "start", endpoint.id()]),
            )
            .await;
            if !matches!(outcome, Ok(Ok(())))
                || !can_query(&endpoint.connstr("cloud_admin", "postgres")).await
                || Processes::inspect(&endpoint).observed(&endpoint, true) != Observation::Ready
            {
                record.transition(Phase::Failed, now()?);
                record.failure = Some(Failure::WakeIncomplete);
                persist(state, &mut record).await?;
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "compute resume requires review",
                ));
            }
            record.transition(Phase::Running, now()?);
            persist(state, &mut record).await?;
            Ok(endpoint)
        }
        Action::Review(failure) => {
            record.transition(Phase::Failed, now()?);
            record.failure = Some(failure);
            persist(state, &mut record).await?;
            Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "compute requires review",
            ))
        }
        _ => {
            persist(state, &mut record).await?;
            Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "compute is draining or requires review",
            ))
        }
    }
}

async fn tick(state: &AppState) -> ApiResult<()> {
    let _guard = state.gate.lock().await;
    let records = state.lifecycle_store.list().await.map_err(internal)?;
    let plane = load_plane(state)?;
    for mut record in records {
        if !state.lifecycle_config.allows(&record.endpoint_id) {
            continue;
        }
        let candidate = plane.endpoints.get(&record.endpoint_id);
        let project = state
            .catalog
            .ready_project(&record.project_id)
            .await
            .map_err(internal)?;
        let Some(endpoint) = candidate
            .filter(|endpoint| binding(&record, endpoint))
            .filter(|_| project.is_some())
        else {
            if record.phase != Phase::Failed || record.failure != Some(Failure::BindingChanged) {
                record.transition(Phase::Failed, now()?);
                record.failure = Some(Failure::BindingChanged);
                persist(state, &mut record).await?;
            }
            continue;
        };
        let processes = Processes::inspect(endpoint);
        let sample =
            if processes.postgres == Presence::Alive && processes.controller == Presence::Alive {
                activity(&endpoint.connstr("cloud_admin", "postgres")).await
            } else {
                None
            };
        let observed = processes.observed(endpoint, sample.is_some());
        let time = now()?;
        if record.phase == Phase::Running && sample.is_some_and(|v| v.clients > 0 || v.prepared > 0)
        {
            record.last_demand = time;
        }
        match record.tick(time, observed, sample) {
            Action::Sleep => smart_stop(state, &mut record, endpoint).await?,
            Action::Sleeping => {
                record.transition(Phase::Sleeping, now()?);
                persist(state, &mut record).await?;
            }
            Action::Running => {
                record.transition(Phase::Running, now()?);
                record.failure = None;
                persist(state, &mut record).await?;
            }
            Action::Review(failure) => {
                record.transition(Phase::Failed, now()?);
                record.failure = Some(failure);
                persist(state, &mut record).await?;
            }
            _ if record.phase == Phase::Running => {
                record.failure = if sample.is_none() {
                    Some(Failure::ActivityUnavailable)
                } else {
                    None
                };
                persist(state, &mut record).await?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn spawn(state: AppState) {
    if !state.lifecycle_config.enabled {
        return;
    }
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if !matches!(
                tokio::time::timeout(Duration::from_secs(25), tick(&state)).await,
                Ok(Ok(()))
            ) {
                eprintln!(
                    "Briven lifecycle: tick incomplete; retained state requires reconciliation"
                );
            }
        }
    });
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Configure {
    enabled: bool,
    idle_seconds: Option<u32>,
}

pub(super) async fn configure(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath((id, branch)): RoutePath<(String, String)>,
    Json(input): Json<Configure>,
) -> ApiResult<Json<Record>> {
    let principal = authorize(&headers, &state.auth, true)?;
    if !matches!(principal.role, auth::Role::Owner | auth::Role::Admin) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "owner or administrator required",
        ));
    }
    writable_branch(&branch)?;
    let _guard = state.gate.lock().await;
    let projects = state
        .catalog
        .load(&principal.organization)
        .await
        .map_err(internal)?;
    let found = project(&projects, &id)?;
    if !matches!(found.state, ProjectState::Ready) {
        return Err(ApiError(StatusCode::CONFLICT, "project is not ready"));
    }
    let plane = load_plane(&state)?;
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let timeline = env.get_branch_timeline_id(&branch, id.parse::<TenantId>().map_err(internal)?);
    let endpoint = plane
        .endpoints
        .values()
        .find(|endpoint| {
            endpoint.tenant_id.to_string() == id && Some(endpoint.timeline_id) == timeline
        })
        .ok_or(ApiError(StatusCode::NOT_FOUND, "compute not found"))?;
    if !state.lifecycle_config.allows(endpoint.id()) {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "compute lifecycle is not enabled for this endpoint",
        ));
    }
    let time = now()?;
    let mut candidate = Record::new(
        endpoint.id(),
        &id,
        &endpoint.timeline_id.to_string(),
        input.idle_seconds.unwrap_or(DEFAULT_IDLE_SECONDS),
        time,
    )
    .map_err(|_| {
        ApiError(
            StatusCode::BAD_REQUEST,
            "idle window must be 60 to 3600 seconds",
        )
    })?;
    if !input.enabled {
        if let Some(mut existing) = state
            .lifecycle_store
            .get(endpoint.id())
            .await
            .map_err(internal)?
        {
            if !binding(&existing, endpoint) {
                return Err(ApiError(StatusCode::CONFLICT, "compute binding changed"));
            }
            existing.enabled = false;
            existing.idle_seconds = candidate.idle_seconds;
            persist(&state, &mut existing).await?;
            return Ok(Json(existing));
        }
    }
    if Processes::inspect(endpoint).observed(
        endpoint,
        can_query(&endpoint.connstr("cloud_admin", "postgres")).await,
    ) != Observation::Ready
    {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "enrollment requires a verified running compute",
        ));
    }
    candidate.enabled = input.enabled;
    if let Some(existing) = state
        .lifecycle_store
        .get(endpoint.id())
        .await
        .map_err(internal)?
    {
        if !binding(&existing, endpoint) {
            return Err(ApiError(StatusCode::CONFLICT, "compute binding changed"));
        }
        candidate.generation = existing.generation;
        candidate.starts = existing.starts;
        persist(&state, &mut candidate).await?;
    } else if !state
        .lifecycle_store
        .save(&candidate, None)
        .await
        .map_err(internal)?
    {
        return Err(ApiError(StatusCode::CONFLICT, "compute lifecycle changed"));
    }
    Ok(Json(candidate))
}

pub(super) async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath((id, branch)): RoutePath<(String, String)>,
) -> ApiResult<Json<Option<Record>>> {
    let principal = authorize(&headers, &state.auth, false)?;
    let _guard = state.gate.lock().await;
    let projects = state
        .catalog
        .load(&principal.organization)
        .await
        .map_err(internal)?;
    project(&projects, &id)?;
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let timeline = env
        .get_branch_timeline_id(&branch, id.parse::<TenantId>().map_err(internal)?)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "branch not found"))?;
    let plane = load_plane(&state)?;
    let endpoint = plane
        .endpoints
        .values()
        .find(|endpoint| endpoint.tenant_id.to_string() == id && endpoint.timeline_id == timeline)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "compute not found"))?;
    let record = state
        .lifecycle_store
        .get(endpoint.id())
        .await
        .map_err(internal)?;
    if record
        .as_ref()
        .is_some_and(|record| !binding(record, endpoint))
    {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "compute binding unavailable",
        ));
    }
    Ok(Json(record))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enrollment_is_explicit_bounded_and_has_no_implicit_targets() {
        assert!(!Config::parse(None, "[]").unwrap().enabled);
        assert!(!Config::parse(Some("true"), "[]").unwrap().allows("ep-a"));
        let allowed = Config::parse(Some("true"), "[\"ep-a\"]").unwrap();
        assert!(allowed.allows("ep-a"));
        assert!(!allowed.allows("ep-b"));
        for targets in ["[\"ep-a\",\"ep-a\"]", "[\"../other\"]", "{}", "[1]"] {
            assert!(Config::parse(Some("true"), targets).is_err());
        }
        assert!(Config::parse(Some("yes"), "[]").is_err());
        let excessive =
            serde_json::to_string(&(0..33).map(|v| format!("ep-{v}")).collect::<Vec<_>>()).unwrap();
        assert!(Config::parse(Some("true"), &excessive).is_err());
    }
    #[tokio::test]
    async fn real_activity_protects_transactions_idle_clients_and_prepared_work() {
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
        // This URI is a Unix socket query string; activity normally consumes a
        // query-free trusted engine URI. Use the same sample function with its
        // URI option separator handled correctly below.
        let client = super::super::catalog::pg_client(&dsn).await.unwrap();
        client
            .batch_execute(
                "CREATE TABLE lifecycle_activity_fixture(value integer);
                             BEGIN; INSERT INTO lifecycle_activity_fixture VALUES(1)",
            )
            .await
            .unwrap();
        let during = activity(&dsn).await.unwrap();
        assert!(during.clients >= 1);
        assert_eq!(during.prepared, 0);
        client.batch_execute("COMMIT").await.unwrap();
        assert!(activity(&dsn).await.unwrap().clients >= 1);
        client
            .batch_execute(
                "BEGIN; INSERT INTO lifecycle_activity_fixture VALUES(2);
                             PREPARE TRANSACTION 'briven_lifecycle_fixture_prepared'",
            )
            .await
            .unwrap();
        assert_eq!(activity(&dsn).await.unwrap().prepared, 1);
        client
            .batch_execute("ROLLBACK PREPARED 'briven_lifecycle_fixture_prepared'")
            .await
            .unwrap();
        drop(client);
        let mut empty = false;
        for _ in 0..30 {
            let sample = activity(&dsn).await.unwrap();
            if sample.clients == 0 && sample.prepared == 0 {
                empty = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(
            empty,
            "inspector excludes itself and finished inspection connections close"
        );
        assert!(
            activity("postgresql://flndrn@127.0.0.1:1/postgres")
                .await
                .is_none()
        );
    }
}
