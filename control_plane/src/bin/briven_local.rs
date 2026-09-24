use std::env;
use std::os::unix::process::CommandExt;
use std::process::{Command, exit};

fn main() {
    let current_exe = env::current_exe().expect("resolve current executable path");
    let bin_dir = current_exe
        .parent()
        .expect("resolve current executable directory");
    let compatibility_binary = bin_dir.join("neon_local");

    let status = Command::new(compatibility_binary)
        .arg0("briven_local")
        .args(env::args_os().skip(1))
        .status()
        .expect("launch Briven local compatibility binary");

    exit(status.code().unwrap_or(1));
}
