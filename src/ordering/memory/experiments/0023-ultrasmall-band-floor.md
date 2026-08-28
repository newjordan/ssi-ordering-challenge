<!-- 0023 -->
# 0023 — Ultra-small-band restart floor (nnz ≤ 8k, budget 1.5M): killed at probe stage — the census gain was ALREADY CLAIMED

- **Date:** 2026-08-29
- **Score:** dev **0.842223 → 0.842223** (unchanged — kill criterion fired at the
  pre-registered probe; no mechanism was ever wired into `order()`, no scored run paid)
- **Status:** NEGATIVE — killed; the 0019 census table is STALE as a target list

## Hypothesis

A fourth relabel floor at `nnz <= 8_000` with budget 1_500_000 (~234 seeds on
`chimera_lga-01` vs ~94 shipped) would claim the 0019 winners-census gains:
`chimera_lga-01` 0.8706 → 0.8085 (0.0621, the corpus's largest single census gain),
plus `chimera_k64ising-02`, `hydroenergy2`, `chimera_selby-c16-02`. Expected dev
−0.0003 to −0.0007. Pre-registered kill criterion: < 50% of the 0.0621 by seed 234.

## What changed

Only `probe.rs` (cfg(test), kept): **`probe_ultrasmall_census`** replays the EXACT
shipped candidate stream — per seed, all three multistart objectives (relabelled AMD,
relabelled AMF a5, relabelled no-dense AMD), same `relabel(n, seed)` stream, seeds
1..=800 — on the four census targets, printing the running best at checkpoint seed
counts, every improvement event with its seed, and measured per-seed cost.
`src/ordering/mod.rs` was **never touched**.

## Result

Gain vs the CURRENT shipped final, by seed count (running relabel-family best):

| matrix | nnz | current final | census target | gain @94 | gain @234 | gain @800 | improvement seeds past 94 |
|---|---|---|---|---|---|---|---|
| `chimera_lga-01` | 6400 | **0.8034** | 0.8085 | 0.0000 | **0.0000** | 0.0169 | **781** only |
| `chimera_k64ising-02` | 3846 | 0.6229 | — | 0.0000 | 0.0000 | 0.0000 | none (saturated at seed 7) |
| `chimera_selby-c16-02` | 10878 | 0.5519 | 0.5557 | 0.0000 | 0.0000 | 0.0000 | none (saturated at seed 32) |
| `hydroenergy2` | 6236 | 0.8759 | 0.8622 | 0.0000 | 0.0086 | 0.0137 | 154, 196, 308, 376 |

Kill line was gain@234 ≥ 0.031 on `chimera_lga-01`; measured **0.0000**. Killed.

## Why it lost

**The 0019 census table was measured against the PRE-0019 baseline (24 restarts) and
the 0019 low-nnz regime itself already claimed almost all of it.** The winning seeds
sit at 91 (`chimera_lga-01`), 7 (`chimera_k64ising-02`), 32 (`chimera_selby-c16-02`) —
all within the ~94 passes the shipped `600_000 / nnz` floor already buys. The shipped
final on lga (0.8034) is *below* the census target 0.8085 (downstream phases improved
on the relabel best). The "0.0621 unclaimed" framing in 0019's Next section and in
log.md/index.md describes the pre-Phase-2 state and survived, stale, into the plan
that ranked this candidate #2.

What actually remains past seed 94 in this class:

- `chimera_lga-01`'s next improvement is at **seed 781** (0.8085 → 0.7865, amf):
  budget ≈ 781 × 6400 ≈ **5M**, i.e. ~2.3 s added per matrix class-wide at the
  measured cost constant — cap-suicidal, unreachable by any sane resize.
- `hydroenergy2` is the only live residual: 0.0086 @234, 0.0137 @376+ (matches the
  census's 0.8622 exactly, winning seed 376 → needs ≈ 2.35M). Back-of-envelope dev
  value: Δln = ln(0.8622/0.8759) = −0.0158 in a ~108-matrix bucket at weight 0.30 →
  **≈ −0.00004 dev** — an order of magnitude below the candidate's promised range,
  for +0.6–1.0 s added on EVERY nnz ≤ 8k matrix (added time is `budget × c`, roughly
  constant per matrix across the class) plus 0020-mode TELOS-seed regression risk on
  every matrix whose relabel best would change.

The resize option (3M → 468 seeds) buys exactly the same: lga/k64/selby 0.0000,
hydroenergy2 0.0137. Killed rather than resized.

**Cost model CONFIRMED even though the gain premise died:** measured per-seed cost
2.54–6.01 ms across the four matrices ≈ **4.7e-7 s/nnz per seed** (3 objectives),
matching 0019's measured ~4.1–4.5e-7 and again nowhere near the heavy tier's 6e-8.
The plan's timing math was right; its value math was stale.

## Method note (the transferable part)

A census is a snapshot against a specific shipped configuration. **Before building
on a census number, re-run the census against the CURRENT incumbent** — a ~15 s
flops-only probe (this one) invalidated a candidate whose design doc, cost model and
gating were all otherwise sound. This is the third consecutive stage (0021's 150k
rider, 0022's rank-2 seed, this) where a recorded "ready/unclaimed" claim died on
re-measurement.

## Follow-ups

- The stale "chimera_lga-01 0.0621 unclaimed" claims in 0019's Next section,
  log.md's 0019 entry, and any plan derived from them are RETIRED by this page.
- `hydroenergy2` −0.00004 at seed 376 is real but not worth a floor; record only.
- The strict-beat scoring vehicle for the eventual submission must come from
  elsewhere: the live measured leads are 0022's `procurement1large` cross-basin
  descent (≈ −0.0006 weighted, measured) and `crudeoil_lee4_10`'s 0.0229 (blocked
  by the load-bearing sparsity guard).

## Links

- Predecessors: [0019](0019-mid-band-restart-floor.md) (the census and the floors),
  [0021](0021-permute-i32-wiring.md) / [0022](0022-telos-topk-runner-up.md)
  (the same stale-claim failure mode, twice)
- Probe: `probe_ultrasmall_census` in `src/ordering/probe.rs`
