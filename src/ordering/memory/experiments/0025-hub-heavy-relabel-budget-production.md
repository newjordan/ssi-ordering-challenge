# 0025 - hub-heavy relabel-budget deletion, production-tree A/B

- **Date:** 2026-08-29
- **Baseline:** exact official `5141ad0`
- **Status:** positive validation candidate; no submission

## Hypothesis and static discriminator

A global relabel-family budget reduction from 300,000 to 150,000 changed four
public outputs: one win (`pooling_sppa0pq`) and three losses. The sole win is
hub-heavy, while all three losses can be excluded by a direct structural rule:

```text
nnz <= 150_000 && max_deg > n / 50
```

The rule is not a matrix-name exception. It is the complementary side of the
already promoted mid-band floor's load-bearing hub gate. Large hubs make the
quotient-graph relabel passes disproportionately expensive; the low-hub side is
where the incumbent adds work, and this proposal deletes work on the high-hub
side.

The four causal rows support the boundary:

| matrix | n | nnz | max degree | n/50 | gate |
|---|---:|---:|---:|---:|---|
| `pooling_sppa0pq` | 1,649 | 24,192 | 64 | 32 | true |
| `meanvar-orl400_05_e_7` | 5,205 | 174,804 | 404 | 104 | false: nnz ceiling |
| `crudeoil_lee4_10` | 17,809 | 120,632 | 84 | 356 | false: low hub |
| `rsyn0840m04m` | 8,508 | 24,272 | 19 | 170 | false: low hub |

This is deliberately a coarse, portable boundary. It does not encode a hash,
matrix name, exact dimension, exact nnz, or a narrow numeric window centered on
the public winner. The `150k` ceiling already defines the incumbent mid-band
regime. The `n/50` hub ratio is already part of the promoted source and has a
mechanistic interpretation: it separates hub-heavy quotient graphs, on which a
restart is costly, from the low-hub graphs allowed to receive the extra restart
floor. The candidate simply makes the opposite cost decision on the
complementary side.

## Implementation

The production tree chooses a 150,000 relabel budget under the rule above and
retains 300,000 everywhere else. The selected budget is reused by all three
existing relabel families: AMD, AMF, and no-dense AMD. No candidates, seeds,
gates, scorers, or downstream phases were added. The mutation only removes a
suffix of deterministic relabel seeds.

For the causal measurement only, a `cfg(test)` atomic switch let one release
test binary run the exact `5141ad0` policy and the production gated policy in
sequence. The switch is absent from production builds.

The budget selection happens once after `n`, off-diagonal `nnz`, and maximum
off-diagonal column degree have been computed. That one value is then supplied
to the three existing `relabel_restarts` calls. All downstream code is shared.
For a changed matrix, the candidate evaluates seeds `1..=r_candidate`, which is
an exact prefix of the baseline's `1..=r_baseline`. Therefore the source-level
work delta is auditable: no replacement candidates are introduced and every
removed operation is a suffix pass from an existing family.

## Measurement protocol

The ignored release probe loads the repository's canonical 300-matrix dev
corpus once. For each matrix it performs the following paired sequence in one
process and one compiled test executable:

1. Set the test-only switch false and call the unmodified 300k policy.
2. Set it true and call the production static-gate policy.
3. Score both final permutations with the same exact symbolic flop routine.
4. Compute the same three size buckets and weighted geomean as the harness.
5. Compare full permutation bytes, exact integer flops, restart counts, and
   paired wall time.

The test runs with one test thread, so the process-global measurement switch is
never observed concurrently. Its default is true, so ordinary tests exercise
the production candidate. A non-test release build does not contain the switch
or the baseline branch at all.

Candidate-pass deletion is counted analytically from the actual restart-count
difference and the family gates already in the source. The ordinary AMD relabel
family always contributes one removed-pass multiplier; AMF contributes when
`nnz <= 130k`; no-dense AMD contributes when `nnz <= 60k`. This gives an exact
315-pass deletion independent of noisy wall clocks.

## Full 300-matrix same-binary A/B

