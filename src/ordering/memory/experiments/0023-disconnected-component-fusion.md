# 0023 — disconnected-component anchor/incumbent fusion

- **Date:** 2026-08-29
- **Base:** `26b1103`
- **Candidate:** `40c8523`
- **Funnel state:** **READY_FOR_SLOT**
- **Evidence state:** verified-correct and worker-verified; performance is a hypothesis

## Mechanism

At the end of the complete ordering pipeline, compare the AMD anchor and final
incumbent independently inside each disconnected graph component. Use the
lower-FLOPS projection for each component, concatenate the selected projections,
and accept the fused permutation only if its trusted actual score exactly matches
the componentwise prediction and strictly improves the final incumbent.

The component map reuses the AMD elimination forest. For a structurally
symmetric graph, elimination cannot add an edge between disconnected components,
so each elimination-forest tree is exactly one original connected component.
The `sum(c_j^2)` objective is additive across those components, making
componentwise selection valid.

## Frontier response

`26b1103` expands the four-thread portfolio, heavy/mid-band candidate sets, and
RGREEDY, then places a deterministic TELOS multi-start arm after every prior
phase. This candidate is a COMPLEMENT: it runs after that new final TELOS arm
and only combines the fully downstream incumbent with the AMD anchor across
independent components. It neither changes a search schedule nor adds another
ordering search.

Because it is the last phase, no downstream heuristic consumes the fused result
as a seed. The current frontier's earlier scheduling and search mechanics are
unchanged.

## Admission and fail-closed checks

The pre-gate requires:

- `n <= 60_000`;
- off-diagonal `nnz <= 150_000`;
- at least two elimination-forest components of size at least three.

Components of size one or two have no order-dependent elimination choice, so a
single nontrivial core plus isolates or isolated edges cannot produce opposing
component winners and is rejected before an additional symbolic score.

For an admitted structure, the final incumbent is rescored once to expose
column counts by original vertex. Fusion is attempted only when the per-component
minimum predicts a strict improvement. The fused permutation is then checked as
a bijection and exact-scored with the trusted workspace. Acceptance requires:

```text
trusted_actual == predicted_component_sum
trusted_actual < existing_final_flops
```

Any malformed forest, permutation, overflow, score inconsistency, or absent
strict opportunity returns the original final incumbent without panic.

## Correctness and default reach

Three focused release tests passed:

- a two-component synthetic witness where AMD has local profile `14 + 9` and
  the final incumbent has `9 + 14`, yielding a fused `9 + 9 = 18`;
- default `order()` reach, determinism, and bijection on two disconnected
  three-vertex paths;
- fail-closed behavior for malformed parents, a duplicate AMD permutation, and
  a single nontrivial component plus isolates.

The sandboxed release candidate-worker build passed on the exact `26b1103`
integration base.

## Complexity and cap reasoning

The free preflight reads the retained AMD forest in `O(n)`. Only a qualifying
multi-component graph pays one additional exact symbolic count pass for the
final incumbent. Only a predicted strict improvement pays one additional exact
score for the fused permutation. Component accounting and splicing are `O(n)`.

No new orderer, restart, edge traversal, completion graph, recursive search, or
dependency is added. Component arrays and the optional fused permutation use
`O(n)` memory; the existing `ScoreWorkspace` owns the large reusable scoring
buffers. The hard `n/nnz` gate excludes giant and dense cap walls, while the
two-choice-bearing-component pre-gate excludes common isolate-heavy single-core
patterns before the new scoring work.

## Evidence classification and decision

The focused theorem witness, fail-closed tests, default-path activation, and
release worker build are **VERIFIED-CORRECT** for their stated properties. No
public-corpus aggregate was used: local score effect is **HYPOTHESIS**, restricted
to matrices with at least two nontrivial disconnected components whose AMD and
final-incumbent winners oppose each other.

Numerical score safety is structural for completed executions because this is
the final phase and trusted actual score must strictly improve before adoption.
The remaining risk is only added runtime on qualifying patterns. The bounded
scoring-only cost, current parallel frontier headroom, and narrow opportunity
gate make that risk admissible for an official slot. Yukon terminal evaluation
is the performance and hidden-cap authority.
