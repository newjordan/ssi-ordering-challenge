# 0024 — Current-main composition: promoted Top-K + promoted MID800 + a hidden-valid large brake

## Question

The public frontier moved twice while the first cap-safe RGSUB seed-transfer
submission was validating. Narrow MID800 first promoted at `e97cb34`, then the
Top-K TELOS candidate replaced it at `4270fd7`. The accepted Top-K tree does not
contain MID800. Can the two independently accepted policies be composed with a
large-matrix RGSUB change that has itself completed the hidden 500-matrix
watchdog, while retaining enough score to beat the newer frontier?

This page records the exact current-main composition. It is intentionally not a
claim that independently measured scores are always additive. Every component
was ported onto the new source tree and the whole 300-matrix corpus was rerun.

## Exact base and moving-frontier discipline

The production branch starts directly from current `origin/main`:

```text
base commit: 4270fd72b68a22f7c7ca90bc40bb0ad8fae2b5f3
accepted submission: 408ded76-b343-4949-966c-4c088df54bdc
official score: 0.821048
candidate branch: codex/current-topk-plus-mid800-large-brake
candidate code tip: 00b1d251da2dc981099727df2542679f24b251d8
```

The base is the accepted Top-K TELOS tree, including its bounded seed pool,
extra-descent gates, and hub gate-wall trim. No old-base overlay is being
submitted. The frontier and account queue are refreshed again immediately
before dispatch; if `origin/main` changes, this tree becomes research evidence
and must be restacked and rescored.

## Component A: accepted Top-K source

The base retains several strong candidates instead of only the immediate
portfolio argmin, then runs bounded TELOS descents from selected alternative
seeds. This addresses seed sensitivity: a numerically worse starting ordering
can descend into a basin the current incumbent cannot reach. The accepted tree
also skips a measured-zero relabel workload on a hub-heavy gate-wall class.

This experiment does not modify that machinery. It matters to the composition
because both later phases see the incumbent produced by Top-K. Therefore it
would be invalid to add score deltas from an older source tree without rerunning
the complete pipeline. The authoritative result below is from the merged tree.

## Component B: accepted narrow MID800 policy

The Top-K acceptance superseded the immediately preceding MID800 tree, so the
exact accepted predicate was reapplied. Whole-matrix rgreedy already assigns a
minimum word-operation floor according to matrix size. For a narrow PEP class,
the floor changes from 600M to 800M:

```text
nnz <= 60,000
and (
  nnz <= 2*n
  or (4*n <= nnz and 2*nnz <= 9*n)
)
```

The predicate uses saturating integer arithmetic. It reads no clock, entropy,
environment, filesystem, or thread completion order. Search still uses the
same phase, candidate validation, trusted symbolic scorer, and strict
improvement rule. Submission `7080853` validated this mechanism on the hidden
corpus and promoted at official score `0.821458` before Top-K superseded its
tree.

MID800 adds work only inside the pre-existing `n <= 3000` rgreedy gate. It is
disjoint from the large RGSUB brake below, whose behavior changes only above
10,000 vertices. It can still interact through the global best-of pipeline on
small matrices, which is why the full merged score rather than a summed delta
is authoritative.

## Component C: large RGSUB trajectory transfer with a structural brake

The RGSUB phase refines disjoint elimination-tree subblocks of the current
incumbent. Its promoted configuration searches as many as two rounds, 96 blocks
per round, two deterministic streams per block, and 5M word operations per
stream.

The candidate changes the second stream only for `n > 10,000`:

- replace its deterministic RNG seed with the promoted PEP D1 seed
  `0xD1B5_4A32_D192_ED03`;
- retain the existing stream parameters and strict merge;
- cap that transferred stream at `min(parent budget, 3M)` operations;
- keep stream zero at the original 5M budget; and
- run exactly one RGSUB round above 10,000 vertices.

Below and at 10,000 vertices, RGSUB behavior is byte-identical to current main.
Above 10,000, maximum block-search work is one `5M + 3M` pass instead of as many
as two `5M + 5M` passes. The one-round gate also removes a subtle dynamic risk:
a changed first-round basin cannot unlock an additional permute, elimination
tree, column-count, block-search, and trusted-rescore pass that the parent would
have skipped.

The first version of the seed transfer kept the full two-round work envelope.
It scored well on dev but failed the official per-matrix cap. That failure was
treated as strong negative evidence. A sequence of budget and round ablations
then selected the present one-round policy. Its standalone old-base submission
`5d3375dd-8b34-439e-8b81-7162add329b5` completed both the integration job and
the official 500-matrix benchmark successfully. It was rejected only on score
after the frontier moved:

```text
official score: 0.821858
official fill:  0.932042
benchmark:      success, 500 matrices, no cap kill
commit:         bffce54
```

That result is the important safety evidence carried into this composition. It
does not prove every upstream interaction is free, but Top-K cannot make the
large RGSUB envelope larger: the gates, number of blocks, stream budgets, and
round count remain explicit functions of `n`, `nnz`, and constants.

## End-to-end result on the merged tree

The candidate worker was rebuilt through the repository sandbox and the full
300-matrix harness completed:

```text
current Top-K main dev: approximately 0.819376
merged candidate dev:  0.818950
fill tiebreak:          0.934962

bucket                 count    flop geomean
lt_1k                    147       0.8925
1k_10k                   108       0.8619
gt_10k                    45       0.7316
```

The exact merged improvement versus the published current-main dev result is
about `-0.000426`. The large bucket carries the strongest gain, as intended;
MID800 and Top-K continue to contribute in the smaller buckets.

An isolated serial `probe_timing_and_score` then repeated all 300 order calls.
It reproduced score `0.818950`, passed, and measured worst `order()` at
**0.765 seconds** on `crudeoil_pooling_ct3`. The slow large representatives
included `gams05` at 0.733 seconds, `crudeoil_lee4_09` at 0.649 seconds, and
`crudeoil_lee4_10` at 0.615 seconds. Timing is noisy and local timing cannot
prove hidden safety, but these measurements are below the already hidden-valid
one-round carrier and preserve substantial local headroom.

## Correctness, determinism, and scope

Every RGSUB output still passes the existing bijection check and a fresh whole-
matrix trusted symbolic flop calculation. It replaces the incumbent only on a
strict improvement, so AMD and all upstream candidates remain available. The
new seed is a compile-time constant. Budgets, size gates, and the round count
are deterministic integer functions. Parallel work remains on disjoint blocks
and uses the existing deterministic merge path.

Only `src/ordering/mod.rs` and `src/ordering/rgreedy.rs` differ from current
main in production code. There are no dependency, FFI, build-script, network,
filesystem, environment, time, or unsafe-policy changes. `git diff --check`
passes. All 38 active release tests pass; the ignored timing probe passes
separately.

## Decision rule and follow-up

This candidate is suitable for submission only while `4270fd7` remains the
live source and the account has no pending validator. A final refresh of
`origin/main`, the live Yukon board, and the account queue is mandatory just
before dispatch. If a pending candidate promotes first, restack these two small
production diffs on that tree and rerun the complete corpus.

The next research arm is wide Ranked64 block selection plus the D1 seed. Its
public dev result is stronger and its scheduled work decreases, but it is still
in hidden validation. Do not replace the present independently validated brake
with that arm until its official result is known.
