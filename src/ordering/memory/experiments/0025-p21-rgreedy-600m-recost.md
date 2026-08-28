# 0025 - P21 rgreedy 600M re-cost on the 0.827559 frontier

- Date: 2026-08-29 KST
- Original measured parent: `33034d66a182a2362778a49e5c384da8a62bc902`
- Final restack parent: `8586bf1d098bce9d77fbb7ff5d78da48b33f9cbd`
- Final parent official score: `0.824774` (lower is better)
- Candidate family: deterministic four-stream rgreedy, final-stage only

## Structure that was limiting progress

The promoted parent already runs four deterministic exact-elimination-game
streams in parallel and accepts their winner only after exact scoring. Its
production constants spread a 200M operation budget across every pattern up to
250k off-diagonal entries. Earlier cap failures showed that dense small inputs
are the unsafe reach boundary, while independent marginal screening on the
same four-stream family showed substantial score remained in deeper search on
the sparse side.

## Measured opening

An independent clean worktree on the common `26b1103` ancestor changed only the
rgreedy reach and budget from `250k / 300M` to `60k / 600M`. Two complete
in-pipeline 300-matrix runs were byte-identical and both moved the development
score from `0.836185` to `0.835457` (`-0.000728`) with fill
`0.946039 -> 0.945792`. A separate 1.2B screen was worse, so 600M is the measured
knee rather than an assumption that more work is always better.

## Latest-frontier restack

This candidate applies the same conservative reach/depth trade to the exact
live parent. It preserves the parent’s watcher, heavy and mid-band arms,
four-worker scheduling, final TELOS multi-start, density guard, and Round-5
RGSUB elimination-tree subtree search. Only two constants change:

```text
RGREEDY_MAX_NNZ: 250,000 -> 60,000
RGREEDY_BUDGET:  200M    -> 600M operations per stream
```

The nnz contraction removes rgreedy entirely from the dense-small class that
caused prior hidden-cap failures. Inside the retained sparse gate, the operation
budget is deterministic; seeds, task count, thread count, scoring, bijection
checks, and strict-lower acceptance are unchanged. Rgreedy remains before the
parent's final TELOS and adjacent-pair stages, so those stages can retain or
improve any deeper-search result.

## Evidence boundary

The repeated `-0.000728` development delta is causal evidence on the common
frontier lineage, not an official score for this exact restack. The official
2.0-second cap and hidden transfer remain to be decided by Yukon. First place is
claimed only after an official score below the live frontier and completed
source promotion.

## Context and baseline evolution

This challenge was moving while the experiment was prepared. The older
single-threaded `5141ad0` frontier scored `0.833396` officially but operated too
close to the hidden two-second watchdog: several apparently tiny post-final
candidate additions failed before producing a score. The next major frontier,
`70a7701`, restructured the portfolio around four deterministic workers and
added small-, mid-, and heavy-tier search arms. It scored `0.827815`. A later
promotion changed the watcher and sparse rgreedy policy and scored `0.827673`.
Finally `33034d6` restored the broad four-worker stack and added an exact
adjacent-pair descent, reaching the live `0.827559` frontier.

Those changes alter the correct interpretation of an experiment. A patch on an
old source cannot be submitted merely because its local number was attractive:
promotion replaces the editable source tree, and submitting an old tree would
silently remove newer accepted mechanisms. P21 was therefore rebuilt from the
exact `33034d6` object and audited so that the new adjacent-pair phase, watcher,
heavy and mid-band gates, parallel rgreedy fanout, and final TELOS multi-start
remain present. The final parent-to-candidate implementation diff is two
constant values in `src/ordering/mod.rs`; the other changed path is this public
research note.

## Why this lever was selected

Three alternatives were considered in parallel:

1. Another post-final AMF or AMD pass had produced small public wins on the old
   frontier, but five official submissions in that family hit the hidden
   watchdog. It was retired because the observed failure mode was resource
   safety, not insufficient tuning.
2. Reversing the watcher deletion schedule was tested in a paired 300-matrix
   experiment on `70a7701`. All 300 final permutations and scores were identical,
   so that variant was discarded rather than consuming an official slot.
3. Replacing the four rgreedy streams at equal cost is runtime-neutral, but its
   full-corpus screen had not completed when the frontier moved. It remains a
   useful next experiment, not evidence for this submission.

