<!-- 0027 -->
# 0027 — Deterministic flops ceiling on the second TELOS descent: the cap-safety re-gate of 0025's win

- **Date:** 2026-08-29
- **Score:** on the pre-watcher tree, 0.841594 → **0.841594** (dev OUTPUT
  byte-identical by design; the change is eval-side cap safety only). After
  the mid-session rebase onto mellowcroc's promoted watcher-MINL frontier
  (official 0.832843, dev baseline **0.841526** measured in this worktree),
  the candidate = watcher frontier + 0025 mechanism + this ceiling; scored
  runs below.
- **Status:** shipped as the cap-safety re-gate of 0025's win; the submission
  vehicle is unchanged 0025 scoring + this guard
- **Rebase note:** mellowcroc's frontier fused the three relabel multistarts
  into one seed-major loop with a hand-rolled tie-priority accept
  (`consider_relabel`) — a NEW pre-TELOS accept site. The runner-up tracking
  was integrated there (strict accepts route through `accept_top2`;
  equal-flops priority tie-swaps replace `best_perm` without touching the
  runner-up). Their watcher-MINL runs post-TELOS and cannot affect the
  TELOS-entry top-2, and their fused loop is documented selection-identical
  to the old family-major order — both re-verified by re-running the census
  on the rebased tree (0023 staleness rule).
- **Numbering/provenance:** two sessions attacked ORD-0025's grader FAIL in
  parallel. [0026](0026-cap-guard-second-descent.md) implemented and measured
  the wall-clock draft, independently found both fatal flaws (determinism-gate
  conflict; the 600 ms guard erases `procurement1large`'s win at ~1.08 s gate
  arrival), and recommended a deterministic covariate gate as "ORD-0027".
  THIS page documents the deterministic gate that actually shipped — designed
  concurrently by this session before 0026's write-up landed; commit 29a6e32
  carries this code (at the interim 25M threshold) under 0026's stale
  "wall-clock guard" title. The code in the commit is the flops ceiling; the
  title is wrong; this page is the record.

## Why the flops ceiling beats 0026's recommended density carve-out

0026's sketch (`nnz/n <= 10 OR n <= 500`) is deterministic and preserves the
procurement win, but (a) it deletes `maxcsp-langford-3-11` (nnz/n 44.9) — a
MEASURED mover — and (b) density is still a proxy: a sparse-but-deep-fill
hidden matrix (3D-grid-like, nnz/n ≈ 7, nnz ≤ 100k, Σc² ~ 1e9) sails through
a density gate and pays a catastrophic second descent, because the phase's
per-eval cost is ∝ nnz(L), not ∝ nnz/n. `best_flops = Σ cⱼ²` bounds the
hazard by its own covariate, keeps every measured mover, and is equally
deterministic — strictly better on win preservation and hazard coverage.

## What happened to ORD-0025 on the grader

Submission `5076f74` (dev 0.841594, all local gates green) **FAILED the
grader**: a hidden matrix exceeded the 2.0 s per-matrix cap and was killed.
The 0025 second descent was gated by `nnz <= 100_000` plus the distinctness
gap — both cost PROXIES. The hidden eval corpus carries a dense cost class
dev entirely lacks (0005: eval nnz/n up to 497 vs dev's densest heavy 38.6);
a dense hidden matrix small enough to pass the nnz gate ran a second descent
and blew the cap. Local cap tests could not have caught it (0010's rule: a
green local cap test is weak evidence).

## The rejected first draft: a wall-clock guard

The obvious fix — run the second descent only when `order()` has spent less
than X ms so far — was drafted and **rejected before measurement**. It
violates the file's oldest covariate rule (the candidate set must be a pure
function of `(n, nnz, max_deg, density)`, never wall-clock) for the reason
the rule exists: the grader calls `order()` TWICE per matrix and rejects the
submission unless both permutations agree byte-for-byte. Any matrix whose
elapsed hovers near the guard boundary AND whose second descent changes the
output is a determinism-gate coin flip. A cap-safety guard that adds its own
whole-submission failure mode is not a guard.

## The deterministic covariate: incumbent flops

The TELOS cap-calibration re-gate already established the phase is
EVAL-bound: per-eval cost ∝ nnz(L), the incumbent's fill. The second descent
is ≈ one more TELOS run from a same-fill seed, so its cost scales with the
same quantity — and `best_flops = Σ cⱼ²`, the exact size measure of the
incumbent's factor, is ALREADY IN HAND at the gate: deterministic,
pattern-pure, zero added cost, maximal precisely on the dense class (fill
grows quadratically with density). Same shape as FILLG's incumbent-lnnz
gate. New gate (all must pass): `nnz <= 100k` AND gap > 0.001 AND
**`best_flops <= TELOS_TOPK_MAX_FLOPS = 20_000_000`**.

## Census (probe_telos_topk ALL, FLOPSGATE lines, 2026-08-29)

All 53 nnz+gap-admitted dev matrices, by incumbent flops at the gate:

