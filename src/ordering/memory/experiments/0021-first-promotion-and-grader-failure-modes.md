<!-- 0021 -->
# 0021 - first promotion to #1, and the grader failure mode that gated it

- **Date:** 2026-08-28
- **Score:** dev `0.842722 -> 0.842223`; **graded `0.834007 -> 0.833396`, promoted**
- **Status:** WIN, shipped. First #1 on this challenge.

## Result

Submission `f6e0c2d` promoted at graded `0.833396` (`-0.000611`, `-0.07%`), taking the
frontier from `fkiene`'s `0.834007`. Content: the two restart floors of 0019
(hub-gated mid band + cap-bypassing low-nnz regime).

## The decisive operational lesson: `failed` is not `rejected`

The submission list exposes two distinct outcomes, and they mean opposite things:

- **`rejected`** - a score was produced and was not strictly better. Cheap, informative.
- **`failed`** - no score at all, no metrics. Carries zero information about the candidate's
  quality.

Three attempts today, in order:

| submission | content | worst dev `order()` | outcome |
|:--|:--|--:|:--|
| `b904857` | floors + 2nd TELOS seed (ungated by nnz) | 1.828 s | **failed** |
| `f6e0c2d` | floors only | 1.784 s | **promoted 0.833396** |
| `3689919` | floors + 2nd TELOS seed (`nnz <= 40_000`) | 1.849 s | **failed** |

The controlled comparison is unambiguous: the discriminator is the **second TELOS descent**,
not worst-case per-matrix time. Attempt 3 tightened the gate specifically so that no matrix
above `nnz 40_000` got a second descent - every dev matrix over 1.4 s has `nnz >= 52_108` -
and it failed anyway, while the variant without the descent promoted.

Note also that worst-case time was *attributed*, not assumed: on all three the slowest
matrix is `ringpack_30_2`, which both gates exclude, so its `1.784-1.852 s` spread across
five runs is machine variance on an untouched matrix, not an effect of any edit. The
frontier's own code measures `1.852 s` there.

## Global-budget hypothesis: REFUTED by measurement (2026-08-28)

I first blamed a **global** wall-clock budget on the benchmark command, on the impression
that the scored run went from ~180 s to ~250-330 s. Those figures were contaminated: they
included a load-wait loop. Timed properly, back to back on an idle machine:

| build | benchmark wall time |
|:--|--:|
| promoted (floors only) | **216 s** |
| floors + 2nd TELOS descent | **221 s** |

**+2.3%.** A 2% increase cannot produce a hard failure, so the global-budget hypothesis is
dead. Do not spend time on it.

## Surviving hypothesis: per-matrix cap on matrices not in the dev corpus

The second descent adds up to the largest measured TELOS time (`0.314 s`) to **every matrix
that passes its gate**, and the graded corpus is a different matrix set from the dev corpus.
This is the only mechanism left that fits all three attempts, and it correctly predicts the
otherwise-puzzling part: tightening the gate to `nnz <= 40_000` shrank the exposed set but
did not empty it, so a single unseen gated matrix sitting near the cap still fails the run.

It also explains why worst-case dev time was a useless guide here - dev worst is dominated
by `ringpack_30_2`, which the gate excludes, so dev worst barely moves while the risk is
carried by matrices in the middle of the distribution.

Consistent with the earlier evidence, which is unchanged:

- the build emits no errors and only pre-existing warnings on this toolchain;
- the local corpus produces a valid score with valid bijections every time;
- per-matrix worst case is dominated by a matrix the change never touches.

What would settle the surviving hypothesis: a per-matrix time bound that holds for ANY
matrix passing the gate, not just for the ones in this corpus. The honest version of that is
a much narrower structural gate sized on `n` and `nnz` jointly with a worst-case argument -
not another sweep against 300 matrices we can see. Deprioritised: the increment is worth
`-0.000191` of dev score against a demonstrated 2-for-2 failure rate, and a failed
submission cannot demote a promoted one but also buys nothing.

## Held-back increment (measured, valid, not shippable as-is)

The second TELOS descent from the AMD anchor is genuinely zero-downside on score - verified
per-matrix, **6 better, 0 worse**, dev `0.842223 -> 0.842032`. Gains: `chp_partload`
`0.8086 -> 0.7786`, `crudeoil_lee2_06` `0.9885 -> 0.9674`, `rsyn0840m04m`
`0.9153 -> 0.9061`, `gasprod_sarawak16` `0.9955 -> 0.9909`. Its dev score is *identical*
with and without the `nnz <= 40_000` bound, so all of its value comes from small matrices.

If the global-budget hypothesis is right, the shippable form is a much narrower gate -
e.g. `nnz <= 20_000`, which keeps `chp_partload` and `gasprod_sarawak16` - bought with a
measurement of total run time rather than another blind attempt.

## Transfer from dev to graded, measured for the first time

Two data points from today, both on the same 300-matrix dev corpus:

- mine: dev `-0.06%` -> graded `-0.07%` (promoted)
- another solver's: local improvement -> graded `+0.09%` (rejected)

So the correlation is real but noisy at this magnitude. This is the argument for preferring
changes that are zero-downside *per matrix* by construction over tuning-shaped gains of
similar size: their sign does not depend on which matrices the grader happens to hold.
