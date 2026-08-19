//! End-to-end: the harness enforces the per-matrix time cap by killing a slow
//! order() worker, and a normal run over a deterministic small fixture succeeds.
//! Drives the release binary the way a contestant does.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

fn harness_bin() -> std::path::PathBuf {
    // The integration test binary runs from target/<profile>/deps; the harness
    // binary is two levels up at target/<profile>/ssi-ordering-challenge.
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // deps
    p.pop(); // profile dir
    p.push("ssi-ordering-challenge");
    p
}

struct TempCorpus(PathBuf);

impl TempCorpus {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempCorpus {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn small_corpus_fixture() -> TempCorpus {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/dev/patterns.jsonl");
    let mut reader = BufReader::new(File::open(&source).expect("open dev corpus"));
    let mut first = String::new();
    assert!(reader.read_line(&mut first).expect("read first matrix") > 0);
    let n = first
        .strip_prefix("{\"n\":")
        .and_then(|rest| rest.split(',').next())
        .and_then(|n| n.parse::<usize>().ok())
        .expect("first corpus line starts with a numeric n");
    assert!(n < 1_000, "first corpus matrix must stay small, got n={n}");

    loop {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ssi-time-cap-small-{}-{id}.jsonl",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let corpus = TempCorpus(path);
                file.write_all(first.as_bytes())
                    .expect("write small corpus fixture");
                return corpus;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create small corpus fixture: {error}"),
        }
    }
}

#[test]
fn normal_run_succeeds_on_small_corpus() {
    let corpus = small_corpus_fixture();
    let out = Command::new(harness_bin())
        .env("SSI_CORPUS_FILE", corpus.path())
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
    // Select a harness-owned worker that sleeps 30s without calling order().
    // The 2s cap must kill the real worker process and FAIL the run well under
    // 30s, while contestant code gets no environment or argv test seam.
    let start = Instant::now();
    let out = Command::new(harness_bin())
        .arg("--test-time-cap")
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
