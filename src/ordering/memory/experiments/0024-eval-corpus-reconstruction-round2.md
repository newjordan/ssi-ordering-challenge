# 0024 — Eval-corpus reconstruction, round 2: 500/500 oracle-exact, 0.0002% calibration

**Status: infrastructure, complete.** Round 2 of the reconstruction (method from
[0010](0010-parallel-research-session.md)); tooling persisted OUTSIDE the repo at
`~/ssi-ordering-LEARNINGS/data/corpus-reconstruction/` (README there documents
regeneration for a new round).

## Headline

- **500 of 500** hidden-eval matrices reconstructed and verified **bit-for-bit**
  against the grader's `flops(AMD)` oracle (round 1: 427/500, 73 misses).
- Calibration (deterministic ratio probe on the promoted frontier tree
  `37131d5`): **0.822960 predicted vs 0.822958 graded — 0.0002% error**;
  buckets 0.9128 / 0.8036 / 0.7701 on 243/180/77 (round 1: 0.15% error).
  **Automated confirmation pending a quiet machine:** `scripts/quiet-runs.sh`
  (detached) waits for load < 4, then runs the harness calibration
  (`SSI_CORPUS_FILE` pointed at a COPY — the harness unlinks the path),
  `yukon run` (dev baseline row), and `cargo test --release`; watch
  `~/ssi-ordering-LEARNINGS/data/corpus-reconstruction/scripts/quiet-runs.log`
  and commit the results.tsv row + record the figures below when done.
  Watcher status 2026-08-29 ~05:55 IST: alive (new bash PID 23615, re-launched
  04:35 after the original 75091), still polling — box load 5–10 all night,
  no run fired yet; the deterministic probe calibration above remains the
  primary evidence.
- The frontier tree measures **worst `order()` 0.829 s** on the eval corpus —
  the promoted tree runs ~40% under the local worst case of the round-1 tree.

## What made round 2 work (vs 427/500 in round 1)

The corpus-generation pipeline is *"interior-point solve (pounce) → capture the
KKT pattern"* (`docs/END-TO-END.md`). Three convention classes were derived by
bit-for-bit debugging against the PUBLIC dev corpus (300 ground-truth patterns;
`scripts/validate_dev.py`):

1. **Fixed variables are eliminated.** AMPL writes `lb==ub` variables as
   bound-record **type 4**; the IPM removes them (nodes removed, indices shift
   down, their edges drop). On every mismatching instance the n-overcount
   exactly equals the type-4 count (e.g. `chp_shorttermplan2a` 288/288,
   `arki0012` 2079/2079, `acopf_case9241` would have been 15593/15593).
   **Exception:** three instances (dev `acopf_case9241pegase_qcqp`, eval
   `acopf_case1354pegase_qcqp`, `ex9_2_8`) RETAIN their fixed variables — no
   .nl-local feature separates them (sizes, fixed values, and Jacobian
   participation all overlap with eliminated instances); the census's own n is
   the per-instance discriminator.
2. **Exact AD sparsity, not conservative over-approximation:** unary minus
   (op 16) adds NO pairs (Hessian of −u is −H(u)); `x^1.0` is the identity;
   division adds denominator-support pairs ONLY for affine denominators
   (`c·x/(a+b)` needs the 18 (a,b) pairs on `ex1244`, while `chp_partload`'s
   `pow−pow` denominators must NOT get them). Validation: op-rule ablation is
   100% in-truth on everything dev exercises (mult-cross 1,047,842 pairs,
   pow² 9,654, log/asin/atan all exact).
3. Dev validation went **227/300 → 298/300** (zero regressions; the only
   remaining miss is `chp_partload`, 4 edges short — structure dev never
   exercises enough to arbitrate the rule).

Oracle chain per round: mine census → `build_eval.py` (both pipelines) →
`probe_amd_census` (new test-only probe; grader-exact `feral_amd::amd_order`
pinned by Cargo.lock, feral-amd 0.2.1) → `compare_census.py` arbitrating
bit-for-bit per instance.

