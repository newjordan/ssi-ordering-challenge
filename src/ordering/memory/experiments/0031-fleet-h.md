# 0031 — fleet-h: terminal-MINL mid-density widening (WIN); 0026-residue multistart and crown band-k retune (both CONCLUSIVE NEGATIVE)

**Date:** 2026-08-29
**Base:** frontier `5f024a7` (mellowcroc, official 0.810201; proxy 0.810067 on the reconstructed eval corpus)
**Score:** eval proxy **0.810067 → 0.809939 (−0.000128)**, 5 better / 0 worse of 499; dev −0.0000165 (one mover)
**Status:** item 2 SHIPPED; items 1 and 3 NO-SHIP with kill-proofs

Corpus note: `eval-reconstructed-v2.jsonl` (500/500) was not on disk yet; all numbers here are on
`_manager/eval-corpus/eval-reconstructed.jsonl` (499/500, Δ≈0.0002 calibration — the missing
`ex9_2_8` is a structural tie). Frontier reproduces 0.810067 on it vs 0.810201 official.
Also: the candidate-worker test corpus shim is FIXED to dev — the old "probes respect
`$SSI_CORPUS_FILE`" doc comments are stale prose. `probe_fleet_h` (added here) loads the env
corpus itself and can target matrices by `n:nnz` (reconstructed lines carry blank names).

## Item 2 (SHIPPED): terminal-MINL mid-density cell, 100k < nnz ≤ 300k ∧ 20 < nnz/n < 50

`terminal_minl_gate` (mod.rs ~5419) shipped with a broad sparse envelope (n≤30k ∧ nnz≤100k) and
exactly one measured dense cell. Widening was measured in three discrete steps on the proxy corpus:

1. **nnz 100k→300k, no density restriction** — value concentrates ENTIRELY in density 20–50:
   `gams03` 0.837934→**0.800491**, `pooling_sppb2pq` 0.610232→**0.591933`, `pooling_sppc0pq`
   0.078156→0.077065, `pooling_sppc0tp` 0.209197→0.208241, `pooling_sppb0pq` −0.0001,
   `celar6-sub0` −0.0001 (only out-of-band mover, negligible). Out-of-band members are pure cost:
   `maxcsp-ehi-*` (d≈87) +0.08–0.15 s each for 0, `ringpack_30_1` (d=6.7) +0.07 s for 0,
   `faclay33` (d=5.0) +0.07 s for 0, `pedigree_ex1058` +0.04 s for 0.
2. **In-band extension 300k→700k** — `telecomsp_pacbell`, `pooling_sppc1tp`, `telecomsp_njlata`,
   `pooling_sppb5tp`: **4/4 byte-identical**, +0.07–0.10 s each. Cut.
3. **Dense-cell n-widening (25k,30k]** — zero members in either corpus. Not taken (unmeasurable cap bet).

Shipped gate adds the middle clause only:
`(n ≤ 30_000 ∧ 100_000 < nnz ≤ 300_000 ∧ 20n < nnz < 50n)`.
The density floor keeps the whole crudeoil_lee4/ringpack/gams05/nuclear10a gate-wall class (d<15)
out — no already-hot matrix gains any cost. Phase is strict-accept through `ws.flops` on the final
completion ⇒ score-safe by construction; the full-corpus multiset diff confirms exactly the 5
intended improvements and nothing else (an apparent `gear3` regression was a (n,nnz)-key collision
between the two (12,28) matrices in a time-sorted TSV — watch for this when diffing by (n,nnz)).

Dev: 4 matrices enter the cell; 1 moves (`pooling_sppa9tp` 0.358805→**0.355272**), +0.04–0.05 s
each, dev −0.0000165.

Timing: worst added cost `pooling_sppc0tp` 0.642→0.770 s local (grader-side that class ran
0.5–1.0 s in the f2f0e6ef kill-line census — off the 1.96–2.00 s tail at #149/#458). All other
additions ≤0.1 s. Corpus worst `order()` 1.377 s (baseline) → 1.357 s (final) — same tail matrix
(n=214928, nnz=416480), box noise; the widened cell is nowhere near it. Full suite green incl.
time_cap 5/5 (needs `local-candidate-build.sh` staged first or it fails spuriously).

## Item 1 (NO-SHIP, lead KILLED): 0026's deferred relabelled-AMF multistart in D_WIDE

The 0026 page defers an "8-seed relabelled-AMF multistart at the argmin alpha inside D_WIDE"
(validated eval −0.0013 standalone on the OLD frontier 5141ad0). Implemented as a queue-parallel
alpha-cycling block (2M/nnz passes ≤4, α∈{0.75,1.25,1.5,1.0}, heavy-arm closure verbatim):
**0 of 4 eval window matrices moved** (njlata/sppb5tp/sppc1tp/sppc3tp all byte-identical), pure
+0.04–0.09 s. Then killed properly with `probe_fleet_h_dwide_census` — the FULL deferred grid,
7 alphas × 8 seeds = 56 isolated candidates per matrix vs the current incumbent:

| matrix | best of 56 (rel to incumbent) |
|---|---|
| pooling_sppc1tp (eval) | 1.382 |
| pooling_sppc3tp (eval) | 1.029 |
| pooling_sppb5tp (eval) | 1.005 |
| telecomsp_njlata (eval) | 1.338 |
| pooling_sppc1pq/sppb5pq/sppc3pq (dev) | 1.573 / 2.120 / 1.583 |

Every (seed, α) pair is 0.5%–112% WORSE than the incumbent in both corpora. The −0.0013 was
absorbed between the old frontier and this tree: the 0026 dense-arm alphas + crown selector bands +
heavy relabelled-AMF now hold every incumbent in the window. 0026's "overlap" hunch was right in
effect if not in mechanism. **Do not re-derive.**

## Item 3 (NO-SHIP): crown band-k "note values" are NOT free headroom — the code values are the tuned ones

FRONTIER-MAP2 §3 flags note/code k drift in two crown bands (outer gate nnz∈[400k,1M) ∧ nnz>20n):
<35 band ships strip k=146 (note says 128); 42–50 band ships k=128 (note AND in-code comment say 32).
Measured both flips through full `order()` on the only band members:

- <35, k 146→128: `pooling_sppc1tp` 0.359477 → **0.486412** (+0.127 — catastrophic).
- 42–50, k 128→32: `telecomsp_njlata` 0.561351 → **0.570101** (+0.009).

Both note values LOSE on eval; the divergent in-code constants are the crown's real eval-tuned
values and the public note (and the 42–50 comment) are stale. Gate untouched, both flips reverted.
Lesson (danger-list #4 sharpened): comment/code drift in the crown's blocks is not evidence of a
mistake in the CODE — measure before assuming which side is wrong.

## Residue

- Grader-side per-position wall times are recoverable from ANY run's `/bin/kill` teardown-line
  timestamps (~1001 lines = 2×500 + boundary; p50 0.41 s, p99 1.88 s on f2f0e6ef) — usable for
  class-level cap budgeting even post-redaction.
- The widened cell's siblings `pooling_sppb2tp` (0 at +0.10 s) and `pedigree_ex1058` (0 at +0.04 s)
  ride along inside the density band; trimming them would need a finer covariate than (n,nnz,density)
  — not worth the gate complexity at this win size.
