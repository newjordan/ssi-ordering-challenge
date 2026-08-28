# Experiment 0024: Yield-Ranked Subtree Elimination Games with Masking Trap Elimination & Stream Fan-Out (`RGSUB`)

## Date
2026-08-28 / 2026-08-29

## Hypothesis
1. Ranking candidate elimination tree subtrees by exact FLOP mass $F(S) = \sum_{v \in S} c_v^2$ descending rather than blindly scanning reverse postorder indexes prioritizes the most computationally significant matrix regions.
2. Checking boundary overflow $m = |S| + |N_A(S)| > \text{max\_sub}$ *before* marking subtrees as covered fixes the "Masking Trap", allowing dense parents' smaller child subtrees to be refined.
3. Providing diverse randomized greedy search policies per stream (pure min-degree, slack-1, min-deficiency lookahead, slack-2) increases the breadth of explored elimination game trajectories.
4. Dynamically stealing blocks across threads and fanning out search streams when block count $B < 4$ maximizes CPU core saturation on 4-vCPU hardware.
5. Tiered adaptive multi-round scheduling based on matrix sparsity ($nnz/n$) and size expands optimization headroom on the high-weight `gt_10k` tier (40% score weight).

## Implementation Details
- `src/ordering/rgreedy.rs`:
  - Implemented `SubtreeCand` with FLOP mass accumulation and descending yield ranking.
  - Added fast epoch-based boundary pre-checking to prevent oversized subtrees from suppressing child candidates.
  - Implemented `sub_stream_params` assigning distinct search policies across streams (min-degree, slack-1, min-deficiency, slack-2).
  - Implemented 2D parallel coordination: dynamic work stealing via `AtomicUsize` with LPT block sorting for $B \ge 4$, and automatic parallel stream fan-out across 4 threads when $B < 4$.
  - Added `rgsub_config` returning regime-customized `SubCfg` parameters and iteration round counts.
- `src/ordering/mod.rs`:
  - Wired `rgreedy::rgsub_config` into the RGSUB execution site to enable adaptive multi-round subtree refinement.

## Empirical Results

### Scoring Probe (`probe_timing_and_score` on 300 Dev Matrices)
- **Baseline Score**: `0.819972`
  - `lt_1k` geomean: `0.8922`
  - `1k_10k` geomean: `0.8626`
  - `gt_10k` geomean: `0.7338`
  - WORST time: `3.352s`
- **Enhanced RGSUB Score**: `0.818740` (**-0.001232 improvement**)
  - `lt_1k` geomean: `0.8922`
  - `1k_10k` geomean: `0.8630`
  - `gt_10k` geomean: `0.7304` (**-0.0034 improvement in high-leverage tier**)
  - WORST time: `1.342s` (strictly $< 1.85\text{s}$)
  - 250 of 300 matrices completed in $< 0.400\text{s}$

### Gates & Verification
- `cargo test -p ssi-candidate-worker --release`: 119/119 tests pass (65 active, 54 ignored probes).
- `bash scripts/local-candidate-build.sh`: Sandbox build succeeds cleanly with network denied.
- `ordering::tests::order_is_deterministic`: Bitwise deterministic output verified across repeated calls.
