//! Fixed unprivileged resource launcher; exec only the bundled compute_ctl.
use anyhow::{Context, Result, bail};
use std::os::unix::process::CommandExt;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let endpoint = args.next().context("bound endpoint required")?;
    let endpoint = endpoint.to_str().context("endpoint must be ASCII")?;
    control_plane::compute_resource::project_for_endpoint(endpoint)?;
    let remaining = args.collect::<Vec<_>>();
    if remaining.first().is_some_and(|v| v == "--resource-probe") {
        return probe(endpoint, &remaining);
    }
    if remaining.is_empty() {
        bail!("fixed compute arguments required");
    }
    control_plane::compute_resource::enter(endpoint, &control_plane::local_env::base_path())?;
    let binary = std::env::current_exe()?.with_file_name("compute_ctl");
    if !binary.is_file() {
        bail!("bundled compute controller missing");
    }
    Err(std::process::Command::new(binary)
        .args(remaining)
        .exec()
        .into())
}

/// Owner-invoked, fixed stress probes. No database directory or arbitrary child
/// command is accepted, and this path is disabled during normal operation.
fn probe(endpoint: &str, args: &[std::ffi::OsString]) -> Result<()> {
    if std::env::var("BRIVEN_RESOURCE_PROBE_ENABLED").as_deref() != Ok("true")
        || args.len() != 2
        || control_plane::local_env::base_path()
            .join("endpoints")
            .join(endpoint)
            .exists()
    {
        bail!("isolated resource probe is unavailable");
    }
    let mode = args[1]
        .to_str()
        .context("fixed resource probe mode required")?;
    if !matches!(mode, "cpu" | "memory" | "pids") {
        bail!("unsupported resource probe");
    }
    control_plane::compute_resource::enter(endpoint, &control_plane::local_env::base_path())?;
    println!("{{\"probe\":\"{mode}\",\"scopeBound\":true}}");
    std::io::Write::flush(&mut std::io::stdout())?;
    match mode {
        "cpu" => {
            let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while std::time::Instant::now() < until {
                std::hint::spin_loop();
            }
        }
        "memory" => {
            // Touch 768 MiB; the 512 MiB scope must kill this isolated probe.
            let mut allocation = vec![0u8; 768 * 1024 * 1024];
            for page in allocation.chunks_mut(4096) {
                page[0] = 1;
            }
            std::hint::black_box(&allocation);
            bail!("memory hard limit was not enforced");
        }
        "pids" => {
            let mut children = Vec::new();
            let mut refused = false;
            for _ in 0..150 {
                match std::process::Command::new("/bin/sleep").arg("10").spawn() {
                    Ok(child) => children.push(child),
                    Err(_) => {
                        refused = true;
                        break;
                    }
                }
            }
            let created = children.len();
            // Keep the scope alive briefly for the host kernel-counter proof.
            std::thread::sleep(std::time::Duration::from_secs(2));
            for mut child in children {
                let _ = child.kill();
                let _ = child.wait();
            }
            if !refused || created > 127 {
                bail!("process hard limit was not enforced");
            }
            println!("{{\"processCreationRefused\":true,\"children\":{created}}}");
        }
        _ => unreachable!(),
    }
    Ok(())
}
