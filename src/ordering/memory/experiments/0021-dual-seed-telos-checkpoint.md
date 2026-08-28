# 0021 — bounded dual-basin TELOS from a relabel checkpoint

- **Date:** 2026-08-29
- **Score:** full 300 aggregate intentionally not re-run under host load;
  focused `chp_shorttermplan2d` `0.514660 → 0.483650`
- **Status:** candidate, correctness- and worker-verified; official result pending

## Hypothesis

[0020](0020-telos-seed-sensitivity.md) isolated a non-monotone interaction:
the mid-band restart floor improved the exact upstream candidate pool on
`chp_shorttermplan2d`, but the better incumbent seeded TELOS into a worse basin.
TELOS reached `1_975_321` flops from the older, worse incumbent and only
`2_104_445` from the newer, better incumbent. If the pre-floor relabel basin is
retained as a second seed, the same deterministic TELOS descent can explore
both basins and keep the exact best.

This is a follow-on to the promoted restart-floor mechanism, not another
restart-budget increase and not a redraw. The causal change is the number of
distinct TELOS starting basins available inside one `order()` call.

## What changed

Only `src/ordering/mod.rs` changes executable ordering behavior; the other
changes are knowledge-base records under `src/ordering/memory`.

1. The shipped inline TELOS block was factored into `telos_descent`, whose
   explicit inputs are a permutation and its exact flop count. Its tier gates,
   round counts, sub-orderers, peel grid, exact scorer, bijection checks,
   tie-breaks, and strict-improvement rule are unchanged.
2. `relabel_restarts_before_floors` reconstructs the restart count before the
   mid-band and low-nnz floors, retaining the older heavy-sparse rule.
3. During the first `checkpoint_restarts` seeds of each existing relabelled AMD,
   AMF, and no-dense AMD loop, a second exact incumbent is retained. No extra
   relabel candidate is generated.
4. The normal incumbent runs TELOS exactly as before. If the retained
   end-of-relabel permutation differs from the normal pre-TELOS incumbent, it
   runs the same TELOS descent once more; only an exact strict improvement is
   merged.

The checkpoint is intentionally described as an **end-of-relabel seed**. It
does not replay hill climb and every other pre-TELOS phase from the historical
lower-restart configuration, so it is not a byte-exact reconstruction of that
older full pipeline. The focused result stopping at `0.483650` rather than the
historical `0.483081` is consistent with that narrower mechanism.

## Default-path and fatal gates

The second descent is admitted only when all of these hold:

```text
n <= 40_000
30_000 < nnz <= 150_000
nnz <= 6*n
max_degree <= floor(n/50)
current_restarts > checkpoint_restarts
checkpoint_permutation != normal_pre_TELOS_permutation
```

The structural predicate reaches exactly 21 of the 300 dev patterns. Their
largest `n` is `33_155`; their largest off-diagonal `nnz` is `91_016`. The new
`n <= 40_000` ceiling is load-bearing for hidden safety: TELOS performs an
`O(n log n)` rank plus multiple `O(n)` candidates and exact evaluations, while
an isolate-heavy matrix could otherwise satisfy the nnz/degree gate at roughly
the contract's maximum vertex count.

The `n=40_000` boundary keeps all 21 measured dev patterns but prevents the
new path from doubling TELOS on `n≈340k` isolate-heavy inputs. A byte-identical
checkpoint is skipped, avoiding a duplicate descent.

An early draft also stopped inserting relabel permutations into the exact-score
cache, matching an old scoping comment and reducing a possible memory tail.
That was removed before admission because it is an independent performance
carrier. This candidate preserves the promoted cache and scoring semantics so
the diff contains one mechanism.

## Result

Evidence is deliberately separated by authority.

### VERIFIED-CORRECT

- Network-denied Seatbelt release build of `ssi-candidate-worker`: pass.
- Locked/offline release parent build: pass.
- `cargo-deny 0.20.2`: present.
- Non-ignored candidate-worker suite: **36 passed, 0 failed**.
- Empty, singleton, edgeless, bijection, repeated determinism, and AMD-floor
  focused tests: pass.
