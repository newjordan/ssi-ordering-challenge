<!-- 0022 -->
# 0022 — RGREEDY: the small tier was never at its floor, it was only unreachable from AMD

- **Date:** 2026-08-28
- **Score:** 0.842223 → 0.839420 as a standalone arm (−0.002803); re-measured on the
  merged `stack` tip the phase itself is worth **−0.002359** at the shipped budget
- **Status:** **win** — the largest single-phase gain in the `lt_1k`/`1k_10k` tiers
  recorded in this knowledge base, and it contradicts a conclusion three prior
  experiments had converged on

## Hypothesis

[0012](0012-exact-dp-small-n.md), [0016](0016-spectral-fiedler-nd.md) and
[0018](0018-headroom-census.md) had converged on "`lt_1k` is at its floor: AMD is at
or near optimal". But 0018 states its own limit explicitly: *"the deep search
explored relabelled AMD and AMF only … not a proof of optimality — some third
method could still win where 3000 relabelings did not."*

That limit is load-bearing. **`feral_amd` is an approximate minimum-degree code**:
supervariables, aggressive absorption, approximate external degrees, multiple
elimination. Relabelling only perturbs its internal tie-breaks, so 3000 relabel
seeds sample a narrow neighbourhood of "orderings AMD's approximations happen to
produce" — not the space of orderings. The hypothesis: search the **exact**
elimination game with an arbitrary randomized pivot rule instead.

## What changed

New module [`../rgreedy.rs`](../rgreedy.rs), plus a call site at the very END of
`order_with_options` (after PHARMAKOS), so the phase is strictly-improving over
whatever the whole pipeline produced and cannot perturb any earlier phase's seed
incumbent — the non-monotonicity trap [0020](0020-telos-seed-sensitivity.md)
documented for TELOS.

Three ideas do the work.

1. **The objective is free.** In the elimination game the column count charged when
   `v` is eliminated out of `S` is exactly `c(v,S) = 1 + |N_{G_S}(v)|` — the same
   identity [`exact_dp.rs`](../exact_dp.rs) is built on. So a greedy run
   *accumulates `Σ c_j²` as it goes*: there is no separate scoring pass at all, and
   a partial sum that already reaches the incumbent aborts the run mid-flight
   (`Σ c²` is monotone in the prefix). That is what makes tens of thousands of
   EXACT objective evaluations affordable inside the cap. Verified against the
   trusted `column_counts_gnp` path on 181 corpus matrices × 3 permutations
   (`probe_rgreedy_identity`), and every accepted ordering is re-scored through
   `ScoreWorkspace` anyway.
2. **Randomized exact min-degree.** The fill graph is bitsets (`⌈n/64⌉` words per
   vertex), degrees are exact and maintained incrementally, and the pivot is drawn
   uniformly from `{v : deg(v) ≤ dmin + slack}` over a *ladder* of slacks
   {0,1,2,3,5,8,16} plus one min-deficiency-tie-break policy.
3. **Prefix-freezing ILS with SIDEWAYS acceptance.** Replay a prefix of the current
   order verbatim, re-randomize the suffix. Accept on `f ≤ cur` (equal cost), track
   the global best separately on strict `<`.

Shipped as four INDEPENDENT streams merged by `(flops, stream index)` argmin
(`search_par` / `search_seed`), 200M word-ops each.

## Result

Per-matrix, measured against the SHIPPED `order()` (not against AMD): **64 of 211
in-gate dev matrices strictly improve, 0 regress** (the best-of floor makes
regression impossible). Bucket geomeans `lt_1k` 0.899432 → 0.8937, `1k_10k`
0.875342 → 0.8724.

The wins are **family-wide, not one-offs** — which is what matters, since the
graded corpus is a different draw from the same families:

| family | examples (r_before → r_after) |
|---|---|
| `multiplants_*` | stg1 0.8996→0.8529, mtg1b 0.7044→0.6918, stg1b 0.6303→0.6274, stg5 0.4300→0.4274 |
| `waterund*` | waterund11 0.7998→0.7553, waterund14 0.4054→0.3819 |
| `chimera_*` | mgw-c8-439-onc8-001 0.8206→0.7883, selby-c16-01 0.6578→0.6577, lga-01 0.8034→0.7947, mgw-c16-2031-01 0.8994→0.8937 |
| `graphpart_clique-*` | -20 0.8130→0.7985, -30 0.8085→0.7950, -70 0.8058→0.7936 |
| `sonet*` / `sonetgr*` | sonet24v5 0.5977→0.5932, sonetgr17 0.8210→0.8190 |
| `pooling_*` | digabel19 0.8955→0.8896, sppa0pq 0.4024→0.3978, sppa9pq 0.6511→0.6350 |
| `rsyn0*`, `syn*m`, `blend7*`, `space25*`, `ndcc*`, `qspp_*` | 0.2–2.4% each |
| singles | maxcsp-langford-3-11 0.3900→**0.3347**, ndcc13 0.7053→**0.6392**, nuclear25a 0.6479→**0.5882**, risk2bpb 0.5906→0.5756, lop97icx 0.8816→0.8673 |

