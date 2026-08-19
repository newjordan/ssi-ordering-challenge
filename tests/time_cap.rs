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

#[test]
fn normal_run_succeeds_on_sample_corpus() {
    let out = Command::new(harness_bin())
        .args(["--note", "time_cap test: normal"])
        .output()
        .expect("run harness");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "expected success, got {:?}\nstdout:\n{}",
        out.status,
        stdout
    );
    assert!(
        stdout.contains("matrix") && stdout.contains("nnz(A)"),
        "public dev run should retain the per-matrix census:\n{stdout}"
    );
}

#[test]
fn hidden_corpus_omits_per_matrix_census() {
    let dir = std::env::temp_dir().join(format!("ssi-hidden-output-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let corpus = dir.join("hidden.jsonl");
    std::fs::write(
        &corpus,
        concat!(
            r#"{"n":4,"nnz":12,"indptr":[0,3,6,8,12],"indices":"#,
            r#"[0,1,3,0,1,3,2,3,0,1,2,3],"hash":"hidden","#,
            r#""source":"SECRET_MATRIX_CANARY"}"#,
            "\n"
        ),
    )
    .unwrap();

    let out = Command::new(harness_bin())
        .env("SSI_CORPUS_FILE", &corpus)
        .current_dir(&dir)
        .output()
        .expect("run harness against hidden corpus");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "expected hidden-corpus success, got {:?}\nstdout:\n{}",
        out.status,
        stdout
    );
    assert!(!stdout.contains("SECRET_MATRIX_CANARY"), "{stdout}");
    assert!(!stdout.contains("nnz(A)"), "{stdout}");
    assert!(!stdout.contains("flops(base)"), "{stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hidden_corpus_load_failure_omits_raw_name() {
    let dir = std::env::temp_dir().join(format!(
        "ssi-hidden-load-failure-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let corpus = dir.join("malformed-hidden.jsonl");
    std::fs::write(
        &corpus,
        r#"{"source":"SECRET_LOAD_FAILURE_CANARY","n":"not-a-number"}"#,
    )
    .unwrap();

    let out = Command::new(harness_bin())
        .env("SSI_CORPUS_FILE", &corpus)
        .current_dir(&dir)
        .output()
        .expect("run harness against malformed hidden corpus");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "malformed hidden corpus should fail");
    assert!(stdout.contains("failed to load hidden corpus"), "{stdout}");
    assert!(!stdout.contains("SECRET_LOAD_FAILURE_CANARY"), "{stdout}");
    assert!(!stderr.contains("SECRET_LOAD_FAILURE_CANARY"), "{stderr}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn slow_ordering_is_killed_and_fails_promptly() {
    // Force every order() call to sleep 30s; the 2s cap must kill it and FAIL
    // the run in well under 30s.
    let start = Instant::now();
    let out = Command::new(harness_bin())
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
