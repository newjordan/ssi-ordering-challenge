<!-- 0028 -->
# 0028 — Hub-gate relabel-multistart skip: 23/23 dev matrices score-neutral (cap-survival lead, measurement only)

- **Date:** 2026-08-29
- **Score:** unchanged (measurement only; NOT shipped)
- **Status:** strong cap-survival LEAD, held back from the 0027 submission on
  eval-transfer risk — needs eval-log confirmation before shipping

## Motivation

The grader is in a mass-failure window (docs/INFRA-2026-08-29-grader-massfail.md):
≥14 fails across 10 accounts, runner-image rollout, failing runs dying at the
4-6 min mark = per-matrix 2 s cap kills mid-corpus. In this climate, reclaiming
worst-case `order()` time is not a nicety — it is grading-survival probability.
`ringpack_30_2` holds the corpus worst case (~1.78 s of 2.0 s) AND the largest
single win (ratio 0.3017); a FAIL there scores nothing at all.

0015 measured that on `ringpack_30_2` the relabel multistart contributes
EXACTLY 0 to the ratio (0.3017 with and without) while costing ~0.153 s. This
experiment re-proves that on the CURRENT watcher-MINL tree (0023 staleness
rule) and extends it to the whole hub class.

## Method

`probe_midband_hub_relabel`: for every dev matrix in the candidate gate
`30_000 < nnz <= 150_000 && max_deg > n/50`, compare `order()` (full) against
`order_with_relabel_cap(pat, 0)` (zeroes the mid/heavy-band restarts; the
small-band nnz<=20k floor does not apply to this nnz range, so this genuinely
removes the multistart for in-gate matrices). Report flops of both and whether
the permutation changed. Flops-only, load-independent.

## Result — 23/23 SCORE-NEUTRAL

Every one of the 23 hub-gate matrices has `flops_cap0 == flops_full` exactly
(rel_delta +0.000000), including `ringpack_30_2` (3_739_119 both ways),
`faclay35` (33_621_390), `edgecross24-115` (28_688_513), `pooling_sppa9tp`
(37_534_275). One matrix (`qspp_0_11_0_1_10_1`) has a different permutation at
identical flops — a tie-break, same score. So on dev, skipping the multistart
for the entire hub class is free by direct measurement, and it reclaims the
multistart's time on the corpus worst-case matrix.

The mechanism is 0019's, inverted: a large hub is WHY these matrices' restarts
are expensive, and (0015's finding) that expense buys nothing because AMD is
already near-optimal on the hub-dominated structure.

## Why NOT shipped in 0027

1. **Cannot ship alone** (0017 rule): a score-neutral change is a 0.00% diff,
   rejected. It would have to RIDE a scoring win — 0027 is that win, but:
2. **Eval-transfer risk.** The probe measures DEV only. The relabelled-AMF
   multistart is a historical EVAL winner overall (0011); "all hub matrices get
   nothing" is a broader claim than 0015's single-matrix ringpack result. On a
   hub matrix where the multistart wins on eval, blanket-skipping it loses eval
   score. 0027 is currently strict-improvement-or-identical on EVERY matrix by
   construction (strict-keep + a ceiling that only removes descents); folding in
   an unverified eval-side skip would forfeit that clean guarantee on a
   submission that has none to spare in a FAIL-happy fleet.
3. No eval-log access this session (no `gh` CLI; backend repo private), so the
   eval-side census that would de-risk this (0005's mining method) cannot run.

## Next step

Ride the hub gate on the NEXT scoring win AFTER confirming eval-side via the
Actions-log census (0005 method) — mine the promoted run's per-matrix table for
the hub-class matrices and verify the multistart is not the argmin there. If
confirmed, implement as an early `return 0`-style skip inside `relabel_restarts`
gated on `30k<nnz<=150k && max_deg>n/50` (identity-safe, deterministic), and
re-measure worst-case `order()` — expect `ringpack_30_2` to drop ~0.15 s,
buying ~8% of the cap-survival margin the whole corpus is currently short on.

## Links

- Original ringpack measurement: [0015](0015-zero-return-time-attribution.md)
- The hub covariate + mechanism: [0019](0019-mid-band-restart-floor.md)
- Why score-neutral can't ship alone: [0017](0017-relabel-budget-saturation.md)
- The fleet climate: docs/INFRA-2026-08-29-grader-massfail.md
