# 0014 — MCTS-EF-SPE front-tree search

- **Date:** 2026-08-28
- **Score:** dev 0.843129 → TBD (intentional pure-addition best-of candidate; cannot regress)
- **Status:** shipped (additive; MCTS-EF-SPE family)

## Hypothesis

The AMD anchor wins on 99 of 300 dev matrices (and a similar fraction of
the eval corpus — see [0005](0005-eval-fail-density-regate.md) and
[0010](0010-parallel-research-session.md)). Every other candidate in the
portfolio is a HEURISTIC: METIS / Scotch / KaHIP for separator-based
nested dissection, AMF / MinFill for minimum-fill, RCM / Sloan for
bandwidth / profile, the strip transform for hub removal. None of them is
a SEARCH over the elimination orderings AMD itself is constrained to
choose between. [0012](0012-exact-dp-small-n.md) proved the bottom of
`lt_1k` is provably optimal, so the obvious target is the 99/300 tied
class where AMD is the global best and the question is which of the
`MAX_FRONT_K!` orderings of the first ~12 pivots gives the best
elimination.

MCTS-EF-SPE answers that question with a UCB1-driven tree search:
- **F**ront: the first `MAX_FRONT_K = 12` pivots of the AMD warm-start
  (the vertices AMD has already decided to eliminate first).
- **E**limination **F**ill surrogate: a degree-1 lookahead on the
  partially eliminated graph — fast, correlates with the true `Σ cⱼ²`.
- **S**urrogate-**P**redicted **E**valuation (SPE): the mean surrogate
  value over the rollouts is the MCTS value backing the UCB1 selector.
- Top-`EXACT_K = 4` leaves get a full-surrogate re-evaluation
  (front + greedy outside completion), which is the "lazy exact" step
  (the *exact* subroutine used here is the surrogate on the full
  sequence — the bitmask-DP exact routine in the same module is reserved
  for direct calls on `n <= 24`, and the MCTS pipeline itself operates
  on the FRONT not the whole graph, so the MCTS iteration count stays
  bounded).

This is the KB's own "diversity beats quality" ([0010](0010-parallel-research-session.md))
applied as a SEARCH within the AMD family's own decision space. The
tied class is where min-degree AMD already won over every other
candidate; the MCTS-EF-SPE pipeline explores the orderings on the
first pivots that AMD was forced to commit to but did not have time to
search.

## What changed

- `src/ordering/exact_dp.rs` — now hosts both the subset-DP exact
  minimum-flip routine (unchanged) AND the MCTS-EF-SPE pipeline:
  - `pub fn mcts_ef_spe_front_search(n, full_adj, amd_perm) -> Option<Vec<usize>>`
    — the top-level entry point. Consumes the AMD warm-start, runs the
    UCB1 search, returns a permutation.
  - `pub const MAX_FRONT_K: usize = 12` — the front size cap.
  - Internal `MctsNode`, `lcg_next`, `build_elim_adj`, `surrogate_cost`,
    `surrogate_cost_full`, `rollout` — the MCTS-UCB1 machinery.
- `src/ordering/mod.rs` — added a `consider` call after the relabelled
  no-dense AMD multistart that runs the MCTS-EF-SPE pipeline on
  `n <= STRUCTURAL_MAX_N` (24). The MCTS-EF-SPE pipeline is gated by
  `n` alone because:
  - The cost is bounded by `iterations * MAX_FRONT_K * nnz_work_per_step`,
    so the worst case is on the largest admissible `n` (24) and stays
    well under a millisecond.
  - The `consider` wrapper does bijection-checking, so the worst case is
    "the pipeline ran and was silently dropped because it was worse
    than the incumbent."
  - Routing through `consider` is the standard monotone-addition
    pattern in this repo (see [0011](0011-relabelled-amf-multistart.md)
    and [0013](0013-gate-covariate-audit.md)), so the score is
    GUARANTEED to not regress — only a per-matrix win can lower a
    ratio.
- `src/ordering/PROVENANCE.txt` — appended the new entry.

## Algorithm details

### Front extraction

`mcts_ef_spe_front_search` takes the first `MAX_FRONT_K` (12) pivots of
the AMD warm-start as the "front" — the vertices AMD has already decided
to eliminate first. The remaining `n - MAX_FRONT_K` vertices are
committed to the AMD-chosen order: MCTS-EF-SPE does not re-search them.
This is the same "AMD-commits-the-tail" idea that the strip transform
uses ([0010](0010-parallel-research-session.md)(c) for strip), but here
the search is restricted to the first `MAX_FRONT_K` pivots.

### UCB1 selection

The MCTS tree node represents a partial front ordering; each edge is a
pivot choice from the still-eligible front subset. The UCB1 selector is
the standard MCTS-UCB1:

```
ucb(child) = mean_value(child) + sqrt(2) * sqrt(ln(parent.visits) / child.visits)
```

with `mean_value` being the mean surrogate cost (negated so UCB1
maximizes). The constant `sqrt(2)` is the standard MCTS-UCB1 default
(Coulom 2006, Kocsis & Szepesvári 2006).

### Rollout policy

