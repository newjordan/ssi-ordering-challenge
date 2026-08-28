# 0013 — Gate-covariate audit: three pure additions, and one instructive negative

**Outcome: WIN.** dev 0.843535 → **0.843129** (−0.000406). Worst `order()`
1.124 s → 1.077 s (unmoved within the ~1.6× run-to-run variance). All 55
harness tests green including the enforced time-cap test.

## The organizing principle

Every change here is a **pure addition** under the best-of floor. That matters
more than the size of the win: adding a candidate can only lower a per-matrix
ratio or leave it alone, never raise it. So the score is **monotone** in the
candidate set, and the only risk any of these carries is TIME. This also makes
the result verifiable — a pure-addition change that regresses a bucket is proof
of a bug, and it caught one below.

## What shipped

### 1. `AMF_MAX_N` 250_000 → 320_000, `AMF_SWEEP_MAX_N` 300_000 → 320_000

Both doc comments state AMF's cost driver is **nnz** ("a smooth ~1.4× of AMD's",
"all three AMF passes total only ~0.64 s" at nnz=1.38M), yet both carried an `n`
cap that was the *binding* constraint on the two largest, sparsest matrices in
the corpus. Classic wrong-covariate gate.

`acopf_case9241pegase_qcqp` (n=313,068, nnz=1,292,408, density 4.1) is the most
under-served matrix on the corpus — it exceeded `AMF_SWEEP_MAX_N` by 4.4% and so
was excluded from ALL THREE AMF regimes, receiving only the anchor plus one heavy
relabel restart.

Note a correction to the obvious reading: `faclay75` (n=272,878) was NOT excluded
from the sweep — it already received the regime-(c) α−1 pass. Only the α5 base
pass (gated by `AMF_MAX_N`) was new to it.

### 2. D_WIDE sparse arm — a documented win that was switched off

`mod.rs` claims WIN D's α0.5 pass takes `cont6-qq` from 0.8899 to 0.8484. But
that matrix received **neither** the base D block (`nnz=557,994` exceeds
`D_MAX_NNZ=400_000`) **nor** the D_WIDE recovery window (`D_WIDE_MIN_DENSITY`
demands `nnz > 20n`; cont6-qq is density 4.63). The timing census confirmed it
sitting at exactly 0.8899 — the claimed win had never been live.

D_WIDE gates the DENSE side of that nnz window. This adds the SPARSE side, using
`HEAVY_SPARSE_MAX_AVG_DEG` (`nnz <= 6n`) — the guard derived from the
`telecomsp_metro` grader FAIL in [0005](0005-eval-fail-density-regate.md), which
has held every heavy addition since. Only α0.5 is offered, not the dense arm's
α2.5: α0.5 is the pass with the measured win, and halving the passes keeps
`unitcommit` (the heaviest matrix admitted) well under the corpus worst case.

This was the single largest contributor: `gt_10k` 0.776113 → 0.775244.

### 3. Relabelled **no-dense AMD** multistart, `nnz <= 60_000`

The relabel trick is a property of the **shared quotient-graph machinery**, not
of `amd_order` specifically: degree buckets are built by an ascending-index scan
with LIFO head insertion and `select_pivot` takes the head, so among all
minimum-degree candidates the highest original index wins — for EVERY
`amd_order_opts` variant. `AmdOptions` exposes no seed, so relabelling is the
only diversity knob the entire family has.

Base ordering is the no-dense / non-aggressive variant the ROBUST block already
ships. Suppressing dense-column deferral puts hubs back into the normal
min-degree flow, so its restarts explore a **different basin** from the
relabelled-AMD multistart rather than resampling the same one — "diversity beats
quality" ([0010](0010-parallel-research-session.md)) applied within the anchor's
own family, the same move that produced [0011](0011-relabelled-amf-multistart.md).

Gated at `nnz <= 60_000`, BELOW the 100k–130k "gate wall" class
(`ringpack_30_2` nnz=121,458, `crudeoil_lee4_09/10`) that carries the global
worst `order()`, so it provably cannot move the worst case. Contributed
`1k_10k` 0.877442 → 0.877247.

