<!-- 0015 -->
# 0015 — Where the time goes on matrices that return ratio exactly 1.0000

- **Date:** 2026-08-28
- **Score:** unchanged (measurement only, no code shipped)
- **Status:** inconclusive — a quantified, actionable lead, not a result

## Hypothesis

Time is the binding constraint on this challenge (worst `order()` = 1.852 s against
a 2.0 s cap, see [0014](0014-ndfm-amd-leaves.md)), and
[open-questions.md](../open-questions.md) records that candidate **diversity** past
four variants is still unmeasured against the real portfolio. Both cannot be served
at once unless headroom is found first. So: on matrices that return ratio exactly
**1.0000** — where the portfolio provably gained nothing — which phase spent the
time, and is it reclaimable at zero score cost?

## Method

`order_with_relabel_cap(pattern, 0)` and
`order_with_options(pattern, _, orbit=false, _)` /
`(_, _, _, pharmakos=false)` already exist precisely for phase attribution. Timed
each against full `order()` on the nine highest-time / ratio-1.0000 matrices, and
re-scored every variant through `flops_of` so the ratio column is trustworthy
rather than assumed.

## Result

| matrix | n | nnz | full | no-relabel | reclaim | r_full | r_norelabel |
|---|---|---|---|---|---|---|---|
| `ringpack_30_2` | 17999 | 121458 | 1.827 s | 1.674 s | **0.153 s** | 0.3017 | **0.3017** |
| `faclay75` | 272878 | 1379706 | 1.253 s | 0.917 s | **0.336 s** | 1.0000 | 1.0000 |
| `squfl030-150` | 13680 | 36000 | 0.781 s | 0.550 s | 0.231 s | 1.0000 | 1.0000 |
| `nuclear104` | 39098 | 257806 | 1.170 s | 1.107 s | 0.063 s | 1.0000 | 1.0000 |
| `supplychainr1_053050` | 16640 | 57800 | 0.677 s | 0.543 s | 0.134 s | 1.0000 | 1.0000 |
| `faclay35` | 26778 | 132380 | 0.666 s | 0.612 s | 0.054 s | 0.9997 | 0.9997 |
| `emfl100_5_5` | 21925 | 52300 | 0.656 s | 0.526 s | 0.130 s | 1.0000 | 1.0000 |
| `kissing2` | 20772 | 401024 | 0.606 s | 0.589 s | 0.017 s | 1.0000 | 1.0000 |
| `emfl050_5_5` | 13175 | 32300 | 0.436 s | 0.308 s | 0.128 s | 1.0000 | 1.0000 |

ORBIT and PHARMAKOS are near-free on all nine (≤0.03 s either way); the relabel
multi-start is the whole cost.

**The headline is the first row, not the 1.0000s.** `ringpack_30_2` is the matrix
that holds the corpus worst case *and* the largest single win on it (ratio 0.3017),
and the relabel multi-start contributes **exactly nothing** to that ratio while
spending 0.153 s of a 2.0 s budget on it. That is counter-intuitive given
[0003](0003-relabelled-amd-multistart.md) and
[0011](0011-relabelled-amf-multistart.md) make the multi-start the largest historical
winner overall — its value is real but is earned on *other* matrices.

## Why this is not yet a change

The reclaim is only realisable through a gate that predicts these matrices
*before* paying for them, and "will this tie AMD?" is circular — the tie is the
result of the run. Two honest candidate discriminators, both unmeasured:

1. **Structural.** The 1.0000 set is dominated by facility-location / supply-chain
   /assignment families (`faclay*`, `emfl*`, `squfl*`, `supplychainr1*`, `kissing2`)
   where AMD is already optimal or near it. If a cheap structural signature
   separates that family from the ones the multi-start wins on, the gate is safe.
2. **Early-exit.** Run one relabel restart, and if it fails to beat the anchor at
   all, skip the remaining schedule. This needs the *first* restart's outcome to
   predict the rest — measurable directly with `probe_relabel_schedule`.

Either way the safe first target is `ringpack_30_2`: dropping its multi-start is
**score-neutral by measurement** (0.3017 both ways) and buys 8% of the worst case,
which is the single largest score-at-risk item on the corpus (+0.0084 if a slower
grader times it out).

## Caveat on these numbers

Timings are single-shot on an idle host. [0014](0014-ndfm-amd-leaves.md) documents
that this box's load can vary run-to-run by ~1.6× and that the 2 s cap is unguarded
wall-clock, so treat the *ratios* here as exact (they are re-scored through
`flops_of`) and the *seconds* as one significant figure. Re-measure before gating.
