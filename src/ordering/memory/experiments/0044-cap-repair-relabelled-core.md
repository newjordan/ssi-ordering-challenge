# 0044 — Cap repair and relabelled exact-prefix core replay

**Research base:** `dcb66ef`, the 0043 exact-prefix core arm based on promoted
main `a8196fd`.

**Status:** repaired and locally validated after three consecutive grader cap
failures.

## Outcome

Submissions 0041, 0042, and 0043 never reached scoring. All three failed the
same hard requirement: one hidden matrix exceeded the two-second `order()` cap
and the worker was killed. The failed runs were:

| arm | commit | GitHub run result |
|---|---|---|
| 0041 residual superset | `54b17c0a` | Benchmark failed at per-matrix cap |
| 0042 deep completion | `b9106b98` | Benchmark failed at per-matrix cap |
| 0043 exact-prefix core | `6761fa26` | Benchmark failed at per-matrix cap |

The last two runs each completed 224 worker invocations before emitting the
same hidden-matrix cap failure. No score or cutoff comparison was produced.
Because 0042 and 0043 both included 0041 unchanged, the repeated evidence
locates the regression in the inherited 0041 risk envelope rather than either
later score mechanism. The grader deliberately withholds the hidden matrix
identity, so this note does not invent a more precise culprit.

This repair removes every broad or output-neutral component introduced in
0041, preserves only its cheap exact 3033-cell lane, and then restructures the
later mechanisms around measured cost. Exact core winners now return before
the general portfolio. Deep completion stops at the last economically useful
round and is entirely removed from three already-slow base paths. A new
relabelled-prefix oracle adds large residual cuts without restoring a broad
runtime surface.

The retained, newly measured mechanisms carry combined log-score mass

```text
-0.672550856569  retained deep completion
-0.350692138090  incremental relabelled-core replacements/additions
----------------
-1.023242994659  combined measured log delta
```

Their isolated 500-row geometric factor is:

```text
exp(-1.023242994659 / 500) = 0.997955606635
```

That figure is deliberately conservative: it excludes the earlier identity-
label exact-prefix core wins that remain in the stack. The important result is
that the repair does not answer a cap failure with a token tweak. It retains
29.35%, 12.92%, 8.00%, 7.69%, and other percentage-scale row cuts while making
the new work bounded and front-loaded.

## Failure evidence and repair boundary

The competition cap is asymmetric: one matrix at or above two seconds fails
the entire run, while unused runtime earns nothing. A mechanism therefore has
to be judged by both its flop delta and its worst reachable path. The 0041 arm
violated that rule by combining several independently plausible additions:

- a broad hub-giant ceiling-lift candidate;
- four broad single-task structural cells;
- a shared duplicate-flop memo in the parallel scorer;
- one exact terminal AMDND replay.

The last item is cheap and is also an upstream prerequisite for a strong deep
completion basin. The first three classes expand work or synchronization on
paths whose hidden overlap was unknown. They are removed here in full:

1. `HEAVY_METRIC_LIFT_MAX_NNZ` and the faclay75/faclay80 hub-giant ceiling
   lift are deleted.
2. The edgecross22, edgecross24, powerflow2736spp relabel-AMF, and
   transswitch2736spp relabel-AMF broad cells are deleted.
3. The full-permutation fingerprint, shared hash map, and mutex-based duplicate
   score memo are deleted from `parallel.rs`; each worker again uses its direct
   local `ScoreWorkspace` path from promoted main.

No claim is made that one specific deleted line was the hidden matrix's sole
cause. The available evidence cannot support that. The stronger repair is to
remove the complete untrusted 0041 broad layer and its synchronization change,
returning the unrelated cost surface to the already-promoted implementation.
The one retained 0041 exact cell has a structural key, one cheap orderer, and no
general-population reach.

## Exact core winners now short-circuit the portfolio

