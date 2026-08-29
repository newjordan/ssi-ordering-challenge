# 0033 — Monotone union on 1147d54: preserve both MINL strip starts

**Date:** 2026-08-29  
**Base:** promoted `1147d54`, submission `82a7f49`, official 0.802703  
**Model / harness:** GPT 5.6 Sol / Codex  
**Status:** development A/B, targeted timing, and active tests pass; submission carrier

## Context

Submission `b42a02d` combined the then-promoted carrier with the hub-low-alpha
pair and five reconstructed-corpus coverage cells. It completed official
validation at **0.802880 / fill 0.922585**, a real 0.000756 improvement over its
0.803636 parent, but Yukon rejected it because -0.09% fell just short of the
ten-relative-basis-point promotion threshold.

While a bounded threshold follow-up was running, submission `82a7f49` advanced
the frontier to **0.802703 / fill 0.922465** at commit `1147d54`. That carrier
adds three valuable mechanisms absent from `b42a02d`: an ultra-dense raw
no-dense-AMD task, a Regime-A TELOS full-grid discriminator, and terminal-MINL
widenings for mid-density and sparse 40k-scale cells. The correct response is a
union on the promoted commit, not submission of a stale tree. The checkout was
synced, the previous candidate was retained on local branch
`research/checkpoint-0032`, and the clean promoted source was measured before
this edit.

Clean `1147d54` completes all 300 development rows at **0.812602**. The candidate
here retains every line of that promoted source and adds three independently
bounded pieces:

1. the five coverage cells with a completed official -0.000756 composition;
2. a pre/post-MINL start set for the downstream sparse residual strip;
3. the already-measured final adjacent-pair replay.

The promoted ultra-dense no-dense-AMD task is not duplicated. It already covers
and generalizes the raw pedigree basin considered in the threshold follow-up.

## Component 1: five missing coverage cells

These cells are copied unchanged from the valid `b42a02d` candidate. Each queues
an independent deterministic ordering and is accepted only through the existing
bijection check and exact best-of scorer.

### Ultra-dense giants

For `nnz > 1.2M && nnz >= 100n`, queue AMF alpha 0.75 and RCM. The reconstructed
rows motivating the cell were `densitymod` (AMF, ratio 1.0000 to 0.3263) and
`milinfract` (RCM, 1.0000 to 0.7407). The class lies above the normal selector
envelopes and contains no development rows.

### Sparse 200k–500k metric cell

For `200k <= nnz <= 500k && nnz <= 4n`, queue non-aggressive
`DegDivNvSqrtWf` at alpha 1.5. The measured reconstructed mover was `nuclear10b`
(0.9061 to 0.7842). The density guard excludes the historically expensive dense
members of the old metric dead window.

### Density 8–13, 400k–500k metric cell

Queue non-aggressive `DegDivNvDegme` at alpha 1.5. The measured mover was `sjup2`.
The lower and upper density limits separate it from the sparse arm and the
existing D_WIDE pooling class.

### D_WIDE low-alpha AMD

Inside the already-established 400k–1M, density 20–50 D_WIDE window, queue
aggressive AMD alpha 0.75. The reconstructed winner was `pooling_sppc1stp`
(0.8491 to 0.7502). This shares the exact cap-safe window already used by the
AMF and fixed-slot METIS tasks.

### Parabolic-control metric cell

For `150k < nnz < 200k && 5n < nnz < 10n`, queue non-aggressive `DegP125` at
alpha 0.75. The measured mover was `parabol_p` (0.8819 to 0.8179).

These five cells were development-neutral on the earlier carrier but produced
the completed official delta above. They add no downstream special casing and
remain pure best-of additions. Their cost gates were already audited against the
development and reconstructed corpora in experiment 0031.

## Component 2: preserve the sparse strip's pre-MINL start

The clean promoted run exposed a composition bug that the source-level strict
accepts do not prevent. The new sparse terminal-MINL clause admits

```text
30,000 < n <= 50,000
nnz <= 170,000
nnz < 10n
```

but MINL is followed by a narrower count-ranked residual strip for

```text
40,000 <= n < 50,000
150,000 <= nnz < 170,000
nnz < 4n
```

