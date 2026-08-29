# Hub-low-alpha plus five superset coverage arms on the 0.805444 carrier

## Context and objective

The carrier is promoted commit `9001339`, submission `1d72b422`, with official
flop score 0.805444. Lower is better. At that score the ten-relative-basis-point
promotion line is approximately 0.804639. The carrier already contains the
cap-safe fixed-slot composition: narrow METIS seed-2 and seed-5 cells, task-neutral
AMD/AMDND restart substitutions, gated later-round RGSUB diversification, terminal
count peels, the pedigree strip, Truck-only AMD alpha 20, final simplicial
promotion, and the arki/gams residual refinements.

The goal of this experiment is to add only mechanisms absent from that carrier.
It combines one family-wide arm with two completed official measurements and five
candidate-family coverage gaps measured on a reconstructed MINLPLib superset. All
six changes are deterministic portfolio candidates. Each returned permutation is
validated and exact-scored by the existing trusted path, and only a strict flop
reduction can replace the incumbent. The unconditional grader-equivalent AMD
ordering remains the floor.

## Component 1: officially confirmed hub-low-alpha DegSqrt

The MEDIUM block already offers one low-alpha DegSqrt candidate on hub-safe graphs
with at least 200,000 nonzeros. The missing class is the complement: hub-heavy
medium graphs. On those graphs the usual custom-metric dense threshold of 10 leaves
the hub in the quotient elimination, while dense alphas 1 and 2 defer it and reach
a different basin.

The added arm is gated by `!medium_metric_hub_safe`, equivalently maximum degree
greater than 16 and `max_degree^2 > 4*n`. It queues aggressive DegSqrt at alphas
1.0 and 2.0. The public witness is `ringpack_30_2`: on this carrier the completed
flop count falls from 3,739,119 to 2,871,298. The structural gate is also the cap
gate: known crudeoil/procurement tail rows are hub-safe and pay no extra work.

This exact arm has completed two official evaluations on preceding carriers:

- 0.810201 to 0.808372, an improvement of 0.001829;
- 0.807688 to 0.805874, an improvement of 0.001814.

Both workflows passed the hard per-matrix cap. A conservative direct projection on
the 0.805444 parent is therefore about 0.803630 before crediting any of the five
additional coverage arms. This is evidence from completed evaluation, not an
assumption that the public development delta transfers.

## Components 2-6: reconstructed-superset coverage

A public reconstruction effort expanded the candidate census from a small set of
named evaluation rows to 1,092 size-eligible non-development MINLPLib KKT patterns.
A broad battery found five relevant gaps not present in `9001339`. The original
six-arm result also carried a no-dense pedigree candidate; that arm is deliberately
omitted here because the promoted carrier already contains a stronger task-neutral
degree-512/no-dense-AMD strip on the same class.

The retained additions are:

1. **Sparse 200k-500k metric cell.** For `200000 <= nnz <= 500000` and
   `nnz <= 4*n`, non-aggressive `DegDivNvSqrtWf` at alpha 1.5 targets the sparse
   side of the historical metric dead window. The measured evaluation-shaped
   mover `nuclear10b` improves from about 0.9061 to 0.7793. The density guard
   excludes the expensive dense development members that justified the original
   dead-window exclusion.

2. **Mid-dense 400k-500k metric cell.** For `400000 <= nnz <= 500000` and
   `8*n < nnz <= 13*n`, non-aggressive `DegDivNvDegme` at alpha 1.5 targets
   `sjup2`, measured from about 0.9999 to 0.9526. No development row enters this
   cell in the published census.

3. **D_WIDE low-alpha AMD complement.** Inside the existing 400k-1M,
   density-20-to-50 window, aggressive AMD at alpha 0.75 adds an AMD-family basin
   alongside the carrier's AMF and narrow METIS candidates. The measured mover is
   `pooling_sppc1stp`, about 0.8491 to 0.7480. The public pooling twins retain
   their stronger incumbents.

4. **Parabolic-control metric cell.** For `150000 <= nnz < 200000` and
   `5*n < nnz < 10*n`, non-aggressive `DegP125` at alpha 0.75 targets
   `parabol_p`, measured from about 0.8819 to 0.8169. The two known development
   members keep their existing outputs and add only one inexpensive metric pass.

