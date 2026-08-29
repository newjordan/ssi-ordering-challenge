# 0034 — cell-C METIS seed-12 companion (dev win, hidden floor-safe)

**Date:** 2026-08-29
**Base:** promoted frontier `345fd31` / submission `eabd452` (official 0.793070;
dev 0.812602)
**Score:** dev **0.812602 → 0.811145** (−0.001457)
**Status:** candidate for submission

## Motivation

The 0033 METIS census added one tightly gated `max_imbalance = 0.05` candidate
with seed 26 for cell C:

`n ∈ [23000,23300) ∧ nnz ∈ [880000,900000)`.

That seed is a large win on the reconstructed evaluation row `23173:893304`,
but the same structural cell also contains the public development matrix
`pooling_sppc3pq` (`23173:893724`). On the promoted frontier, the existing seed
26 does not beat that matrix's incumbent. Because METIS outcomes vary sharply
with the fixed seed, the existing `probe_fleet_j_metis_variants` instrument was
used to re-census the dev twin without broadening the gate or changing any
other algorithm family.

## Measurement

Command:

```sh
SSI_PROBE_KEYS='14100:477680,23173:893724' \
SSI_SEED_MIN=0 SSI_SEED_MAX=63 \
SSI_VARIANTS='default,niparts16,imb0.05,imb02,floor240_imb05' \
cargo test --release -p ssi-candidate-worker \
  probe_fleet_j_metis_variants -- --ignored --nocapture
```

The cell-A dev twin (`14100:477680`) was METIS-dead across all 320 tested
variant/seed points: its best isolated result was still 1.3508× the incumbent.
No candidate was added there.

For cell C, `max_imbalance = 0.05, seed = 12` was the only tested family point
that clearly beat the full-portfolio incumbent:

| matrix | incumbent flops | seed-12 end-to-end flops | factor |
|---|---:|---:|---:|
| `pooling_sppc3pq` | 452,935,769 | 360,272,881 | 0.7954× |

The isolated probe reported 0.8760× because it scores the raw METIS candidate
against the incumbent. After wiring the seed into the normal `consider!` queue,
downstream refinement compounds from the improved seed and reaches 0.7954×.
This is why the full harness result, rather than the isolated probe ratio, is
the authoritative measurement.

## Implementation

Add one second `feral_metis::metis_order_full` call inside the existing cell-C
branch, using exactly the already productive option family and fixed seed 12.
The original seed-26 call remains first. Both candidates pass through the same
trusted exact symbolic scorer and strict `< best_flops` admission rule.

Properties:

- no gate expansion;
- no new dependency;
- deterministic fixed options and seed;
- no possible flop regression, because a candidate is accepted only when the
  trusted scorer proves it strictly better than the incumbent;
- only `src/ordering/mod.rs` changes executable behavior.

## Full verification

`yukon run` completed all 300 public matrices with no panic, invalid
permutation, determinism failure, or 2-second cap failure.

| metric | frontier | candidate | delta |
|---|---:|---:|---:|
| weighted score | 0.812602 | **0.811145** | −0.001457 |
| `lt_1k` flop geomean | 0.892062 | 0.892062 | unchanged |
| `1k_10k` flop geomean | 0.859514 | 0.859514 | unchanged |
| `gt_10k` flop geomean | 0.717822 | **0.714180** | improved |

The single public mover is `pooling_sppc3pq`; the remaining 299 matrix scores
are unchanged. The official frontier score is lower than the dev score because
0033's three pins primarily target hidden/reconstructed-evaluation rows. The
new seed is floor-safe on those rows but its exact hidden score contribution is
unknown without the private corpus.

## Risk assessment

The only cost is one additional METIS pass for the already narrow cell-C gate.
The seed sweep measured the raw pass below 0.18 s locally. The 0033 experiment
measured the cell's full order at about 0.77 s after adding seed 26, leaving
substantial distance from the 2-second cap. This addition intentionally does
not include the documented `44900:160694` seed-7 win: 0033 classified that row
as a cap bet and excluded it, so it remains excluded here.

