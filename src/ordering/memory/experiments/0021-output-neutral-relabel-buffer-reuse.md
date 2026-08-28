<!-- 0021 -->
# 0021 — Output-neutral relabel buffer reuse

- **Date:** 2026-08-29
- **Base:** exact promoted source `5141ad0`
- **Objective:** increase hard-2s-cap headroom without changing a single public
  permutation or flop count
- **Status:** candidate; no official submission made

## Bottleneck

The relabelled AMD, relabelled AMF, relabelled no-dense AMD, hill-climb, and
count-relabel phases all repeated the same preparation sequence for every seed:

1. allocate a fresh `Vec<usize>` relabeling;
2. call vendor `permute_pattern`, which allocates a fresh `usize` CSC pattern;
3. allocate two more vectors while casting both CSC arrays to `i32`;
4. run the ordering and then discard all of those buffers.

This is especially wasteful in the low-nnz restart floor, which runs up to 200
seeds. The production tree already contained
`ScoreWorkspace::permute_i32_into`: a faithful allocation-reusing port of vendor
permutation with a direct vendor-equivalence unit test. Its own comment said it
was intentionally left unwired for a future cap-safety pass. This experiment is
that pass.

## Change

- Preserve every restart budget, gate, seed, candidate order, ordering option,
  strict-`<` winner rule, and downstream phase.
- Reuse one `relabel_q`, one i32 `col_ptr`, and one i32 `row_idx` allocation for
  the full call to `order()`.
- Route relabel/HC/count-relabel pattern construction through the existing
  `ScoreWorkspace::permute_i32_into` implementation.
- Compose the returned ordering directly to `Vec<usize>` instead of allocating
  an intermediate `Vec<i32>` and immediately converting it.
- Do not grow the portfolio permutation cache with restart candidates. A cache
  miss is still scored through the identical exact workspace scorer; this only
  removes a clone and hash-table insertion. The cache is a memoization layer,
  so removing an insertion cannot change the value computed for any candidate.
- Preserve the original producer `catch_unwind`, bijection check, test-only
  runner-up bookkeeping, exact score calculation, and strict winner update.

No scoring or search parameter changed. The optimization is independent of the
P7/P9 relabel-budget work.

## Exact public-corpus A/B

A test-only `probe_output_fingerprints` records a stable FNV-style hash of the
entire returned permutation plus the exact symbolic flop count for every public
matrix. It was run once before the production change and once after it, both in
release mode from this same worktree.

| check | result |
|---|---:|
| baseline rows | 300 |
| candidate rows | 300 |
| permutation-hash mismatches | **0** |
| exact-flop mismatches | **0** |

The whole release-mode fingerprint test, which runs full `order()` and an exact
symbolic flop pass on all 300 rows, took **165.80 s baseline vs 150.93 s
candidate** in sequential runs on the same host: -14.87 s / **-8.97%**. Host
load makes that aggregate wall-clock figure indicative rather than a hard
guarantee; the isolated same-process A/B below is the causal measurement.

## Isolated restart-preparation A/B

`probe_relabel_buffer_reuse` runs old and new preparation paths in the same
process, five times each, and reports the minimum. It excludes AMD/AMF itself so
the measured delta is exactly the code removed by this change.

| matrix | shipped restarts | old | reused | speedup |
|---|---:|---:|---:|---:|
| `crudeoil_lee1_07` | 31 | 7.076 ms | 6.059 ms | **1.168x** |
| `ringpack_30_2` | 2 | 4.548 ms | 2.666 ms | **1.706x** |
| `nuclear10a` | 1 | 2.465 ms | 1.827 ms | **1.349x** |
| `squfl030-150` | 24 | 18.083 ms | 9.521 ms | **1.899x** |

The speedup is present on all four structurally different representatives,
including the public worst-case `ringpack_30_2` and the low-nnz multi-restart
class. Absolute end-to-end savings depend on how much of a matrix's time is in
ordering/scoring rather than preparation, but this path is strictly less work
and removes allocator variance from every restart.

## Validation commands

```text
cargo test --release -p ssi-candidate-worker permute_i32_matches_vendor -- --nocapture --test-threads=1
cargo test --release -p ssi-candidate-worker probe_output_fingerprints -- --ignored --nocapture --test-threads=1
cargo test --release -p ssi-candidate-worker probe_relabel_buffer_reuse -- --ignored --nocapture --test-threads=1
```

The direct vendor-permutation equivalence test passed. The full fingerprint A/B
passed 300/300 exactly. No submission was made.

## Decision

**KEEP AS AN OUTPUT-NEUTRAL CAP-HEADROOM CANDIDATE.** Unlike P4/P6/P8, this adds
no candidate family and no gated expensive path. It removes repeated allocation,
copy, conversion, cache-insertion, and hashing work from an already-shipping
phase while preserving all public outputs exactly.
