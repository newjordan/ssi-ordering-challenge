# 0036 — the REALIZATION step is a live seam: seeded-MCS PEO diversity on the descended graph (WIN, dev −0.000316, 3/0) + a grader-FAIL autopsy

**Date:** 2026-08-29
**Base:** re-verified across four frontier merges as the leaderboard moved — `3f497f6`
(0.791285), `8c2ce41` (0.789898), `5581542` (0.789054), `a0f4fdc` (0.788137). All four score
**0.812306 on dev** (their wins are eval-row-specific and invisible here).
**Score:** dev **0.812306 → 0.811990** predicted (−0.000316), 3 better / 0 worse
**Status:** SHIPPED (seeded MCS only). The AMD-family variant is KILLED — see the autopsy.

## The lead

`aa23179` (vibecodooor) introduced a new mechanism and stated its own generalisation: the
completion-lattice descent *chooses a chordal completion `G'`*, and realization then *chooses
a PEO/tie-break basin for `G'`* — a separate problem, and "MCS and AMD do not exhaust those
basins". They shipped it behind a one-matrix cell. 0034 had just proved the selector and gate
axes saturated, so a brand-new mechanism swept on one row was the only live seam on the board.

## What generalises, and what does not

Censused with a per-row flops+time dump probe, diffed process-to-process:

- Forcing their SHIPPED extra realizations (`AMF(-1)`, relabel-767+`AMF(-1)`) on for every
  terminal-gate row changes **zero** dev rows. That pair is genuinely `gabriel07`-specific.
- A wider realization menu moves **4 rows, 0 worse**. The MECHANISM generalises; the specific
  realizations do not.

## THE RESULT THAT MATTERS: cheap realizer diversity beats expensive realizer diversity

| realizer family | dev Δscore | cost |
|---|--:|---|
| AMD option points on `G'` `{(false,-1),(true,2.5),(true,20)}` | −0.000059 | 3 ordering passes over `G'`; **+0.38 s on `edgecross24-115` for ZERO gain** |
| **seeded MCS, k=2** | **−0.000316** | 2 linear sweeps; no measurable cost on the same row |

**Seeded MCS is 5× the value at a fraction of the cost.** MCS repeatedly takes a vertex of
maximum visited-neighbour count; on a chordal graph *any* such choice is a valid PEO, so the
tie-break is free to vary and each distinct tie-break is a genuinely different realization.
It is the cheapest PEO producer available — one linear sweep, no supervariable/absorption
machinery — which is exactly why it is affordable where AMD is not.

Saturation is immediate: **k=2 captures the whole effect** (k=2 −0.000307, k=8 −0.000308 as a
marginal on top of the AMD family), so two seeds ship. Movers: `pooling_sppa9tp` (n=5040,
density 24) **−0.126 ln** — a 12% flop cut on one row and essentially the whole win —
plus `crudeoil_pooling_dt2` −0.0015 and `mpbp_48` −0.0007.

## GRADER-FAIL AUTOPSY: gate a hazard by ITS OWN covariate (submission 82b7baa9)

The AMD-family version was submitted and **FAILED the grader's cap on a hidden matrix ~8 s
into the run** — i.e. an early, SMALL matrix. It was gated on `(n, nnz, density)` of `A`.

> **The cost covariate is `nnz(G')`, the DESCENDED graph — not `nnz(A)`.** `G'` is a chordal
> completion, so its size is not bounded by `A`'s: a small sparse matrix can complete into a
> huge one. Measured on dev, `ri.len()` spans **108k to 3.09M** over rows whose `nnz(A)`
> spans 30k–894k, with no usable correlation. Gating on `A` therefore predicts nothing.

