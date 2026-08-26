# 0012 — Exact minimum-flop DP at small `n`: the bottom of `lt_1k` is SOLVED

**Outcome: NEGATIVE, and conclusively so.** The portfolio is already *provably
optimal* on every dev matrix with `n <= 24`. Zero improvements out of 30.
This closes a question the knowledge base had only ever answered statistically.

## The idea

Every candidate family in this repo is a heuristic. For small enough `n` the
scored objective can be optimized EXACTLY, so we can stop guessing and measure
the true remaining gap.

In the elimination game the fill graph after eliminating a *set* `S` does not
depend on the order within `S` (the standard result behind elimination trees).
So a partial elimination is fully described by `S`, the column count charged
when `v` is eliminated out of `S` is `c(v,S) = 1 + |N_{G_S}(v)|`, and the
objective is additive:

```
f(S ∪ {v}) = min_v [ f(S) + c(v,S)² ],   f(∅) = 0
```

`f(V)` is then the exact minimum of `Σ cⱼ²` over all `n!` orderings.

Implemented in [`../exact_dp.rs`](../exact_dp.rs). The one implementation trick
worth keeping: `c(v,S)` needs `v`'s neighbourhood in the fill graph, which is
its own neighbours outside `S` plus the outside-neighbours of every connected
component of `G[S]` it touches. Those components depend only on `S`, so they are
computed ONCE per state and reused for all `n` pivots — an O(n) BFS per
transition becomes a few `u32` word ops. Distinct components of `G[S]` are
non-adjacent by construction, so no chaining between them is possible.

## Why this class looked promising

- 30 dev matrices have `n <= 24`, and **all 30 together** consume 0.015 s of
  `order()` time against a 2 s per-matrix cap. The budget there is effectively
  unbounded — whatever we do costs nothing.
- 13 of those 30 sit at ratio **exactly 1.0000**, i.e. the entire portfolio
  fails to beat plain AMD on them. Under the best-of floor each is pure upside.
- [0010](0010-parallel-research-session.md) had established `lt_1k` is
  METHOD-limited rather than budget-limited (flat from 24 to 2000 restarts).
  "Exact" is the strongest possible new method.

## Result

`probe_exact_dp` in [`../probe.rs`](../probe.rs), which for every matrix with
`n <= EXACT_MAX_N` compares AMD, the shipped portfolio, and the exact optimum:

| `n` window | matrices solved | strictly improved | worst exact solve |
|---|---|---|---|
| `n <= 20` | 23 | **0** | 0.084 s |
| `n <= 24` | 30 | **0** | 1.513 s |

`exact == portfolio` on every single matrix. Not "close" — bit-for-bit equal
objective values on all 30.

## What this means

1. **The bottom of `lt_1k` is finished.** [0010](0010-parallel-research-session.md)
   concluded `lt_1k` is method-limited from the flatness of the restart curve;
   this is the stronger statement, and it is a proof rather than an inference:
   for `n <= 24` there is NOTHING left, because the portfolio already returns the
   optimum. Do not spend any further effort below `n = 25` — not more restarts,
   not a new metric, not annealing. The 13 matrices tied at 1.0000 there are tied
   because **AMD is optimal on them**, not because we under-searched.
2. **It does not scale, so it cannot be shipped anyway.** Cost is `2^n · n`. The
   worst solve at `n = 24` is 1.513 s against a 2.0 s SIGKILL, with the ~1.6x
   run-to-run variance this repo has measured repeatedly. Even if it had won,
   `n = 24` would be unshippable and `n <= 20` (0.084 s) covers only 23 dev
   matrices that are already optimal. The window where it is both affordable and
   useful is EMPTY.
3. **It is a strong correctness instrument, and worth keeping for that.** A
   completely independent algorithm reproduced the portfolio's optimum on 30
   matrices, and the DP's internal accumulator agreed with the trusted
   `Σcⱼ²` scorer on every one. `probe_exact_dp` asserts bijection, asserts
   DP-objective == trusted-scorer, and asserts `exact <= portfolio`. Any future
   change that breaks the scoring path or returns a non-optimal ordering on a
   tiny matrix will now fail loudly and cheaply.

## Where the headroom is NOT

Combined with [0010](0010-parallel-research-session.md)'s finding that the tie
seam is inert at the large end (11 of 12 tied large matrices flat to 200
restarts), the search-depth story is now closed at BOTH ends of the size range.
Remaining gains have to come from candidate DIVERSITY (new families, per
[0011](0011-relabelled-amf-multistart.md)) or from reclaiming and respending
time budget — not from searching harder anywhere.

## Reproduce

```sh
cargo test --release --offline --locked -- --ignored --nocapture --test-threads=1 probe_exact_dp
EXACT_MAX_N=24 cargo test --release --offline --locked -- --ignored --nocapture --test-threads=1 probe_exact_dp
```

`EXACT_MAX_N` defaults to 20. Above 24 the `u32` bitmask and `u8` back-pointer
stop fitting (`STRUCTURAL_MAX_N`), and the table memory is `5 · 2^n` bytes.
