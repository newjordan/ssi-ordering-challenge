# Low-hub widening of the promoted witness-driven completion descent

## Starting point

This submission starts from promoted commit `d020def`, official hidden score
`0.832843`, fill ratio `0.939882`. That revision introduced an event-driven
witness watcher as an independent sibling of ordinary completion
minimalization. It was the first submission after `5141ad0` to improve the
private score and is therefore the new frontier rather than an experimental
side branch.

The promoted implementation is intentionally conservative. Its public broad
census found eleven matrices improved by the watcher, but the submitted gate
retained eight of them:

```text
n <= 30,000
0 < fill_count <= 145,000
5 * fill_count >= filled_arc_count
```

The submitter's measured note states that this preserves 99.06% of the broad
census gain. The ceiling excludes `crudeoil_lee4_06` and `mpbp_48`; the
dimension cap excludes a giant unitcommit case. The fill-fraction predicate,
not the 145,000 ceiling, excludes both known ringpack timing walls.

## Hypothesis

The fill ceiling can be widened enough to recover the two finite-dimensional
responders without reopening the hub-driven timing failure class if the gate
also checks the original pattern's maximum degree.

The repository already uses `max_deg <= n / 50` as its load-bearing guard for
mid-band relabel work. The documented reason is causal rather than
name-specific: `ringpack_30_2` and `crudeoil_lee4_10` are near twins in
dimension and nonzero count, but ringpack has a maximum degree roughly ten
times larger. That hub makes quotient and completion work expensive. The same
cheap pattern statistic is available before the watcher adjacency clone.

The candidate gate is therefore:

```text
n <= 30,000
0 < fill_count <= 180,000
5 * fill_count >= filled_arc_count
max_pattern_degree <= n / 50
```

This remains a structural policy. It does not use matrix names, hashes, corpus
positions, exact dimensions, or values. It can apply to unseen sparse patterns
drawn from the same distribution.

## Why 180,000

The promoted 145,000 limit was selected to exclude two public rows and bound
the watcher storage near 12 MiB. The broad predecessor allowed 600,000 fill
edges and was rejected because it admitted timing walls and raised the noisy
full-run worst. This submission does not return to that broad regime.

`180,000` is a narrow 24% increase over the promoted ceiling and only 30% of
the retired broad ceiling. Four intrusive watcher nodes per fill edge add
approximately:

- four `u32` arrays for previous/next/support bookkeeping;
- one `u32` head per fill edge;
- live and queued flags;
- a bounded queue;
- the independent adjacency clone.

The increase from 145,000 to 180,000 adds only a few MiB in the admitted region
and remains far below the 4 GiB worker address-space limit. The unchanged
60-million operation budget and 2,000-common-neighbor ceiling continue to fail
closed.

The added maximum-degree predicate is deliberately evaluated in the same
short-circuit condition as the existing n, fill-count, and fill-fraction
checks. A rejected pattern performs no watcher adjacency clone and allocates no
watch arrays.

## Correctness and score safety

The watcher algorithm is unchanged. It deletes a fill edge only when the exact
local chordality condition succeeds. A failed test records a unique-chord
four-cycle witness and wakes the candidate only if a removable support edge is
later deleted. Budget exhaustion stops deletion and leaves a chordal
supergraph. The existing maximum-cardinality-search realization produces a
candidate permutation, which is bijection-checked and exact-scored.

The widened watcher remains a sibling candidate from the identical pre-MINL
completion. Ordinary MINL remains available. The final permutation changes
only when the widened watcher returns a strictly lower exact predicted flop
count than every incumbent candidate. Consequently, objective regression is
impossible unless resource limits are exceeded; the new low-hub guard and
narrow fill ceiling are specifically intended to prevent that.

All fused-relabel performance work from the promoted frontier is retained
unchanged. The candidate does not add relabel restarts, TELOS seeds, ordering
families, dependencies, randomness, wall-clock decisions, or external I/O.

## Scope

Only `src/ordering/` changes:

- `WATCH_MINL_MAX_FILL` increases from 145,000 to 180,000.
- The watcher admission condition additionally requires
  `max_deg <= n / 50`.
- This experiment note records the reasoning and official-runner-only
  evaluation policy.

No dependency, harness, scorer, corpus, build, or grader file is modified.
Rust standard library only.

## Evaluation policy

Per the user's explicit instruction, this exact candidate is not run against
the local corpus. The official Yukon runner is the first authority for build,
purity, determinism, hidden timeout, and score. Static checks are limited to
the allowed edit scope and whitespace validation.

The expected outcomes are:

1. If no hidden pattern lies in the newly admitted low-hub fill band, the
   score remains the promoted `0.832843`.
2. If a hidden analogue of the two excluded public responders is present, the
   strict watcher sibling can improve it and move below the frontier.
3. If the widened class contains a hidden expensive pattern despite the
   low-hub guard, the official timeout will reject the submission; no score is
   self-reported.

The experiment is intentionally small because the leaderboard margin is now
thin. It seeks the measured residual of the already successful mechanism
instead of introducing a new search family or paying another broad census at
runtime.

## Submission

The submission is attributed to model `GPT 5.6 Sol` and harness `Codex`.
The public note contains no credentials or private corpus information.
