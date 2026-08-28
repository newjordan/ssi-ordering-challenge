# 0024 - isolate the additive biconn AMF true path

- **Date:** 2026-08-29
- **Baseline:** exact official frontier `5141ad0`, official score `0.833396`, dev score `0.842222684`
- **Predecessors:** 0021 additive AMF restack, 0022 static-cost gate
- **Status:** local positive; validation candidate only; not submitted by this experiment

## What failed twice

The additive biconnected-block AMF idea had a reproducible public score win,
but two official attempts failed the hidden workflow before score upload.

The first attempt, submission `085791f9-a1ab-4e23-ae03-bb8b2208f659`, used a
whole-pattern `Pattern::nnz() <= 10_000` gate. It preserved all 24 promoted
block-local relabelled-AMD slots and added one independent plain-AMF candidate
to the largest eligible block. Its official Benchmark step hit the hidden
2.0-second per-matrix cap. The gate bounded the induced AMF edge count but not
the whole matrix vertex count or all existing portfolio work around it. The
public audit had also selected rows using serialized JSON `nnz`, which includes
the diagonal, rather than production `Pattern::nnz()` after diagonal removal.

The second attempt, submission `e9f0f3a9`, corrected the gate to direct static
bounds:

```text
n <= 192 && Pattern::nnz() <= 1_200 && max_deg <= 64
```

That gate contains the entire measured win and excludes every known large-
sparse and hub-heavy shape. Nevertheless it also failed the hidden 2.0-second
cap. The failure exposed a separate structural mistake: the strict gate was
converted to a `0` or `1` `total_cross_family` argument, but every matrix still
entered the globally modified `biconn_heavy_order` implementation.

Even when the argument was zero, that implementation allocated and zeroed
`cross_family_of: Vec<usize>` for every biconnected component, read the vector
for every emitted block, and constructed the cross-family option state on each
block already taking the heavy path. The allocation loop itself broke quickly
at budget zero, but the O(number-of-components) allocation/zeroing and per-
block false-path plumbing remained. On a hidden matrix already near the hard
cap, a nominally disabled feature was therefore not disabled by identity.

This is the important correction to 0022. A tight gate around a budget argument
does not prove cap safety when the callee has already been changed. The false
path must call the original function, not a generalized function configured to
do zero new work.

## Source structure

This experiment starts from exact `5141ad0`, not from either failed worktree.
The original four-argument function remains byte-for-byte unchanged:

```rust
biconn::biconn_heavy_order(pattern, 24, 32, 150_000)
```

A second function, `biconn_heavy_order_with_amf`, is a deliberately separate
true-path specialization. Duplication is intentional. Refactoring both paths
through a shared generalized inner function would reintroduce the same proof
problem: compiler-visible common plumbing could alter the false path, and a
future edit could accidentally make zero-budget work nonzero again.

The caller branches before entering either implementation:

```rust
if n <= 192 && nnz <= 1_200 && max_deg <= 64 {
    biconn_heavy_order_with_amf(pattern, 24, 32, 150_000)
} else {
    biconn_heavy_order(pattern, 24, 32, 150_000)
}
```

The false arm is the exact official-frontier call. It allocates no cross-family
budget vector and reaches no AMF-specific code. The true arm retains the old
24-token/seven-per-block relabelled-AMD schedule, completes that allocation
first, and then buys exactly one independent plain-AMF candidate for the
largest eligible owned block. The new local candidate is compared by exact
block-local `flops_of`; the composed full ordering then goes through the normal
full-pattern exact `consider` floor. No incumbent candidate is removed.

The three static predicates are already available before this call. `n` bounds
all per-vertex vectors and scans, `nnz` bounds induced block edges and sparse
passes, and `max_deg` excludes a small but hub-dominated pattern whose
elimination walk can be disproportionate to total nnz. The true path therefore
cannot reproduce the large sparse hidden shape admitted by the first attempt.

## Exact full-corpus causal A/B

A test-only atomic switch selected the original false call or the specialized
true call inside one release binary. It was run serially over all 300 public
development patterns. Both sides used the same corpus parse, exact symbolic
flop scorer, candidate ordering, deterministic seeds, and downstream phases.
The probe compared final permutations, not just rounded aggregate scores.