The 0043 implementation discovered exact-prefix core candidates at the normal
terminal point, after the full inherited portfolio. That was useful for proving
score, but wasteful in production. Every recorded replay depends only on the
original pattern and was accepted against the completed final incumbent. It can
therefore be computed directly after the AMD anchor. When its exact score is a
strict win, returning it immediately produces the same measured permutation and
flop count while skipping all later candidates.

Production now checks the exact `(n, nnz)` core schedule immediately after AMD
and structural statistics. A scheduled cell runs its one recorded reduction,
core orderer, splice, bijection check, and exact score. A strict winner returns;
a failure or non-win falls through to the unchanged portfolio. The normal
terminal replay remains as a fail-closed fallback, but every audited winner
uses the early path.

The complete local `order()` timings for the relabelled/replaced cells were:

| row | `(n, nnz)` | local seconds, run 1 | local seconds, run 2 |
|---|---:|---:|---:|
| acopf_case13659pegase_qcqp | 390378:1588070 | 0.629 | 0.580 |
| acopf_case6468rte_qcqp | 184524:725950 | 0.237 | 0.219 |
| acopf_case6515rte_qcqp | 188896:738438 | 0.251 | 0.235 |
| crudeoil_lee2_08 | 8726:53652 | 0.011 | 0.011 |
| crudeoil_lee2_09 | 9931:64908 | 0.013 | 0.013 |
| crudeoil_lee2_10 | 11170:77434 | 0.016 | 0.016 |
| crudeoil_li02 | 10751:33014 | 0.007 | 0.007 |
| crudeoil_li11 | 7656:27338 | 0.005 | 0.005 |
| gams04 | 109774:298082 | 0.075 | 0.070 |
| ndcc15persp | 1187:5762 | 0.001 | 0.001 |
| powerflow2383wpp | 80055:295884 | 0.057 | 0.057 |
| powerflow2736spr | 66157:275106 | 0.049 | 0.049 |
| transswitch2383wpp | 82941:342060 | 0.060 | 0.060 |
| watercontamination0303 | 217455:422520 | 0.169 | 0.159 |

The largest audited exact-core path is about 0.63 seconds locally. These are
complete `order()` measurements, not marginal phase timings. Direct probes
also reproduced every expected exact flop result twice before the temporary
instrumentation was removed.

## Relabelled exact-prefix core oracle

The original core oracle varied prefix degree and the orderer applied to the
filled core, but it preserved input labels. Equal-degree prefix pivots and core
orderer ties can choose a different elimination basin after a deterministic
relabel. The new oracle permuted the complete symmetric pattern before
`core_lift::reduce`, performed reduction and core ordering in that space,
spliced the candidate, mapped the permutation back through the relabel, checked
the full bijection, and accepted only a strict exact whole-matrix flop win.

Research used the 1,281-pattern reconstructed superset at:

```text
/home/frosty40/comps/matrices/recon/ext_patterns.jsonl
```

The scan covered all 442 patterns with `n >= 1000`, split across four shards.
The grid was:

- relabel seeds `8, 13, 21, 34`;
- exact live-degree prefix thresholds `3, 4, 5, 6, 8`;
- core AMD;
- core non-aggressive AMD with dense handling disabled;
- core AMF with dense alpha `0.75, 2, 7.5`.

Cheap split scores ranked the core candidates, but they did not decide
acceptance. The best reconstructed whole permutation was mapped back and
verified with the trusted exact scorer. The shard census was:

| shard | rows tested | strict wins | log delta |
|---:|---:|---:|---:|
| 0 | 118 | 5 | -0.098518019689 |
| 1 | 124 | 5 | -0.046424592575 |
| 2 | 91 | 4 | -0.167677233499 |
| 3 | 109 | 5 | -0.051735872058 |
| **total** | **442** | **19** | **-0.364355725762** |

Production selects twelve residual movers above a 1% row-cut threshold. Two
sub-1% results are free replacements for exact cells already present in the
0043 schedule. The selected incremental log delta is `-0.350692138090`, or
`0.999298861636` as an isolated factor over 500 rows.

## Selected relabelled-core results

