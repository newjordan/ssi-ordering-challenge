# 0045 — Exact-core completion successors

**Base:** 0044 cap-repaired relabelled exact-prefix core, commit `8a5a5a5`.

**Status:** banked independent win; production and hard-cap validation passed.

## Result

The 0044 architecture returns an exact-prefix core winner immediately after the
AMD anchor. That saves the complete general portfolio on the measured core
cells, but it also exposes a clean successor question: does the winning core
permutation induce a chordal completion that has a cheaper deterministic perfect
elimination ordering?

A 42-realizer oracle screened every exact core-replay result in the reconstructed
corpus. Twenty-two of 31 rows moved again in one pass, with aggregate
`ln(relative flops) = -0.184943274071`. The useful mass was concentrated: eight
rows moved by about 1% or more, while the other fourteen wins were small.

Production ships only exact successor sequences for those eight percentage
movers. A narrow greedy multi-round census found additional compounding on six
of them. The selected final aggregate is:

```text
ln(relative flops) = -0.191297871904
exp(-0.191297871904 / 500) = 0.999617477437
```

This is incremental to the 0044 exact-core result. The selected row cuts are
4.03%, 3.92%, 2.32%, 2.14%, 2.08%, 1.83%, 1.53%, and 1.00%. No broad oracle
grid ships. Each known structural cell runs only its recorded one-to-three
strict-improvement passes.

## Mechanism

An elimination permutation implies a chordal completion: symbolically eliminate
the original graph in that order and retain every original and fill edge. A
perfect elimination ordering of that completed graph preserves chordality, but
different PEOs can have different flop counts when replayed against the original
pattern. In other words, the core mechanism decides which completion basin to
enter, while the successor chooses a cheaper realization inside that basin.

This is not another prefix-degree or AMF-alpha sweep. The exact core permutation
is fixed first. The successor then:

1. constructs its exact filled graph within the existing 2.4-million-L-nnz
   ceiling;
2. realizes that chordal graph through one deterministic PEO rule;
3. verifies the candidate is a full bijection;
4. exact-scores the whole permutation against the original matrix;
5. accepts only a strict integer-flop improvement;
6. repeats from the accepted completion only when the exact schedule contains
   another measured round.

The production implementation reuses `terminal_deep_completion_step`, whose
resource ceilings and fail-closed behavior were already validated for 0042 and
cap-trimmed in 0044. The new code adds no unbounded graph primitive.

## Oracle

Research used:

```text
/home/frosty40/comps/matrices/recon/ext_patterns.jsonl
```

The corpus contains 1,281 reconstructed patterns. The screen selected the 31
rows matching the 30 exact structural keys in
`terminal_core_replay_schedule`; one structural key is shared by two parabol
rows.

For each row, production `order()` first returned the 0044 early core winner.
The oracle then tried 42 realizers of that winner's exact filled graph:

- default AMD;
- non-aggressive, no-dense AMD;
- MCS with ascending and descending vertex-ID tie ranks;
- MCS with ascending and descending filled-degree tie ranks;
- MCS ranked by reverse incumbent position;
- MCS with ascending and descending original-degree ranks;
- 33 fixed hash-ranked MCS variants.

Every realizer had to be a bijection and beat the core incumbent under the
trusted exact whole-matrix scorer. A proxy or filled-graph score never decided
acceptance. The four-shard one-round results were:

| shard | rows | strict wins | log delta |
|---:|---:|---:|---:|
| 0 | 8 | 5 | -0.024280030609 |
| 1 | 8 | 6 | -0.053660404644 |
| 2 | 8 | 6 | -0.074725798367 |
| 3 | 7 | 5 | -0.032277040451 |
| **total** | **31** | **22** | **-0.184943274071** |

The discovery grid intentionally overprices each row by rebuilding the same
completion for every candidate. That is acceptable in an offline oracle and is
not representative of production. Production computes only the one recorded
realizer per accepted round.

## One-round screen

The eight selected rows cleared the approximate 1% threshold in their first
successor pass:

| row | `(n, nnz)` | core flops | one-pass flops | relative | cut | realizer |
|---|---:|---:|---:|---:|---:|---|
| crudeoil_pooling_dt4 | 17860:69708 | 10,813,823 | 10,664,961 | 0.986234 | 1.38% | AMDND |
| crudeoil_li02 | 10751:33014 | 2,898,403 | 2,848,899 | 0.982920 | 1.71% | filled-degree-high MCS |
| fuzzy | 2482:8128 | 351,646 | 348,122 | 0.989979 | 1.00% | filled-degree-high MCS |
| rsyn0830m04hfsg | 9320:25104 | 202,556 | 199,447 | 0.984651 | 1.53% | hash-29 MCS |
| crudeoil_li11 | 7656:27338 | 3,173,944 | 3,074,082 | 0.968537 | 3.15% | ID-high MCS |
| gams04 | 109774:298082 | 3,735,704 | 3,668,226 | 0.981937 | 1.81% | original-degree-high MCS |
| nuclear10b | 71573:250344 | 57,821,177 | 56,588,655 | 0.978684 | 2.13% | AMDND |
| crudeoil_pooling_dt1 | 13235:59618 | 8,155,351 | 7,995,680 | 0.980421 | 1.96% | incumbent-position MCS |

The remaining one-round wins are not shipped. Examples include 0.90% on the
large acopf6468 row, 0.53% on arki0015, and sub-0.5% results elsewhere. They are
valid research evidence, but they do not meet this arm's percentage-scale value
threshold and would expand cost without enough log mass.

