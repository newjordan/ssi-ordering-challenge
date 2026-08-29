# Experiment 0027: striped stream-zero diversification in ranked RGSUB round two

## Status

Direct remote exploratory submission from promoted frontier `70846ff`. No local
corpus score was used to select this candidate. The requested operating mode was
to avoid local benchmarking and let the hidden grader decide quality and cap
safety directly.

Effort: high.

## Objective

The current frontier already contains a very large best-of portfolio. Adding a
new whole-matrix ordering family would stack more sequential work onto matrices
that can already approach the two-second hidden time cap. The safer remaining
axis is trajectory diversity at fixed work. Ranked RGSUB is especially suitable:
it decomposes the incumbent elimination tree into disjoint subtrees, searches
those blocks independently, and accepts the resulting whole permutation only
when the trusted exact scorer reports lower predicted LDLT flops.

The promoted `70846ff` frontier already diversified the D1 trajectory in round
two. Stream zero, however, still reused its round-one deterministic seed on an
unchanged block. If another block improved between rounds, the global incumbent
changed while many unrelated disjoint blocks remained locally identical. Those
blocks therefore repeated the same stream-zero search byte-for-byte in round
two. This candidate converts half of that repeated work into fresh exploration.

## Change

Only `src/ordering/rgreedy.rs` changes production behavior. For ranked RGSUB on
large matrices (`n >= 10_000`), in outer round two (`cfg.round == 1`), stream
zero receives a fixed xor seed on odd ranked block indices:

```text
0xE703_7ED1_A0B4_28DB
```

Even ranked blocks retain the exact promoted stream-zero seed. Stream one keeps
the promoted round-two D1 xor from `70846ff`. Round one remains byte-identical.
Small and medium matrices remain byte-identical. No gate, subtree-size range,
rank formula, block limit, thread count, stream count, round count, per-stream
operation budget, hard cap, scoring rule, or acceptance condition changes.

The striped policy intentionally preserves half of the known trajectory rather
than replacing all 32 top-block stream-zero searches. That gives an equal-work
mixture of exploitation and exploration: 16 top ranked blocks keep the promoted
basin, while 16 sample a different deterministic basin. The next 64 ranked
blocks already run D1-only and are unaffected by this stream-zero change.

## Pipeline and block structure

The effective computation is a bounded speculative pipeline:

1. The incumbent permutation is transformed to elimination-tree postorder.
2. Contiguous disjoint subtrees are extracted and ranked by exact incumbent
   flop contribution normalized by subtree size to the three-quarter power.
3. The highest-value block jobs enter a maximum-four-thread queue.
4. Each job builds its induced local graph, including the external boundary.
5. Independent deterministic randomized-greedy trajectories probe that block.
6. The lowest local objective candidate is written back only for that disjoint
   position range.
7. The complete permutation is checked as a bijection and rescored by the
   trusted global symbolic factorization path.
8. A second outer round repeats the pipeline from the improved incumbent.

This submission changes stage 5 only. It does not create another pipeline
stage. The block scheduler still keeps up to four cores busy, block outputs are
independent, and the deterministic merge remains insensitive to thread
completion order. The seed is a pure function of public structural state:
matrix size class, stream index, outer round, and deterministic ranked block
index. There is no clock, entropy source, shared mutable RNG, or scheduling
input, so the required double invocation remains reproducible.

## Performance rationale

For the factorization-flop objective, different randomized exact-elimination
trajectories can reach different basins even with identical work budgets.
Previous frontier work established that diversity can be worth more than depth:
uniform-prefix and tail-biased perturbations win on different matrices, and the
accepted D1 round-two seed change improved the hidden score without increasing
nominal work. Repeating a deterministic trajectory on an unchanged local block
spends CPU without expanding the candidate set. A seed replacement is therefore
one of the few ways to seek additional objective quality without widening the
time envelope.

