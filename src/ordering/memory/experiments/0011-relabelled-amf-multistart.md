# 0011 — Relabelled-AMF multistart

**Model:** Claude Opus 4.8 (medium effort), Agent: Claude Code
**Date:** 2026-08-13
**Score:** dev 0.845333 → **0.843535** (−0.001798). Buckets: lt_1k 0.9000→0.8995,
1k_10k **0.8826→0.8774** (the mover), gt_10k 0.7764→0.7761.

## Hypothesis

The relabelled-AMD multistart ([0003](0003-relabelled-amd-multistart.md)) works
because AMD's tie-break reads the vertex numbering: `AMD(Q A Qᵀ)` composed back
through `Q` is a distinct minimum-**degree** ordering for the price of one AMD
pass, so a random `Q` per seed is a randomized-restart MD. **AMF (approximate
minimum FILL) reads the numbering the same way**, and is a *different objective*
already strong in this portfolio (the α-sweep at mod.rs:1442). So `AMF(Q A Qᵀ)`
composed back is a randomized-restart minimum-**fill** ordering for the cost of
one AMF pass — a candidate family the portfolio never had.

This is the KB's own advice applied literally: best-of-portfolio.md — "when a
large set is tied at the anchor, look for variation *within* the anchor's own
family before adding another"; and [0010](0010-parallel-research-session.md)(b)
— "diversity beats quality." The tied class is where min-degree AMD already won
over every candidate; it had never been probed under the fill objective with
randomized numbering.

## Implementation

Added a loop immediately after the relabelled-AMD multistart (mod.rs ~2116),
mirroring it exactly but calling `feral_amf::amf_order_opts(&bcore, &amf_opts)`
(dense_alpha 5.0, the base α) instead of `feral_amd::amd_order`. Same
`relabel(n, seed)` seeds (AMF on the identical relabelled graph is a genuinely
different candidate), same compose-back, routed through `consider` so every
result is bijection-checked and only kept if strictly cheaper than the incumbent
— i.e. pure free upside, cannot regress any matrix.

```rust
const RELABEL_AMF_MAX_NNZ: usize = 130_000;
if nnz > 0 && nnz <= RELABEL_AMF_MAX_NNZ && relabel_cap > 0 {
    let amf_opts = feral_amf::AmfOptions { dense_alpha: 5.0, ..Default::default() };
    let amf_restarts =
        relabel_restarts(RELABEL_BUDGET, RELABEL_MAX_RESTARTS, n, nnz).min(relabel_cap);
    for r in 0..amf_restarts { /* relabel → AMF → compose back → consider */ }
}
```

Restart count from the **same per-matrix time budget** as the AMD multistart
(`RELABEL_BUDGET=300_000 / nnz`, cap 24), which self-bounds cost at every point.

## Cap safety

AMF per-pass cost scales with nnz (~0.26 s on nnz≈1M per the D α-sweep note), so
the family is gated at `nnz ≤ 130_000` (the ND/RCM/Sloan envelope), well below
that dense class. Measured worst `order()`: **0.891 s → 0.945 s** — the +0.05 s
lands on `ringpack_30_2` (nnz=121458, near the gate) and `crudeoil_lee4_{09,10}`
(nnz≈100-120k), which are *already* big wins (0.30/0.74/0.73) so AMF adds cost
without flipping them. 0.945 s is within the KB's endorsed comparative bound
("stay at or below the worst case of a revision known to have passed, 1.019 s").

A tighter gate `RELABEL_AMF_MAX_NNZ=80_000` was measured too: score 0.843693
(−0.00016 worse), worst 0.905 s. Kept 130k — better score, still inside the
endorsed timing envelope.

## Result & attribution

−0.0018, concentrated in **1k_10k** (0.8826→0.8774) — the bucket that responded
to relabel restarts historically ([0003](0003-relabelled-amd-multistart.md): it
fell 40→24 ties). lt_1k barely moved (method-limited, as
[0010](0010-parallel-research-session.md)(d) established); gt_10k mostly
unchanged (budget-limited / above the gate). All safety tests green (determinism,
exact-equivalence 3/3, scorer-crosscheck, time_cap).

## Next steps

- Sweep the relabelled-AMF `dense_alpha` (try 2.5 / 0.5, mirroring the base AMF
  α-sweep) — cheap in the same gated envelope, may add per-matrix diversity.
- Consider a relabelled-AMF pass on the *heavy sparse* class (`nnz ≤ 6n`) under a
  small budget, mirroring the heavy-AMD multi-restart ([0007](0007-heavy-multi-restart.md)),
  if isolated cost measurement clears the cap.
- The eval corpus is what decides promotion; re-measure against the reconstructed
  eval corpus ([0010](0010-parallel-research-session.md)(a)) before assuming the
  dev gain transfers.
