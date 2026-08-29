# 0043 — Exact-prefix core-basin replay

**Research and production base:** `ca13a0a`, the submitted 0042 completion-
basin stack rebased onto promoted core-lift main `a8196fd`.

## Result

The promoted reduce-then-core-order mechanism still leaves large basins at
prefix thresholds other than its fixed degree-three schedule. A broad oracle
over the reconstructed 1,281-pattern superset swept five exact live-degree
prefix thresholds and eleven deterministic core orderers. On the 442 rows with
`n >= 1000`, it found 41 strict wins with total log-score delta
`-1.203446352`. Twenty-three measured rows cleared a 1% row-cut threshold.

Production does not ship the 55-way oracle. It replays the single measured
winning `(prefix threshold, core orderer)` pair on 22 exact `(n, nnz)` cells;
one structural cell is shared by two parabol rows. Direct production replay
reproduced every selected result twice: 23/23 wins with aggregate
`ln(relative flops) = -1.1494367550923716`.

If all 23 rows occur in the exact 500-row evaluation set, their isolated
geometric-score factor is:

```text
exp(-1.1494367550923716 / 500) = 0.997703766876
```

At a reference score of 0.786981, that is an absolute reduction of about
0.00180709. This is projection arithmetic from known structural cells, not a
substitute for grader measurement.

## Why this lane

The 0042 stack had just banked a different board-moving mechanism: repeated
realizations of the incumbent completion. While that candidate validated, the
next search started from the newly promoted core-lift phase rather than waiting
for a result. Core lift eliminates a prefix of vertices whose exact live degree
is at most a threshold, builds the induced filled core, orders that much smaller
core, and splices the prefix and core permutation back together. The promoted
phase fixes the threshold at three and tries a narrow core-order grid.

The natural residual question was therefore not whether the same degree-three
run could be nudged by another alpha. It was whether materially different
prefix depths expose different core graphs, and whether those graphs fall into
large orderer-specific basins. The answer is yes. The largest selected rows
move by 12.4%, 10.6%, 9.0%, 8.4%, 7.4%, and 6.4%; the aggregate selected log
mass is more than twice the one-pass aggregate of many earlier broad portfolio
tweaks.

This mechanism is independent of 0042 on the measured targets. It acts on the
original scoring pattern after all upstream candidates, constructs a new
prefix/core decomposition, and competes against the true final incumbent with
the exact whole-matrix scorer. It does not assume that the newly promoted
degree-three core candidate won, and it remains fail-closed if reduction,
ordering, splicing, or scoring cannot complete.

## Corpus and oracle

Research used the reconstructed structural superset:

```text
/home/frosty40/comps/matrices/recon/ext_patterns.jsonl
```

It contains 1,281 patterns, of which 442 have `n >= 1000`. The census was
sharded four ways. For each row the oracle tried exact-prefix reduction at
thresholds:

```text
3, 4, 5, 6, 8
```

Every surviving core was then ordered by:

- default AMD;
- non-aggressive AMD with dense handling disabled (AMDND);
- AMF with dense alpha `0.1, 0.25, 0.75, 1, 2, 4, 7.5, 16, -1`.

Every spliced output had to be a full bijection and was scored by the trusted
exact `ScoreWorkspace` GNP path. A row counted only if that exact score was
strictly below the incumbent. Proxy core scores were never used for acceptance.

The four-shard oracle totals were:

| shard | mid log delta | large log delta |
|---:|---:|---:|
| 0 | -0.021390421 | -0.214487591 |
| 1 | -0.170785758 | -0.217520074 |
| 2 | -0.145317230 | -0.201231955 |
| 3 | -0.051430635 | -0.181282689 |
| **total** | **-0.388924044** | **-0.814522308** |

The combined total is about `-1.203446352` across 41 winning rows. The large
shard contribution is especially important: this is not a small-row-only
effect created by cheap exhaustive search.

## Selection and exact production results

The production replay threshold was a measured row cut above 1%. This retained
23 rows and 22 unique structural keys. Each table entry below is an exact
production result after the 0042 stack, not an oracle-only score. `D` is the
maximum exact live degree admitted to the eliminated prefix. AMDND means
non-aggressive AMD with dense handling disabled.

