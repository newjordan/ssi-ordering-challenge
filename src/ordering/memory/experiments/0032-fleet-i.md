# 0032 — fleet-i: TELOS Regime-A full-grid discriminator (WIN) + terminal-MINL sparse-gt cell (WIN); rgsub round-3 widening (CONCLUSIVE NEGATIVE)

**Date:** 2026-08-29
**Base:** frontier `5f024a7` (mellowcroc, official 0.810201; v2 proxy 0.810172)
**Corpus:** `_manager/eval-corpus/eval-reconstructed-v2.jsonl` (500/500, grader-exact; baseline
reproduces 0.810172 vs 0.810201 official). Probes load the env corpus themselves
(`probe_fleet_i`, `probe_fleet_i_fullgrid` — the candidate-worker test shim is dev-fixed).
**Score:** PENDING-FINAL
**Status:** items 1 and 2 SHIPPED; item 3 NO-SHIP with kill-proof

## Item 1 (SHIPPED): full-grid TELOS extras discriminator for Regime A

0022's stated open lead: full-grid extra descents win deep basins the light grid misses, but
needed a discriminator to exclude the regression/cost classes. Regime B (20k–55k, hub-safe)
already ships full-grid; Regime C's Tier-B schedule IS the light grid (full-grid is a no-op
there) — so the whole remaining lead is **Regime A (nnz ≤ 20k)**.

Measured it exhaustively: `probe_fleet_i_fullgrid` runs the whole `order()` under
(gate | fullgrid,K=2 | fullgrid,K=3) for every regime-A matrix on BOTH corpora
(~330 v2 rows, ~250 dev rows). Movers under fullgrid K=2:

| corpus | row | maxdeg | density | gate → fg | hub-safe? | d≥3? |
|---|---|--:|--:|--|--|--|
| v2 | n=2545 nnz=11146 | 41 | 4.38 | 0.332885 → **0.262729** | ✓ | ✓ |
| dev | gasprod_sarawak16 4596:15316 | 22 | 3.33 | 0.982329 → **0.913645** | ✓ | ✓ |
| dev | netmod_kar1 1746:4928 | 79 | 2.82 | 0.812836 → 0.811710 (small win) | ✗ | ✗ |
| v2 | n=1758 nnz=9122 | 61 | 5.19 | 0.905905 → 0.974759 (LOSS) | ✗ | — |
| v2 | n=186 nnz=1446 | 16 | 7.77 | 0.661586 → 0.663700 (loss) | ✗ | — |
| dev | rsyn0820m04m 6028:17372 | 19 | 2.88 | 0.867622 → 0.892554 (LOSS) | ✓ | ✗ |
| dev | st_e31 301:744 | 6 | 2.47 | 0.949052 → 0.951257 (loss) | ✓ | ✗ |

The split is exact in both corpora: **every winner is hub-safe (`max_deg*50 ≤ n`) at density ≥ 3;
every loser is either hub-heavy (0022's downstream-seeding regression class) or near-forest
sparse (`nnz < 3n`, the structural-tie shape)**. Shipped: `telos_extra_fullgrid` gains the
clause `nnz ≤ 20k ∧ nnz ≥ 3n ∧ hub_safe` (K stays 2 — the K=3 column shows fresh losses on
hub-safe rows, e.g. dev mpbp_47, and its only extra winners are hub-UNSAFE rows whose class is
an unshippable mixed bag: v2 n=2114 −0.107 vs its near-twin n=1758 +0.073).

Cost: 33 v2 rows in the cell; worst add is the winner itself (+0.056 s at 0.91 s base,
min-of-3), every other add ≤ 0.04 s; the hottest cell row (n=2944, 1.00 s) adds +0.004 s
(same-process A/B, twice).

## Item 2 (SHIPPED): terminal-MINL sparse-gt cell, 30k < n ≤ 50k ∧ nnz ≤ 170k ∧ nnz < 10n

The 0030 census: winnable sparse gt_10k band mass needs SEARCH reach, not selectors. The
terminal-MINL envelope stopped at n ≤ 30k; the band rows sit just above it. Measured the
widening `n ≤ 50k ∧ nnz ≤ 200k ∧ d < 10` first, then trimmed by measurement:

| v2 row | before → after | Δln | add (min-of-3) |
|---|--|--:|--:|
| 44900:160694 | 0.549608 → **0.538018** | −0.0213 | +0.153 s (0.87→1.02) |
| 44919:160328 | 0.548431 → **0.537539** | −0.0201 | +0.149 s (0.88→1.02) |
| 44909:159902 | 0.535832 → **0.535501** | −0.0006 | +0.161 s (0.88→1.04) |
| 35762:116172 | 0.977419 → **0.975839** | −0.0016 | +0.039 s (0.83→0.87) |
| 43680:172480 | −0.00025 for +0.126 s | cut | (nnz ceiling 200k→170k) |
| 22355:110738, 16229:108532 | zero at +0.05 s each | cut | (n floor 30k) |
| 33001:84000 | zero at ~0 s | rides | in-cell, free |

Strict-accept through `ws.flops` on the final completion ⇒ score-safe; the cell is
evidence-derived on the grader-exact corpus. Timing bet, stated exactly: the three n≈44.9k
twins go 0.87 → 1.02–1.04 s local (≈1.7 s at grader 1.65×, ≈1.83 s if the minimalization
work grades at 2.6×) — below the 1.88 s p99 of the f2f0e6ef kill-line census but the
closest-to-cap addition in this page. If a cap-FAIL points here, drop this clause first.

## Item 3 (NO-SHIP, killed): rgsub third round on the sparse (100k, 350k] band

Widened `rgsub_rounds` 3-round region from `nnz ≤ 100k` to `nnz ≤ 350k ∧ nnz < 10n` (n ≥ 10k)
and measured all 15 cell members targeted (min-of-3, vs same-method baseline): 9 rows improve
but every improvement is −0.0002…−0.001 ln (**−1.4e-5 of score in total**) while the extra
round costs **+0.09–0.21 s on rows already at 1.0–1.3 s local** (71573:250344 1.13→1.29,
109774:298082 1.06→1.23, 82941:342060 0.92→1.13) — and the grader is ~2.6× on rgreedy-style
work. Reverted; the negative is recorded at the `rgsub_rounds` site. The remaining rgsub-won
band mass (rows ≥ 400k nnz) is even deeper in the tail — round-count reach there is dead
without a fundamentally cheaper round.

## Final verification

PENDING-FINAL

## Instruments

- `probe_fleet_i` — corpus-flexible timing/score probe (env `SSI_CORPUS_FILE`,
  targeted min-of-3 via `SSI_PROBE_ONLY` with `n:nnz` keys), probe_fleet_h's pattern.
- `probe_fleet_i_fullgrid` — whole-`order()` A/B of Regime-A extras under
  (gate | fullgrid | fullgrid+K=3), same corpus flexibility.
