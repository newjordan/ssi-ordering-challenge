# 0021 — wire `permute_i32_into` into the relabel/HC loops (zero-alloc permute; free-rider audit)

- **Date:** 2026-08-29
- **Score:** before 0.842223 → after **0.842223** (EXACT, by design — buckets 0.8994 / 0.8753 / 0.7745 and fill 0.948942 all identical; rows 1787927270, 1787929943)
- **Status:** win (time-only, score-neutral carrier material) + one **negative result**: the index.md `RELABEL_BUDGET` 150k free rider is **stale** — measured NOT score-neutral on top of 0019, reverted

## Hypothesis

`ScoreWorkspace::permute_i32_into` (built in the `sbx-perf` session, left
`dead_code` with a vendor-match test) removes ~2 O(nnz) allocations + two
`Vec<i32>` cast allocations per restart from the hottest loops. Wiring it in
should reclaim 0.03–0.15 s exactly where the cap binds (`ringpack_30_2`,
1.784 s of 2.0 s on the fleet box; 24 AMD restarts + AMF multistart at
nnz 121,458) at exactly zero score risk, since the permutations are
byte-identical (`permute_i32_matches_vendor`).

## What changed

`src/ordering/mod.rs` — five sites now route through the workspace instead of
vendor `permute_pattern` + fresh casts:

1. relabelled-AMD multistart, 2. relabelled-AMF multistart, 3. relabelled
no-dense-AMD multistart — these are `consider`-routed, and `consider` holds an
exclusive borrow of `ws` for its whole lifetime, so they use a SECOND
lazily-created workspace (`permute_ws`; matrices whose gates give zero
restarts never allocate it). The permutation is hoisted OUT of the `consider`
closure (the closure only reads the finished i32 buffers); that moves
`relabel`+permute outside `consider`'s catch_unwind, which is safe — it is the
same faithful port `ws.flops` already runs unguarded for every candidate
scored. 4. hill-climb and 5. count-relabel run after `consider`'s last use and
reuse `ws` itself. One shared pair of reused `Vec<i32>` output buffers.

`src/ordering/scoring_ws.rs` — dropped `#[allow(dead_code)]` + stale "NOT
wired" comment. `src/ordering/probe.rs` — added `probe_permute_wire_ab`
(cfg(test): times `order()` on the cap-critical matrices only, 5 reps,
~1 min/invocation, so candidate/incumbent builds can be interleaved on a box
with uncontrollable background load).

**NOT wired** (out of scope, still allocating): `biconn.rs` per-block heavy
relabel (block size varies per block — needs a per-block workspace, different
change), the TELOS/PHARMAKOS/post-hoc `permute_pattern` calls that build a
permuted **usize** pattern for etree/counts (different mechanism, ≤2 calls per
phase), `legacy.rs` (frozen reference), probes.

## Result

Correctness: `cargo test --release -p ssi-candidate-worker` 35/35 green
(incl. `permute_i32_matches_vendor`, `matches_vendor_flops_of_synthetic`).
The harness-side `artifacts::workflow_upload_requires_successful_benchmark`
failure is PRE-EXISTING on this checkout (fails with src/ordering stashed;
caused by the local-only src/main.rs cap patch, outside the editable surface).
Full dev run: **0.842223 / 0.948942, every bucket identical** — twice.

Timing, THIS box (slower + noisy: Chrome/OneDrive at 56–100% load,
uncontrollable; fleet numbers not comparable). Full-corpus probe min-of-3:
worst `order()` incumbent 2.930 s → candidate **2.911 s**; `ringpack_30_2`
incumbent min 2.154 s vs candidate 2.259 s — but the full-corpus probe's
run-to-run spread (2.15→3.94 on ringpack across one build!) swamps the
effect, hence the interleaved A/B (min of 5 reps per invocation, order
C1→I1→C2→I2 under monotonically RISING load, so each adjacent pair is fair):

| matrix | C1 | I1 | C2 | I2 |
|---|---|---|---|---|
| ringpack_30_2 (nnz 121k) | **2.068** | 2.497 | **2.328** | 2.699 |
| crudeoil_lee4_09 (nnz 102k) | **1.755** | 1.843 | **1.886** | 1.980 |
| chimera_lga-01 (nnz 6.4k, 200 restarts) | **0.478** | 0.491 | **0.502** | 0.664 |

Candidate ≤ incumbent in every adjacent pair and at every per-matrix min;
ringpack reclaims **~0.2–0.4 s** here (upper end of the 0.03–0.15 s
prediction, plausibly because this box's allocator is slower under load).
(`crudeoil_lee4_10` was in the A/B name list but never printed — name-match
puzzle left for the next session; three matrices carry the conclusion.)

**FREE-RIDER KILLED:** index.md's "ready to ship" `RELABEL_BUDGET`
300k→150k (0017: "score identical, do not re-measure, just carry it") is
**stale after 0019** — measured on top of this wiring: **0.842457** (1k_10k
0.8753→0.8756, gt_10k 0.7745→0.7749; row 1787928780). Reverted; a warning
paragraph now sits on the constant. 0019 itself predicted this
("re-measure before riding along") and was right, 0017's "do not re-measure"
was wrong. Mechanism: 0019's floors cover nnz ≤ 20k (`small_multi`) and the
un-hubbed 30k–150k band (`mid_multi`), so the halving now genuinely cuts
seeds in the uncovered 20k–30k band and on hub-gated mid-band matrices —
under 0017's pre-floor config those seeds were redundant; under 0019's they
win things.

Local-noise note: one spurious FAIL row (1787929013, torsion50 "≥9.5 s")
landed while the box sat at 100% external load — n=2508 with sub-second
history; the immediate rerun passed. Treat single local FAILs under load as
suspect; verify load before scored runs (0019's lesson, reconfirmed).

## Why it won

Pure removal of per-restart allocator traffic with a proof of byte-identity
already in the tree — the only failure modes were borrow-graph ones
(`consider` vs `ws`), solved structurally (second lazy workspace + hoist)
rather than semantically. Score-exactness was verified, not assumed.

## Follow-ups

- `biconn.rs:609` per-block heavy relabel still allocates per restart —
  needs a per-block-sized workspace; only worth it if biconn-heavy matrices
  ever show up in the slow tail.
- The A/B probe (`probe_permute_wire_ab`) is reusable for any time-only
  change on a loaded box: interleave builds, compare adjacent pairs, min-of-5.
- Free-rider ledger: 150k halving is RETIRED as a rider. If cap margin is
  ever needed on ringpack specifically, revisit as a hub-gated budget
  (ringpack is the only mid-band hub-gated matrix that pays 2 restarts).

## Links

- Techniques: `scoring_ws.rs` (workspace layer), [0019](0019-mid-band-restart-floor.md) (floors + "re-measure the rider"), [0017](0017-relabel-budget-saturation.md) (the now-stale sweep)
