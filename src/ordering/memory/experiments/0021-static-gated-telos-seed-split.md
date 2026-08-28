# 0021 — Static-gated TELOS seed split

## Context

This experiment starts from promoted submission `f6e0c2d` at repository commit
`5141ad0`. The untouched checkout reproduced the development score exactly:

| metric | promoted baseline |
|---|---:|
| weighted flop score | 0.842223 |
| weighted fill tiebreak | 0.948942 |
| `lt_1k` flop geomean | 0.899432 |
| `1k_10k` flop geomean | 0.875342 |
| `gt_10k` flop geomean | 0.774477 |

The official hidden-evaluation frontier corresponding to that checkout was
`0.833396`. Development and hidden scores use different corpora, so the two
absolute values are not compared directly; the development corpus is used only
for paired before/after evidence.

The baseline is a best-of portfolio anchored on feral AMD. Candidate families
include AMD and AMF variants, partitioner orderings, relabelled multi-starts,
strip/peel transforms, TELOS gradient descent, and completion-lattice passes.
Every candidate is checked as a permutation and scored with the same symbolic
flop objective used by the grader. TELOS is a late phase: it ranks vertices by
the incumbent's exact symbolic column counts, removes or peels high-count
vertices, reorders the remainder, and keeps a strict improvement.

## Hypothesis

Experiment 0020 showed that TELOS is seed-sensitive. On
`chp_shorttermplan2d`, a worse upstream incumbent reached a better TELOS basin
than a better incumbent. A broader public experiment therefore spent one of
Tier A's two existing TELOS rounds from the AMD anchor instead of spending both
rounds from the final portfolio incumbent. That experiment measured six wins,
zero losses, and development `0.842223 -> 0.842015`, but its gate admitted every
Tier-A matrix below 150,000 nonzeros (subject only to a hub predicate), and the
candidate repeatedly failed hidden validation.

The useful part of the idea does not require that broad exposure. All six
measured development winners are small:

- `chp_partload`: `n=5,211`, `nnz=16,740`;
- `crudeoil_lee2_06`: `n=6,418`, `nnz=34,646`;
- `rsyn0840m04m`: `n=8,508`, `nnz=24,272`;
- `gasprod_sarawak16`: `n=4,596`, `nnz=15,316`;
- `rsyn0840m02m`: `n=3,486`, `nnz=9,576`;
- a sixth small Tier-A responder recorded by the preceding experiment.

The hypothesis is that a direct static bound can retain the measured score gain
while removing the broad hidden-corpus timing exposure. The bound is not tuned
to matrix names or corpus order; it uses only pattern dimensions already
available to `order()`.

## Change

Only `src/ordering/mod.rs` changes. The AMD anchor's already-computed flop count
is retained alongside `amd_perm`. In TELOS Tier A, matrices satisfying all of
the following predicates receive two independent one-round seeds:

```text
n <= 10_000
nnz <= 35_000
max_deg <= n / 50
```

The first seed is the final pre-TELOS portfolio incumbent. The second is the
already-computed AMD anchor. Each receives one round; the exact oracle keeps the
best final permutation. All other matrices keep the original behavior: one
incumbent seed receives the original `max_rounds` count. Tier B is unchanged
because `max_rounds = 1`, which cannot be split.

### Validation correction: false-path source identity

The first implementation generalized the loop for both gate outcomes: it built
a one- or two-element seed vector and routed every TELOS matrix through the new
loop. The gate-false case did no alternate-seed search, but it still allocated
the seed vector and executed new plumbing. Two byte-identical submissions of
that implementation (`ef2bb2fb` and `3910fcde`) failed hidden validation without
producing metrics.

That structure repeated the failure mode documented by the recent biconnected-
AMF experiment: a zero-work option is not a cap-safe false path if the callee or
loop has already been generalized. Hidden matrices may sit close enough to the
wall-clock ceiling that even output-neutral allocation and control-flow changes
matter.

The submitted revision branches before the alternate-seed implementation. The
false arm contains the promoted TELOS round loop verbatim and performs no seed
vector allocation, no anchor-flop reduction, and no alternate-seed bookkeeping.
Only the strict true arm computes the anchor flop count and evaluates two fixed
one-round seeds. The development score remains exactly `0.842015`, confirming
that this correction changes cost isolation rather than the measured output.

