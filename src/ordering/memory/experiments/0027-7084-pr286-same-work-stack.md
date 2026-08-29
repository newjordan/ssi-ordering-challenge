# 0027 — Recompose the accepted same-work routing stack on 7084

## Result

Starting from accepted frontier `70846ff`, the relabel-alpha routing, broad PEP
stream-policy selector, and pristine-degree reset cache from the previously
accepted `13123bc` tree improve the full 300-matrix development score from
`0.817932` to `0.817858` and the fill ratio from `0.934030` to `0.934007`.
The weighted score delta is `-0.000074`.  The small and middle buckets improve;
the large bucket is byte-for-byte unchanged at aggregate precision.

This is deliberately a low-risk composition.  It does not add a restart, a
search stream, an RGSUB round, a block, or an operation-budget unit.  Two
changes route already-budgeted work to different deterministic policies, and
one change avoids recomputing pristine degrees while preserving the old
deterministic operation charge.  The same three mechanisms were accepted by
the hidden grader in submission `85a2ca7` before `70846ff` replaced that tree.

## Live-parent discipline

The race was moving quickly while this experiment was prepared.  Earlier
source-level A/B evidence existed on the `13123bc` line, but that acceptance
commit was not an ancestor of the new frontier after submission `4bf806c`
promoted.  Reusing the old overlay as a submission would therefore have been a
stale-parent error even though its production source happened to describe the
desired combined solver.

The experiment was rebuilt instead from the live parent:

```text
baseline parent: 70846ffbfbc108735fc0a0199a52ae43acb0f0aa
candidate:       44dfc9c (production experiment commit)
production diff: src/ordering/mod.rs, src/ordering/rgreedy.rs only
```

The two fresh worktrees were built from these exact revisions and run
concurrently on the same 20-core Apple Silicon host.  That removes machine,
corpus, compiler, and concurrent-load differences from the A/B comparison.
Both rows completed all 300 matrices under the normal per-process two-second
watchdog.

## Hypothesis

The current frontier already contains the narrow MID800 budget change and the
equal-work second-round D1 seed diversification from experiment 0026.  The
question here is whether the three mechanisms previously accepted on the
alternative `13123bc` frontier remain useful when composed onto that solver.

The expected interactions were:

1. Relabel alpha cycling should retain independent small-bucket wins because
   it changes candidate diversity inside an unchanged restart envelope.
2. The broad PEP selector overlaps some of MID800's middle-tier wins, so its
   marginal gain should be smaller in this composition than in isolation.
3. The pristine-degree cache should be score-neutral and should not change
   deterministic search admission, because the pre-cache operation charge is
   retained explicitly.
4. The second-round D1 large-bucket improvement should be unaffected because
   these mechanisms do not add or remove RGSUB work.

The full A/B confirms exactly that shape: most marginal value is in the small
bucket, a smaller gain remains in the middle bucket, and the large bucket is
unchanged relative to `70846ff`.

## Mechanism A — relabel alpha cycling with unchanged restart count

The frontier already spends a fixed, per-matrix budget on relabelled AMD and
AMF candidates.  The old implementation used the same dense threshold on every
restart.  The candidate keeps the exact restart count, seeds, closure count,
relabel permutations, scoring path, and strict acceptance rule, but cycles the
dense threshold in three measured density bands.

For relabelled AMD the cycle is:

```text
[10.0, 5.0, 2.0, 16.0]
```

For relabelled AMF it is:

```text
[5.0, 2.5, 10.0, 1.0]
```

Outside the selected density bands, the exact promoted defaults remain in
force.  Slot zero also uses the former default, so matrices receiving only one
restart are unchanged.  The selection is a pure function of `n`, `nnz`, and
the deterministic restart index.  No clock, scheduling state, random device,
or previous candidate result influences it.

This is candidate diversification rather than additional compute.  Every
candidate still passes through the existing bijection validation and trusted
flop scorer, and only a strict improvement can replace the incumbent.

## Mechanism B — choose the existing four-stream PEP policy by mean degree

The PEP gate already launches four exact-elimination search streams within the
same per-stream operation envelope.  The old branch always used the default
seed policy.  The candidate selects the varied seed policy when:

```text
40 * nnz <= 207 * n
```

Both multiplications are saturating.  The selector is therefore deterministic,
integer-only, and safe at the type boundary.  More importantly for the hidden
time cap, both arms retain the same four streams and the same budget passed to
the search engine.  The change decides which already-budgeted trajectories are
explored; it does not widen the PEP gate or add a second search.

This distinction matters because several attractive public experiments failed
the hidden cap after adding work.  A second rgreedy search from the TELOS
runner-up failed, as did a 128-block RGSUB ceiling.  Neither mechanism is part
of this experiment.  Here, the broad PEP policy selector is a replacement
inside the current envelope and has already survived an official hidden run in
the accepted PR286 tree.

## Mechanism C — cache pristine degrees without changing search accounting

