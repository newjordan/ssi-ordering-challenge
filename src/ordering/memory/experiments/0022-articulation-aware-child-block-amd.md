# 0022 — articulation-aware child-block AMD

- **Date:** 2026-08-29
- **Base:** `5f0dfb0`
- **Candidate:** `2a34f40`
- **Submission:** `242a8386-dbaf-4bab-b675-cd54f679c41a`
- **Funnel state:** **TERMINAL — failed before CI, no score; exact retry declined**
- **Evidence state:** correctness/worker verified; local performance inconclusive

## Mechanism

The plain biconnected ordering already decomposes the graph into a block-cut
forest, roots each tree at its largest block, and eliminates child blocks before
their parent block. Previously, a non-root block built its local AMD problem only
from the vertices owned by that block. It removed the parent articulation before
constructing the local graph, so every edge from an owned vertex to that boundary
vertex disappeared from the local degree state.

This candidate retains the parent articulation in the local AMD problem for a
non-root block with at least two owned vertices. After AMD, it projects the
articulation out of the emitted local order. The articulation remains deferred to
the parent block and is emitted exactly once, preserving the block-cut ordering
invariant and the global bijection.

Root blocks are unchanged. Child blocks with zero or one owned vertex keep the
existing no-AMD fast paths. The heavy biconnected variant is unchanged.

## Frontier response

The live frontier advanced during admission from `d020def` to `5f0dfb0`. The new
frontier adds output-identical four-thread portfolio execution, a final exact
elimination-game LNS, heavy-tier AMF multistart, and cap-safe relabel windows.
Its promoted diff does not change `biconn.rs`, and the plain biconnected dispatch
remains present. After a clean rebase, the focused default-`order()` reach test
and all other biconnected tests still pass on `5f0dfb0`.

This candidate is therefore a COMPLEMENT rather than a redraw or follow-on to
the closed TELOS checkpoint: it changes the boundary condition inside an
existing decomposition task and does not add another portfolio task or global
search pass.

## Causal and default-path evidence

A fixed child block shaped as a four-cycle around articulation 0 gives a direct
witness. AMD on owned vertices alone projects to `[2, 1, 3]`; AMD with the
articulation present, followed by projection, gives `[3, 1, 2]`. A test-only
thread-local counter then invokes the public `order()` entry point on a graph
containing that child block and confirms that the new branch is reached exactly
once. This is a production-path reach proof, not a probe-only implementation.

## Correctness and determinism

Ten focused release tests passed. They cover empty and singleton patterns,
isolated vertices, paths and bridge fast paths, a single biconnected root block,
two cliques sharing an articulation, repeated deterministic execution, the
causal four-cycle witness, default `order()` reach, and a mixed graph containing
two child cycles, a bridge tail, and isolated vertices.

The mixed case verifies a deterministic bijection, one occurrence of the shared
articulation, and placement of all child-owned vertices before the articulation.
Both the normal AMD result and all error fallbacks filter the temporarily added
articulation, so fallback behavior preserves the same ownership invariant.

The sandboxed release candidate-worker build passed. No dependency or dispatch
surface changed.

## Complexity and cap review

The algorithm still partitions graph edges across biconnected blocks and scans
each block's own edge list. An eligible child block adds one local vertex, the
parent articulation. The local edges thereby restored are only the
parent-incident edges already belonging to that block. Total graph construction
therefore remains `O(n + nnz)` outside the pre-existing local AMD calls, and the
candidate adds neither a global pass nor a new restart loop. Star and path-like
bridge forests continue to bypass local AMD through the one-owned-vertex fast
path.

This is not a claim that wall time is byte-identical: an eligible local AMD sees
one additional vertex and its incident block edges. It is a bounded change to
work already paid by the plain biconnected candidate, with substantially lower
cap risk than another portfolio member or another TELOS descent.

## Local directional evidence

The candidate's isolated plain-biconnected census completed over the public 300
patterns. The worst isolated plain-biconnected time observed was 0.1466 seconds;
43 patterns beat the AMD anchor. This census establishes that the modified
family remains bounded on the public corpus, but it is not a paired frontier
score claim.

A paired five-pattern small representative worker cohort completed on the
preceding `d020def` frontier under the two-invocation watchdog. Both aggregates
were `0.6140` FLOPS ratio and `0.8719` fill ratio. Classify this as
**HISTORICAL-SNAPSHOT plus LOCAL-DIRECTIONAL: flat/inconclusive**, not an
improvement on the current frontier. The focused causal fixture and the
post-rebase default-path test, rather than this small aggregate, prove semantic
difference and current reach.

The candidate is admitted despite the flat representative cohort under the
campaign's local-vs-official policy: it is a novel default-path mechanism, has a
clear structural generalization hypothesis for hidden block-cut graphs, adds no
new global pass, and has no correctness or broad-regression signal. Official
terminal evaluation is the performance authority.

## Limitations

The portfolio exact-compares the plain biconnected candidate where it is
considered, but later incumbent-dependent heuristics can enter a different
basin. Therefore this experiment makes no global monotonicity claim. The public
representative cohort was intentionally small and flat; hidden benefit remains
a hypothesis until an official terminal result.

## Terminal outcome

Yukon reported the UUID as terminal `failed` with no score and no recorded
commit. Public infrastructure checks found no GitHub workflow run for the exact
submission branch, and the branch itself was absent. The candidate therefore
did not reach build, security, benchmark, or hidden-matrix execution.

Classify this as **OFFICIAL-TERMINAL failed/no score at the pre-CI platform or
archive stage**, not evidence of numerical regression, timeout, or correctness
failure. The exact platform cause was not exposed.

An exact archive retry would consume another campaign UUID. Because the local
representative performance evidence was flat and the live frontier was already
moving, the expected value does not justify that retry. Close this candidate
slot without closing the articulation-boundary hypothesis as a performance
negative; do not redraw or resubmit the unchanged archive.
