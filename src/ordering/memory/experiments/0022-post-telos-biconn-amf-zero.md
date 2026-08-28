# 0022 - post-TELOS biconn-AMF: monotone placement removes the apparent win

- **Date:** 2026-08-29
- **Baseline:** `b23bdde` (`5141ad0` outputs with relabel-buffer reuse)
- **Status:** rejected locally; zero score value; note-only, no submission

## Question

The P4/P6/P8 line repeatedly observed one public win from adding a plain AMF
candidate inside the biconnected-heavy phase. Its safest static form used:

```text
n <= 192 && Pattern::nnz() <= 1_200 && max_deg <= 64
```

On exact `5141ad0`, inserting that candidate before the later incumbent-driven
phases changed `pooling_adhya4pq` from 21,640 to 18,740 flops and moved the dev
score from 0.842222684 to 0.841958705. Two official variants nevertheless hit
the hidden two-second cap. P8 isolated the true path, kept all 214 false-gate
outputs byte-identical, and retained the same public win, but its official run
also failed the hidden cap.

P9 exposed a second problem with pre-TELOS edits. Deleting relabel work gave one
public win and zero losses under a structural gate, yet the official hidden run
scored 0.833436 against the 0.833396 frontier. Experiment 0020 had already
identified the mechanism: TELOS descends from the current incumbent, so an
upstream permutation change can move it to another deterministic basin. An
upstream best-of addition or deletion is not globally score-monotone when the
later phase is seeded from its result.

This experiment asks whether the AMF object itself has value when evaluated
only after the entire inherited pipeline has finished. That placement must
preserve the exact original incumbent and TELOS seed path, then compare the new
candidate to the final result with exact symbolic flops.

## Source structure tested

The experiment was built in an independent worktree rooted at `b23bdde`. That
commit is output-neutral relative to `5141ad0` and reuses relabel permutation
buffers, providing measured runtime headroom without altering any ordering.

The original biconnected-heavy call remained in its original pre-relabel
position. The P8 strict specialization was added as a separate function, but it
was invoked only at the very end of `order_with_options`, after all of:

1. the original candidate portfolio and biconnected-heavy candidate;
2. relabelled AMD, relabelled AMF, and no-dense AMD restarts;
3. hill-climb and the post-hoc blind-spot families;
4. SHOT-3 filled-graph/strip variants;
5. TELOS gradient descent;
6. completion-lattice MINL and ORBIT;
7. PHARMAKOS.

Only after those phases completed did the strict gate call
`biconn_heavy_order_with_amf(pattern, 24, 32, 150_000)`. Its result was checked
for bijection validity, scored with the exact shared workspace, and accepted
only on strict `candidate_flops < final_flops`. Ties retained the inherited
final permutation.

This is genuinely score-monotone rather than merely best-of inside an upstream
subphase. Every inherited phase sees the exact `b23bdde` incumbent, exact seed,
and exact candidate stream. The old final permutation remains available after
all stateful phases have ended. No hidden input can receive worse symbolic
flops from this final comparison, though runtime still has to be bounded.

For the causal A/B, a `cfg(test)` atomic switch disabled or enabled only this
last comparison inside one release test binary. It defaulted to enabled and was
compiled out of production builds. The probe ran serially over the canonical
300 parsed public patterns, compared exact integer flops and full permutation
bytes, and used the official three-bucket weighted geomean.

## Full 300-matrix causal A/B

| metric | exact `b23bdde` | post-final AMF | delta |
|---|---:|---:|---:|
| weighted flop score | 0.842222684 | 0.842222684 | **0** |
| strict flop wins / losses | - | **0 / 0** | - |
| final permutation changes | - | **0** | - |
| strict gate population | 86 | 86 | 0 |
| false-gate population | 214 | 214 | 0 |
| false-gate mismatches | - | **0** | - |
| whole-corpus paired wall | 164.809 s | 166.672 s | +1.863 s |
| gate-population paired wall | 2.146 s | 2.254 s | +0.109 s |
| worst true-gate call | 0.113 s | 0.115 s | +0.002 s |

The host was contended, so the wall totals are directional only. The relevant
cost boundary did behave as intended: all true-gate complete `order()` calls
were around 0.115 seconds or below in this run, and all 214 false-gate outputs
were byte-identical. But there is no score carrier to justify even this small
extra work.

## Target attribution: the AMF candidate was never the 18,740 ordering

A focused probe measured the three distinct objects on the sole P8 target:

```text
pooling_adhya4pq
n=170, nnz=932, max_deg=21
inherited final                  = 21,640 flops
raw biconn_heavy_order_with_amf  = 21,640 flops
final after post comparison      = 21,640 flops
```

This resolves the earlier ambiguity. The raw specialized biconn-AMF ordering
does not have 18,740 flops; it ties the inherited final at 21,640. When inserted
upstream in P6/P8, its different tied permutation became the incumbent passed
to TELOS. TELOS then descended from that alternative seed and found 18,740.
The measured score improvement belonged to the changed TELOS basin, not to the
AMF candidate evaluated independently.

That also explains why evaluating the same object after TELOS produces no
permutation change. Strict `<` correctly rejects the tied candidate, preserving
the original final permutation and proving zero value across the complete
public corpus.

## Relation to the relabel experiments

The same distinction applies to P9. A larger upstream restart set already
contains the smaller set's raw candidates, so deleting restart suffixes cannot
create a new independent ordering. Its public gain came from changing which
upstream tie/winner seeded TELOS. A post-TELOS comparison of raw relabel
candidates therefore cannot reproduce the basin change; running a complete
second pipeline from every alternative seed and best-of-ing the final outputs
would be score-monotone, but it duplicates TELOS and the surrounding pipeline,
which is incompatible with the hidden two-second cap.

The durable architecture rule is now sharper:

- Adding an ordering after every incumbent-dependent phase is truly monotone,
  but the audited AMF object has zero marginal value there.
- Adding it before TELOS can change the score, but the gain and any hidden loss
  are TELOS seed effects, not a monotone best-of property.
- Preserving several upstream incumbents and running TELOS from all of them
  would recover monotonicity, but costs multiple descents and is not cap-safe.
- Therefore neither the strict AMF line nor the relabel-deletion line supplies
  a shippable score-monotone carrier.

## Decision

Reject and revert the post-TELOS candidate. The production function,
specialized biconn implementation, and A/B probes were removed after recording
the results. The branch retains only this negative memory entry on top of
`b23bdde`; no cap/security suite is needed for a note-only tree, and no Yukon
submission was made.

Do not retry P4/P6/P8 by moving the same object later: the full public census
proves zero wins. Do not describe the old 18,740 result as an AMF sole win; it
is a TELOS alternative-seed win. A future monotone direction must produce an
ordering whose raw exact flops beat the complete final incumbent, or introduce
a cheaper way to preserve and descend from multiple upstream basins without
duplicating the pipeline.
