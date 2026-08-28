# 0024 — a promoted whole-matrix seed transfers to elimination-tree subgraphs

- **Date:** 2026-08-28
- **Exact parent:** `698da697d45d2f86bdff9c81b1048442c78c0e42`
- **Score:** 0.819962 → **0.819409** (−0.000553; lower is better)
- **Status:** **win on the full 300-matrix development corpus; official validation pending**

## Hypothesis

RGSUB applies the randomized exact elimination game from experiment 0022 to
small elimination-tree subproblems. Each block currently receives two searches:
stream 0 uses the mixed prefix policy, while stream 1 uses the uniform-prefix
policy. Their seeds come from `stream_rng(0)` and `stream_rng(1)`.

The promoted whole-matrix PEP selector established that a different fixed seed
set transfers to the hidden corpus. Its first seed is already identical to
`stream_rng(0)`, but its second seed is absent from RGSUB. The hypothesis was
that replacing only RGSUB's second trajectory with that independently validated
seed would reach a different subgraph basin without buying any more work.

This is a transfer test, not an assumption that whole-matrix and subtree search
landscapes are identical. The only valid evidence is an A/B on the exact live
parent.

## What changed

Only `src/ordering/rgreedy.rs` changed. Inside `subtree_refine`, stream 1 now
uses fixed seed `0xD1B5_4A32_D192_ED03`, the second seed from the promoted PEP
set. Stream 0 and any future streams keep `stream_rng(k)`.

Everything else remains identical:

- two streams per block;
- `5_000_000` deterministic word-operations per stream;
- stream 0's mixed prefix policy and stream 1's uniform-prefix policy;
- 96-block cap, subtree-size gates, two RGSUB rounds, and global gates;
- block-parallel scheduling and source-order result merge;
- trusted global re-score and strict-improvement admission.

This therefore changes the sampled trajectory, not the amount of work, memory,
coverage, or serial depth.

## Result

The repository's network-denied candidate build and full 300-matrix harness
completed normally.

| metric | parent | candidate | delta |
|---|---:|---:|---:|
| total score | 0.819962 | **0.819409** | **−0.000553** |
| fill tiebreak | 0.934980 | **0.934845** | −0.000135 |
| `lt_1k` (147) | **0.892517** | 0.892561 | +0.000044 |
| `1k_10k` (108) | **0.862245** | 0.862367 | +0.000122 |
| `gt_10k` (45) | 0.733834 | **0.732326** | **−0.001508** |

The gain is concentrated exactly where the hypothesis aimed: the weight-0.40
large bucket. Small and mid buckets move slightly backward, but the large-tier
gain is roughly an order of magnitude larger and produces the net win.

Visible examples of different basins include:

- `gasprod_sarawak81`: 1,023,556 → **942,769** flops;
- `crudeoil_lee4_10`: 200,239,183 → **199,727,877**;
- `transswitch0300p`: 385,380 → **384,101**;
- `pooling_sppc1pq`: 155,708,952 → **155,528,448**;
- `pooling_sppc3pq`: 535,484,919 → **535,257,269**;
- `cont6-qq`: 667,167,862 → **667,076,236**.

There are also losses, as the bucket table shows. For example,
`crudeoil_pooling_dt3` moves 36,208,286 → 36,223,301 and some small matrices
choose different downstream basins. The experiment is not presented as
per-matrix monotonic.

## Why it won

The seed changes which plateau and local basin the exact elimination-game ILS
reaches inside each etree block. It is useful precisely because it is not a
third copy of the same trajectory. The score-per-time shape is unusually good:
the candidate obtains a meaningful large-bucket delta while preserving the
same two calls and the same deterministic operation ceilings.

RGSUB itself admits a block only when its exact local objective improves, but
the whole pipeline is not monotone with respect to a changed intermediate
incumbent. Experiment 0020 already proved that TELOS is seed-sensitive: a better
incoming order can land in a worse downstream basin. That explains why a
same-work RGSUB trajectory replacement can improve many matrices and regress
others even though every local RGSUB edit is strictly improving. The full
end-to-end score, not a local monotonicity argument, is the correct gate.

The result also refines the PEP lesson. A seed that transferred on the hidden
whole-matrix selector was not merely lucky for that one envelope; its second
trajectory finds useful structure when reused on disjoint etree subproblems.
That makes seed-set design a first-class RGSUB axis alongside block count and
per-block budget.

## Safety and verification

- Release test suite: **38 passed, 0 failed, 54 ignored**.
- Candidate build: repository sandbox path, network denied.
- Full scorer: 300/300 matrices, score file produced normally.
- Determinism: the replacement seed is a constant; all gates, operation counts,
  prefix policies, joins, and tie resolution remain pure functions of the
  pattern and source-order stream index.
- Memory: unchanged; the same two sequential searches reuse the same per-block
  adjacency and game allocation.
- Time envelope: structurally unchanged because no stream, block, round, or
  operation budget was added. The isolated full-corpus probe measured worst
  `order()` **0.681 s** (`crudeoil_lee4_10`) and independently reproduced score
  **0.819409**; 255/300 matrices completed under 0.400 s.

## Follow-ups

- Test the third and fourth promoted PEP seeds one at a time in RGSUB stream 1;
  do not combine unmeasured seed changes.
- Test whether the second PEP seed should retain uniform-prefix params or pair
  with `Params::DEFAULT`; seed and policy are separate axes.
- If this validates, census which subtrees/matrix families account for the
  large-bucket gain before widening any gate or block count.
- The rejected `max_blocks=128` submission shows that more coverage from an old
  base does not automatically clear a newer frontier; same-work trajectory
  diversity is the safer next lever.

## Links

- [0020 — TELOS seed sensitivity](0020-telos-seed-sensitivity.md)
- [0022 — randomized exact elimination-game ILS](0022-rgreedy-elimination-game-ils.md)
- [0023 — heavy-tier gate audit](0023-heavy-tier-metric-gate.md)