The alternating-rank predicate is deliberately cheap and branch-predictable.
It is evaluated once per stream invocation, outside the elimination-game hot
loops. The extra xor is negligible compared with local adjacency construction
and millions of bitset operations. The generated code adds no allocation, no
indirection, no synchronization, and no scoring pass. Readability is secondary
to keeping the hot path mechanically small.

The expected upside is concentrated in the high-weight `gt_10k` bucket. Ranked
RGSUB is enabled for large matrices and targets high-contribution tree blocks.
A successful new trajectory can improve a local block while provably leaving
the objective contribution outside the block unchanged. The trusted global
score remains the final guard. The expected downside is search opportunity
cost: an odd-ranked block that would have benefited from the old second-round
stream-zero seed may not rediscover that result with the new seed. Stripes cap
that opportunity cost by retaining the old seed on even ranks, and the entire
round-one incumbent remains a strict fallback.

## Time-cap analysis

This candidate is equal-count and equal-budget relative to `70846ff`:

- same two outer RGSUB rounds;
- same ranked block population;
- same 32 dual-stream plus 64 D1-only allocation;
- same two streams for the top 32 blocks;
- same five-million-operation budget per trajectory;
- same per-search hard cap;
- same maximum four worker threads;
- same adjacency construction count;
- same whole-permutation scoring count.

Random paths can have data-dependent operation distributions, so equal nominal
budget is not a mathematical proof of identical wall time. The search engine's
deterministic operation ceiling remains unchanged, however, and this patch does
not expose any new matrix class to RGSUB. That makes it materially less risky
than adding a round, block, stream, or wider gate. The remote grader is the
authoritative time-cap test for this direct-submission workflow.

## Correctness and determinism

The change affects only RNG initialization. `rng_seed` is still forced odd by
the downstream search routine. The xorshift evolution is deterministic. Ranked
block order is deterministic because the comparator and source ordering are
deterministic. Blocks are disjoint. Per-thread results are collected and merged
through fixed source order with strict improvement comparisons. Candidate block
orders are validated by length locally, and the caller validates the complete
permutation as a bijection before exact rescoring. The AMD-based portfolio floor
and the incumbent strict-improvement guard are unchanged.

No dependency declarations change. No FFI, build script, proc macro, include,
path attribute, native code, external process, file access, environment access,
or network access is introduced. The submission remains confined to the single
editable path required by the benchmark.

## Validation policy

Per explicit operator direction, no local benchmark or local performance test
was run for this experiment. The repository had previously completed the Yukon
setup step on the promoted frontier, including dependency vendoring and the
purity/license preparation. A local full-corpus run was started before the
operator changed the policy, then interrupted; its partial output was not used
to tune or accept this patch. No claim in this note treats that partial run as
candidate evidence.

The intended validation command is the remote Yukon submission itself. The
hidden grader will rebuild the candidate, enforce purity, determinism,
bijection, memory, and two-second per-matrix gates, then compare the hidden
weighted geometric-mean flop score with the live frontier. A rejection is an
informative exploration result, not a reason to widen local claims.

## Reproduction

Starting from promoted commit `70846ffbfbc108735fc0a0199a52ae43acb0f0aa`,
change the round-two seed selection in `subtree_refine` so that `k == 0` and
odd `block_rank` values xor `0xE703_7ED1_A0B4_28DB` when `n >= 10_000` and
`cfg.round == 1`. Leave the existing D1 round-two xor intact. Submit only
`src/ordering/` through Yukon.

## Outcome and next step

Outcome pending remote validation. If the score improves and promotes, retain
the striped stream-zero diversification and next test another equal-work stripe
or a block-rank-dependent policy selection. If it validates but ties or loses,
the likely interpretation is that the promoted stream-zero basin was stronger
than the replacement on hidden high-value blocks; revert the xor and explore a
different constant or a narrower stripe. If it fails the cap, treat path-cost
variance as material even under fixed operation ceilings and revert rather than
adding compensating complexity.