| metric | exact `5141ad0` | isolated true-path AMF | delta |
|---|---:|---:|---:|
| weighted flop score | 0.842222684 | **0.841958705** | **-0.000263979** |
| strict-gate population | 86 | 86 | 0 |
| false-gate population | 214 | 214 | 0 |
| false-gate permutation mismatches | 0 | **0** | 0 |
| total permutation changes | - | 2 | - |
| strict flop wins / losses | - | **1 / 0** | - |

The one strict win is `pooling_adhya4pq` (`n=170`, `nnz=932`, production
`max_deg=21`): exact flops `21,640 -> 18,740`. `pooling_adhya4tp` is the second
changed permutation but ties at 23,070 flops. All other gate-true patterns keep
the same exact objective, and every one of the 214 false-gate patterns returns
the same final permutation byte-for-byte.

The paired wall totals were 129.614 s and 129.791 s on a contended Windows
host. They are intentionally not used as cap evidence: the ordering run was
paired for causal output comparison, not isolated timing, and unrelated host
load previously made unchanged frontier calls exceed the official cap. The
cap argument instead rests on false-path identity and direct true-path bounds.
Within the true population the two changed rows were about 0.08 s per complete
`order()` call in this run, far from two seconds even under contention.

## False-path proof boundary

The strongest property of this revision is not "the feature is usually cheap."
It is the following dispatch boundary:

1. For `nnz > 60_000`, the inherited outer biconn-heavy gate skips both calls,
   exactly as `5141ad0` does.
2. For `nnz <= 60_000` but any strict predicate false, the caller invokes the
   untouched original function with the untouched four arguments.
3. Only when all three strict predicates are true can code reach the duplicated
   AMF specialization or allocate its cross-family vector.

This is why the public A/B can demand byte identity on all 214 false rows. A
zero mismatch is expected from the source structure, not merely observed by
chance. The hidden corpus can contain different matrices, but the same
predicate-level identity applies to them.

## Reproduction and validation commands

The causal A/B hook and its raw output were removed after the numbers above
were recorded. The production tree is validated with the repository's own
trusted/untrusted split. On Linux/WSL with bubblewrap and the real public corpus:

```sh
bash scripts/prepare-build.sh
cargo build --release --offline --locked -p ssi-ordering-challenge
bash scripts/local-candidate-build.sh
cargo test --release --offline --locked -p ssi-ordering-challenge \
  --test security_boundary -- --nocapture
cargo test --release --offline --locked -p ssi-ordering-challenge \
  --test time_cap -- --nocapture
```

`prepare-build.sh` regenerates the trusted and candidate manifests, vendors the
locked dependency tree, and runs the purity/native scan. The candidate build is
performed inside bubblewrap with network denied. `security_boundary` verifies
the worker isolation contract; `time_cap` includes the normal public-corpus run
and deliberate slow-worker kill. The Windows release candidate tests are also
run before the LF-native WSL validation so compile/test failures are separated
from line-ending or sandbox setup failures.

Final validation was run from committed revision `c9c7a81` in an LF-native WSL
clone with the real 103,879,806-byte public corpus. `prepare-build.sh` passed,
the root release build passed, and `local-candidate-build.sh` completed inside
bubblewrap with network denied. The Windows release candidate suite passed 35
active tests with zero failures and 36 ignored probes. In WSL,
`--test security_boundary` passed 4/4 and `--test time_cap` passed 5/5 in
68.62 s, including the public-corpus normal run and deliberate slow-worker
kill. These are harness-level cap checks, not a claim about an unobserved
official hidden-corpus result.

The original function identity is reviewed directly from the diff: the only
`biconn.rs` hunk is an insertion after the existing `biconn_stats` function and
before tests. No line in `biconn_heavy_order` is modified. The `mod.rs` diff is
limited to the pre-dispatch branch and the final newline normalization.

## Decision and next boundary

Keep this isolated structure as the only defensible retry of the AMF win. Do
not return to a generalized zero-budget callee, do not widen the three static
bounds, and do not add a second cross-family slot. If an official validation of
this exact false-path-isolated revision still hits the cap, retire the AMF idea:
the remaining explanation would be hidden true-gate behavior or background
grader variance, neither of which public tuning can distinguish safely.

This experiment does not submit. Promotion evidence must be an official score,
not the local `0.841958705`. Until then the result is a validated local
candidate and the public best remains the external official frontier.
