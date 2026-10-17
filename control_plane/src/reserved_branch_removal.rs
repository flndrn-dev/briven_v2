//! Durable, fail-closed tombstones for explicit removal of reserved branches.
use anyhow::{Context, Result, bail};
use nix::fcntl::{Flock, FlockArg, OFlag};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub use crate::reserved_branch_removal_types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Accepted,
    Quarantined,
    StorageRemoved,
    WalRemoved,
    CacheRemoving,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub version: u8,
    pub scope: RemovalScope,
    pub organization_id: String,
    pub endpoint_id: Option<String>,
    pub phase: Phase,
}

pub struct Journal {
    repo: PathBuf,
    // Same directory lock as briven_local. Never invoke the CLI while held.
    _lock: Flock<File>,
}
impl Journal {
    pub fn lock(repo: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(OFlag::O_NOFOLLOW.bits() | OFlag::O_DIRECTORY.bits())
            .open(repo)?;
        let lock = Flock::lock(file, FlockArg::LockExclusiveNonblock)
            .map_err(|(_, error)| error)
            .context("repository is busy")?;
        Ok(Self {
            repo: repo.to_owned(),
            _lock: lock,
        })
    }
    /// Call only after all preflight refusals; returning successfully means the
    /// branch is irreversibly fenced even if later storage work is interrupted.
    pub fn begin(
        &mut self,
        scope: &RemovalScope,
        organization: &str,
        endpoint: Option<&str>,
    ) -> Result<Record> {
        let record = Record {
            version: 1,
            scope: scope.clone(),
            organization_id: organization.into(),
            endpoint_id: endpoint.map(str::to_owned),
            phase: Phase::Accepted,
        };
        record.validate()?;
        let current = self.get(scope)?;
        let dir = self.directory(scope)?;
        let canonical = dir.join(format!("{}.json", scope.branch));
        let index = dir.join(format!("timeline-{}.json", scope.timeline_id));
        if let Some(current) = &current {
            if !current.same_binding(&record) {
                bail!("removal identity changed");
            }
        }
        if let Some(existing) = read_record(&self.repo, &index)? {
            if !existing.same_binding(&record) {
                bail!("timeline is bound to another removal");
            }
            if current.is_none() {
                bail!("removal branch journal is missing");
            }
        }
        if current.is_none() {
            atomic_record(&canonical, &record)?;
        }
        // Link the first durable intent; later canonical phase replacements do
        // not change the index. Every phase fences starts identically.
        if read_record(&self.repo, &index)?.is_none() {
            fs::hard_link(&canonical, &index)?;
            File::open(&dir)?.sync_all()?;
        }
        Ok(current.unwrap_or(record))
    }
    pub fn get(&self, scope: &RemovalScope) -> Result<Option<Record>> {
        if !scope.valid() {
            bail!("invalid removal scope");
        }
        let path = self
            .repo
            .join("reserved-removals")
            .join(&scope.project_id)
            .join(format!("{}.json", scope.branch));
        let record = read_record(&self.repo, &path)?;
        if record.as_ref().is_some_and(|r| r.scope != *scope) {
            bail!("removal scope changed");
        }
        Ok(record)
    }
    pub fn advance(&mut self, record: &Record, next: Phase) -> Result<Record> {
        record.validate()?;
        if self.get(&record.scope)?.as_ref() != Some(record)
            || !matches!(
                (record.phase, next),
                (Phase::Accepted, Phase::Quarantined)
                    | (Phase::Quarantined, Phase::StorageRemoved)
                    | (Phase::StorageRemoved, Phase::WalRemoved)
                    | (Phase::WalRemoved, Phase::CacheRemoving)
                    | (Phase::CacheRemoving, Phase::Removed)
            )
        {
            bail!("stale or invalid removal phase");
        }
        // Also repairs a crash between canonical intent and timeline index.
        self.begin(
            &record.scope,
            &record.organization_id,
            record.endpoint_id.as_deref(),
        )?;
        let mut updated = record.clone();
        updated.phase = next;
        let path = self
            .directory(&record.scope)?
            .join(format!("{}.json", record.scope.branch));
        atomic_record(&path, &updated)?;
        Ok(updated)
    }
    fn directory(&self, scope: &RemovalScope) -> Result<PathBuf> {
        let root = self.repo.join("reserved-removals");
        for path in [&root, &root.join(&scope.project_id)] {
            match fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => {
                    File::open(path.parent().context("removal parent missing")?)?.sync_all()?
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
            check_directory(&self.repo, path)?;
        }
        Ok(root.join(&scope.project_id))
    }
}
impl Record {
    fn validate(&self) -> Result<()> {
        if self.version != 1
            || !self.scope.valid()
            || self.organization_id.is_empty()
            || self.organization_id.len() > 200
            || !self
                .organization_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            || self.endpoint_id.as_ref().is_some_and(|id| {
                is_saved_copy(&self.scope.branch)
                    || *id != format!("ep-{}-{}", self.scope.project_id, self.scope.branch)
            })
        {
            bail!("invalid removal record");
        }
        Ok(())
    }
    fn same_binding(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.organization_id == other.organization_id
            && self.endpoint_id == other.endpoint_id
    }
}

fn check_directory(repo: &Path, path: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    if !metadata.is_dir()
        || metadata.uid() != fs::metadata(repo)?.uid()
        || metadata.mode() & 0o777 != 0o700
    {
        bail!("unsafe removal directory");
    }
    Ok(true)
}
fn read_record(repo: &Path, path: &Path) -> Result<Option<Record>> {
    let parent = path.parent().context("removal parent missing")?;
    if !check_directory(repo, &repo.join("reserved-removals"))? || !check_directory(repo, parent)? {
        return Ok(None);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(OFlag::O_NOFOLLOW.bits())
        .open(path);
    let file = match file {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != fs::metadata(repo)?.uid()
        || metadata.mode() & 0o777 != 0o600
        || metadata.len() > 4096
    {
        bail!("unsafe removal record");
    }
    let mut body = Vec::new();
    file.take(4097).read_to_end(&mut body)?;
    if body.len() > 4096 {
        bail!("oversized removal record");
    }
    let record: Record = serde_json::from_slice(&body)?;
    record.validate()?;
    if parent.file_name().and_then(|s| s.to_str()) != Some(record.scope.project_id.as_str()) {
        bail!("removal directory binding changed");
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .context("invalid removal filename")?;
    if name != format!("{}.json", record.scope.branch)
        && name != format!("timeline-{}.json", record.scope.timeline_id)
    {
        bail!("removal filename binding changed");
    }
    Ok(Some(record))
}
fn atomic_record(path: &Path, record: &Record) -> Result<()> {
    let temporary = path.with_extension(format!("{:016x}.tmp", rand::random::<u64>()));
    let cleanup = scopeguard::guard(temporary, |path| {
        let _ = fs::remove_file(path);
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(OFlag::O_NOFOLLOW.bits())
        .open(&*cleanup)?;
    file.write_all(&serde_json::to_vec(record)?)?;
    file.sync_all()?;
    fs::rename(&*cleanup, path)?;
    File::open(path.parent().context("removal parent missing")?)?.sync_all()?;
    Ok(())
}

/// Does not create directories on ordinary starts. A missing timeline index
/// after a crash is covered by the branch mapping until begin repairs it.
pub fn ensure_available(
    repo: &Path,
    project: &str,
    branch: Option<&str>,
    timeline: &str,
) -> Result<()> {
    let id = |s: &str| {
        s.len() == 32
            && s.bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    };
    if !id(project) || !id(timeline) {
        bail!("invalid timeline identity");
    }
    let directory = repo.join("reserved-removals").join(project);
    if let Some(branch) = branch.filter(|name| is_reserved_branch(name)) {
        if read_record(repo, &directory.join(format!("{branch}.json")))?.is_some() {
            bail!("reserved branch removal has started");
        }
    }
    if read_record(repo, &directory.join(format!("timeline-{timeline}.json")))?.is_some() {
        bail!("timeline removal has started");
    }
    Ok(())
}

pub fn ensure_timeline_available(
    env: &crate::local_env::LocalEnv,
    tenant: utils::id::TenantId,
    timeline: utils::id::TimelineId,
) -> Result<()> {
    ensure_available(
        &env.base_data_dir,
        &tenant.to_string(),
        None,
        &timeline.to_string(),
    )?;
    // Also fences the durable branch intent if the process stopped before the
    // timeline index was linked. Mapping removal happens only after both exist.
    for (name, bindings) in &env.branch_name_mappings {
        if bindings.contains(&(tenant, timeline)) {
            ensure_available(
                &env.base_data_dir,
                &tenant.to_string(),
                Some(name),
                &timeline.to_string(),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "briven-removal-{}-{:016x}",
                std::process::id(),
                rand::random::<u64>()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn scope() -> RemovalScope {
        RemovalScope {
            project_id: "a".repeat(32),
            branch: format!("sc-{}", "b".repeat(24)),
            timeline_id: "c".repeat(32),
            action: RemovalAction::RemoveSavedBranch,
        }
    }
    #[test]
    fn intent_survives_reopen_and_blocks_both_identities_without_blocking_another_project() {
        let f = Fixture::new();
        let s = scope();
        ensure_available(&f.0, &s.project_id, Some(&s.branch), &s.timeline_id).unwrap();
        assert!(!f.0.join("reserved-removals").exists());
        let record = Journal::lock(&f.0)
            .unwrap()
            .begin(&s, "org_fixture", None)
            .unwrap();
        assert_eq!(record.phase, Phase::Accepted);
        assert_eq!(
            Journal::lock(&f.0).unwrap().get(&s).unwrap(),
            Some(record.clone())
        );
        assert!(ensure_available(&f.0, &s.project_id, None, &s.timeline_id).is_err());
        assert!(ensure_available(&f.0, &s.project_id, Some(&s.branch), &"d".repeat(32)).is_err());
        ensure_available(&f.0, &"e".repeat(32), Some(&s.branch), &s.timeline_id).unwrap();
        let dir = f.0.join("reserved-removals").join(&s.project_id);
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(dir.join(format!("{}.json", s.branch)))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    #[test]
    fn replay_preserves_progress_and_refuses_changed_binding_or_stale_transition() {
        let f = Fixture::new();
        let s = scope();
        let mut j = Journal::lock(&f.0).unwrap();
        let first = j.begin(&s, "org_fixture", None).unwrap();
        let quarantined = j.advance(&first, Phase::Quarantined).unwrap();
        assert!(j.advance(&first, Phase::Quarantined).is_err());
        assert!(j.advance(&quarantined, Phase::Removed).is_err());
        assert_eq!(j.begin(&s, "org_fixture", None).unwrap(), quarantined);
        assert!(j.begin(&s, "org_other", None).is_err());
        assert!(j.begin(&s, "org_fixture", Some("ep-foreign")).is_err());
        let mut changed = s.clone();
        changed.timeline_id = "d".repeat(32);
        assert!(j.begin(&changed, "org_fixture", None).is_err());
        changed = s.clone();
        changed.branch = format!("sc-{}", "e".repeat(24));
        assert!(j.begin(&changed, "org_fixture", None).is_err());
        let storage = j.advance(&quarantined, Phase::StorageRemoved).unwrap();
        let wal = j.advance(&storage, Phase::WalRemoved).unwrap();
        let cache = j.advance(&wal, Phase::CacheRemoving).unwrap();
        let removed = j.advance(&cache, Phase::Removed).unwrap();
        assert_eq!(j.begin(&s, "org_fixture", None).unwrap(), removed);
        assert!(ensure_available(&f.0, &s.project_id, None, &s.timeline_id).is_err());
    }
    #[test]
    fn crash_before_timeline_index_is_repaired_without_losing_the_branch_tombstone() {
        let f = Fixture::new();
        let s = scope();
        let mut j = Journal::lock(&f.0).unwrap();
        let record = j.begin(&s, "org_fixture", None).unwrap();
        fs::remove_file(
            f.0.join("reserved-removals")
                .join(&s.project_id)
                .join(format!("timeline-{}.json", s.timeline_id)),
        )
        .unwrap();
        assert!(ensure_available(&f.0, &s.project_id, Some(&s.branch), &s.timeline_id).is_err());
        assert_eq!(j.begin(&s, "org_fixture", None).unwrap(), record);
        assert!(ensure_available(&f.0, &s.project_id, None, &s.timeline_id).is_err());
    }
    #[test]
    fn corrupt_or_redirected_journals_fail_closed_and_invalid_input_has_no_effects() {
        let f = Fixture::new();
        let s = scope();
        let mut j = Journal::lock(&f.0).unwrap();
        let mut invalid = s.clone();
        invalid.branch = "../outside".into();
        assert!(j.begin(&invalid, "org_fixture", None).is_err());
        assert!(!f.0.join("reserved-removals").exists());
        j.begin(&s, "org_fixture", None).unwrap();
        let path =
            f.0.join("reserved-removals")
                .join(&s.project_id)
                .join(format!("{}.json", s.branch));
        fs::write(&path, b"{").unwrap();
        assert!(j.get(&s).is_err());
        assert!(ensure_available(&f.0, &s.project_id, None, &s.timeline_id).is_err());
        fs::remove_file(&path).unwrap();
        let outside = f.0.join("outside");
        fs::write(&outside, b"untouched").unwrap();
        symlink(&outside, &path).unwrap();
        assert!(j.begin(&s, "org_fixture", None).is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"untouched");
        fs::remove_file(&path).unwrap();
        drop(j);
        let other = Fixture::new();
        symlink(&other.0, f.0.join("redirect")).unwrap();
        assert!(Journal::lock(&f.0.join("redirect")).is_err());
    }
    #[test]
    fn repository_lock_excludes_concurrent_mutations() {
        let f = Fixture::new();
        let j = Journal::lock(&f.0).unwrap();
        assert!(Journal::lock(&f.0).is_err());
        drop(j);
        Journal::lock(&f.0).unwrap();
    }

    #[test]
    fn every_committed_cleanup_phase_reopens_fenced_and_replays_without_regression() {
        let f = Fixture::new();
        let s = scope();
        let mut record = Journal::lock(&f.0)
            .unwrap()
            .begin(&s, "org_fixture", None)
            .unwrap();
        for next in [
            Phase::Quarantined,
            Phase::StorageRemoved,
            Phase::WalRemoved,
            Phase::CacheRemoving,
            Phase::Removed,
        ] {
            record = Journal::lock(&f.0).unwrap().advance(&record, next).unwrap();
            assert_eq!(
                Journal::lock(&f.0)
                    .unwrap()
                    .begin(&s, "org_fixture", None)
                    .unwrap(),
                record
            );
            assert!(
                ensure_available(&f.0, &s.project_id, Some(&s.branch), &s.timeline_id).is_err()
            );
            assert!(ensure_available(&f.0, &s.project_id, None, &s.timeline_id).is_err());
        }
    }
}
