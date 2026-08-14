# 0012 — Relabelled-AMF dense_alpha sweep

**Model:** Claude Opus 4.8 (medium effort), Agent: Claude Code
**Date:** 2026-08-13
**Score:** dev 0.843535 → **0.843405** (−0.00013).

## Hypothesis

0011's explicit next step. The base AMF family in the portfolio already runs a
`dense_alpha` sweep (mod.rs:1442) because α changes which vertices AMF treats as
dense and thus the ordering it produces. But the *relabelled*-AMF multistart
(0011) ran only the base `dense_alpha=5.0`. Sweeping α on the relabelled graph is
another axis of the diversity-beats-quality play (0010b): each (α, seed) pair is a
distinct min-fill candidate for one AMF pass.

## Implementation

Replaced the single-α relabelled-AMF loop with a 3-variant α-sweep:

```rust
const RELABEL_AMF_MAX_NNZ: usize = 130_000;
const RELABEL_AMF_ALPHA_MAX_NNZ: usize = 60_000;
let amf_variants: [(f64, usize); 3] = [
    (5.0, RELABEL_AMF_MAX_NNZ),
    (2.5, RELABEL_AMF_ALPHA_MAX_NNZ),
    (0.5, RELABEL_AMF_ALPHA_MAX_NNZ),
];
```

The base α=5.0 keeps the full nnz≤130k envelope; the two added α's (2.5, 0.5) are
gated tighter at nnz≤60k so the extra passes stay off the near-cap tier (same
asymmetric-risk reasoning as everywhere else). Same seeds, same `relabel_restarts`
budget, same `consider` free-upside path.

## Result

−0.00013, cheap per-matrix diversity within a winning family. Worst order() ~0.935 s,
inside the endorsed 1.019 s envelope. All safety tests green.

## Next steps

- Push α diversity into the STRIP-AMF core (0013) too.
- Heavy-sparse relabelled-AMF (0014).
