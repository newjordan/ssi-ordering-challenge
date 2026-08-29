# Stack the proven watcher onto the four-thread frontier

## Base

This submission starts from promoted commit `5f0dfb0`, official score
`0.830299`, fill ratio `0.938676`. That frontier introduced a four-thread
output-identical portfolio carrier, exact elimination-game LNS for small
patterns, and recosted mid/heavy relabel windows.

Its parent, promoted commit `d020def`, scored `0.832843` and introduced an
independent witness-driven completion minimalizer. The new four-thread commit
removed that module while replacing the surrounding ordering implementation,
so the two proven score gains have not yet been composed.

## Change

Restore `src/ordering/minl_watch.rs` exactly from `d020def` and register the
watcher as a third completion-descent task beside ordinary MINL and ORBIT.
All three tasks start from the same pre-descent incumbent. The parallel carrier
already executes independent permutation tasks with deterministic task-index
acceptance, so completion order cannot affect the returned permutation.

The restored gate is exactly the promoted watcher gate:

```text
n <= 30,000
0 < fill_count <= 145,000
5 * fill_count >= filled_arc_count
```

No widening is attempted. An earlier official experiment widening the fill
ceiling failed the hidden timeout, so this stack uses only the identity gate
that has already passed the complete official runner and earned a private
score improvement.

## Mechanism

For a chordal completion, a fill edge `uv` is non-deletable exactly when two
common neighbors `x,y` are nonadjacent. The four-cycle `u-x-v-y-u` then has
`uv` as its unique chord. That witness stays valid until one of its removable
support edges disappears.

Ordinary MINL repeatedly scans surviving fill edges. The watcher instead
records up to four support-edge watches for each blocked candidate and wakes
that candidate only after a watched support is deleted. This changes the valid
deletion schedule and can reach a different inclusion-minimal chordal
completion from the same starting completion.

Every deletion uses the exact local chordality test. Budget exhaustion fails
closed. The resulting graph is realized with the existing MCS PEO routine,
bijectivity is checked by the parallel scoring carrier, and the candidate is
accepted only when its exact predicted FLOP count strictly improves the
incumbent.

## Why this composition is timely

The watcher previously improved eight public matrices and moved the hidden
score from `0.833396` to `0.832843`. The new frontier independently moved the
hidden score to `0.830299`, mostly through small-tier exact-game search and
new relabel candidates.

The mechanisms are orthogonal:

- exact-game LNS constructs new eliminations directly for `n <= 3000`;
- relabel windows explore AMD/AMF tie-breaking and objective variants;
- the watcher descends from the final incumbent's chordal completion.

The watcher runs after TELOS and receives whatever incumbent the new frontier
produces. Strict best-of means it may discover fewer, equal, or different wins,
but it cannot worsen an ordering.

## Runtime argument

The previous watcher frontier executed MINL, watcher, and ORBIT largely
serially and still passed the official cap under its tight gate. The new
frontier reports a local worst of `0.773 s` versus `1.225 s` for the old
single-threaded frontier, with the completion siblings explicitly scheduled as
parallel tasks.

Adding the watcher as task index 1 makes the batch:

1. ordinary MINL;
2. witness-driven MINL;
3. ORBIT.

The runner exposes four vCPUs and the carrier uses at most four scoped workers.
Thus the watcher consumes an otherwise available worker rather than adding its
full isolated duration to the critical path. Its promoted gate bounds fill
storage and excludes the known ringpack walls before watcher arrays are
allocated.

The operation budget remains 60 million, the common-neighbor ceiling remains
2000, and no wall-clock decision is introduced.

## Determinism

Parallel tasks write results by task index. Acceptance is sequential in the
declared source order and keeps only the earliest strict argmin. Randomized
small-tier search uses its existing fixed seed and is complete before this
batch begins. The watcher itself has fixed queue order and no randomness.

The result is therefore a pure function of the sparsity pattern and remains
identical across repeated runs.

## Scope and evaluation

Only `src/ordering/` changes:

- restore the previously promoted `minl_watch.rs`;
- add its module import and four promoted constants;
- enqueue one tightly gated watcher sibling;
- add this public experiment note.

No dependency, corpus, scorer, harness, or grader file changes. Standard
library only.

Per the user's instruction, this exact stack is not benchmarked locally. The
official Yukon runner is the authority for compilation, policy, determinism,
resource limits, and score. Static validation is limited to the allowed path,
the concise source diff, and whitespace checks.

## Expected result

If the new incumbent completions retain watcher-deletable structure on hidden
patterns, the score should improve below `0.830299`. If all former watcher wins
are subsumed by exact-game LNS or relabel candidates, it should tie and be
rejected. The already-promoted gate and concurrent scheduling are intended to
make timeout risk substantially lower than in the previous serial watcher
frontier.

Submission attribution: model `GPT 5.6 Sol`, harness `Codex`.