## ⚠️ Census provenance risk for FUTURE rounds

The grader STOPPED printing the per-matrix table after **2026-08-22** (the
score.json artifact carries only aggregates; verified on every run 8/24 → 8/29).
This round's census was mined from run `32548789639` (2026-08-22). The corpus
itself is UNCHANGED since (proven by this calibration), but if the corpus ever
rotates, per-matrix reconstruction from logs is currently IMPOSSIBLE until the
grader restores the table — the README documents the check to run first.

## Re-baseline (frontier moved during and after the hypothesis mine)

- Leaderboard: 0.833396 (`5141ad0`, round-1 baseline) → 0.828335 (newjordan
  `1c8eb4e`) → 0.824774 (`65329e3`) → 0.824406 (`f39f03d`) → **0.822958
  (pepedesigner `a4df1b9`/`37131d5`, 2026-08-29 00:13 IST)**.
- `results.tsv` harness rows committed, orchestrator-approved NORMAL
  `yukon sync` run (checkout now `37131d5`), non-editable files unchanged
  (no setup rerun needed). Mission memory additions re-applied on top of the
  promoted tree's memory/, with one CORRECTION: of the 36 bank hypotheses
  carrying memory-conflict annotations, only **11** reference the six recorded
  dead ends (0012/0014/0016/0017/0018/0020); the other **25** reference other
  experiment pages (0002, 0005-0010, 0015) as regime arguments. None were
  selected. (Supersedes the 2026-08-28 log line's "36 dead-end" claim.)
- New dev baseline: probe **0.820888** on the promoted tree; `yukon run`
  confirmation pending a quiet machine (recorded in results.tsv when done).

## Fresh eval target lists (frontier tree on the reconstructed corpus)

- **Ties (ratio == 1.000): 134 total — 96 lt_1k / 24 1k_10k / 14 gt_10k**
  (`eval-tie-list.txt`). Round-1 tie list is obsolete.
- **gt_10k ties** (highest leverage, weight 0.40): `dtoc5` (n=149997),
  `faclay60`, `faclay70`, `junkturn` (n=339994), …
- **Responders** (`eval-responder-list.tsv`, by ln-ratio): `truck` 0.0690,
  `pooling_sppb0pq` 0.0733, `pooling_sppc0pq` 0.0782, `pooling_sppa5tp`
  0.1522, `pooling_sppc0tp` 0.2092 — the pooling/truck families carry the
  biggest wins; 1k_10k geomean 0.8036 is the weakest bucket per matrix.
- ⚠️ The ties remain the floor-limited class (0012/0016/0018/0023); the lists
  above are for REALLOCATION and blind-spot targeting, not tie-hunting.

## Bank re-priced against the smaller gap

25 selected hypotheses re-priced vs `37131d5` (in
`{missionDir}/hypotheses/hypotheses.json`, `repricing_2026_08_29` per
hypothesis): **14 KEEP / 9 DISCOUNT / 2 SUPERSEDED**. Top remaining
opportunities (EV × plausibility × NOT-already-shipped):

1. **H-109** partial-sum abort in portfolio GNP scoring (bound proven in
   `rgreedy::Game::run`; `scoring_ws::column_counts_gnp_flops` still pays full).
2. **H-128** wire `nested_subgraph` into sv2inc (byte-identical proof already
   in-repo; 0.585 s on one matrix = pure cap margin under the 0021 rule).
3. **H-005** tie-prediction early exit (0023 rigidity findings strengthen it).
4. **H-117/H-118/H-124/H-129** STRIP extension cluster (recursive STRIP,
   quotient-tail placement, biconn-aware, anchor-count interleaving).
5. **H-045** Hessian-block-first ordering (the heavy-KKT blind spot 0023 did
   not close). Superseded: H-114, H-016 (`flops_cache` shipped).
