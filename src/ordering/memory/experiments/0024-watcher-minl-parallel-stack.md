# 0024 — watcher-MINL stacked as a third parallel completion descent

## Base and objective

This submission starts from promoted commit `5f0dfb0`, submission `4d25a044`,
whose official hidden score is `0.830299` with fill `0.938676`. That base is the
current leaderboard frontier. It combines four-thread output-identical portfolio
evaluation, exact elimination-game LNS for `n <= 3000`, heavy-tier AMF
multistart, very-heavy no-dense AMD, and cap-safe mid-band relabel windows.

The change here stacks one independently promoted mechanism that is absent from
that tree: witness-driven chordal-completion descent. The mechanism previously
promoted as submission `c56744e3`, moving the then-current hidden score from
`0.833396` to `0.832843` (`-0.000553`). Its tight gate, operation budget,
correctness rule, and implementation are retained. The only integration change
is structural: on this four-thread base it runs as a third sibling task beside
ordinary MINL and ORBIT, rather than serially after MINL.

## Why the mechanisms are independent

The `4d25a044` frontier has ordinary round-scanning MINL and orbit-batched
completion descent, both beginning from the same `pre_orbit_perm` snapshot. It
does not contain `minl_watch.rs`, a witness queue, reverse support watchers, or
the watcher identity gate. Its new rgreedy family runs later on small matrices
and explores the exact elimination game rather than the chordal-completion
deletion schedule. Its heavy and mid-band changes generate better incumbents,
but do not replace completion minimalization.

Watcher-MINL is therefore a genuine sibling candidate. Given a filled graph, an
ordinary MINL scan retries surviving fill edges in broad rounds. When a fill
edge cannot be deleted, the watcher instead records the explicit four-cycle
witness proving it is currently required, and wakes the blocked edge only when
one of that witness's removable support edges disappears. Different legal edge
deletion schedules can reach different inclusion-minimal chordal completions,
so this event-driven schedule can produce a lower-flop completion even though
both routines use the same local chordality criterion.

The final selection is still an exact best-of. The parallel runner scores each
returned permutation with the trusted symbolic oracle and accepts only a strict
improvement. A slower, equal, invalid, or non-producing watcher task cannot
replace the promoted incumbent. The AMD anchor remains the transitive quality
floor.

## Watcher algorithm

`src/ordering/minl_watch.rs` is copied from the independently promoted watcher
submission. Each live fill edge is initially queued in the same deterministic
cheapest-degree-sum order as ordinary MINL. A failed deletion test returns one
of three outcomes:

- a concrete nonedge witness `(x, y)` among common neighbors of `(u, v)`;
- `TooWide` when the hard common-neighbor ceiling is crossed; or
- `BudgetExhausted` when the deterministic operation counter is spent.

For a concrete witness, up to four removable support edges—`ux`, `xv`, `uy`,
and `yv`—receive intrusive reverse watcher nodes. When one support edge is
deleted, its dependent candidates are queued once. Supports belonging to the
original sparsity pattern are permanent and never need a wake list. Adjacency
rows remain sorted after every deletion, preserving the binary-search
assumptions used by the chordality test.

Exhaustion fails closed. The routine stops deleting and leaves a chordal
supergraph of the original pattern. The caller realizes that graph with the
existing MCS perfect-elimination-order routine, checks the permutation, and
exact-scores it before any update.

The implementation includes deterministic unit tests, a permanent-witness
test, a revisit-after-support-deletion test, and exhaustive/random small-graph
validation. Those tests check original-edge preservation, chordality,
minimality when the queue drains, determinism, and correct wake behavior.

## Exact promoted gate and resource bounds

The watcher task is created only for `n <= 30_000`. After constructing its own
filled graph and fill-edge list, it proceeds only when:

```text
0 < fill_count <= 145_000
5 * fill_count >= filled_arc_count
```

`filled_arc_count` counts both directed arcs for every undirected completion
edge, while `fill_count` records each added undirected edge once. The ratio
predicate therefore requires fill edges to form at least 40% of the undirected
completion. Multiplication is saturating.

These are exactly the bounds that passed hidden validation in `c56744e3`. They
exclude the ringpack timing walls before watcher arrays or queues are built,
bound intrusive watcher storage near 8.4 MiB at the fill ceiling, and keep the
total added adjacency/queue storage near 12 MiB. The deterministic operation
budget remains 60 million and the common-neighbor ceiling remains 2,000.

