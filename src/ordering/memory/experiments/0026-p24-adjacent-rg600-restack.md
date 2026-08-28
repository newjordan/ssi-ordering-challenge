# 0026 — P24 exact adjacent-pair restoration with sparse RG600

- Date: 2026-08-29 KST
- Exact parent: `37131d54bd044612e6b833229b983463533c79af`
- Parent official score: `0.822958` (lower is better)
- Status: clean candidate prepared for orchestration; **not submitted**

## Decision

Restore the exact adjacent-pair descent that disappeared when the frontier
moved from `33034d6` to the RGSUB lineage, run its measured second convergence
cycle, and apply the independently measured sparse rgreedy re-cost:

```text
RGREEDY_MAX_NNZ: 250,000 -> 60,000
RGREEDY_BUDGET:  200M    -> 600M operations per stream
pair gate:       1,000 <= n <= 4,000 and 0 < nnz <= 60,000
pair schedule:   two cycles; each cycle = parity 0 + parity 1, 32M ops total
```

Everything newly promoted by `37131d5` remains intact. In particular, the
low-nnz relabel floor is the parent's safer `600k / cap 200`, not the transient
3M experiment, and RGSUB remains two rounds with `min_s=32`, `max_s=700`,
`max_sub=3500`, `max_blocks=96`, `budget=5M`, and two streams. Heavy metric
gates, watcher MINL, the four-worker portfolio, final TELOS, bucketed Game, and
Round-5 logic are unchanged.

## 1. Structure that was blocked

The live source has two exact-game search scales. Whole-matrix rgreedy explores
sparse small patterns, while RGSUB searches independently optimizable
elimination-tree subtrees. Both are powerful global or mesoscopic searches,
but neither guarantees that the final returned permutation is locally oriented
at every adjacent pivot boundary.

For consecutive pivots `a,b` in an exact elimination state, a useful identity
holds. If they are non-adjacent, their eliminations commute. If they are
adjacent, both orientations give the second pivot the same union neighbourhood
and leave the same residual graph after both eliminations. Only the first
column-count term differs. Therefore placing the lower-current-degree endpoint
first is an exact strict improvement whenever the degrees differ in that
direction.

The earlier promoted implementation applied this rule in two disjoint-pair
sweeps, parity zero then parity one. The later RGSUB promotion replaced the end
of `rgreedy.rs` and removed the primitive and its tests entirely. That created
a capability gap: stronger incumbents were being produced, but the exact final
local polish no longer ran.

A second scheduling obstruction also remained in the original implementation.
Two sweeps visit every boundary, but a parity-one swap can create a new
inversion on a parity-zero boundary already visited. Thus one two-sweep cycle
does not imply a fixed point. A bounded second cycle is the smallest way to
revisit only the landscape exposed by the first cycle.

## 2. How the obstruction is opened

The original adjacent primitive is restored against the current bucketed
`Game`; no old version of `Game`, `search_with`, or RGSUB is copied back. The
function uses the current `build_adj`, `new`, `reset`, and `eliminate` methods.
It validates the seed bijection, replays the exact fill graph, alternates parity,
and returns `None` on malformed input, no swap, or deterministic budget
exhaustion.

At the call site, adjacent descent is placed after the new two-round RGSUB and
immediately before return. Each completed candidate must pass a bijection check
and the shared trusted exact scorer. It replaces `best_perm` only on strict
lower FLOPS. A failure or non-improvement stops the cycle chain. The second
cycle is therefore attempted only after the first cycle produced a trusted
strict win, so non-activating matrices do not pay a second replay.

The sparse rgreedy change is orthogonal. It contracts reach from 250k to 60k
off-diagonal entries and increases deterministic depth from 200M to 600M per
stream inside the retained sparse gate. No seed, stream count, merge order,
density guard, scorer, or acceptance rule changes.

## 3. Measured ascent

### Sparse RG600 evidence

Two complete 300-pattern runs on the common lineage were byte-identical and
moved development score from `0.836185` to `0.835457`, a gain of `0.000728`.
The same screen found 1.2B worse, so 600M is the measured knee rather than an
assumption that more operations monotonically improve deterministic ILS.

### Second-cycle marginal evidence on `33034d6`

Starting from the already promoted first adjacent cycle, one extra two-sweep
cycle produced five wins and zero losses:

```text
score: 0.828645869 -> 0.828639385  (delta -0.000006484)
extra total time over 300: 0.177677 s
extra worst single phase: 0.012908 s
```

This justified two cycles rather than an unbounded convergence loop.

### Full paired A/B after RG600 on the 885 lineage

The candidate was then tested in-pipeline with the sparse RG600 constants,
RGSUB, TELOS, watcher, and all other phases held fixed. A test-only switch ran
the adjacent two-cycle phase off and on for every one of the 300 patterns:

```text
changes: 21
wins:    21
losses:  0
score off: 0.822658052
score on:  0.822471831
delta:    -0.000186221
total seconds off/on: 226.591711 / 227.247829
```

The wins span `sporttournament48`, four `rsyn/syn` cases, `nuclear25a`, five
chimera cases, `pooling_sppa0pq`, `crudeoil_lee1_07`, `blend718`, `netmod_kar1`,
`kan_r3_h1_n4`, and `space25`. The broadening from the old five-win marginal
screen shows that the deeper sparse search creates additional adjacent
opportunities rather than making the local phase redundant.

The paired wall-clock maximum was noisy: off reached 1.971 s and on reached
2.309 s on different matrices, while total time differed by only 0.656 s over
all 300. This host was also running an external challenge process. Treat those
maxima as contended local observations, not isolated cap measurements. The
phase remains guarded, deterministically budgeted, conditionally second-cycle,
and the earlier isolated marginal worst was 0.0129 s. Official Yukon execution
remains the cap authority.

The full A/B parent was `885d0d8`. While it ran, `37131d5` promoted a safer
600k relabel floor and a stronger two-round RGSUB. The final code was rebuilt on
that exact source. Because the stronger RGSUB changes the final incumbent, the
21-win count is directional lineage evidence, not claimed as an exact 371
aggregate. Numerical non-regression on 371 remains structural because adjacent
is final and trusted-strict.

## 4. Where future structural changes belong

Adjacent exchange belongs at the last boundary of `order_with_options`, after
every incumbent-sensitive phase. Moving it before RGSUB or TELOS would make the
later search consume a changed seed and could move into a worse basin despite
the local strict win. At the end, its only possible completed effect is
`best_flops = min(best_flops, candidate_flops)`.

The exact primitive belongs beside rgreedy because it reuses the same exact
bitset elimination game. It must not replace the current Game implementation:
the current file contains degree buckets, partial games, subtree boundaries,
and multi-round RGSUB machinery absent from the old adjacent commit. This
restack adds only the independent primitive and focused tests.

Future ascent should remain marginal. A third pair cycle requires a fresh
full-corpus screen; the small second-cycle-only gain suggests rapid saturation.
Triple exchanges require a proof or exact enumeration of all orientations,
not a degree heuristic. Equal-cost rgreedy stream diversity is a separate axis
and should be screened without changing the adjacent schedule. Every future
candidate must start from the then-current promoted source and audit the final
phase order before restacking.

## Correctness and failure boundaries

Four focused release tests cover a strict star witness, deterministic repeated
output, 96 randomized small graph/permutation cases checked against the trusted
symbolic score, malformed and budget-exhaustion fail-closed behavior, and reach
through public `order()`. On the exact `37131d5` restack they pass 4/4. The full
candidate-worker suite passes 42 tests with zero failures and 54 intentionally
ignored research probes.

Determinism follows from fixed parity, fixed seed order, exact graph state, and
an operation counter rather than a clock. Each cycle has an independent 32M
budget. Exhaustion discards that cycle; it never returns a partial permutation.
The production path rechecks bijection and exact FLOPS. Equal scores are not
accepted, so tie behavior is unchanged.

## Parent audit and handoff

Direct `git show <object>:src/ordering/mod.rs` audit established the exact
low-nnz history: `8586bf1` is `600_000 / cap 200`, `885d0d8` is
`3_000_000 / cap 800`, and `37131d5` deliberately returns to
`600_000 / cap 200`. P24 is byte-identical to `37131d5` at that expression;
`git diff 37131d5..P24` contains no `small_multi` hunk. The `885..371` diff
reverts the floor in the same commit that expands RGSUB to two rounds and two
streams. The retained source comment supplies the cap rationale: 3M added
1.226 s on `crudeoil_lee1_07`, put six matrices above 1.4 s, while 600k was
sized near 0.27 s. Thus 371 trades the transient low-nnz restart depth for the
broader exact subtree improvement and a safer tail; its better official score
(`0.822958` versus `885`'s `0.824406`) is the aggregate outcome, not proof that
either individual half alone caused the full delta.

Final-parent checks establish:

- parent is exactly `37131d54bd044612e6b833229b983463533c79af`;
- low-nnz relabel remains `600_000 / nnz`, capped at 200;
- RGSUB retains its promoted two rounds, two streams, and expanded block gates;
- heavy and watcher source is untouched;
- RG changes are exactly the two intended constants plus explanatory comment;
- adjacent is after RGSUB and before return;
- no harness, scorer, corpus, protocol, dependency, or limit file changes.

Do not submit or cancel from this worktree. The orchestrator must compare the
live source reference again before any restack or official action.
