use std::process::Command;

const CANDIDATE_MARKER: &[u8] = b"ssi-candidate-worker-entrypoint-v1:src/ordering::order";

fn parent_binary() -> &'static str {
    env!("CARGO_BIN_EXE_ssi-ordering-challenge")
}

#[test]
fn trusted_parent_excludes_candidate_entrypoint() {
    let binary = std::fs::read(parent_binary()).expect("read trusted parent binary");
    assert!(
        !binary
            .windows(CANDIDATE_MARKER.len())
            .any(|window| window == CANDIDATE_MARKER),
        "trusted parent contains the candidate entrypoint marker"
    );

    let parent_source = include_str!("../src/main.rs");
    assert!(!parent_source.contains("mod ordering;"));
    assert!(!parent_source.contains("ordering::order"));

    let parent_manifest = include_str!("../Cargo.toml.in");
    assert!(!parent_manifest.contains("GENERATED CANDIDATE DEPS"));
    assert!(!parent_manifest.contains("feral ="));
}

#[test]
fn benchmark_command_rebuilds_candidate_worker() {
    let benchmark_manifest = include_str!("../benchmark.json");
    let expected = r#""benchmarkCommand": ["bash", "-lc", "cargo build --release -p ssi-candidate-worker --offline --locked && cargo run --release"]"#;
    assert!(
        benchmark_manifest.contains(expected),
        "benchmarkCommand must rebuild the candidate worker before running the trusted parent"
    );
}

#[test]
fn grader_mode_fails_when_sandbox_selection_is_missing() {
    let output = Command::new(parent_binary())
        .arg("--grader")
        .env_remove("SSI_GRADER_SANDBOX")
        .env_remove("SSI_CANDIDATE_WORKER")
        .output()
        .expect("run trusted parent");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("SSI_GRADER_SANDBOX must be exactly \"bubblewrap\""),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn missing_local_candidate_reports_build_command() {
    let missing =
        std::env::temp_dir().join(format!("ssi-missing-candidate-{}", std::process::id()));
    let _ = std::fs::remove_file(&missing);
    // Opt out of the default local worker sandbox so this test reaches the
    // candidate-path check regardless of whether the runner has bwrap installed
    // (the sandbox selection now runs before the candidate lookup).
    let output = Command::new(parent_binary())
        .env_remove("SSI_GRADER_SANDBOX")
        .env("SSI_ALLOW_UNSANDBOXED_WORKER", "1")
        .env("SSI_CANDIDATE_WORKER", missing)
        .output()
        .expect("run trusted parent");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("candidate worker not found")
            && stderr.contains("cargo build --release --workspace --offline --locked")
            && stderr.contains("rebuild `ssi-candidate-worker` after ordering edits"),
        "unexpected stderr: {stderr}"
    );
}
