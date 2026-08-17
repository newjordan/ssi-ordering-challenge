# 0012 — MinFill gate nudge (workflow test)

**Idea.** The MinFill portfolio candidate (best-of-routed via `consider`, so
zero score-downside) was gated very tightly at `n<3000 && nnz<12000`. Small
sparse tied-at-AMD matrices just above that window (`n` 3000–4000, `nnz`
12000–16000) received no MinFill candidate. Extend the gate to
`n<4000 && nnz<16000`.

**Diff.** `MINFILL_MAX_N` 3000→4000, `MINFILL_MAX_NNZ` 12000→16000 (mod.rs).

**Result (dev, 300-matrix corpus).** 0.843535 → **0.843473** (−0.000062).
Per bucket: lt_1k 0.899524 (unchanged) · 1k_10k 0.877442→**0.877236** ·
gt_10k 0.776113 (unchanged). No bucket regressed (best-of floor).

**Safety.** Still far below the slow tier (`nnz≥163816`); MinFill membership
matrix ≤ n² bytes = 16 MB at the new cap. Full test suite green: 48 unit +
3 exact_equivalence (closed-form invariants) + 2 scorer_crosscheck + 2 time_cap.

**Status.** Small, measured, cap-safe. Submitted as a submission-workflow test
(Model: Claude Opus 4.8). Not expected to move the eval frontier materially;
the win is one small-bucket dev matrix. The larger documented leads
(under-served gt_10k ties gated on `n` while cost tracks `nnz`) remain open.
