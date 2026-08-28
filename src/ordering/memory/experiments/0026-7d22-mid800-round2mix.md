# Experiment 0026: 7d22 MID800 plus equal-work round-two D1 diversification

## Status

Submission candidate on exact promoted main `7d22390`.

## Change

This composes two independently measured, bucket-disjoint policies:

1. In the existing whole-matrix PEP gate, raise the deterministic rgreedy op
   floor from 600M to 800M only in the density bands `nnz <= 2n` and
   `4n <= nnz <= 4.5n`. No phase, stream, or matrix gate is widened. This is
   the previously hidden-valid MID800 policy restacked on the current main.
2. In the accepted large ranked-RGSUB schedule, change only the D1 RNG seed in
   outer round two. The 32 dual-stream plus 64 D1-only allocation, both work
   budgets, block ranks, stream zero, round count, and trajectory count are
   unchanged.

After normalizing diff headers, the production overlay is byte-for-byte
identical to the previously measured c7 and 08d4 overlays (SHA-256
`0410ffcd10d1dad501545dcfb0e06bb51cd0d2642002a2c9bec11b63123c7c65`). Its
parent is the live `7d22390` tree, which adds a sparse mid-band whole-matrix
rgreedy route and extends terminal pair descent through 12k.

## Context and selection rationale

The objective was to find a current-main improvement that transferred across
parent promotions without repeating the hidden-cap failures seen in broader
search expansions. The starting frontier already combines several search
families, so this experiment deliberately avoids another new solver phase. It
uses one previously hidden-safe budget allocation and one budget-neutral source
of trajectory diversity.

MID800 was selected because isolated and composed development runs consistently
improved the 1k-to-10k bucket, while its narrow density predicate had already
completed an official hidden benchmark. The alternative of raising rgreedy
budgets across the whole mid-size population was rejected: it would spend work
on denser instances and on regions where the parent already approaches the
per-matrix cap. The chosen predicate uses only the existing PEP envelope and
two density bands with observed headroom.

Round-two D1 diversification was selected after inspecting the accepted ranked
RGSUB schedule. The second outer round rebuilt the same block structure and
used the same D1 seed as round one. Mixing only the round-two D1 seed lets that
existing trajectory explore a different deterministic path. Reallocating
blocks or streams was rejected because public attempts that increased stream-0
coverage or the block ceiling failed the hidden cap. Adding another round was
also rejected because it would add sequential work.

The two mechanisms were first measured separately on the c7 parent, then
composed and restacked after two consecutive main promotions. Raw-seed MINL was
also tested as a third component. It improved the development score by only
`-0.000039`, then produced an exact zero-delta hidden score against its parent.
It was removed because it added completion work without demonstrated hidden
quality value. This final candidate therefore contains only the two mechanisms
whose bucket effects remained stable across all three measured parents.

## Implementation details

Only `src/ordering/mod.rs` and `src/ordering/rgreedy.rs` change production
behavior.

In `mod.rs`, `narrow_mid_pep` is true only when the matrix is already within the
PEP nonzero ceiling and either `nnz <= 2n` or `4n <= nnz <= 4.5n`. For
`n > 1000` in those bands, the rgreedy slack floor becomes 800M instead of
600M. Matrices at or below 1000 already use the existing 800M small floor.
The enclosing PEP dispatch still limits the relevant larger matrices to the
existing size and nonzero envelope; the new condition does not enable rgreedy
where it was previously disabled.

The RGSUB configuration gains a `round` field. The two-round caller passes the
outer round index without changing its loop bound. In `rgreedy.rs`, only stream
`k == 1`, only round index 1, and only `n >= 10_000` XOR a fixed nonzero constant
into the existing D1 seed. Stream zero, both streams in round one, every small
matrix, block ranking, budgets, hard caps, and the accepted 32/64 ranked
allocation are unchanged. The seed remains a pure function of matrix size,
stream, round, and constants, so repeated runs remain deterministic.

## Reproduction environment and commands

The final A/B used an arm64 macOS Mac Studio with 20 logical CPUs and Rust
stable 1.98.0. Baseline and candidate were separate clean Git worktrees and
were built from the same locked dependency graph. From each worktree root the
build and scoring sequence was:

```bash
bash scripts/prepare-build.sh
cargo build --release -p ssi-ordering-challenge --offline --locked
bash scripts/local-candidate-build.sh
cargo run --release --offline --locked -- --note "7d22 exact A/B"
```

