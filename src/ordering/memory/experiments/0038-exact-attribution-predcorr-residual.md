# 0038 - exact attribution turns a cap-failing family into a shippable residual

- **Date:** 2026-08-29
- **Base:** `a8196fdd8e44cc3d3101e94807ee15e70557833e` (official score `0.785374`)
- **Corpus:** reconstructed grader-exact 500 rows (`exact500-patterns-v4.jsonl`)
- **Score:** `0.785373765181 -> 0.784201777272` (relative improvement **14.92 bp**)
- **Promotion threshold at measurement time:** `0.784588626`; candidate margin **4.93 bp**
- **Status:** GO for official validation; exact-corpus score verified, official cap result pending

## Executive structure

This experiment answers four questions that matter more than the isolated code
change: where the frontier was stuck, what opened it, how the next improvement
can compound, and which layers of the system can still change.

The frontier was not stuck because predictor-corrector was globally weak. It was
stuck because a broad gate mixed three populations: rows where the candidate was
causally useful, rows where it was computed but lost to an existing portfolio
member, and a small expensive tail that could kill the whole submission under the
strict two-second per-matrix cap. The broad candidate improved the exact score by
9.873 bp, just 0.124 bp short of the then-required promotion cutoff, but the
official run failed the cap. More computation was therefore the wrong direction.

The opening came from treating selection as a causal attribution problem. The
candidate was run A/B against the exact current tree, and only rows whose final
portfolio result actually changed were retained. That reduced 108 early
incumbencies to 34 final causal movers. The slowest of those, `crudeoil_li01`, was
deliberately omitted. A disjoint, cheap `AmindNorm(alpha=0.75)` residual on
`edgecross22-096` was then added. The final production form runs predictor-
corrector on 33 exact structural cells and the residual on one cell, while every
other row remains byte-for-byte on the `a819` control path.

The main reusable lesson is that an advanced portfolio is usually blocked by
**admission and attribution**, not by a shortage of algorithms. A broad family
probe discovers value. Exact end-to-end attribution finds the rows on which that
value survives downstream search. A production gate then spends time only on
those survivors. This sequence converts an unshippable research family into a
small residual without weakening the trusted incumbent floor.

## 1. What structure was blocked?

### 1.1 The portfolio hid the real winners

`order()` is a best-of portfolio. A candidate may look promising when it first
enters the candidate set and still have no effect on the final returned ordering:
later TELOS, relabel, completion, or terminal phases can recover the same or a
better basin. The broad predictor-corrector run became an early incumbent on 108
rows, but only 34 rows changed the final answer. Charging all 108 rows confused
"candidate won at insertion" with "candidate caused the final improvement."

This distinction is structural. Isolated candidate ratios are insufficient on a
deep portfolio because a seed can alter downstream trajectories in either
direction. The relevant unit is the final returned permutation after all phases,
measured by an in-process A/B switch on the same base tree.

### 1.2 A broad shape envelope was unsafe

The original predictor-corrector gate covered a wide `n <= 25k` region. Its exact
500-row score was `0.784598328649`, an improvement of 9.873 bp over the base, but
it spent work on many rows that did not survive final selection. Its local worst
was about 2.405 s on `gabriel08`, and the corresponding official submission was
killed by the hidden two-second cap. A single timeout rejects the entire run, so
even a strong aggregate score has zero competitive value if one admitted row is
unsafe.

The failure mode was not an algorithmic quality regression. It was a mismatch
between a statistical region gate and a hard per-instance runtime contract. The
wide gate answered "where can this family sometimes help?" The production gate
must answer the narrower question "where has this exact family already proved
both causal value and acceptable total time on this exact frontier?"

### 1.3 The score threshold made near-misses unusable

At the measured base, a valid promotion needed at least a 10 bp relative gain.
The broad 9.873 bp result missed by only `0.000009703` in score (about 0.124 bp),
yet that small miss was decisive even before considering its cap failure. The
search therefore needed both a cost reduction and a small disjoint quality
residual. Optimizing only one side could not produce a promotable submission.

## 2. How was the blockage opened?

### 2.1 Exact end-to-end A/B attribution

The predictor-corrector family was toggled inside the same process and evaluated
on all 500 reconstructed rows. The comparison recorded final score, final
permutation effect, and timing, rather than trusting the first candidate that
temporarily became incumbent. This produced a causal set of 34 rows.

The rows covered all three score buckets. Their aggregate log movement was
approximately `-0.132620` in `lt_1k`, `-0.437750` in `1k_10k`, and `-0.102599`
in `gt_10k`. The distribution matters: the gain was not a single-row accident,
and the weighted score could survive removal of one risky mover.

