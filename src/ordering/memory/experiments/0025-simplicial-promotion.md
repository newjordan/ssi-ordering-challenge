# 0025 — non-adjacent simplicial promotion after adjacent-pair descent

- **Date:** 2026-08-29
- **Base:** `33034d6`
- **Candidate:** `46d267a`
- **Funnel state:** **READY_FOR_SLOT**
- **Evidence state:** verified-correct and worker-verified; performance is a hypothesis

## Mechanism

Replay the exact elimination game for the promoted final incumbent. At position
`i`, scan future positions `i+2..i+16` and promote a vertex `v` only when it is
adjacent to the planned pivot `x`, has strictly lower current degree, and is
simplicial in the current fill graph (`deficiency == 0`). Preserve every other
vertex's relative order.

A simplicial vertex remains simplicial while intervening vertices are removed.
It can therefore bubble left across them: a non-adjacent exchange commutes, and
an adjacent exchange cannot increase the score. The final exchange with `x` is
strict because `degree(v) < degree(x)`. Multiple promotions compose
sequentially. The shared trusted scorer still independently requires a strict
improvement before adoption.

## Causal boundary and fixed witness

Distance one is forbidden, so this is not a redraw of the promoted adjacent-pair
descent. On `K4` minus edge `(2,3)`, seed `[0,1,2,3]`, every consecutive exchange
is tied and adjacent-pair strict descent is blocked. Vertex `2` is simplicial at
distance two, adjacent to `0`, and lower degree; promotion produces
`[2,0,1,3]`, with trusted FLOPS `30 -> 23`. The graph is connected and
biconnected, excluding component and articulation explanations.

The direction overlaps MINL/watcher only at the broad completion-lattice level.
It does not change those algorithms or their schedules: it runs after final
TELOS and the promoted adjacent-pair phase, recovering a bounded bundle of
incident fill that remains in the fully downstream final incumbent.

## Gate and fail-closed bounds

```text
1,000 <= n <= 4,000
0 < off-diagonal nnz <= 60,000
nnz <= 24*n
lookahead = 16
maximum promotions = 256
cumulative deterministic operation budget = 32,000,000
```

The implementation reserves the next atomic deficiency/elimination work before
executing it. Insufficient budget, malformed CSC, invalid seed, or no promotion
returns `None`; no partial result escapes. The bitset graph is about 4 MiB at
the vertex ceiling, with remaining state linear in `n`. A candidate pays one
trusted exact score and is adopted only on strict improvement.

## Verification

- focused simplicial-promotion release tests: 4/4 pass;
- promoted adjacent-pair regression tests: 4/4 pass;
- fixed biconnected plateau witness: trusted `30 -> 23`;
- 96 deterministic small graph/permutation property cases: every returned
  candidate independently scores strictly below its seed;
- singleton, duplicate permutation, malformed CSC, and budget exhaustion fail
  closed without panic;
- public `order()` default path reaches the new phase exactly once and returns a
  deterministic bijection;
- network-denied sandbox release worker build passes on exact `33034d6`.

No public aggregate is claimed. Hidden activation frequency and benefit remain
a **HYPOTHESIS**. Completed calls preserve the prior final quality floor through
last-phase trusted strict adoption; Yukon terminal is the performance and
hidden-cap authority.