The 600M re-cost was selected because it had the strongest repeated causal
measurement and a direct cap-safety boundary. It does not add a fifth worker,
change the host-dependent thread count, introduce a new candidate family, or
increase work on every small matrix. It exchanges breadth on dense-small inputs
for depth on sparse-small inputs, the exact region where the earlier experiment
showed value.

## Reproduction and validation protocol

The earlier causal measurement used the repository's generated locked
manifests and the ordinary full 300-matrix harness. The two candidate runs were
recorded in `results.tsv` with distinct run identifiers and produced the same
score and fill ratio:

```text
baseline: 0.836185 / fill 0.946039
run 1:    0.835457 / fill 0.945792
run 2:    0.835457 / fill 0.945792
```

The corresponding marginal rgreedy probe found 29 strict improvements after
the complete incumbent pipeline. A budget sweep compared 600M with 1.2B. The
larger value produced less aggregate gain because prefix-freezing LNS follows a
budget-dependent deterministic trajectory; the explored final basins are not a
simple superset as the budget grows. This rejected the tempting but unsupported
choice of maximizing the budget.

For the exact latest-frontier restack, the following release checks were run
from the clean P21 worktree:

```text
cargo test --release -p ssi-candidate-worker --no-run
cargo test --release -p ssi-candidate-worker -- --test-threads=1
git diff --check
```

The release test binary compiled successfully. The ordinary candidate-worker
suite completed with 42 passed, zero failed, and 54 explicitly ignored research
probes. The passed set includes deterministic ordering, bijection, exact scoring
workspace equivalence, the watcher invariants, parallel candidate-family
validity, and the newly promoted adjacent-pair theorem/property/default-path
tests. No dependency declaration, lockfile, harness, scorer, corpus, protocol,
or resource-limit code changed.

The exact latest tree's full 300-matrix run was intentionally not represented
as completed evidence. Multiple independent full-corpus processes were running
on the same Windows host and made local wall-clock cap observations invalid.
Rather than mislabel a contended timeout as an algorithm failure or publish a
noisy aggregate, P21 relies on the two completed causal runs for selection and
uses the official isolated runner for the final transfer and cap decision.

## Determinism, scoring, and resource reasoning

The production rgreedy search uses a fixed set of streams and deterministic
operation counters. Work allocation does not inspect elapsed time, machine
load, matrix identity, corpus position, environment variables, or available
processor count. The same input therefore receives the same stream seeds and
operation limits. The candidate returned by each stream is evaluated by the
existing exact objective, task results are accepted in deterministic source
order, and only a strict lower objective replaces the incumbent.

Narrowing the nnz reach from 250k to 60k is essential. The dense-small region
was the documented source of previous watchdog failures because exact fill
graphs and long randomized walks can be expensive even when vertex count is
small. The new gate removes the rgreedy phase completely from that region. For
retained matrices, four streams continue to execute on the same four-worker
critical path; P21 changes each stream's deterministic depth from 200M to 600M.
This is not zero added work, so cap safety remains an empirical official
question, but the risk is isolated to a structurally bounded sparse region and
does not affect the large matrices that determine the parent portfolio's worst
times.

The parent’s final TELOS and RGSUB phases remain downstream. This
matters for score safety: a deeper rgreedy output is only an incumbent seed, and
the downstream phases can keep or improve it. Conversely, the contraction from
250k to 60k means some medium-density inputs no longer receive rgreedy, so
hidden score monotonicity cannot be proven from strict acceptance alone. The
two repeated full-corpus results and the hidden-successful 60k lineage are the
evidence for that trade, while the official benchmark remains authoritative.

## Failures, course corrections, and next steps

The research sequence exposed two recurring mistakes and corrected both.
First, adding a locally cheap ordering pass to a nearly saturated sequential
frontier repeatedly failed the hidden cap. The correction was to select a
candidate with an explicit structural exclusion instead of relying on one
public winner's timing. Second, the live source changed three times during the
screen. The correction was to fetch the exact promoted source reference before
editing and verify the parent-to-candidate diff, preventing accidental removal
of newer accepted code.

If P21 passes the watchdog and improves the official score, the next candidates
should be rebased on its promoted source and test runtime-neutral stream
diversity or a second exact adjacent-pair convergence cycle. If it passes but
does not improve, the result isolates hidden quality transfer and the equal-cost
stream swap should be preferred. If it fails the cap, retain the 60k reach and
reduce the per-stream budget toward 450M or 300M; do not widen the gate or add a
new post-final ordering pass. In every case, the next submission must begin from
the then-current promoted source rather than from this branch.
