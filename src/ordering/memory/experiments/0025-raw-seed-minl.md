# 0025 — Raw seed-pool completion descent

Date: 2026-08-29  
Discovery source: `bac2f03e9ad55ba2f91e0ce84462a99e5efbca10`
Restack source: `c7d5a8b3199fcacb01eaad0526b8f4a67831d97b`
Status: strict exact-source win on both carriers; no submission was made from
this worktree.

## Question

Experiment 0022 established that downstream search is seed-sensitive. The
portfolio retains the best eight distinct candidate permutations in
`seed_pool`; TELOS already descends several of them. The current carrier also
keeps the best distinct *post-TELOS* runner-up and runs the completion-descent
trio from that alternate completion. That still leaves one structurally
different object unused: a good raw portfolio permutation before TELOS changes
its basin.

The question here was deliberately narrow:

> Does ordinary MINL find a new completion when started from one distinct raw
> `seed_pool` permutation, even after the incumbent and post-TELOS alternate
> completion have already been descended?

This is not another global heuristic and it does not replace any incumbent
path. It is one more candidate in the existing strict-best merge.

## Where the structure was blocked

The completion-descent stage had two kinds of starts:

1. `pre_orbit_perm`, the final incumbent after TELOS;
2. `desc_alt`, the best distinct runner-up produced by the extra TELOS
   descents.

Both starts have already passed through TELOS. A raw seed that is worse before
TELOS can encode a different chordal completion, but once TELOS maps the pool
to its local endpoints the raw basin is no longer represented. This is the
same information-loss pattern diagnosed in 0020, one layer farther
downstream: keeping only the best transformed endpoint is not equivalent to
keeping structurally distinct inputs for a seed-sensitive descent.

The missing edge was therefore:

```text
portfolio top-K raw seed
        |
        +---- TELOS ---- existing alternate completion trio
        |
        +---- MINL  ---- previously absent candidate
```

The implementation selects the first pool entry distinct from both
`pre_orbit_perm` and `desc_alt`, clones it, and sends only that start through
ordinary MINL. The watcher and ORBIT siblings stay disabled for the new start.
This keeps the experiment attributable and bounds added work to one familiar
completion descent.

## Correctness trap found during implementation

The first patch represented descent starts as `(perm, run_trio)`. An initial
placement of the `run_trio` guard accidentally wrapped ordinary MINL as well as
the watcher, so the raw start produced an empty task batch. Compilation alone
did not expose the mistake because an empty batch is valid. The fix moved the
guard below the ordinary MINL task: MINL now runs for every start, while
watcher and ORBIT run only for the two established trio starts.

This is a useful orchestration rule for future candidate-batch work: a valid
zero-task control flow can silently turn an experiment into a no-op. Targeted
row movement must be confirmed before spending a full-corpus run.

## Broad targeted probe

The first real A/B used the exact `bac2f03` tree and ran only matrices with
`0 < nnz <= 20,000`. A test-only `Option<bool>` override enabled exact OFF/ON
measurement in the same process; production reads only the deterministic
structural gate.

Results over 221 rows:

- better: **3**
- worse: **0**
- same: **218**
- mid-bucket summed log delta: **-0.016060548**

Exact movers:

| matrix | n | nnz | baseline flops | candidate flops | ratio before | ratio after |
|---|---:|---:|---:|---:|---:|---:|
| `syn40hfsg` | 1,022 | 2,624 | 8,176 | 8,155 | 0.969639 | 0.967149 |
| `oil` | 3,270 | 8,430 | 35,783 | 35,585 | 0.985405 | 0.979952 |
| `rsyn0840m02m` | 3,486 | 9,576 | 45,773 | 45,411 | 0.963054 | 0.955438 |

All three movers occupy a compact mean-degree band:

- `2.5 <= nnz/n <= 2.8`;
- `1,000 < n <= 3,500`;
- `nnz <= 10,000`.

The broad `nnz <= 20k` probe had an ON worst of 1.3706 s on
`crudeoil_lee1_07`, an unchanged row. The score evidence was real, but paying
the extra completion build over the entire broad band was unnecessary cap
risk.

## Gate descent

The first cap-first structural gate was:

```text
1000 < n <= 3500
2.5 <= nnz/n <= 2.8
nnz <= 10000
```

It admitted 17 visible rows: 3 better, 0 worse, and 14 identical. In the
targeted run its worst ON time was 1.1007 s. A first full-corpus ON run still
showed a noisy 1.2017 s on the unchanged `squfl010-080`; the score improved,
but this violated the 1.15 s affected-window criterion used for this sequence.

