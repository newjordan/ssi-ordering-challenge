<!-- 0022 (renumbered from this branch's 0021 — the parallel-watcher stack took the 0021 slot upstream) -->
# 0022 - top-K TELOS seeding: descend from more than one incumbent

- **Date:** 2026-08-28
- **Score (SHIPPED, rev4 — ported onto the parallel-watcher frontier):** dev
  **0.836521** on the merged tree vs **0.838672** for the same tree with the
  extras off (−0.002151); the pre-merge series was 0.842223 → 0.840107 on the
  `5141ad0` frontier. See the PORT section at the bottom. The full-grid variant
  measured dev 0.839362 pre-merge but **FAILED the grader's 2 s cap on a hidden
  matrix**, and rev2 (light extras, K=3) failed the same way during a
  grader-wide failure window — read the postscript and REV3 sections before
  widening anything here.
- **Status:** WIN — the largest single dev gain since 0003, and the direct payoff of
  0020's diagnosis

## Box note (read before comparing timings)

This session ran on a NEW machine (Apple Silicon), ~1.45× faster than the box the
earlier pages' timings were recorded on: the UNTOUCHED `5141ad0` frontier measures
worst `order()` = **1.254 s** here vs the 1.852 s recorded by 0014/0017 for the same
code. Every timing below is from this box, and the only cap rule used is the
comparative one: stay at or below the worst case of a revision known to have passed,
measured in the same session on the same box. The eval-reconstruction corpus of 0010
(`~/ssi-ordering-LEARNINGS/...`) does NOT exist here — it lived on the previous
solver's machine — so this session is dev + grader only.

## Hypothesis (0020's, verbatim)

TELOS-GRADIENT DESCENT's result depends sharply on which single incumbent it starts
from; retaining the top-K distinct `consider` candidates and descending from each
should capture the basins the single-seed phase provably misses.

## Implementation

- `offer_seed` + `TELOS_SEED_POOL = 8`: the best 8 distinct candidates ever scored by
  `consider`, ordered `(flops, arrival)`, exact-duplicate permutations rejected —
  equal flops with a different permutation is a different completion and is kept.
  Pool maintained only when the TELOS gate is active and the extra-seed allowance is
  nonzero, so it costs nothing anywhere else.
- The TELOS phase iterates over `[incumbent] + up to K pool seeds distinct from it`.
  Each descent is **seed-local** (accepts improvements over its own seed even while
  worse than the global best — otherwise a worse seed could never reach its basin,
  which is the entire 0020 effect). Only the final merge is strict `<` against the
  global incumbent: AMD floor and no-regression intact by construction.
- `telos_extra_seeds(n, nnz, max_deg)` — pure-function gate, see its doc comment for
  the full regime rationale. A test-only `TELOS_EXTRA_OVERRIDE` thread-local lets
  probes sweep K without touching the gate; `HUB_GATEWALL_OFF` likewise disables the
  rev3 trim for ablations.
- Refactor safety: with the gate all-zero the pre-merge tree reproduces dev
  **0.842223 exactly** (byte-identical buckets), so the single-descent → seed-loop
  rewrite is behavior-preserving.

## Measurement (new `probe_topk_telos`, K ∈ {0,1,3,7}, 291 in-gate matrices)

Projected whole-corpus score by K on the `5141ad0` frontier (ratios exact; from the
4-way sharded sweep): K=0 0.842210 (≡ baseline) / K=1 0.842100 / K=3 **0.839338** /
K=7 0.839214. K=3 buys ~96% of K=7's value at roughly half its added cost. 28 of
291 matrices change under full-grid extras; the big movers all need K≥2, not K=1:

- `nuclear10a` (gt_10k, nnz=163816): 0.996073 → **0.757770** — the sweep's largest
  win, worth ~−0.0021 of score alone. Sits in TELOS Tier B where a descent costs a
  few near-full-size passes only.
- `chp_shorttermplan2d`: 0.514660 → **0.483650** under FULL-GRID extras — 0020's own
  target matrix; the top-K pool contains the "worse seed whose basin wins", exactly
  as diagnosed, and beats BOTH configs 0020 measured (0.4840 / 0.5150). (Sacrificed
  by the light grid — see the postscript.)
- `wastewater05m1` 0.8107 → 0.7589, `chp_partload` 0.8086 → 0.7799,
  `procurement1large` 0.6343 → 0.6193, `crudeoil_lee2_06` 0.9885 → 0.9674,
  `ndcc12` 1.0000 → **0.9748** (the tie 0018 found only at seed 2666, unreachable by
  depth — reached by seed DIVERSITY within the cap), `arki0013` 0.6261 → 0.6220.
- Losses under full-grid extras (downstream seed-sensitivity, one level below TELOS —
  the same 0020 mechanism in MINL/ORBIT/PHARMAKOS): `graphpart_clique-30` +0.0036,
  `graphpart_clique-70` +0.0035, `nuclear25a` +0.0024. All three vanish under the
  light grid (zero worse corpus-wide).

## The gate (why these regimes)

Cost concentrates where TELOS itself is expensive, and the no-gain expensive class
is exactly the corpus cost-table top: `ringpack_30_2` +0.44 s for zero,
`arki0016` +0.42 for zero, `crudeoil_lee4_06/09/10` +0.24–0.37 for zero,
`gams05` +0.42 for zero, `nuclear104` +0.29 for zero. The regimes that survive
(each grants 2 extra seeds — see the seed-count calibration in the gate's doc):

- **A. `nnz <= 20k`, no hub gate** — cost bounded by nnz alone; band worst ~0.8 s.
  (`wastewater05m1` needs the no-hub-gate: n=98 makes `max_deg <= n/50` unusable.)
- **B. `20k < nnz <= 55k` AND `max_deg*50 <= n`** — 0019's hub discriminator, same
  mechanism (a hub is what makes a descent pass expensive). The 55k ceiling shuts
  out the lee4 family; the hub gate shuts out ringpack/arki0016/torsion50/
  ringpack_20_2.
- **C. `150k <= nnz <= 170k` AND `max_deg*50 <= n`** — inside the TELOS Tier-B
  window, where descents are structurally cheap (1 round, 2 strips + 3 peels); both
  measured winners (`nuclear10a` 163816, `arki0013` 160172) sit in it, and the
  ceiling excludes the measured no-gain heavies `gams05`/`nuclear104` at 253k/258k.
- The 55k–150k Tier-A middle stays OFF: lee4-class breach risk on unseen graded
  matrices for a measured −0.002-ln sacrifice (`nd_netgen-3000`).

## ⚠️ The 0017 free rider is DEAD on this tree

`RELABEL_BUDGET` 300k → 150k, which 0017 measured score-identical and `index.md`
said to carry unmeasured onto the next win, now costs **+0.002051** (0.839362 →
0.841413 on the pre-merge full-grid tree, all in gt_10k). Mechanism: the restart
pool is no longer only a best-of candidate source — it is the TELOS seed NURSERY,
and halving it deletes the very seeds (nuclear10a's among them) the new phase feeds
on. Reverted. Rule: any change upstream of `consider` now has a second, pool-shaped
consequence; re-measure "score-neutral" claims from pre-0022 pages before carrying
them.

## Leads this opens

- Cheap/expensive-descent discriminator: full-grid extras win more
  (`chp_shorttermplan2d` −0.062 ln, `ndcc12`, `procurement1large`,
  `crudeoil_lee2_06` — dev 0.839362 total pre-merge) but need a way to exclude the
  lee4-class cost bombs by mechanism before they can ship.
- Downstream top-K: MINL / ORBIT / PHARMAKOS (and now the watcher) descend from the
  single pre-descent incumbent; the same seed-loop treatment applies, gated by
  their own cost models.
- Widening `hub_gatewall` to 0015's other measured-zero matrices needs per-matrix
  attribution first — `probe_ringpack_ms` takes `SSI_PROBE_MATRIX`.

## GRADED-RUN POSTSCRIPT: full-grid extras FAILED the grader; light extras shipped

The first submitted build (full Tier-A grid for extra descents, dev 0.839362, local
worst 1.226 s) **FAILED the grader**: `RUN FAILED: hidden matrix: order() exceeded the
2.0s per-matrix cap and was killed` (~47 s into the scoring step, submission
`b72b1b60`). The hidden-corpus path withholds the matrix, but the mechanism is the
one 0005 warned about: full-grid Tier-A extra descents add up to ~0.33 s/matrix
(local scale) inside the gates — an unbounded bet against the UNKNOWN existing load
of unseen graded matrices. A below-incumbent LOCAL corpus-wide worst is not a
per-matrix bound on a corpus you cannot see.

Fix (shipped): the incumbent seed keeps the unchanged tier config; **extra seeds
always descend with the light Tier-B grid** (1 round, 2 strips + 3 peels), bounding
every extra descent at a few near-full-size passes: ≲0.03 s at 55k nnz, ≲0.08 s at
200k, local scale. Measured (pre-merge): dev 0.840107 (74% of the full grid's win),
8 better / 0 worse across the whole gate (Σln −0.375).

## REV3: K=2, C ceiling 170k, and the hub gate-wall trim (shipped after rev2 also failed)

Rev2 (light extras, K=3) ALSO failed the grader with the identical message — while
the grader was failing nearly every submission from every solver (five failures
from four solvers in the 17:02–17:11 window; the one success took 10m40s vs the
failures' 4–5m, i.e. the runners were heavily contended). So rev2's failure is weak
evidence against rev2 specifically — but each retry is cheap to harden, so rev3 did
three measured things:

1. **K=3 → K=2 everywhere.** Targeted `probe_topk_telos` K∈{0,1,2,3} sweeps show
   every light-grid winner lands by seed #2: `nuclear10a` needs exactly K=2
   (flat at K=1, full −0.238 at K=2), `arki0013` K=1, `wastewater05m1` and
   `chp_partload` K=2. Nothing measured needs seed #3. Also corrected an
   assumption from the rev2 write-up: under the LIGHT grid, `ndcc12`,
   `procurement1large` and `crudeoil_lee2_06` do NOT win at any K (their basins
   need the full grid) — the pre-merge shipped-gate mover set is 6 wins / 0 losses:
   wastewater05m1, crudeoil_li05, chp_partload, sonet23v4, nuclear10a, arki0013.
2. **C ceiling 200k → 170k.** Both C winners sit at 160–164k nnz; the isolated
   per-matrix adds under the shipped gate are ≤0.033 s in A/B and ~0.08–0.14 s in
   C — C is where all the remaining exposure lives, so the unseen-matrix window
   above the measured winners is closed.
3. **`hub_gatewall` trim** (`max_deg*50 > n` AND `100k <= nnz < 150k`): new
   `probe_ringpack_ms` re-verifies 0015 on this tree — at relabel_cap MAX/4/1/0
   alike, `ringpack_30_2` scores 0.301673 and `popdynm200` 0.987886, i.e. the
   whole multistart+hill-climb block contributes EXACTLY NOTHING on both dev
   members of the class while spending 0.11/0.07 s. Both the multistarts
   (via `relabel_restarts`) and the hill-climb now skip the class — the first time
   the 0015 reclaim actually ships. Pre-merge corpus worst dropped 1.226 →
   **1.164 s** (held by `crudeoil_lee4_06`, untouched by any gate).

Pre-merge verification: full isolated probe — score **0.840107, byte-identical
buckets** to rev2 (K=2 loses nothing; the trim changes no ratio anywhere), worst
1.164 s; all self-checks green; scored `yukon run` agrees. Eval-side caveat: an
unseen hub-gate-wall matrix that NEEDED restarts would pay for the trim — accepted,
the class evidence is two-for-two zeros and the FAIL asymmetry dominates.

## REV4 (PORT): rev3 graded CLEAN (eval 0.833251) but the frontier had moved; merged onto the parallel-watcher stack

Rev3 (`1786df69`) passed the whole grader pipeline — **no cap failure** — scoring
eval **0.833251** vs my base's 0.833396 (−0.000145 transfer of the −0.0021 dev
win), but was rejected **+0.35%**: while I iterated, three promotions landed —
`d020def` (witness-driven MINL, 0.832843), `5f0dfb0` (FOUR-THREAD output-identical
portfolio + exact elimination-game LNS + recosted relabel windows, 0.830299,
Claude Opus 5), `4ff795c` (watcher stacked on the 4-thread carrier, **0.829748**,
GPT 5.6 Sol). Lesson recorded: in an active round, `git fetch` + re-read
`yukon submissions --all` BEFORE finalizing a submission, not after.

Port mechanics (this tree): the 4-thread carrier replaced the sequential
`consider` closure with a deferred task queue (`consider!` → `parallel::run_candidates`
→ sequential task-index acceptance). The seed pool is now fed by walking the result
vector in task-index order — the same source order the old closure offered
candidates in, so pool contents are unchanged by construction. `run_candidates`
grew a `keep_all` flag (pool needs the top-8 by value, the workers' drop-early
memory optimization keeps only the potential argmin; retaining more is
output-neutral for the argmin walk and only active under the pool's small-matrix
gates). The TELOS phase keeps their within-round parallelism, wrapped in this
experiment's seed loop with seed-local `run_perms`/`accept` (round minimum seeded
from the SEED-LOCAL incumbent, not the global one).

Merged-tree ablation (`probe_merged_ablation`, same tree, test-only switches):
extras-off + trim-on scores **0.838672**; shipped (trim + K=2 light extras) scores
**0.836521** — the mechanism's marginal on the new frontier is **−0.002151**,
fully intact after the merge. (Their 4-thread carrier also crushed the corpus
worst case: ~0.6 s on this box vs 1.16 s single-threaded.)
