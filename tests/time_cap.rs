//! End-to-end: the harness enforces the per-matrix time cap by killing a slow
//! order() worker, and a normal run over the sample corpus still succeeds.
//! Drives the release binary the way a contestant does.

use std::process::Command;
use std::time::{Duration, Instant};

fn harness_bin() -> std::path::PathBuf {
    // The integration test binary runs from target/<profile>/deps; the harness
    // binary is two levels up at target/<profile>/ssi-ordering-challenge.
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // deps
    p.pop(); // profile dir
    p.push("ssi-ordering-challenge");
    p
}

fn candidate_bin() -> std::path::PathBuf {
    let mut path = harness_bin();
    path.set_file_name(format!(
        "ssi-candidate-worker{}",
        std::env::consts::EXE_SUFFIX
    ));
    path
}

#[test]
fn normal_run_succeeds_on_sample_corpus() {
    let corpus =
        std::env::temp_dir().join(format!("ssi-time-cap-corpus-{}.jsonl", std::process::id()));
    std::fs::write(
        &corpus,
        "{\"n\":2,\"nnz\":4,\"indptr\":[0,2,4],\"indices\":[0,1,0,1],\"hash\":\"time-cap\",\"source\":\"time-cap\"}\n",
    )
    .unwrap();
    let out = Command::new(harness_bin())
        .env("SSI_CANDIDATE_WORKER", candidate_bin())
        .env("SSI_CORPUS_FILE", corpus)
        .args(["--note", "time_cap test: normal"])
        .output()
        .expect("run harness");
    assert!(
        out.status.success(),
        "expected success, got {:?}\nstdout:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn slow_ordering_is_killed_and_fails_promptly() {
    // Force every order() call to sleep 30s; the 2s cap must kill it and FAIL
    // the run in well under 30s.
    let start = Instant::now();
    let out = Command::new(harness_bin())
        .env("SSI_CANDIDATE_WORKER", candidate_bin())
        .env("SSI_TEST_SLEEP_MS", "30000")
        .args(["--note", "time_cap test: slow"])
        .output()
        .expect("run harness");
    let elapsed = start.elapsed();

    assert!(!out.status.success(), "slow ordering should FAIL the run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("time cap")
            || stdout.contains("per-matrix cap")
            || stdout.contains("RUN FAILED"),
        "expected a time-cap failure message, got:\n{stdout}"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "cap was not enforced promptly: {elapsed:?}"
    );
}
