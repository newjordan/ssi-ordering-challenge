# 0025 — Pair-descent extension to (4000, 12000], hub-gated

**Status: WIN (small, pure).** Scoped n<=10000 A/B on the 0024 tree:
**15 matrices improve, ZERO regress**, Σdln −0.0256 in `1k_10k` → projected
−0.000061. The n<=12000 leg adds `mpbp_21` −0.00057 and `powerflow0300p`
~−0.0001 dln in `gt_10k` (≈ −4e-6 of score) — kept because it is free and the
hidden corpus may hold more band members.

## Why this phase and not another

`adjacent_pair_descent` is the TERMINAL phase — nothing runs after it, so a
strict accept is monotone on the final score BY CONSTRUCTION. That removes the
one failure mode 0024 measured (downstream reseed sensitivity: rsyn0815m04m
+0.0120): extending a terminal phase cannot regress any matrix. The measured
outcome matches: 15/0 win/loss versus 0024's 7/2.

## Gate

Same sparse envelope (`nnz <= 60k`), same hub discriminator as 0024's mid band
(`max_deg * 50 <= n`) for the extension region `4000 < n <= 12000`. The 12000
ceiling is `rgreedy::MAX_N` — `Game::build_adj` returns None above it, so a
higher gate would be a silent no-op; raising MAX_N is a separate (memory-safe
to ~30k but unmeasured) decision.

## Negative recorded on the way

The hub-gated RGS arm for the mid band (60k < nnz <= 250k with
`max_deg*50 <= n`) admits **zero** dev matrices — every dense mid-band pattern
has a hub column. That arm is structurally empty; do not re-derive it.

Also: whole-matrix rgreedy at n in (10000,12000] is unpromising by direct
kernel measurement — plain exact min-degree lands 2-7x WORSE than the
portfolio incumbent on the mpbp family (whose 0.31 basins came from relabelled
AMD, unreachable by the elimination game at this budget), and only
`powerflow0300p` (game 0.993 of AMD vs incumbent 0.962) is within randomized
reach. Expected value below one score unit of 1e-5; not implemented.