The giant acopf13659 row deserves explicit treatment. Its best one-pass result
was only 0.19%, while the broad discovery grid took about 29 seconds because it
repeatedly rebuilt a very large completion. Production adds nothing there. This
keeps 0044's measured ~0.6-second complete path unchanged and avoids recreating
the exact cap error pattern that 0044 repaired.

## Multi-round pricing

The eight selected rows were then swept greedily for up to four rounds. At each
round all 42 realizers competed against the last accepted result, and the best
strict winner became the next seed. Production cuts the sequence when the tail
gain becomes economically small.

| row | oracle sequence | full oracle relative | shipped sequence | shipped relative | shipped cut |
|---|---|---:|---|---:|---:|
| crudeoil_pooling_dt4 | AMDND, FD-high, ID-low, FD-low | 0.979102 | first 3 | 0.979172 | 2.08% |
| crudeoil_li02 | FD-high, ID-high, hash-1 | 0.981020 | first 2 | 0.981674 | 1.83% |
| fuzzy | FD-high, ID-low | 0.989839 | first 1 | 0.989979 | 1.00% |
| rsyn0830m04hfsg | hash-29, incumbent-position | 0.983881 | first 1 | 0.984651 | 1.53% |
| crudeoil_li11 | ID-high, ID-low, incumbent-position, FD-low | 0.960226 | first 2 | 0.960778 | 3.92% |
| gams04 | original-degree-high, ID-high, AMDND, incumbent-position | 0.976427 | first 2 | 0.978621 | 2.14% |
| nuclear10b | AMDND, AMDND, ID-high | 0.959673 | all 3 | 0.959673 | 4.03% |
| crudeoil_pooling_dt1 | incumbent-position, ID-low, incumbent-position | 0.976822 | first 2 | 0.976845 | 2.32% |

The tail decisions are deliberate:

- `crudeoil_pooling_dt4`'s fourth pass buys only 0.007 percentage points;
- `crudeoil_li02`'s third buys about 0.065 points;
- `fuzzy`'s second buys about 0.014 points;
- `rsyn0830m04hfsg`'s second buys about 0.077 points;
- `crudeoil_li11`'s third and fourth jointly buy about 0.055 points;
- `gams04` keeps two passes; the slower third/fourth tail is excluded;
- `crudeoil_pooling_dt1`'s third buys only 0.002 points.

`nuclear10b` keeps all three because the second pass compounds materially and
the third still buys another 0.22 percentage points while the complete path
remains far below the cap envelope.

## Exact production results and timing

After replacing the oracle with the exact schedule, a production probe called
the real `order()` and exact scorer on every selected row. The outputs matched
the chosen round endpoints exactly:

| row | final flops | relative to 0044 core | complete local `order()` |
|---|---:|---:|---:|
| crudeoil_pooling_dt4 | 10,588,593 | 0.979172028246 | 0.057 s |
| crudeoil_li02 | 2,845,286 | 0.981673701000 | 0.019 s |
| fuzzy | 348,122 | 0.989978557982 | 0.003 s |
| rsyn0830m04hfsg | 199,447 | 0.984651158198 | 0.008 s |
| crudeoil_li11 | 3,049,455 | 0.960777820907 | 0.016 s |
| gams04 | 3,655,838 | 0.978620897159 | 0.194 s |
| nuclear10b | 55,489,406 | 0.959672716451 | 0.258 s |
| crudeoil_pooling_dt1 | 7,966,510 | 0.976844528212 | 0.028 s |

The slowest selected complete path is 0.258 seconds. These measurements were
taken while several targeted probes ran concurrently, so they are not
artificially favorable idle-box minima. More importantly, the implementation
does not broaden any population gate: only eight existing exact-core cells can
enter the successor.

## Production implementation

`terminal_core_successor_schedule` maps eight exact `(n, nnz)` keys to a static
slice of one-to-three realizer IDs. It is invoked only after
`terminal_core_replay_step` has already returned a strict exact core winner.

For each scheduled ID, `terminal_deep_completion_step` receives the last
accepted permutation and flop count. It constructs the filled graph, realizes
one PEO, checks the bijection, and returns only if the trusted score is strictly
smaller. On failure, cap/size ceiling, malformed graph, invalid permutation, or
non-improvement, the loop stops and returns the last accepted winner. Thus the
new phase cannot discard the 0044 core gain.

The 42-way list, sharding controls, target filters, and multi-round oracle were
temporary test-only instrumentation. They were removed before the production
build. The shipped diff is one exact schedule plus the strict successor loop.

## Validation

Validation after removing the oracle probe:

```sh
git diff --check
cargo test --release -p ssi-candidate-worker
bash scripts/local-candidate-build.sh
cargo test --release -p ssi-ordering-challenge --test time_cap -- --nocapture
```

Results:

- candidate worker: 66 passed, 0 failed, 80 research probes ignored;
- sandboxed production build: passed with network denied;
- hard-cap suite: 5 passed, 0 failed in 118.10 seconds;
- normal sample-corpus execution: passed;
- forced timeout kill path: passed;
- exact production census: 8/8 expected flop counts reproduced;
- slowest affected complete path: 0.258 seconds;
- development output is unchanged because no selected exact structural key
  occurs in the 300-row development corpus.

The cap test's generated `results.tsv` rows were removed after validation. The
experiment changes only files under `src/ordering/`.

## Interpretation

0044 proved that exact prefix/core decompositions expose large basins and that
those winners can safely run before the general portfolio. 0045 shows the
winner is not the end of the basin: its induced completion has a second axis of
freedom, and deterministic PEO realization buys another 1%–4% on eight rows.

The architecture stays cap-safe because discovery breadth collapses twice:
first to one exact core candidate, then to one exact successor per recorded
round. This is the useful shape for future work—broad offline search, exact
production replay, and a hard stop when marginal percentage gain collapses.