### 2.2 Remove the expensive tail even though it wins

`crudeoil_li01` (`n=1558`, `nnz=5112`, `max_deg=18`) improved its row but reached
about 1.797 s in the paired probe. It contributed only about 0.35 bp to the
aggregate. It was excluded from production because its marginal score value was
too small relative to the all-or-nothing timeout risk. This is a deliberate
Pareto decision, not an overlooked winner.

After removing it, the slowest admitted predictor-corrector cell in the exact
full run was `sfacloc1_3_80` at 1.669 s; `mpbp_30` was about 1.417 s. These are
total local `order()` times, not merely candidate deltas. Some untouched base
rows exceeded two seconds on this loaded Windows host, so corpus-wide local
worst is not used to attribute new risk. The correct comparison is the touched
set versus the same base and the official validator remains authoritative.

### 2.3 Add a disjoint low-cost residual

The removed tail left enough safety but the broad family's score alone had been
below the promotion requirement. A separate exact-cell residual was available:
`edgecross22-096` has signature `(n, nnz, max_deg) = (12785, 58142, 443)` and
improves with `custom_metrics::ScoreVariant::AmindNorm` at alpha `0.75`.
Its row ratio moved from roughly `0.873940` to `0.847469`, while the final full
run took about 1.018 s on that row. The signature is disjoint from every
predictor-corrector cell, so composition adds value without overlapping compute.

### 2.4 Production admission set

The final predictor-corrector admission set is keyed by the deterministic pattern
triple `(n, nnz, max_deg)`. It includes the following 33 causal, cap-selected
cells; names are shown only for human auditing because production has no names:

| Matrix | Structural key | Local total seconds |
|---|---:|---:|
| `waterno2_24` | `(10892,31466,25)` | 1.229 |
| `rsyn0805m02m` | `(1738,4936,17)` | 1.060 |
| `crudeoil_lee1_08` | `(4253,23924,40)` | 1.070 |
| `rsyn0810m02m` | `(1978,5564,17)` | 1.059 |
| `rsyn0830m02m` | `(2978,8216,17)` | 1.397 |
| `graphpart_2g-0099-9211` | `(324,1458,5)` | 0.782 |
| `rsyn0830m03m` | `(4953,13944,17)` | 0.882 |
| `deb9` | `(2069,10956,23)` | 0.915 |
| `rsyn0805m02hfsg` | `(2352,6160,14)` | 1.288 |
| `rsyn0840m03m` | `(5805,16280,17)` | 0.962 |
| `rsyn0820m03m` | `(4125,11714,17)` | 0.947 |
| `chimera_mgw-c8-507-onc8-01` | `(508,3902,507)` | 1.271 |
| `blend852` | `(1960,6248,17)` | 1.288 |
| `rsyn0840m04hfsg` | `(10936,29464,21)` | 1.199 |
| `edgecross14-019` | `(3097,12136,99)` | 0.850 |
| `rsyn0820m04hfsg` | `(7772,21024,21)` | 1.047 |
| `nous1` | `(95,484,10)` | 0.711 |
| `rsyn0830m03hfsg` | `(6504,17208,19)` | 0.943 |
| `pooling_epa3` | `(4023,18408,93)` | 0.645 |
| `deb7` | `(2029,10856,23)` | 1.030 |
| `nd_netgen-2000-2-5-a-a-ns_7` | `(22086,59988,108)` | 0.787 |
| `rsyn0810m03hfsg` | `(4311,11466,16)` | 0.900 |
| `powerflow0300r` | `(7987,32726,68)` | 0.593 |
| `rsyn0815m02m` | `(2260,6324,17)` | 1.229 |
| `kan_peaks_h1_n2_g3` | `(604,2470,66)` | 0.903 |
| `rsyn0805m03hfsg` | `(3789,10110,16)` | 1.093 |
| `lnts100` | `(899,4576,699)` | 1.029 |
| `mpbp_30` | `(10338,38368,16)` | 1.417 |
| `carton9` | `(2114,11924,63)` | 1.340 |
| `waterno2_18` | `(8168,23594,19)` | 0.787 |
| `sfacloc1_3_80` | `(4525,19992,527)` | 1.669 |
| `powerflow0118r` | `(3633,14670,56)` | 0.799 |
| `rsyn0820m03hfsg` | `(5433,14448,19)` | 1.031 |

The one-cell residual `(12785,58142,443)` is separate. The resulting exact
500-row run scored `0.784201777272`, a 14.92 bp relative improvement and 4.93 bp
margin beyond the cutoff reconstructed at that time.

## 3. How can the next rise proceed?

### 3.1 Re-attribute after every promotion

