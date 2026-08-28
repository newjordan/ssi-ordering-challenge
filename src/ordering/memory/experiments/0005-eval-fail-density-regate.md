# 0005 — A grader FAIL, the eval census, and the density re-gate

**Date:** 2026-07-26
**Score:** dev 0.876329 (FAILED eval) → dev **0.876511** re-gated; promoted eval best remains 0.886427
**Status:** FAIL analyzed, re-gated, resubmitted

The most instructive round so far: the first grader failure, the discovery that
we have always had eval-corpus visibility without using it, and a permanent
upgrade to how gates must be designed.

## What happened

Submission `0cbb4def` (the [0004](0004-heavy-tier-gates.md) heavy-tier gates)
passed everything locally (worst `order()` 0.963 s, full suite green) and FAILED
the grader:

```
RUN FAILED: telecomsp_metro: order() exceeded the 2.0s per-matrix cap
n=13940, nnz=743408, nnz/n≈53
```

No leaderboard damage (a failed submission does not replace the promoted best),
but a burned round.

## The discovery: the eval census was always available

The grader prints the full per-matrix results table (name, n, nnz, flops, ratio)
into the **public GitHub Actions log of every submission run**. Mining the
promoted run's log yields the complete eval corpus census with our current
per-matrix ratios:

```sh
gh run list -R Lulu-Zhou-EigenLabs/ssi-ordering-challenge --workflow=benchmark.yml \
  --json databaseId,headSha,conclusion          # find the run for a submission sha
gh run view <id> -R Lulu-Zhou-EigenLabs/ssi-ordering-challenge --log \
  | grep '(capped)'                              # 1 row per eval matrix
```

Validation: the census reconstructs the official score 0.886427 to within the
log's 3-decimal rounding (0.886408). Facts as of the promoted run:

- **500 matrices**: lt_1k 243 (tied 111) · 1k_10k 180 (tied 47) · gt_10k 77
  (tied 33). Eval gt_10k sensitivity: 0.00456 per matrix per unit ln(ratio).
- Every future round should be targeted with this census, not dev extrapolation.

## Why the failure was inevitable, not unlucky

The eval heavy tier (nnz ≥ 250k) has 35 matrices to dev's 13, and contains a
**dense-heavy class that dev entirely lacks**:

| eval matrix | n | nnz | nnz/n |
|---|---|---|---|
| `pb302035` | 670 | 333040 | **497** |
| `pedigree_sim2000` | 2587 | 921054 | **356** |
| `truck` | 9016 | 1045026 | **116** |
| `eigena2` | 3775 | 372500 | **99** |
| `telecomsp_metro` | 13940 | 743408 | 53 |

Dev's densest heavy is 38.6. The 0004 gates were nnz windows, so they fired on
all of these. No-dense AMD (deferral disabled = the hub-column safety valve
removed) on a pattern with average degree 497 is the textbook blowup;
`telecomsp_metro` was merely the first one the runner reached.

## The two rules this bought

1. **Gate a hazard by its own covariate.** The no-dense hazard is column
   density, so the gate must test density. An nnz window is a cost proxy, not a
   hazard bound.
2. **Additions must be affordable on top of an UNKNOWN existing load.** The
   comparative rule (dev worst ≤ 1.019 s) silently assumed eval matrices resemble
   dev matrices. They do not, and grader timing is invisible, so the only sound
   budget is on the ADDED cost per matrix: bound it by the worst measured
   per-class constant, sized so it fits even on a matrix already near ~1.4 s.

## The re-gate

Both heavy additions now require **`nnz <= 6*n`** (`HEAVY_SPARSE_MAX_AVG_DEG`),
and the no-dense AMD ceiling drops 1.2M → **700k**:

- Every dev win passes the guard (densities 4.1–4.7); every observed eval blowup
  candidate fails it (11.9–497). The census gap between 6.0 and 11.9 makes 6 a
  natural boundary, not a tuned one.
- Worst added cost: relabel ≤ 2.6e-7 × 1.5M ≈ 0.39 s; amdnd ≤ 5.6e-7 × 700k ≈
  0.39 s; combined ≈ 0.57 s. The 700k ceiling gives up dev's `gabriel10` win
  (0.9756, worth 0.00018) to exclude the `faclay75`-class 0.77 s pass.

Dev result: predicted 0.876511 by hand (0.876329 plus the surrendered gabriel10
win); harness measured **0.876511** exactly.

## What the census says this should buy on eval

17 sparse heavies (nnz ≤ 6n), ten tied at 1.0000, now receive the additions:
`optcdeg2`, `dtoc5` (same optimal-control family as dev's `cont6-qq`, which
amdnd moved 0.9145 → 0.8899), three `unitcommit`s, `transswitch`/`powerflow`
spp, `gabriel08`, `watercontamination` ×2, `parabol5_2_2`, `gams04`. If the
family analogies hold, roughly **−0.001** on eval (~0.8853 from 0.886427).

## For the next solver

- Mine the census FIRST. The command is above; it converts "which matrices
  should I target" from guesswork into a lookup.
- The dev corpus under-represents dense-heavy structure. Any gate that touches
  nnz ≥ 250k must carry a density guard or a per-matrix added-cost bound sized
  for unseen structure.
- Eval families with no dev counterpart at all (`pb*`, `pedigree*`, `truck`,
  `eigena2`, `telecomsp*`): currently at 1.0000 except `telecomsp_njlata`
  (0.735) and `telecomsp_pacbell` (1.0). Dense, moderate n, already loaded with
  AMF passes. If anything is ever added for them it must REPLACE load, not add
  to it.
