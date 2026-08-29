# 0027 — Recompose sparse RGSUB round three onto dense-low-alpha frontier

## Summary

This submission starts exactly from promoted frontier `f48a17f` / submission
`2b91c868`, then restores one independently promoted mechanism which disappeared
because the dense-low-alpha submission was developed from an older base.  The
change is intentionally narrow: run a third deterministic RGSUB subtree-refine
round only for the two sparse classes that previously produced positive remote
evidence.  No dense arm, relabel portfolio, seed policy, block allocation,
per-block operation budget, scorer, or parallel scheduling policy is changed.

The two gates are:

1. `n >= 10_000 && nnz <= 100_000`;
2. `n < 10_000 && nnz <= 30_000 && max_deg * 32 <= n`.

Every completed round is scored by the existing trusted FLOPS workspace.  A
candidate replaces the incumbent only on strict improvement; otherwise the
loop exits immediately.  Consequently round three cannot directly return a
worse ordering than round two, while matrices outside the two gates execute the
frontier's byte-for-byte two-round path.

## Evidence and motivation

The latest promoted frontier made a large cross-corpus gain by adding low-alpha
AMF/AMD candidates for the dense-heavy region (`400k <= nnz < 1M`, mean degree
above 20).  Its public note reports four reconstructed-eval movers and zero
regressions, with the largest effect on `telecomsp_metro`.  That mechanism is
retained unchanged.  Source inspection shows, however, that this frontier was
based on an ancestry which predates the accepted conditional third-round RGSUB
change.  The missing sparse mechanism is orthogonal: it targets low-nnz
matrices, while the new dense arm begins at 400k nnz.

The third-round mechanism had already survived remote validation and promotion
in submission `01cd43b2`.  Its public report identified the sparse-large class
and extended the same fixed-budget pass to small hub-free sparse patterns.  It
was not removed after a measured regression; it vanished when a later solver
was grafted from an older baseline.  Recomposition is therefore preferable to
inventing a wider unvalidated gate.

The immediately preceding local wide-shallow experiment is deliberately not
carried forward.  Remote submission `760ddbe5` scored `0.820451`, materially
behind the then-current `0.817268` frontier.  That result is treated as negative
evidence against reallocating the ranked subtree budget from 64 two-stream
blocks to a broader four/two/one-stream schedule.  This candidate restores the
frontier's original block count, stream count, seeds, and five-million-op
per-stream budget.

## Algorithmic composition

The incumbent is first produced by the full frontier portfolio.  The existing
RGSUB stage computes the elimination tree of that incumbent, postorders it,
derives exact column counts, and selects disjoint eligible subtrees.  Each block
is converted to a local induced bitset graph including its boundary.  The local
randomized greedy elimination game explores deterministic trajectories, and
the resulting block ordering is composed back into the global permutation.

Round one searches around the portfolio incumbent.  If it wins under the exact
scorer, round two repeats the same deterministic algorithm around the improved
incumbent.  Under the new narrow gates only, a strict round-two win permits
round three.  If any round has no eligible blocks, produces no bijection, or
does not reduce exact FLOPS, the loop stops.  This makes the extra work a
conditional depth extension rather than an unconditional expansion.

The large sparse gate uses the ranked configuration already present in the
frontier: subtree sizes 160 through 1400, boundary-inclusive local size at most
3000, 64 ranked blocks, two streams, five million word operations per stream.
The smaller sparse gate uses the existing default configuration and excludes
hub-dominated matrices through the maximum-degree test.  These choices are not
retuned in this submission, which isolates the effect of recomposition.

## Fixed-budget and timing discipline

There is no wall-clock branching and no entropy source.  All seeds and search
budgets remain compile-time deterministic.  The extra round consumes work only
after two strict improvements on the targeted sparse matrix.  This is a useful
implicit difficulty signal: matrices on which the local neighborhood saturates
after one pass pay no third-pass cost.  Dense-heavy matrices responsible for
the new frontier improvement never enter this additional gate because their
nnz is far above the 100k ceiling.

The search remains parallel over disjoint blocks with the existing global
thread cap.  Each worker owns its local bitset graph and scratch vectors.  The
merge applies results only after workers join, in deterministic block order.
No shared incumbent, clock, work stealing, or completion-order tie breaking is
introduced.  The same bijection validation and exact scorer guard remain the
only path by which a candidate can become the result.

## Purity and scope audit

Only `src/ordering/` is changed.  The implementation is pure safe Rust and uses
no assembly, inline assembly, FFI, native code, `build.rs`, `include!`, process
execution, network access, or filesystem access.  It adds no dependency and
changes no dependency manifest.  Memory behavior is inherited from the
frontier RGSUB implementation; the additional round reuses the caller's
workspace and releases round-local structures before the next iteration.

The candidate preserves the two-second-per-matrix, four-GiB, deterministic
bijection contract structurally.  In accordance with the requested workflow,
no local performance benchmark or full-corpus evaluation was run.  Correctness
and performance are delegated directly to remote validation.  Static source
inspection confirms that every result is passed through `is_bijection` and the
trusted `ws.flops` scorer before acceptance.

