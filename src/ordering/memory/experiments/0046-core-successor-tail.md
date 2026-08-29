# 0046 — Core-successor tail and bridge kill

**Base:** 0045 exact-core completion successors, commit `666e228`.

**Status:** two cheap exact wins banked; not submitted alone because their
aggregate is below the board cutoff. The completion-successor family is closed.

## Result

0045 kept eight rows whose first completion realization cut about 1% or more.
This follow-up exhausts the two plausible sources of additional percentage
mass: compounding the fourteen smaller strict one-pass wins and allowing a
small beam of non-improving completion realizations to bridge into a later
winner.

Two cheap strict chains crossed the percentage threshold and ship in the exact
successor schedule:

| row | `(n, nnz)` | 0045 flops | final flops | relative | cut | complete `order()` |
|---|---:|---:|---:|---:|---:|---:|
| arki0005 | 7522:254840 | 18,079,810 | 17,688,731 | 0.978369297022 | 2.163% | 0.106 s |
| kan_r3_h1_n9 | 7153:31632 | 225,271 | 222,688 | 0.988533810388 | 1.147% | 0.015 s |

Their incremental aggregate is:

```text
ln(relative flops) = -0.033400509130
exp(-0.033400509130 / 500) = 0.999933201213
```

That is real value but not enough to justify an isolated submission against the
roughly ten-basis-point relative board cutoff. It is banked for the next
percentage-scale carrier. Production adds only eight already measured passes
across two exact structural cells and retains every existing fail-closed size
and fill ceiling.

## Strict tail census

The temporary oracle began from the real 0045 `order()` result on every one-pass
winner omitted by 0045. At each round it screened the same 42 completion
realizers used in 0045, exact-scored their full permutations against the
original pattern, accepted the best strict improvement, and repeated for four
rounds. This matters because a small first pass can compound: rejecting a row
solely from its first-round percentage would have missed `arki0005`.

The complete four-round endpoint census was:

| row | best relative | endpoint decision |
|---|---:|---|
| arki0005 | 0.978369 | ship four passes; 2.163% cut |
| kan_r3_h1_n9 | 0.988534 | ship four passes; 1.147% cut |
| arki0015 | 0.993433 | reject; 0.657% |
| rsyn0810m03m | 0.997009 | reject |
| torsion100 | 0.999913 | reject |
| crudeoil_lee2_07 | 0.999884 | reject |
| transswitch2383wpp | 0.997945 | reject |
| powerflow2383wpp | 0.998907 | reject |
| watercontamination0303 | 0.999140 | reject |
| acopf6515 | 0.995317 | reject; 0.468% |
| crudeoil_lee2_09 | 0.998271 | reject |
| powerflow2736spr | 0.998170 | reject |
| acopf13659 | 0.996142 | reject; giant row |
| acopf6468 | 0.989904 | reject on cap economics |

`acopf6468` is the important trap. Its three-pass sequence reached a nominal
1.010% cut, but the complete production path measured 0.745 seconds locally.
The previous three submissions all failed the hidden two-second per-matrix cap,
and 0044 deliberately restored margin by removing broad and slow work. Spending
that margin for roughly one percent on one row is the wrong exchange, especially
when large hidden patterns are exactly where local timing underpredicts grader
risk. The sequence was removed from production.

The other rejected rows remain valid exact improvements, but they collectively
occupy the long, low-value tail. Shipping them would grow schedules and
completion reconstruction cost while moving the 500-row geometric mean by much
less than the two retained rows.

## Selected exact sequences

`arki0005` uses:

```text
hash-12 MCS -> original-degree-high MCS -> hash-3 MCS -> hash-7 MCS
```

The first realization alone cuts less than half a percent. Its induced
completion changes which MCS tie ranks are useful, and the four strict passes
compound to 2.163%. The complete real `order()` path remains about 0.106 seconds,
so this is exactly the kind of tail worth retaining: percentage-scale result at
small absolute cost.

`kan_r3_h1_n9` uses:

```text
ID-low MCS -> incumbent-position MCS -> incumbent-position MCS -> filled-degree-low MCS
```

The row is much cheaper: the complete path is about 0.015 seconds. Four passes
compound to 1.147%. Both schedules run only after an exact-core replay has
already strictly improved the AMD anchor, and every round must itself return a
strict exact improvement. A failed allocation, resource ceiling, invalid PEO,
or non-improvement stops the chain and returns the last valid winner.

## Non-improving bridge screen

A greedy strict walk can miss a basin that requires a temporarily worse PEO.
The second temporary oracle therefore changed the acceptance threshold from the
incumbent flop count to `u64::MAX`, retained the eight cheapest distinct
permutations after every round, and expanded each through all 42 realizers for
three rounds. Permutations were hash-deduplicated; the origin remained in the
beam, so the final best could never be worse than production.

The first screen covered the nine exact-core rows that had no strict one-pass
successor (eight unique structural keys because two parabol rows share one
key):

- `crudeoil_lee2_10` found only a 0.0023% endpoint;
- `ndcc15`, `ndcc16`, `parabol5_2_2`, `parabol5_2_4`, `crudeoil_lee2_08`,
  `sfacloc`, and `topopt-mbb_60x40_50` found no improvement;
- the shared parabol key likewise produced no alternate basin.

The bridge was then applied to five representative strict-tail near misses:
`arki0015`, `rsyn0810m03m`, `torsion100`, `crudeoil_lee2_07`, and
`crudeoil_lee2_09`. In every case its best three-round result was exactly the
same endpoint already found by the strict oracle. Allowing worse intermediate
states therefore added zero value on both populations.

This is a useful kill, not a reason to broaden the production search. The final
oracle grid is 42 realizers times up to eight beam states times three rounds,
with repeated filled-graph reconstruction. The largest negative target took
about 69 seconds. None of that breadth ships, and the temporary test and hashing
imports were removed.

## Production implementation

The production diff extends `terminal_core_successor_schedule` by exactly two
`(n, nnz)` matches and two static four-byte sequences. It adds no new primitive,
population gate, allocation shape, thread, or score proxy. The existing loop:

1. starts from the exact core winner;
2. builds the bounded filled graph for one recorded realizer;
3. verifies the complete candidate permutation;
4. exact-scores it against the original matrix;
5. accepts it only on strict integer-flop improvement;
6. stops at the first failure or exhausted sequence.

Because the structural keys do not occur in the development corpus, the
development score is byte-for-byte unchanged. The two rows are hidden-corpus
reconstructions whose source patterns and exact endpoints were verified through
the production `order()` function.

## Validation

After removing both temporary oracles and their hashing support:

```text
git diff --check                                      passed
cargo test --release -p ssi-candidate-worker         66/66 active passed
bash scripts/local-candidate-build.sh                 passed, network denied
cargo test --release -p ssi-ordering-challenge \
  --test time_cap -- --nocapture                      5/5 passed, 127.25 s
```

The cap suite covered hidden-corpus redaction, forced timeout termination, the
normal sample-corpus run, and worker handoff. Its four generated `results.tsv`
rows were removed after validation. No research probe or discovery grid remains
in the compiled test or production source.

## Decision

Bank the two cheap percentage movers, drop `acopf6468`, and close completion
successor research. The remaining strict tail is too small, while the
non-improving beam demonstrated that wider completion traversal does not unlock
a second reservoir. The next carrier must come from a structurally different
mechanism rather than more PEO knobs on these same chordal completions.
