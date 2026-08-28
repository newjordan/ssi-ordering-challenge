# 0006 — Hill-climb phase after the random multi-start

**Result: dev 0.876511 → 0.875359. WIN.** (lt_1k 0.9070 · 1k_10k 0.9069 ·
gt_10k 0.8279; fill tiebreak 0.9621 → 0.9619.)

## Hypothesis (from open-questions "structured relabelings, top lead")

A structured relabeling `Q` (hill-climb: perturb the best permutation so far)
should beat i.i.d. uniform `Q` at identical cost.

## The equal-budget answer is NO

`probe_relabel_schedule` (new plumbing: `order_with_relabel_cap(pat, 0)` runs
the portfolio minus the multi-start so schedules compare at a true equal
budget of R = shipped restarts):

| schedule | score |
|---|---|
| rand seeds 1..=R (shipped) | 0.876511 (reproduces dev exactly — sanity) |
| hc16 (all passes hill-climb) | 0.877547 |
| mix50 | 0.878667 |
| mix75 | 0.878570 |

Random exploration wins while it is still productive. **Open question resolved,
negatively, for the equal-budget form.**

## The surviving positive: exploitation AFTER exploration saturates

`probe_relabel_structured` (R extra passes on top of the full shipped
`order()`): hc16 0.875801, mix 0.874843 — versus 0003's own sweep where the
same doubled spend on MORE RANDOM (600k/48) bought only 0.876130. Past the
0003 knee, hill-climb passes beat random passes.

## Shipped design (`probe_hc_phase`, `probe_hc_bucket_budget` P2)

After every other candidate: perturb the incumbent `best_perm` by random
transpositions (alternating ~n/16 and ~n/4 swaps by pass parity — the
alternating kick beat fixed n/16 slightly and captured chimera_rfr-02), one
AMD pass on `Q A Qᵀ`, compose back, keep if strictly better.

- Gate `nnz <= 60_000` (every measured win but gabriel10 lands below it;
  restart cost tracks nnz).
- **Bucket-weighted budget** (this also answers open question #2):
  `n >= 10_000`: 300k/24 — gt_10k has ~4.4× per-matrix leverage and its gated
  matrices' `order()` time is far under the global worst;
  `n < 10_000`: 100k/8 — keeps the phase to ~0.009 s on `arki0016` (n=7993),
  the measured `order()`-time worst, which is (n,nnz)-indistinguishable from
  winning matrices (avg deg 4.7, nnz 37k ≈ mpbp_07). Uniform 300k/24 scored
  only 0.000184 better (0.875175) and was not worth the arki risk.
- Deterministic: pure functions of `(n, nnz, seed)`; every winner
  bijection-checked.

Timing after shipping: `probe_timing_and_score` WORST order() = 0.431 s
(cool-machine run; ±1.6× noise applies), test suite incl. determinism +
time-cap green.

## Big movers

chp_shorttermplan2d 0.728→**0.630** · chimera_rfr-02 0.771→0.725 ·
chimera_selby-c16-01 (via phase-adjacent noise? re-check) · rocket50
0.996→0.973 · former ties fallen: ndcc12 1.000→0.985 (dev), batchs121208m
1.000→0.993 (probe; harness shows 1.000 — bijection tie-break, verify) ·
gabriel10 stays 1.000 (nnz 1.15M is gated out; still the open heavy-tie lead).

## Caveats

- The n≥10k/300k budget on eval: eval matrices with nnz just under 60k and a
  slow portfolio could add ~0.1 s; local worst combined measured 0.90 s in the
  hot run. Comparative rule satisfied (≤ 1.019 s known-pass).
- Probe1's per-strategy improvements vary run-to-run only via which seeds are
  spent — all shipped paths are deterministic.