`rgreedy::Game::reset` restores the pristine bitset adjacency before every
stream.  Previously it also recomputed every pristine degree from that bitset
on every reset.  The candidate computes the same degree vector once in
`Game::new`, stores it as `deg0`, and restores it with `copy_from_slice`.

The output is identical because `deg0[v]` is defined by the same popcount over
the same pristine row that reset used to evaluate.  The bucket rebuild, live
list rebuild, and all subsequent mutations are unchanged.  Crucially, the old
operation charge remains unchanged even though the physical popcount work is
removed.  Search-stream admission and deterministic budget exhaustion therefore
remain compatible with the pre-cache implementation; the cache cannot buy
extra randomized runs merely by making reset cheaper.

The cache is performance-neutral or favorable in wall time and neutral in the
algorithm's logical work budget.  It is not used to justify a larger cap.

## Exact 300-matrix A/B

The fresh current-parent results are:

| Metric | `70846ff` baseline | Candidate | Delta |
|---|---:|---:|---:|
| weighted flop score | 0.817932 | **0.817858** | **-0.000074** |
| geomean fill ratio | 0.934030 | **0.934007** | **-0.000023** |
| lt_1k flop ratio | 0.892472 | **0.892277** | **-0.000195** |
| 1k_10k flop ratio | 0.861332 | **0.861280** | **-0.000052** |
| gt_10k flop ratio | 0.729477 | 0.729477 | 0.000000 |
| completed matrices | 300 | 300 | 0 |

Fill ratios by bucket also move in the expected direction:

```text
lt_1k:   0.962167 -> 0.962141
1k_10k: 0.954101 -> 0.954050
gt_10k: 0.897875 -> 0.897875
```

The arithmetic agrees with the earlier independent source-equivalent A/B on
the PR286 line (`0.817933 -> 0.817858`).  The one-unit baseline difference in
the sixth decimal comes from the different current parent; the combined
candidate lands at the same six-decimal score.  This repeat is useful evidence
that the contribution is not a one-run artefact.

## Reproduction protocol

Both revisions were prepared and built independently with the repository's
normal release pipeline:

```text
bash scripts/prepare-build.sh
cargo build --release -p ssi-ordering-challenge --offline --locked
bash scripts/local-candidate-build.sh
cargo run --release --offline --locked -- --note "<revision label>"
```

The baseline and candidate commands ran concurrently on distinct clean
worktrees.  Their generated `results.tsv` rows were stashed after reading the
authoritative `score.json` files so the worktrees returned to a clean state.
The compiler, lockfile, corpus, host, and watchdog were identical.

The production-identical documented revision passed the complete release test
suite: 52 unit tests, 3 exact-equivalence tests, 2 scorer cross-checks, 4
security-boundary tests, and 5 time-cap tests, with zero failures and the two
expected sandbox-only ignores.  The live `origin/main` and all public and
account submissions are refreshed again immediately before dispatch.  A new
promotion retires this parent and requires another restack; a pending account
submission prevents dispatch.

## Risk analysis

The primary hidden risk in this challenge is the strict per-matrix time cap,
not a development-score regression.  This composition controls that risk in
four ways:

- It adds no restart, stream, round, block, or logical operation budget.
- The PEP change is mutually exclusive policy selection inside the existing
  four-stream envelope.
- Relabel alpha cycling replaces options in existing restart slots.
- The reset cache removes physical work while retaining logical operation
  accounting.

Quality is also monotone at each candidate acceptance boundary: generated
permutations are validated and freshly scored, and only strict improvements
replace the incumbent.  Downstream seed sensitivity can still make a different
early winner change later trajectories, which is why the complete 300-matrix
A/B is required rather than relying on local monotonicity alone.  The measured
aggregate result is positive in both affected buckets and neutral in the third.

The composition intentionally excludes newer public added-work experiments
while they are validating.  In particular, it does not copy the runner-up
TELOS extra descents, a broad third sparse RGSUB round, the failed second
rgreedy search, or the failed 128-block ceiling.  Those are separate hypotheses
with separate hidden-cap evidence and must not hitchhike on this low-risk run.

## Decision

Submit this exact current-main composition if and only if the frontier remains
`70846ff`, the full release suite passes, the worktree is clean, and the account
has no active validation.  If any public validator promotes first, do not send
the old-parent overlay: reconstruct these three mechanisms on the new accepted
tree and repeat the exact A/B.

## Follow-ups

1. Compare the official hidden result with the development delta to estimate
   how much of the small-bucket relabel diversity transfers.
2. Let public added-work validators establish cap safety before considering
   their mechanisms; never compose an unvalidated workload increase into this
   submission.
3. Continue searching for replacement-policy, seed-routing, cache, and
   deterministic tie-order changes, because those are the most promising way
   to gain score without consuming the remaining hidden time margin.
4. Preserve the exact `70846ff` baseline and candidate artifacts so any future
   composition can be compared to a known current-parent result rather than a
   stale aggregate.