| metric | exact `5141ad0` | gated deletion | delta |
|---|---:|---:|---:|
| weighted flop score (lower is better) | 0.842222684 | **0.842221471** | **-0.000001213** |
| strict flop wins / losses | - | **1 / 0** | - |
| final permutation changes | - | 1 | - |
| relabel-family passes | - | -315 | strictly less work |
| whole-corpus wall total | 141.859 s | 135.738 s | -6.121 s |
| actually affected wall total | 27.965 s | 24.874 s | -3.091 s |

The static gate is true on 202 matrices, but the promoted low-nnz restart floor
dominates the ordinary budget on 170 of them. Therefore only **32 matrices**
actually lose relabel passes. The gate-false path covers 98 matrices and is
byte-identical on all 98. Across the whole corpus, 299 final permutations are
byte-identical.

The distinction between gate-eligible and restart-affected matters. At
`nnz <= 20k`, the incumbent's 600k low-nnz floor is applied after the ordinary
budget/cap calculation. It therefore dominates either 300k or 150k and keeps
the final restart count unchanged. This explains why a wide, structurally
natural gate can have a narrow actual effect without relying on identity.

The only output change is the intended win:

```text
pooling_sppa0pq: 1,507,275 -> 1,506,523 flops
                 0.607832 -> 0.563715 s in this paired run
```

Timings are directional cost evidence only. The host was contended and several
unmodified calls exceeded the nominal two-second cap, so absolute maxima are
not grader-cap evidence. The deterministic count of 315 deleted candidate
passes is the reliable cost fact.

The whole-corpus wall total fell 4.3%, while the sum over the 32 matrices whose
restart count actually changed fell 11.1%. Those percentages are consistent
with the code-level deletion, but the paired ordering was always baseline then
candidate and the machine was busy, so they must not be treated as calibrated
performance estimates. In particular, the noisiest individual rows moved in
both directions despite the candidate doing less deterministic work.

## Score and hidden-corpus interpretation

The public score gain is very small: 0.000001213 absolute, or about 0.000144%
relative. It is nevertheless an exact integer-flop change propagated through
the official aggregation, not rounding noise. The sole output change reduces
flops by 752 and every other public final permutation is byte-identical.

That does not make the change mathematically score-monotone on unseen inputs.
The final portfolio contains TELOS after the relabel families. TELOS is seeded
from the current incumbent, and prior causal work proved that a better or worse
upstream incumbent can lead its deterministic descent into a different basin.
Deleting restart candidates can therefore alter the downstream result in
either direction. This is exactly why the global budget reduction was rejected
after producing three public losses, and why the present static exclusion test
is substantive rather than cosmetic.

The hidden-case argument is about cost and structure, not guaranteed score:

- high-hub patterns are the side on which quotient-graph restarts are costly;
- the candidate only removes work and improves time-cap margin by construction;
- the three known public regressions do not satisfy the gate for independent
  structural reasons;
- the gate is broad enough to apply to unseen matrices without recognizing a
  public identity;
- TELOS seed sensitivity remains the irreducible quality risk and must be
  settled by the official hidden benchmark.

## Validation checklist

- Exact parent/source content: `5141ad0`.
- Full corpus: 300/300 matrices, same release test binary.
- Public causal result: one strict win, zero strict losses.
- False-path proof: 98/98 gate-false permutations byte-identical.
- Whole-corpus identity: 299/300 permutations byte-identical.
- Work accounting: 315 deterministic relabel-family passes removed.
- Production release build: succeeds.
- Candidate-worker non-ignored release tests: 35 passed, zero failed.
- No dependency or manifest change: `src/ordering/deps.toml` unchanged.
- No submission made from this worktree.

## Decision

Keep as a positive, generalizable validation candidate. It reproduces the one
public win with zero public losses, proves false-path identity, and only deletes
work. Do not infer hidden monotonicity: fewer upstream restarts can change the
seed passed to TELOS, so the hidden corpus can still regress even though the
source mutation is a cost deletion. No submission was made from this worktree.
