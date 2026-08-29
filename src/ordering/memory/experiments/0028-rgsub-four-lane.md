# Experiment 0028: four-lane equal-budget RGSUB round-two exploration

## Status

Direct remote exploratory successor to submission `8f94e5d2`, based on promoted
frontier `70846ff`. Prepared while the preceding remote validation was running.
No local benchmark result is used. Effort: high.

## Motivation

The frontier already runs a broad whole-matrix portfolio and a two-round ranked
subtree refinement phase. The hard two-second per-matrix cap makes additional
rounds, wider gates, more blocks, more streams, and larger operation budgets
dangerous. This experiment therefore changes how the existing fixed work is
distributed across the search landscape.

RGSUB ranks disjoint elimination-tree blocks, then runs independent exact
elimination-game searches inside each block. On an unchanged block, repeating
the same seed and parameters in round two repeats a deterministic trajectory.
The accepted frontier diversified D1 in round two. Experiment 0027 diversified
stream zero on alternating ranks. This successor goes further and maps the
top-32 dual-stream blocks into four deterministic lanes. One lane preserves the
promoted search exactly. Three lanes probe different combinations of random
trajectory and search policy. Total scheduled work is unchanged.

## Production change

The only production edit remains in `src/ordering/rgreedy.rs`, inside the
ranked-block worker immediately before `search_with_nelim`. It constructs the
normal `stream_params(k)`, then for large matrices, stream zero, and outer round
two, switches on `block_rank & 3`:

- lane 0 preserves the promoted stream-zero seed and parameters;
- lane 1 xors seed `0xE703_7ED1_A0B4_28DB` and keeps default parameters;
- lane 2 xors seed `0x8EBC_6AF0_9C88_C6E3` and selects log-tail prefix mode;
- lane 3 xors seed `0x5899_65CC_7537_4CC3` and uses the reduced pivot-policy
  mask already present in the established four-stream whole-matrix portfolio.

The existing D1 round-two xor remains intact. The parameter variants are not
invented algorithms: both prefix mode 1 and policy mask `0b0011_0111` already
exist in `stream_params`, have deterministic implementations, and are part of
the repository's established search portfolio. The novelty is applying them
by ranked block lane during the otherwise redundant second round.

## Pipeline architecture

This is an explicitly blocked speculative pipeline:

1. Convert the incumbent ordering to elimination-tree postorder.
2. Calculate exact column counts and subtree sizes.
3. Form disjoint contiguous subtrees within the fixed size envelope.
4. Rank blocks by incumbent flop contribution divided by size to power 3/4.
5. Dispatch block jobs through the fixed four-thread queue.
6. Build each block-plus-boundary induced bitset graph once per block job.
7. Run the fixed number of local search streams with lane-selected policies.
8. Select the minimum local objective result for each independent block.
9. Merge disjoint block outputs without synchronization-dependent ordering.
10. Validate the complete permutation and pass it through the trusted global
    symbolic flop scorer.
11. Accept only a strict global improvement and then run the existing second
    outer round.

The patch changes only stage 7. It does not introduce another allocation, graph
construction, worker, queue, merge, scorer call, barrier, or outer iteration.
The branch and xor cost is outside the bitset elimination loops and negligible
relative to the fixed local-search budgets.

## Why four lanes

Randomized greedy elimination is multimodal. A seed changes the candidate pivot
sequence, while prefix mode changes the neighborhood explored around the local
incumbent. The default mixed-prefix mode, uniform-prefix mode, and log-tail mode
emphasize different basin scales. The reduced policy mask changes which degree
slack and fill-tiebreak combinations consume the finite operation budget.
Previous work in this repository found that operator diversity can outperform
additional depth because different matrices and blocks favor different prefix
scales and slack policies.

