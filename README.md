# SSI Ordering Challenge

Fill-reducing elimination orders for sparse symmetric indefinite matrices.

The trusted harness scores a permutation by the predicted cost of an LDLᵀ
factorization and compares it with [feral](https://github.com/jkitchin/feral)'s
Approximate Minimum Degree (AMD) ordering. Lower is better. AMD is defined to
be **1**. Any bijection is a valid order; a bad one only makes the factor more
expensive.

The license is MIT. See [LICENSE](LICENSE).

## Layout

| Path | Role |
| --- | --- |
| `src/main.rs` | Trusted harness: purity gate, two timed `order()` runs, scoring, `score.json`, `results.tsv` |
| `src/ordering/` | The submission. `order(&Pattern) -> Vec<usize>` and the helpers it uses |
| `candidate-worker/` | Untrusted process that calls `order()`. Built separately from the harness |
| `ssi-scoring/` | The only scorer. Pattern-only symbolic analysis on feral `0.11.0` |
| `ssi-purity/` | Purity, dependency allowlist, and `cargo-deny` gate |
| `ssi-worker-protocol/` | Pattern and permutation files passed to the worker |
| `prototype-oracle/` | Independent Σc² scorer used only by tests |
| `corpus/dev/patterns.jsonl` | Public development corpus (Git LFS) |

`Cargo.toml` and `candidate-worker/Cargo.toml` are generated. Edit
`Cargo.toml.in`, `candidate-worker/Cargo.toml.in`, and `src/ordering/deps.toml`.

## What `order()` does

`src/ordering/` is a portfolio of fill-reducing heuristics. It anchors on
`feral_amd::amd_order` with that crate's default options, which is the same
call the harness uses as the baseline, and keeps another permutation only when
the trusted flop count is strictly lower. The returned vector is a
deterministic bijection of `0..n`: `perm[k]` is the original index eliminated
at step `k`.

Contest rules for editing that directory live in [RULES.md](RULES.md). Notes
under `src/ordering/memory/` are research records. They are not instructions
and they are not read by the harness.

## Requirements

- Rust **1.100.0**, pinned in `rust-toolchain.toml` (`rustup` selects it from that file)
- `cargo-deny` **0.20.2**, the same release the grader runs
- Git LFS, if you want the development corpus blob
- On Linux, `bubblewrap` (`bwrap`) for the default worker sandbox

These versions are pins, not leftovers. `ssi-scoring` depends on
`feral = "=0.11.0"` and `feral-amd` / `feral-ordering-core` at `=0.2.1`.
`src/ordering/deps.toml` may name only the reviewed direct crates at the exact
versions in `ssi-purity` (`feral` 0.11.0 and the `feral-*` 0.2.1 companions).
Changing them changes the score definition or fails the purity gate.

No model weights are fetched. `scripts/prepare-build.sh` vendors the crate
sources locked in `Cargo.lock`.

## Setup

```sh
git lfs install
git lfs pull    # skip only if you do not need corpus/dev/patterns.jsonl
cargo install cargo-deny --version 0.20.2 --locked
bash scripts/prepare-build.sh
cargo build --release -p ssi-ordering-challenge --offline --locked
bash scripts/local-candidate-build.sh
```

`prepare-build.sh` checks `deps.toml`, writes the generated manifests, vendors
the lockfile, and scans the vendor tree. It needs network once. Later
`cargo build` / `cargo run` / `cargo test` use `--offline --locked`.

The candidate worker is untrusted code. Build it with
`scripts/local-candidate-build.sh`, which sandboxes the compile. Do not
`cargo build -p ssi-candidate-worker` or `cargo build --workspace` for a
submission you do not trust. `SSI_ALLOW_UNSANDBOXED_WORKER=1` turns the
runtime sandbox off and prints a warning.

The Git LFS pointer at `corpus/dev/patterns.jsonl` records `size 103879806`.
Until `git lfs pull`, the harness refuses to parse it and says so.

## Usage

```sh
cargo run --release --offline --locked -- --note "what I tried"
cargo run --release --offline --locked -- --help
```

On the public development corpus the harness prints one row per matrix
(`n`, `nnz(A)`, baseline flops, your flops, ratio, time), then the per-bucket
geomeans and the score. `time` is the slower of the two determinism runs, in
wall-clock seconds, not a placeholder.

It writes `score.json` and appends one row to `results.tsv`.

| Variable | Effect |
| --- | --- |
| `SSI_CORPUS_FILE` | Grade this JSONL file instead of `corpus/dev/patterns.jsonl` |
| `SSI_MAX_MATRIX_N` | On the public dev corpus only, keep matrices with `n` at most this value. `0`, blank, and non-integers mean no limit. Ignored for any other corpus path |
| `SSI_CANDIDATE_WORKER` | Path to the candidate binary. Default: next to the harness binary |
| `SSI_ALLOW_UNSANDBOXED_WORKER=1` | Local opt-out of the worker sandbox |

A corpus path that is not the shipped dev file is treated as hidden. Matrix
names and sizes are left out of the failure text, and the per-matrix table is
not printed. The dev file is deleted only when the path is not that file.

## Score

For each matrix the harness builds the elimination tree and the
Gilbert–Ng–Peyton column counts of `L` through `ssi-scoring`. With `c_j` the
count in column `j` including the diagonal:

- `nnz(L) = Σ c_j`
- `flops = Σ c_j²`

Matrices are bucketed by dimension: `lt_1k` (`n < 1000`), `1k_10k`
(`1000 ≤ n < 10000`), `gt_10k` (`n ≥ 10000`), with weights **0.30 / 0.30 /
0.40**. Within a bucket the figure is the geometric mean of `flops(yours) /
flops(AMD)`. The score is the weighted mean of those geomeans. Empty buckets
are left out and the remaining weights are rescaled. The tiebreak is the same
reduction on `nnz(L)`.

A run is rejected, not scored worse, when any matrix fails one of these:

- the purity and license gate
- the permutation is not a bijection of `0..n`
- the two `order()` runs disagree
- either run exceeds the **2 second** wall-clock cap (the worker is killed)
- `order()` panics or the worker exits nonzero

`SSI_MAX_MATRIX_N` changes which matrices are scored. It does not change the
formula. The grader does not set it.

## Tests

```sh
bash scripts/prepare-build.sh
cargo build --offline --locked --workspace --bins
cargo test --offline --locked --workspace
```

`cargo test` on the pinned toolchain builds test harnesses and does not leave
`ssi-candidate-worker` next to the harness binary. The build step does. The
time-cap integration test needs that binary.

That runs the closed-form scorer checks, the independent oracle cross-check,
the purity policy tests, the worker-protocol bounds, the harness CLI and
failure-redaction tests, and a candidate-worker check that small graphs return
a deterministic bijection whose flop count is no worse than AMD.

`cargo test --release --test time_cap normal_run` grades the public dev corpus
up to `n = 2000` and checks the table format. It skips when
`patterns.jsonl` is still an LFS pointer. Ignored tests under `src/ordering/`
are measurement probes; they are not part of CI.

GitHub Actions workflow `.github/workflows/ci.yml` runs this test suite on
pull requests and on pushes to `main`. It does not download the corpus blob.
`.github/workflows/benchmark.yml` is the grader. It is `workflow_dispatch`
only and is the workflow that can read the private eval corpus.

## Purity

Before scoring, the harness scans `src/ordering/` and runs `cargo-deny` against
`deny.toml`. The submission may not contain build scripts, FFI, `#[no_mangle]`,
`#[link]`, proc-macros, `include!`, or `#[path = "..."]`. Declared dependencies
must be the reviewed name/version pairs, permissively licensed, and free of
native-code signals (`*-sys`, a C toolchain build, a shipped blob). A missing
`cargo-deny` fails the run.

The allowlist is `TRUSTED_DIRECT_DEPENDENCIES` in `ssi-purity`. Adding a crate
is a maintainer change to that list and to `Cargo.lock` together. See
[docs/DECISION-crate-policy.md](docs/DECISION-crate-policy.md).
