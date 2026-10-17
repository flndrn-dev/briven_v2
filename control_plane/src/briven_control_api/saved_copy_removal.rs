//! Private, single-target removal. No customer-selected path, command or PID.
use super::*;
use control_plane::endpoint::Endpoint;
use control_plane::pageserver::PageServerNode;
use control_plane::reserved_branch_removal::{
    Journal, Phase, Record, RemovalAction, RemovalScope, is_saved_copy,
};
use control_plane::safekeeper::SafekeeperNode;
use control_plane::storage_controller::StorageController;
use lifecycle_process::{Presence, Processes};
use nix::fcntl::OFlag;
use pageserver_api::shard::TenantShardId;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RemoveBranch {
    timeline_id: TimelineId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Refusal {
    DependentBranch,
    ActiveConnections,
    PreparedTransactions,
    ActivityUnavailable,
    IdentityUnavailable,
    MappingChanged,
    UnsupportedStorage,
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum Outcome {
    Blocked { reason: Refusal },
    Removed,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Receipt {
    project_id: String,
    branch: String,
    timeline_id: String,
    #[serde(flatten)]
    outcome: Outcome,
}

fn receipt(scope: &RemovalScope, outcome: Outcome) -> Json<Receipt> {
    Json(Receipt {
        project_id: scope.project_id.clone(),
        branch: scope.branch.clone(),
        timeline_id: scope.timeline_id.clone(),
        outcome,
    })
}

fn compute_preflight(
    processes: &Processes,
    activity: Option<lifecycle::Activity>,
) -> Option<Refusal> {
    match (processes.postgres, processes.controller) {
        (Presence::Absent, Presence::Absent) => None,
        (Presence::Alive, Presence::Alive) => match activity {
            None => Some(Refusal::ActivityUnavailable),
            Some(a) if a.prepared > 0 => Some(Refusal::PreparedTransactions),
            Some(a) if a.clients > 0 => Some(Refusal::ActiveConnections),
            Some(a) if a.clients == 0 && a.prepared == 0 => None,
            Some(_) => Some(Refusal::ActivityUnavailable),
        },
        _ => Some(Refusal::IdentityUnavailable),
    }
}

pub(super) async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath((project_id, branch)): RoutePath<(String, String)>,
    Json(input): Json<RemoveBranch>,
) -> ApiResult<Json<Receipt>> {
    let principal = authenticated(&headers, &state.auth)?;
    let scope = RemovalScope {
        project_id,
        branch,
        timeline_id: input.timeline_id.to_string(),
        action: RemovalAction::RemoveSavedBranch,
    };
    if !principal.removal_for(&scope.project_id, &scope.branch, &scope.timeline_id) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "single-target removal grant required",
        ));
    }
    let _gate = state.gate.lock().await;
    let projects = state
        .catalog
        .load(&principal.organization)
        .await
        .map_err(internal)?;
    let project = project(&projects, &scope.project_id)?;
    if !matches!(project.state, ProjectState::Ready) {
        return Err(ApiError(StatusCode::CONFLICT, "project is not ready"));
    }
    let mut journal = Journal::lock(&state.repo)
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "engine mutation is busy"))?;
    tokio::time::timeout(
        Duration::from_secs(120),
        reconcile(&state, &mut journal, &scope, &principal.organization),
    )
    .await
    .map_err(|_| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "removal requires reconciliation",
        )
    })?
}