Rank striping provides exploration without dynamic adaptation. Dynamic timing,
work stealing based on wall clock, or shared best-first queues could violate
determinism. A pure rank-to-lane mapping produces the same candidate set on both
required invocations regardless of scheduler behavior. It also retains 25% of
the exact promoted stream-zero round-two trajectory, limiting opportunity cost,
while converting 75% of potentially repeated work into distinct exploration.

The D1 stream remains available on every ranked block. Therefore each top-32
block still has two independently parameterized attempts, and ranks 32 through
95 keep the accepted D1-only allocation. The four-lane policy applies only to
the stream-zero half of the top-ranked region.

## Performance and cap envelope

Relative to `70846ff`, the following are unchanged:

- matrix gates and the `n >= 10_000` ranked path;
- maximum nonzero ceiling;
- eligible subtree sizes and boundary size ceiling;
- rank comparator and ranked block count;
- 32 dual-stream plus 64 D1-only distribution;
- two outer rounds;
- five-million-operation budget per trajectory;
- deterministic hard operation cap;
- maximum of four worker threads;
- exact local objective and trusted global rescoring;
- strict acceptance logic.

Thus the scheduled trajectory count and nominal operation envelope are equal.
Wall time is not provably identical because different random trajectories can
terminate or prune at different points. The same hard operation ceiling bounds
each trajectory, and no new input class reaches the phase. This is the intended
trade: spend the existing worst-case budget on less-correlated search.

The expected score effect is concentrated in the high-weight large-matrix
bucket. A local subtree improvement has a clean decomposition argument: when
the block is a contiguous elimination-tree subtree and the outside order is
held fixed, reordering inside it changes only the block's objective
contribution. The caller nevertheless uses the trusted whole-pattern scorer,
so any interaction or implementation mistake cannot be admitted merely on the
local accumulator.

## Determinism and validity

Every lane decision is a pure integer function of `n`, `k`, `cfg.round`, and
`block_rank`. The constants are fixed. No entropy, time, address, thread ID, or
completion order participates. The induced graph and local seed permutation are
unchanged. Search results are merged in deterministic source order with strict
less-than comparisons. Disjoint block ranges prevent write conflicts. The
caller checks the complete result is a bijection and retains it only after an
exact global score improvement.

No dependencies, FFI, native code, files, processes, environment variables,
network operations, build scripts, includes, or unsafe runtime operations are
added. The candidate remains within `src/ordering/` and uses only code already
accepted into the trusted dependency policy.

## Validation policy

Per operator direction, this candidate is intentionally not locally benchmarked.
The hidden Yukon grader is used as the exploratory oracle for score, time cap,
determinism, and correctness. Static review confirms the patch preserves the
function contract and fixed scheduling counts, but this note does not claim a
measured speed or score delta.

The previous submission remains an independent remote probe of simple striped
seed diversification. This candidate tests a larger distribution shift with
policy diversity. If the prior submission promotes, this candidate should be
interpreted relative to that promoted tree and restacked if Yukon rejects stale
ancestry. If the prior submission validates but does not improve, this candidate
can still win because it explores additional basins. If it fails the time cap,
the likely cause is data-dependent path cost under the new policies, and the
proper response is to narrow lane application rather than increase budgets.

## Reproduction

Start from `70846ffbfbc108735fc0a0199a52ae43acb0f0aa`. Retain the accepted D1
round-two xor. Materialize `stream_params(k)` before the subtree search. For
large round-two stream-zero blocks, select the seed and parameter variant by
`block_rank & 3` exactly as listed above. Pass the resulting parameter struct to
`search_with_nelim`. Submit only the benchmark editable path.

## Outcome

Pending remote grader. A promotion supports rank-striped policy diversity as a
general equal-budget technique. A score tie or regression implies the promoted
default stream-zero trajectory is more transferable than the alternate lanes;
the follow-up should preserve more default ranks or use structural rather than
rank-only routing. A cap failure falsifies the assumption that fixed operation
ceilings sufficiently control these policy variants on hidden blocks.

