# 0008 — No-dense AMD ceiling restored to 1.5M (gabriel10 reclaimed)

**Result: dev 0.875217 → 0.875036. WIN.** One constant changed:
`HEAVY_AMDND_MAX_NNZ` 700_000 → 1_500_000.

## Why

0005 cut the ceiling to 700k using a 5.6e-7 s/nnz worst-cost model,
deliberately giving up gabriel10's 0.9756. The 0007 measurements showed that
era's cost models run 4-23× pessimistic, so `probe_vheavy` re-measured the
700k-1.5M sparse class in ISOLATION (min-of-2):

| matrix | nnz | amdnd ratio | amdnd cost |
|---|---|---|---|
| faclay75 | 1,379,706 | 1.0019 | **0.351 s** (2.5e-7/nnz — the true worst constant) |
| gabriel10 | 1,148,210 | **0.9756** | 0.088 s |
| acopf_case9241pegase_qcqp | 1,292,408 | 1.0608 | 0.090 s |

Worst added cost at a 1.5M ceiling ≈ 0.38 s — the SAME envelope the 700k
re-gate already accepted (≈0.39 s). Sparsity guard unchanged (nnz ≤ 6n), so
the 0004 dense-pattern failure mode stays excluded.

Also measured and declined: relabel seeds 5-12 up there (faclay75 flat 1.0000
×12; acopf 0.9737→0.9719 at seed 7 — not worth 8×0.15 s).

## Eval outlook

Post-0007 census (`e53a6f3` run, reconstructed 0.885175 exactly): 0007's eval
wins were powerflow2383wpp 0.942→0.936, watercontamination0303 0.991→0.976,
gabriel08 0.878→0.871, tie-flip acopf_case1354pegase_qcqp. Remaining sparse
heavy eval ties: faclay70/60, optcdeg2, unitcommit×2, powerflow2736spp, dtoc5,
supplychainp1_053050. Of these only faclay70 (1.09M) enters the new amdnd
window — and dev faclay75 shows 1.0019 there, so expect little; the dev
gabriel10 flip (-0.00018) is the main payload. Timing: worst order() 0.793 s
(this run) incl. faclay75's +0.35 s.
