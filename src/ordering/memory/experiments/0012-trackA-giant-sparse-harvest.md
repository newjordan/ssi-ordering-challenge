# 0012 — Track A: harvest the giant sparse gt_10k class (amdnd re-widen + heavy relabelled-AMF)

Date: 2026-08-18. Machine: the shared 16-vCPU VM (3 other agents co-resident; all
timings below are min-of-N isolated and treated as upper-bound contaminated; flop
ratios are exact). Branch `w1`. Score **0.843535 → 0.842794** (gt_10k bucket
0.776113 → 0.774262; lt_1k and 1k_10k untouched). All 300 matrices ran, none
SIGKILLED.

## Scope

Track A owned the big tied gt_10k matrices. Baseline census of the 45-matrix
gt_10k bucket found exactly **two** still at 1.000: `faclay75` and `gabriel10`
(the task brief's `acopf_case9241pegase_qcqp` 0.974 and `unitcommit_200_100_1_mod_8`
0.998 were already won by earlier passes — the brief's tie list was stale).
Near-ties: `faclay35` 0.9997, `unitcommit` 0.998, `transswitch2736spr` 0.9882.

## What shipped (two changes, both in `src/ordering/mod.rs`)

### 1. `HEAVY_AMDND_MAX_NNZ` 700k → 1.2M (gabriel10 win)

`probe_giants_amdvars` (AMD option grid on the 700k–1.4M sparse giants) measured
gabriel10 (n=244056, nnz=1148210, density 4.71): **non-aggressive AMD = 0.9756,
α-invariant** (all of α∈{-1,2,5,10}); aggressive = 1.0000. The win comes from
suppressing aggressive absorption, not the dense threshold. The 0008 RCA had
reverted this ceiling to 700k over a faclay75 timing hazard — but faclay75 has
nnz=1379706, so a **1.2M** ceiling re-admits gabriel10 while still excluding
faclay75 (and acopf, nnz=1292408, whose amdnd loses at 1.0608 anyway). gabriel10's
amdnd pass costs 0.105 s isolated on a cold matrix (full `order()` 0.353 s →
≈0.46 s). The global worst case (gams05 ≈0.95 s) is structurally out of the
window. This is strictly cheaper than the 1.5M ceiling 0008 shipped and the grader
passed (run 37ec258).

**Decoupling side-fix:** `PEEL_CNTREL_MAX_NNZ` was `= HEAVY_AMDND_MAX_NNZ`; left
coupled it would have widened PEEL/COUNT-RELABEL to the 700k–1.2M giants.
`probe_giants_posthoc` measured those families there and found **no value**
(faclay75: peel/count-relabel/telos-strip all exactly 1.0000; gabriel10: best
count-relabel 0.9790 < the 0.9756 amdnd already gets; acopf: best peel 0.9729 vs
incumbent 0.9737, negligible) — so it was pinned to a literal `700_000`.

### 2. Heavy tier for relabelled-AMF: nnz ∈ (130k, 400k], `nnz <= 6n`, budget `clamp(900k/nnz, 1, 3)`

The light relabelled-AMF gate (`nnz <= 130k`) barred every heavy matrix from the
fill-objective multistart. `probe_strip_giants` (seeds 1–3 on the gt_10k heavy
class) measured two wins in the sparse-heavy band:
- `transswitch2736spr` (69651, 331010, d4.75): 0.9882 → **0.9249** (seed 2)
- `arki0013` (44909, 160172, d3.57): 0.6261 → **0.6159** (seed 2)

Guards, both load-bearing:
- `nnz <= 6n` (the 0005 `HEAVY_SPARSE_MAX_AVG_DEG` sparsity guard) does double
  duty: excludes the eval dense-heavy class AND the two hottest dev matrices in
  the band — gams05 (nnz=253k, d14.6, `order()` 0.96 s) and nuclear104 (nnz=258k,
  d6.6, 0.86 s) — both of which LOSE under relabelled-AMF (1.30 / 1.48) and would
  be pushed over the known-passing ~0.99 s local bound. Every survivor is cold
  (≤0.63 s) and sparse (d ≤ 5).
- 400k ceiling + ≤3 passes: the only winners sit at nnz ≤ 331k; beyond 400k the
  class ties or loses (unitcommit 1.018, cont6-qq 1.14) while AMF passes grow
  (faclay75-class 0.28 s/pass). Worst added cost ≈ 3×0.031 s = 0.09 s on arki0013
  (0.56 s → ≈0.65 s). The window needs n > 130000/6 ≈ 21667 to be non-empty, so
  it only ever touches gt_10k.

## Negative results (the closed matrices — do not re-probe)

`faclay75` is **structurally AMD-optimal** across every family probed (all exact
ties unless noted): relabel-AMD ×12 seeds, amdnd (1.0019), amf5/amfnd, strip
deg{6,24,48,64,96} (1.0013–1.0482), lcnt64, relabelled-AMF ×3, PEEL ×16,
count-relabel asc/desc, telos-strip ×6, AMD α-grid ×8 (aggressive ties 1.0000,
non-aggressive 1.0019), METIS (2.23), Scotch (9519×). ~20+ candidates, zero wins.
`nuclear104` (everything ≥ 1.0000, and hot at 0.85 s) and `kissing2` (every
candidate exactly 1.0000, density 19.3) are likewise closed. The small ties
(supplychainr1, squfl030, emfl100, emfl050, faclay35) already receive the full
portfolio (nnz < 130k) and tie — method-limited, per 0010's "tie seam is inert".

## Attribution

| matrix | before | after | mechanism |
|---|---|---|---|
| gabriel10 | 1.000 | 0.9756 | amdnd via HEAVY_AMDND 700k→1.2M |
| transswitch2736spr | 0.9882 | 0.9249 | heavy relabelled-AMF, seed 2 |
| arki0013 | 0.6261 | 0.6159 | heavy relabelled-AMF, seed 2 |

Δscore = 0.40 × Δ(gt_10k geomean) = 0.40 × (0.774262 − 0.776113) = **−0.000741**.
Hand-computed from the three Δln terms (−0.02470, −0.06621, −0.01643; sum
−0.10734; ×exp(−0.10734/45)) = 0.842795 — matches the measured 0.842794 to 1e-6.

## Documented lead (measured, deliberately NOT shipped)

`probe_giants_rlamdnd` (relabelled NON-aggressive AMD, seeds 1–6): gabriel10
improves past the plain-amdnd 0.9756 — seed 1 0.9732, seed 3 0.9710, **seed 5
0.9684**, seed 6 0.9703 (faclay75 stays 1.0019, acopf ≥1.04 — both closed). A
gate `nnz ∈ (700k, 1.2M] && 6n` would touch ONLY gabriel10 on this corpus (the
two pooling matrices in range are dense-excluded; faclay75/acopf exceed 1.2M).
NOT shipped because: (a) the gain is marginal — best case 0.9684 is Δscore ≈
−0.00007, and staying inside the proven ≤0.99 s envelope (gams05) caps it at
seeds 1–3 for ≈ −0.00004; (b) seed 5 pushes gabriel10 to ≈1.36 s, a NEW cost
envelope above the known-passing worst — exactly the asymmetric risk 0010e warns
against on a shared, timing-contaminated VM; (c) gabriel10 is dev-only (0008), so
the eval payoff would come only from its sibling gabriel08. Revisit with a
dedicated timing budget if the cap bound is ever re-proven above 0.99 s.

## Cap-risk assessment

Worst-case `order()` is unchanged: gams05 ≈0.95 s (excluded from both new windows
by the 6n guard), faclay75's 0.59 s embedded amdnd cost NOT re-admitted (1.2M <
1.38M). New worst added cost anywhere ≈ 0.09 s (arki0013). Full corpus ran with
zero SIGKILLs. Probes added (test-only, `#[cfg(test)]`, never shipped):
`probe_strip_giants`, `probe_giants_posthoc`, `probe_giants_amdvars`.
