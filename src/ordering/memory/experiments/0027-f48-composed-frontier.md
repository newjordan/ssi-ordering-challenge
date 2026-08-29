# Experiment 0027: compose the accepted frontier mechanisms on f48

## Status and decision

Submission candidate on exact accepted frontier
`f48a17f6b1757ca6d2ef399bf70aa3f286d44b27`, prepared with GPT-5.6 Sol in
the Codex harness. The parent is the validation commit for submission
`2b91c868-bb49-4e71-87da-2a2414165255`, which promoted at official score
`0.817268` and fill ratio `0.929124` after adding the dense low-alpha arm.

The candidate restores four mechanisms that were accepted or independently
validated on the other moving frontier but were absent from the `f48a17f`
production tree:

1. the narrow MID800 PEP operation floor;
2. deterministic D1 seed diversification in RGSUB round two;
3. the conditional third RGSUB round; and
4. deterministic striped S0 diversification on odd top-32 round-two blocks.

The current parent already contains the relabel-alpha routing, broad PEP policy
selector, and pristine-degree reset cache from the PR286 line. Those are not
re-applied. It also contains the new dense low-alpha passes. The candidate is
therefore a composition of the strongest independently accepted source lines,
not a stale overlay submitted on an obsolete parent.

A fresh exact current-parent full-corpus comparison is positive:

```text
f48a17f baseline                         0.817933 / 0.934048
f48a17f + MID800 + D1 + round 3 + S0    0.817717 / 0.933883
delta                                  -0.000216 / -0.000165
```

Submit only if `origin/main` is still exactly `f48a17f`, the documented final
tree passes the full release suite, the worktree is clean, and the account has
no active validation. A new promotion retires this branch and requires another
fresh restack and A/B.

## Why the restack is necessary

The race has multiple accepted source lines that do not contain one another's
production changes. The commit graph is linear at validation time, but an
accepted validation commit may carry the submitter's source tree rather than a
simple patch on the immediately previous production tree. Score comparison
alone therefore cannot establish that all earlier mechanisms remain present.

`f48a17f` was submitted from the PR286 production line. It includes:

- relabelled AMD/AMF dense-alpha cycling inside unchanged restart counts;
- the four-stream PEP policy selector selected by mean degree;
- the `deg0` pristine-degree cache with historical operation charging; and
- the newly accepted dense low-alpha AMF/AMD candidates for the heavy dense
  `400,000 <= nnz < 1,000,000` class.

It does not include the MID800/D1 production changes accepted in `70846ff`, nor
the conditional third round later accepted through submission `01cd43b`, nor
the striped S0 replacement whose hidden run improved its submitted parent in
submission `8f94e5d`.

The new branch starts directly from `f48a17f`. The three production commits are:

```text
197f688  restack MID800 and round-two D1
45e86bd  restore the conditional third RGSUB round
01e2afc  stripe round-two S0 on alternating top-32 blocks
```

No commit from the obsolete `0d4da3e` branch is used as the ancestry base.

## Source-identity proof

The previous fully measured composition before the new frontier was production
commit `7e5f1a4`. It contained PR286, MID800, D1, the conditional third round,
and striped S0. A whitespace-normalized production comparison between that tree
and the new `01e2afc` candidate changes only `src/ordering/mod.rs`, with exactly
53 inserted lines: the dense low-alpha arm now accepted in `f48a17f`.

```text
src/ordering/mod.rs | 53 +++++++++++++++++++++++++++++++++++++++++++++++++++++
```

`src/ordering/rgreedy.rs` is identical after normalization. This proves both
directions that matter:

1. none of the previously qualified production mechanisms was lost during the
   current-main reconstruction; and
2. the only additional production behavior is the accepted dense low-alpha
   source from the new parent.

The current candidate remains a direct descendant of `f48a17f`; the comparison
to `7e5f1a4` is an identity audit, not its submission ancestry.

## Mechanism A: narrow MID800 operation floor

The PEP rgreedy phase already selects four deterministic search streams. For
`n > 1,000`, the accepted parent gives those streams a 600M operation floor.
Two previously measured sparse density bands retain enough headroom to use the
same 800M floor already used below `n <= 1,000`:

```text
nnz <= 2*n
or
4*n <= nnz and 2*nnz <= 9*n
```

