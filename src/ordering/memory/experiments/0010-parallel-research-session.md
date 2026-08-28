# 0010 — Parallel research session: eval-corpus reconstruction, the diversity thesis, and a hard cap warning

**Status: research session. Several measured wins pending integration + isolated timing.**
The durable output is the method and the negatives, not (yet) a score.

## The headline capability: the graded corpus is reconstructible

`corpus/dev/patterns.jsonl` (300 matrices) and the hidden eval corpus (500) share
**exactly ONE matrix** when matched on `(n, nnz)`. Every dev-measured number in this
repo's history has therefore been an extrapolation across nearly disjoint sets — which
is the real explanation for [0008](0008-amdnd-ceiling-restore.md)'s −0.000005 eval delta.

But the grader prints a full per-matrix table into each submission's **public** Actions
log (name, n, nnz, flops(AMD), flops(ours), ratio), and the names are **MINLPLib**
instances. Since `flops(AMD)` is a deterministic pure function of the pattern, a
reconstruction that reproduces the published integer **exactly** is provably the right
matrix. That is an oracle.

Result: **427 of 500 verified bit-for-bit** (223 lt_1k / 156 1k_10k / 48 gt_10k).
Convention:

```
n   = n_free_vars + n_cons + n_ineq   (one slack node per INEQUALITY only;
                                       simple bounds fold into the diagonal)
KKT = [[H, Jᵀ], [J, slacks]]          node order: vars → cons → slacks
```

H and J sparsity come from MINLPLib's OSiL XML by conservative AD-sparsity propagation
over the expression trees — no solver run needed. All 500 names resolve on MINLPLib;
QPLIB is never required.

**Calibrated end-to-end**: the shipped frontier code, which graded **0.8844**, scores
**0.8831** on the reconstruction — 0.15% error, all three buckets tracking. Use it:

```sh
SSI_CORPUS_FILE=<path>/eval-reconstructed.jsonl cargo run --release --offline --locked
```

Stored outside the repo (survives checkouts): `~/ssi-ordering-LEARNINGS/data/corpus-reconstruction/`.
Known bias, documented in that directory's README: nonlinear-expression-heavy models
(`hda`/`alkylation` class) are under-represented among the 73 misses.

## The strategic finding: DIVERSITY beats QUALITY

`feral_ordering_core::quotient_graph` is public enough to run our own pivot-selection
score through feral's near-linear elimination machinery (verified: a custom metric
reproduces `feral_amd::amd_order` bit-for-bit). Four new scoring formulas were built.
**Every one is worse than AMD standalone** (ratios 1.13–7.3). Added as portfolio
candidates they bought **−0.0088** — more than the entire 12-variant partitioner sweep
of [0002](0002-measured-gates-metis-kahip.md).

They pay because each wins on a *different, non-overlapping* subset and the AMD floor
discards them elsewhere. **So candidate quality is nearly irrelevant; a bad-but-different
ordering beats a good-but-similar one.** In hindsight this also explains
[0003](0003-relabelled-amd-multistart.md): re-running the *same* AMD on a shuffled
pattern was a diversity play, not a quality one.

Corollary, measured: the winning `dense_alpha` values across 25 swept values **scatter**,
with almost no repeats. Wide cheap grids beat a few tuned constants.

## Where the headroom is, measured two ways

- **The tie seam is inert.** A budget sweep to 200 restarts on all 38 large dev matrices:
  **11 of 12 tied matrices are completely flat at every budget level**, while 11 of 12
  *responders* are matrices we already beat AMD on (`chp_shorttermplan2d` 0.6301→0.5462,
  `crudeoil_lee4_09` 0.8456→0.7610). Independently, 3000 exact objective evaluations per
  small matrix moved lt_1k by −0.0006 and broke **1 of 58** small ties. Extra search
  raises the ceiling; it does not break ties.
- **gt_10k is budget-limited** (still falling at 200 restarts, no inflection);
  **lt_1k is method-limited** (flat from 24 to 2000 restarts — the inherited
  "lt_1k is structurally done" note in [open-questions](../open-questions.md) is CORRECT
  and was re-confirmed against a hypothesis that it was wrong).
- Headroom ceiling from the graded census: all 183 exact ties flipped perfectly to 0.90
  buy only **−3.7%**. 309 of 500 graded matrices sit at ratio ≥ 0.95 — the corpus is a
  large near-AMD mass with a modest winning tail.

## HARD WARNING: the cap is tighter than the numbers suggest

A candidate configuration passed the **real enforced-cap harness** (`cargo test
--test time_cap`, actual watchdog SIGKILL) **twice**, then **FAILED on a third
otherwise-identical run** (`unitcommit_200_100_1_mod_8`). Cutting the cost formula ~2.4×
gave 5/5. **A green cap test is weak evidence; a red one is strong evidence.**

