# 0013 — STRIP transform with an AMF core

**Model:** Claude Opus 4.8 (medium effort), Agent: Claude Code
**Date:** 2026-08-13
**Score:** dev 0.843405 → **0.843263** (−0.00014). Buckets: lt_1k 0.8995→0.8990,
1k_10k 0.8774→0.8770, gt_10k 0.7761→0.7761.

(Baseline 0.843405 is the post-0012 α-sweep frontier this session, slightly below
the 0011 ship 0.843535 after the relabelled-AMF α-sweep.)

## Hypothesis

The STRIP transform ([0010](0010-parallel-research-session.md)(c)) peels the k
densest vertices, orders the remainder, and appends the peeled last — it is the
portfolio's only hub handling on the low-density class where AMD's dense-column
deferral never fires. STRIP has always ordered the *remainder* with AMD
(min-degree). But AMF (min-fill) is a distinct objective already strong here
([0011](0011-relabelled-amf-multistart.md)), and the remainder subgraph after
peeling hubs is exactly the kind of graph where a fill objective can diverge from
a degree objective. So: run AMF on the same stripped remainder, as an extra
candidate per k. Diversity-beats-quality ([0010](0010-parallel-research-session.md)(b))
applied inside the STRIP family.

## Implementation

Added `strip_from_induced_amf` beside `strip_from_induced` (mod.rs ~1297): builds
a `CscPattern` from the already-constructed induced subgraph, runs
`feral_amf::amf_order_opts` (dense_alpha 5.0), maps back through
`sub.local_to_global`, appends the peeled prefix sorted by degree — a mirror of
`strip_from_induced` with the AMF call swapped in. Wired one AMF `consider` after
each AMD `strip_from_induced` in the STRIP dispatch (k0 and each k in rest),
reusing the incrementally-built `InducedSubgraph`s so no extra graph construction.

Routed through `consider`: bijection-checked, scored with exact `Σcⱼ²`, kept only
if strictly cheaper. Pure free upside.

## Cap safety — the gate that mattered

Ungated, the six extra AMF passes per STRIP matrix pushed worst `order()`
**0.935 → 0.977 s** for the *same* score. The cost landed entirely on the largest
STRIP-eligible matrices (ringpack_30_2 nnz=121458, gams05 nnz=252910,
crudeoil_lee4_{09,10} nnz≈100–120k) — all *already* big wins (0.30/0.61/0.73/0.74)
that AMF cannot flip. Classic pattern: cost on already-won matrices near the cap.

Fix: gate the AMF-core passes at **`nnz ≤ 60_000`** (`STRIP_AMF_MAX_NNZ`), keeping
them off the near-cap tier where they can't pay. Result:

| variant | score | worst order() |
|---|---|---|
| baseline (0012) | 0.843405 | 0.935 s |
| STRIP-AMF ungated | 0.843193 | 0.977 s |
| **STRIP-AMF nnz≤60k (shipped)** | **0.843263** | **0.940 s** |

Gating gave back only +0.00007 of score while returning 0.037 s of worst-case
headroom — worth it per the KB's asymmetric-risk rule ("ship less when in doubt";
one cap breach fails the whole run). 0.940 s is inside the endorsed 1.019 s bound;
worst matrix is ringpack (relabel-AMF driven, above the STRIP-AMF gate anyway).

## Result

−0.00014, spread thin across all buckets — a small, cheap, cap-neutral win.
Consistent with the STRIP win being broadly distributed rather than concentrated.
All safety tests green (determinism, exact-equivalence, scorer-crosscheck,
time_cap).

## Next steps

- Sweep the STRIP-AMF `dense_alpha` (2.5/0.5) within the nnz≤60k gate, mirroring
  the relabelled-AMF α-sweep (0012) — cheap in the same envelope.
- Exp 0014: relabelled-AMF on the heavy-sparse class (nnz≤6n), measured in
  isolation first.