- Dedicated boundary test: `n`, `nnz`, `max_degree`, and equal-restart edges all
  admit/exclude as intended.
- Independent source review found the extracted TELOS logic semantically equal
  to the shipped inline path and found no new correctness, determinism,
  bijection, or panic defect.

### LOCAL-DIRECTIONAL

The motivating matrix is active through normal `order()` dispatch:

| matrix | promoted recorded ratio | candidate ratio |
|---|---:|---:|
| `chp_shorttermplan2d` | `0.514660` | **`0.483650`** |

The candidate therefore recovers nearly all of the measured between-basin
loss and proves the second seed is not inert. No full-300 local aggregate was
used as an admission ticket. The exact comparison is local to the TELOS
boundary: later seed-sensitive phases can respond differently, so global
monotonicity is not inferred from their own strict-improvement rules. The 279
dev patterns outside the predicate retain the single extracted TELOS path.

Focused `order()` timing was measured sequentially while the host load remained
high, making these conservative directional observations rather than quiet-host
benchmarks:

| representative | reason | `order_ms` |
|---|---|---:|
| `chp_shorttermplan2d` | causal mover | 962 ms |
| `nd_netgen-3000-1-1-b-b-ns_7` | cohort max `n=33_155` | 613 ms |
| `mpbp_48` | cohort max `nnz=91_016` | 825 ms |

### WORKER-VERIFIED

Two native Seatbelt/watchdog checks ran outside an outer sandbox so the worker
was genuinely isolated rather than failing macOS nested Seatbelt setup.

1. `chp_shorttermplan2d` extended with isolated diagonal-only vertices to
   exactly `n=40_000` preserved off-diagonal `nnz=52_108` and
   `max_degree=37`. Both deterministic worker executions completed below the
   2-second per-matrix watchdog. Total trusted-command wall time, including two
   worker startups and scoring, was 3.54 s.
2. The exact 21-pattern activation cohort ran each pattern twice under the same
   native Seatbelt/watchdog boundary: all 42 worker executions passed with no
   timeout, panic, nondeterminism, or bijection failure. Total command wall time
   was 31.43 s. A paired run of the same population scored promoted `5141ad0`
   at `0.833047` / fill `0.953378` and the candidate at `0.831452` / fill
   `0.953240`. This is local falsification evidence, not a full-corpus or
   hidden-score prediction.

### NOT EVIDENCE

Passing `--help` to the benchmark binary once started a run because that binary
has no generic help parser. Inside an outer sandbox its attempted nested
`sandbox-exec` exited 71. This created no Yukon UUID and says nothing about
candidate performance. The one appended FAIL row was removed. CLI syntax is
now checked with `yukon run --help`; harness behavior is read from source.

## Why it is ready for an official slot

The mechanism is novel relative to the frontier, directly follows a measured
causal residual, reaches the default path, recovers its motivating regression,
improves the paired public activation cohort, and passes the worker watchdog
over that entire cohort plus the `n=40k` boundary. The official hidden corpus
remains the only performance authority; a terminal rejection will close this
exact bounded end-of-relabel dual-TELOS mechanism rather than being redrawn
unchanged.

## Follow-ups

- If official terminal is positive, inspect the new frontier residual before
  widening K or changing the gate. Marker-only or identical-code redraw has no
  score value for deterministic ordering.
- The prepared orthogonal candidate is boundary-conditioned biconnected local
  AMD: include a child block's parent articulation while finding its local
  order, then filter that articulation from the emitted owned vertices. Preserve
  the bridge fast path and do not change `biconn_heavy_order` in the same diff.
- A separate cache-scope memory carrier remains possible, but it must be
  measured and submitted only with an independent scoring mechanism.

## Links

- [0019 — restart floors](0019-mid-band-restart-floor.md)
- [0020 — TELOS seed sensitivity](0020-telos-seed-sensitivity.md)
- [best-of portfolio](../techniques/best-of-portfolio.md)