| matrix | flops at gate | margin to 20M | 2nd descent |
|---|---|---|---|
| `crudeoil_lee2_06` | 23,031,935 | 0.87x | **SKIPPED** (was zero-move; 1.643 s total order()) |
| `maxcsp-langford-3-11` | 13,076,136 | 1.53x | runs — **mover/winner** |
| `faclay30` | 12,854,123 | 1.56x | runs (zero-move) |
| `crudeoil_pooling_dt2` | 11,018,381 | 1.82x | runs (zero-move) |
| `procurement1large` | 8,790,075 | 2.28x | runs — **mover/winner** |
| … 47 more, all ≤ 3.0e6 … | | ≥ 6.7x | unchanged |

- The five 0025 winners all still descend twice and reproduce their finals
  exactly: `procurement1large` 0.581426, `st_e31` 0.954345 (flops 4,419),
  `graphpart_clique-20` 0.820095 (28,948), `maxcsp-langford-3-11` 0.385088,
  `nuclear25a` 0.652069.
- The ceiling excludes exactly ONE dev matrix, `crudeoil_lee2_06` — whose
  descent was a measured zero-move (runner-up 0.994564 → 0.994564) inside a
  1.643 s total `order()`: the pay-a-lot-learn-nothing profile that killed
  `5076f74`, alive in dev. Its output is unchanged (strict-keep had already
  discarded the descent); dev's costliest admitted descent simply disappears.
- Every mover sits at ≤ 13.1e6; the band (13.1e6, 23.0e6] contains exactly
  one dev matrix and its descent was measured useless. The ceiling sits 12x
  under `pooling_sppb0pq`'s 2.5e8 (the named tier-A single-descent budget
  item) and 260x under eigena2's 5.2e9 dense-class scale.

## Rebased-tree probe: the mechanisms COMPOSE

Targeted probe on the rebased tree (min-of-3, ceiling active): all five 0025
winners keep `seeds_descended=2` and three finals IMPROVE beyond their
pre-watcher values because watcher-MINL minimalizes the completion the second
descent's basin lands in — `procurement1large` 0.581426 → **0.581356**,
`maxcsp-langford-3-11` 0.385088 → **0.383630**, `nuclear25a` 0.652069 →
**0.651616** (`st_e31`/`graphpart_clique-20` unchanged; `chp_shorttermplan2d`
0.514660 → 0.514435 from watcher alone). `crudeoil_lee2_06` skips
(`seeds_descended=1`, final 0.988509 unchanged). Strict-keep chains: the
descent finds the basin, the watcher polishes it.

## Full dev runs

Pre-watcher tree (pre-registered: byte-identical 0.841594 twice) — PASS:
runs 1787937360 and 1787937688 both **OK 0.841594 / fill 0.948853**, every
bucket identical to 0025's scored run.

Rebased tree (pre-registered: strictly < 0.841526, byte-identical twice):
- Run 1 (`results.tsv` 1787938818): **OK 0.841312 / fill 0.948743**
  (−0.000214 vs the watcher baseline; gt_10k 0.7731 → 0.7726). The delta is
  smaller than the pre-watcher −0.000629 because watcher-MINL had already
  claimed overlapping value on the winners' matrices; the improvement is
  strict, which is what the submission gate requires.
- Run 2 (`results.tsv` 1787939156): **OK 0.841312 / fill 0.948743** —
  byte-identical to run 1; the in-run two-call determinism gate passed on
  all 300 matrices in both runs.

## Grader-fleet context at ship time (see docs/INFRA-2026-08-29-grader-massfail.md)

The 5076f74 FAIL that motivated this guard occurred during a mass-failure
window: ≥14 failed runs across 10 accounts (8-28 23:16 – 8-29 01:53), failed
runs dying at the 4-6 min mark vs 8-10 min for normal runs, coinciding with a
GitHub runner-image rollout (Node 20 → 24 deprecation warnings appeared).
So the attribution "the second descent blew the cap on a dense hidden matrix"
is PLAUSIBLE but not proven — infra slowdown alone may have killed 5076f74,
as it killed same-window submissions from many accounts including code with
no second descent at all. The ceiling is shipped anyway: it is free on dev
(output byte-identical, one useless descent removed), bounds the worst
hidden-matrix cost regardless of which explanation is true, and in a slowed
fleet every reclaimed second is grading-survival probability.

## Why this transfers where 0025 did not

The failure mode was never the mechanism (strict-keep cannot lose score) —
it was unbounded per-matrix TIME on a hidden class dev cannot exhibit. The
flops ceiling bounds the second descent's cost by the same covariate the
grader's own cost obeys, using only quantities the grader's determinism gate
cannot perturb. What is genuinely given up: hidden matrices with incumbent
flops in (2.0e7, ∞) lose their second-descent lottery ticket — on dev that
band contains one matrix and zero movers.

## Links

- The win this protects: [0025](0025-distinct-runner-up-descent.md)
- Mechanism: [0022](0022-telos-topk-runner-up.md), seed sensitivity: [0020](0020-telos-seed-sensitivity.md)
- The dense eval class: [0005](0005-eval-fail-density-regate.md)
- Why identity gating beats timing gates: ORBIT post-mortem (mod.rs) + [0010](0010-parallel-research-session.md)
