# 0036 — the terminal REALIZATION step generalises: a sparse-cell AMD realization family (WIN, dev −0.000059, 2/0)

**Date:** 2026-08-29
**Base:** frontier `3f497f6` (vibecodooor `aa23179`, official 0.791285; dev 0.812602), then
re-applied cleanly onto `8c2ce41` (official 0.789898)
**Score:** dev **0.812602 → 0.812543**, fill 0.932155 → 0.932137; reproduced byte-identically
on two scored runs
**Status:** SHIPPED

## The lead, and why it was live

`aa23179` introduced a genuinely new mechanism and stated its own generalisation in the note:
the completion-lattice descent *chooses a chordal completion*, and the realization step then
*chooses a PEO/tie-break basin for that graph* — two different problems, and "MCS and AMD do
not exhaust those basins". They shipped it behind a one-matrix cell (`gabriel07`, worth
0.5454 → 0.4537 there). **A mechanism that new, swept on one row, is the highest-value thing
on the board** — unlike the selector/gate seams, which 0034 proved saturated.

## Census (probe_flops_dump, test-only, in-tree)

Per-row `name → final flops` dumps, diffed process-to-process (no mid-run env mutation).
The tree already carries vibecodooor's `SSI_TERMINAL_ALL` hook, which forces the narrow
realization cell true everywhere; I added `SSI_TERMINAL_MENU` for menu families.

**First result, decisive for direction:** turning on the SHIPPED extra realizations
(`AMF(-1)`, relabel-767+`AMF(-1)`) for every terminal-gate row changes **zero** dev rows —
that pair is genuinely `gabriel07`-specific. But a WIDER menu moves **4 rows, 0 worse**. The
mechanism generalises; the particular realizations shipped for one eval row do not.

Family attribution (each run alone):

| family | wins |
|---|---|
| AMD option points `{(false,-1), (true,2.5), (true,20)}` | `crudeoil_pooling_dt2` −0.0081, `ct3`, `dt3` −0.0001 |
| AMF alpha grid | `ct3` only (same basin the AMD family reaches) |
| relabelled realizations (4 seeds × {AMF(-1), AMD}) | `chimera_selby-c16-01` −0.0015 |

The AMD family carries the value and is the cheapest. Shipped alone.

## Gate (evidence-derived, not tuned to winners)

`n <= 30_000 ∧ original_nnz <= 100_000 ∧ original_nnz < 10n`

The first two are the terminal gate's own first clause; `nnz < 10n` is the discriminator.
Every measured winner is sparse (density 4.1–5.4); the rows that pay time for nothing are
exactly the excluded ones — `pooling_sppc3pq` (density 38.6, **+0.42 s**, zero gain) and
`arki0013` (n=44 909, **+0.23 s**, zero gain). `crudeoil_pooling_dt3` (n=30 660) is
deliberately sacrificed: its gain was −0.0001 ln and widening the n ceiling re-admits the
`arki0013` cost class.

Production movers: `crudeoil_pooling_dt2` −0.00813 ln (gt_10k, weight 0.40 — this is
essentially the whole win) and `crudeoil_pooling_ct3` −0.00292 ln.

## THE COST MODEL — cost one local cap FAIL, and it is the durable output

Widening the menu inside the sparse cell (11 alphas × {AMF, AMD} + 16 relabel seeds × 6)
adds only `chp_shorttermplan1a` (−0.0018) while taking `crudeoil_pooling_dt3` from 0.60 s to
**4.13 s**. Restricting the wide menu to `n <= 5_000` — where the census showed the added
time *inside run-to-run noise* (`ct3` 1.19→1.08 s, `chp_shorttermplan1a` 0.76→0.72 s) — still
**FAILED the enforced cap** on `chimera_selby-c16-02` (n=2031, nnz=10878).

> **A realization candidate is FAR more expensive than an ordinary portfolio candidate,
> because the realizer runs on the DESCENDED GRAPH, not on `A`.** The descended graph is a
> chordal completion, so `nnz(G') >> nnz(A)` (often by 1-2 orders of magnitude): one AMD/AMF
> pass there costs what a pass on `A` never would, and each relabelled realization also
> permutes `G'` itself. The caller's `ws.flops` pass is comparatively minor (it works on
> `A`). Measured: the wide menu costs `crudeoil_pooling_dt3` **+3.53 s for ~118 extra
> candidates ≈ 30 ms each**, at nnz(A)=152 210 where a scoring pass is a few ms.
> Budget realization candidates against `nnz(G')`, never against `nnz(A)`.

**Method error that hid it, worth not repeating:** I diffed on FLOPS and only inspected
timings for rows whose flops CHANGED. `chimera_selby-c16-02`'s flops did not change, so its
time regression was invisible. **Audit timings on every row, changed or not.**

Consequence for anyone extending this seam: since the cost is the REALIZER pass over `G'`,
a scoring-side pre-filter would not help. The unlock is to make realization itself cheaper —
realize on a REDUCED graph (the descended graph's clique tree / a supernode quotient rather
than `G'` vertex-by-vertex), or reuse one elimination structure across several tie-break
policies instead of re-running a full ordering code per option point. Only then do the alpha
grid and the relabelled realizations become affordable.

## Verification

- 2 rows better / 0 worse; two scored runs identical (0.812543 / 0.932137).
- Full-corpus timing audit, ALL rows: worst `order()` **0.907 s**; largest increases
  `popdynm25` +0.113 s (0.663→0.776), `clay0304m` +0.080, `chp_partload` +0.067. Nothing
  above 0.7 s grows >0.1 s; nothing approaches 1.0 s.
- 66 self-checks green incl. enforced `time_cap`.
- Deterministic: gate is a pure function of `(n, original_nnz)`; three fixed option points;
  the `SSI_TERMINAL_*` hooks are `#[cfg(test)]` only.
- Score-safe by construction: strict `ws.flops` admission against the AMD-anchored
  incumbent, so a bad realization is discarded — only time is ever at risk.

**Box-noise caveat:** this box ran at load average 5–31 (unrelated Xcode/Swift builds). The
*unmodified* frontier tree spuriously tripped the cap once on `rsyn0805m03m` at load 9.4 —
a red cap result on a loaded host must be re-checked before it is attributed to a change.