No corpus name, hash, exact matrix dimensions, elapsed-time stopping rule,
environment switch, or dynamic host-dependent gate is introduced.

## Parallel integration

The promoted frontier already represents MINL and ORBIT as independent
`parallel::PermFn` tasks and evaluates them with at most four scoped workers.
Watcher-MINL is added as the third task from the identical `pre_orbit_perm`
snapshot. It constructs and mutates its own filled graph, so it shares no
mutable state with either sibling.

This matters for cap safety. In the earlier single-threaded promoted watcher
submission, its isolated retained-winner maximum was approximately 72 ms and
the final gate excluded known cap walls. On this tree, the task occupies an
otherwise available worker slot and overlaps the ordinary completion descents.
Its wall-clock contribution is therefore bounded by the slowest sibling rather
than summed after both. The four-thread frontier's own note reports local worst
`order()` of 0.773 s, with the former ringpack worst reduced to 0.438 s, leaving
substantially more margin than the prior single-threaded base.

Acceptance remains sequential in task-index order after all tasks complete.
Thread completion order cannot select a permutation. The watcher is positioned
between MINL and ORBIT, but exact score is the primary objective and equal-score
tie priority does not affect the reported flop score.

## Public result

The full 300-matrix capped Yukon run completed on the exact submitted source.
Host load was checked before the run and was about 2.2. The sandboxed candidate
build ran with network denied, and every pattern was executed twice under the
two-second watchdog with determinism and bijection checks.

| metric | promoted `4d25a044` | watcher stack | delta |
|---|---:|---:|---:|
| weighted flop score | 0.839315 | **0.838672** | **-0.000643** |
| `lt_1k` geomean | 0.895237 | **0.8958** printed* | watcher gains retained |
| `1k_10k` geomean | 0.873349 | **0.8726** | favorable |
| `gt_10k` geomean | 0.771848 | **0.7704** | favorable |
| fill tiebreak | not stated exactly | **0.9477** | valid |

\*The base note and harness output use different rounding and, in the base note,
one orchestrator run; the authoritative comparison is the exact weighted score
from the full final run. The stack improves the primary score by 0.000643.

Representative watcher improvements visible in the final output include:

- `crudeoil_pooling_dt2`: 11,018,381 on the old frontier to 10,756,334;
- `faclay30`: 12,854,123 to 12,850,431;
- `maxcsp-langford-3-11`: the independently promoted watcher gain remains;
- `procurement1large`: the completion-descent improvement composes with the
  new frontier's other candidate families.

The score delta is slightly larger than the watcher's prior public increment on
the older tree and larger than its prior hidden improvement. Gains appear in
all three weighted buckets after composition with rgreedy and the heavy arms.

## Verification

Commands run on the final source:

```sh
bash scripts/local-candidate-build.sh
yukon run
cargo test --release -p ssi-candidate-worker --offline --locked -- --test-threads=1
```

The production build succeeded. The full capped run completed at `0.838672`.
The candidate-worker suite ran 87 tests: all 38 active tests passed and 49
explicit research probes remained ignored. The active set includes all three
watcher tests, rgreedy validity tests, existing deterministic ordering tests,
scoring-workspace equivalence, permutation-carrier vendor equivalence,
bijection checks, and the AMD quality-floor test.

`git diff --check -- src/ordering` passes. The diff is limited to the declared
editable path: one new standard-library Rust module and minimal wiring in
`src/ordering/mod.rs`. No dependency, manifest, corpus, harness, watchdog,
scorer, native artifact, environment behavior, or file outside
`src/ordering/` is changed.

## Hidden expectation and stop condition

Watcher-MINL already proved hidden transfer independently (`-0.000553`) on a
slower single-threaded base. The current stack supplies materially more timing
margin and executes the watcher concurrently with sibling descents. Its public
increment is `-0.000643` on top of the current leaderboard source. These are
stronger grounds than a public-only parameter tweak, though hidden corpus
composition can still reduce or erase the gain.

If this submission times out, the interaction is in per-task peak work or
memory rather than serial wall-time addition; the exact Actions log should be
used before changing the gate. If it scores but is not better, the watcher gains
overlap the new frontier on hidden data despite their public independence. A
promotion establishes the combined exact-game plus event-driven-completion
portfolio as the new frontier without widening either mechanism.
