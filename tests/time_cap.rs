//! End-to-end: the harness enforces the per-matrix time cap by killing a slow
//! order() worker, and a normal run over the sample corpus still succeeds.
//! Drives the release binary the way a contestant does.
//!
//! These tests exercise corpus handling, the census redaction, and the time
//! cap — not the worker sandbox, which has its own coverage (the `sandbox`
//! unit tests, `--grader --sandbox-*-check`, and the Linux/macOS sandbox CI).
//! Local runs now sandbox order() by default (issue #47), so each invocation
//! sets `SSI_ALLOW_UNSANDBOXED_WORKER=1` to run the trusted candidate directly
//! and stay hermetic on runners without a native sandbox installed.

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

fn harness_bin() -> PathBuf {
    // Cargo 1.100 places integration-test executables under
    // target/<profile>/build/<pkg>/<hash>/out, not target/<profile>/deps.
    // CARGO_BIN_EXE is the harness binary cargo actually produced.
    PathBuf::from(env!("CARGO_BIN_EXE_ssi-ordering-challenge"))
}

fn candidate_bin() -> PathBuf {
    let mut path = harness_bin();
    path.set_file_name(format!(
        "ssi-candidate-worker{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        path.is_file(),
        "candidate worker not built at {}. Run `cargo build --offline --locked -p ssi-candidate-worker --bin ssi-candidate-worker` before these tests.",
        path.display()
    );
    path
}

fn public_dev_corpus() -> PathBuf {
    std::fs::canonicalize(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/dev/patterns.jsonl"),
    )
    .expect("canonicalize public dev corpus")
}

/// The trusted parent publishes score.json/results.tsv at the repo root
/// regardless of its CWD, so concurrent harness runs from different tests
/// would clobber each other's artifacts. Serialize every harness invocation.
fn harness_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn score_json_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("score.json")
}

fn results_tsv_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("results.tsv")
}

/// The harness appends to the repo's `results.tsv` even when its CWD is a temp
/// dir. Integration tests used to leave those rows in the committed log.
struct ArtifactRestore {
    results: Option<Vec<u8>>,
}

impl ArtifactRestore {
    fn capture() -> Self {
        Self {
            results: std::fs::read(results_tsv_path()).ok(),
        }
    }
}

impl Drop for ArtifactRestore {
    fn drop(&mut self) {
        match &self.results {
            Some(bytes) => {
                let _ = std::fs::write(results_tsv_path(), bytes);
            }
            None => {
                let _ = std::fs::remove_file(results_tsv_path());
            }
        }
        let _ = std::fs::remove_file(score_json_path());
    }
}

fn dev_corpus_is_lfs_pointer() -> bool {
    use std::io::Read as _;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/dev/patterns.jsonl");
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 80];
    let n = file.read(&mut buf).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).starts_with("version https://git-lfs.github.com/spec/")
}

fn top_level_score(json: &str) -> f64 {
    let (_, tail) = json
        .split_once("\"score\":")
        .expect("top-level score field");
    let end = tail
        .find(|c: char| c == ',' || c == '}')
        .expect("score field terminator");
    tail[..end].trim().parse().expect("numeric top-level score")
}

