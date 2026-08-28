# 0021 - post-final late-seed robust AMD on exact promoted base

- **Date:** 2026-08-29
- **Exact parent:** `5141ad0bc152c1e0a2de8b74528354cf8e065b75`
- **Family:** relabelled robust AMD (`aggressive=false`, `dense_alpha=-1`)
- **Selected seed:** `2666`
- **Production gate:** `n <= 1,000`, `0 < nnz <= 6,000`
- **Expected public result:** `ndcc12`, 345,696 -> 342,379 flops
- **Status:** exact-base restack; no submission

## Why this restack exists

The first P15 implementation was developed on `b23bdde`, an output-neutral
optimization that replaces the promoted relabel-pattern allocation sequence
with reusable buffers.  Its 300-matrix equivalence proof established that b23
does not change a public permutation or flop count, and P15 itself had one
strict public win.  That is still not the cleanest official experiment after
multiple hidden two-second failures: an output-neutral local optimization can
change binary layout, allocation timing, or an unseen cost tail even when every
public output agrees.

This revision therefore removes that variable completely.  Its parent is the
exact source currently promoted by Yukon, `5141ad0`.  It copies only the P15
score mechanism and uses the promoted source's own ordinary allocation path.
There is no cherry-pick of b23, no `relabel_into`, no reusable relabel CSC
buffer, and no change to any existing candidate or phase.

## Retired AMF direction

The P11-P14 sequence established a firm boundary.  Several independent
post-final relabelled-AMF candidates improved the public exact objective, but
their official benchmark runs repeatedly failed the hidden two-second cap even
after narrow dimension, nnz, and degree-structure gates.  The hidden corpus
contains an AMF cost tail the public corpus does not identify reliably.  All
relabelled-AMF alpha, seed, and gate variants are retired.

P15 is a different mechanism.  It calls robust/no-dense AMD, not AMF.  Dense
column deferral and aggressive absorption are disabled through
`AmdOptions { aggressive: false, dense_alpha: -1.0 }`, but the implementation
remains the same quotient-graph AMD machinery used throughout the promoted
source.  There is no AMF fill-score bucket scan.

## Why a late fixed seed

The promoted low-nnz restart policy deliberately stops at 200 seeds.  Adding
more candidates upstream is unsafe for a separate reason: TELOS descends from
the incumbent produced by the portfolio.  A better upstream ordering can move
TELOS into a worse local basin on hidden data.  P9 demonstrated that public
strict wins can reverse officially through this seed interaction.

Experiment 0018's deep census found a material residual on `ndcc12` at seed
2666.  P15 isolated that point by screening robust AMD alone after the complete
pipeline.  Eight fixed late seeds were tested on 234 public matrices under a
broad temporary screen:

`257, 474, 521, 1021, 1531, 1988, 2666, 4093`.

Seven seeds produced no strict win.  Seed 2666 alone improved `ndcc12`:

| matrix | n | nnz | max degree | inherited | candidate | exact gain |
|---|---:|---:|---:|---:|---:|---:|
| `ndcc12` | 974 | 5,982 | 47 | 345,696 | **342,379** | **3,317** |

The full candidate pass measured roughly one millisecond on that row.  P15
production therefore contains one fixed seed, not a widened restart loop.

## Exact promoted allocation path

The new block is placed after PHARMAKOS, when the complete promoted path has
finished.  Inside the structural gate it performs exactly the allocation style
already used by `5141ad0`'s relabel phases:

1. `let q = relabel(n, 2666)` allocates the deterministic Fisher-Yates
   permutation using the promoted SplitMix64 stream;
2. `permute_pattern(&scoring_pat, &q)` constructs the relabelled vendor pattern;
3. the promoted-style iterator casts allocate `Vec<i32>` column pointers and
   row indices;
4. `feral_ordering_core::CscPattern::new` validates the borrowed i32 pattern;
5. one robust AMD pass produces the relabelled ordering;
6. composition through `q` returns original vertex numbers;
7. the existing bijection check rejects malformed output;
8. the existing exact `ScoreWorkspace::flops` scorer compares the candidate;
9. only `candidate_flops < best_flops` replaces the inherited final.

The scorer workspace is already part of exact `5141ad0`; using it here does not
import b23's relabel-buffer optimization.  The candidate construction itself
uses fresh promoted-style allocations on every call.

## Placement and monotonicity

Placement after PHARMAKOS is the score-safety invariant.  Before the block,
`best_perm` has already passed through the original portfolio, relabelled AMD,
relabelled AMF alpha 5, no-dense AMD restarts, hill climb, post-hoc transforms,
SHOT-3, TELOS, MINL, ORBIT, and PHARMAKOS.  The new ordering cannot seed any of
those phases because none remains after it.

A hidden candidate can be better, equal, or worse than the inherited final.
Strict exact comparison returns the new ordering only in the first case.  Ties
and losses preserve the original permutation.  This is stronger than an
upstream best-of argument because there is no downstream local optimizer whose
basin can be perturbed.

## Gate and runtime boundary

The production predicate is:

```rust
n <= 1_000 && nnz > 0 && nnz <= 6_000
```

It admits 137 of 300 public rows and retains `ndcc12`, which sits near both
ceilings.  The predicate is a cost envelope, not a matrix identity.  Inputs
outside it take the exact promoted source path and do not allocate `q`, the
permuted pattern, or either i32 cast buffer.

The envelope is far below every known cap wall:

- at most 1,000 vertices;
- at most 6,000 off-diagonal entries;
- one quotient-graph AMD pass;
- no AMF call;
- no extra loop or adaptive wall-clock decision.

Local timing cannot certify a hidden corpus, but this mechanism removes both
known risk amplifiers: AMF's hidden structure-sensitive tail and b23's
allocation/binary-layout delta from the promoted source.

## Verification plan and decision rule

The exact parent is verified both by `git rev-parse` and merge-base before the
candidate commit.  The production candidate worker must compile from this
worktree.  Output parity is checked against the previously measured P15 result:

- gate population remains 137 public matrices;
- `ndcc12` remains inside the gate;
- its exact final remains 342,379 flops;
- the full permutation fingerprint remains the P15 winning fingerprint;
- full-corpus A/B must show one strict win, zero losses, one changed
  permutation, and no changes outside the gate.

If any of those values differs, the restack is rejected rather than explained
away as an allocation optimization.  If they agree, keep the exact-base commit
as the only P15 carrier.  Do not combine it with b23 or any retired AMF block,
and do not submit from this experiment.
