# 0022 — zero-allocation relabel search pays for a bounded low-nnz expansion

- **Date:** 2026-08-28
- **Parent:** promoted `5141ad0`, hidden score `0.833396`
- **Development score:** `0.842223 -> 0.842103`
- **Development fill tiebreak:** `0.948942 -> 0.948893`
- **Status:** full 300-matrix capped run and 66 active tests passed

## Context and failure diagnosis

The first experiment in this checkout changed the TELOS descent seed. Although
it improved the development corpus, five official attempts ended `failed`
without metrics. The CLI does not print the hidden failure reason, so treating
that pattern as a general validator outage was incorrect. The benchmark's
public GitHub Actions logs are authoritative: every one of those attempts
reached the benchmark and ended with:

```text
RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed
```

That rules out packaging, compilation, purity, and score as the immediate
problem. The promoted portfolio already spends most of the cap on some hidden
small sparse matrix. Even exchanging TELOS rounds rather than adding one can be
slower because the AMD-seeded graph enters a more expensive basin. The TELOS
change was therefore removed completely before this experiment. The original
promoted incumbent-seeded TELOS loop is restored.

The new approach begins with cost deletion. It reuses machinery already present
in `ScoreWorkspace` and changes no ordering candidate by itself. The recovered
time then pays for a larger deterministic prefix of the existing low-`nnz`
relabel search. This is deliberately one existing family, one existing budget
formula, and two output-neutral implementation fixes rather than a new ordering
algorithm.

## Change 1: bound the exact-score memo table

`consider` scores a candidate and memoizes its permutation in
`HashMap<Vec<usize>, u64>`. The early portfolio contains meaningful duplicate
candidates, but the relabel phases generate hundreds of almost-always-distinct
permutations on the low-`nnz` class. An unbounded table clones every such
`n`-element key, hashes it, grows its buckets, and later rehashes the retained
keys. None of that changes the exact symbolic flop computation.

The table is now capped at 96 entries. Existing entries remain readable for the
whole call, and a miss is still scored by the same `ScoreWorkspace::flops`.
Only insertion is skipped after the cap. Memoization cannot affect the returned
ordering, so this is output-neutral by construction. The bound is intentionally
above the roughly forty early portfolio candidates where cross-family duplicate
hits occur. It is not a new cache policy or eviction scheme; after the useful
prefix, the cheapest cache is no insertion at all.

## Change 2: reuse the existing permutation workspace

The AMD, AMF, no-dense AMD relabel loops and the AMD hill-climb previously did
the following on every restart:

1. allocate a fresh `usize` permuted CSC pattern;
2. allocate and populate a fresh `Vec<i32>` column-pointer array;
3. allocate and populate a fresh `Vec<i32>` row-index array;
4. run the unchanged feral ordering routine;
5. compose the returned local order through the same relabel permutation.

`src/ordering/scoring_ws.rs` already contained
`ScoreWorkspace::permute_i32_into`, including a direct vendor-equivalence test,
but the production loops did not call it. The loops now reuse one lazily-created
permutation workspace plus two caller-owned `Vec<i32>` buffers. The three loops
that run through the long-lived `consider` closure use a second workspace to
respect Rust's exclusive borrow of the scoring workspace. Later hand-written
hill-climb and count-relabel loops reuse the primary workspace after the last
`consider` call.

The permutation, candidate orderer, seed sequence, composition, bijection check,
exact flop scorer, strict comparison, and fallback behavior are unchanged. The
repository's `permute_i32_matches_vendor` test compares this carrier directly
with feral's permutation path. This edit removes allocation traffic rather than
replacing an algorithm.

## Change 3: spend the recovered time on the existing low-nnz search

For `nnz <= 20_000`, the promoted restart rule is:

```rust
(600_000 / nnz).clamp(1, 200)
```

The candidate uses:

```rust
(900_000 / nnz).clamp(1, 250)
```