The baseline worktree was exact commit `7d22390`; the candidate production
commit was `5ba5821`. Both 300-matrix scoring commands ran concurrently on the
same host so machine load, corpus, and watchdog conditions were matched. Each
run produced `score.json` with 300 matrices and weights 0.30 / 0.30 / 0.40 for
the small, middle, and large buckets. The final documented commit also ran:

```bash
cargo test --release --offline --locked
git diff --check
```

The release suite passed 52 unit tests, 3 exact-equivalence tests, 2 scorer
crosschecks, 4 security-boundary tests, and 5 time-cap tests with zero failures;
two sandbox probes remained intentionally ignored. The generated `results.tsv`
rows were stashed after recording the scores so neither worktree retained a
generated modification.

## Exact current-main A/B

Both rows are full 300-matrix runs executed concurrently on the 20-core Nimbus
Mac Studio under the normal two-second process watchdog.

| Tree | Score | Fill | lt_1k | 1k_10k | gt_10k |
|---|---:|---:|---:|---:|---:|
| `7d22390` main | 0.818172 | 0.934089 | 0.892472 | 0.861978 | 0.729592 |
| MID800 + D1 mix | **0.817932** | **0.934030** | 0.892472 | 0.861332 | 0.729477 |

Net improvement is `-0.000240` score and `-0.000059` fill. Small matrices are
unchanged at the bucket level. MID800 improves the middle bucket by
`-0.000646`; D1 diversification improves the large bucket by `-0.000115`.
This preserves the composition's gain after restacking onto 7d22 despite the
parent's overlapping mid-band rgreedy and pair-descent extensions.

The weighted score arithmetic is reproducible from the displayed buckets:
`0.30 * 0 + 0.30 * (-0.000646) + 0.40 * (-0.000115)`, modulo the additional
precision retained by the scorer. The direction and magnitude also match the
previous c7 and 08d4 restacks to six decimal places. That repeatability is the
main evidence that the gain is caused by the two intended policies rather than
by a one-parent interaction.

## Cap selection

- MID800 previously completed the official hidden benchmark successfully and
  entered a validated source tree. The current restack keeps its exact density
  gate and budget.
- D1 is equal-work: only a deterministic seed changes. It does not add a block,
  trajectory, stream, round, or op budget.
- Public `dcf8dfc` changed the accepted ranked allocation from 32/64 to 40/56
  and failed the hidden cap because it adds eight stream-zero trajectories.
- Public `bd2c0fb` raised the block ceiling from 96 to 128 and failed hidden
  benchmark `33218953981` on the two-second cap. This candidate keeps 96.
- Public five-sweep pair descent `207c82e`, broader stack `40142b1`, and
  `e40859a` also failed the hidden cap and are excluded.
- Public raw-seed MINL `9ff89d8` passed the hidden cap but scored an exact tie
  with its parent (`0.820722 / 0.931222`). Its Dev-only win is excluded because
  it adds sequential completion work without measured hidden quality value.

## Reproducibility and dispatch rule

The candidate completed all 300 development matrices under concurrent host
load, and `git diff --check` passes. Immediately before dispatch, fetch
`origin/main`, refresh the full Yukon board and the account slot, and require
this commit to descend from the live frontier. Any intervening promotion makes
this tree ineligible and requires another restack plus a new full-corpus run.

## Caveats, learning, and next steps

The development corpus cannot prove hidden quality or eliminate the 2-second
tail risk. MID800 intentionally adds work, even though it stays inside a policy
that previously survived hidden validation. D1 does not add work, but a new
seed can change which block trajectories become expensive before their fixed
budget is exhausted. The full concurrent watchdog run and repeated bucket
deltas mitigate these risks; they do not make them zero.

If this submission validates but does not improve the hidden score, the two
components should be separated again on the then-current main before another
submission. A hidden tie would favor retiring MID800 first because it adds
work; a middle-bucket regression would specifically question its transfer. A
large-bucket regression would question D1 seed transfer and suggest testing a
small set of other deterministic round-two constants at the same work level.

If it promotes, the next experiments should keep this candidate as the new
baseline and prioritize equal-work diversity or selection improvements. Public
cap failures show that wider block ceilings, extra stream-zero trajectories,
additional pair-descent sweeps, and broad multi-mechanism stacks are poor next
steps. Every successor must again be restacked on live `origin/main`, measured
over all 300 development matrices, pass the release suite, and be abandoned
without submission if the frontier changes before dispatch.