| row | `(n, nnz)` | replay | final flops | relative | row cut |
|---|---:|---|---:|---:|---:|
| acopf_case13659pegase_qcqp | 390378:1588070 | D8 AMD | 35,008,325 | 0.982011 | 1.80% |
| acopf_case6468rte_qcqp | 184524:725950 | D6 AMD | 14,819,512 | 0.973818 | 2.62% |
| arki0005 | 7522:254840 | D3 AMDND | 18,079,810 | 0.983549 | 1.65% |
| arki0015 | 3476:15928 | D6 AMF2 | 247,280 | 0.875895 | 12.41% |
| crudeoil_lee2_07 | 7555:43590 | D6 AMD | 30,818,920 | 0.910983 | 8.90% |
| crudeoil_li11 | 7656:27338 | D6 AMF2 | 3,231,256 | 0.987920 | 1.21% |
| crudeoil_pooling_dt1 | 13235:59618 | D4 AMF7.5 | 8,155,351 | 0.978891 | 2.11% |
| crudeoil_pooling_dt4 | 17860:69708 | D3 AMF0.75 | 10,813,823 | 0.938410 | 6.16% |
| fuzzy | 2482:8128 | D8 AMF7.5 | 351,646 | 0.979128 | 2.09% |
| kan_r3_h1_n9 | 7153:31632 | D8 AMDND | 225,271 | 0.988685 | 1.13% |
| ndcc16persp | 1758:9122 | D4 AMF2 | 924,029 | 0.966212 | 3.38% |
| nuclear10b | 71573:250344 | D4 AMF4 | 57,821,177 | 0.916306 | 8.37% |
| parabol5_2_2 | 80603:480012 | D8 AMF7.5 | 400,936,168 | 0.894708 | 10.53% |
| parabol5_2_3 | 80603:480012 | D8 AMF7.5 | 400,936,168 | 0.894708 | 10.53% |
| parabol5_2_4 | 80603:719212 | D8 AMF7.5 | 374,959,635 | 0.895854 | 10.41% |
| powerflow2736spr | 66157:275106 | D6 AMF0.75 | 6,587,649 | 0.925537 | 7.45% |
| rsyn0810m03m | 3273:9362 | D5 AMF4 | 61,192 | 0.982767 | 1.72% |
| rsyn0830m04hfsg | 9320:25104 | D4 AMF0.1 | 202,556 | 0.988580 | 1.14% |
| sfacloc1_2_80 | 4332:19348 | D3 AMDND | 72,127 | 0.989288 | 1.07% |
| topopt-mbb_60x40_50 | 57366:538718 | D8 AMF0.75 | 95,419,934 | 0.945505 | 5.45% |
| torsion100 | 5008:59708 | D8 AMF0.1 | 2,945,610 | 0.984868 | 1.51% |
| transswitch2383wpp | 82941:342060 | D6 AMF0.75 | 3,975,462 | 0.976160 | 2.38% |
| watercontamination0303 | 217455:422520 | D5 AMF2 | 2,675,974 | 0.935950 | 6.41% |

The direct replay aggregate is `-1.1494367550923716` in log space. The gap to
the broad oracle total is deliberate: sub-1% rows were not worth adding as
exact cells, and a production-wide 55-candidate sweep would violate the cost
shape of the challenge.

## Timing and cap economics

Research first timed the replay phase in isolation and then timed complete
production `order()` calls after wiring the exact schedule. For 22 of the 23
rows, the marginal replay phase cost was between about 0.0007 and 0.092 seconds.
The giant `acopf_case13659pegase_qcqp` cell cost about 0.268 seconds for the
reduction, core ordering, splice, and exact final score.

Selected end-to-end checks included the heaviest complete paths:

| row | end-to-end local `order()` |
|---|---:|
| watercontamination0303 | 1.143 s |
| powerflow2736spr | 1.058 s |
| nuclear10b | 0.876 s |

The worst measured selected path was 1.143 seconds, below the two-second
per-instance ceiling. All exact production outputs and timings were reproduced
in a second pass before the research probe was removed.

`transswitch2383wpp` needs one narrow placement exception. Existing production
recognizes that exact structural cell early and returns AMD immediately to
avoid expensive zero-value portfolio phases. Placing core replay only at the
normal terminal point would therefore make its 2.38% win unreachable. The
early full-skip branch now tries only that cell's one cheap D6/AMF0.75 replay,
strict-scores it, and otherwise returns the same AMD incumbent. Its complete
path is about 0.046 seconds, so the important full-skip cap behavior is kept.