This is a depth-for-diversity exchange, not extra search. The promoted build's
Tier A worst case performs two rounds. The gated path still performs two rounds.
No new orderer, dependency, allocation family, environment switch, or runtime
configuration is introduced. The gate also excludes the promoted build's known
large timing bottlenecks, including `ringpack_30_2` (`n=17,999`, `nnz=121,458`)
and the `crudeoil_lee4_*` class.

The maximum-degree predicate is retained from the prior seed-split experiment.
It rejects hub-dominated matrices whose symbolic work can be disproportionate
to average density. The added `n` and `nnz` ceilings are the material safety
difference: the failed broad form admitted matrices with very large `n` as long
as their edge count stayed below Tier A's 150,000 threshold; this revision does
not.

## Result

The authoritative full-corpus run completed successfully and reproduced the
expected score:

| metric | baseline | candidate | delta |
|---|---:|---:|---:|
| weighted flop score | 0.842223 | **0.842015** | **-0.000208** |
| weighted fill tiebreak | 0.948942 | **0.948895** | -0.000047 |
| `lt_1k` flop geomean | 0.899432 | 0.899432 | 0 |
| `1k_10k` flop geomean | 0.875342 | **0.874651** | -0.000691 |
| `gt_10k` flop geomean | 0.774477 | 0.774477 | 0 |

The entire measured gain is in the intended medium bucket. Representative exact
flop improvements observed in the full output include:

| matrix | baseline candidate flops | new candidate flops |
|---|---:|---:|
| `chp_partload` | 411,740 | **396,438** |
| `crudeoil_lee2_06` | 23,031,935 | **22,540,954** |
| `rsyn0840m04m` | 210,435 | **208,326** |
| `rsyn0840m02m` | 46,846 | **46,725** |
| `gasprod_sarawak16` | 212,890 | **212,162** |

No development regression was observed. The result matches the broader
budget-split experiment's aggregate score, showing that the discarded gate
population contributed no development benefit.

## Verification

The exact commands were:

```sh
yukon setup
yukon run                         # untouched baseline
bash scripts/local-candidate-build.sh
yukon run                         # candidate, full 300-matrix corpus
bash scripts/local-candidate-build.sh
cargo test --release -p ssi-ordering-challenge -- --test-threads=1
```

The candidate build ran through the repository's macOS Seatbelt sandbox with
network denied. The full run exercised purity/license checks, two deterministic
worker calls per matrix, bijection validation, the 2-second watchdog, and the
exact symbolic scorer. The test command passed 66 active tests with zero
failures: 52 unit tests, 3 exact-equivalence tests, 2 scorer cross-checks, 4
security-boundary tests, and 5 time-cap tests. Two explicitly ignored sandbox
probes remained ignored as in the baseline.

The documented `probe_timing_and_score` filter was attempted against the root
package, but this checkout does not compile that ignored probe into the selected
test target; Cargo reported zero matching tests. No timing number is invented
from that non-run. Cap confidence instead comes from the successful full capped
run and the source-level bound: the changed path is limited to `n <= 10,000`,
`nnz <= 35,000`, no giant hub, and performs no more TELOS rounds than the
promoted code.

## Tradeoffs and limits

The hard gate deliberately leaves possible seed-diversity gains outside the
small class unclaimed. That is the correct trade after multiple broader forms
failed hidden validation: a strict improvement that grades is worth more than a
larger local improvement that times out. The ceiling should be widened only
with hidden-cap evidence or an identity-preserving way to reduce work elsewhere
on every newly admitted matrix.

The change does not prove that two seeds dominate two rounds from one seed on an
arbitrary corpus. It is a measured development result with zero observed
losses, protected by the portfolio's AMD floor and exact final comparison. The
first seed receives one round instead of two on gate-true matrices, so a hidden
matrix could in principle prefer the removed second incumbent round. That is
the transfer risk. Against it, the anchor seed is structurally different and
the public responder set shows that diversity is useful in precisely this
small, cheap regime.

No dependency changes were made. All source and documentation changes remain
inside the manifest's sole editable path, `src/ordering/`.

## Next step

Submit this exact candidate before attempting another optimization. If it is
promoted, the next safe experiment is a paired seed policy inside the same
static gate, not a wider gate: retain a second strong pre-TELOS portfolio seed
without increasing the two-round budget. If validation fails, do not add more
timing heuristics; inspect the failure class and either retry byte-identical
content for infrastructure flakiness or retire the seed split if the grader
reports a real worker timeout.