async fn reconcile(
    state: &AppState,
    journal: &mut Journal,
    scope: &RemovalScope,
    organization: &str,
) -> ApiResult<Json<Receipt>> {
    let mut env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let tenant = scope.project_id.parse::<TenantId>().map_err(internal)?;
    let timeline = scope.timeline_id.parse::<TimelineId>().map_err(internal)?;
    let previous = journal.get(scope).map_err(engine)?;
    if let Some(record) = &previous {
        if record.organization_id != organization {
            return Err(ApiError(StatusCode::FORBIDDEN, "removal identity changed"));
        }
        if record.phase == Phase::Removed {
            return Ok(receipt(scope, Outcome::Removed));
        }
    }
    // Resolve the authoritative attached storage; this implementation refuses
    // sharded tenants rather than silently deleting only one shard.
    let located = StorageController::from_env(&env)
        .tenant_locate(tenant)
        .await
        .map_err(engine)?;
    if located.shards.len() != 1 || located.shards[0].shard_id != TenantShardId::unsharded(tenant) {
        if previous.is_none() {
            return Ok(receipt(
                scope,
                Outcome::Blocked {
                    reason: Refusal::UnsupportedStorage,
                },
            ));
        }
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage binding changed",
        ));
    }
    let shard = located.shards[0].shard_id;
    let pageserver = PageServerNode::from_env(
        &env,
        env.get_pageserver_conf(located.shards[0].node_id)
            .map_err(engine)?,
    );
    let plane = ComputeControlPlane::load(env.clone()).map_err(engine)?;
    let candidates = plane
        .endpoints
        .values()
        .filter(|ep| ep.tenant_id == tenant && ep.timeline_id == timeline)
        .cloned()
        .collect::<Vec<_>>();
    let expected = format!("ep-{}-{}", scope.project_id, scope.branch);
    let invalid_endpoint = plane
        .endpoints
        .get(&expected)
        .is_some_and(|ep| ep.tenant_id != tenant || ep.timeline_id != timeline)
        || candidates.len() > 1
        || candidates
            .iter()
            .any(|ep| is_saved_copy(&scope.branch) || ep.id() != expected);
    let mut record = if let Some(record) = previous {
        if (record.phase != Phase::Accepted && !candidates.is_empty())
            || invalid_endpoint
            || candidates
                .first()
                .is_some_and(|ep| record.endpoint_id.as_deref() != Some(ep.id()))
        {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "compute binding changed",
            ));
        }
        // Repair any interrupted creation of the immutable timeline index.
        journal
            .begin(scope, organization, record.endpoint_id.as_deref())
            .map_err(engine)?
    } else {
        let mapped = env.get_branch_timeline_id(&scope.branch, tenant);
        if mapped.is_some_and(|id| id != timeline) {
            return Ok(receipt(
                scope,
                Outcome::Blocked {
                    reason: Refusal::MappingChanged,
                },
            ));
        }
        if invalid_endpoint {
            return Ok(receipt(
                scope,
                Outcome::Blocked {
                    reason: Refusal::IdentityUnavailable,
                },
            ));
        }
        let stored = pageserver
            .http_client
            .list_timelines_and_offloaded(shard)
            .await
            .map_err(engine)?;
        if stored
            .timelines
            .iter()
            .any(|t| t.timeline_id == timeline && t.ancestor_timeline_id.is_none())
            || stored
                .offloaded
                .iter()
                .any(|t| t.timeline_id == timeline && t.ancestor_timeline_id.is_none())
            || env.branch_name_mappings.iter().any(|(name, bindings)| {
                name != &scope.branch && bindings.contains(&(tenant, timeline))
            })
        {
            return Ok(receipt(
                scope,
                Outcome::Blocked {
                    reason: Refusal::IdentityUnavailable,
                },
            ));
        }
        let exists = stored.timelines.iter().any(|t| t.timeline_id == timeline)
            || stored.offloaded.iter().any(|t| t.timeline_id == timeline);
        if mapped.is_none() && (exists || !candidates.is_empty()) {
            return Ok(receipt(
                scope,
                Outcome::Blocked {
                    reason: Refusal::MappingChanged,
                },
            ));
        }
        if stored
            .timelines
            .iter()
            .any(|t| t.ancestor_timeline_id == Some(timeline))
            || stored
                .offloaded
                .iter()
                .any(|t| t.ancestor_timeline_id == Some(timeline))
        {
            return Ok(receipt(
                scope,
                Outcome::Blocked {
                    reason: Refusal::DependentBranch,
                },
            ));
        }
        if let Some(endpoint) = candidates.first() {
            let processes = Processes::inspect(endpoint);
            if processes.postgres == Presence::Absent
                && processes.controller == Presence::Absent
                && !Processes::absence_confirmed(endpoint)
            {
                return Ok(receipt(
                    scope,
                    Outcome::Blocked {
                        reason: Refusal::IdentityUnavailable,
                    },
                ));
            }
            let activity = if processes.postgres == Presence::Alive
                && processes.controller == Presence::Alive
            {
                lifecycle_runtime::activity(&endpoint.connstr("cloud_admin", "postgres")).await
            } else {
                None
            };
            if let Some(reason) = compute_preflight(&processes, activity) {
                return Ok(receipt(scope, Outcome::Blocked { reason }));
            }
        }
        journal
            .begin(scope, organization, candidates.first().map(|ep| ep.id()))
            .map_err(engine)?
    };
    if record.phase == Phase::Accepted {
        if let Some(endpoint) = candidates.first() {
            wait_for_exit(endpoint).await?;
            quarantine(&state.repo, endpoint, &record).map_err(engine)?;
        } else if let Some(id) = &record.endpoint_id {
            // A prior attempt may have renamed the stopped endpoint and exited
            // before committing the next phase. Never treat a missing cache as proof.
            verify_quarantine(&state.repo, id, &record).map_err(engine)?;
        }
        record = journal
            .advance(&record, Phase::Quarantined)
            .map_err(engine)?;
    }
    if record.phase == Phase::Quarantined {
        loop {
            let status = match pageserver
                .http_client
                .timeline_delete(shard, timeline)
                .await
            {
                // The existing pageserver reports an already-running async
                // deletion as 409. It is pending, never completion or a new
                // preflight refusal that could make the copy usable again.
                Err(pageserver_client::mgmt_api::Error::ApiError(status, _))
                    if status == reqwest::StatusCode::CONFLICT =>
                {
                    reqwest::StatusCode::ACCEPTED
                }
                result => result.map_err(engine)?,
            };
            if status == reqwest::StatusCode::NOT_FOUND {
                break;
            }
            if status != reqwest::StatusCode::ACCEPTED {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "storage removal is unconfirmed",
                ));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        record = journal
            .advance(&record, Phase::StorageRemoved)
            .map_err(engine)?;
    }
    if record.phase == Phase::StorageRemoved {
        if env.safekeepers.is_empty() {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "WAL cleanup is unconfirmed",
            ));
        }
        for conf in &env.safekeepers {
            SafekeeperNode::from_env(&env, conf)
                .http_client
                .delete_timeline(tenant, timeline)
                .await
                .map_err(engine)?;
        }
        record = journal
            .advance(&record, Phase::WalRemoved)
            .map_err(engine)?;
    }
    if record.phase == Phase::WalRemoved {
        env.unregister_branch_mapping(&scope.branch, tenant, timeline)
            .map_err(engine)?;
        env.persist_config().map_err(engine)?;
        if let Some(id) = &record.endpoint_id {
            verify_quarantine(&state.repo, id, &record).map_err(engine)?;
        }
        record = journal
            .advance(&record, Phase::CacheRemoving)
            .map_err(engine)?;
    }
    if record.phase == Phase::CacheRemoving {
        if let Some(id) = &record.endpoint_id {
            remove_quarantine(&state.repo, id, &record).map_err(engine)?;
        }
        journal.advance(&record, Phase::Removed).map_err(engine)?;
    }
    Ok(receipt(scope, Outcome::Removed))
}

