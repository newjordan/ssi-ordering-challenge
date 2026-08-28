<!-- 0022 -->
# 0022 - the TELOS second seed is BLOCKED: three grader failures, every cheap cause refuted

- **Date:** 2026-08-28
- **Score:** no change shipped. Promoted candidate remains graded `0.833396` (#1).
- **Status:** blocked, with the causal question OPEN. Do not attempt a fourth variant
  without new information (a grader log, or a reproduction of the grader environment).

## What is being attempted

Exploit 0020's finding that `TELOS-GRADIENT DESCENT` is seed-sensitive, by descending from
the AMD anchor as well as the incumbent. Measured value on dev: **6 matrices better, 0
worse**, dev score `0.842223 -> 0.842032` (additive form) or `-> 0.842015` (budget-split
form). Both forms are real gains. Neither can be shipped.

## Three attempts, one controlled variable

| submission | form | worst dev `order()` | total bench time | outcome |
|:--|:--|--:|--:|:--|
| `b904857` | additive 2nd descent, gate `nnz < TELOS_MAX_NNZ` | 1.828 s | ~221 s | **failed** |
| `f6e0c2d` | **no TELOS change** (restart floors only) | 1.784 s | 216 s | **promoted 0.833396** |
| `3689919` | additive 2nd descent, gate `nnz <= 40_000` | 1.849 s | ~221 s | **failed** |
| `d390ef6` | budget-SPLIT: 1 round incumbent + 1 round anchor | 1.818 s | ~221 s | **failed** |

The only variable that tracks the outcome is *whether the TELOS seed loop was restructured
at all*. Three restructurings failed; the one submission that left it alone promoted.

## Hypotheses refuted, each by measurement not argument

1. **Global wall-clock budget on the benchmark command.** Timed back to back on an idle
   machine: promoted **216 s**, TELOS variant **221 s**. `+2.3%`. Dead. (My earlier
   180->250-330 s figures were contaminated by an included load-wait loop - my instrumentation
   error, not a property of the build.)
2. **Per-matrix cap via added time.** Attempt `d390ef6` splits the EXISTING round budget:
   `rounds_per_seed = max_rounds / seeds`, so the total number of descent rounds is
   unchanged and the worst case is exactly what the promoted build already does. It failed
   anyway. This kills added-time as the mechanism.
3. **Corrupted / stateful `ws.flops`.** Re-scored **every** `consider` candidate against the
   workspace-free `flops_of` path across all 300 matrices: **0 mismatches**. `flops_cache` is
   a per-call local `HashMap` (`mod.rs:1574`), so no cross-matrix contamination either.
4. **Fatal warnings.** No `-D warnings` anywhere in the harness; the 7 warnings the crate
   emits are pre-existing on the promoted build too.
5. **The `#[ignore]`d byte-identical-to-`legacy` equivalence test.** The benchmark command is
   `bash scripts/local-candidate-build.sh && cargo run --release` - no `cargo test` at all.
6. **Build sandbox.** `local-candidate-build.sh` builds under bubblewrap with network denied
   and writes confined to target/cargo/tmp. The change interacts with none of it.
7. **Memory.** Peak extra allocation is ~4 permutations of `n` (tier A caps `n` well under
   150k), i.e. single-digit MB against a 4 GiB per-matrix cap.

## What is still standing

Something about the restructured loop fails in the grader environment on input we cannot
see. The residual suspects, none testable from here:

- a graded matrix that makes the anchor-seeded descent take a pathological path;
- a panic reachable only on a matrix shape absent from the 300-matrix dev corpus (the
  `std::mem::take(&mut best_perm)` in the no-anchor branch leaves `best_perm` empty until the
  loop's first assignment - locally always restored, and `telos_out` is unconditionally set
  once per iteration, but this is the one novel invariant the change introduces);
- an interaction with the parent process's per-matrix supervision that the local
  `yukon run` path does not exercise identically.

## Decision, and the discipline behind it

Stopped after three attempts. Expected value is `-0.0002` of dev score against a 3-for-3
failure rate and no diagnostic channel. A failed submission cannot demote a promoted one, so
the attempts were positionally free - but they are not free in time, and the same hours buy
more elsewhere.

Preserved so a future session with a grader log can finish it in one step: the exact
budget-split patch is the cheapest form to retry, because its worst-case work is provably
identical to the shipped build, which removes an entire class of explanation up front.

## Method note worth keeping

The measurement harness (`~/ssi-ordering-LEARNINGS/bin/ssi-measure.sh`) earned its keep here.
`diff promoted` proved the tree was byte-equivalent to the promoted submission after each
revert (0 better, 0 worse over 300 matrices) - which is a far stronger check than "the score
matches", and it is what let three risky variants be tried and backed out without ever
endangering the live #1.