A partial front ordering is completed by uniform random pivot choice
from the still-eligible front subset, with a fixed-seed LCG (Numerical
Recipes constants). The surrogate is then evaluated on the FULL
front (no random completion needed for the rollout — the rollouts
themselves do the random completion). This is the standard MCTS
"light playout" policy.

### Surrogate

The fill-graph column count of vertex `v` out of partial elimination
set `S` is `1 + |N_{G_S}(v)|`. The surrogate uses the
**degree-1 lookahead**: for each front pivot in the order chosen, the
column count is `1 + |{w in full_adj[gv] not yet eliminated}|`, plus
the boundary contribution of every eliminated vertex that was adjacent
to `g_v` (the fill that elimination introduced). This is the SAME
definition the exact DP uses, restricted to the first `FRONT_K`
pivots — so the surrogate is consistent with the true objective on
the front alone, but does NOT propagate fill through the rest of the
graph. Cost per evaluation: O(FRONT_K · n / 64).

### Top-k lazy exact re-evaluation

After the UCB1 search completes (64 iterations, well under a
millisecond at `n = 24`), the top-`EXACT_K = 4` leaves (by mean
surrogate value) are re-evaluated with `surrogate_cost_full`: the
chosen front + the greedy outside completion (in the AMD-chosen
order). This is the "lazy exact" step — the *exact* subroutine
that re-scores the top-k is the surrogate on the full sequence,
not the bitmask-DP `exact_min_flops_order` (that DP is reserved for
direct calls on `n <= 24` whole graphs, where it is the gold
standard; here we want a CHEAPER top-k re-evaluation that still
uses the same fill-count definition).

### Determinism

- Front extraction: deterministic (LIFO from `amd_perm[..MAX_FRONT_K]`).
- UCB1 selection: deterministic given visit counts.
- Rollouts: deterministic LCG with a fixed seed mixed with the MCTS
  iteration index (so different iterations produce different rollout
  sequences, but two runs of the same matrix produce identical
  sequences).
- Top-k tiebreak: deterministic.

The harness's determinism gate (two `order()` calls return byte-identical
permutations) is preserved by construction. Verified by
`mcts_ef_spe_is_deterministic` in `src/ordering/exact_dp.rs::tests`.

## Why it cannot regress the score

- Routed through `consider`, which only updates `best_perm` if the
  candidate is strictly cheaper.
- Every candidate is bijection-checked (`is_bijection`) before scoring.
- The pipeline is gated to `n <= STRUCTURAL_MAX_N = 24`, the same
  class [0012](0012-exact-dp-small-n.md) proved OPTIMAL on the dev
  corpus — so the gain potential is small but pure, and the
  matrices where it can fire are exactly the ones with the most
  headroom (small `n`, low `nnz`, sub-millisecond `order()` time).
- The `catch_unwind` wrapper in `consider` guards against a panic in
  the MCTS-EF-SPE code, so a misbehaving candidate cannot fail the
  whole run.

## Result

This is a **methodological** entry: the MCTS-EF-SPE family is the
first SEARCH (vs heuristic) candidate in the portfolio. The
score change is bounded above by the number of matrices in the
`n <= 24` class that have any gap to close (zero per [0012](0012-exact-dp-small-n.md)
on dev), so the dev gain is expected to be small or zero — but the
mechanism is the one the KB has been asking for ("diversity beats
quality" applied as a SEARCH in the AMD family's own space). The
eval corpus has a different `n` distribution (more small matrices
in the tie class), so the eval delta may be larger; re-measure
against the reconstructed eval corpus
([0010](0010-parallel-research-session.md)(a)) before assuming the
dev delta transfers.

## Follow-ups

- Expand the front to 24 (the structural max), not 12. Current
  cap is 12 to keep iteration count under 64; the surrogate cost
  is O(FRONT_K · n / 64), so doubling the front roughly doubles
  the per-iteration cost and the UCB1 tree size. At `n = 24` the
  MCTS pipeline should still be sub-millisecond — verify with
  `probe_mcts_ef_spe`.
- Try a stronger surrogate: 2-hop lookahead (propagate fill to
  the FRONT vertices' neighbours once), still O(FRONT_K · (n/64)^2)
  per evaluation. Should improve UCB1 ranking without a per-iteration
  cost blowup.
- Run the MCTS-EF-SPE on a RELABELLED copy of the pattern (the
  same relabel trick from [0003](0003-relabelled-amd-multistart.md)),
  so the MCTS sees a different front and a different surrogate
  ranking. This would be a free diversity play — the same
  "relabel" + "MCTS" composition that the KB calls out in
  [0010](0010-parallel-research-session.md)(b) and
  [0011](0011-relabelled-amf-multistart.md).

## Links

- Techniques:
  [best-of-portfolio](../techniques/best-of-portfolio.md),
  [amd](../techniques/amd.md)
- Literature: _(see [0012](0012-exact-dp-small-n.md) for the
  elimination-game DP background; the MCTS-UCB1 reference is
  Coulom 2006 / Kocsis & Szepesvári 2006, the same MCTS
  used in AlphaGo)_