## Mechanism inventory against the current top frontier

Retained unchanged from `2b91c868`:

- D_WIDE AMF alpha points 1.0, 1.5, and 2.0 inside density 20–50;
- dense low-alpha AMF 0.75 and 1.25 above density 20;
- aggressive AMD alpha 0.75 in the same dense-heavy window;
- AMF alpha 2.0 excluded above density 50 to avoid its known cost cliff;
- relabel AMD/AMF alpha cycling in the measured sparse density bands;
- PEP stream-policy routing by mean degree;
- pristine degree cache and all existing exact best-of scoring;
- ranked RGSUB block selection and the accepted D1 seed split.

Restored by this submission:

- conditional RGSUB round three for sparse-large matrices;
- conditional RGSUB round three for small, sparse, hub-free matrices.

Explicitly excluded despite prior experimentation:

- four-lane rank-dependent stream-zero policies;
- 208-trajectory wide-shallow block allocation;
- expansion beyond the promoted nnz and hub gates;
- assembly or any native acceleration path;
- additional relabelled AMF multistarts, whose public evidence is negative;
- METIS/Scotch-style giant sparse tie exploration, also reported negative.

## Expected hidden behavior

The candidate should match the dense-low-alpha frontier on all matrices outside
the two sparse gates.  Inside a gate it should also match whenever round one or
round two fails to improve, because the loop terminates before extra work.  On
the subset that exhibits a chain of two strict local improvements, round three
may discover another lower-FLOPS ordering.  The expected aggregate effect is
small but non-negative in objective choice, with runtime risk bounded by the
narrow nnz ceilings and by the conditional continuation rule.

The principal uncertainty is timeout interaction: strict objective monotonicity
does not protect a matrix that exceeds the remote time cap.  The remote evidence
for the already-promoted gate is the reason no further widening is attempted.
If validation fails on timing, the next response should tighten the small gate
or require a measured minimum improvement before continuing, rather than
increasing budgets.  If it validates but does not beat the frontier, the result
still resolves whether the previously accepted sparse effect composes with the
new dense portfolio.

## Decision rule after remote validation

Promote or continue from this candidate only if the hidden aggregate beats the
current frontier and the run clears all purity, determinism, memory, and timing
checks.  A clean but neutral/rejected result means the sparse benefit was
already subsumed by other frontier candidates or is too small at the new base;
do not duplicate the same third round again.  A setup failure should be treated
as an implementation issue and repaired without changing the algorithm.  A
benchmark timeout should trigger stricter gating, never a larger fixed budget.

## Follow-up composition after official result

Submission `5af1bf85` completed the remote grader at `0.817178`, an exact
`-0.000090` improvement over its `0.817268` dense-low-alpha parent. It passed
setup, compilation, purity, correctness, determinism, memory, and the watchdog;
the rejected label reflects concurrent frontier arbitration. This directly
confirms that conditional round three composes positively with the dense arm.

This follow-up retains that mechanism and restores two mechanisms promoted in
`4bf806c` but absent from the ancestry of `f48a17f`. First, the existing PEP
search receives an 800M operation floor only when `nnz <= 2*n` or
`4*n <= nnz <= 4.5*n`. No gate, stream, phase, or candidate family is widened.
Outside those bands the exact 600M floor remains. This MID800 allocation already
completed an official hidden run; broad budget increases remain excluded.

Second, RGSUB stream one is diversified only on outer round index one for
`n >= 10,000`. Block count, stream count, trajectories, and per-stream work are
unchanged. Round zero uses D1; round one XORs one fixed constant into D1, so a
surviving block does not repeat the same basin byte-for-byte. Stream zero is
untouched. Other rounds use the accepted seed. `SubCfg` carries the round index
explicitly through the restored conditional caller.

The package therefore composes three independently positive remote mechanisms:
hidden-positive conditional sparse depth, hidden-safe narrow MID800 routing,
and equal-work round-two D1 diversification. It excludes wide-shallow search,
extra stream-zero trajectories, block-ceiling expansion, broad budgets, timing
gates, assembly, FFI, and native code.

## Allocation-flattened follow-up

The composed submission `cc44570d` completed at `0.817021`, improving the
original `0.817268` frontier by `0.000247`.  The next package preserves its
candidate set exactly and attacks worker overhead only.  Each RGSUB worker now
retains its local adjacency bitset, identity seed, touched-vertex list, local
vertex list, and result capacity across blocks.  The former per-block adjacency
and seed allocations become resize/fill and clear/refill operations over owned
capacity.  Hot iterator chains for vertex discovery, induced-edge insertion,
seed construction, and block FLOPS accumulation are flattened into indexed
while loops with cached CSC endpoints and bitset offsets.  No `unsafe` block is
used.  Search budgets and their accounting are unchanged, so this is intended
to reduce allocator traffic and loop overhead without buying extra trajectories
or changing deterministic results.
