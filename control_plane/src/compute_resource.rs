//! Fixed host resource service. The engine never receives cgroup write access.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

pub const SOCKET: &str = "/run/briven-compute-control/control.sock";
pub const MEMORY_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_ACTIVE: usize = 4;
pub const MAX_PROJECT_ACTIVE: usize = 2;
pub const MAX_ENDPOINTS: usize = 128;
pub const MAX_PROJECT_ENDPOINTS: usize = 32;

pub fn project_for_endpoint(endpoint: &str) -> Result<&str> {
    let tail = endpoint
        .strip_prefix("ep-")
        .context("resource endpoint prefix missing")?;
    if tail.len() < 32 || endpoint.len() > 96 || !tail.is_ascii() {
        bail!("invalid resource endpoint");
    }
    let project = &tail[..32];
    if !project
        .bytes()
        .all(|v| v.is_ascii_digit() || matches!(v, b'a'..=b'f'))
        || (!tail[32..].is_empty() && (!tail[32..].starts_with('-') || tail.ends_with('-')))
        || !tail
            .bytes()
            .all(|v| v.is_ascii_lowercase() || v.is_ascii_digit() || v == b'-')
    {
        bail!("invalid resource endpoint");
    }
    Ok(project)
}

pub fn enabled() -> Result<bool> {
    match std::env::var("BRIVEN_COMPUTE_RESOURCE_SOCKET")
        .ok()
        .as_deref()
    {
        None | Some("") => Ok(false),
        Some(SOCKET) => Ok(true),
        Some(_) => bail!("compute resource socket must be the fixed host service"),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    version: u8,
    ok: bool,
    action: String,
    endpoint: Option<String>,
    memory_bytes: u64,
    cpu_quota_usec: u64,
    cpu_period_usec: u64,
    max_pids: u64,
    parent_verified: bool,
    process_bound: bool,
}

impl Receipt {
    fn validate(&self, action: &str, endpoint: Option<&str>) -> Result<()> {
        if self.version != 1
            || !self.ok
            || self.action != action
            || self.endpoint.as_deref() != endpoint
            || self.memory_bytes != MEMORY_BYTES
            || self.cpu_quota_usec != 50_000
            || self.cpu_period_usec != 100_000
            || self.max_pids != 128
            || !self.parent_verified
            || (action == "prepare" && !self.process_bound)
        {
            bail!("compute resource service refused or returned an unbound receipt");
        }
        Ok(())
    }
}

fn request(action: &str, endpoint: Option<&str>, project: Option<&str>) -> Result<()> {
    if !enabled()? {
        bail!("compute resource adapter is not configured");
    }
    if !cfg!(target_os = "linux") {
        bail!("hard compute resource control requires Linux");
    }
    let metadata = std::fs::symlink_metadata(SOCKET).context("resource socket unavailable")?;
    if !metadata.file_type().is_socket() || metadata.uid() != 0 || metadata.mode() & 0o007 != 0 {
        bail!("resource socket must be root-owned and inaccessible to other users");
    }
    let mut stream = UnixStream::connect(SOCKET).context("resource service unavailable")?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let message = serde_json::json!({"version": 1, "action": action, "endpoint": endpoint, "project": project});
    serde_json::to_writer(&mut stream, &message)?;
    stream.write_all(b"\n")?;
    // Bound the response before allocation and require a complete receipt.
    let mut bounded = BufReader::new(std::io::Read::take(stream, 1025));
    let mut response = Vec::new();
    bounded.read_until(b'\n', &mut response)?;
    if response.len() > 1024 || response.last() != Some(&b'\n') {
        bail!("invalid bounded resource receipt");
    }
    let receipt: Receipt = serde_json::from_slice(&response)?;
    receipt.validate(action, endpoint)
}

pub fn preflight() -> Result<()> {
    request("preflight", None, None)
}

pub fn admit(project: &str, active: &[String]) -> Result<()> {
    if active.len() >= MAX_ACTIVE
        || active
            .iter()
            .filter(|id| project_for_endpoint(id).ok() == Some(project))
            .count()
            >= MAX_PROJECT_ACTIVE
    {
        bail!("compute capacity is occupied; no process allocated");
    }
    Ok(())
}

pub fn admit_start(project: &str) -> Result<()> {
    if enabled()? {
        project_for_endpoint(&format!("ep-{project}"))?;
        request("admit", None, Some(project))?;
    }
    Ok(())
}

pub fn admit_definition(project: &str, plane: &crate::endpoint::ComputeControlPlane) -> Result<()> {
    if !enabled()? {
        return Ok(());
    }
    if plane.endpoints.len() >= MAX_ENDPOINTS
        || plane
            .endpoints
            .values()
            .filter(|ep| ep.tenant_id.to_string() == project)
            .count()
            >= MAX_PROJECT_ENDPOINTS
    {
        bail!("project compute definition limit reached; no endpoint allocated");
    }
    admit_start(project)
}

/// The host service derives the PID from SO_PEERCRED, then attaches this
/// launcher to a fixed systemd scope before exec. No caller-selected PID/command.
pub fn enter(endpoint: &str, _repository: &Path) -> Result<()> {
    let project = project_for_endpoint(endpoint)?;
    request("prepare", Some(endpoint), Some(project))?;
    membership(endpoint, std::process::id() as i32)
}

fn membership(endpoint: &str, pid: i32) -> Result<()> {
    if pid < 2 {
        bail!("invalid bound compute process");
    }
    let unit = format!("briven-compute-{endpoint}.scope");
    let content = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    // Private cgroup namespaces can render the sibling scope with /../.
    // Require the complete final unit component, never a substring.
    if !content.lines().any(|line| {
        line.strip_prefix("0::").is_some_and(|path| {
            Path::new(path).file_name().and_then(|v| v.to_str()) == Some(unit.as_str())
        })
    }) {
        bail!("compute is outside its bound resource scope");
    }
    Ok(())
}

pub fn verify(endpoint: &str, pid: i32) -> Result<()> {
    let project = project_for_endpoint(endpoint)?;
    membership(endpoint, pid)?;
    request("verify", Some(endpoint), Some(project))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn altered_limits_and_unbound_process_receipts_are_rejected() {
        let mut receipt = Receipt {
            version: 1,
            ok: true,
            action: "prepare".into(),
            endpoint: Some("ep-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
            memory_bytes: MEMORY_BYTES,
            cpu_quota_usec: 50_000,
            cpu_period_usec: 100_000,
            max_pids: 128,
            parent_verified: true,
            process_bound: true,
        };
        let endpoint = receipt.endpoint.clone();
        assert!(receipt.validate("prepare", endpoint.as_deref()).is_ok());
        receipt.process_bound = false;
        assert!(receipt.validate("prepare", endpoint.as_deref()).is_err());
        receipt.process_bound = true;
        receipt.memory_bytes += 1;
        assert!(receipt.validate("prepare", endpoint.as_deref()).is_err());
        receipt.memory_bytes = MEMORY_BYTES;
        assert!(
            receipt
                .validate("prepare", Some("ep-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"))
                .is_err()
        );
        assert!(receipt.validate("verify", endpoint.as_deref()).is_err());
    }
    #[test]
    fn resource_names_cannot_escape_or_claim_another_project() {
        let project = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert_eq!(
            project_for_endpoint(&format!("ep-{project}-rc-123")).unwrap(),
            project
        );
        for name in [
            "ep-../other",
            "--exec",
            "ep-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/other",
            "ep-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-",
            "ep-éaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            assert!(project_for_endpoint(name).is_err());
        }
    }
    #[test]
    fn full_pool_and_project_capacity_refuse_without_allocating_a_process() {
        let a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert!(admit(a, &[]).is_ok());
        assert!(admit(a, &[format!("ep-{a}"), format!("ep-{a}-branch")]).is_err());
        assert!(admit(b, &[format!("ep-{a}"), format!("ep-{a}-branch")]).is_ok());
        let full = vec![
            format!("ep-{a}"),
            format!("ep-{a}-branch"),
            format!("ep-{b}"),
            format!("ep-{b}-branch"),
        ];
        assert!(admit("cccccccccccccccccccccccccccccccc", &full).is_err());
    }
}
