<!-- 0018 -->
# 0018 — Headroom census: the near-AMD mass is at its floor, not under-served

- **Date:** 2026-08-28
- **Score:** unchanged (measurement only)
- **Status:** conclusive — **retires the premise behind
  [open-questions.md](../open-questions.md)'s "what method moves the near-AMD mass?"**

## Hypothesis

The base has treated the ratio-≥0.95 mass as the place the remaining score lives
("309 of 500 graded matrices sit at ratio ≥ 0.95, concentrated in `lt_1k`… look for
portfolio blind spots"). That framing **assumes headroom exists there**.
[0012](0012-exact-dp-small-n.md) proved AMD optimal only for `n ≤ 24`. For everything
above that, nobody had asked the prior question: *is there anything to find at all?*

## Method

Spend a budget far beyond the graded cap and see if ANY ordering beats what ships.
For every matrix the portfolio leaves at ratio ≥ 0.995 with `n ≤ 20 000` (91 of them):
**3000 relabel seeds × both objectives** (relabelled-AMD and relabelled-AMF, the two
families the multi-start actually uses) = ~546 000 scored orderings, versus the ~24
restarts the 2 s cap permits. That is ~125× the shipped search depth per matrix.

Scored on **flops only**, which is deterministic and load-independent, so unlike
`probe_timing_and_score` this was safe to run with the box saturated — 24 threads via
`std::thread::scope`, ~390–900% CPU sustained.

## Result

| matrix | n | r_shipped | r_deep | seed found at |
|---|---|---|---|---|
| `ndcc12` | 974 | 1.0000 | **0.9853** | 2666 |
| `syn30m03m` | 2508 | 0.9957 | 0.9954 | 474 |
| `meanvar-orl400_05_e_7` | 5205 | 0.9999 | 0.9999 | 1988 |

**3 of 91.** Two of the three are numerically negligible. Only `ndcc12` has a real
gain (1.5%), and it was found at **seed 2666** — two orders of magnitude beyond the
~24 restarts the cap allows, so it is not reachable by tuning the budget
(cf. [0017](0017-relabel-budget-saturation.md), which showed the budget is saturated
anyway). One matrix of 147 in `lt_1k` improving 1.5% moves the weighted score by
~0.00003.

## What this retires

**The near-AMD mass is not an under-served opportunity — it is at its floor.** 88 of
91 matrices cannot be improved by 125× the shipped search within the two objectives
the portfolio's winning family uses. Combined with
[0016](0016-spectral-fiedler-nd.md), where a wholly orthogonal family (spectral
bisection) ran on 88 of these same matrices and was never once the argmin, the
convergent reading is that AMD is at or extremely near optimal on this class, exactly
as [0012](0012-exact-dp-small-n.md) proved for the bottom end — and 0012's
conclusion generalises much further up than its `n ≤ 24` proof.

So the standing lead *"what method (not depth) moves the near-AMD mass?"* rests on a
premise this census contradicts. It should not be the next thing a session spends
budget on.

**Honest limit of this result:** the deep search explored relabelled AMD and AMF only.
It is a *depth* probe within the two shipped objectives, **not** a proof of
optimality — some third method could still win where 3000 relabelings did not. What
makes the conclusion strong is not this census alone but its agreement with 0016,
which tested precisely that alternative and also found nothing.

## Where that leaves the challenge

Four measured dead ends this session, from four different directions:

| lever | direction | result |
|---|---|---|
| [0014](0014-ndfm-amd-leaves.md) | better version of a covered family | 0 |
| [0016](0016-spectral-fiedler-nd.md) | an entirely new family | 0 |
| [0017](0017-relabel-budget-saturation.md) | more depth in the best family | 0 (and worst_s → 1.986 s) |
| 0018 | is there headroom in the ties at all? | **no — 3/91, all negligible but one** |

The ties are immovable and the depth is saturated, so the remaining score is neither
in the near-AMD mass nor in searching harder. It has to come from beating AMD/AMF on
matrices where the portfolio **already wins** (`gt_10k` geomean 0.7748 at weight
0.40 is the leverage), with a method 0016 has shown is not simply "another separator
family". That is a genuine research problem, not a tuning one — and it is the same
wall the frontier holder has been hitting for 12 consecutive submissions rejected at
exactly 0.00%.

The probe is preserved at
`~/ssi-ordering-LEARNINGS/rejected-implementations/probe-headroom-census.rs`; it is
reusable as-is against any future candidate set, and its target-selection phase is
the one part worth parallelising if re-run (it is sequential and costs ~2 min of one
core before the parallel section starts).
