//! Untrusted candidate process.
//!
//! This package is the only package that includes `src/ordering` and the
//! dependencies declared in `src/ordering/deps.toml`. The trusted parent starts
//! this executable inside bubblewrap before any of its Rust initialization or
//! contestant dependency code can run.

#[path = "../../src/ordering/mod.rs"]
mod ordering;

#[cfg(test)]
mod corpus {
    pub fn corpus() -> Vec<(String, crate::Pattern)> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../corpus/dev/patterns.jsonl");
        ssi_scoring::load_corpus_jsonl(&path).expect("load development corpus")
    }
}

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

pub use ssi_scoring::Pattern;

pub const CANDIDATE_ENTRYPOINT_MARKER: &str =
    "ssi-candidate-worker-entrypoint-v1:src/ordering::order";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("--identity") => {
            println!("{CANDIDATE_ENTRYPOINT_MARKER}");
            0
        }
        Some("--sandbox-self-check") => sandbox_self_check(&args[1..]),
        Some("--sandbox-timeout-self-check") => timeout_self_check(&args[1..]),
        Some("--sandbox-timeout-child") => timeout_child(),
        Some("--worker-test-timeout") => test_timeout_worker(),
        _ => run_ordering(&args),
    };
    ExitCode::from(code as u8)
}

fn run_ordering(args: &[String]) -> i32 {
    let (Some(pattern_file), Some(output_file)) = (args.first(), args.get(1)) else {
        eprintln!("usage: ssi-candidate-worker <pattern_file> <output_file>");
        return 2;
    };
    if args.len() != 2 {
        eprintln!("candidate worker expected exactly two paths");
        return 2;
    }

    let pattern = match ssi_worker_protocol::read_pattern(Path::new(pattern_file)) {
        Ok(pattern) => pattern,
        Err(error) => {
            eprintln!("candidate worker failed to read pattern: {error}");
            return 3;
        }
    };
    let permutation = ordering::order(&pattern);
    match ssi_worker_protocol::write_permutation(Path::new(output_file), &permutation) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("candidate worker failed to write permutation: {error}");
            4
        }
    }
}

fn sandbox_self_check(args: &[String]) -> i32 {
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        return 2;
    };
    let check = || -> Result<(), String> {
        if std::process::id() != 1 {
            return Err(format!(
                "worker is PID {}, expected PID 1 in its own namespace",
                std::process::id()
            ));
        }
        // The parent env_clears and bwrap runs with --clearenv, so nothing is
        // inherited. bubblewrap itself then sets PWD to its --chdir target ("/")
        // after clearing; that single variable is injected by the sandbox, not
        // leaked from the parent, so allow it and reject anything else.
        for (name, _) in std::env::vars_os() {
            if name != "PWD" {
                return Err(format!("worker inherited environment variable {name:?}"));
            }
        }
        for hidden in ["/workspace", "/home", "/root", "/tmp"] {
            if Path::new(hidden).exists() {
                return Err(format!("unexpected host path is visible: {hidden}"));
            }
        }
        if std::fs::write(input, b"overwrite").is_ok() {
            return Err("pattern input is writable".to_string());
        }
        for fd in 0..=2 {
            let target = std::fs::read_link(format!("/proc/self/fd/{fd}"))
                .map_err(|error| format!("inspect fd {fd}: {error}"))?;
            if target != Path::new("/dev/null") {
                return Err(format!("fd {fd} is inherited ({})", target.display()));
            }
        }
        let visible_pids = std::fs::read_dir("/proc")
            .map_err(|error| format!("read private /proc: {error}"))?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .chars()
                    .all(|character| character.is_ascii_digit())
            })
            .count();
        if visible_pids != 1 {
            return Err(format!(
                "private /proc exposes {visible_pids} processes, expected one"
            ));
        }
        let routes = std::fs::read_to_string("/proc/net/route")
            .map_err(|error| format!("read network namespace routes: {error}"))?;
        if routes.lines().skip(1).any(|line| {
            line.split_whitespace()
                .nth(1)
                .is_some_and(|destination| destination == "00000000")
        }) {
            return Err("network namespace has a default route".to_string());
        }
        std::fs::write(output, b"ok\n").map_err(|error| format!("write permitted output: {error}"))
    };

    match check() {
        Ok(()) => 0,
        Err(error) => {
            let _ = std::fs::write(output, format!("error: {error}\n"));
            1
        }
    }
}

fn timeout_self_check(args: &[String]) -> i32 {
    let Some(output) = args.get(1) else {
        return 2;
    };
    if std::fs::write(output, b"candidate started\n").is_err() {
        return 3;
    }
    let child = Command::new("/candidate-worker")
        .arg("--sandbox-timeout-child")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if child.is_err() {
        let _ = std::fs::write(output, b"candidate child spawn failed\n");
        return 4;
    }
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

fn timeout_child() -> i32 {
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

/// Harness-owned time-cap regression mode (parent `--test-time-cap` runs):
/// sleep far past the 2 s cap without ever calling `ordering::order()`, so the
/// watchdog is exercised against a real worker while contestant code cannot
/// observe any test flag or environment variable.
fn test_timeout_worker() -> i32 {
    std::thread::sleep(Duration::from_secs(30));
    0
}