Fix: gate on `ri.len()` — the realized size of `G'`, known at runtime — so the bound is a
**hard cost bound rather than a prediction**. `REALIZE_MAX_GPRIME = 400_000` admits every
measured winner (`pooling_sppa9tp` 396 982, `mpbp_48` 385 518, `crudeoil_pooling_dt2` 326 444)
and excludes the payers (`crudeoil_lee4_06` 521 900, `crudeoil_pooling_dt3` 875 230,
`arki0013` 1.34M, `pooling_sppc3pq` 3.09M), which contribute ≤ −0.00025 ln between them.

**But the `G'` bound alone did NOT make the AMD family safe** — `edgecross24-115`
(G' = 289 520, inside the bound) still paid +0.38 s for identical flops, because an AMD pass's
cost also scales with `n` and structure, not with `|E(G')|` alone. That is what killed the AMD
family: a bound on `|E(G')|` bounds an MCS sweep but not an `amd_order` call. MCS's cost IS
linear in `n + |E(G')|`, so for it the bound is genuine.

## Method errors worth not repeating

1. **Diffing on flops and reading timings only for flops-CHANGED rows.** `chimera_selby-c16-02`
   was unchanged in flops and regressed in time — invisible in that view, and it produced a
   local cap FAIL. Audit timings on EVERY row.
2. **Trusting single-shot timings on a loaded box.** With unrelated Xcode builds at load 10–28,
   whole-corpus dumps show ±0.9 s swings on rows whose flops are identical, and *the
   unmodified frontier tree itself fails the cap* (three different rows across three runs:
   `rsyn0805m03m`, `chimera_k64ising-02`, plus mine). A red cap result on a loaded host is
   evidence about the host. Use targeted min-of-N per row, and a PURE-tree control run.

## Verification

- 3 better / 0 worse; predicted dev 0.812306 → 0.811990 from the exact flop census.
- Targeted min-of-5 on the cost-suspect rows: `edgecross24-115` 0.561 s vs pure 0.658 s (no
  increase); `pooling_sppa9tp` 0.356 s.
- Added work is analytically bounded: 2 MCS sweeps, each O(n + |E(G')|) with |E(G')| capped at
  400k by a runtime check, plus 2 `ws.flops` passes on `A` — milliseconds.
- 66 self-checks green incl. enforced `time_cap`.
- Deterministic: fixed splitmix64 stream per seed, `seed == 0` reproduces `minl_mcs_peo`
  byte-for-byte; no wall-clock, no env in production.
- Score-safe by construction: strict `ws.flops` admission against the AMD-anchored incumbent.

## Open leads

- Seeded MCS at the OTHER three realization sites (MINL, watcher-MINL, ORBIT all call
  `minl_mcs_peo` alone). The AMD version of that widening measured −0.000013 marginal for
  +0.56 s and was cut — but MCS is orders cheaper, so the trade may invert. Untested.
- Raising `REALIZE_MAX_GPRIME` for MCS specifically: the bound was sized to protect the AMD
  family that no longer ships. `crudeoil_pooling_dt3` (G' 875k) and `crudeoil_lee4_06`
  (G' 522k) are cheap for MCS and were measured positive (−0.00025, −0.00014).

## Negative worth not repeating: DETERMINISTIC degree tie-breaks add ~nothing

Having found that seeded (random) MCS tie-breaks pay, the obvious follow-up is a *structural*
tie-break: among the max-cardinality candidates take the smallest- or largest-degree vertex
in `G'`. Measured as a marginal on top of the shipped configuration: **2 better, 1 worse,
−0.000028** (`crudeoil_pooling_dt2` −0.0045, `mpbp_48` −0.0006, but `procurement1large`
+0.0007 from downstream reseeding). An order of magnitude below the bar, and not monotone.
Note `28ddfc0` ships a *low-initial-degree* MCS variant in pinned cells, so the rule is not
worthless — it just adds nothing on top of seeded diversity at the general gate. The
realization axis responds to DIVERSITY of tie-break, not to any particular clever rule,
which is the same lesson 0010's "diversity beats quality" recorded for selectors.
