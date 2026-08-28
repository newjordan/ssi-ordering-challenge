# 0023 - independent post-TELOS relabelled-AMF alpha 0.5

- **Date:** 2026-08-29
- **Base:** exact `b23bdde` production source, which is output-neutral relative
  to the promoted `5141ad0` ordering and only reuses relabel buffers
- **Development score:** `0.842222684 -> 0.842164975`
- **Exact public result:** 1 strict win, 0 losses, 1 permutation change on 300
- **Status:** locally positive and harness-validated; no official submission

## Why this experiment exists

Two recent attempts exposed a distinction that the older best-of reasoning did
not make. A candidate is not genuinely score-monotone merely because it is
accepted with strict `<` before the end of the pipeline. TELOS gradient descent
starts from the current incumbent. An upstream candidate can therefore change
the one seed passed to TELOS, causing the final result to enter a different
basin. That different basin may be better on public data and worse on hidden
data even when the upstream candidate itself never loses.

[0022](0022-post-telos-biconn-amf-zero.md) resolved the P4/P6/P8 biconnected-AMF
line exactly this way. The apparent `pooling_adhya4pq` improvement to 18,740
flops did not belong to the raw AMF ordering: the inherited final, raw AMF, and
post-final AMF were all 21,640. The 18,740 result came from TELOS descending
from a different tied seed. P9 then showed the same hidden-corpus danger for a
relabel-budget deletion: changing the upstream restart stream looked positive
on public data but officially regressed.

This experiment imposes a harder invariant. The complete original `b23bdde`
pipeline must run first, including its exact incumbent evolution and TELOS seed.
Only after every incumbent-dependent phase has ended may a new ordering be
generated and compared by exact symbolic flops. Such a final strict comparison
cannot worsen the ordering objective on any public or hidden matrix; its only
remaining risk is runtime.

## Candidate-family audit

The audit started from the existing generators and negative notes rather than
inventing another broad family.

- Plain biconnected AMF was already tested in the required post-final position
  by 0022 and produced 0 wins, 0 losses, and 0 permutation changes.
- Separator extensions are strongly negative outside their existing gates;
  METIS and Scotch were measured 1.4x to 4.5x worse than AMD on the relevant
  sparse-heavy class, with a pathological Scotch tail recorded in 0009.
- NDFM with AMD leaves added quality inside an already-covered separator family
  and was never the argmin (0014).
- Spectral/Fiedler nested dissection supplied an orthogonal representation on
  260 matrices and was also never the argmin (0016).
- The broad custom-metric grid was already greedily forward-selected against
  the real portfolio; the shipped set contains every measured positive-marginal
  point through pick 22, while wider power-shaped metrics carry bucket-cost
  cliffs near the 100k-130k wall.
- More default relabel depth is saturated: 0017 found almost no score at 3x the
  budget and 4x the cap, and 0018 found only three changes after roughly 125x
  the shipped search depth.

One narrow parameter remained explicitly untested. Experiment 0011's follow-up
list says to sweep the relabelled-AMF `dense_alpha` at 2.5 and 0.5. The shipping
relabelled-AMF loop uses `dense_alpha = 5.0`; the plain AMF sweep does not answer
the relabelled case because renumbering changes quotient-graph tie resolution.
This is therefore a new candidate stream inside a proven family, not another
copy of a candidate already present in the original path.

## Cheap screening probe

The temporary screen was deliberately narrower than a full-corpus integration:

- matrices: `n >= 1,000`, `0 < nnz <= 60,000`;
- parameters: `dense_alpha` in `{0.5, 2.5}`;
- deterministic relabel seeds: `{1, 2, 3, 4}`;
- 117 eligible public matrices;
- eight raw candidates per eligible matrix, evaluated independently after a
  complete unmodified `order()` result;
- exact `sum(c_j^2)` comparison, never ratio rounding or fill proxy;
- no candidate output was fed back into TELOS or any other phase.

Seven of the eight fixed alpha/seed combinations produced zero strict wins.
Exactly one combination won exactly one matrix:

| alpha | seed | strict wins | maximum absolute flop gain |
|---:|---:|---:|---:|
| 0.5 | 1 | 0 | 0 |
| **0.5** | **2** | **1** | **23,953** |
| 0.5 | 3 | 0 | 0 |
| 0.5 | 4 | 0 | 0 |
| 2.5 | 1 | 0 | 0 |
| 2.5 | 2 | 0 | 0 |
| 2.5 | 3 | 0 | 0 |
| 2.5 | 4 | 0 | 0 |

The winner was `crudeoil_pooling_ct3` (`n=2,644`, `nnz=11,426`): the complete
inherited final was 1,021,136 flops and the independent relabelled-AMF candidate
was 997,183. The maximum wall time for the entire eight-pass screening schedule
on any eligible matrix was 0.128233 seconds (`crudeoil_lee2_06`). That screens
both value and cost: production keeps only one of those eight passes.

The screen therefore justified a full-300 A/B, but did not justify adding a
parameter sweep. Production contains only alpha 0.5 with seed 2.

## Production structure

