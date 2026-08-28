<!-- 0026 -->
# 0026 — Wall-clock guard on the second TELOS descent: cap-safety after ORD-0025's hidden-matrix timeout — SHIPPED the guard, but measurement says DO NOT SUBMIT as-is (two independent blockers)

- **Date:** 2026-08-29
- **Score:** on this (loaded) box, flops-based **0.842177** (baseline 0.842223,
  0025 tree 0.841594); deterministically bounded in **[0.841594, 0.842223]**
  and grader-speed-dependent (see below)
- **Status:** **CAUTION / PARTIAL.** The wall-clock guard is implemented per
  the ORD-0026 plan, builds, 35/35 green, and is cap-safe against the ORD-0025
  timeout by construction. But two measured facts make the pure wall-clock
  guard unsafe to submit: (1) it **erases the main win** (procurement1large
  reaches the descent gate at ~1.08 s even clean, far above the 600 ms guard,
  so its second descent is skipped and gt_10k reverts to ~baseline); (2) it
  **conflicts with the grader's two-run determinism gate** (`main.rs:369`,
  "nondeterministic ordering (two runs differ)") on any winning matrix whose
  grader-elapsed straddles 600 ms — `nuclear25a` is the dev exemplar. The
  deterministic `(n, nnz)`-covariate gate recommended below avoids both.

## The failure this responds to (ORD-0025 grader FAIL)

ORD-0025 (0.842223 → 0.841594, first scoring win since 0019) added a second
TELOS descent from a distinct runner-up seed, gated `nnz <= 100_000` AND
relative-flops-gap `> 0.001`. The official grader FAILED it:

```
hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed
```