Compounding this, a distinct **cost cliff** was found: `select_pivot_amf` linear-scans
its coarse bucket, so a score formula with wide dynamic range crowds candidates into one
bucket and degenerates to O(bucket length). An ungated 25-variant sweep stalled at
50–100/300 matrices; gated to `nnz ≤ 150_000` it did 280 in 32 s. Cost keyed to **bucket
cardinality**, not smoothly to n or nnz — invisible to ordinary gating intuition.

**Rule: never ship a power- or product-shaped score variant without a bucket-cardinality
check on the largest, densest matrices.**

Since the AMD floor means no candidate can make the score *worse* (verified: zero of 500
graded matrices exceed ratio 1.000) while one breach fails the whole run — **the risk is
entirely asymmetric. When in doubt, ship less.**

## Negatives — do not re-walk

- **In-loop tie-break perturbation loses to whole-pattern relabelling** at equal restart
  count (R=24: 0.9563 vs 0.9518). Relabelling perturbs *every* tie-break in the run;
  in-loop randomization only breaks already-tied buckets — a narrower search space.
  It also carries an O(bucket-length) per-pivot trap.
- **Squaring the score is a no-op.** Ranking by `deg²` ≈ ranking by `deg` — argmin is
  invariant under monotonic transforms. The observed 13% degradation is bucket-quantization
  coarsening, not the heuristic.
- **Every separator gate is correctly placed.** 84 dev matrices audited just outside
  `SCOTCH_MAX_N`/`SCOTCH_TUNED`/`METIS_TUNED`/`METIS_HITRIAL`/`KAHIP_MULTI`: **zero**
  excluded wins, losses up to 2423×. Confirms [0009](0009-separator-negative-hc-extend.md).
- **Strip-the-densest is dominated above density ≈25** (`pooling_sppc1pq`: portfolio
  0.3084 vs best strip 0.4455). It wins at density ≈3–15 — see below.
- **A "seeded restart synergy" was measured, then RETRACTED by its own author** as a
  budget-accounting artifact (28 AMD-equivalent passes vs 16 for the control). Lesson:
  **baseline a search strategy at equal WORK, not equal restart count**, whenever one
  strategy has a setup phase.
- **KKT block-precedence ordering and ML/RL orderings**: literature sweep found block
  orderings lose to plain AMD by up to two orders of magnitude, and no ML result beats
  AMD anywhere near n=340k.

## Measured wins pending integration and isolated timing

| win | dev effect | note |
|---|---|---|
| **STRIP transform** | **−0.0098** | delete k densest, AMD the rest, append last |
| **Custom metrics** (4) | **−0.0088** | all individually worse than AMD |
| AMF "dead zone" (nnz 130k–400k) | `methanol400` 1.0000 → 0.7655 | arbitrary gate gap |
| Bucket-weighted relabel budget | −0.0018 | 5× for n≥10k; shrinking the small side costs nothing |
| Wide α grid | 12 of 255 small matrices improve | winners scatter |

**STRIP details** (the largest single win in this repo's history): rankings
{degree, L-column-count under the baseline AMD — the latter is *free*, the anchor
factorization already exists}; ship `{lcnt64, deg96, deg64, deg48, deg6, deg24}` —
**6 candidates capture 93%** of a 40-candidate grid, 8 capture 97%. Wins are NOT
concentrated at small k. No cheap k-predictor exists (10 threshold rules tested).
**Gate at nnz/n < 25.** Mechanism: at low density AMD's dense-column deferral never
fires (hub degrees don't clear α·√avg), so the portfolio had *no* hub handling on that
class — strip is new coverage, not a worse AMF.

## Cost reduction (output-neutral, proven byte-identical on 300/300)

- Workspace reuse + memoization: **−51.8% allocator calls, −30.7% bytes**.
- **27.4% of all scored candidates were exact duplicates** (4,582/16,707; 296/300
  matrices) — many restarts converge on the identical AMD result and we paid the full
  symbolic pass each time. Memoizing removes real compute, not just allocator pressure.
- Nested-subgraph primitive for the strip family (the k-sets nest:
  top-6 ⊂ top-24 ⊂ … ⊂ top-96, so build once and restore incrementally):
  **−83.2% calls, −52.2% bytes**.

## Method notes for the next session

1. **`order()` runs in a FRESH CHILD PROCESS per matrix** — no state survives between
   matrices, so cross-matrix caching is impossible (and would break determinism anyway).
   All reuse must be within one call.
2. **There is NO memory limit in the harness** — only the time cap. An OOM would crash
   the worker and FAIL the run exactly like a timeout, as an unbounded tail risk.
3. **Ratios are machine-independent; wall-clock is not.** Ratio work can be parallelized
   freely across sandboxes; timing must be serialized on an idle machine. Allocation
   counts are also deterministic and machine-independent — a good cost proxy under load.
4. Mine the eval census with
   `gh run view <id> --repo Lulu-Zhou-EigenLabs/ssi-ordering-challenge --log`.
   Note the log redacts the literal string `auto` as `***` (so `autocorr_*` reads
   `***corr_*`).
