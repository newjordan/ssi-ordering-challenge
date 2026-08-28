<!-- 0019 -->
# 0019 — Mid-band restart floor, gated on max degree: the first real gain of the session

- **Date:** 2026-08-28
- **Score:** dev **0.842722 → 0.842223** (−0.000499); fill tiebreak 0.949319 → **0.948942**
- **Worst `order()`:** 1.852 s → **1.784 s** (i.e. *safer*, not costlier)
- **Status:** WIN — two regimes; final score reproduced on THREE scored runs
- **Phases:** mid-band floor alone −0.000138; adding the low-nnz regime −0.000499 total

## How this was found

Not by guessing a lever. [0018](0018-headroom-census.md) retired the near-AMD mass
as a target, which pointed at `gt_10k` (weight 0.40), so the same census method was
turned on the **winners** instead of the ties: for every matrix the portfolio already
beats AMD on (`ratio < 0.995`, `1000 ≤ n ≤ 20 000`, 100 of them), spend far beyond
the cap and see if more exists.

**15 of 100 improved, Σln(new/old) = −0.28279** — a completely different picture from
the ties' 3 of 91. The leverage is in the winners, exactly where 0018 said to look.

Then the decisive detail: several of those gains appeared at only **50–94 seeds**,
while the shipped rule `restarts = RELABEL_BUDGET / nnz` was giving those same
matrices **2–9 restarts**:

| matrix | nnz | shipped restarts | census seeds | measured slack | census gain |
|---|---|---|---|---|---|
| `procurement1large` | 41068 | **7** | 73 | 1.32 s | 0.0266 |
| `crudeoil_lee4_10` | 120632 | **2** | 50 | 0.62 s | 0.0229 |
| `mpbp_15` | 31692 | **9** | 94 | 1.40 s | 0.0079 |

So the 30k–150k nnz band is **seed-starved, not saturated**: `budget / nnz` starves it
while `heavy_multi` only fires above nnz 150 000, and the matrices in the gap are
sitting on 0.6–1.4 s of unused budget. [0017](0017-relabel-budget-saturation.md)'s
"saturated" verdict was about the **global** budget — which is pinned by the one
matrix that has no slack — and only reached 96 restarts.

## The hub gate is the whole trick

Raising the band's floor naively would also hit **`ringpack_30_2`**, which holds the
corpus worst case at 1.85 s of a 2.0 s cap and where a FAIL scores *nothing*. And it
cannot be excluded by `(n, nnz)`, because it is a **twin** of a matrix that must be
included:

| | n | nnz | avg deg | **max deg** | slack |
|---|---|---|---|---|---|
| `crudeoil_lee4_10` | 17809 | 120632 | 6.77 | **168** | 0.62 s |
| `ringpack_30_2` | 17999 | 121458 | 6.75 | **1632** | 0.15 s |

**Max degree separates them by 10×**, and it is not a coincidence — a hub of that size
is *why* each of ringpack's restarts is expensive. So `max_deg <= n / 50` is
simultaneously the cheap discriminator and the mechanism. Implemented as a floor
(`max` with `heavy_multi`), gated `nnz ∈ (30k, 150k]`, plus the heavy tier's existing
sparsity guard, which is load-bearing there (without it the one-pass floor once FAILED
the grader on eval matrices with nnz/n up to 497).

**The alternative — an nnz-window wide enough to include both — is not merely riskier,
it is strictly worse, and this is measured rather than argued.** `ringpack_30_2` was
*in* the census target set (ratio 0.302 < 0.995, n ≤ 20 000) and appears **0 times**
among the 15 improvers: more seeds buy it **nothing**. So a window including it would
spend its remaining 0.15 s of cap margin — the corpus's scarcest resource, where a FAIL
scores nothing at all — for zero return. That also corroborates
[0015](0015-zero-return-time-attribution.md) independently, which found the relabel
multi-start contributes nothing to ringpack's ratio.

A third route exists and is worth keeping for the low-nnz follow-up: key the extra
restarts on a **modeled cost budget** (sum of analytic per-family cost estimates
subtracted from the cap) rather than on `nnz` alone. That is strictly more general than
any single covariate and would let the floor extend to `chimera_lga-01` (nnz 6400,
1.78 s slack, census gain 0.0621) without needing a new discriminator per band. It is
also considerably more work to get right, and the cost model is exactly what
[0007](0007-heavy-multi-restart.md) found to be 4-23× pessimistic once measured — so it
must be calibrated against measured per-family times, not assumed.

## Phase 2 — the low-nnz regime (where the cap, not the budget, was binding)

11 of the 15 census gainers sat *below* the mid band, and for them the binding
constraint was `RELABEL_MAX_RESTARTS = 24` itself: `budget / nnz` would allow 19-78
passes and the cap clamped them to 24, while the census found gains at 198-780 seeds.

| matrix | nnz | budget/nnz | shipped | census seeds | census gain |
|---|---|---|---|---|---|
| `chimera_lga-01` | 6400 | 46 | **24** | 468 | 0.0621 |
| `chimera_k64ising-02` | 3846 | 78 | **24** | 780 | 0.0022 |
| `hydroenergy2` | 6236 | 48 | **24** | 481 | 0.0123 |
| `chimera_selby-c16-02` | 10878 | 27 | **24** | 275 | 0.0191 |