The final gate retains the same density envelope but pays only at the two ends
where the actual movers occur:

```text
1000 < n <= 3500
(n <= 1100 OR n >= 3200)
2.5 <= nnz/n <= 2.8
nnz <= 10000
```

Integer arithmetic in production is overflow-safe:

```rust
2 * nnz >= 5 * n
5 * nnz <= 14 * n
```

This admits four visible rows: the three strict movers plus the unchanged
`syn10m03hfsg`. In the final full-300 run the four times were:

| matrix | seconds | result |
|---|---:|---|
| `rsyn0840m02m` | 1.1255 | strict improvement |
| `syn40hfsg` | 1.0319 | strict improvement |
| `syn10m03hfsg` | 0.9861 | identical |
| `oil` | 0.6020 | strict improvement |

The affected-window worst is therefore **1.1255 s**, under 1.15 s even in a
heavily contended run. The same run's global worst was 2.335 s on an unrelated,
out-of-gate row, and is not an incremental-cost signal: the exact OFF run had
already shown large host variance, while every gate row is listed above.

## Full-corpus score

Exact same-tree full-300 measurements on `bac2f03`:

| configuration | lt_1k | 1k_10k | gt_10k | weighted score |
|---|---:|---:|---:|---:|
| raw-seed MINL OFF | 0.8924 | 0.8620 | 0.7322 | **0.819206** |
| final narrow gate ON | 0.8924 | 0.8619 | 0.7322 | **0.819168** |

Delta: **-0.000038**. The rounded bucket display hides most of the movement,
so acceptance uses the six-decimal aggregate and the exact per-row flops
above. Because the extra result enters through the existing exact scorer and
strict `<` acceptance, the arm cannot worsen a matrix's objective.

## Determinism and cap safety

- Seed selection is deterministic: `seed_pool` is ordered by exact flops and
  deterministic arrival order, and exact duplicate permutations are rejected.
- The gate is a pure function of `(n, nnz)` and uses saturating integer
  arithmetic.
- The raw seed is excluded if it equals either established completion start.
- Only ordinary MINL is added. Watcher and ORBIT are not duplicated for this
  start.
- MINL retains its existing filled-graph limit, operation budget, fail-closed
  deletion checks, bijection validation, exact rescoring, and strict-best
  merge.
- Outside the four-row visible structural cell, production output and work are
  byte-identical to the carrier.

## What can rise next

This result confirms that the top-K nursery is valuable beyond TELOS: raw
portfolio diversity survives as useful completion diversity. The next safe
extensions are not “add every seed everywhere.” They are:

1. restack this exact small delta onto the current official source and repeat
   OFF/ON, because a new carrier may already consume one of the three basins;
2. attribute which raw pool slot produced each win, then test whether a single
   stable slot is enough;
3. look for another compact structural cell with a strict mover before widening
   any boundary;
4. consider a cheaper completion pilot only if it predicts whether MINL removes
   at least one edge, so zero-removal rows can avoid the full scan.

The rejected direction is broad downstream duplication. The 0022 hidden cap
failure and this experiment's noisy unchanged rows both say the same thing:
diversity has value, but the unit of shipping is a measured structural cell,
not an appealing global mechanism.

## Exact c7 restack

While this experiment was finishing, the official frontier advanced from
`bac2f03` (official 0.821046) to
`c7d5a8b3199fcacb01eaad0526b8f4a67831d97b` (official 0.820722). The c7 change
adds ranked large-matrix RGSUB breadth and returns PEP whole-matrix rgreedy to
the default seed policy. The latter overlaps the four-row size class and was
therefore a real interaction risk, even though the RGSUB work is disjoint.

Exact c7 targeted A/B preserved the result bit-for-bit: 3 better, 0 worse, 1
same, the same three exact flop deltas, summed mid-bucket log delta
`-0.016060548`, and worst ON 0.7744 s. Exact c7 full-300 then measured:

| configuration | lt_1k | 1k_10k | gt_10k | weighted score |
|---|---:|---:|---:|---:|
| raw-seed MINL OFF | 0.8925 | 0.8625 | 0.7296 | **0.818335** |
| final narrow gate ON | 0.8925 | 0.8624 | 0.7296 | **0.818297** |

The c7 restack delta is again **-0.000038**. In the ON full run the four gate
times were 0.6542 s (`syn10m03hfsg`), 0.5382 s (`syn40hfsg`), 0.7818 s (`oil`),
and 0.7127 s (`rsyn0840m02m`), so affected worst was **0.7818 s**. The global
1.919 s worst was an unrelated out-of-gate host excursion on `slay05h`; it is
not caused by this arm.