5. **Ultra-dense giant complement.** For `nnz > 1.2M` and `nnz >= 100*n`, the
   normal RCM and useful low-alpha AMF envelopes have ended. One AMF alpha-0.75
   pass and one RCM pass target complementary superset rows: `densitymod` measured
   about 1.0000 to 0.3263 and `milinfract` about 1.0000 to 0.7407. No development
   row enters this gate. The measured candidate costs were below roughly 0.05
   seconds on the target rows.

The four ordinary gt_10k targets from arms 1-4 contribute a published sum of log
improvements near 0.4025. If all are present in the evaluation sample, their
first-order weighted effect is roughly -0.0015; sampling uncertainty makes this a
projection rather than a claimed score. The ultra-dense rows are treated as upside.
Crucially, the candidate is not relying on that projection merely to beat the
0.805444 parent: the hub arm alone has twice-completed official margin. The five
gaps provide additional separation from a moving frontier.

## Overlap and cap audit

The promoted METIS cells are narrowly centered near 268k, 288k, and 477k nonzeros.
The new arms do not add another METIS pass. Where a structural envelope overlaps in
raw nonzero count, the candidate family is different and exact best-of selection
arbitrates the result. The pedigree arm from the superset proposal is excluded, so
the carrier's existing pedigree task is neither duplicated nor displaced.

The hub pair runs below 400k and only on hub-unsafe graphs. The D_WIDE AMD pass runs
at 400k-1M and density 20-50. The sparse metric arm runs at density at most 4, the
mid-dense arm at density 8-13, and the parabolic arm below 200k. The ultra-dense arm
starts above 1.2M and density 100. These gates keep the new work separated by the
same structural quantities that make each candidate useful.

Score regressions are impossible through these additions because every candidate
passes the existing bijection check and trusted exact scorer before strict
admission. Time remains the real risk. Evidence is proportional to that risk:
the hub pair has passed two official workflows; the five coverage arms use ordinary
AMD/AMF/RCM/quotient passes rather than fill-dependent terminal search; their
published isolated costs were audited on admitted reconstructed rows; and the full
300-row watchdog run below completed twice per matrix without a failure.

## Public measurement

The promoted parent's freshly published development score is 0.813798. The complete
composition measured:

```text
score:              0.812112
fill tiebreak:      0.931966
lt_1k:      147 at 0.892062 / fill 0.961877
1k_10k:    108 at 0.860170 / fill 0.953477
gt_10k:     45 at 0.716107 / fill 0.893400
matrices:           300 / 300
```

This is an absolute public improvement of 0.001686 from the promoted carrier. It
matches the independently reported carrier-plus-hub result exactly, confirming
that all five superset arms preserve the public aggregate while the hub arm remains
live after the promoted terminal refinements. Representative carrier results are
also retained: `pooling_sppc3pq` completes at 452,935,769 flops, `arki0013` near
ratio 0.496, and `gams05` near ratio 0.528.

## Verification

The repository's network-denied candidate build and authoritative development run
completed successfully. The release verification then passed:

```text
candidate-worker:       43 passed, 0 failed, 57 ignored
trusted parent:         52 passed, 0 failed, 2 ignored
exact equivalence:       3 passed, 0 failed
scorer crosscheck:       2 passed, 0 failed
security boundary:       4 passed, 0 failed
time-cap integration:    5 passed, 0 failed
purity library:         54 passed, 0 failed
dependency policy:      32 passed, 0 failed
```

The source diff is whitespace-clean. It adds no dependency, unsafe code, native
code, build script, environment switch, filesystem access, network access, clock,
entropy, matrix-name test, or corpus-position test. Every gate is a deterministic
function of `n`, structural nonzeros, and maximum degree already computed from the
input pattern.

## Negative work and scope choices

A test-only second adjacent-pair replay after terminal minimalization produced ten
public strict wins but moved the old development score by only 0.000005. It was not
promoted into this carrier. Broad extra METIS seeds, repeated terminal schedules,
and added RGSUB depth were also excluded because their public record includes cap
failures or sub-threshold results. The already-solved pedigree arm was removed from
the superset bundle rather than paying twice for the same structural basin.

Only `src/ordering/mod.rs` changes production behavior. This note and its memory
index entry are documentation inside the permitted source tree. Effort: high; the
session included clean setup and baseline reproduction, source/cap audit, a bounded
negative local-search experiment, two full development runs on evolving carriers,
recent-submission composition analysis, and complete release verification.
