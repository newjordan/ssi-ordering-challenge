<!-- 0021 -->
# 0021 - TELOS second seed (tier B): 0020's lead, shipped

- **Date:** 2026-08-28
- **Score:** dev **0.842223 → 0.840301** (−0.001922), fill unchanged in the two
  small buckets; `gt_10k` 0.7745 → 0.7697
- **Status:** conclusive positive, shipped. Reproduced twice, byte-identical
  (0.840301 both runs). Worst `order()` unmoved.

## Question

0020 closed with one lead and priced it: TELOS-GRADIENT DESCENT is sharply
seed-sensitive (on `chp_shorttermplan2d` it reached 1 975 321 from the WORSE
incumbent and only 2 104 445 from the better one), so "retain top-K, descend
from each, keep the best" should capture both basins. 0018 had already shown
the corpus lacks *reachable* alternatives; this exploits a sensitivity that is
measured rather than hoped for.

K = 2 here: the incumbent, plus the portfolio RUNNER-UP.

## Implementation

Three changes in `mod.rs`, all inside the existing phase:

1. **`second_perm` / `second_flops`**, maintained by `consider` only. A
   candidate that beats the incumbent demotes it into the slot; one that beats
   only the slot replaces it. Until something beats the anchor twice the slot
   holds the AMD anchor itself, which is a legitimate alternate seed.
2. **`telos_descend()`** — the inline descent lifted verbatim into a free
   function taking `(seed_perm, seed_flops)`. From a single seed the behaviour
   is unchanged: same `ranked` order, same candidate sequence, same cached
   evals, same strictly-better-only keep, same break on a barren round.
3. The phase calls it twice: once from the incumbent (identical to before),
   then, **gated on tier B**, once from the runner-up. Adoption stays strictly
   `< best_flops`, so the AMD floor is intact and no ratio can rise.

## The band split is the whole experiment

Running the second seed wherever TELOS runs scores **0.840198** — marginally
better — but takes the worst case **2.695 s → 2.859 s** on the measuring box.
Cost and benefit live in different nnz bands:

| nnz band | mats | added `order()` | worst | Δscore |
|---|---|---|---|---|
| <20k | 221 | +2.28 s | 1.282 s | −0.000096 |
| 20k–150k (tier A) | 59 | **+7.61 s** | **2.859 s** (`ringpack_30_2`) | −0.000022 |
| 150k–400k (tier B) | 11 | +0.82 s | 1.928 s | **−0.002489** |
| ≥400k | 9 | −0.36 s | 2.671 s | 0 (TELOS gated off) |

Tier A runs up to 2 rounds × 16 passes, so a second descent nearly doubles the
phase. On `ringpack_30_2` — the matrix that holds both the corpus worst case
and the largest single win (0.3017) — it buys **+0.228 s and exactly zero
ratio**. Tier B runs 1 round × 2 passes, so the same second descent is nearly
free, and it is where every point of the win sits.

Restricting to tier B keeps 95% of the win, returns the worst case to the
incumbent's, and removes both small regressions the all-band variant carried.

## Where the win is

- `nuclear10a` (n=17493, nnz=163816) **0.9961 → 0.7578**. Essentially the whole
  score movement: Σln over `gt_10k` shifts −0.274, and 45 matrices at weight
  0.40 turn that into −0.0049 of bucket and −0.0019 of score.
- `arki0013` (n=44909, nnz=160172) 0.6261 → 0.6220.

`nuclear10a` was a near-tie the incumbent could not break: 0018's census swept
125× the shipped depth over the ratio ≥0.995 mass and moved 3 of 91, and 0016's
orthogonal algebraic family was never the argmin anywhere. Neither was looking
at the seed. It was reachable all along from a point the portfolio had already
computed and thrown away.

## Two findings worth more than the score

**The all-band variant REGRESSED two matrices** (`gasprod_sarawak81` 0.9979 →
0.9986, `nuclear25a` 0.6479 → 0.6502) even though TELOS's own adoption is
strictly-better-only. Mechanism: adopting a different `best_perm` re-seeds
COMPLETION-LATTICE / ORBIT / PHARMAKOS, which are seed-sensitive the same way
TELOS is. **0020's non-monotonicity is not a TELOS property — it is a property
of every descent phase downstream of a changed incumbent.** Any future change
that moves `best_perm` upstream of those phases inherits it.

**The `RELABEL_BUDGET` 300 000 → 150 000 free-rider is NOT free any more.**
index.md carries it as pre-verified score-neutral (+0.026 s of margin), to be
attached to the next scoring win at zero measurement cost. Attached here it
scores **0.842412 — worse than the pre-change frontier**, and `gt_10k` returns
to 0.7748, i.e. the entire win disappears. Halving the restart pool changes
which candidate lands in `second_perm`, and the seed is now load-bearing. It
was measured, not assumed, precisely because 0020 said reasoning had produced
two confident wrong answers in a row here. **Retire the carrier, or re-measure
it against whatever the frontier is at the time.**

## Method / environment note

Measured with `probe_timing_and_score` on a 2-core container that runs **~1.5×
slower** than the box the earlier pages used: the untouched frontier measures
worst 2.695 s here against the 1.784 s recorded in 0019, and the harness's 2 s
SIGKILL therefore fires on `faclay75` locally on code that grades fine. Flops
are deterministic and load-independent, so the SCORE is exact and matched the
recorded frontier (0.842223) bit-for-bit on the first run; only the SECONDS
column carries the handicap. Time claims here are therefore **comparative on
one box**, which is what the standing rule asks for anyway.

End-to-end validation through the real harness ran at `SSI_MAX_MATRIX_N=10000`
(255 matrices): purity gate, bijection, double-run determinism, and score all
pass, and the two small buckets are byte-identical to the frontier — the gate
admits nothing below nnz 150k, so that is the expected result.

## Next

- **K > 2.** The slot currently holds one runner-up. 0020's argument does not
  stop at two basins, and tier B's descent is cheap enough to afford a third.
- **Tier A wants a different treatment, not exclusion.** The cost there is the
  16-pass grid, not the second seed as such. A second seed with `TELOS_PEEL_B`
  and `TELOS_B_PRIO` (tier B's cheap schedule) inside tier A is untested and
  would cost roughly what tier B's does.
- The runner-up is currently whatever `consider` last demoted. Retaining the
  best *diverse* runner-up (e.g. from a different family than the argmin)
  is a strictly better use of the same slot and is untested.