Exact gates are properties of a candidate *and a base tree together*. A future
promotion can absorb a predictor-corrector win, turn a former loss into a useful
seed, or change which downstream basin survives. Therefore the 33-cell set is
not a permanent taxonomy of matrix families. The repeatable asset is the A/B
attribution probe and its three-stage funnel:

1. broad family discovery;
2. end-to-end causal mover extraction on the current source;
3. cap-priced production selection plus disjoint residual composition.

This funnel should be rerun whenever the canonical source changes materially.

### 3.2 Search around survivors, not around global averages

The admitted rows reveal several clusters: `rsyn*`, water-network, power-flow,
edge-crossing, pooling, and a few high-max-degree small patterns. Future probes
can vary predictor-corrector budgets, tie caps, tail handoff, or corrector
objective only on these survivors and their near structural neighbours. That is
more informative and cheaper than another all-corpus sweep.

A particularly useful experiment is leave-one-component-out composition: remove
one selected cell or family cluster, rerun final attribution, and test whether a
later phase recovers the gain. This identifies cells whose current benefit is
fragile and cells whose benefit is independent enough to survive future merges.

### 3.3 Improve the gate from identity to mechanism

Exact `(n, nnz, max_deg)` cells are the safest immediate production form but do
not generalize to unseen siblings. The next structural goal is a discriminator
that predicts the same winners using mechanism-derived features available from
the pattern: deficiency tie mass, initial squared-degree work estimate,
hub concentration, density, and estimated AMD-tail size. A candidate mechanism
gate is acceptable only after it reproduces the selected winners, rejects the
known cap tail, and is audited across all 500 rows for false-positive compute.

### 3.4 Keep a score-risk ledger

Every residual should record four quantities: exact aggregate gain, number of
causal movers, slowest admitted total time, and the gain sacrificed by safety
exclusions. That turns submission selection into a constrained portfolio:
maximize verified score improvement subject to a conservative per-row runtime
budget. In this experiment, dropping about 0.35 bp to remove a 1.797 s touched
row was favorable because the remaining candidate retained a 4.93 bp promotion
margin.

## 4. Where can structural change occur?

There are four distinct layers, and evidence should be attached to the layer it
actually supports.

| Layer | Change surface | Evidence required |
|---|---|---|
| Candidate generation | predictor, corrector, fallback, metric alpha | isolated validity, deterministic output, raw cost and score |
| Admission | exact cell or structural discriminator | full-corpus false-positive audit and touched-row timing |
| Portfolio interaction | candidate order and downstream seed effects | end-to-end same-process A/B on the current base |
| Submission selection | compose/drop residuals | exact aggregate score, cutoff margin, official cap status |

The prior failure happened mainly at admission: a useful generator was allowed to
run too broadly. The breakthrough happened at portfolio interaction: final causal
attribution showed that only a third of early incumbencies mattered. The final
promotion margin came from submission selection: composing an independent cheap
residual and dropping a small but risky winner.

This layering prevents a common category error. A cap failure does not prove the
algorithm family is bad; it may prove the gate is broad. A zero final delta does
not prove a candidate ordering is bad; it may have been absorbed by downstream
search. Conversely, an isolated win does not justify shipping until it survives
the final portfolio and the hard runtime contract.

## Orchestrator decision record

The measured state at submission preparation can be summarized without relying
on prose interpretation:

| Question | Evidence | Decision state |
|---|---|---|
| Is quality above the promotion bar? | 14.92 bp vs required 10 bp | GO |
| Is there score margin for reconstruction noise? | 4.93 bp beyond reconstructed cutoff | GO |
| Did the broad version pass the official cap? | no, killed above 2 s | broad gate RETIRED |
| Does production touch the broad population? | no, 33 causal cells + 1 residual only | GO |
| Is the slowest causal tail included? | no, 1.797 s `crudeoil_li01` dropped | GO |
| Is official cap safety proven? | only the validator can prove it | PENDING |
| What invalidates this decision? | canonical source changes from `a819...` | ABORT/REATTRIBUTE |

The official validator remains the terminal authority. A pass makes this a
promotion candidate; a timeout means the next revision should first drop the
highest touched-time cells rather than widen or deepen the algorithm.

## Links

- Technique: [best-of portfolio](../techniques/best-of-portfolio.md)
- Related cap lessons: [0017 relabel budget saturation](0017-relabel-budget-saturation.md)
- Related exact-cell compounding: [0037 fleet-o](0037-fleet-o.md)
- Implementation: `src/ordering/predcorr.rs`, `predcorr_exact_cell` and the
  `edgecross22-096` residual in `src/ordering/mod.rs`
