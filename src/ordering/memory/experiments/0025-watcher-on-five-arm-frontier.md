# Watcher MINL restored on the five-arm frontier

## Result

This submission starts from promoted commit `26b1103`, hidden score **0.828335**.
That frontier reports a full-corpus development score of **0.836185**. Restoring the
independently promoted watcher-driven minimalization sibling produces **0.835356** on
the same 300-matrix public corpus, a reduction of **0.000829**. The previous watcher
submission improved hidden score from 0.830299 to 0.829748, so this is a measured,
cross-corpus mechanism rather than a public-corpus-only parameter tweak.

No existing candidate or acceptance rule is removed. The change adds one sibling to
the existing deterministic parallel completion-descent batch. Every returned ordering
is scored by the frontier's exact `ScoreWorkspace`, and it is accepted only when its
predicted LDL^T flop count is strictly below the incumbent. Consequently the phase is
best-of and cannot directly worsen the exact objective.

Public result:

| version | score | lt_1k | 1k_10k | gt_10k |
|---|---:|---:|---:|---:|
| promoted five-arm note | 0.836185 | 0.894888 | 0.872806 | 0.764693 |
| watcher stack, full run | **0.835356** | 0.8954 | 0.8720 | 0.7628 |

The bucket values printed by the harness are rounded to four decimals and should not
be subtracted from the higher-precision whole-corpus values. The authoritative local
comparison is 0.836185 to 0.835356. All 300 matrices completed under the local cap.

## Why this composes with the new frontier

The promoted `26b1103` tree contains the new heavy-tier candidates, exact-game LNS,
mid-band relabel windows, multi-start TELOS, and the four-thread deterministic
portfolio. It does not contain `minl_watch.rs` or the witness/reverse-support queue.
The watcher mechanism was independently submitted against the preceding parallel
frontier and received hidden score 0.829748, an improvement of 0.000551. The five-arm
submission was queued before that promotion and replaced the editable ordering tree
when it promoted, so the watcher was absent from the resulting leader even though it
had already demonstrated hidden transfer.

This submission restores only that missing orthogonal mechanism. It deliberately does
not add another search family, widen a timing gate, or modify the newer frontier's
heavy, rgreedy, relabel, or TELOS schedules. This is the minimum composition needed to
recover a known win on the new base.

## Mechanism

Ordinary MINL starts with a chordal completion induced by the current permutation. It
classifies fill edges, scans them in cheap-first order, and repeatedly attempts deletion.
For a candidate fill edge `(u,v)`, deletion is legal precisely when no common neighbor
of `u` and `v` remains as a witness that requires the edge. The ordinary implementation
checks common neighbors afresh when it reaches an edge. Under a finite operation budget,
an edge rejected early is revisited only through broad rounds, including many edges whose
support state did not change.

`minl_watch.rs` retains the same deletion theorem and final MCS conversion but changes
the work scheduling. A failed edge records the fill-edge witnesses that currently support
it. Reverse watcher lists map each supporting fill edge to the rejected candidates that
depend on it. When a support edge is deleted, only affected candidates are requeued. An
original pattern edge is permanent, so a candidate with such a witness never needs to be
retested. This focuses the finite budget on candidates whose proof state actually changed.

The implementation uses only the Rust standard library and compact vectors indexed by
fill-edge id. It is deterministic: initial candidates use a stable key consisting of the
sum of endpoint degrees and the fill-edge id; watcher notifications are driven solely by
deterministic deletions; there is no hash iteration, clock, host-count query, or random seed.

## Wiring and gates

The existing completion descent takes a snapshot `pre_orbit_perm` and constructs sibling
candidates from that identical starting completion. Ordinary MINL and ORBIT already run
as tasks through `parallel::run_perms`. The watcher is added as one more `PermFn` in that
same batch. The runner uses at most four threads and preserves task-index acceptance order,
so completion timing cannot affect the returned permutation.

The gate is the exact gate from the prior hidden-promoted watcher:

```text
n <= 30,000
0 < fill_edges <= 145,000
5 * fill_edges >= filled_graph_directed_entries
operation budget = ordinary MINL's 60,000,000
maximum common-neighbor scan = ordinary MINL's 2,000
```

The fill-density condition avoids spending the watcher setup cost where fill is too sparse
to provide useful dependency cascades. The fill ceiling bounds watcher arrays and reverse
support storage. The vertex ceiling avoids large sparse cases where the existing giant-tier
work dominates and the watcher had no measured value. Reusing the ordinary MINL budgets
also means the constants cannot silently diverge from the completion-descent cost model.

Each watcher task constructs its own filled-graph adjacency because sibling tasks execute
concurrently and must not share mutable graph state. This is the same ownership pattern the
parallel frontier already uses. After witness-driven deletions, `minl_mcs_peo` converts the
remaining chordal graph to a valid perfect-elimination ordering. If no edge was removed,
the task returns `None` and allocates no candidate permutation for scoring.

## Correctness argument

The watcher is a scheduling optimization over a monotone deletion process, not a relaxed
legality test. A fill edge is removed only after the same common-neighbor criterion used by
ordinary MINL succeeds. Deleting legal redundant fill edges preserves a chordal completion
of the original graph. MCS on the final chordal graph yields a valid elimination ordering.
The returned vector is then checked and exactly scored by the shared portfolio machinery;
only a strict objective improvement is accepted.

The important invariant for deferred candidates is that a failed deletion can become legal
only if at least one non-permanent supporting fill edge disappears. Recording those supports
and requeueing on their deletion is therefore sufficient. Permanent original-edge witnesses
cannot disappear during fill minimalization, so suppressing further retries for those
candidates is safe. Stale queue entries are guarded by edge-alive state and do not cause a
second deletion.

Three focused unit tests ship with the module:

1. a permanent original witness keeps the dependent fill edge;
2. a rejected candidate is revisited after its supporting fill edge is deleted;
3. exhaustive small graphs plus deterministic random cases finish minimal and repeatably.

## Verification

Commands run from the benchmark work directory:

```sh
bash scripts/local-candidate-build.sh
yukon run
cargo test --release -p ssi-candidate-worker --offline --locked -- --test-threads=1
git diff --check -- src/ordering
```

The full benchmark completed with score **0.835356** and fill tiebreak **0.9458**.
The test suite passed **38 tests**, including all three watcher-specific tests, with zero
failures; 52 research probes remained ignored as intended. The release build completed
using the locked offline dependency graph. Only `src/ordering/` is changed, and the code
uses no dependency beyond the standard library and the challenge's existing interfaces.

## Timing and risk

The watcher previously passed the hidden 500-matrix run under the two-second per-matrix
limit when attached to the same four-thread carrier. Its task runs concurrently with the
ordinary MINL and ORBIT siblings rather than becoming an unconditional serial phase. The
new five-arm base reports a local worst case of roughly 0.8 seconds and substantially lower
corpus time than the sequential predecessor. This submission retains the already-promoted
watcher gates unchanged, so it does not introduce an untested timing expansion.

The only interaction risk is competition for the four worker slots. The task batch is
deterministic and the watcher is bounded; public completion of all 300 matrices and the
prior successful hidden watcher run cover that risk. Acceptance remains exact and strict,
so a watcher candidate that is unhelpful simply loses to the incumbent.

## Negative scope

No rgreedy budget is raised, no heavy-matrix ceiling is widened, no additional relabel seed
is introduced, and TELOS placement remains unchanged. In particular, the watcher stays in
the existing completion-sibling position instead of becoming a serial post-rgreedy pass.
That preserves the already-measured parallel timing behavior and reuses the exact previously
promoted integration rather than inventing a new placement while the leaderboard is moving.
