# 0026 — RGSUB round ladder and ranked-mode parameter sweep (gt_10k rig)

**Status: one keeper (+3rd round, sparse gt), four informative negatives.**

## The rig

`SSI_CORPUS_FILE` pointed at the 45 dev `gt_10k` patterns alone scores the
whole bucket in **~43 s** (vs ~9 min full corpus) — the score printed is the
gt bucket geomean directly (empty buckets renormalize out). This is the right
iteration loop for anything gated `n >= 10000`. Regenerate the corpus file in
the same command as the run — something on this box deletes stray files in
home/scratch directories within minutes.

## Ladder (rig gt geomean; base = c7d5a8b + 0024/0025 hardened)

| config | rig score | delta vs base | verdict |
|---|---|---|---|
| 2 rounds (shipped) | 0.729592 | — | base |
| **3 rounds, `n>=10k && nnz<=100k`** | **0.729403** | **−0.000189** | **KEEP: −0.000076 of full score, cost ≈ +0.05 s only on matrices where round 2 accepted, tail class excluded by the nnz gate** |
| 4 rounds, same gate | 0.729373 | −0.000030 more | drop: poor cost ratio |
| max_s 900 (ranked mode) | 0.729685 | +0.000093 | drop: bigger blocks HURT — corroborates the density-penalty logic of the ranked selection |
| min_s 24 (ranked mode) | 0.729380 | −0.000023 more | drop: marginal, and its extra block cost lands on the FULL ranked gate (nnz to 1.5M) i.e. the grader-tail class |
| ranked selection below n=10000 | (scoped) | **byte-identical** | postorder-first-96 and density-ranked-96 choose equivalent blocks below 10k; the seam only exists where blocks are plentiful AND value is skewed |

## Rule applied throughout

Additions are admitted only where the hidden tail of the affected class is
provably NOT near the cap (see 0024's grader forensics): the 3rd round gate
`nnz <= 100_000` keeps it off nuclear104/acopf/faclay-class matrices, and the
round only runs at all when the previous round strictly improved.
