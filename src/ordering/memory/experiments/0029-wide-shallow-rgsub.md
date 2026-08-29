# Experiment 0029: wide-shallow ranked RGSUB pipeline

## Status

Aggressive direct-remote exploratory candidate restacked on promoted frontier
`0d4da3e`. It was prepared while submission `8f94e5d2` was validating; that
probe later scored 0.820339 versus its 0.820344 parent but was rejected after
the live frontier advanced to 0.820246. No local benchmark is used. Effort: high.

## Summary

This candidate changes the shape of the large-matrix ranked subtree search,
not merely its random seed. The promoted schedule spends its per-round search
work as 32 high-ranked blocks with two five-million-operation trajectories and
64 additional blocks with one five-million-operation D1 trajectory. That is
128 trajectories and 640 million nominal operations per outer round.

The new schedule spends nearly the same total operation envelope on much more
breadth:

- ranked blocks 0 through 23: four trajectories each;
- ranked blocks 24 through 63: two trajectories each;
- ranked blocks 64 through 95: one trajectory each;
- three million operations per trajectory.

This is 208 trajectories and 624 million nominal operations per outer round.
Total nominal work decreases by 2.5%, while the number of independent probes
increases by 62.5%. Maximum thread count, ranked block count, outer round count,
matrix gates, subtree envelopes, global rescoring, and strict acceptance remain
unchanged.

## Hypothesis

The ranked RGSUB search landscape contains many small independent improvement
pockets. Earlier repository experiments found that block breadth bought more
score than depth and that randomized operator diversity beat repeated deep
search in several regimes. A five-million-operation search on a subtree of at
most 700 eliminated vertices plus bounded boundary may spend its tail budget
deepening one basin after the easy local improvements have already been found.
Redistributing that work across independent seeds and prefix policies should
increase the chance that at least one trajectory reaches a better local basin.

The top 24 blocks carry the highest incumbent flop contribution normalized by
subtree size. They receive four distinct parameterized streams: mixed-prefix,
uniform-prefix, log-tail, and reduced-policy mixed-prefix search. The next 40
blocks receive streams 2 and 3, focusing on the less-correlated log-tail and
reduced-policy variants. The final 32 blocks receive stream 3 only. This maps
the highest diversity to the highest-value blocks while preserving coverage of
all 96 ranked blocks.

## Implementation

`src/ordering/mod.rs` changes the ranked RGSUB configuration from two streams at
five million operations to four streams at three million operations. The same
configuration struct and existing search engine are reused.

`src/ordering/rgreedy.rs` recognizes the large ranked four-stream schedule. It
still takes 96 ranked blocks. For each block it computes `first_stream` from the
rank tier: zero for the first 24, two for ranks 24 through 63, and three for the
remaining ranks. The ordinary loop from `first_stream` to `cfg.streams` then
produces exactly the 4/2/1 allocation without another scheduler or allocation.

Round two keeps deterministic diversification. Stream one retains the accepted
D1 xor. Streams two and three receive a block-rank-dependent multiplicative xor
in round two so an unchanged local block does not repeat the same deterministic
trajectory. Stream zero is divided into four rank lanes: promoted parameters,
new seed, new seed with log-tail prefix mode, and new seed with the reduced
pivot-policy mask already used elsewhere in the accepted portfolio.

All operations are integer-only and outside the elimination hot loops. No new
heap allocation, locking, channel, work queue, scorer pass, graph construction,
or thread is added. The existing block-parallel worker queue remains the only
parallel execution mechanism.

## Speculative pipeline

The large-matrix phase operates as a deterministic speculative pipeline:

1. Exact symbolic analysis creates the incumbent elimination tree and counts.
2. Postorder makes each eligible subtree a contiguous position interval.
3. Blocks are filtered for fixed local and boundary size limits.
4. Blocks are ranked by exact incumbent contribution normalized by size.
5. Up to four block jobs execute in parallel.
6. Each job constructs one local induced bitset graph.
7. The rank tier selects one, two, or four shallow independent probes.
8. The block keeps its best exact local objective candidate.
9. Disjoint block outputs merge into the complete permutation.
10. The trusted global symbolic scorer verifies the candidate.
11. Only strict improvement becomes the next incumbent.
12. The second outer round repeats from the improved ordering with diversified
    deterministic trajectories.

The wider fan-out is implemented inside the existing block job. Parallelism is
still over blocks, avoiding nested thread creation and oversubscription. Search
streams within a block execute sequentially, while four independent block jobs
occupy the fixed worker threads. This keeps synchronization overhead minimal
and preserves deterministic result merging.

## Objective safety

The AMD portfolio floor is unchanged. The RGSUB phase starts from the current
best permutation. Local subtree search returns only candidates below the local
incumbent objective. After merging blocks, the whole permutation must pass a
bijection check. The caller then recomputes exact predicted LDLT flops through
the trusted scorer and retains the work only if the whole objective strictly
decreases. A poor random trajectory therefore wastes bounded compute but cannot
directly worsen the returned ordering.

