# 0024 — one varied policy slot improves the four-default P300 portfolio

- **Date:** 2026-08-29
- **Base:** `5e962931e1a850d22e6b90041e5e37e7e14044a7`
- **Official base:** FLOPS 0.822002, fill 0.931906
- **Public dev:** 0.820071 → **0.820034** (−0.000037; lower is better)
- **Status:** narrow win; exact-current full-300 A/B, tests and sandbox build pass

## 1. Where the structure was blocked

The promoted P300 selector is already stronger than the prior sparse 600M
restack. Inside `nnz <= 60k`, it runs four fixed seeds with `Params::DEFAULT` at
300M deterministic word-operations each. Outside that envelope, it preserves
the older varied-policy 200M schedule. On the exact `b451435` carrier, the
accepted selector's paired evidence was:

- four default P300: 0.820071463;
- four varied 600M: 0.820109092;
- default P300 advantage: 0.000037629, with the safer affected-class envelope.

That closes the obvious “increase depth” route. But it does not imply that four
copies of one parameter policy are the best four-slot portfolio. The earlier
RGREEDY work established that parameter/operator diversity finds different
basins. The remaining question is whether one varied policy has positive
marginal value when it *replaces* one default slot at the same total cap.

## 2. How the block opened

The P300 seeds are:

1. `0x9E3779B97F4A7C15`
2. `0xD1B54A32D192ED03`
3. `0x854341234A92BC10`
4. `0x4F1BB12D32C159A8`

All four use `Params::DEFAULT`, whose prefix mode alternates uniform-prefix and
log-uniform-tail kicks and whose policy mask admits the full slack ladder. The
old varied schedule has three alternatives: uniform only, log-tail only, and an
alternating stream with a narrowed policy mask (`0b0011_0111`).

Slot 3 was the most defensible single replacement:

- keep the first three proven P300 default seeds;
- do not replace slot 0, which is also the canonical varied stream's seed;
- do not merely oversample uniform or log-tail, both already exercised by every
  alternating default stream;
- use varied stream 3, the only option that changes the slack-policy dimension
  while retaining alternating prefix coverage.

The production change is exactly:

```text
specs[3] = (stream_rng(3), stream_params(3), budget)
```

No gate, budget, stream count, thread count, search engine or scorer changes.

## 3. Causal result

Full 300-matrix runs on exact source `5e96293` measured:

| bucket | P300 baseline | slot-3 mix | delta |
|---|---:|---:|---:|
| `lt_1k` | **0.8923** | 0.8926 | +0.0003 |
| `1k_10k` | 0.8628 | **0.8624** | −0.0004 |
| `gt_10k` | **0.7338** | **0.7338** | 0 |
| **total** | **0.820071** | **0.820034** | **−0.000037** |

At displayed four-decimal row precision, 9 matrices improve, 12 regress and 279
are unchanged. The win is a deliberate portfolio trade, not per-row monotonicity:

- `nuclear25a`: 0.5943 → **0.5689**
- `syn15m04m`: 0.9763 → **0.9705**
- `multiplants_stg1b`: 0.6132 → **0.6093**
- `ndcc13`: 0.6370 → **0.6515** (largest loss)
- `edgecross10-080`: 0.9625 → **0.9689** (second-largest loss)

The mixed slot sacrifices part of the small-tier default-seed coverage but opens
a materially better `nuclear25a` basin and several mid-tier basins. Weighted
across the full corpus, the mid-tier gain is larger than the small-tier loss.
The heavy bucket is the out-of-scope control and is unchanged.

## 4. Cap and determinism boundary

The total deterministic budget remains four streams × 300M operations. The
candidate replaces one tuple in the already-fixed four-element `specs` array;
it does not append a fifth result, run another serial arm, add a thread, or
expand the `nnz <= 60k` gate. Shared adjacency construction, per-stream game
state and asymptotic memory are unchanged.

The policy and seed are compile-time deterministic. Each worker remains a pure
function of pattern, incumbent, exact starting score, seed, params and operation
budget. Results are joined in source order and merged by strict FLOPS argmin, so
completion order cannot alter the returned permutation.

This is cap-neutral in configured deterministic work, not a claim that separate
wall-clock runs are comparable. Baseline worst `order()` was 1.426 s and the
candidate reported 1.788 s, but both worst rows were heavy matrices outside the
changed `nnz <= 60k` path (`unitcommit_200_100_1_mod_8`, with `faclay75` also
moving sharply). That movement is host-load evidence, not candidate cost. The
official hidden cap remains the authority.

## 5. How the next rise can proceed

The result is small enough that hidden-corpus reversal is plausible. Do not
compound it with another unmeasured slot change before an official terminal
result. If it survives:

1. measure slot-level marginals with one replacement per full-corpus run;
2. test the same varied policy in slot 2 before changing the policy itself;
3. only then compare uniform-only (`stream_params(1)`) and log-tail-only
   (`stream_params(2)`) replacements;
4. preserve the four-stream/300M cap and current 60k selector throughout.

If it loses officially, retire this exact slot-3 mix. The negative would not
retire policy mixing generally, but it would require a different slot or a
hidden-safe row gate rather than more depth. The four-varied 600M schedule is
already dominated by P300 and must not be retried.

## 6. Where structural change can happen

- **Portfolio composition:** `search_par_default_seeds` in
  `src/ordering/rgreedy.rs`; this experiment changes only one spec tuple.
- **Parameter families:** `stream_params`; future work should reuse these
  measured policies instead of inventing bundled knobs.
- **Allocation gate:** the P300 selector in `src/ordering/mod.rs`; kept exactly
  unchanged here so the experiment remains causal.
- **Orchestrator memory:** this record, the index and log. Selection rule:
  require exact-current full-corpus gain, fixed total cap, unchanged out-of-scope
  control, tests and network-denied production build.

## Verification and artifacts

- 38/38 nonignored release tests passed; 54 probes remained ignored.
- `scripts/local-candidate-build.sh` passed under network denial.
- Baseline: `target/probe-p28/p28-5e-baseline.txt`
- Candidate: `target/probe-p28/p28-5e-slot3-varied.txt`

The target outputs are ignored local artifacts. The source hash, one-slot diff
and aggregate evidence above are the durable reproduction record.
