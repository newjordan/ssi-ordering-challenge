# 0024 — Cap-safe large-only RGSUB seed transfer

## Status

Submission candidate on promoted source `698da697d45d2f86bdff9c81b1048442c78c0e42`.
The production-code commit is `48a5da3`: it improves the full 300-matrix
development score from **0.819962** to **0.819649**, while replacing the risky
two-round large-matrix RGSUB tail with one bounded round.

The important result is not just that the new deterministic trajectory finds a
different basin. An unrestricted version already proved that and then failed the
hidden two-second cap. The result here is the score/runtime trade: keep most of the
large-bucket gain, make small and medium matrices byte-identical to the promoted
source, reduce the changed stream's budget, and prevent the changed first-round
incumbent from unlocking any second RGSUB setup or search pass.

## Starting observation

RGSUB refines disjoint elimination-tree subgraphs with two deterministic
randomized elimination-game trajectories. The promoted source uses two fixed
stream seeds, a 5,000,000 word-operation budget per stream, and up to two rounds.
The second trajectory is not redundant: replacing its RNG constant with the
second seed from the already-promoted PEP portfolio changes which locally optimal
orders are reached while retaining RGSUB's uniform-prefix move policy.

The first exact-parent experiment changed only stream 1's seed:

```rust
0xD1B5_4A32_D192_ED03
```

That candidate scored **0.819409** versus **0.819962**, a development improvement
of **0.000553**. Its bucket movement was:

| bucket | promoted parent | unrestricted seed transfer | delta |
|---|---:|---:|---:|
| `lt_1k` | 0.892517 | 0.892561 | +0.000044 |
| `1k_10k` | 0.862245 | 0.862367 | +0.000122 |
| `gt_10k` | 0.733834 | **0.732326** | **-0.001508** |

The signal is concentrated in the large bucket. Small and medium movement is
noise or downstream basin interaction, not the reason to carry the seed.

## Why the unrestricted candidate was retired

The unrestricted candidate was submitted as
`4b240a20-5bf0-4118-9b5d-0634e91d4b57`. Its official benchmark run
`33209384911` failed with:

```text
RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed
```

This was not a build, purity, archive, correctness, or determinism failure. A new
seed is nominally “same work,” but that is not a sufficient cap argument: a
different first-round result can change whether round two runs, change its block
set, and add another complete `permute_pattern`, elimination-tree construction,
postorder, column-count calculation, block-adjacency construction, and refinement
pass. The hidden failure is decisive evidence that local wall-clock headroom does
not make that dynamic path safe.

Two other promoted PEP seeds were measured at the same unrestricted envelope.
They scored 0.819397 and 0.819466, but were not submitted because they inherit the
same failure mechanism. Sampling more constants is not a response to a structural
timeout.

## Production change

The candidate uses two matched gates for the benchmark's large bucket:

1. Only when `n > 10_000`, RGSUB stream 1 uses the transferred fixed seed and a
   budget capped at 3,000,000 operations. Stream 0 remains the promoted 5,000,000
   trajectory. For `n <= 10_000`, both seeds and both budgets are exactly the
   promoted program.
2. Only when `n > 10_000`, RGSUB executes exactly one round. Smaller matrices keep
   the promoted two-round ceiling.

The first-round incumbent, etree, postorder, eligible-block order, block
adjacencies, stream count, stream parameters, and stream-0 search are unchanged.
The changed stream has a 40% smaller counted budget than its promoted counterpart.
After that first round there is no second large-matrix RGSUB pass, whether or not
the transferred seed improves the incumbent.

The resulting large-matrix configured search envelope is one round of
`5M + 3M` per eligible block. The promoted parent permits up to two rounds of
`5M + 5M`. More importantly, the one-round ceiling removes the result-dependent
transition that caused the unrestricted candidate's cap argument to fail.

No phase is appended after RGSUB for this size band: terminal adjacent-pair descent
is limited to `n <= 4_000`. Thus a different large RGSUB result cannot unlock a
later large-matrix refinement phase.

## Full-corpus ablation path

Every policy below was measured end to end rather than projected from selected
rows:

| candidate | full score | fill | decision |
|---|---:|---:|---|
| promoted parent `698da69` | 0.819962 | 0.934980 | baseline |
| unrestricted seed 2, two 5M streams | **0.819409** | 0.934845 | official cap failure; retire |
| seed 2 only for `n>10k`, changed stream 3M | **0.819480** | 0.934926 | score winner, but dynamic round-two risk remains |
| same gate, second round capped to 1M per stream | 0.819591 | 0.935096 | safer configured search, weaker score |
| Ranked64 + capped second round | 0.819595 | 0.935097 | no composition benefit |
| **same gate, exactly one large round** | **0.819649** | **0.935164** | selected strict candidate |
| Ranked64 + exactly one large round | 0.819764 | 0.935262 | Ranked64 antagonistic in this basin |

The one-round candidate gives back 0.000169 relative to the 3M/two-round score
winner, but still improves the promoted parent by **0.000313**. That is a rational
exchange under an all-or-nothing cap: a timeout yields no score at all.

Small and medium bucket scores are unchanged from the parent at 0.892517 and
0.862245. The large bucket rounds to **0.7331**, versus 0.733834 at the parent.

## Isolated timing evidence

The exact production tree was built and measured independently on the Nimbus Mac
Studio. `probe_timing_and_score` ran all 300 matrices serially through `order()`:

| measurement | result |
|---|---:|
| complete rows | 300 / 300 |
| score reproduced | **0.819649** |
| worst `order()` | **0.873 s** |
| worst matrix | `crudeoil_lee4_10`, 0.8725 s |
| known tail case `ringpack_30_2` | **0.6004 s** |
| matrices below 0.400 s | 122 / 300 |

The local number is evidence, not a proof that the official runner is equally
fast. Earlier measurements show that grader slowdown varies by algorithm family,
and recent submissions with apparently adequate local margins still failed.
The stronger safety fact is parent-relative: in the only size band whose behavior
changes, the candidate keeps the same first-round structural setup, shortens one
of the two searches, and deletes the complete second pass. In the other two score
buckets it executes the promoted program.

Three public validations completed while this candidate was being measured and
all failed the hidden cap: fifth pair-descent sweep `c6cd162`, the locally
0.818750 gate-6000/Ranked64/full-budget seed stack `6f1b54b`, and the ostensibly
runtime-reducing small-tier/pair-gate candidate `d1f1db8`. Their failed benchmark
runs stopped after 926, 916, and 916 public watchdog reap lines respectively.
This confirms that “same nominal budget” or “lower total corpus wall time” is not
enough. The relevant unit is the worst individual hidden matrix, which is why this
candidate removes work in the large tail rather than adding another score feature.

## Determinism and correctness

- The seed and both thresholds are compile-time constants.
- No clock, entropy source, environment input, filesystem read, network input, or
  thread-completion order is used.
- Stream results are still merged in deterministic source order.
- `search_with_nelim` retains its measured-cost reservation and 1.25x hard stop.
- A candidate permutation must still pass `is_bijection`.
- The trusted global scorer still admits an RGSUB result only on strictly lower
  flops.
- The AMD anchor and every earlier portfolio phase remain available.
- Dependencies and `src/ordering/deps.toml` are unchanged.

The candidate can therefore change which strict improvement is returned, but it
cannot admit a malformed permutation or intentionally accept a worse objective.

## Verification

- Exact parent: `698da697d45d2f86bdff9c81b1048442c78c0e42`.
- Production-code commit: `48a5da3`.
- Sandboxed candidate-worker build: passed.
- Release suite: **66 passed, 0 failed** (52 unit, 3 exact-equivalence, 2 scorer
  cross-check, 4 security-boundary, 5 time-cap integration tests; 2 explicit
  opt-in tests ignored).
- Full sandboxed 300-matrix benchmark on Nimbus: **0.819649**, fill **0.935164**.
- Independent serial timing probe on the exact branch: **0.873 s worst**, all 300
  rows completed.
- `git diff --check`: passed.

## Decision and next lead

Submit the plain one-round candidate, not the lower-scoring two-round development
winner and not the Ranked64 composition. If official validation still finds a
cap breach, the next step is not another RNG seed. The next safe axis is static
work reallocation: fewer blocks and one stream under a hub-aware structural gate,
with round count independent of whether an intermediate candidate happens to win.

If it promotes, rebase immediately and test the independently observed score-neutral
time reductions as carriers for the next mechanism. The competition is moving
quickly enough that every later submission must refresh `origin/main` and the live
Yukon source before packaging.
