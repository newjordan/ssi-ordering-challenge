<!-- 0021 -->
# 0021 - top-K TELOS seeding: descend from more than one incumbent

- **Date:** 2026-08-28
- **Score:** dev **0.842223 → 0.839362** (−0.002861; fill 0.949068 → 0.948534), worst
  `order()` 1.254 → **1.226 s** on the measuring box (see the box note below)
- **Status:** WIN — the largest single dev gain since 0003, and the direct payoff of
  0020's diagnosis

## Box note (read before comparing timings)

This session ran on a NEW machine (Apple Silicon), ~1.45× faster than the box the
earlier pages' timings were recorded on: the UNTOUCHED frontier tree measures worst
`order()` = **1.254 s** here vs the 1.852 s recorded by 0014/0017 for the same code.
Every timing below is from this box, and the only cap rule used is the comparative
one: the shipped config's worst case (1.226 s) is BELOW the untouched frontier's
worst case measured in the same session on the same box, so this ships strictly more
margin than the incumbent had, wherever the grader's absolute speed lands. The
eval-reconstruction corpus of 0010 (`~/ssi-ordering-LEARNINGS/...`) does NOT exist
here — it lived on the previous solver's machine — so this session is dev + grader
only.

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
  probes sweep K without touching the gate.
- Refactor safety: with the gate all-zero the tree reproduces dev **0.842223
  exactly** (byte-identical buckets), so the single-descent → seed-loop rewrite is
  behavior-preserving.

## Measurement (new `probe_topk_telos`, K ∈ {0,1,3,7}, 291 in-gate matrices)

Projected whole-corpus score by K (ratios exact; from the 4-way sharded sweep):

| K extra seeds | projected score |
|---|---|
| 0 | 0.842210 (≡ baseline) |
| 1 | 0.842100 |
| 3 | **0.839338** |
| 7 | 0.839214 |

K=3 buys ~96% of K=7's value at roughly half its added cost → every admitted regime
ships 3. 28 of 291 matrices change; the big movers all need K=3, not K=1:

- `nuclear10a` (gt_10k, nnz=163816): 0.996073 → **0.757770** — the sweep's largest
  win, worth ~−0.0021 of score alone. Sits in TELOS Tier B where a descent costs a
  few near-full-size passes only.
- `chp_shorttermplan2d`: 0.514660 → **0.483650** — 0020's own target matrix; the
  top-K pool contains the "worse seed whose basin wins", exactly as diagnosed, and
  beats BOTH configs 0020 measured (0.4840 / 0.5150).
- `wastewater05m1` 0.8107 → 0.7589, `chp_partload` 0.8086 → 0.7799,
  `procurement1large` 0.6343 → 0.6193, `crudeoil_lee2_06` 0.9885 → 0.9674,
  `ndcc12` 1.0000 → **0.9748** (the tie 0018 found only at seed 2666, unreachable by
  depth — reached here by seed DIVERSITY within the cap), `arki0013` 0.6261 → 0.6220.
- Losses (downstream seed-sensitivity, one level below TELOS — the same 0020
  mechanism now visible in MINL/ORBIT/PHARMAKOS): `graphpart_clique-30` +0.0036,
  `graphpart_clique-70` +0.0035, `nuclear25a` +0.0024. Absorbed several times over
  by the regime aggregates; a future 0022 could top-K those phases too.

## The gate (why these regimes)

Cost concentrates where TELOS itself is expensive, and the no-gain expensive class
is exactly the corpus cost-table top: `ringpack_30_2` +0.44 s for zero,
`arki0016` +0.42 for zero, `crudeoil_lee4_06/09/10` +0.24–0.37 for zero,
`gams05` +0.42 for zero, `nuclear104` +0.29 for zero. The regimes that survive:

- **A. `nnz <= 20k`, no hub gate** — cost bounded by nnz alone; band worst ~0.8 s.
  (`wastewater05m1` needs the no-hub-gate: n=98 makes `max_deg <= n/50` unusable.)
- **B. `20k < nnz <= 55k` AND `max_deg*50 <= n`** — 0019's hub discriminator, same
  mechanism (a hub is what makes a descent pass expensive). The 55k ceiling shuts
  out the lee4 family; the hub gate shuts out ringpack/arki0016/torsion50/
  ringpack_20_2. Regime worst measured 1.234 s contended / covered by the isolated
  full-corpus check below.
- **C. `150k <= nnz <= 200k` AND `max_deg*50 <= n`** — exactly the TELOS Tier-B
  window, where descents are structurally cheap (1 round, 2 strips + 3 peels); the
  200k ceiling excludes gams05/nuclear104. Captures nuclear10a and arki0013.
- The 55k–150k Tier-A middle stays OFF: lee4-class breach risk on unseen graded
  matrices for a measured −0.002-ln sacrifice (`nd_netgen-3000`).

Isolated full-corpus verification of exactly the shipped config: **0.839362, worst
1.226 s** (below the incumbent's 1.254 s), all 66 harness self-checks green
including the enforced time-cap suite.

## ⚠️ The 0017 free rider is DEAD on this tree

`RELABEL_BUDGET` 300k → 150k, which 0017 measured score-identical and `index.md`
says to carry unmeasured onto the next win, now costs **+0.002051** (0.839362 →
0.841413, all in gt_10k). Mechanism: the restart pool is no longer only a best-of
candidate source — it is the TELOS seed NURSERY, and halving it deletes the very
seeds (nuclear10a's among them) the new phase feeds on. Reverted. Rule: any change
upstream of `consider` now has a second, pool-shaped consequence; re-measure
"score-neutral" claims from pre-0021 pages before carrying them.

## Leads this opens

- Downstream top-K: MINL / ORBIT / PHARMAKOS descend from the single post-TELOS
  incumbent and produced this experiment's three losses; the same seed-loop
  treatment applies, gated by their own cost models.
- K=2 was never measured (grid was 0/1/3/7); if time budget ever tightens, check
  whether K=2 keeps the K=3 wins.
- The 55k–150k middle band still holds `nd_netgen`'s win and possibly more on eval;
  reopening it needs a discriminator that excludes the lee4 family by mechanism,
  not by name.