### 4. `RCM/SLOAN/ND/NDFM_MAX_NNZ` 130_000 → 180_000

These four are the cheapest candidates in the file (O(nnz) BFS, "a few
milliseconds even at large n" by their own docs), gated on a premise that is now
**false** — see the systemic finding below. Stops at 180k deliberately to keep
`gams05` (252,910) and `nuclear104` (257,806), the 2nd and 3rd slowest matrices,
out.

## The negative: DO NOT "restore" the WIN F gate

`WINF_MAX_NNZ` is declared 130_000, but its `if nnz < WINF_MAX_NNZ` sits inside
a block already gated `nnz < 130_000`. **The condition is a no-op**, and its doc
comment claims the opposite — that `probe_winf_gatewall` measured all seven
variants "no-win" on `ringpack_30_2` and `crudeoil_lee4_10` and that the gate
"stops well below the gate wall".

Acting on that comment, this session set `WINF_MAX_NNZ = 100_000` to restore the
documented intent and reclaim ~0.09 s from the worst-case matrix. Measured
result: **dev 0.843535 → 0.843632, a REGRESSION of +0.000097**, entirely in
`gt_10k` (0.776113 → 0.776501). Since a pure addition cannot regress a bucket,
the removal was unambiguously the cause.

**The seven WIN F variants DO win on the gate-wall matrices.** The doc comment is
wrong, and the gate being an accidental no-op was load-bearing. Reverted to
130_000. Lesson: "restore the documented intent" is a hypothesis to measure, not
a cleanup to apply — and in a file this heavily annotated, a stale comment is
more dangerous than no comment.

## Systemic finding: the "slow tier" premise is refuted

Eleven doc comments justify their gate with one shared claim — *"`nnz < 130000`
keeps this strictly below the slow tier (`nnz >= 163816`), so it cannot move the
worst case."* Current timings refute it:

| rank | matrix | nnz | secs |
|---|---|---|---|
| 1 | `ringpack_30_2` | **121,458** | 1.124 |
| 2 | `gams05` | 252,910 | 0.973 |
| 3 | `nuclear104` | 257,806 | 0.942 |
| 4 | `crudeoil_lee4_10` | **120,632** | 0.891 |
| 5 | `crudeoil_lee4_09` | **101,792** | 0.839 |
| 9 | `nuclear10a` | 163,816 | 0.692 |

`nnz=163816` is literally `nuclear10a`, the slowest matrix circa 0002/0003 — now
**9th**. Six of the ten slowest now sit *below* 130k, and 279 of 300 dev matrices
have `nnz < 130,000` while consuming 79% of total corpus time. Time is not
monotone in nnz: `faclay75` (nnz 1.38M) costs 0.61 s while `ringpack_30_2`
(nnz 121k) costs 1.12 s.

**The 130k wall is where cost is MAXIMAL, not where safety begins.** Any future
gate reasoning should be re-derived against `ringpack_30_2` / `crudeoil_lee4_*`,
not against `faclay75` or `nuclear10a`.

## Results

| | dev score | lt_1k (147) | 1k_10k (108) | gt_10k (45) | worst `order()` |
|---|---|---|---|---|---|
| baseline (`a912d66`) | 0.843535 | 0.899524 | 0.877442 | 0.776113 | 1.124 s |
| + WIN F "fix" (REVERTED) | 0.843632 | 0.899524 | 0.877247 | 0.776501 | — |
| **shipped** | **0.843129** | 0.899524 | 0.877247 | **0.775244** | **1.077 s** |

`lt_1k` is unmoved to six digits, exactly as [0012](0012-exact-dp-small-n.md)
predicts — that bucket's bottom is proven optimal and no longer responds to
anything.

## Caveat on pricing

Dev and the graded corpus share essentially no matrices, so the dev delta is not
a forecast of the graded delta. What DOES transfer is the mechanism: every
shipped change is a pure addition, so the graded score can only improve or stay
equal. The risk being carried is entirely the time cap, and the worst case did
not move.

## Reproduce

```sh
cargo run --release --offline --locked -- --note "0013"
cargo test --release --offline --locked -- --ignored --nocapture --test-threads=1 probe_timing_and_score
cargo test --release --offline --locked
```
