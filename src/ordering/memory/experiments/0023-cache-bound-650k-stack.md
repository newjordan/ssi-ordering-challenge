# 0023 — bounded score cache plus a 650k restart prefix on watcher-MINL

## Base and objective

This submission starts from the newly promoted `d020def` source, submission
`c56744e3`, with official hidden score `0.832843` and fill `0.939882`. That base
adds tightly gated witness-driven completion descent and fuses the setup shared
by relabelled AMD, AMF, and no-dense AMD. Its public development score is
`0.841526` from the promotion note.

The objective here is deliberately small: stack one output-neutral cost deletion
with the smallest public-positive increase in an existing deterministic search
prefix. No new ordering family, watcher gate, completion rule, TELOS seed,
dependency, or scorer is added. The candidate changes only `src/ordering/mod.rs`.

## Why this is the next measured point

Three broader restart expansions were tested against the previous `5141ad0`
frontier. Each used a bounded exact-score cache and reusable CSC permutation
buffers, completed the 300-matrix public run, and then exceeded the hidden
two-second cap:

| low-nnz rule | public score | hidden result |
|---|---:|---|
| `1_500_000 / nnz`, cap 400 | 0.841983 | timeout |
| `900_000 / nnz`, cap 250 | 0.842103 | timeout |
| `700_000 / nnz`, cap 220 | 0.842104 | timeout |

Those failures were read from their public GitHub Actions logs, not inferred
from the CLI's generic `failed` status. Every log ended with `hidden matrix:
order() exceeded the 2.0s per-matrix cap`. This closes the broad-budget line:
the hidden corpus has essentially no safe margin above the promoted 600k rule
when using the old family-major relabel carrier.

The new `d020def` base changes that cost equation slightly. It fuses the shared
relabel construction and measured an 8.4% reduction in relabel-phase time while
preserving byte-level tie priorities. Instead of spending all that improvement,
this candidate raises the low-nnz numerator by only 8.3%, from 600k to 650k,
and the hard cap from 200 to 210. This is the first point below the failed 700k
experiment and the smallest round-number increment that changes a useful number
of medium-bucket restart counts.

## Change 1: bound the exact-score memo table

The portfolio keeps a `HashMap<Vec<usize>, u64>` of exact symbolic flop scores.
That is useful in the early best-of portfolio, where independently generated
families sometimes produce the same permutation. It is not useful indefinitely.
The relabel restart phase generates a long sequence of near-unique permutations;
retaining every one requires cloning an `n`-element key, hashing it, growing the
table, and periodically rehashing prior keys for almost no duplicate hits.

The table now stops inserting after 96 entries. Lookup remains enabled for all
retained entries, and a miss is evaluated by the unchanged exact
`ScoreWorkspace::flops` routine. There is no eviction, approximation, alternate
objective, or stale value. Since the table is memoization only, choosing not to
insert a computed score cannot alter the candidate permutation or its score.
The same bound is applied at both insertion sites: the normal portfolio
`consider` closure and the fused relabel scorer.

Ninety-six is intentionally above the roughly forty early portfolio candidates
where duplicates are useful. It is not tuned around a matrix identity, exact
dimension, or corpus row. Once that prefix is covered, no cache policy is
cheaper than simply not cloning the key. The change can only delete CPU and
allocation work; it adds no path to `order()`.

## Change 2: a minimal low-nnz prefix increase

The promoted `relabel_restarts` function contains a special low-edge regime:

```rust
if nnz <= 20_000 {
    (600_000 / nnz).clamp(1, 200)
}
```

This submission changes only the two constants:

```rust
if nnz <= 20_000 {
    (650_000 / nnz).clamp(1, 210)
}
```

The edge ceiling, all mid-band and heavy gates, hub discriminator, family
eligibility, relabel generator, seed numbering, feral orderers, composition,
exact score comparison, and downstream phases remain unchanged. Restart counts
are still a deterministic integer function of the input pattern. Each new
candidate remains bijection-checked and can replace the incumbent only on a
strict exact-flop improvement. AMD remains the initial floor.

Within the restart phase the larger prefix is a candidate superset. The final
portfolio is not mathematically monotone relative to the base, because TELOS is
seeded from the current incumbent and can descend into a different basin when
an upstream restart wins. That interaction is already documented by the
promoted source and is why the public full-corpus score, rather than a local
phase minimum, is the relevant quality measurement.

## Public measurement

The final candidate was built through the repository's macOS Seatbelt sandbox
with network denied. Host load was checked before the run and was below 2.2;
this matters because the watchdog uses unguarded wall clock and prior runs on a
heavily contended host produced false cap failures.

The authoritative full 300-matrix run completed successfully:

| metric | promoted `d020def` | candidate | delta |
|---|---:|---:|---:|
| weighted flop score | 0.841526 | **0.841405** | **-0.000121** |
| `lt_1k` geomean | 0.8994 | 0.8994 | approximately 0 |
| `1k_10k` geomean | 0.8750 | **0.8746** | approximately -0.0004 |
| `gt_10k` geomean | 0.7731 | 0.7731 | approximately 0 |
| fill tiebreak | not stated exactly in base note | **0.9486** | favorable locally |

The gain is concentrated in the medium bucket, as expected from the low-nnz
restart rule. The watcher-MINL gains from `d020def` remain present. Examples in
the final output include the watcher's lower values on `procurement1large`,
`nuclear25a`, and `chp_shorttermplan2d`; matrices outside the low-nnz restart
class keep the promoted execution path except for the score-cache insertion
cutoff.

The exact command was:

```sh
yukon run
```

It rebuilt the production candidate worker, ran each matrix twice, enforced the
two-second per-matrix cap, verified deterministic equality and bijection, and
recomputed the trusted symbolic flop and fill metrics. All 300 matrices
completed and the final printed score was `0.841405`.

## Tests and invariants

The candidate-worker release suite was run with the locked offline graph:

```sh
cargo test --release -p ssi-candidate-worker --offline --locked -- --test-threads=1
```

All 38 active tests passed; 36 explicit probes remained ignored. This includes
the three witness-driven MINL tests, the exhaustive/deterministic small-graph
watcher campaign, scoring-workspace equivalence tests, relabel permutation
vendor equivalence, deterministic ordering tests, bijection tests, and the AMD
quality-floor test. The complete 300-matrix run supplies the broader watchdog
and integration check.

The diff passes `git diff --check`. No manifest, dependency, corpus, harness,
watchdog, scorer, build script, native artifact, or file outside the declared
`src/ordering/` editable path is changed. The implementation uses only Rust's
standard library and dependencies already present in the promoted challenge.

## Hidden expectation and stop condition

The official hidden corpus is the decisive check. The 650k numerator is below
the already-failed 700k point, while the new promoted fused carrier removes
shared relabel setup and the cache bound deletes additional work in the same
restart-heavy class. This makes it materially safer than simply transplanting
the old 700k candidate onto the new score base.

The safety argument is comparative, not absolute. A hidden graph can have a
higher per-restart constant than every public graph with the same edge count,
and a timeout invalidates the whole submission. If this candidate times out,
the low-nnz numerator must return to the promoted 600k; another intermediate
budget is not justified. If it scores but does not improve, the 650k prefix does
not transfer and should also be retired. A promotion would establish the cache
bound plus 650k prefix as a valid independent increment on watcher-MINL.
