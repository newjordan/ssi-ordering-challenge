# 0038 — Reduce-then-AMF, terminal (exact degree-≤3 prefix + AMF alpha grid on the residual core)

**Date:** 2026-08-29
**Base:** accepted `a0f4fdc` (official 0.788137; dev 0.812306)
**Score:** reconstructed eval **0.788137 → 0.786953** (−0.001184); dev 0.812306 → 0.811877
**Status:** WIN, submitted. 0 of 500 / 0 of 300 regressions by construction.

## Idea

Peel degree-≤1 vertices, then eliminate every degree-≤3 vertex EXACTLY (clique on live neighbours,
ascending (degree,index) order via a lazy min-heap) — the residual is the exact fill graph after the
prefix. Run the AMF alpha grid {0.5, 2.5, 5, 10} and AMD on that core, splice the prefix in front,
admit through the trusted scorer with strict `<` against the FINISHED pipeline. New module
`core_lift.rs` (`reduce` / `splice`), one terminal block in `mod.rs`, consts `REDUCE_*`.

## Why terminal

As `consider!` candidates the same producers scored better (−0.0027 on the 13123bc-era tip) but
regressed 3 eval + 2 dev matrices (`watercontamination0303` 0.915→0.925, `nuclear104` 0.842→0.855):
a new pool argmin re-seeds the descent chain (0020). Terminal placement leaves the pipeline
byte-identical; the phase can only lower the result.

## Measured (isolated probes, 4 threads)

Eval 12 better / 0 worse: nuclear10b 0.7793→**0.6835**, edgecross24-057 0.7203→0.6545, uselinear
0.9997→0.9522, edgecross14-176, edgecross22-096, unitcommit_200_{0_5_mod_7, 100_2_mod_7, 100_2_mod_8},
methanol100, pinene100, gams02, optmass. Dev 5 better / 0 worse: faclay75 1.0000→0.9667, hydroenergy2,
methanol200/400, unitcommit_200_100_1_mod_8.
Worst order(): eval 1.356→1.407 s (watercontamination0202, both), dev 1.233→1.282 s (arki0013, both) —
within box noise. Largest addition telecomsp_metro 0.59→0.95 s (core ≈ whole matrix). Tests 65/65.

## Attribution / negatives

- On the faclay cores the ENTIRE win is AMF α5/α10 (1 ms); 24×2 relabel restarts (+0.13–0.26 s), TELOS,
  rgreedy add nothing; α0.5/α2.5 are 2× worse there. The grid stays wide because nuclear10b wins at α2.5.
- Follow-up (score-neutral): gate the phase on `core_n < 0.9·n` to skip dense matrices whose core is the
  whole matrix (telecomsp_metro); raise `REDUCE_MAX_CORE_N/EDGES` only with a timing census.
- Structured ties (squfl*, sssd*, kall*, autocorr_bern*) are at the exact elimination floor — reduce
  leaves a complete-bipartite hub core whose cost is order-independent. Not a lever.
