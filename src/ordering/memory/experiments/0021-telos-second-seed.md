<!-- 0021 -->
# 0021 - TELOS second seed (tier B): 0020's lead, shipped

- **Date:** 2026-08-28
- **Score:** dev **0.842223 → 0.840346** (−0.001877); `gt_10k` 0.7745 → 0.7698.
  The two small buckets are byte-identical to the frontier.
- **Status:** conclusive positive on score. **TWO submissions FAILED the
  grader** — `1c6bfde` (nnz gate, +10.3 s of added descent) and `baff007`
  (nnz + headroom + size gates, six matrices, +0.10 s worst). §7 has the whole
  arc. The shipped design ADDS NO WORK AT ALL: in tier B, where the schedule is
  already 1 round × 2 passes, a tied matrix descends from the runner-up
  INSTEAD of the incumbent. Same round count, same candidate count, same
  evaluations — only the starting point differs. Exactly one matrix's ratio
  moves on dev (`nuclear10a`), and its own `order()` time is unchanged at
  1.39 s.

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

## 7. The first submission FAILED the grader, and what that taught

Submission `1c6bfde` shipped the nnz-gate-only variant (dev 0.840301). The
grader returned **failed** — workflow failure at the Benchmark step, no score,
no metrics. The Actions log is not readable without a GitHub session, so the
cause is inferred, but the inference is tight: the only enforced quantity that
variant moved is per-matrix wall clock, and inside tier B the cost and the
benefit are *still* not the same matrices.

| matrix | n | nnz | `order()` | Δratio |
|---|---|---|---|---|
| `nuclear10a` | 17 493 | 163 816 | 1.39 → 1.45 s | **−0.2383** |
| `arki0013` | 44 909 | 160 172 | 1.32 → 1.48 s | −0.0041 |
| `nuclear104` | 39 098 | 257 806 | 1.61 → 1.76 s | 0 |
| `gams05` | 17 364 | 252 910 | **1.73 → 1.90 s** | 0 |

`gams05` reaches **95% of the 2 s SIGKILL** on the measuring box for exactly
zero ratio. The hidden corpus is ~500 matrices to dev's 300 and shares exactly
one with it, so a tier-B matrix there taking +0.18 s is the likeliest breach —
and index.md's own warning says a green cap test is weak evidence while a red
one is strong.

**nnz was the wrong gate.** Two more, both pure functions of state the phase
already holds:

- **Headroom.** A second basin can only pay where the portfolio has not found
  one. `nuclear10a` was still at 0.9961 of AMD; `gams05` was already at 0.6075.
  Gate: `best_flops * 100 >= amd_anchor_flops * 99`, integer-only so the two
  required runs agree bit-for-bit. This alone drops `gams05` and `arki0013`
  and takes the added corpus time from +10.3 s to +1.7 s.
- **Size.** The tie test is coarser than it reads — it fires on `best_flops`
  as TELOS *receives* it, and on several matrices the portfolio is still near
  AMD there while the post-TELOS lattice phases do the winning later. So it
  still admitted `nuclear104` (+0.19 s, Δratio 0) and `transswitch2383wpr`
  (+0.11 s, Δratio 0). A second descent costs what the first costs, so bound it
  by the same size proxy the rest of the file uses: `n < 30 000`.

Shipped envelope: `nnz ∈ [150k, 400k)` **and** `n < 30 000` **and** still tied
with AMD. Six matrices on dev, the largest addition **+0.10 s** (`nuclear10a`,
1.39 → 1.49 s), total corpus `order()` time **152.9 s → 152.7 s** — unmoved
within noise. Score is unchanged from the tie-gate variant at **0.840346**,
because only `nuclear10a`'s ratio ever moved in the first place.

The lesson generalizes past this experiment: on this challenge a gate should be
priced on the matrices it ADMITS AND DOES NOT HELP, not on the ones it wins.
The first variant's own attribution table showed `gams05` and `nuclear104` at
Δratio 0 and I shipped anyway, because the corpus-level worst case had not
moved. The corpus-level worst case is the wrong statistic when the hidden
corpus is a different corpus.

## 8. Two FAILs later: stop adding work, move it

`baff007` was the careful version — three gates, six matrices, +0.10 s on the
worst of them, total corpus `order()` time 152.9 → 152.7 s. **It failed too.**

That kills the "just gate it tighter" branch. The grader is a 2-vCPU GitHub
Actions runner (`.github/workflows/benchmark.yml`) running each of the two
required `order()` calls in its own bubblewrap namespace, and index.md already
records the frontier at ~93% of the cap with identical code passing twice and
failing a third time. On a corpus of ~500 matrices that share exactly one with
dev, "+0.10 s on the matrices dev happens to show me" is not a bound.

So the design changed shape. Do not run a second descent — run the SAME descent
from a different point:

> Where the portfolio is still tied with AMD, the incumbent has no basin worth
> descending from — that is what the tie MEANS — so descend from the runner-up
> INSTEAD.

In tier B the schedule is already `(1 round, TELOS_B_PRIO, 2 passes,
TELOS_PEEL_B)`, so the swapped descent is byte-for-byte the same work: same
rounds, same candidates, same evals. **The phase cannot cost more than the
frontier because it does not do one extra thing.**

Measured: score **0.840346** (reproduced twice), exactly ONE ratio changes
(`nuclear10a` 0.9961 → 0.7578), that matrix's own time is **1.39 s → 1.39 s**,
and total corpus `order()` is 152.9 → 152.5 s. The residual ±0.1 s wiggles are
on matrices whose ratio did not move — box noise, not the change.

Two intermediate designs, both measured and both rejected:

- **Swap the seed everywhere, full schedule.** 0.840340, but +14.5 s over the
  corpus and `ringpack_30_2` 2.63 → 2.95 s. A worse seed makes round 1 find an
  improvement, so round 2 fires where the incumbent's descent would have broken
  out immediately. Descending from a worse point is not free.
- **Swap everywhere with tier B's cheap schedule.** 0.840349 and net −3.5 s,
  which is genuinely cheaper than the frontier — but it moves a second ratio
  (`qspp_0_11_0_1_10_1` 0.9911 → 0.9926, a regression) and it perturbs tier A,
  where the corpus worst case lives. Tier-B-only gives up 0.000003 of score to
  touch nothing outside a band where the work is provably constant.

## Next

- **K > 2.** The slot currently holds one runner-up. 0020's argument does not
  stop at two basins, and tier B's descent is cheap enough to afford a third.
- ~~**Tier A wants a different treatment, not exclusion.**~~ **MEASURED, NEGATIVE**
  (same session): a second seed with tier B's cheap schedule inside tier A scores
  0.840301 → 0.840217, i.e. −0.000084, while `ringpack_30_2` goes 2.719 → 2.810 s.
  Cheaper than the full schedule (+0.23 s) but not free — tier A's descent is
  expensive because the graphs are, not because of the peel grid. Rejected.
- The runner-up is currently whatever `consider` last demoted. Retaining the
  best *diverse* runner-up (e.g. from a different family than the argmin)
  is a strictly better use of the same slot and is untested.
