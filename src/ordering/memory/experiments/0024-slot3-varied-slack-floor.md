# 0024 — diversify one PEP stream on the slack-floor frontier

- **Date:** 2026-08-29
- **Score:** 0.819962 → **0.819852** (−0.000110; full deterministic 300-matrix corpus)
- **Status:** win; network-denied tests and production build pass
- **Exact base:** `698da697d45d2f86bdff9c81b1048442c78c0e42`

## Question and exact frontier

The promoted frontier is no longer the earlier 300M or 600M PEP configuration. It
combines two newer changes: adjacent-pair descent was deepened from two sweeps / 32M
operations to four sweeps / 128M operations, and randomized elimination-game search
received deterministic size-band floors. For `64 <= n <= 1000`, both PEP and the
legacy selector receive 800M operations per stream; above 1000 they receive 600M;
below 64 the floor is zero and the selector retains its old 300M PEP or 200M legacy
budget. Therefore every policy experiment had to be re-stacked on exact source 698.
Results from an older source are directional evidence only: the pair-descent phase can
change the incumbent handed to later phases, while the new search depth can expose a
different part of each policy's trajectory.

The question was deliberately narrow: can one of the four already-running PEP default
trajectories be replaced by a complementary deterministic policy without adding a
thread, a stream, or a single operation to the per-stream budget? Prior isolated A/Bs
on the older source suggested that slot 3 was the best replacement position, but this
run treats that only as a hypothesis and measures it again on the current frontier.

## What changed

`search_par_default_seeds` still constructs exactly four specifications and still
passes the same `budget` to every specification. Slots 0, 1, and 2 retain their accepted
PEP fixed seeds and `Params::DEFAULT`. Slot 3 is replaced with
`(stream_rng(3), stream_params(3), budget)`. For index 3, `stream_params` keeps the
alternating prefix mode (`prefix_mode = 2`) but narrows the policy mask to `0b0011_0111`;
`stream_rng(3)` supplies a fixed, pattern-independent seed. Search and merge remain pure
and deterministic: independent streams run concurrently and strict argmin with the
lowest index as tie-break decides the returned permutation.

This is a portfolio substitution, not an additive phase. The array is still length four,
and `search_par_specs` executes at most
`PAR_MAX_THREADS.min(specs.len())`, so the cap and fan-out are unchanged. A fifth array
entry is not a free follow-up: under the present four-thread cap it would simply be
ignored; changing the cap or batching it would add work/wall time and would require a
new cap audit. The useful comparison is consequently one varied stream against one of
the four correlated defaults, not a fifth candidate bolted onto the portfolio.

## Full-corpus A/B

Both sides were built from exact source 698 and run through the same release harness on
all 300 matrices. Lower is better.

| Metric | exact-698 baseline | slot-3 varied | Change |
|---|---:|---:|---:|
| weighted score | 0.819962 | **0.819852** | **−0.000110** |
| `lt_1k` geomean (147) | 0.8925 | 0.8928 | +0.0003 |
| `1k_10k` geomean (108) | 0.8622 | **0.8616** | **−0.0006** |
| `gt_10k` geomean (45) | 0.7338 | 0.7338 | 0.0000 |
| observed worst `order()` | 1.515 s | **1.412 s** | −0.103 s |

At printed row precision, 13 matrices improve, 9 regress, and 278 are identical. The
largest gain is `crudeoil_pooling_ct3`, 0.6865 → 0.6486 (−0.0379). Other material gains
include `waterund11` 0.7641 → 0.7546, `chimera_k64ising-02` 0.6123 → 0.6047,
`chimera_mgw-c16-2031-01` 0.8444 → 0.8406, `hydroenergy2` 0.8579 → 0.8544,
`rsyn0820m02m` 0.9445 → 0.9417, and `chimera_selby-c16-01` 0.5935 → 0.5920.
The largest regressions are `pooling_adhya4pq` 0.7698 → 0.7973 and `ndcc13`
0.6370 → 0.6486. The weighted geomean nevertheless improves strictly because the
middle bucket's gains outweigh both those losses and the small-bucket drift.

The score was not inferred from a subset or from the older PEP runs. The two complete
outputs are retained as ignored local artifacts under `target/probe-p32/`:
`p32-698-baseline.txt` and `p32-698-slot3-varied.txt`. The candidate ended with
`SCORE = 0.819852`, `WORST order() = 1.412 s`, and a passing harness result.

## Why it wins

Four copies of the default policy differ in seed but remain correlated in operator
choice. Randomized exact elimination has large sideways plateaus: equal-cost moves and
different prefix distributions can lead into different basins even when the starting
AMD/AMF permutation and operation budget are identical. The slot-3 policy preserves the
alternating-prefix behavior that performed well in the original RGREEDY study, while
its narrower mask changes which proposal operators consume the fixed budget. Three
default streams remain as insurance for families where the accepted PEP trajectory is
already best. The fourth worker explores a complementary basin and the global exact
scorer admits it only when its flop count is lower.

The new band floors make this replacement more, not less, important to validate. On
`64 <= n <= 1000`, the varied stream gets the same 800M operations as each default; for
`n > 1000` it gets the same 600M; below 64 it gets the selector's unchanged budget.
Nothing about the varied policy bypasses or stacks on top of those floors. This also
explains the bucket shape: large matrices outside the useful randomized-search envelope
remain effectively unchanged, while the 1k–10k population contains enough reachable
basins for policy diversity to pay. The small bucket's +0.0003 warns that diversity is
not componentwise monotone, hence keeping three default streams matters.

## Time-cap and determinism audit

The acceptance rule for this round is stricter than merely staying below the nominal
two-second process limit: the observed full-run worst must be at most 1.55 s. The
candidate's 1.412 s passes that bound with 0.138 s of measured margin. The baseline's
1.515 s slowest row and candidate's 1.412 s slowest row are both outside the changed
slot's useful gate, so their difference is ordinary whole-run timing noise, not a claim
that this patch accelerates the algorithm. For affected rows, the same number of
threads and exactly equal deterministic operation budgets provide the structural cap
argument. Local timing remains evidence about this machine only; hidden-hardware
variance is why no fifth stream or raised budget accompanies the win.

Determinism is preserved because the seed and policy are compile-time deterministic,
there is no clock or entropy input, streams share no mutable search state, and merge
order cannot change strict argmin/tie-breaking. The complete network-disabled release
test invocation exited 0: 92 tests were enumerated, 54 probe/measurement tests were
ignored, and all **38 active tests passed**. The sandboxed production build also ran
offline with its network namespace disabled. The production-code delta from source 698
is confined to `src/ordering/rgreedy.rs`; pair descent and the size-band floors in
`src/ordering/mod.rs` are the unchanged exact-frontier base.

## Decisions and follow-ups

- Promote only from exact source 698 while it remains live; do not merge this patch
  onto the obsolete 286 source or quote the older 0.819790 measurement as current.
- Keep one varied slot. Slot 2 (log-tail-only) and slot 1 (uniform-only) are separate
  substitution experiments, not changes to stack together without another full-300
  A/B. Test them only if this exact candidate is rejected or a newer source supersedes
  it.
- Do not try a fifth stream under the current four-worker cap. It is either ignored or
  requires extra scheduling/work, losing the cap-neutral property that makes this win
  safe.
- Re-run exact A/B after every live-source change. Both stronger pair descent and future
  budget-floor changes can alter policy complementarity, so score deltas do not commute
  across frontier revisions.

## Links

- Prior randomized elimination-game mechanism: [0022](0022-rgreedy-elimination-game-ils.md)
- Current frontier context: [0023](0023-heavy-tier-metric-gate.md)
