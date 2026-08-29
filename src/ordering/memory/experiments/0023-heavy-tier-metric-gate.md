# 0023 — the heavy tier's biggest gap was a gate inherited from a SELECTION sweep

**Session:** `arm/heavy` (2026-08-28). **Score:** 0.839987 → **0.835144** on 4
threads (−0.004843), gt_10k 0.773528 → 0.761421. Sequentially against the
pre-parallel frontier the same work measured 0.842223 → 0.838862.

## The question

The heavy tier — gt_10k with `nnz ≥ 250k` or `n ≥ 100k` — is the highest-leverage
class on the board (0.00689 of score per unit ln-ratio *each*) and the frontier's
`(n, nnz)` gates exclude it from almost every candidate. Four of its matrices tie
at *exactly* the AMD flop count. What, if anything, moves them?

## Method

`probe_heavy_census`: run **every candidate family the crate exposes, ALONE**, on
each of the 15 heavy matrices, and record the exact ratio *and* the isolated
seconds. 22 AMD variants (`aggressive` × 11 `dense_alpha`), 11 AMF alphas, ~35
METIS configs, ~13 SCOTCH, 12 KaHIP, `nd_order`/`ndfm_order`/`biconn`/
`biconn_heavy`/RCM/Sloan, 16 relabelled-AMD seeds, 8 relabelled-AMF, 8
relabelled-no-dense-AMD. Then `probe_heavy_structure` (degree distribution, hubs,
components, bipartiteness), `probe_heavy_peel` (explicit dense-row peel with AMD
or METIS on the sparse core), and `probe_heavy_metrics` — the one that mattered.

All A/B measurement went through `probe_heavy_ab`: the whole corpus, arm ON and
arm OFF **in the same run, interleaved, min of 2**, driven by a test-only
`HEAVY_ARM` thread-local (a `const fn` returning `true` in a non-test build). On
a box shared with four other agents, cross-run timing comparison is worthless —
untouched matrices show ±0.07 s between the two arms of the *same* run.

## The finding

`custom_metrics::ScoreVariant` (3 specs) and `metric_sweep::EXTRA_METRICS` (15
specs) — **22 orderings, the largest candidate family in the file** — were both
gated at `nnz < 130_000`. That bound came from the sweep that **selected** them:
`sbx-metrics2` ran its greedy forward-selection on `nnz<130_000`-gated matrices
*only*. It was never a measurement that they are useless or unaffordable above
it. Run alone on the heavy tier they are the largest source of headroom found
anywhere in this arm:

| matrix | nnz | incumbent → this candidate |
|---|---|---|
| `pooling_sppc3pq` | 893 724 | 0.6818 → **0.5338** `extra_deg2_div_nv_wf05` |
| `crudeoil_pooling_dt3` | 152 210 | 0.9541 → **0.7538** `SqPure` α=5 |
| `faclay35` | 132 380 | 0.9997 → **0.9523** `extra_deg_div_nv_wf2` |
| `cont6-qq` | 557 994 | 0.8484 → **0.7983** `extra_deg2_div_nv_wf002` |
| `gabriel10` | 1 148 210 | 0.9793 → **0.9480** `extra_deg2_div_nv_wf05` |
| `pooling_sppb5pq` | 674 470 | 0.4100 → **0.4077** `SqDiv` |

**Generalisable rule: when a gate constant's provenance is the sweep that chose
the candidate, it bounds the EVIDENCE, not the candidate.** Audit every gate for
which "we measured it is unsafe/useless above X" and "we only ever looked below
X" have been conflated.

## The self-correction inside it

My first census swept `dense_alpha` ∈ {10, 5, −1} on the 15 heavy targets, found
all three **byte-identical on every one**, and concluded α was inert. It is not.
Widening to α ∈ {10, 5, 2.5, 2, 1, 0.5, −1} over the whole *in-band* set (every
matrix with 130k ≤ nnz ≤ 1.2M, not just the heavy targets) showed:

* `crudeoil_pooling_dt3`: `SqPure` α=10 → 0.8932, α=**5** → **0.7538**
* `pooling_sppc3pq`: `..wf05` α=10 → 0.5338, α=**2.5** → **0.4846**

The α that matters is *below* the 5..10 the first census sampled — exactly where
AMD's `max(16, α√n)` threshold starts reaching the real hub classes of these
patterns. **A parameter that is provably inert on a structurally homogeneous
subsample is not inert.** `HEAVY_METRIC_ORDER` is therefore a `(spec, α)` list in
which the same spec appears twice at two α values on purpose.

Saturation check afterwards: all 22 specs × 10 α values × `aggressive` on/off
over the whole in-band set yields exactly **one** further improvement anywhere
(`faclay35` 0.9523 → 0.9522). `aggressive = false` never wins anything the `true`
variants do not already reach. The family is now exhausted at these gates.

## Two smaller shipped wins

