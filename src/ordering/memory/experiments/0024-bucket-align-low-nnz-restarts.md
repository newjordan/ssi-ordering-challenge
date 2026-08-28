# 0024 — Bucket-align the low-nnz restart floor

Date: 2026-08-28

Status: **WIN — 0.820416 → 0.820378 locally (−0.000038)**

Effort: high

## Summary

The promoted `b451435` frontier combines expanded iterative RGSUB, adjacent-pair
descent, an AMD-seeded TELOS gate, and the previously promoted three-million-work
low-nnz relabel floor. A clean `yukon run` on that exact revision showed that the
low-nnz floor's known composition effect remained visible: it helped the medium
bucket but left `lt_1k` at the same slightly regressed 0.893392 measured before
the newer pair and TELOS phases were promoted.

This experiment aligns the floor with the bucket where it wins. The only runtime
change is:

```rust
let small_multi = if n >= 1_000 && nnz <= 20_000 {
```

instead of:

```rust
let small_multi = if nnz <= 20_000 {
```

For `n < 1000`, this does not disable relabelled AMD. It merely stops overriding
the normal budget/cap calculation with the much larger 3,000,000-work floor.
The existing bounded restart policy, AMD anchor, RGSUB, TELOS, pair descent, and
all trusted rescoring remain unchanged.

The full 300-matrix sandboxed benchmark produced:

| metric | promoted baseline | candidate | delta |
|---|---:|---:|---:|
| weighted flop score | 0.820416 | **0.820378** | **−0.000038** |
| weighted fill tiebreak | **0.935341** | 0.935436 | +0.000095 |
| `lt_1k` flop geomean | 0.893392 | **0.893265** | **−0.000127** |
| `1k_10k` flop geomean | 0.862884 | 0.862884 | 0 |
| `gt_10k` flop geomean | 0.733834 | 0.733834 | 0 |
| `lt_1k` fill geomean | **0.962766** | 0.963081 | +0.000315 |
| `1k_10k` fill geomean | 0.955094 | 0.955094 | 0 |
| `gt_10k` fill geomean | 0.899958 | 0.899958 | 0 |

At displayed precision, the weighted primary improvement is exactly the small
bucket delta: `0.30 × (0.893265 - 0.893392) = -0.0000381`. Both other buckets
are unchanged. Fill gets worse, but the benchmark ranks predicted LDLT flops as
the primary score and uses fill only as a tiebreak. This candidate strictly
improves the primary objective, so the fill tradeoff is acceptable and recorded
rather than hidden.

## Baseline provenance

The official source baseline is promoted submission `90f5b23`, commit
`b451435`, score 0.822229. Its public note says it was itself rebuilt from
promoted `37131d5` and composed three independently measured pieces:

1. the 3,000,000-work / 800-cap low-nnz relabel floor;
2. terminal exact adjacent-pair descent; and
3. an AMD-seeded TELOS substitution on a sparse low-hub band.

After `yukon sync`, the worktree was clean at `b451435`. The untouched local
baseline was measured rather than taken from the submission note:

```text
score:    0.820416
tiebreak: 0.935341
lt_1k:    0.893392 / fill 0.962766
1k_10k:   0.862884 / fill 0.955094
gt_10k:   0.733834 / fill 0.899958
```

The local and official values differ because Yukon promotion uses the hidden
grader corpus. Only same-host, same-corpus local before/after values are used to
claim the causal delta. Official validation decides whether it transfers.

## Why this gate is justified

The three-million floor was first promoted independently from an earlier source
tree. When expanded RGSUB later promoted from an older branch, that line was
temporarily lost; recomposing it on `37131d5` changed the local score from
0.820888 to 0.820615. The bucket decomposition of that exact composition was
diagnostic:

```text
lt_1k:   0.893263 -> 0.893392  (+0.000129, worse)
1k_10k: 0.864584 -> 0.863546  (-0.001038, better)
gt_10k: 0.733834 -> 0.733834  (unchanged)
```

Thus the floor's net win was real, but it was not uniformly helpful. The medium
gain outweighed a small-bucket regression. After the additional pair and TELOS
phases promoted in `b451435`, the clean baseline still reported `lt_1k =
0.893392`, exactly preserving the regressed value, while the other phases drove
the medium bucket lower. This provided a direct, low-ambiguity follow-up: retain
the floor for `n >= 1000` and let the established capped policy handle `n <
1000`.

The boundary is not fitted to a matrix name or hidden property. It is the
benchmark's published bucket boundary and matches the aggregate evidence being
acted upon. It also reduces work rather than adding work, so it cannot create a
new watchdog-risk class.

## Why more early search can worsen the final ordering

At first glance, removing restart candidates appears unable to help because the
portfolio accepts candidates only under strict trusted-score improvement. That
argument would be valid if all candidates were collected into one flat best-of
set. The actual pipeline contains downstream seeded local searches.

Experiment 0020 isolated this mechanism on TELOS. Raising the relabel restart
count gave a strict superset of early candidates and a strictly better early
incumbent, yet TELOS descended from that incumbent into a worse final basin.
Starting from the weaker early incumbent reached the better final ordering.
Disabling TELOS alone restored monotonicity; independent restart budgets and the
trusted scorer ruled out budget contention and a best-of implementation defect.

The present result has the same signature. Restricting the oversized early
restart floor improves flop score but worsens fill, while leaving out-of-scope
buckets exactly unchanged. It is therefore best described as seed-basin control,
not as evidence that fewer AMD candidates are intrinsically higher quality.
Future pipeline changes can alter this interaction and must remeasure the gate.

## Validation and safety

Both the baseline and candidate completed `yukon run` over all 300 matrices.
The command rebuilt the candidate worker under the network-denied sandbox and
ran each matrix in a watchdog-controlled process. The candidate produced no
panic, invalid permutation, timeout, or failed matrix. It exited zero and wrote
`score.json` with score 0.820378.

The implementation remains deterministic integer branching over already
available `n` and `nnz`. It introduces no dependency, allocation, randomness,
clock read, I/O, unsafe block, or iteration-order dependency. It changes only
`src/ordering/mod.rs`, inside the permitted `src/ordering/` boundary, and uses
the Rust standard library already used by the implementation.

Runtime risk is strictly reduced for affected matrices. Patterns with `n <
1000 && nnz <= 20000` no longer receive the oversized restart floor; every
other path is byte-identical. The existing per-matrix watchdog therefore has
at least as much headroom under this candidate as under the promoted frontier.
The harmless macOS `kill: -PID: No such process` cleanup messages occurred in
both successful runs and reflect a worker exiting before watchdog cleanup.

`git diff --check -- src/ordering` passes. The source change is one condition
plus explanatory comments. The generated `results.tsv` row is removed before
submission, leaving modifications only under the authorized directory.

## Decision

Ship. The primary delta is small but isolated, reproducible, and structurally
safe. It is comparable to recent frontier improvements that have promoted. The
candidate removes work, restores the small-bucket flop ratio almost exactly to
its pre-composition value, preserves all of the medium-bucket benefit, and
leaves the heavy bucket untouched. The fill tiebreak regression is explicit;
it does not override a strict primary flop-score improvement.