#[test]
fn normal_run_succeeds_on_sample_corpus() {
    let _serialize = harness_lock();
    let _restore = ArtifactRestore::capture();
    if dev_corpus_is_lfs_pointer() {
        eprintln!(
            "skipping normal_run_succeeds_on_sample_corpus: corpus/dev/patterns.jsonl is a Git LFS pointer"
        );
        return;
    }
    let dir = std::env::temp_dir().join(format!("ssi-public-output-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let corpus = public_dev_corpus();

    // This test verifies the successful public-dev OUTPUT FORMAT (per-matrix
    // census + score.json schema), not order() speed — cap enforcement is
    // covered deterministically by `slow_ordering_is_killed_and_fails_promptly`.
    // Bound the scored set to small matrices (SSI_MAX_MATRIX_N) so the format
    // assertions never depend on the heavy portfolio beating the 2s wall-clock
    // cap on the largest matrices, which is runner-speed flaky (issue #60). The
    // run stays on the public-dev path (same corpus file), so the census and
    // bucket keys are still emitted; only the huge matrices are skipped.
    let out = Command::new(harness_bin())
        .env("SSI_CANDIDATE_WORKER", candidate_bin())
        .env("SSI_ALLOW_UNSANDBOXED_WORKER", "1")
        .env("SSI_CORPUS_FILE", &corpus)
        .env("SSI_MAX_MATRIX_N", "2000")
        .current_dir(&dir)
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
    assert!(
        !stdout.contains("(capped)"),
        "time column should be measured wall clock, not a placeholder:\n{stdout}"
    );
    assert!(
        stdout.contains("score.json"),
        "a successful run should name the score file:\n{stdout}"
    );
    assert!(
        corpus.exists(),
        "absolute public dev corpus path must never be unlinked"
    );
    let score = std::fs::read_to_string(score_json_path()).unwrap();
    assert!(score.contains("\"matrices\""), "{score}");
    assert!(score.contains("\"count\""), "{score}");
    assert!(score.contains("\"weights\""), "{score}");
    assert!(score.contains("\"buckets\""), "{score}");
    for key in ["lt_1k", "1k_10k", "gt_10k"] {
        assert!(stdout.contains(key), "{stdout}");
        assert!(score.contains(key), "{score}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hidden_corpus_omits_per_matrix_census() {
    let _serialize = harness_lock();
    let _restore = ArtifactRestore::capture();
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
        .env("SSI_CANDIDATE_WORKER", candidate_bin())
        .env("SSI_ALLOW_UNSANDBOXED_WORKER", "1")
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
    assert!(!stdout.contains("count"), "{stdout}");
    assert!(!stdout.contains("per-bucket ("), "{stdout}");

    let score = std::fs::read_to_string(score_json_path()).unwrap();
    assert!(!score.contains("\"matrices\""), "{score}");
    assert!(!score.contains("\"count\""), "{score}");
    assert!(!score.contains("\"weights\""), "{score}");
    assert!(!score.contains("\"buckets\""), "{score}");
    for key in ["lt_1k", "1k_10k", "gt_10k"] {
        assert!(!stdout.contains(key), "{stdout}");
        assert!(!score.contains(key), "{score}");
    }
    assert!(score.contains("\"geomean_flop_ratio\""), "{score}");
    assert!(score.contains("\"geomean_fill_ratio\""), "{score}");
    assert!(top_level_score(&score).is_finite(), "{score}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hidden_corpus_load_failure_omits_raw_name() {
    let _serialize = harness_lock();
    let _restore = ArtifactRestore::capture();
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
        .env("SSI_CANDIDATE_WORKER", candidate_bin())
        .env("SSI_ALLOW_UNSANDBOXED_WORKER", "1")
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
    // The parent-only --test-time-cap flag selects a harness-owned worker mode
    // that sleeps 30s without ever calling order(). The 2s cap must kill that
    // real worker process and FAIL the run well under 30s, while contestant
    // code gets no environment or argv test seam.
    //
    // The corpus is a one-line synthetic pattern, not the Git LFS dev file.
    // An unresolved LFS pointer used to fail this test in corpus load, and the
    // assertion still passed because that path also prints "RUN FAILED".
    let _serialize = harness_lock();
    let _restore = ArtifactRestore::capture();
    let dir = std::env::temp_dir().join(format!("ssi-slow-output-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let corpus = dir.join("tiny.jsonl");
    std::fs::write(
        &corpus,
        concat!(
            r#"{"n":3,"nnz":7,"indptr":[0,2,5,7],"indices":[0,1,0,1,2,1,2],"#,
            r#""source":"tiny"}"#,
            "\n"
        ),
    )
    .unwrap();
    let start = Instant::now();
    let out = Command::new(harness_bin())
        .env("SSI_CANDIDATE_WORKER", candidate_bin())
        .env("SSI_ALLOW_UNSANDBOXED_WORKER", "1")
        .env("SSI_CORPUS_FILE", &corpus)
        .current_dir(&dir)
        .args(["--test-time-cap", "--note", "time_cap test: slow"])
        .output()
        .expect("run harness");
    let elapsed = start.elapsed();

    assert!(!out.status.success(), "slow ordering should FAIL the run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("per-matrix cap"),
        "expected a time-cap failure message, got:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "cap was not enforced promptly: {elapsed:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_does_not_need_a_worker_or_corpus() {
    let out = Command::new(harness_bin())
        .arg("--help")
        .output()
        .expect("run harness --help");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--note"));
    assert!(stdout.contains("SSI_CORPUS_FILE"));
    assert!(stdout.contains("AMD"));
}

#[test]
fn unknown_flag_is_rejected_before_scoring() {
    let out = Command::new(harness_bin())
        .arg("--not-a-flag")
        .output()
        .expect("run harness");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown argument"), "{stderr}");
}

#[test]
fn fetch_eval_corpus_script_does_not_count_hidden_lines() {
    let script = include_str!("../.github/scripts/fetch-eval-corpus.sh");
    assert!(!script.contains("wc -l"), "{script}");
    assert!(!script.contains("${lines} lines"), "{script}");
}