**Heavy relabelled-AMF multistart.** The shipped relabelled-AMF block stops at
nnz 130k, and nothing else gives a heavy matrix a randomized restart unless it is
sparse enough for `heavy_multi` (AMD-only) — so the **dense** heavy tier got
**zero** restarts while AMF is the family that wins there. Sparse sub-tier
(nnz ≤ 6n, 130k–400k) at α=5, dense sub-tier (nnz > 6n, 400k–700k) at α=2.5,
`clamp(1.5e6/nnz, 1, 4)` passes. `transswitch2736spr` 0.9886 → **0.9249** at seed
2; `pooling_sppc1pq` 0.3084 → 0.3037.

**Very-heavy no-dense AMD**, `HEAVY_AMDND` ceiling 700k → 1.5M behind a hub gate
`max_deg·2000 ≤ n`. The cost of `aggressive=false, dense_alpha=−1` is set by the
**largest hub, not by nnz**: 0.070 s on `gabriel10` (max_deg 54) against 0.330 s
on `faclay75` (max_deg 2777) at a *smaller* nnz per vertex. Same discriminator
and same mechanism as `mid_multi`'s `max_deg ≤ n/50` in 0019.

## Negative results

* **Library partitioners are hopeless above the existing gates.** Full option
  grids for METIS, SCOTCH and KaHIP on all 15 heavy matrices: best result is
  **1.4× to 20× worse than plain AMD**, never beats the incumbent on one. SCOTCH
  19.3× on `pooling_sppc3pq`; METIS 2.23× on `faclay75` at 8.3 s; KaHIP-Strong
  2.70× on `acopf` at 6.2 s. Corroborates 0009 on a far larger grid.
* **Explicit dense-row peel does not exist as a win.** Threshold `t·mean_deg`,
  t ∈ {1.5 … 40}, AMD *or* METIS on the sparse core, hubs appended: every heavy
  matrix's best peel is exactly 1.0000 (only when the peel set is empty) or far
  worse — `faclay75` 1.9654 at every threshold, `gabriel10` up to 889×, `acopf`
  up to 9515×. AMD's own α grid is not leaving anything below its `max(16, α√n)`
  floor.
* **`faclay75`, `kissing2`, `nuclear104` are AMD-RIGID.** They tie at exactly the
  AMD flop count and stay there under everything listed under Method *plus* all
  22 metric variants × 7 α × `aggressive` on/off. For `faclay75` all 16
  relabellings return the *identical* flop count. Structurally all three are one
  component, non-bipartite, with a few hundred hubs at degree 120–300 over a
  large low-degree mass. Do not spend more here.
* **Relabelling the WINNING α does not help.** Relabel multistart × full α grid ×
  {AMF, AMD, non-aggressive AMD} = 33 configs × 8 seeds on the 9 heavy matrices
  with no win: **zero** beat the incumbent.
* **TELOS on `kissing2` is not worth 1024 nnz.** It misses `TELOS_MAX_NNZ`
  (400 000) by 1024; raising the gate to 420 000 changes its ratio by nothing and
  costs +0.08 s. Reverted.

## Cap safety

Coordinator calibration mid-session moved the target twice (grader ≈1.65× slower
than this box; then a FAIL at 1.24 s local; then a 4-thread stack with local
worst 0.673 s). Final gates confine this arm to ≤ 0.433 s on any matrix and to
**one** queued AMD-class pass above nnz 700k, where the grader's worst hidden
position now lives: `HEAVY_METRIC_GIANT_MIN_NNZ = 700_000` (one variant above
it) and `HEAVY_RELABEL_AMF_DENSE_MAX_NNZ` cut 1M → 700k, the latter free because
the pass it removes on `pooling_sppc3pq` (0.7050) is dominated by the metric
block's 0.5338. `pooling_sppc3pq` ends up **faster** with the arm on than off.

## Postscript — a gate that survived three cost-trims because nobody re-asked

The `HEAVY_AMDND` ceiling extension (700k → 1.5M behind `max_deg·2000 ≤ n`)
existed for exactly one matrix, `gabriel10`, which it took 0.9793 → 0.9756. When
the metric block later reached **0.9480** on the same matrix, the extension
became a pure ~0.07 s AMD-class pass on a 1.15M-nnz graph — and it survived
three separate rounds of cost trimming, because every round asked "is this
affordable?" and none asked "does it still win?". Re-measured directly:
`gabriel10` is 0.9480 with the extension and 0.9480 without it. Removed.

**Rule: when a new family lands, re-check every earlier single-matrix gate it
might have subsumed.** In a best-of portfolio a candidate's value is not a
property of the candidate; it is a property of the candidate *and the current
incumbent*, and it silently goes to zero when something better arrives.

## Final gates and numbers

`stack` 95c48e3, 4 threads, `probe_heavy_ab` (arm on vs off, same run):
**0.833739 → 0.828970 (−0.004769)**, gt_10k 0.761604 → 0.749681, worst
`order()` 0.662 → 0.657 s (`crudeoil_lee4_10`, outside every gate of this arm),
worst matrix inside any gate of this arm 0.468 s.

Four cost trims imposed by the grader's tail, all measured **free on dev**:
metric block dead window 200k–500k; metric block sparse-only below 200k; metric
block capped at 2 variants below 200k with `wf2` capped at 150k; relabelled-AMF
sparse floor 130k → 200k and dense floor 400k → 420k. Above nnz 700k this arm
now contributes **one** queued metric variant and nothing else.
