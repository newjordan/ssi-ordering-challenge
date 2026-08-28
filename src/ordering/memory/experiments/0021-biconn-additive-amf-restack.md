# 0021 — additive biconn AMF restacked on the restart-floor frontier

- **Date:** 2026-08-28
- **Baseline:** official frontier source `5141ad0` (submission `f6e0c2d7-bef6-4dc9-8e72-108f41e3a259`, official score `0.833396`)
- **Status:** controlled local improvement; review candidate only; not submitted

## Why this is a restack, not a replay

Experiment 0015 added a single block-local AMF candidate to the biconnected
heavy ordering. It was designed specifically to correct P2's failed
cross-family experiment: P2 used a fixed extra-slot schedule and replaced
promoted later relabelled-AMD passes with AMF/no-dense candidates. The hidden
corpus rejected that schedule, `0.834007 -> 0.834098`, despite a local slice
win. The conclusion was not that cross-family block candidates are worthless;
it was that a best-of portfolio cannot preserve a candidate which was removed
before exact scoring takes place.

0015 therefore retained all 24 global biconn-heavy relabelled-default-AMD
tokens and their up-to-seven-per-block allocation, then bought one plain AMF
block candidate from a separate budget under a narrow full-matrix `nnz <=
10_000` gate. On the former `3b1d869` frontier's 255-matrix `n <= 10_000`
slice, that changed `0.888004 -> 0.887565`. The gate overlaps the small/medium
population, so the result must be remeasured after the new official frontier's
restart floors; carrying an old score through that source change would be a
false comparison.

`5141ad0` is the requested new reference. It contains the official best
`0.833396` and adds two relabel restart-floor regimes in `mod.rs`: a
max-degree-gated mid band and a low-nnz floor. These are independent ordering
phases, but the low-nnz floor deliberately expands the restart search for
matrices at or below 20,000 nnz. That overlaps the P4 AMF population and can
change both the incumbent that the later biconn candidate has to beat and the
timing profile. This experiment reuses none of the old frontier code as a
baseline; it applies only the biconn additive mechanism to exact `5141ad0`.

## Source reconciliation

The restack was made in a fresh worktree rooted at `5141ad0`. All new frontier
changes remain in place, including the `max_deg` computation, the sparse
mid-band guard, the low-nnz restart floor, the TELOS seed-sensitivity records,
and every experiment/memory entry through 0020. No P4 memory file was copied
over wholesale, because that would lose or stale-downgrade those records.

Only three source adjustments were made:

1. `biconn_heavy_order` takes an additional `total_cross_family` budget.
   Its existing `total_restarts` allocation runs first, unchanged: eligible
   blocks remain largest-first, existing budget remains 24, each still gets at
   most seven relabelled default-AMD restarts, and the old deterministic seed
   stream is untouched.
2. A second allocation consumes the new budget only after the old allocation
   is complete. It is also largest-eligible-block first and is globally bounded.
   The shipped call purchases exactly one token. Token one is plain AMF on the
   induced block; a second no-dense-AMD token is supported by the helper but is
   not enabled or measured here.
3. The biconn caller purchases that one token only if the *whole matrix* has
   `nnz <= 10_000`. The inherited biconn-heavy `nnz <= 60_000`, `min_block =
   32`, and `block_nnz_cap = 150_000` conditions still apply. The test-only
   biconn-heavy probe explicitly passes zero extra cross-family tokens, so its
   historical measurement remains a measurement of the promoted schedule.

Every old biconn candidate is therefore still generated. The new AMF ordering
is compared against it by the existing exact local `flops_of` scorer; the
composed biconn ordering is then submitted to the normal full-matrix exact
`consider` comparison against the entire `5141ad0` portfolio. The AMD anchor
remains present. A new candidate can be ignored, but an old biconn relabel
candidate cannot disappear as it did in P2.

## Controlled measurement

The causal gate population is not merely `n <= 10_000`: the production
addition is enabled by nnz. We generated an explicit JSONL subset from the
public development corpus containing exactly the 186 matrices with `nnz <=
10_000`. Both sides ran the same subset in separate LF-native WSL exports,
each built with the required prepare step, offline locked trusted release
parent build, and sandboxed candidate-worker build. Scores are written by the
trusted scorer after valid deterministic worker results.

| revision | matrices | weighted flop score | fill tiebreak |
|---|---:|---:|---:|
| exact `5141ad0` | 186 | 0.906951 | 0.971098 |
| `5141ad0` + additive biconn AMF | 186 | **0.906481** | **0.970952** |
| delta | 0 | **-0.000470** | **-0.000146** |

This is an exact same-export baseline/candidate comparison, not an inference
from P4's former slice. The score improvement remains after the new low-nnz
restart floor is present. It is also monotone with respect to objective
selection: all existing candidates and the AMD floor are available, while the
new local candidate is added under exact scoring. The only remaining material
risk is runtime, not symbolic quality.

## Cap audit

The 186-matrix gate population completed on the restack. This is the narrow
population that can pay for the extra AMF work, and it excludes every
whole-matrix timing wall that motivated P4's gate: `arki0016` (`nnz=37,208`),
the rejected P2 30k region, the remainder of the inherited 60k biconn-heavy
window, and all known 100k-plus gate-wall matrices. The addition is one induced
block AMF pass per matrix, never per block and never a new multi-start.

The broader `n <= 10,000` local run was not a reliable cap discriminator on
this host: repeated runs were killed in inherited/new-frontier workloads on
`qapw` (`nnz=87,496`) and `mpbp_07` (`nnz=30,810`), both outside the AMF gate.
Those failures are evidence that a broad local cap run is noisy under current
machine contention, not evidence that this 10k-nnz addition is responsible.
They must not be used to claim a larger gate is safe. Conversely, they do not
invalidate the completed gate-population comparison. The policy is unchanged:
ship less when timing is uncertain, and never attribute a cap failure to code
which cannot run on that matrix.

## Decision

Keep the restacked mechanism for review. The controlled score improvement is
reproducible against the exact official-frontier source and retains the
structural protection missing from P2. Do not submit it from this worktree:
the official frontier's tight timing profile and the new restart floors require
review or reconstructed-evaluation evidence before external validation.

The durable rule is stronger than the original P2 correction. A new candidate
must be additive not only relative to the immediately previous biconn schedule,
but also relative to any newly promoted global phase that overlaps its gate.
Restacking and remeasuring on the real current source is required whenever a
frontier changes the incumbent portfolio in the same population. A second
cross-family token remains deliberately deferred until it earns an independent
same-frontier score and cap budget.

## Links

- [0019](0019-mid-band-restart-floor.md) documents the newly promoted restart
  floors and their hub-based timing discriminator.
- [0020](0020-telos-seed-sensitivity.md) records why extra restart candidates
  are not globally score-monotone after downstream TELOS seeding.
- [0015](0015-zero-return-time-attribution.md) records the frontier timing
  fragility that requires a narrow P4 gate.