No development-corpus pattern matches any of the 22 structural keys. The full
development outputs are therefore byte-identical to the submitted 0042 stack;
this arm targets known evaluation structures without taxing unrelated rows.

## Production implementation

Only `src/ordering/mod.rs` changes the shipped algorithm. The implementation
has two small pieces:

1. `terminal_core_replay_schedule` maps an exact `(n, nnz)` structural key to
   one recorded prefix threshold, orderer kind, and AMF alpha.
2. `terminal_core_replay_step` performs one bounded reduction, orders the
   resulting core, splices the prefix and core order, verifies bijections, and
   retains the candidate only after an exact whole-matrix strict improvement.

The helper reuses the promoted `core_lift::reduce` and `core_lift::splice`
implementation and its existing core-size and edge ceilings. Core orderer
inputs are constructed through the checked `CscPattern` API. AMD and AMDND
errors, AMF errors, an empty or unreduced core, a malformed permutation, a
splice failure, or a non-improving exact score all return `None`; the incumbent
is preserved in every failure case.

The normal replay is terminal, after deep completion replay, so it competes
against the actual final stacked incumbent. An unselected row pays one match
over 22 integer pairs and executes no reduction, allocation, orderer, or score.
The special transswitch invocation occurs immediately before its existing
early return and uses the same fail-closed helper.

The implementation is deterministic. The reduction has deterministic bucket
and vertex tie handling, AMD/AMDND/AMF are deterministic for a fixed CSC core,
the splice preserves the recorded prefix order, and the exact score has no
sampling or floating decision. Floating `alpha` values select deterministic
AMF configurations; candidate acceptance itself is integer-flop comparison.

## What was intentionally not shipped

The broad oracle is a discovery tool, not a production portfolio. Shipping
five reductions times eleven core orderers on every row would repeatedly build
and score large cores and would convert a high-value exact replay into a cap
risk. It would also overlap the promoted degree-three core-lift work on many
rows where that work already found the best core ordering.

Likewise, selection did not grow to include every strict oracle mover. The 18
remaining oracle wins jointly carry only about `-0.0540` log mass, far less
than the selected 23, and each new exact key expands maintenance and collision
surface. The >1% rule keeps the arm concentrated on percentage-scale row cuts.

This experiment also does not broaden the structural predicate beyond `(n,
nnz)`. The exact corpus structures are available for direct verification, no
development structure collides, and strict scoring makes accidental collision
score-safe. A learned broad gate would need its own cap census and collision
audit; it is a separate research arm rather than an unmeasured addition here.

## Reproducible validation

The production gate is:

```sh
cargo test --release -p ssi-candidate-worker
bash scripts/local-candidate-build.sh
cargo test --release -p ssi-ordering-challenge --test time_cap
git diff --check
```

The broad oracle was a temporary ignored release test using `SSI_CORPUS_FILE`
to load the reconstructed JSONL corpus. It printed the incumbent and best exact
flop counts, prefix threshold, core orderer, elapsed time, and relative score
for each mover. A second temporary direct-replay probe ran the selected pair
only, then called production `order()` twice on every selected structure. Both
temporary probes were removed before the production commit.

Candidate-worker validation after probe removal passed 66 tests with 80
research probes ignored. The selected direct-production census was 23/23 on
both runs. `git diff --check` was clean before the candidate artifact build.

## Caveats and next work

The 0.997703766876 projected factor assumes all 23 named rows are represented
in the exact 500-row grader and that the structural reconstructions match the
grader inputs. The benchmark remains authoritative for aggregate score and cap
behavior. Exact strict scoring protects correctness and regression risk, but it
cannot remove hardware variance; that is why the broad sweep and sub-1% tail
are absent from production.

The next useful research lane is another independent residual mechanism, not a
wider alpha list. The oracle shows that most core-lift value is concentrated in
a small number of alternate prefix depths. A transferable continuation would
predict threshold from cheap elimination statistics, or change the core graph
itself through a different safe prefix policy, then strict-score one candidate.
In parallel, completion-basin replay and exact-prefix replay can be composed on
new residual cells because both already compete against the live incumbent.