async fn wait_for_exit(endpoint: &Endpoint) -> ApiResult<()> {
    let processes = Processes::inspect(endpoint);
    if processes.postgres == Presence::Unknown || processes.controller == Presence::Unknown {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "compute identity unavailable",
        ));
    }
    if processes.postgres == Presence::Alive {
        // Does not signal compute_ctl or force existing clients to disconnect.
        endpoint.request_smart_shutdown().map_err(engine)?;
    }
    loop {
        let processes = Processes::inspect(endpoint);
        if processes.postgres == Presence::Absent && processes.controller == Presence::Absent {
            return if Processes::absence_confirmed(endpoint) {
                Ok(())
            } else {
                Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "compute exit unconfirmed",
                ))
            };
        }
        if processes.postgres == Presence::Unknown || processes.controller == Presence::Unknown {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "compute exit unconfirmed",
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn quarantine_root(repo: &Path, create: bool) -> Result<PathBuf> {
    let path = repo.join("removed-endpoints");
    if create {
        match fs::DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => File::open(repo)?.sync_all()?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_dir()
        || metadata.uid() != fs::metadata(repo)?.uid()
        || metadata.mode() & 0o777 != 0o700
    {
        bail!("unsafe endpoint quarantine");
    }
    Ok(path)
}
fn quarantine(repo: &Path, endpoint: &Endpoint, record: &Record) -> Result<()> {
    let root = quarantine_root(repo, true)?;
    let destination = root.join(endpoint.id());
    if fs::symlink_metadata(&destination).is_ok() {
        bail!("endpoint quarantine already exists");
    }
    verify_config(&endpoint.endpoint_path(), record)?;
    fs::rename(endpoint.endpoint_path(), &destination)?;
    File::open(repo.join("endpoints"))?.sync_all()?;
    File::open(root)?.sync_all()?;
    Ok(())
}
fn verify_config(path: &Path, record: &Record) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() {
        bail!("unsafe quarantined endpoint");
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(OFlag::O_NOFOLLOW.bits())
        .open(path.join("endpoint.json"))?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > 65536 {
        bail!("invalid endpoint definition");
    }
    let mut body = Vec::new();
    file.take(65537).read_to_end(&mut body)?;
    let config: serde_json::Value = serde_json::from_slice(&body)?;
    if config["endpoint_id"].as_str() != record.endpoint_id.as_deref()
        || config["tenant_id"].as_str() != Some(&record.scope.project_id)
        || config["timeline_id"].as_str() != Some(&record.scope.timeline_id)
    {
        bail!("quarantined endpoint identity changed");
    }
    Ok(())
}
fn verify_quarantine(repo: &Path, id: &str, record: &Record) -> Result<()> {
    verify_config(&quarantine_root(repo, false)?.join(id), record)
}
fn remove_quarantine(repo: &Path, id: &str, record: &Record) -> Result<()> {
    let root = quarantine_root(repo, false)?;
    let path = root.join(id);
    match fs::symlink_metadata(&path) {
        // Retry after cache removal and before the final journal write.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    if record.phase != Phase::CacheRemoving {
        bail!("cache cleanup was not accepted");
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_dir() || metadata.uid() != fs::metadata(repo)?.uid() {
        bail!("unsafe removed cache");
    }
    fs::remove_dir_all(path)?;
    File::open(root)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifecycle::Activity;
    #[test]
    fn quarantined_cache_cleanup_resumes_partial_removal_and_refuses_redirects() {
        use std::os::unix::fs::symlink;
        let path =
            std::env::temp_dir().join(format!("briven-quarantine-{:016x}", rand::random::<u64>()));
        fs::create_dir(&path).unwrap();
        let cleanup = scopeguard::guard(path, |path| fs::remove_dir_all(path).unwrap());
        let root = quarantine_root(&cleanup, true).unwrap();
        let scope = RemovalScope {
            project_id: "a".repeat(32),
            branch: format!("rc-{}", "b".repeat(24)),
            timeline_id: "c".repeat(32),
            action: RemovalAction::RemoveSavedBranch,
        };
        let id = format!("ep-{}-{}", scope.project_id, scope.branch);
        let mut record = Record {
            version: 1,
            scope,
            organization_id: "org_fixture".into(),
            endpoint_id: Some(id.clone()),
            phase: Phase::WalRemoved,
        };
        let cache = root.join(&id);
        fs::create_dir(&cache).unwrap();
        fs::write(cache.join("endpoint.json"),serde_json::to_vec(&serde_json::json!({"endpoint_id":id,"tenant_id":record.scope.project_id,"timeline_id":record.scope.timeline_id})).unwrap()).unwrap();
        verify_quarantine(&cleanup, &id, &record).unwrap();
        assert!(remove_quarantine(&cleanup, &id, &record).is_err());
        // Interruption after the accepted cache-cleanup phase may leave only
        // part of the reconstructable cache. It must remain safely retryable.
        record.phase = Phase::CacheRemoving;
        fs::remove_file(cache.join("endpoint.json")).unwrap();
        fs::create_dir(cache.join("pgdata")).unwrap();
        fs::write(cache.join("pgdata/remaining"), b"cache").unwrap();
        remove_quarantine(&cleanup, &id, &record).unwrap();
        remove_quarantine(&cleanup, &id, &record).unwrap();
        let outside = cleanup.join("preserved");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("data"), b"retained").unwrap();
        symlink(&outside, &cache).unwrap();
        assert!(remove_quarantine(&cleanup, &id, &record).is_err());
        assert_eq!(fs::read(outside.join("data")).unwrap(), b"retained");
    }
    #[test]
    fn preflight_protects_clients_prepared_work_and_ambiguous_process_identity() {
        let running = Processes {
            postgres: Presence::Alive,
            controller: Presence::Alive,
        };
        assert_eq!(
            compute_preflight(
                &running,
                Some(Activity {
                    clients: 1,
                    prepared: 0
                })
            ),
            Some(Refusal::ActiveConnections)
        );
        assert_eq!(
            compute_preflight(
                &running,
                Some(Activity {
                    clients: 0,
                    prepared: 1
                })
            ),
            Some(Refusal::PreparedTransactions)
        );
        assert_eq!(
            compute_preflight(&running, None),
            Some(Refusal::ActivityUnavailable)
        );
        assert_eq!(
            compute_preflight(
                &running,
                Some(Activity {
                    clients: -1,
                    prepared: 0
                })
            ),
            Some(Refusal::ActivityUnavailable)
        );
        assert_eq!(compute_preflight(&running, Some(Activity::default())), None);
        assert_eq!(
            compute_preflight(
                &Processes {
                    postgres: Presence::Absent,
                    controller: Presence::Absent
                },
                None
            ),
            None
        );
        for (postgres, controller) in [
            (Presence::Alive, Presence::Unknown),
            (Presence::Unknown, Presence::Absent),
            (Presence::Absent, Presence::Alive),
            (Presence::Alive, Presence::Absent),
        ] {
            assert_eq!(
                compute_preflight(
                    &Processes {
                        postgres,
                        controller
                    },
                    Some(Activity::default())
                ),
                Some(Refusal::IdentityUnavailable)
            );
        }
    }
}