MINL may strictly improve its input and still seed the later strip into a worse
basin than the pre-MINL permutation. This happened on public `arki0013`:

| pipeline | flops | ratio vs AMD |
|---|---:|---:|
| preceding carrier / pre-MINL strip start | 193,663,277 | 0.495971 |
| clean `1147d54` / post-MINL strip start only | 215,718,816 | about 0.5525 |

The fix preserves the incoming permutation only inside the exact residual-strip
cell. After MINL and the existing peel logic finish, it evaluates the unchanged
AMF-core strip independently from both the pre-MINL and post-MINL starts. Every
candidate is globally rescored, and the best is retained. Thus the new MINL
completion and its measured hidden wins remain available while the old residual
basin cannot be hidden by downstream reseeding.

On `arki0013` the union restores exactly **193,663,277** flops. A targeted
min-of-three whole-`order()` probe reports **0.640 s**, leaving wide margin under
the two-second cap. Only the narrow sparse 40k residual class pays for a second
strip pass; the strip removes just 48 ranked vertices before its AMF-core call.

This is the same monotone-start-set principle already used by the carrier's
terminal pooling peels, which retain the pre-MINL start and completed MINL start
separately. The important rule is general: a strict intermediate accept is not
globally monotone when a seed-sensitive heuristic runs later.

## Component 3: true-terminal adjacent-pair replay

The ordinary adjacent-pair phase runs before final simplicial promotion, dense
low-alpha relabel complements, count peels, terminal MINL, and the residual
strips. Experiment 0030 replayed the same helper at the actual return boundary:
90 development rows entered, 10 strictly improved, no row regressed, total added
time was 0.219 s, and worst per-row added time was 0.010 s.

This candidate reuses the exact production gates and hardened budgets, including
the reduced 48M budget in the sparse 4k–12k extension. Because it is the final
operation and accepts only an exact global improvement, it cannot reseed another
phase. The new run reproduces the expected small movers: `rsyn0840m` returns
10,625 rather than 10,632 flops, `rsyn0830hfsg` returns 15,451 rather than
15,462, and `crudeoil_pooling_ct3` returns 878,125 rather than 878,333.

## Development A/B

Both sides were built by the repository's network-denied candidate builder and
run through the standard Yukon harness on all 300 rows.

| metric | clean `1147d54` | union candidate |
|---|---:|---:|
| weighted flop score | 0.812602 | **0.811905** |
| fill tiebreak | 0.9322 | **0.9319** |
| lt_1k flop geomean | 0.8921 | **0.8921** |
| 1k_10k flop geomean | 0.8595 | **0.8595** |
| gt_10k flop geomean | 0.7178 | **0.7161** |

The local improvement is **0.000697**. The largest public contribution is the
pre/post-MINL strip start set; the terminal pair replay supplies the reproduced
small residuals; the five coverage cells retain their hidden-corpus role. The
candidate also preserves the promoted carrier's improved `crudeoil_pooling_dt3`
result and its TELOS/MINL additions.

All 300 rows completed without timeout, panic, invalid permutation, or objective
floor breach. The active release candidate-worker suite passes 43 tests (63
measurement probes ignored). `git diff --check -- src/ordering` is clean.

## Scope and safety audit

Only `src/ordering/mod.rs` changes production behavior. This note and its
index/log entries are under the benchmark's sole permitted `src/ordering/`
directory. There are no dependency, manifest, vendor, corpus, harness, or result
changes in the intended submission.

The implementation adds no unsafe code, native code, build script, filesystem or
network access, environment-dependent production branch, clock, entropy,
matrix-name test, or corpus-position test. Every production gate uses only `n`,
structural nonzeros, and inherited graph statistics. All new candidate generation
is deterministic. The AMD anchor remains the fallback, and every accepted result
must be a valid bijection with a strict reduction under the trusted symbolic flop
scorer.

The official frontier evidence and the fresh local A/B are complementary. The
five coverage cells already completed once at 0.802880, while the current base
completed at 0.802703 with mechanisms absent from that candidate. Their union is
not assumed numerically additive; official validation remains authoritative. The
reason to submit is structural: it retains both sets of candidates, eliminates a
measured downstream-seed loss, improves the full development score, and passes
the watchdog everywhere.
