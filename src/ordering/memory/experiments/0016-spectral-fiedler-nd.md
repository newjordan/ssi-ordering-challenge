<!-- 0016 -->
# 0016 — Spectral (Fiedler-vector) nested dissection: a genuinely new family, worth zero

- **Date:** 2026-08-28
- **Score:** 0.842722 → 0.842722 (dev, 300 matrices; byte-identical in all three buckets)
- **Status:** loss / reverted — but it **answers** [open-questions.md](../open-questions.md)'s
  "does candidate diversity saturate?"

## Hypothesis

Two records pointed the same way. [0010](0010-parallel-research-session.md)b measured
that a **different** ordering beats a merely better one — four custom metrics, every
one worse than AMD standalone, together bought −0.0088 — and
[open-questions.md](../open-questions.md) records the near-AMD mass as
*method*-limited rather than depth-limited, with an explicit instruction to hunt
families the portfolio is **blind to** instead of better versions of families it has.

Inventory of what ships: AMD, AMF, METIS, Scotch, KaHIP, `nd_order` (BFS median
level) ×2 arms, `ndfm_order` (GGGP graph-growing), RCM, Sloan ×2, minimum-fill,
biconnected decomposition, the custom-metric sweeps, and the relabel multistart /
hill-climb / heavy-restart layer. **Every separator in that list cuts either
combinatorially or via a multilevel library partitioner. Nothing uses the graph's
algebraic structure.** Spectral bisection (Pothen–Simon–Liou) is therefore a real
blind spot, and it can disagree with all of them on the same graph — the exact shape
0010b says pays.

## What changed

New `src/ordering/spectral.rs`: recursive bisection splitting on the **median** of
the Fiedler vector (median, not zero, so the bisection stays balanced and the
recursion logarithmic), separator = smaller of the two edge-cut boundaries (matching
`ndfm_order`'s choice so the families stay comparable), leaves handed to
`amd_fill_induced` (matching `nd_order_impl`'s AMD-leaf arm).

Fiedler vector by **deflated power iteration** on `M = dmax·I − L`: shifting turns
the smallest eigenvalue of the Laplacian into the largest of `M`, and projecting out
the all-ones vector on every step removes the trivial null eigenvector, leaving the
second-smallest — i.e. Fiedler. Determinism came first, since the contract demands
it: fixed alternating-sign start vector (never random), fixed iteration count (never
a convergence tolerance, which could stop at a different step on a different
machine), and index tie-breaks on equal entries including the all-zero iterate.
Registered via `consider` under a deliberately tight `n < 40_000 && nnz < 60_000`
gate, so the best-of floor made it zero-downside on score.

## Result

**Exactly zero.** 0.842722 before and after, identical to six decimals in every
bucket (`lt_1k` 0.899476, `1k_10k` 0.876533, `gt_10k` 0.774798). The candidate was
never the argmin on any matrix.

Crucially this is **not** a gating artifact — the gate admitted **260 of 300**
matrices, including **88 of the 100 at ratio ≥ 0.995**, i.e. it ran across almost the
whole near-AMD mass this was aimed at. Worst `order()` 1.852 s → 1.829 s
(unchanged within the documented ~1.6× run-to-run variance; `ringpack_30_2` at
nnz=121458 is outside the gate entirely), so it is affordable — just worthless.

Validity was checked before pricing: bijection + determinism on a disconnected
graph, an **edgeless** graph (the iterate collapses to zero, so the index tie-break
decides the split), a 20×20 grid deep enough to really bisect, and a complete graph
(every cut is the whole boundary, firing the separator-swallows-subset guard).

## Why it lost, and what that answers

This is a direct measurement on the open question *"does candidate diversity
saturate?"*, which noted that 0010b's still-climbing 25-variant curve **was measured
against an AMD-only baseline, not the real portfolio**. Answer, at least at this
point on the curve: **yes, it has saturated.** A brand-new algebraic family, run on
260/300 matrices, never once beat the incumbent best-of. So "candidate generation is
the whole game" does **not** hold against what we actually ship — the portfolio's
existing separators already dominate wherever a separator can win at all, and adding
an orthogonal *method* is no longer sufficient by itself.

Taken with [0014](0014-ndfm-amd-leaves.md) (quality inside a covered family: zero)
this session produced two independent zeros from opposite directions — better version
of an existing family, and an entirely new family. The remaining live lead is
neither: it is [0015](0015-zero-return-time-attribution.md)'s observation that the
multistart spends 0.06–0.34 s on matrices where it provably changes nothing, so the
next thing to test is whether *reallocating* existing budget beats *adding* to it.

**Reverted** on the 0014 rule: zero gain with nonzero cost is strictly negative when
the cap is the scarce resource. The implementation and its wiring patch are preserved
outside the checkout at
`~/ssi-ordering-LEARNINGS/rejected-implementations/spectral-fiedler-nd.rs` (+
`spectral-wiring.patch`), since `yukon sync` would wipe anything left under
`src/ordering/`. If a future session widens the gate or wants a spectral separator
inside a *different* construction (e.g. as a relabeling source for the multistart
rather than as a standalone ordering), start from that file rather than rewriting the
eigensolve.
