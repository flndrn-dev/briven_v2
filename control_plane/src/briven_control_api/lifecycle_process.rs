//! Inspect only bound endpoint processes. Refused sockets never prove shutdown.
use std::path::Path;

use control_plane::endpoint::{Endpoint, EndpointStatus};
use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;

use super::lifecycle::Observation;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Presence {
    Alive,
    Absent,
    Unknown,
}

pub(super) struct Processes {
    pub postgres: Presence,
    pub controller: Presence,
}

fn process(path: &Path, endpoint: &Endpoint, controller: bool) -> Presence {
    let contents = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Presence::Absent,
        Err(_) => return Presence::Unknown,
    };
    let Some(pid) = contents
        .lines()
        .next()
        .and_then(|v| v.parse::<i32>().ok())
        .filter(|v| *v > 1)
    else {
        return Presence::Unknown;
    };
    match kill(Pid::from_raw(pid), None) {
        Err(Errno::ESRCH) => return Presence::Absent,
        Err(_) => return Presence::Unknown,
        Ok(()) => {}
    }
    #[cfg(target_os = "linux")]
    {
        let status = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Presence::Absent,
            Err(_) => return Presence::Unknown,
        };
        if status
            .rsplit_once(") ")
            .is_some_and(|(_, tail)| tail.starts_with("Z ") || tail.starts_with("X "))
        {
            return Presence::Absent;
        }
        let command = match std::fs::read(format!("/proc/{pid}/cmdline")) {
            Ok(value) => value,
            Err(_) => return Presence::Unknown,
        };
        let argv = command
            .split(|byte| *byte == 0)
            .filter(|v| !v.is_empty())
            .map(|v| std::str::from_utf8(v).unwrap_or(""))
            .collect::<Vec<_>>();
        let directory = endpoint.pgdata();
        if matches_command(
            &argv,
            directory.to_str().unwrap_or(""),
            endpoint.id(),
            controller,
        ) {
            if !controller
                && (control_plane::compute_resource::enabled().is_err()
                    || (control_plane::compute_resource::enabled().unwrap_or(false)
                        && control_plane::compute_resource::verify(endpoint.id(), pid).is_err()))
            {
                return Presence::Unknown;
            }
            Presence::Alive
        } else {
            Presence::Unknown
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (endpoint, controller);
        Presence::Unknown
    }
}

#[cfg(any(target_os = "linux", test))]
fn matches_command(argv: &[&str], pgdata: &str, endpoint: &str, controller: bool) -> bool {
    let binary = argv
        .first()
        .and_then(|value| Path::new(value).file_name())
        .and_then(|v| v.to_str());
    if controller {
        binary == Some("compute_ctl")
            && argv.windows(2).any(|v| v == ["--pgdata", pgdata])
            && argv.windows(2).any(|v| v == ["--compute-id", endpoint])
    } else {
        binary == Some("postgres") && argv.windows(2).any(|v| v == ["-D", pgdata])
    }
}

impl Processes {
    pub fn inspect(endpoint: &Endpoint) -> Self {
        Self {
            postgres: process(&endpoint.pgdata().join("postmaster.pid"), endpoint, false),
            controller: process(
                &endpoint.endpoint_path().join("compute_ctl.pid"),
                endpoint,
                true,
            ),
        }
    }

    pub fn observed(&self, endpoint: &Endpoint, sql_ready: bool) -> Observation {
        match (self.postgres, self.controller) {
            (Presence::Alive, Presence::Alive) if sql_ready => Observation::Ready,
            (Presence::Alive, Presence::Alive) | (Presence::Absent, Presence::Alive) => {
                Observation::Alive
            }
            (Presence::Absent, Presence::Absent)
                if matches!(
                    endpoint.status(),
                    EndpointStatus::Stopped | EndpointStatus::Crashed
                ) =>
            {
                Observation::Absent
            }
            _ => Observation::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recycled_pid_or_another_endpoint_cannot_match_bound_process() {
        let command = [
            "/usr/local/bin/compute_ctl",
            "--pgdata",
            "/repo/endpoints/ep-a/pgdata",
            "--compute-id",
            "ep-a",
        ];
        assert!(matches_command(
            &command,
            "/repo/endpoints/ep-a/pgdata",
            "ep-a",
            true
        ));
        assert!(!matches_command(
            &command,
            "/repo/endpoints/ep-b/pgdata",
            "ep-b",
            true
        ));
        assert!(!matches_command(
            &["sleep", "100"],
            "/repo/endpoints/ep-a/pgdata",
            "ep-a",
            true
        ));
        assert!(matches_command(
            &[
                "/usr/local/v16/bin/postgres",
                "-D",
                "/repo/endpoints/ep-a/pgdata"
            ],
            "/repo/endpoints/ep-a/pgdata",
            "ep-a",
            false
        ));
        assert!(!matches_command(
            &["postgres", "-D", "/other"],
            "/repo/endpoints/ep-a/pgdata",
            "ep-a",
            false
        ));
    }
}