Root cause: `nnz <= 100k` is a **cost proxy, not a time bound**. The hidden
eval corpus carries a dense cost class dev entirely lacks (experiment 0005:
eval `nnz/n` up to **497** vs dev's densest heavy **38.6**). A dense matrix
small enough to pass the 100k nnz gate ran the second descent and blew the
**2.0 s** cap. Our own ORD-0025 win matrix `procurement1large` is `nnz/n=3.8`
(sparse); the killer is dense. This is a re-run of 0005's two rules — *gate a
hazard by its OWN covariate*, and *budget additions against an UNKNOWN load*.
The local box never saw it because `src/main.rs` had carried a local-only 8 s
cap patch (now reverted to origin's 2.0 s by the non-editable-path restore, so
the local harness ALSO enforces 2.0 s again).

## What was implemented (the ORD-0026 plan, verbatim)

`src/ordering/mod.rs`:
- `let order_start = std::time::Instant::now();` as the first statement of
  `order_with_options` (there was no pre-existing per-call start time), so the
  guard measures the FULL real time the call has spent when it reaches the
  runner-up-descent decision.
- `const SECOND_DESCENT_TIME_GUARD: std::time::Duration =
  Duration::from_millis(600);` (= 0.30 × the grader's 2.0 s `TIME_CAP`). The
  doc comment references the grader's **2.0 s** value literally, NOT the local
  patched cap, per the plan.
- The runner-up `seed_count` gate gains a fourth conjunct,
  `order_start.elapsed() < SECOND_DESCENT_TIME_GUARD`, ON TOP OF the existing
  `nnz <= TELOS_TOPK_MAX_NNZ`, `second_perm.is_some()`, and gap conjuncts
  (defense in depth — all four must hold).
- Test-only `TELOS_TOPK_ELAPSED` thread-local captures
  `(elapsed_at_gate, nnz_gap_gate_passed)` for the probe.

`src/ordering/probe.rs`: `probe_telos_topk` now prints a `GUARD` line per
gate-passing matrix — `elapsed_ms`, `guard_ms=600`, `margin_ms`,
`would_run_2nd`.

Rationale for wall-clock (the plan's, and it IS the semantically-correct cap
gate): elapsed self-adjusts to box speed AND density — a slow/dense matrix
burns more real time in the first descent and thus self-disqualifies the
second, so total added time is bounded to one descent-worth and ONLY on
matrices already fast, no matter how dense the hidden corpus is. Neither `nnz`
nor `nnz/n` alone captures the `(dense × large) = expensive` interaction that
the cap actually punishes (e.g. `maxcsp-langford-3-11` is `nnz/n=44.9` — dense
— yet fast because `n=660` is tiny, so it SHOULD keep its second descent).

## Cap-safety evidence — the elapsed-margin table (measurement #2)

`probe_telos_topk` `ALL`-census, single rep, this box (see the caveat on load
below). Every matrix whose `nnz`+gap gate admits a second descent, sorted by
where it lands vs the 600 ms guard. `would_run_2nd = (elapsed < 600 ms)`.

**Gate-passers that CORRECTLY skip (elapsed ≥ 600 ms) — the cap-safety win:**

| matrix | n | nnz | nnz/n | elapsed (ms) | 2nd descent |
|---|---|---|---|---|---|
| `chp_shorttermplan2d` | 16364 | 52108 | 3.2 | 1260 | SKIP |
| `crudeoil_lee2_06` | 6418 | 34646 | 5.4 | 1268 | SKIP |
| `procurement1large` | 14416 | 41068 | 2.8 | **1079** | **SKIP** (was the win) |
| `mpbp_35` | 11120 | 40790 | 3.7 | 1006 | SKIP |
| `chimera_selby-c16-01` | 2031 | 10964 | 5.4 | 991 | SKIP |
| `crudeoil_li05` | 4225 | 14628 | 3.5 | 829 | SKIP |
| `mpbp_34` | 11556 | 40860 | 3.5 | 829 | SKIP |
| `crudeoil_pooling_ct3` | 2644 | 11426 | 4.3 | 762 | SKIP |
| `crudeoil_pooling_dt2` | 18742 | 75910 | 4.0 | 661 | SKIP |
| `faclay30` | 16678 | 82242 | 4.9 | 632 | SKIP |
| `rsyn0810m04m` | 4772 | 13836 | 2.9 | 612 | SKIP |
| `rsyn0815m04m` | 5456 | 15756 | 2.9 | 609 | SKIP |

The largest/densest gate-passers land at 0.6–1.3 s and skip — which is exactly
the class that would blow the grader cap if it also ran a second descent. A
dense HIDDEN matrix would be even slower → skip harder. The mechanism works.

**The five ORD-0025 winners (the ones whose skip changes the SCORE):**

| matrix | n | nnz | nnz/n | elapsed (ms) | survives on this box? |
|---|---|---|---|---|---|
| `graphpart_clique-20` | 80 | 1260 | 15.8 | 35 | ✅ (huge margin) |
| `st_e31` | 301 | 744 | 2.5 | 73 | ✅ |
| `maxcsp-langford-3-11` | 660 | 29646 | 44.9 | 246 | ✅ |
| `nuclear25a` | 1942 | 16030 | 8.3 | **527 … 924** | ⚠️ **BOUNDARY** |
| `procurement1large` | 14416 | 41068 | 2.8 | **1079 … 1628** | ❌ **KILLED** |

Max elapsed among all gate-passers: `procurement1large` 1079 ms (clean rep) /
1628 ms (loaded max-of-3). The three small winners have ≥250 ms of margin. Two
winners are the problem — see the two blockers.

## Blocker 1 — the guard ERASES the main win (measurement #1)

`procurement1large` was the whole point of ORD-0025 (−0.0529 single-matrix in
gt_10k, ≈ −0.0006 weighted, the biggest contributor). Its full `order()` runs
~1.4 s clean (0025's A/B) / ~1.9 s loaded, and it reaches the second-descent
gate at **~1.08 s even on a clean rep** — because the first descent + the whole
multistart/HC/ORBIT/PHARMAKOS pipeline all run before the gate. 1.08 s ≫
600 ms, so its second descent is skipped and it reverts to 0.634285 (from
0.581426).

Flops-based full-corpus score on this box (`probe_timing_and_score`, no cap so
it does not spuriously fail):

| bucket | count | geomean | vs baseline | vs 0025 |
|---|---|---|---|---|
| lt_1k | 147 | 0.8993 | = | = |
| 1k_10k | 108 | 0.8753 | −0.0001 | = |
| gt_10k | 45 | **0.7745** | ≈ baseline 0.7748 | 0025 was **0.7730** |
| **SCORE** | | **0.842177** | −0.000046 | +0.000583 (WORSE) |

`gt_10k` at 0.7745 (≈ baseline) is the fingerprint of `procurement1large`
skipped. The guard recovered ~93 % of the 0025 loss: 0.842177 is only
−0.000046 under baseline, vs ORD-0025's −0.000629. Raising the guard "just
enough" cannot fix this — procurement's 1.08 s clean elapsed exceeds even the
plan's 1000 ms ceiling. This is the plan's own "box too slow to judge" branch,
and it holds even on a CLEAN rep of this box: the win survives ONLY if the
grader is materially faster than this box (procurement's gate-elapsed < 600 ms
there). The guard's benefit is therefore contingent on grader speed, which is
invisible — the same asymmetric-risk trap 0005/0010 warn about.

## Blocker 2 — the guard breaks the two-run DETERMINISM gate (decisive)

`src/main.rs:279-377` runs `order()` **twice** per matrix (tags `a`, `b`) and:

```rust
if perm1 != perm2 {
    failed = Some(matrix_failure(..., "nondeterministic ordering (two runs differ)", ...));
    break;
}
```

A wall-clock guard makes the second-descent decision **timing-dependent**. For
any matrix whose second descent STRICTLY WINS (the 5 dev winners + unknown eval
winners) AND whose grader-elapsed straddles 600 ms, run `a` may run the second
descent (→ improved perm) while run `b` skips it (→ un-improved perm) →
`perm1 != perm2` → the WHOLE RUN FAILS. Zero-move gate-passers (41 of 53 on
dev) are immune — same perm either way — so the hazard is confined to *winning*
matrices near the threshold.

**Empirical, on dev:** `nuclear25a` (a winner) measured **527 ms** (ALL-census
rep) and **924 ms** (named max-of-3 under load) — straddling 600 ms — and its
second-descent decision actually FLIPPED between probe runs (skipped in the
GUARD probe, ran in the score probe, which is why 1k_10k read 0.8753 there).
Under the recorded ~1.6× run-to-run timing variance, a wide band of matrices
(census shows ≥8 gate-passers within ±60 ms of 600 ms) can cross the threshold
between the grader's two runs. One flip on any WINNING matrix fails the run —
the same round-burning outcome ORD-0025 already paid for once, just a different
failure mode (nondeterminism instead of cap). The ORD-0026 plan did not account
for this gate.

## Verification

- `bash scripts/local-candidate-build.sh` (with `SSI_ALLOW_UNSANDBOXED_WORKER=1`):
  clean build.
- `cargo test --release -p ssi-candidate-worker`: **35/35 green**, 40 ignored.
- Full harness `cargo run --release`: FAILED on `crudeoil_lee4_06` (n=10429,
  nnz=55492, nnz/n≈5) at the 2.0 s cap — a **spurious load artifact** (box at
  68 % CPU from unrelated apps). `crudeoil_lee4_06` is NOT a gate-passer (its
  nnz+gap gate never fires → zero second descents), so this breach is a pure
  baseline-under-load event affecting all versions equally, not ORD-0026. The
  memory's rule held: verify load < ~4 before any scored run; this box could
  not provide one.

## Recommendation for the next stage (ORD-0027)

Do **NOT** submit the pure wall-clock guard. Replace it with a **deterministic
`(n, nnz)`-covariate gate** — pure function of the matrix, so it passes the
determinism gate — that captures the same `(dense × large) = expensive`
hazard the grader's own message names ("gate expensive paths by BOTH n and
nnz"):

- Run the second descent only when `nnz/n <= D` (sparse) **OR** `n <= N_small`
  (tiny → fast regardless). A carve-out like `nnz/n <= 10 OR n <= 500` admits
  ALL of `procurement1large` (2.8, sparse — **the big win is preserved**),
  `st_e31`, `graphpart_clique-20`, `nuclear25a` (8.3), and excludes a plausible
  dense hidden killer (`n~5000, nnz~90000, nnz/n~18`). Only `maxcsp-langford-3-11`
  (nnz/n 44.9, n 660) is lost — a sub-0.0001 win.
- This is 0005's rule applied correctly: gate the density hazard by density,
  keep `nnz <= 100k` and the gap gate, and it is deterministic AND cap-safe AND
  preserves the win the wall-clock guard erases. Strictly better on all three
  axes measured here.
- If wall-clock semantics are still wanted, they must be made deterministic
  (e.g. a fixed op-count budget on the second descent's sub-orderers, checked
  against a matrix-derived threshold — never wall-clock across the two runs).

## Links

- The mechanism this guards: [0025](0025-distinct-runner-up-descent.md)
- The cap-asymmetry + density-covariate rules: [0005](0005-eval-fail-density-regate.md)
- "Ship less when in doubt" / green-cap-is-weak-evidence: [0010](0010-parallel-research-session.md)