## Why it won

**Every win is on a matrix the portfolio ALREADY beat AMD on.** The tie census is
the sharpest statement of this: 55 `lt_1k` matrices sit at exactly 1.0000 before
rgreedy; rgreedy breaks **exactly one**, and 5× the budget with a 4-stream fan-out
(20× compute) breaks **no more**.

So 0012/0016/0018 were right about what they measured and wrong about what it
implied. The ties genuinely are at AMD's floor. But "the near-AMD mass is at its
floor" was silently read as "the small tier is finished", and the small tier's
score does not live in the ties — it lives in the ~90 matrices where the portfolio
already wins, and those had 2–7% left in them that no amount of relabelling reaches
because **relabelling cannot leave AMD's approximation class.**

Two mechanisms inside the search, both measured:

- **Sideways acceptance is worth 6% of the phase** (−0.002434 → −0.002580 at equal
  budget). `Σ c²` is massively degenerate — huge plateaus of equal-cost orders
  differing only in elimination-tree postorder — so a strict-descent walk freezes
  on the first plateau it lands in.
- **Operator diversity beats operator quality.** The LNS prefix draw alternates
  uniform-over-`0..n` with log-uniform-tail. Uniform alone: −0.00208 / 45 matrices.
  Log-uniform alone: −0.00187 / 59 matrices. Alternating: −0.00258 / 63. The two
  extremes win on *different* matrices. Same pattern for the slack ladder: the best
  single slack is never the best portfolio.

## Negative results (do not retry these)

- **Threshold accepting** (walk may move to within 0.1 / 0.5 / 2 / 5% of the global
  best): −0.00213 / −0.00231 / −0.00234 / −0.00208, all WORSE than pure sideways
  (−0.00258). This landscape needs plateau drift, not uphill moves.
- **ILS restarts** from a fresh randomized greedy after stalling: catastrophic at
  stall limit 1 (−0.00104), never reached at limits ≥ 30. Never evict the walk.
- **A population of 2 / 4 / 8 concurrent walks** in one stream: a wash.
- **More budget is nearly worthless**: 5× (600M → 3000M single-stream) buys 11%.
  Four streams at 200M beat one stream at 300M on BOTH score and wall time.
- **`3000 < n ≤ 10000`**: 13 of 44 matrices improve, worth only −0.00022, at
  0.19–0.41 s of added wall. A greedy run is ATOMIC (it must eliminate all `n`
  vertices to yield an ordering) and costs `O(n² + nnz(L)·⌈n/64⌉), so two or three
  runs already exhaust the budget there. More budget cannot fix it; a
  quotient-graph/supervariable engine could.

## The cap lesson (cost this arm one grader FAIL)

The first shipped version used `while ops < budget` with an ATOMIC unit of work
inside. A single run therefore overshot the whole budget: at `n≈5000` a 150M budget
cost **0.32 s instead of 0.08 s**. Worse, ops-per-run is a *prediction* — a
randomized pivot sequence can generate far more fill than the incumbent's, so on an
unseen matrix one run can cost several times what the last one did. Submission
35ba5f24 FAILED on a hidden matrix for exactly this reason (+0.5 s on every small
matrix on the grader, which is ~2.6× this box on bitset work vs ~1.65× on
AMD-style code — the ratio is NOT uniform across code kinds).

Two fixes, both structural rather than calibrated: the loop reserves the previous
run's measured cost before starting another, and `Game::run` hard-stops mid-run at
1.25× the budget. Added wall time then became p50 0.067 s / p90 0.078 s / MAX
0.094 s across the whole gate. **A budget denominated in a unit you cannot preempt
is not a budget.**

A second, cheaper lesson: building the pristine bitset adjacency per stream made 4
threads take ~4× one thread's wall time at `n≈5000` (allocator + kernel page-zeroing
serialise). Build it once, share it immutably, and the fan-out is free.

## Follow-ups

- The 3k–10k band needs a quotient-graph / supervariable elimination engine, not a
  bigger budget. That is a real implementation, not a tuning pass.
- `parallel::run_perms`'s `PAR_MIN_WORK` gate (`len·nnz < 20_000` → sequential)
  falls back to sequential for a 4-task fan-out whenever `nnz < 5000`, i.e. most of
  `lt_1k`. Any queue that schedules rgreedy seeds needs a per-task work estimate.
- The same elimination-game engine with the same ILS should be tried on `gt_10k`
  SUBGRAPHS (a separator-induced block is small enough), where the weight is 0.40.

## Links
- Refutes the generalisation drawn from [0018](0018-headroom-census.md) and
  [0012](0012-exact-dp-small-n.md) (both remain correct about the ties themselves).
- Placement rationale: [0020](0020-telos-seed-sensitivity.md).
- Grader-vs-local timing ratio: [0021](0021-midband-relabel-reach.md).