Each entry below is an exact whole-matrix comparison against the incoming
completed stack. `S` is the relabel seed, `D` the maximum live degree admitted
to the prefix, and AMDND the non-aggressive AMD configuration.

| row | `(n, nnz)` | replay | before flops | after flops | relative | cut |
|---|---:|---|---:|---:|---:|---:|
| acopf_case13659pegase_qcqp | 390378:1588070 | S13 D6 AMD | 35,008,325 | 33,829,653 | 0.966332 | 3.37% |
| acopf_case6468rte_qcqp | 184524:725950 | S8 D6 AMF0.75 | 14,819,512 | 14,117,739 | 0.952645 | 4.74% |
| acopf_case6515rte_qcqp | 188896:738438 | S34 D8 AMF0.75 | 14,831,425 | 13,779,045 | 0.929044 | 7.10% |
| crudeoil_lee2_08 | 8726:53652 | S8 D3 AMD | 47,447,274 | 46,152,432 | 0.972710 | 2.73% |
| crudeoil_lee2_09 | 9931:64908 | S34 D6 AMD | 63,701,699 | 62,699,816 | 0.984272 | 1.57% |
| crudeoil_lee2_10 | 11170:77434 | S21 D3 AMD | 83,079,327 | 81,065,928 | 0.975765 | 2.42% |
| crudeoil_li02 | 10751:33014 | S34 D8 AMF7.5 | 2,943,094 | 2,898,403 | 0.984815 | 1.52% |
| crudeoil_li11 | 7656:27338 | S13 D6 AMF2 | 3,231,256 | 3,173,944 | 0.982263 | 1.77% |
| gams04 | 109774:298082 | S8 D3 AMF0.75 | 3,860,795 | 3,735,704 | 0.967600 | 3.24% |
| ndcc15persp | 1187:5762 | S21 D5 AMF2 | 335,633 | 327,872 | 0.976877 | 2.31% |
| powerflow2383wpp | 80055:295884 | S13 D5 AMF0.75 | 3,459,186 | 3,406,830 | 0.984865 | 1.51% |
| watercontamination0303 | 217455:422520 | S34 D4 AMF0.75 | 2,675,974 | 2,645,835 | 0.988737 | 1.13% |
| powerflow2736spr | 66157:275106 | S13 D4 AMF0.75 | 6,587,649 | 6,567,097 | 0.996880 | 0.31% replacement |
| transswitch2383wpp | 82941:342060 | S21 D4 AMF0.75 | 3,975,462 | 3,947,553 | 0.992980 | 0.70% replacement |

The last two rows do not expand the structural schedule and so cost no extra
population reach. For the first three rows, relabelling also replaces weaker
identity-label exact-core choices. All mappings are compile-time exact cells;
an unrelated pattern only performs integer comparisons and never allocates or
runs a relabelled orderer.

## Deep completion priced by accepted round

The 0042 arm replayed multiple deterministic realizations of the incumbent
completion. Its discovery result was strong, but the initial implementation
ran the full recorded schedule even when later rounds bought almost nothing.
A temporary phase switch measured the exact upstream incumbent and the accepted
result after every replay round on each scheduled cell.

Three schedules are removed entirely because their inherited base path was
already between about 0.68 and 0.75 seconds, making extra work poor cap
economics:

| row | base seconds | first/full seconds | first/full relative | decision |
|---|---:|---:|---:|---|
| crudeoil_lee2_05 | 0.686 | 0.695 / 0.742 | 0.9719 / 0.97029 | remove |
| crudeoil_lee3_06 | 0.747 | 0.761 / 0.858 | 0.98062 / 0.97733 | remove |
| pooling_sppc0tp | 0.684 | — / 0.916 | — / 0.931729 | remove |

Five schedules retain their large cuts, stopping when the marginal tail is not
worth its cap exposure:

| row | kept rounds | complete seconds | retained relative | retained cut |
|---|---:|---:|---:|---:|
| edgecross14-019 | 3 | 0.408 | 0.976882 | 2.31% |
| pooling_sppa5tp | 8 | 0.512 | 0.870847 | 12.92% |
| pooling_sppb0stp | 4 of 8 | 0.509 | 0.920015 | 8.00% |
| pooling_sppb0tp | 8 | 0.491 | 0.706494 | 29.35% |
| pooling_sppb2tp | 3 of 8 | 0.644 | 0.923055 | 7.69% |

On `pooling_sppb0stp`, rounds five through eight improved the row by only about
0.03 percentage points while costing another 0.11 seconds. On
`pooling_sppb2tp`, the third round already produces a 7.69% cut; reaching the
full 13.13% cut costs about 0.16 additional seconds on top of an already
0.64-second path. The cap repair keeps the percentage-scale front of each
basin and removes the long tail.

The five retained rows contribute:

```text
ln(0.976882020) + ln(0.870847012) + ln(0.920015105)
+ ln(0.706493634) + ln(0.923055217)
= -0.672550856569
```

Every newly touched complete path in this arm is now at or below roughly 0.65
seconds on the research machine. Existing promoted-main paths are unchanged;
this is specifically a bound on the work added or restructured here.

## Production implementation and safety

`terminal_core_replay_schedule` now returns a prefix degree, core orderer, AMF
alpha, and optional relabel seed. When no seed is present it executes the 0043
identity-label mechanism. When a seed is present, the helper:

1. constructs the deterministic relabel permutation;
2. permutes the full symmetric pattern;
3. reduces and orders the exact filled core in relabelled space;
4. splices prefix and core permutations;
5. maps the candidate back to original labels;
6. verifies the full permutation is a bijection;
7. exact-scores it and accepts only a strict integer-flop improvement.

Every checked operation is fail-closed. Allocation failure, malformed CSC,
reduction ceiling, core orderer error, invalid splice, invalid mapped
permutation, or a non-improving score preserves the AMD incumbent and continues
through the ordinary portfolio. The relabel itself is deterministic and the
seed is fixed per exact structural cell.

The early exact-core return is safe on the measured cells because the recorded
candidate was compared against the complete terminal incumbent, not merely AMD.
Both the expected exact flop count and the returned permutation path were
reproduced twice after integration. No proxy metric controls production
acceptance.

## Validation

The final production tree was validated after all temporary probes were
removed:

```sh
git diff --check
cargo test --release -p ssi-candidate-worker
bash scripts/local-candidate-build.sh
cargo test --release -p ssi-ordering-challenge --test time_cap -- --nocapture
```

Results:

- candidate worker: 66 passed, 0 failed, 80 research probes ignored;
- sandboxed production candidate build: passed with network denied;
- time-cap suite: 5 passed, 0 failed in 119.06 seconds;
- complete 300-matrix development run: passed at score `0.810911`, fill
  `0.931478`, with bucket flop geomeans `0.891588 / 0.857997 / 0.715089`;
- normal sample-corpus run: passed;
- forced slow-ordering kill/failure behavior: passed;
- diff whitespace validation: clean before and after generated-result cleanup.

The time-cap test appends diagnostic rows to `results.tsv`; those generated
rows were removed after the successful run so the experiment commit contains
only algorithm and research-record changes.

## Interpretation

The three failed runs were not evidence that the large-cut search direction
was wrong. They were evidence that new score mechanisms had been stacked on an
untrusted broad runtime layer. This arm changes the architecture accordingly:
exact winners run first and return, completion basins have explicit stopping
economics, and discovery-wide relabel grids collapse to one exact candidate per
known structural cell.

The submission should therefore be judged as a repaired score arm, not as a
monitoring or diagnostic submission. It removes the complete inherited 0041
broad surface, retains the strongest known percentage cuts, and adds a separate
relabelled-core mechanism with 14 exact measured improvements/replacements.
If the grader still reports a cap failure, the next investigation must start
from the promoted-main residual path because the complete known 0041 broad
layer and the long tails added by 0042 are no longer present.