The rule remains a deterministic integer function of `nnz`, and the candidate
sequence remains seeds `1..=restarts`. This is a bounded prefix expansion, not
an elapsed-time loop. Every new ordering passes through the same bijection and
exact-flop checks, and AMD remains the initial per-matrix floor.

The 20,000 edge ceiling is load-bearing. It excludes the larger sparse-heavy
class and confines the larger multiplier to the class on which the cache and
permutation allocation deletions recover their time. The cap of 250 also
matters: a small graph can still have an expensive per-restart structure, and
`restarts * cost_per_restart`, not `nnz` alone, determines wall time.

The final score is not mathematically monotone relative to the promoted whole
portfolio. Within the relabel phase the candidate set is a strict superset, but
downstream TELOS is seeded by the current best permutation and can reach a
different basin. The development corpus contains both wins and losses, with a
favorable net weighted result:

| metric | promoted | candidate | delta |
|---|---:|---:|---:|
| weighted flop score | 0.842223 | **0.842103** | **-0.000120** |
| weighted fill | 0.948942 | **0.948893** | -0.000049 |
| `lt_1k` flop geomean | 0.899432 | 0.899432 | 0 |
| `1k_10k` flop geomean | 0.875342 | **0.8749** | approximately -0.0004 |
| `gt_10k` flop geomean | 0.774477 | 0.774477 | 0 |

Representative gains include `crudeoil_lee1_07`; representative regressions
include `edgecross14-156`. The public gain is entirely in the medium bucket,
where the low-`nnz` restart expansion changes the search depth. The zero-allocation
carrier and memo-table bound do not intentionally change any candidate bytes.

## Verification

The host load average was checked before the scored run and was below 3.0. This
matters because the two-second watchdog measures unguarded wall clock and prior
research recorded false cap failures under heavy machine contention.

The exact commands for the final source were:

```sh
bash scripts/local-candidate-build.sh
yukon run
cargo test --release -p ssi-ordering-challenge -- --test-threads=1
```

The sandboxed build succeeded with network denied. The normal benchmark scored
all 300 development matrices, running the candidate twice per matrix under the
two-second watchdog, checking deterministic equality and bijection validity,
and recomputing the exact symbolic flop and fill objectives. It completed at
`0.842103` with fill `0.948893`.

The release suite passed 66 active tests with zero failures: 52 unit tests, 3
closed-form exact-equivalence tests, 2 scorer cross-checks, 4 security-boundary
tests, and 5 time-cap tests. Two pre-existing sandbox probes remained explicitly
ignored. The time-cap suite includes a real slow worker kill, so its roughly
65-second duration is expected.

Only `src/ordering/` is part of the submission. No dependency, manifest,
corpus, harness, scorer, watchdog, build script, or generated result is changed.
The implementation uses the Rust standard library and ordering crates already
vendored by the challenge.

## Hidden-corpus expectation and limits

The previous official failures establish that hidden time margin, not public
score alone, is decisive. This candidate directly attacks the hot allocation
path executed by the low-`nnz` restart class before increasing its work. The
bounded cache removes hundreds of full-permutation key clones; the permutation
workspace removes repeated CSC and cast-buffer allocations. Both savings apply
to the same class receiving the larger restart prefix.

This is stronger evidence than adding the restart budget alone, but it is not a
proof against every hidden structure. A hidden graph can have a much higher
ordering constant than a public graph with the same `(n, nnz)`, and the official
watchdog remains the only authoritative cap test. Likewise, the public net win
does not guarantee the hidden corpus has the same TELOS basin response.

The first submission, `b4b9fdb3`, timed out on a hidden matrix. The next revision
therefore retains both output-neutral speed fixes but reduces the search rule to
`(900_000 / nnz).clamp(1, 250)`. It does not add another gate or candidate
family. If it scores but is rejected, the hidden objective result
will determine whether the restart expansion transfers. If it is promoted, the
speed changes should remain as headroom for future work rather than immediately
being consumed by another broad search.
