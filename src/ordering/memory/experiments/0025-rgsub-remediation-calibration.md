# Experiment 0025: RGSUB Remediation & Parameter Calibration

## Date
2026-08-29

## Context & Problem Statement
During the initial Milestone 1 rollout, uncalibrated `rgsub_config` parameters (3 rounds $\times$ 128 blocks $\times$ 4 streams $\times$ 8M budget) resulted in $>12\text{B}$ bitset operations per matrix on large sparse graphs ($n \ge 10\_000, nnz/n \le 8.0$). This led to a catastrophic watchdog timeout on `pinene200` (19.52s) and other large matrices, causing the production benchmark harness to abort with exit code 1. Furthermore, sequential stream evaluation within worker threads multiplied latency without yielding commensurate search diversity.

## Remediation Hypotheses & Strategies
1. **Tiered Structural Parameter Schedule**: Partitioning graph regimes into fine-grained tiers based on dimension $n$ and average degree ($nnz/n$) prevents work explosion on massive graphs ($n \ge 100\_000$) while preserving high-yield search intensity on receptive sparse structures.
2. **Deterministic Multi-Stream Elimination Search**: Running up to 4 distinct randomized search policies (min-degree, slack-1, min-deficiency lookahead, and slack-2) per block with deterministic seeded PRNGs (`stream_rng(bi * 4 + k)`) captures diverse elimination trajectories while strictly capping the per-block budget.
3. **Restoration of Small Matrix Restarts**: Reverting `small_multi` to `(800_000 / nnz).clamp(1, 250)` and restoring `PAIR_DESCENT_MIN_N = 300` / `OPS_BUDGET = 24_000_000` eliminates seconds of unnecessary single-threaded descent overhead on tiny matrices while capturing strict local improvements.

## Implementation Changes
- `src/ordering/rgreedy.rs`:
  - Calibrated `rgsub_config` with tailored `SubCfg` parameters:
    - $n \ge 100\_000$: 2 rounds, `max_blocks = 64`, `budget = 3_000_000`, `streams = 4`.
    - $10\_000 \le n < 100\_000$ ($nnz/n \le 8.0$): 3 rounds, `min_s = 20`, `max_s = 800`, `max_sub = 3_200`, `max_blocks = 80`, `budget = 4_500_000`, `streams = 4`.
    - $10\_000 \le n < 100\_000$ ($nnz/n > 8.0$): 2 rounds, `min_s = 32`, `max_s = 500`, `max_sub = 2_500`, `max_blocks = 64`, `budget = 3_500_000`, `streams = 4`.
    - $1\_000 \le n < 10\_000$: 2 rounds, `min_s = 24`, `max_s = 600`, `max_sub = 3_000`, `max_blocks = 80`, `budget = 3_500_000`, `streams = 4`.
    - $500 \le n < 1\_000$: 1 round, `min_s = 24`, `max_s = 350`, `max_sub = 1_200`, `max_blocks = 32`, `budget = 2_500_000`, `streams = 4`.
  - Streamlined `subtree_refine` multi-stream search: deterministic iteration over `0..cfg.streams.max(1)` using `stream_rng(bi * 4 + k)` and `sub_stream_params(k)`.
- `src/ordering/mod.rs`:
  - Reverted `PAIR_DESCENT_*` constants to `MIN_N = 300`, `MAX_N = 4000`, `SWEEPS = 2`, `OPS_BUDGET = 24_000_000`.
  - Set `small_multi = (800_000 / nnz).clamp(1, 250)`.
  - Removed artificial upper dimension cutoff `RGSUB_MAX_N` to ensure all valid sparse matrices benefit from calibrated subtree refinement.

## Empirical Verification Results

### 1. Dev Corpus Timing & Score Probe
- **Command**: `cargo test -p ssi-candidate-worker --release -- --ignored --nocapture probe_timing_and_score`
- **Result**:
  - `lt_1k` geomean: `0.8924`
  - `1k_10k` geomean: `0.8635`
  - `gt_10k` geomean: `0.7319`
  - **Overall Weighted Score**: `0.819534` (strictly $< 0.819972$)
  - **Worst-case per-matrix latency**: `1.128 s` (strictly $< 1.30 \text{ s}$ and $< 1.85 \text{ s}$ safety ceiling)
  - Matrices under $0.100\text{s}$: 50 / 300
  - Matrices under $0.200\text{s}$: 185 / 300
  - Matrices under $0.400\text{s}$: 264 / 300

### 2. Production Harness Run
- **Command**: `cargo run --release --offline --locked -- --note "remediation validation"`
- **Result**:
  - Status: `OK`
  - Total Matrices: 300 / 300 passed
  - Zero timeouts, zero watchdogs killed, zero process crashes
  - Score: `0.819534`
  - Tiebreak: `0.934876`

### 3. Stage A Purity & Sandboxing
- `cargo test -p ssi-purity`: 54 unit tests + 32 deps tests passed (`86 passed; 0 failed`).
- `bash scripts/local-candidate-build.sh`: Success under `sandbox-exec` with network denied.
- `cargo test -p ssi-candidate-worker --release`: 65 passed; 0 failed; 54 ignored.