**No hub gate here, deliberately.** `chimera_lga-01` has max degree ~1119 of n=1120 — a
near-complete hub the mid band's `max_deg <= n / 50` rejects outright — yet 1.78 s of
slack and the largest single census gain on the corpus. At nnz <= 20k the whole pattern
is small enough that restart cost is bounded by nnz alone, so the mid band's gate would
discard exactly the matrix worth having. Implemented *after* `.min(cap)`, since
respecting the cap would make the regime a no-op.

### Sizing it cost two wrong attempts, both instructive

**0007's ~6e-8 s/nnz per pass does not transfer here.** First attempt at budget
3_000_000 gave `crudeoil_lee1_07` (nnz 19322) 155 passes and cost **+1.226 s** — about
**4.1e-7 s/nnz, seven times** the inherited constant. Score went to 0.841806 (the best
seen) but it put **six** matrices above 1.4 s where one had been. The max stayed under
the incumbent's 1.852 s, so it was defensible by the repo's comparative rule, but six
matrices near a hard cap is a worse risk profile than one.

Because `restarts = budget / nnz`, added time is `budget × c` and therefore roughly
**constant across the class, independent of nnz**. At the measured c, 600_000 buys
~0.27 s per matrix. Settled there: score **0.842223**, worst 1.784 s, 3 matrices above
1.4 s.

A midpoint of 1_500_000 was also tried and **is not reported as a data point**, because
its scored run FAILED — and that failure was *not* the config. `crudeoil_lee4_09`
breached the 2.0 s cap while measuring **1.253 s** under the same build (1.270 s at
baseline, i.e. untouched by any of this). See below.

## The contention finding, which cost a scored run

The failing run coincided with a fleet agent running an Intel SDE screen on the same
box: load average **41.6** on 24 threads. `crudeoil_lee4_09` needs a ~1.6× excursion to
go from 1.253 s to 2.0 s, which is exactly the run-to-run variance
[open-questions.md](../open-questions.md) already records.

Two consequences worth carrying:

- **`machine-quiet.lock` was FREE during that run.** The neighbouring fleet's
  *measurements* take it, but its builds and emulation runs do not, so acquiring it is
  not sufficient protection. A scored ssi run needs the box actually idle, verified by
  load average, not merely the lock. Waiting for load < 4 and re-running reproduced
  0.842223 twice.
- **This exposes the incumbent, not just this change.** Any matrix at 1.25 s FAILs under
  a 1.6× excursion, and the frontier config has one at 1.852 s. The comparative rule
  ("stay at or below the worst case of a revision known to have passed") is necessary
  but not sufficient when the passing revision itself sits at 93% of the cap.

## Result, measured against the shipped baseline

| | matrices |
|---|---|
| better | **4** — `mpbp_48` −0.0250, `procurement1large` −0.0180, `mpbp_15` −0.0030, `nd_netgen-3000-1-1-b-b-ns_7` −0.0030 |
| worse | **1** — `chp_shorttermplan2d` 0.4840 → 0.5150 |
| unchanged | 296 |

Worst `order()` **1.852 → 1.796 s**: the boost lands only where slack exists
(`procurement1large` 0.679 → 0.958 s against 1.32 s of slack), and `ringpack_30_2` is
untouched by the hub gate, so the corpus worst case does not move. Both required runs
scored 0.842584 identically.

`crudeoil_lee4_10` — the matrix that motivated the band — was ultimately **excluded**
by the heavy tier's sparsity guard (`nnz ≤ HEAVY_SPARSE_MAX_AVG_DEG · n`), so its
0.0229 of census gain is still unclaimed. That guard is there for a measured
grader-FAIL reason and was not touched.

## The important caveat: this is NOT monotone on score

The floor is monotone on the *restart count* (seeds are `0..restarts`, so a larger
count is a superset and the best-of floor can only improve within the relabel set).
It is **not** monotone on score, because those restarts share a budget with the phases
appended after them — the hill-climb of [0006](0006-hillclimb-phase.md) especially —
so spending more here can starve a phase that was winning a matrix. That is exactly
what `chp_shorttermplan2d` is: +0.031 against four gains, net −0.000138.

**Do not widen this gate on the assumption that more restarts cannot hurt.** The
honest framing is that this is a *reallocation* with a favourable net, not free upside,
and the next widening must be measured matrix-by-matrix the same way.

## Next

- The `chp_shorttermplan2d` loss is worth understanding rather than gating around: if
  it is hill-climb starvation, the fix is to make the appended phase's budget
  independent of the restart count rather than to shrink the band.
- 11 of the 15 census gainers are still unclaimed, including `chimera_lga-01`
  (0.8706 → 0.8085 at 468 seeds, 1.78 s slack, nnz 6400 — *below* this band) and the
  three `chimera_*` matrices. A low-nnz analogue of this floor is the obvious follow-up.
- `RELABEL_BUDGET 300_000 → 150_000` (index.md's "ready to ship" free rider) is
  **still applicable** and still unclaimed: it was score-neutral at the old restart
  counts and should be re-measured on top of this change before riding along.
