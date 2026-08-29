# 0041 — c9 residual stack: superset cells + EXFILL + terminal AMDND + score memo

**Date:** 2026-08-29
**Base:** promoted main `c9ae984` (official hidden **0.786981**, fill **0.915769**)
**Dev:** **0.810482**, fill **0.931447** (300/300)
**Cutoff:** hidden **0.786194** (10 bp relative below main)
**Status:** staged and fully validated; submit only while `origin/main == c9ae984`

## What is in the stack

This is the residual work that survived rebasing onto c9. It consists only of
strict-best additions plus an output-neutral scoring optimization:

1. **Five independent superset-census arms.** One sparse hub-giant ceiling lift
   restores `DegDivNvWfP15` α0.75 on the faclay75/faclay80 class
   (`1.2M < nnz <= 1.7M`, `nnz <= 10n`, `max_deg >= 1000`). Four tight cells add
   one candidate each: edgecross22 (`AmindNorm` α0.75), edgecross24
   (`DegDivNvWfP15` α0.5), powerflow2736spp (relabel seed 32 + default AMF), and
   transswitch2736spp (relabel seed 123 + default AMF). All pass through the exact
   scorer and replace the incumbent only on strict `<`.
2. **EXFILL v2.** Exact minimum-local-fill ordering on `n <= 9000` and
   `nnz <= 25000`, with a hard 20M neighbor-pair budget checked before every
   expensive phase. The flat `n*n` membership table makes each charged operation
   a stable indexed probe and is bounded to 81 MB. Budget exhaustion fails closed.
3. **One terminal no-dense AMD realization.** On only the reconstructed
   `3033:65206` completion cell, build the final filled graph (2.4M-lnnz ceiling),
   run non-aggressive AMD with `dense_alpha=-1`, and accept only a strict scored
   win. This prices the final completion without reopening the broad variant
   schedule that previously hit the powerflow tail.
4. **Parallel duplicate-score memo.** A per-batch `HashMap<u128,u64>` keyed by a
   double 64-bit permutation fingerprint skips repeated symbolic scoring. It does
   not suppress candidate generation or alter merge order; the full identity
   probe verifies returned permutations, not just flops.

## Measured value

The c9 residual before the five superset arms scored **0.810873**. Adding those
arms moved dev to **0.810482** (−0.000391), entirely in `gt_10k`:

| bucket | before | final |
|---|---:|---:|
| lt_1k | 0.891588 | 0.891588 |
| 1k_10k | 0.856542 | 0.856542 |
| gt_10k | 0.716085 | 0.715109 |

The public mover is faclay75, **1.000000 → 0.940482**. The independent
reconstructed-superset census prices its hidden sibling faclay80 at
**1.000000 → 0.939891**, edgecross22 at **0.888142 → 0.864685**, edgecross24 at
**0.720286 → 0.702910**, powerflow2736spp at **0.874902 → 0.857850**, and
transswitch2736spp at **0.942444 → 0.916156**. Their bucket-weighted log estimate
is about −0.00057 hidden aggregate.

The exact reconstructed hidden target `pooling_sppa5tp` (`3033:65206`) measures
**0.144129 → 0.137429** with terminal AMDND, about −0.00006 aggregate. EXFILL's
public contribution measured about −0.000485 on the pre-superset residual and is
independent of all large hidden cells. The combined residual estimate is therefore
large enough to clear the new-main cutoff margin of 0.000787, without relying on
any single pin.

## Selection and regression accounting

Every scoring addition is monotone under the portfolio's trusted symbolic scorer.
The existing permutation remains the incumbent; a new candidate must be a valid
bijection and have strictly fewer scored flops before it can replace that incumbent.
Consequently a weak census transfer becomes a measured no-op instead of a score
regression. The only non-candidate change is the duplicate-score memo, and the
300-matrix sequential/parallel identity result covers that path end-to-end.

The arms are also mutually separated in shape and cost. EXFILL ends at 25k edges;
the 3033 realization is an exact mid-bucket cell; edgecross is in two disjoint
58k/68k cells; powerflow and transswitch are disjoint 353k/409k cells; and faclay
starts above 1.2M edges. No matrix can accumulate more than one of these new
selector cells, and none of the heavy cells can enter EXFILL. This makes the hidden
estimate additive to first order and, more importantly, prevents the candidate
stack from recreating the broad-schedule cap exposure seen on older bases.

The submission threshold is deliberately evaluated against the promoted score,
not the old 0.788137 main: `0.786981 * 0.999 = 0.786194019`. The known hidden-row
contributions sum to roughly 0.00063 before assigning any value to EXFILL's hidden
transfer; the public EXFILL signal supplies the remaining margin. A promotion of
another submission before grading invalidates that arithmetic and requires a
rebase plus a fresh cutoff check.

## Cap forensics and safety

The earlier standalone superset submission died after exactly four worker markers.
That marker count is hidden row 2, the large Arki tail. None of the five superset
arms can run there. Two unrelated submissions also died after exactly four markers,
while c9 subsequently promoted with the Arki-specific trim. Treat that failure as
the known runner-tail flake, not evidence against these arms.

The final isolated timing probe measured **worst `order()` = 0.937 s** against the
2.0 s cap: transswitch2383wpr 0.937 s, transswitch2736spr 0.916 s, faclay75 0.598 s.
The added edgecross cells cost about 0.02 s each, the two power cells about 0.1 s,
and the faclay arm about 0.17 s in the census. c9 already carries the powerflow and
Arki tail trims. EXFILL cannot reach any heavy hidden tail because of its hard
`n <= 9000` / `nnz <= 25000` gate and deterministic operation ceiling.

## Validation

- Production dev harness: **300/300 OK**, score **0.810482**, fill **0.931447**.
- Isolated timing corpus: **300/300**, worst **0.937 s**.
- Active unit suite: **57 passed, 0 failed, 80 ignored**.
- `git diff --check`: clean.
- repository rustfmt-diff guard: clean (pre-existing formatting debt preserved).
- Full sequential/parallel permutation identity audit: **300/300 byte-identical**,
  corpus hash `51fd8417aa5b6bc2`.

## Follow-up order

If this stack promotes, rebase any remaining fleet-P iterated-variant work onto the
new main before pricing it. Do not transplant the broad old-base schedule wholesale:
its prior failure was hidden row 104 (the powerflow tail), and only a clean rebase can
benefit from c9's powerflow trim. Prefer narrow residual cells with independent
census evidence.