The phase gate is unchanged. The stream policy, seeds, stream count, parallel
merge, exact scorer, and strict acceptance rule are unchanged. This is the only
added operation budget in the MID800 mechanism.

The dense low-alpha parent arm requires `nnz >= 400,000`, whereas PEP requires
`nnz <= 60,000`. The two additions cannot execute on the same matrix. The new
heavy hidden win therefore does not consume the time headroom used by MID800.

## Mechanism B: equal-work D1 round-two seed

Ranked large-matrix RGSUB searches up to 96 blocks. The top 32 receive S0 and
D1; ranks 32 through 95 receive only D1. Before this mechanism, D1 could repeat
the identical deterministic trajectory in round two when the relevant block
and incumbent state were unchanged.

The candidate xors D1's round-two seed with:

```text
0xA076_1D64_78BD_642F
```

under the exact predicate:

```rust
n >= 10_000 && k == 1 && cfg.round == 1
```

No block, stream, round, trajectory, or operation unit is added. The same D1
jobs receive a different fixed seed. The change is deterministic and can also
interact positively with the new dense parent arm because RGSUB reaches large
dense matrices under its existing `nnz <= 1,500,000` gate. The current-parent
full A/B captures that interaction instead of assuming independence.

## Mechanism C: conditional third RGSUB round

The parent runs two RGSUB rounds. The candidate permits a third only when:

```text
n >= 10,000 and nnz <= 100,000
or
n < 10,000 and nnz <= 30,000 and max_degree * 50 <= n
```

The loop reaches the third round only when the preceding round made a strict
whole-matrix improvement; otherwise it breaks. The block ceiling, two-stream
allocation, 5M per-trajectory budget, ranking, hard cap, and whole-matrix
admission remain unchanged.

This mechanism adds work on a narrow sparse class, but that class is disjoint
from the dense low-alpha arm's `nnz >= 400,000` window. The exact third-round
source already passed the hidden cap and promoted in submission `01cd43b`.
The present branch remeasures its end-to-end interaction with the stronger
current parent.

## Mechanism D: equal-work striped S0

S0 also repeats on unchanged blocks between rounds. In round two, odd ranked
top-32 blocks xor S0's fixed seed with:

```text
0xE703_7ED1_A0B4_28DB
```

The predicate is:

```rust
n >= 10_000 && k == 0 && cfg.round == 1 && block_rank & 1 == 1
```

Ranks at or above 32 already skip S0, so exactly 16 existing round-two S0
trajectories are replaced. No search is added. The same patch previously
validated on the hidden corpus and improved its submitted parent by about
`-0.000005`; it was rejected only because another validator moved main first.

## Exact matched 300-matrix comparison

Both revisions were built independently from source and launched concurrently
on the same 20-core arm64 Mac Studio. Each evaluated all 300 development
matrices under the repository's normal two-second process watchdog. Compiler,
lockfile, corpus, harness, host load, and launch window were matched.

Authoritative `results.tsv` summaries:

| build | score | fill | score delta | fill delta |
|---|---:|---:|---:|---:|
| exact `f48a17f` parent | 0.817933 | 0.934048 | 0.000000 | 0.000000 |
| exact `01e2afc` composition | **0.817717** | **0.933883** | **-0.000216** | **-0.000165** |

Displayed bucket geomeans:

| build | lt_1k | 1k_10k | gt_10k |
|---|---:|---:|---:|
| exact parent | 0.8923 | 0.8614 | 0.7296 |
| composition | 0.8923 | 0.8611 | 0.7293 |

All 300 rows completed in both runs. The middle and large buckets improve. The
small bucket is unchanged at four-decimal display precision. Fill improves as
well as the primary flop score.

Several rows demonstrate that the composition is not merely recovering an old
aggregate mechanically. For example, the candidate changes RGSUB outcomes on
large matrices already touched by the dense parent, including
`pooling_sppc1pq` and `pooling_sppc3pq`. It also recovers the previously
measured sparse and middle-tier movements. The complete pipeline result is the
quality gate because an earlier strict improvement can reseed later phases.

The exact candidate score `0.817717 / 0.933883` also matches the previous
composition's full-300 result. That is expected from the identity proof: the
new dense arm moves no development row according to its accepted experiment,
while the source containing every other mechanism is identical. The fresh run
confirms the expectation rather than relying on it.

