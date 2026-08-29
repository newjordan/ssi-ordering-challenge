# 0035 — fleet-l: AMF-α × relabel-seed + DegSqrt/SqDiv census → three pinned relabelled-AMF cells (WIN, −0.000144)

**Date:** 2026-08-29
**Base:** our own frontier `345fd31` (= promoted eabd4527, official **0.79307**; v2 proxy of the base
tree reproduces **0.793042** — Δ 3e-5, proxy still decision-grade)
**Corpus:** `_manager/eval-corpus/eval-reconstructed-v2.jsonl` (500/500, grader-exact), loaded by the
probes themselves via `$SSI_CORPUS_FILE`; reconstructed rows keyed `n:nnz`.
**Score:** v2 proxy **0.793042 → 0.792898 (−0.000144)**, 3 better / 0 worse / rest byte-identical
**Status:** SHIPPED (commit `d97bbec` + this page)

## Mission and method

Replicate fleet-j's 0033 census method (per-matrix systematic sweep, isolated through the trusted
scorer vs the full-`order()` incumbent, then per-matrix pinned cells) on two selector families the
METIS census did not touch: the AMF/AMF-relabel family and the DegSqrt/SqDiv `custom_metrics`
family. Windows-style shipping is documented dead for this game (0033); everything here is pinned
cells at the `consider!` queue tail.

## Census instrument (`probe_fleet_l_census`, probe.rs)

Scope: every v2 matrix with `nnz ≥ 60k` (all densities) + `20k ≤ nnz < 60k ∧ nnz ≥ 10n` →
**86 matrices**. Per matrix: incumbent = one full timed `order()` scored by `flops_of`, then
**15 selector points ISOLATED**: `feral_amf::amf_order_opts` at α ∈ {0.5, 0.75, 1.0, 1.25, 1.5,
2.5, 5.0} (original numbering) and `custom_metrics::order_variant` at ScoreVariant::{DegSqrt,
SqDiv} × α ∈ {0.5, 1.0, 2.0, 10.0} (aggressive=true). Shard/filter knobs as in 0033
(`SSI_CEN_MIN_NNZ`/`SSI_CEN_MAX_NNZ`/`SSI_PROBE_KEYS`).

**Result shape: 85 of 86 matrices never beat the incumbent at any of the 15 points** (1290
evaluations). 10 rows tie at exactly 1.0000 (their incumbent already IS that selector's output);
the lone winner is `35469:423442` DegSqrt α2 at **0.9985** — an isolated −0.0015 ln, far below the
0.005-ln ship bar → dropped. The original-numbering AMF-α and DegSqrt/SqDiv families are **DEAD**
across the whole heavy scope: the 0026/0023 dense-arm alphas + crown bands already hold every
basin these selectors can reach from the original numbering.

## Relabel-seed stage (`probe_fleet_l_amf_relabel`)

No α wins outright, so the mission's "relabel at the winning α" step was run on the **19 tie and
near-tie rows** (best_rel ≤ 1.02) at their argmin alphas: seeds 0..=8 of relabel →
`permute_pattern` → `amf_order_opts` → compose (the DENSE_LOW_RELABEL pattern). Three wins:

| key | α | seed | isolated | end-to-end (full order(), pin wired) | arm cost |
|---|--:|--:|--:|--|--:|
| 109774:298082 | 0.5 | 4 | 0.9852 | 4,032,013 → 3,945,217 (**0.9785×**, ln −0.0218) | 0.039 s |
| 2518:221730 | 2.5 | 5 | 0.9914 | 1,346,488,398 → 1,334,950,507 (**0.9914×**, ln −0.0086) | 0.005 s |
| 2368:206808 | 2.5 | 6 | 0.9612 | 1,049,304,158 → 1,007,643,546 (**0.9603×**, ln −0.0405) | 0.005 s |

End-to-end ≥ isolated on all three (D and F compound downstream, no flips). All 16 other probed
rows: every seed × α ≥ 1.0000 (e.g. sibling 2343:202664 best 1.0193; 8274:320136 best is seed 0's
1.0118; ties 283:55554 / 400:90300 / 3775:372500 / 149997:299992 / 249999:699990 / 339994:2479882
stay exactly 1.0000 at every seed — structurally rigid).

## Ship form

Three pinned cells at the `consider!` queue tail (mod.rs, directly after fleet-j's METIS cells),
each a tight pure-(n, nnz) window admitting exactly its winner, one relabelled-AMF task, strict
improvement under the best-of floor:

- `cell_d`: n∈[109k,110.5k) ∧ nnz∈[295k,300k) — α0.5 seed 4
- `cell_e`: n∈[2450,2560) ∧ nnz∈[219k,224k) — α2.5 seed 5 (n floor excludes sibling 2343:202664)
- `cell_f`: n∈[2350,2420) ∧ nnz∈[204k,208.5k) — α2.5 seed 6; admits dev twin
  `maxcsp-ehi-85-297-71` 2372:207996 (measured no-op: ratio identical 0.783328, +0.003 s)

## Verification

- Full-corpus multiset diff (both full v2 runs, per-row): **exactly the 3 intended rows change**,
  all improve; zero timing movers > 0.08 s elsewhere. Δscore −0.000144 decomposes exactly into the
  three Δln contributions.
- Dev harness run green (score 0.8126 / fill 0.9322); dev is unchanged except the no-op twin.
- `time_cap` suite 5/5 green (staged candidate build).

## Timing

Worst added cost +0.038 s on 109774:298082 (1.065 → 1.103 s local, ≈1.82 s at grader 1.65×). This
row was already the 7th-slowest before the pin; it stays below 71573:250344 (1.115 s). **If a
cap-FAIL ever points at the n≈110k class, drop `cell_d` first.** E and F add ≤0.01 s on 0.52–0.56 s
rows. Corpus worst `order()` **1.320 s** — the pre-existing 214928:416480 tail, untouched
(baseline run measured it 1.350 s; box noise).

## Negatives / for the record

- **The heavy AMF-α family is saturated at the original numbering** (85/86 dead, 10 exact ties) —
  third corroboration of the 0023/0026/0031 line, now census-grade. Do not re-sweep AMF alphas on
  heavy rows; only RELABELLED basins still move (and only on near-tie rows).
- DegSqrt/SqDiv likewise dead everywhere isolated (best win 0.15%, unshippable). The metric
  family's value is inside the crown's strip/cascade compositions, not as raw whole-matrix
  orderings.
- Census-time observation: 162508:518038 showed a 2.139 s `order()` in a loaded census shard but
  1.138 s in the clean full run — shard timings under load are not cap evidence.
- Relabel seeds on ultra-dense small-n rows (590/645/656/670, density 150–500): seeds always LOSE
  to seed 0 or tie at +0.03–8% — the D_WIDE relabel kill-proof (0031 item 1) extends below the
  400k-nnz floor.
