# 0029 — 0028 composition + the dense low-alpha relabel complement (a32d51f5)

**Date:** 2026-08-29
**Base:** the 0028 composition (submission 1e1acf12 = f48a17f + 19b52f11 + bbebe613)
**Score:** dev unchanged (0.817692 → 0.817692); expected official −0.00006 on top of 0028
**Status:** submitted

## The idea

a32d51f5 graded 0.817208 on the parent `f48a17f` (−0.000060) with a single terminal block:
in the accepted dense low-alpha window (`400k <= nnz < 1M`, `nnz > 20n`) it probes four
deterministic relabelings (`relabel(n, seed)`, seeds 1..=4) of each of the two AMF basins
found by the 0026 alpha census (α = 0.75, 1.25) — eight `feral_amf::amf_order_opts` tasks in
two four-worker waves — and accepts through `parallel::accept` against the completed
pipeline's incumbent, so it cannot reseed a downstream phase or regress the exact objective.
It is the last block of `order()` and touches no other hunk, so it is disjoint from both
0028 deltas (rgreedy/RGSUB phases; TELOS multi-start plan).

## Measured

- Full harness on dev: 0.817692 → 0.817692, zero per-matrix changes. The dev corpus has no
  matrix in the window (the accepted 0026 arm was likewise dev-neutral and −0.0032 official).
- `probe_timing_and_score`: worst `order()` 0.888 s (chimera_mgw-c16-2031-01, nnz 16k —
  outside the window; run-to-run noise on this box is ±0.3 s). No window matrix on dev, so
  the added cost is not locally measurable; the graded a32d51f5 run's tail was #149 1.97 s /
  #458 1.96 s, the same shape as every other run of this round (b286d274: #149 1.98 s,
  #458 1.85 s), i.e. the block's cost on the hidden dense-heavy class fits inside the
  existing envelope on that runner.

## Risk

Pure hidden-side bet on the heavy tier, priced at −0.00006. If this fails the cap while
1e1acf12 passed, the attribution is unambiguous: revert this block.