Changing the search allocation can lose an improvement that the old deep
trajectory found; this is the principal quality risk. The hypothesis is that
the additional 80 trajectories and broader policy mix recover more high-value
local improvements than are lost to the two-million-operation depth reduction.
The first outer round still supplies a strong incumbent to round two, and four
top-tier streams preserve substantial exploitation.

## Time and memory envelope

Nominal per-round search operations change from:

```text
(32 * 2 + 64 * 1) * 5M = 640M
```

to:

```text
(24 * 4 + 40 * 2 + 32 * 1) * 3M = 624M
```

The maximum number of block graph constructions remains 96. Each block still
allocates one adjacency bitset and reuses it for all local trajectories. The
additional trajectories reuse the same graph, so memory peak is unchanged.
Each search has the same deterministic 1.25x hard-cap mechanism, now applied to
a smaller three-million nominal budget. More search calls create fixed setup
overhead, but fewer total nominal bitset operations and shorter individual hard
caps offset it. The remote two-second gate is authoritative.

The distribution can still change wall time because pruning and atomic greedy
runs are data-dependent. This candidate intentionally accepts that risk in
exchange for a major search-shape change. It does not widen the phase to any new
matrix, density, hub, subtree, or boundary class.

## Determinism

Rank tier selection uses only `block_rank`. Seeds are fixed arithmetic functions
of rank, stream, and outer round. Parameter variants come from existing constant
tables. There is no clock, entropy, thread identity, address, or completion-order
input. Blocks are disjoint and results are merged in fixed source order with
strict comparisons. The candidate is therefore deterministic under the two-run
gate despite concurrent block execution.

## Purity and dependency boundary

Only `src/ordering/` changes. No dependency is added or upgraded. No FFI,
native code, build script, proc macro, include, path attribute, external file,
network operation, environment access, child process, or unsafe hot-path code
is introduced. The implementation uses existing pure-Rust data structures and
search functions already present in the promoted source tree.

## Validation policy

By explicit operator request, this is a direct remote experiment. No local
performance benchmark, corpus comparison, or score-driven tuning is performed.
The Yukon grader will rebuild the editable path, run purity and license gates,
verify deterministic bijections under the memory and two-second caps, and score
the hidden corpus. Any numeric result must come from that remote validation.

The preceding narrower seed-striping submission remains useful as an ablation.
If it validates first, its outcome distinguishes whether simple diversity is
valuable before the larger breadth/depth reallocation. If this candidate fails
the cap despite lower nominal operations, call/setup overhead or path variance
is the likely cause. If it validates but loses score, five-million depth is more
valuable than the additional breadth on hidden blocks. If it promotes, the
result supports broader shallow blocked search as the next optimization axis.

## Reproduction

Start from `0d4da3e1d301d4ea3d094f0861b04d46ce21d3c2`. Set ranked RGSUB to four
streams and a three-million-operation per-stream budget. Recognize that exact
configuration in `subtree_refine`, retain 96 ranked blocks, and choose first
stream 0/2/3 for rank ranges 0..24, 24..64, and 64..96. Preserve the frontier's
conditional third-round policy and all existing gates. Apply deterministic round-two seed diversity to
streams two and three, plus rank-lane diversity to stream zero. Submit only the
editable ordering directory.

## Outcome

The first remote dispatch `0166433a` did not reach Benchmark. Rust parsed
`k as u64 << 17` as generic arguments during Setup and exited 101. The repair
parenthesizes the cast as `((k as u64) << 17)`. This is a compile-only failure,
not evidence about score, time, memory, determinism, or purity.

### Top-ten live composition added after the failed dispatch

Before redispatch, the current promoted top ten and their public notes were
re-read. Three same-work mechanisms with prior hidden-positive evidence were
absent from the latest source after graft replacement and are now composed
with the wide-shallow RGSUB experiment:

1. Existing relabelled AMD restarts cycle dense alphas
   `[10, 5, 2, 16]` in cross-corpus-positive density bands. Existing AMF
   restarts analogously cycle `[5, 2.5, 10, 1]`. Restart counts, seeds, task
   counts, and scorer calls are unchanged; slot zero preserves the old default.
2. The existing four-stream PEP search routes low-average-degree patterns to
   the already implemented varied-policy schedule using the deterministic
   integer condition `40*nnz <= 207*n`. Both arms use four streams and the same
   operation budget.
3. `rgreedy::Game` caches pristine vertex degrees once beside the pristine
   adjacency and restores them with `copy_from_slice`, removing repeated
   popcount work on every reset. The old logical operation accounting remains,
   so outputs and budget admission are not intentionally changed by the cache.

These are pure-Rust replacements and caching, not added passes. Inline assembly
and FFI were deliberately excluded because the benchmark purity gate rejects
native-code paths before scoring. The new source retains the frontier's
conditional third RGSUB round, the 208-trajectory wide-shallow pipeline, the
top-ten same-work routing mechanisms, and the parser repair. Remote validation
is the sole performance/correctness oracle per operator direction.
