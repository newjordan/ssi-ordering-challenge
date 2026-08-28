<!-- 0024 -->
# 0024 — strip_amf: AMF-α5 sub-ordering of the SPARSE-class stripped core — killed at probe stage, a clean zero

- **Date:** 2026-08-29
- **Score:** dev **0.842223 → 0.842223** (unchanged — kill criterion fired at the
  pre-registered flops-only probe; no mechanism was ever wired into `order()`,
  no scored run of the mechanism paid)
- **Status:** NEGATIVE — killed; the 0014/0016-class zero the plan itself priced in

## Hypothesis

The SHOT-3 dense-class mutual exclusion (`STRIPV2_DENSE_SPLIT_AVG_DEG = 35.0`,
mod.rs) exists because AMF-α5 beat AMD on stripped cores of the DENSE class —
yet no sparse-class (`nnz < 25n`) stripped core had ever been offered AMF: a
verified coverage hole, and exactly the "transform giving an EXISTING family
new coverage" shape 0017/0018 converged on. Candidate: offer AMF-α5 as the
sub-orderer of the shipped STRIP transform's k=64 core (deg-ranked and
lcnt-ranked), purely additive via `consider()`, gated `nnz < 120_000`
(mandatory — excludes both gate-wall matrices per 0013). Honest prior of a
0014/0016-class zero — hence rank 4 and probe-gated: a zero costs one
flops-only probe, not a scored run.

**Pre-registered kill criterion:** zero matrices where a variant is argmin or
beats the shipped incumbent by ln-advantage ≥ 0.005 ⇒ drop WITHOUT any scored
run.

## What changed

Only `probe.rs` (cfg(test), kept): **`probe_strip_amf`** — for every corpus
matrix inside the shipped STRIP gate (`n < 30_000 && nnz < 400_000 &&
nnz < 25n`) AND `nnz < 120_000`, computes flops of the full shipped `order()`
(incumbent best) vs the two candidate variants, built with the EXACT shipped
pieces: `strip_general_v2` (the shipped STRIP-V2 construction), the shipped
`sub_amf5` closure (`AmfOptions { dense_alpha: 5.0 }`), the shipped deg
ranking, and the shipped lcnt ranking (baseline-AMD L-column-counts, identical
to the shipped `lcnt64` candidate's). Reports per-matrix flops ratio and
ln-advantage. `src/ordering/mod.rs` was **never touched**.

## Result

**205 matrices in gate. WINS (ln_adv ≥ 0.005): 0. Near-misses (0 < ln_adv <
0.005): 0.** Killed.

Stronger than the kill line required: the AMF-α5 stripped-core variant never
beat the incumbent AT ALL — every ln_adv ≤ 0, with exactly 3 zero-ties
(`maxmin`, `orth_d4m6_pl`, `knp5-43`, all with variant ratio 1.0000, i.e. flops
identical to the incumbent). Everywhere else it loses, typically badly:
variant/incumbent flops ratios cluster at 1.1–7x, with tails to 12–29x
(`catmix200` 28.8x, `rocket50` 13.2x deg / 26.0x lcnt, `cvxnonsep_psig30r`
23.9x lcnt). Representative rows (variant ratio = best-of-two / incumbent):

| matrix | n | nnz | deg64-amf r | lcnt64-amf r | ln_adv |
|---|---|---|---|---|---|
| `crudeoil_lee4_09` | 15904 | 101792 | 1.2630 | 1.0987 | −0.094 |
| `gabriel09` | 21688 | 89702 | 1.2265 | 1.1756 | −0.162 |
| `emfl100_5_5` | 21925 | 52300 | 1.0568 | 1.0838 | −0.055 |
| `clay0304m` | 258 | 840 | 1.0438 | 15.3694 | −0.043 (best non-tie in visible rows) |
| `chimera_lga-01` | 1120 | 6400 | 3.3602 | 2.1076 | −0.746 |
| `knp5-43` | 2065 | 31132 | 1.0000 | 1.0031 | ±0.000 (tie) |

Cost was never the issue: per-variant cost measured 0.000–0.035 s (largest on
`crudeoil_lee4_09`) — the candidate died on VALUE, not time.

## Why it lost

The dense-class precedent does not transfer, and the reason is visible in the
shipped code's own comments: `sub_amf5` won the dense class (avg_deg ≥ 35)
where "AMF's dense_alpha handling is the natural fit" — fat columns are the
problem AMF's α-deferral solves. The sparse-class STRIP transform exists for
the opposite regime (density 3–15) and its k=64 strip ALREADY removes the fat
columns before the sub-orderer runs: after stripping, the core is exactly the
clean low-degree graph where minimum DEGREE is at/near optimal (0012's exact-DP
proof, 0018's census: AMD near-optimal on the tie class). Min-FILL on that
core is a strictly worse local objective — hence not "different basin,
sometimes better" but "systematically worse, often multiples". The three ties
are matrices where the strip tail dominates flops entirely, so the sub-orderer
choice cannot matter. In hindsight the 0016 lesson ("against the real
portfolio, a new orthogonal candidate adds nothing") plus the mechanism of the
dense split predicts this zero; the probe cost ~2 min to confirm.

With 0014 (in-family quality), 0016 (orthogonal family), and now 0024
(cross-class transfer of a winning sub-family), diversity bets have produced
three zeros from three different directions against this portfolio. The
remaining live scoring lead stays 0022's `procurement1large` cross-basin
descent (≈ −0.0006, measured), not candidate diversity.

## Follow-ups

- None from this mechanism. Do NOT retry AMF variants (other α, aggressive
  absorption) on sparse-class stripped cores without a new mechanism argument —
  the loss margins here (1.1–7x) are far beyond parameter-tuning range.

## Links

- Techniques: [0016-spectral-fiedler-nd.md](0016-spectral-fiedler-nd.md) (the
  diversity-zero precedent), [0013-gate-covariate-audit.md](0013-gate-covariate-audit.md)
  (the 120k gate-wall exclusion this candidate's gate honored),
  [0023-ultrasmall-band-floor.md](0023-ultrasmall-band-floor.md) (probe-before-build protocol)
- Probe: `probe_strip_amf` in `src/ordering/probe.rs`