Generated result rows are stashed after reading the authoritative summaries so
both Nimbus worktrees return to a clean state.

## Hidden evidence

The parent supplies the most important new hidden evidence: its dense arm moved
the official score from `0.820246` to `0.817268`, a `-0.002978` improvement,
while passing the official cap and correctness gates. This candidate preserves
that exact accepted source.

The restored mechanisms have separate hidden evidence:

- MID800 plus D1 produced accepted tree `70846ff` and scored `0.820344`, beating
  the then-current PR286 score by `-0.000122`.
- the conditional third-round submission promoted by `-0.000068` to
  `0.820246` on its frontier;
- striped S0 improved its submitted parent by `-0.000005` and passed hidden
  validation, although it lost the race to a simultaneously promoted tree; and
- the PR286 mechanisms already present in `f48a17f` were themselves accepted
  and are not modified here.

These results do not replace the current-parent development A/B. They establish
cap and transfer evidence for each component while the fresh A/B establishes
the exact combined source behavior.

## Risk analysis

### Time cap

The parent dense arm adds heavy-matrix AMF/AMD passes, but its window is
disjoint from both added-work mechanisms in this candidate. MID800 runs only in
the PEP `nnz <= 60,000` envelope. The third round runs only at `nnz <= 100,000`
for large rows or `nnz <= 30,000` for sub-10k rows. The parent dense arm starts
at `nnz = 400,000`.

D1 and S0 are equal-work replacements. They can change wall time slightly by
choosing different elimination paths, but they retain the same deterministic
operation budget and hard cap. The pristine-degree cache already in the parent
removes physical reset work while preserving logical accounting.

The exact final tree must still pass all five release time-cap tests. No wider
block ceiling, fourth round, TELOS runner-up descent, or second whole-matrix
rgreedy search is included.

### Determinism and purity

All selectors use matrix dimensions, deterministic restart or block indices,
fixed integer comparisons, and fixed constants. No wall clock, entropy source,
randomized hash state, environment variable, filesystem input, network input,
or thread completion order enters the new decisions. Candidate results remain
thread-local until the existing deterministic merge.

### Correctness and admission

Every generated permutation passes the existing bijection validation and
trusted whole-matrix scorer. Only strict improvements replace the incumbent.
Local monotonicity is not claimed as end-to-end monotonicity because later
phases can be reseeded. The full 300-matrix comparison measures the final
pipeline and is therefore the decisive score evidence.

### Frontier volatility

Several public validators started while this candidate was measured. The
branch must not be submitted merely because it descends from what was current
at construction time. The final dispatch gate fetches `origin/main` twice,
checks the exact expected SHA, verifies ancestry and a clean worktree, checks
the note size, and confirms the account slot is free. Any mismatch retires the
candidate without an official submission.

## Reproduction protocol

The two isolated Nimbus worktrees used the normal offline release path:

```bash
bash scripts/prepare-build.sh
cargo build --release -p ssi-ordering-challenge --offline --locked
bash scripts/local-candidate-build.sh
cargo run --release --offline --locked -- --note "<revision label>"
```

Worktrees:

```text
/Users/Nimbus/work/repos/matrices-f48-baseline
/Users/Nimbus/work/repos/matrices-f48-composed
```

Release qualification for the documented final commit is the complete
`cargo test --release --offline --locked` suite: unit, exact-equivalence,
scorer cross-check, security-boundary, and time-cap tests. After qualification,
refresh the live frontier and both global and account queues, then repeat those
checks immediately before invoking Yukon.

## Excluded alternatives and next work

- No 128-block RGSUB ceiling; that variant failed the hidden cap.
- No runner-up TELOS descents or second whole-matrix rgreedy search; both added
  real work and failed hidden validation.
- No fourth RGSUB round or widened third-round density gate.
- No post-hoc gate fitted to the visible 45-row large bucket.
- The D1 tail-prefix operator experiment remains isolated and unmeasured on the
  new frontier.
- Deterministic unchanged-null tracking between RGSUB rounds remains the next
  higher-leverage equal-work research direction.

The race sequence is: qualify and submit this exact current-main composition
only if every live gate stays green; monitor the official result without
cancelling it; and continue researching the next isolated successor while the
slot validates.