The implementation adds one block at the end of `order_with_options`, after
PHARMAKOS. At that point the original pipeline has already completed:

1. the anchor and whole candidate portfolio;
2. relabelled AMD, relabelled AMF alpha 5, and relabelled no-dense AMD;
3. hill climb and all post-hoc incumbent transforms;
4. SHOT-3 / filled-graph transforms;
5. TELOS gradient descent;
6. MINL and ORBIT completion-lattice phases;
7. PHARMAKOS.

Only then, for `n >= 1,000` and `0 < nnz <= 60,000`, the code constructs the
deterministic seed-2 relabeling, reuses `b23bdde`'s existing i32 pattern buffers,
runs one AMF pass with `dense_alpha = 0.5`, composes the permutation back to the
original numbering, checks bijection validity, scores exact symbolic flops, and
accepts only if `candidate_flops < best_flops`. Ties retain the inherited final
permutation.

This location is the safety property. The candidate cannot change which
ordering seeded TELOS, cannot change TELOS's move sequence, and cannot change
the starting point of MINL, ORBIT, or PHARMAKOS. A hidden matrix may or may not
benefit from the new ordering, but it cannot receive higher symbolic flops from
the final comparison.

The gate is structural and cost-oriented, not an identity dispatch trick. It
matches the already-established cheap relabel/no-dense and PHARMAKOS region,
keeps the `ringpack_30_2` / `crudeoil_lee4_*` 100k-130k cap wall completely out,
and avoids spending on the floor-limited `lt_1k` bucket. No matrix name, corpus
lookup, environment variable, timing check, or adaptive wall-clock decision is
used.

## Full 300 same-binary A/B

A temporary `cfg(test)` switch disabled only the last comparison. Both arms
were run in the same optimized test binary; all upstream code and every TELOS
input were identical. The probe compared exact integer flops and full
permutation bytes for every public pattern.

| metric | exact `b23bdde` | post-final alpha 0.5 / seed 2 | delta |
|---|---:|---:|---:|
| weighted development score | 0.842222684 | **0.842164975** | **-0.000057709** |
| strict flop wins | - | **1** | +1 |
| strict flop losses | - | **0** | 0 |
| final permutation changes | - | **1** | +1 |
| aggregate wall time | 120.895888 s | 120.745129 s | -0.150759 s (noise) |
| worst complete `order()` | 1.908079 s | 1.888123 s | same excluded matrix |

The only changed row was again `crudeoil_pooling_ct3`, exactly
1,021,136 -> 997,183. No other permutation changed, including all 183 rows
outside the production gate. The worst complete call remained
`ringpack_30_2`, which has `nnz=121,458` and therefore cannot execute the new
candidate. Aggregate candidate time appearing slightly lower is ordinary host
noise, not a claimed speedup. The useful isolated cost evidence is the screen's
0.128233-second maximum for eight passes and the changed row's same-binary
0.750818 -> 0.768978 seconds; the selected single pass adds about 0.018 seconds
there.

## Validation and editable-scope audit

The temporary screening and A/B probes, including their test-only toggle, were
removed after recording the results. The production diff against `b23bdde` is
one insertion in `src/ordering/mod.rs`, plus this memory page and index/log
links. `biconn.rs`, `probe.rs`, `scoring_ws.rs`, dependencies, restart budgets,
candidate gates, and every earlier phase remain exact `b23bdde`. No dependency
was added or changed; `src/ordering/deps.toml` remains the reviewed allowlist.

Validation uses the repository's trusted/untrusted split. `prepare-build.sh`
regenerates manifests, vendors the locked dependency tree, and runs the purity
and native-code scan. The trusted parent is built offline and locked. The
candidate worker is built in the Linux bubblewrap sandbox with network denied.
The harness security boundary and all five time-cap tests are then run against
the real public corpus. The clean LF-native WSL validation passed every stage:

- `prepare-build.sh`: dependency emission passed and the vendored tree scanned
  clean for purity/native-code violations;
- trusted parent offline/locked release build: passed;
- candidate worker: built under bubblewrap with network denied in 15.20 seconds
  and staged successfully;
- `security_boundary`: **4/4 passed**;
- `time_cap`: **5/5 passed** in 62.33 seconds, including the ordinary sandboxed
  sample-corpus run and the deliberate slow-worker SIGKILL check.

These are harness-level public checks, not a claim about an unobserved hidden
corpus. They establish that the dependency boundary, sandbox selection,
determinism/bijection path exercised by the normal run, and enforced timeout
behavior all remain intact.

## Decision and next boundary

Keep this candidate as a locally positive, genuinely score-monotone SSI
revision. It satisfies the narrower standard that P9 and 0022 established:
preserve the complete original TELOS path, add exactly one independent output,
and compare it only after every incumbent-dependent phase.

Do not widen the 60k gate, add the seven zero-win screen combinations, move the
candidate upstream, or reinterpret the win as TELOS behavior. The win belongs
to the raw alpha-0.5/seed-2 AMF ordering itself; that is demonstrated by the
post-final exact comparison. No official submission is made from this
experiment. Promotion remains contingent on separate authorization and an
official hidden-corpus result.
