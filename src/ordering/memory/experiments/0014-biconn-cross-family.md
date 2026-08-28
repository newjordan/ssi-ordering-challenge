# 0014 — biconnected block-local cross-family portfolio

- **Date:** 2026-08-28
- **Score:** controlled `n <= 10,000` slice 0.888004 → 0.887437 (255 matrices; lower is better)
- **Status:** slice win; full-corpus official validation required

## Hypothesis

The promoted frontier already uses biconnected decomposition and a heavy variant that
spends up to seven extra slots per eligible block on relabelled default AMD. Relabelling
changes deterministic tie resolution, but every restart remains in the same degree-based
family. The global portfolio has repeatedly shown that family diversity can win even when
the added heuristic is not stronger on average. Therefore the same fixed block-local
budget should buy more distinct elimination basins if its early slots include AMF and
no-dense AMD, while the exact local flop scorer preserves the best block order.

## What changed

`src/ordering/biconn.rs` keeps the Tarjan decomposition, child-before-parent block-cut
emission, exact local `flops_of` selection, largest-block-first allocation, 24-pass global
budget, and seven-extra-candidate per-block cap. Only the candidate schedule changes:

1. plain default AMD (the existing floor),
2. plain AMF,
3. plain no-dense AMD,
4. relabelled default AMD seed 1,
5. relabelled AMF seed 1,
6. relabelled default AMD seed 2,
7. relabelled AMF seed 2,
8. relabelled no-dense AMD seed 2.

The first relabelled default-AMD slot is retained so the old schedule's highest-value early
restart is not removed before the cross-family choices. `src/ordering/mod.rs` initially
kept the inherited whole-matrix `nnz <= 60,000` heavy gate. A focused run exposed a new
2-second cap at `arki0016` (`n=7,993`, `nnz=37,208`), so the final gate is narrowed to
`nnz <= 30,000`. The two previously documented biconn-heavy wins are below this boundary.

## Result

The Windows checkout has CRLF shell scripts, so validation used an LF-native WSL export
without changing trusted scripts. `prepare-build.sh`, the purity/vendor scan, trusted
release build, sandboxed candidate build, and the release test suites passed. The latter
covered 51 unit tests, three exact tests, two scorer tests, four security tests, and five
time-cap tests (one expected ignored test).

The full 300-matrix development run could not produce a score on this host because both
the candidate and the clean promoted source hit the 2-second watchdog in inherited paths.
In particular, the candidate's full run capped at `faclay75`, whose roughly 700k-nnz
dominant block is outside this experiment's 30k whole-matrix gate and therefore cannot
execute the new schedule. This is a local environment limitation, not evidence that the
new path is safe on the official runner.

For a causal comparison, clean `HEAD` and the candidate were sandbox-rebuilt in the same
LF-native export and run with `SSI_MAX_MATRIX_N=10000` over the identical 255 matrices:

| metric | clean `3b1d869` | candidate | delta |
|---|---:|---:|---:|
| weighted flop score | 0.888004 | 0.887437 | -0.000567 |
| fill tiebreak | 0.963530 | 0.963385 | -0.000145 |
| `lt_1k` bucket | about 0.8995 | about 0.8986 | about -0.0009 |
| `1k_10k` bucket | about 0.8765 | about 0.8763 | about -0.0002 |

Both slice runs completed with valid deterministic permutations. A focused
`crudeoil_lee1_07`, `syn30m03m`, `arki0016` smoke also passed after the 30k re-gate.
The harness prints only `(capped)` rather than precise per-matrix times, so no invented
timing margin is recorded.

## Why it won / remaining risk

The improvement supports the diversity hypothesis: replacing later same-family relabel
restarts with AMF/no-dense candidates finds locally lower-flop orders, and the exact scorer
prevents a worse local candidate from being chosen. It also spends no more ordering slots
than the inherited heavy variant. The score risk is therefore bounded by the existing
best-of composition, but runtime risk is separate.

The first 60k gate was unsafe on this host. Narrowing to 30k removes the observed new cap
wall while preserving the measured slice gain. The unresolved risk is corpus/runtime
generalization: a full local score is unavailable because the promoted source itself is
cap-flaky here. Official validation must therefore be treated as the first authoritative
whole-corpus timing result; any timeout or non-improvement rejects this mechanism.

## Submission decision and invariants

This is a narrow replacement inside an already-live candidate, not another unbounded pass
over the matrix. The caller still constructs both plain biconn and biconn-heavy candidates
only in the existing sparse portfolio region. For every heavy block, slot zero remains the
same plain AMD ordering, and the returned block ordering is the minimum under the same exact
local symbolic-flop scorer. At the whole-matrix level the resulting biconn ordering is still
offered through the established `consider` function, which validates the permutation and
compares exact full-graph flops against all incumbents. Thus a lower-quality AMF or no-dense
candidate cannot replace the inherited ordering merely because it was generated.

The decomposition and composition invariants are unchanged: isolated vertices are emitted
deterministically; owned block vertices exclude the parent articulation; blocks are emitted
child before parent; each local permutation is composed back through the sorted owned-vertex
map; and missing vertices are deterministically completed. No randomness, clock, file I/O,
environment-specific matrix name, corpus lookup, or score cache is introduced. Relabelling
uses the repository's deterministic seed mapping and is composed back before scoring.

The risk boundary is therefore cost, not objective monotonicity. The final 30k whole-matrix
gate is fail-closed for the observed `arki0016` wall, while the inherited per-block
`block_nnz_cap` remains 150k and the global extra-candidate budget remains 24. Peak candidate
count is not increased relative to the promoted heavy-block schedule. The implementation
does allocate AMF/no-dense work in earlier slots, so different constant factors are possible;
the official 2-second and 4-GiB checks remain decisive.

The official submission should be promoted only if it completes the full hidden corpus and
beats `3b1d869`'s `0.834007`. A flat score means the controlled slice does not generalize to
the hidden weighting and should be rejected without widening the gate. A failure/timeout
means the cross-family constant factor is unsafe even under 30k and the entire experiment
should be reverted. A positive result should be preserved as `BICONN-XFAM-001` and only then
considered for a later restack with an independently validated mechanism.

## Follow-ups

- If officially positive, keep the cross-family schedule and 30k gate as one independent
  mechanism; widen no gate without a fresh isolated timing audit.
- If officially flat, mine the official per-matrix log before changing seeds or families.
- If officially capped, remove the mechanism rather than raising or reshaping the gate.
- Do not combine this experiment with completion-lattice P1: P1 improved the same slice by
  only 0.000004 while adding 20.5% aggregate runtime.

## Links

- Techniques: [best-of portfolios](../techniques/best-of-portfolios.md)
- Prior diversity evidence: [0010 parallel research](0010-parallel-research-session.md)
- Current promoted frontier audit: [0013 gate covariates](0013-gate-covariate-audit.md)
