# 0012 — α-diversity across relabel restarts (cost-neutral), and win C re-measured

**Model:** Claude Opus 5 (medium effort), Agent: Claude Code
**Date:** 2026-08-18
**Score:** dev 0.843535 → **0.843417** (−0.000118). Buckets: lt_1k 0.8995→0.8993,
1k_10k 0.8774→0.8772, gt_10k 0.7761→0.7761 (unmoved).

Base checkout: `a912d66` = promoted frontier (submission `1e79ec8`, darenspears,
eval 0.834569). Local dev baseline reproduced the KB's recorded 0.843535 exactly,
so the inherited numbers in [index](../index.md) are trustworthy this round.

## Hypothesis

Both relabel multistarts spend their whole restart budget at a SINGLE
`dense_alpha`: the AMD loop ([0003](0003-relabelled-amd-multistart.md)) at AMD's
library default α=10.0, the AMF loop ([0011](0011-relabelled-amf-multistart.md))
at α=5.0. But [0010](0010-parallel-research-session.md) measured that winning
`dense_alpha` values **scatter across matrices with almost no repeats**, and that
"wide cheap grids beat a few tuned constants."

So cycling α across restarts should add diversity **at zero cost**: each restart
already has its own seed, so varying α as well makes every pass strictly *more*
distinct than before, while the number of AMD/AMF generation passes — and hence
every existing cost model and cap analysis — is **exactly unchanged**. This is the
cheapest possible bet under the asymmetric-risk rule.

## Implementation

Two ~4-line changes in `mod.rs`, both pass-count-neutral:

```rust
// relabelled-AMD multistart (was: plain amd_order, i.e. α=10 default)
const RELABEL_AMD_ALPHAS: [f64; 4] = [10.0, 5.0, 2.0, 16.0];
let amd_alpha = RELABEL_AMD_ALPHAS[r % 4];
let opts = feral_amd::AmdOptions { aggressive: true, dense_alpha: amd_alpha };

// relabelled-AMF multistart (was: fixed α=5.0)
const RELABEL_AMF_ALPHAS: [f64; 4] = [5.0, 2.5, 10.0, 1.0];
```

Design details that matter:

- **α=10.0 is grid slot 0** in the AMD loop, and 10.0 + `aggressive: true` IS
  `amd_order`'s library default. So restart 0 — and therefore every matrix whose
  budget affords only ONE restart, i.e. the whole heavy/sparse tail — is
  **bit-identical to the frontier**. Only matrices rich enough for a second
  restart change at all.
- Seeds are untouched (`seed = r+1`), so this ADDS a diversity axis rather than
  trading seed diversity for α diversity.
- Both loops route through `consider`, so the AMD floor still means a candidate
  can only win or be discarded — never regress a matrix.
- α values are the ones the non-relabelled AMD sweep already proved carry unique
  wins ({10, 5, 2} plus 16 from the sweep), not fresh guesses.

## Results, measured separately

| revision | dev score | Δ | where |
|---|---|---|---|
| frontier `a912d66` | 0.843535 | — | — |
| + AMF α-cycling | 0.843468 | −0.000067 | all in 1k_10k |
| + AMD α-cycling | **0.843417** | −0.000051 | lt_1k + 1k_10k |
| + win C re-sized | 0.843382 | −0.000035 | gt_10k 0.7761→0.7760 |
| **shipped** | **0.843417** | **−0.000118** | win C reverted |

## Win C: re-sized from measurement, then declined anyway

`RELABEL_BUDGET_LARGE` (1.5M/150) has sat disabled with a doc comment rejecting it
on an ANALYTIC ~1.5 s envelope read off the "budget = microseconds" convention.
That envelope is wrong in a way worth recording: cost is `restarts * k * nnz` and
`restarts = budget/nnz`, so **nnz cancels** — added cost ≈ `k * budget`,
independent of nnz. With the isolated constant measured in
[0007](0007-heavy-multi-restart.md) (`k ≈ 6e-8 s/nnz`, 4-23× cheaper than the
2.6e-7 model that sized the old floor), a 900k budget implies ≈ 0.054 s added, and
even at the pessimistic legacy 2.6e-7 it is ≈ 0.234 s — both inside the ≈ 0.39 s
worst-added envelope [`HEAVY_SPARSE_MAX_AVG_DEG`] already accepts.

So I shipped `(900_000, 48)` under `n >= 10_000 && nnz <= 6n` (the load-bearing
density guard — the `k` constants were measured on sparse heavies ONLY, and
ignoring that is the size-by-analogy error that FAILED the grader once already).

**It bought −0.000035.** gt_10k moved 0.7761 → 0.7760. That is a rounding error,
and it is the ONLY change in this revision that adds work — everything else is
pass-count-neutral. Spending the entire revision's cap risk on −0.000035, when no
candidate can improve the score by more than it can catastrophically FAIL the run,
is exactly the trade [0010](0010-parallel-research-session.md) says to decline.
**Reverted.** The re-sizing argument is preserved in the code comment for whoever
revisits it with a bigger lever.

## Why gt_10k did not respond

The KB holds that gt_10k is *budget*-limited. This session refines that: gt_10k is
budget-limited **in restart count**, but its restart count is so small (often 1)
that an α GRID cannot reach it — slot 0 is all those matrices ever see, by
construction. And when the budget WAS raised 3× (win C), the bucket still moved
only 0.0001, consistent with 0010's finding that 11 of 12 tied large matrices are
flat at every budget level. **The α axis is a small-and-medium instrument; do not
price it against gt_10k.**

## Safety

All gates green on the shipped revision: 48 unit + 3 exact_equivalence + 2
scorer_crosscheck + **2 time_cap** (the real enforced SIGKILL watchdog, 102.8 s).
Cap risk is structurally unchanged from the frontier because the shipped diff adds
zero generation passes — the strongest cap-safety argument available, and better
evidence than a green cap test, which 0010 showed is weak.

## Caveats

- **−0.000118 is small enough that eval transfer is genuinely uncertain.** Dev and
  eval share exactly ONE matrix on `(n, nnz)`, and recent rejections sat at
  +0.0002…+0.0009. This may well grade as no improvement. It is submitted because
  the cost side is provably zero, not because the dev delta is convincing.
- I did NOT re-verify against the reconstructed eval corpus — `~/ssi-ordering-LEARNINGS/`
  does not exist in this environment (it is outside the repo and did not survive).
  Regenerating it is the highest-value setup step for the next session.

## Next steps

1. **Rebuild the eval reconstruction** ([0010](0010-parallel-research-session.md)a).
   At the current margin sizes, dev is no longer able to resolve wins — a −0.0001
   dev delta is unmeasurable against a nearly-disjoint graded set. This is now the
   blocking instrument, not an optimization.
2. Widen the α grids (8-16 values) for matrices with many restarts — still
   cost-neutral, and 0010's scatter finding suggests the curve has not saturated.
3. The unresolved high-leverage lead remains **portfolio blind spots** (open
   question: "what METHOD, not depth, moves the near-AMD mass?"), where STRIP was
   the one genuine hit. α-cycling is depth, and this session re-confirms depth is
   nearly exhausted.
