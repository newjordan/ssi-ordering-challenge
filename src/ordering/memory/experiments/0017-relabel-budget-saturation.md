<!-- 0017 -->
# 0017 — The random multi-start budget is saturated, and the shipped config sits at the cap

- **Date:** 2026-08-28
- **Score:** 0.842722 (unchanged; measurement only, nothing shipped)
- **Status:** conclusive negative for "more/less budget", **plus a fragility finding
  about the incumbent**

## Hypothesis

[open-questions.md](../open-questions.md) records the budget question as answered
for the hill-climb phase but **"Untested for the RANDOM multi-start budget itself."**
And [0015](0015-zero-return-time-attribution.md) showed the multi-start spends
0.06–0.34 s on matrices where it provably changes nothing, which looked like
reclaimable budget.

**First, a correction to that framing.** The 2 s cap is **per matrix**, so time saved
on `faclay75` cannot be spent on `crudeoil_lee4_10`. Cross-matrix reallocation is
impossible by construction; reclaimed time buys only safety margin on the same
matrix. What *is* testable is the global budget policy: matrices where the
multi-start pays have real slack (`crudeoil_lee4_10` 0.62 s, `crudeoil_lee4_09`
0.73 s, `gams05` 0.87 s, `arki0016` 0.92 s), so does a larger budget convert that
slack into score?

## Method

`probe_relabel_budget` already exists for exactly this: it runs enough restarts per
matrix to satisfy every policy once, records running-best flops and cumulative cost
after each restart, then reads off all 8 policies from that single sweep, reporting
`worst_s` = measured `order()` time **plus** measured restart time. Run under the
neighbouring fleet's `machine-quiet.lock` + heavy slots, per 0014's finding that this
box's load invalidates timing.

## Result

| budget | cap | score | worst_s | improved |
|---|---|---|---|---|
| 150 000 | 24 | 0.842722 | **1.832** | 0 |
| **300 000** | **24** | **0.842722** | **1.858** | — (shipped) |
| 300 000 | 48 | 0.842720 | 1.858 | 1 |
| 450 000 | 24 | 0.842722 | 1.884 | 0 |
| 450 000 | 48 | 0.842720 | 1.884 | 1 |
| 600 000 | 48 | 0.842720 | 1.910 | 2 |
| 600 000 | 96 | 0.842712 | 1.910 | 2 |
| 900 000 | 96 | 0.842697 | **1.986** | 3 |

Per-bucket, every policy is `lt_1k` 0.8994–0.8995 / `1k_10k` 0.8765 / `gt_10k`
0.7748 — the sweep does not move a single bucket meaningfully.

**Saturated in both directions.**

- **Upward is a trap.** 3× the budget and 4× the cap buys **0.000025** of score
  (0.842722 → 0.842697) and moves the worst case to **1.986 s against a 2.0 s hard
  cap**. That is not a tradeoff, it is a coin-flip on a FAIL — and a FAIL scores
  nothing at all, not merely worse.
- **Downward is free.** Halving to 150 000 costs **exactly zero** score and returns
  0.026 s of worst-case margin. The shipped 300 000 is already past the point of
  diminishing returns.

The worst matrix is `ringpack_30_2` under every single policy, consistent with
[0015](0015-zero-return-time-attribution.md): it holds the worst case, holds the
largest win (0.3017), and the multi-start contributes nothing to that ratio.

## The fragility finding

The shipped frontier configuration has a worst case of **1.858 s against a 2.0 s
enforced SIGKILL — 93% of budget, 7% margin.** Set that against what
[open-questions.md](../open-questions.md) already records about grader speed: the
header's "3–5× slower than local" claim is false, but **repeat local runs vary
~1.6×** and nothing in the harness exposes grader timing. A 1.6× excursion on
`ringpack_30_2` is a FAIL.

This is a property of the **incumbent**, not of anything tried here — and it means
the standing rule ("stay at or below the worst case of a revision known to have
passed") is currently protecting a configuration that is itself close to the edge.
Reducing `RELABEL_BUDGET` to 150 000 is a **strictly free** risk reduction by this
measurement: identical score, −0.026 s worst case.

**Not shipped anyway**, for a reason worth writing down: a submission must *strictly
beat* the frontier to be accepted, so a score-neutral safety change cannot be
delivered on its own — it would be rejected at 0.00%, exactly like the frontier
holder's own recent streak. It has to ride along with a scoring win. Bank it as a
free rider for whenever one exists.

## What this closes, and where it leaves the challenge

Three independent levers, three zeros, all measured on 300 matrices this session:

| lever | direction | result |
|---|---|---|
| [0014](0014-ndfm-amd-leaves.md) AMD leaves for `ndfm` | better version of a covered family | 0 |
| [0016](0016-spectral-fiedler-nd.md) spectral/Fiedler ND | an entirely new family | 0 |
| 0017 multi-start budget sweep | more (or less) search in a winning family | 0 |

Quality inside a covered family, a genuinely orthogonal method, and depth within the
best family all buy nothing. That is a coherent picture rather than three unlucky
draws: **the portfolio is at a local optimum that none of these three lever types
moves**, which is independent corroboration of the frontier holder's own 12
consecutive submissions rejected at exactly 0.00% diff — they appear to be stuck at
the same place.

So the next attempt should not be a fourth lever of the same kind.

**Superseded pointer (corrected by [0018](0018-headroom-census.md), written after this
page).** This section originally sent the reader to `open-questions.md`'s *"what
method (not depth) moves the near-AMD mass?"*. **Do not follow that.** 0018 spent
~125× the shipped search depth on all 91 matrices at ratio ≥ 0.995 and moved 3, two
of them negligible — the near-AMD mass is **floor**-limited, not method-limited, so
that lead's premise is retired.

What survives is only the *shape*: the STRIP transform's, i.e. a transform giving an
**existing** family new coverage of a class it was blind to, rather than a new family
competing where the portfolio already wins. But it has to be aimed at **`gt_10k`**
(geomean 0.7748 at weight **0.40** — matrices we already win, where the leverage
actually is), not at the ties.
