# 0024 — retain varied RGREEDY only on a cap-safe small-density cell

- **Date:** 2026-08-29
- **Exact parent:** `4270fd72b68a22f7c7ca90bc40bb0ad8fae2b5f3`
- **Parent official score:** `0.821048`
- **Full-300 development score:** `0.819376` → **`0.819353`** (`-0.000023`)
- **Status:** strict exact-parent local win; 1 better / 0 worse / 299 same

## Decision

Replace PEP's fourth default-policy RGREEDY trajectory with the already-existing
deterministic varied trajectory only when all of these structural predicates hold:

```text
0 < n <= 512
5*n <= nnz
4*nnz <= 21*n        # nnz/n <= 5.25, with integer arithmetic
```

Every other pattern keeps all four accepted PEP defaults. The selected cell contains
four development patterns. Exactly one changes and it improves; none regress. The
candidate's eligible-row worst is 0.804 s and its whole-corpus worst is 1.336 s. This
is intentionally much narrower than the earlier broad varied-slot attempt that won
development score but was killed by the official hidden two-second cap.

## 1. What structure was blocked?

The default PEP portfolio runs four fixed seeds with identical `Params::DEFAULT`.
Changing one seed and proposal-policy mask can reach a basin those four correlated
trajectories miss: `stream_params(3)` retains alternating-prefix proposals while
narrowing the policy mask, and `stream_rng(3)` is a fixed deterministic seed. On the
old exact-698 development tree, replacing slot 3 everywhere improved aggregate score,
but the submission was killed on an unnamed hidden matrix. Equal operation counts do
not imply equal wall cost: proposal mix changes the fill states visited and therefore
the work per accounted operation. The official failure retires any broad reuse.

The earlier row-level result also showed why `n <= 1000` is not a sufficient fix. In
that whole small band the varied slot produced three printed gains and four losses;
the small-bucket geomean worsened and the projected score moved `0.819962` to
`0.820033`. The dominant loss was close in size to the best gain. A size ceiling alone
solves cap exposure but not portfolio quality.

The remaining opportunity was a small structural cell in which the alternate
trajectory is complementary rather than merely different. Replaying the old exact A/B
rows over deterministic `(n, nnz)` predicates found the density interval 5.0–5.25 at
`n <= 512`: four eligible rows, one gain, zero loss, prior eligible worst below 0.65 s.
No matrix name, corpus index, timing measurement, clock, or environment value enters
the production predicate.

## 2. How was the block opened?

`search_par_default_seeds` first creates the exact accepted array of four default
specifications. It computes `nnz` from `row_idx.len()` and applies the integer gate
above. Only when the gate is true is element 3 replaced by
`(stream_rng(3), stream_params(3), budget)`. The array remains length four, so
`PAR_MAX_THREADS.min(specs.len())` still runs no more than four workers. The same
per-stream budget is passed to all four specifications; there is no fifth trajectory,
second batch, or added search depth.

The multiplication uses saturating integer operations, although the explicit
`n <= 512` bound already makes overflow impossible in practice. This keeps the gate
total on arbitrary input and avoids floating-point density comparisons. Fixed seeds,
fixed parameters, a pure structural predicate, independent search state, and the
existing strict argmin merge preserve determinism.

The exact parent matters. While this experiment was running, source moved first from
the P35 validation commit `e97cb342` and then to `4270fd72`. The new leader removed
P35's narrow mid-band 800M floor and added top-K TELOS seeding, light/full-grid extra
descents, an alternate completion-descent seed, and a measured-zero hub gate-wall
trim. Those upstream phases can change the incumbent handed to RGREEDY. The small
slot change was therefore recreated on `4270fd72` and both sides were rerun; no score
from the stale e97 tree is claimed as current evidence.

## 3. Exact-4270 full-corpus result

Both release probes ran offline inside a network-unshared bubblewrap boundary and
scored all 300 development patterns.

| Metric | exact 4270 baseline | gated varied slot | Change |
|---|---:|---:|---:|
| weighted score | 0.819376 | **0.819353** | **-0.000023** |
| `lt_1k` geomean (147) | 0.8925 | **0.8924** | -0.0001 |
| `1k_10k` geomean (108) | 0.8625 | 0.8625 | 0 |
| `gt_10k` geomean (45) | 0.7322 | 0.7322 | 0 |
| printed row changes | — | 1 better / 0 worse / 299 same | monotone visible A/B |
| eligible-row worst | 0.703 s | **0.804 s** | below 1.15 s gate |
| whole-corpus worst | 1.564 s noisy/outside gate | **1.336 s** | below 1.45 s gate |

The sole changed row is `waterund11` (`n=160`, `nnz=828`), ratio
`0.7641 -> 0.7546`. The other three eligible rows are byte-score identical at printed
precision: `graphpart_3g-0244-0244` (`128/672`), `pointpack06` (`55/276`), and
`sporttournament08` (`31/156`). The candidate's slowest eligible row is the unchanged
graphpart row at 0.804 s; the improving row takes 0.593 s. The candidate's global
slowest row, `unitcommit_200_100_1_mod_8` at 1.336 s, is outside the gate and therefore
retains exact-parent code.

The baseline's 1.564 s maximum was also outside the gate (`acopf_case9241pegase_qcqp`)
and is treated as host-load variance, not as an acceleration claim. What the code can
support structurally is narrower: only four low-n patterns change policy; worker count
and budgets are identical; and the changed domain remains far below both the 1.15 s
affected-row limit and the hidden two-second cap. The candidate run itself also meets
the predeclared 1.45 s whole-corpus local bound.

## 4. Where can ascent continue?

The win demonstrates that policy diversity is valuable only conditionally. Future
work should extend the gate by independently justified structural cells, one at a
time, rather than widening the size ceiling. The next precomputed candidate is the
separate `800 <= n <= 850` and `4*n <= nnz <= 5*n` cell: the old attribution had one
gain and zero loss there, but it is not stacked here because one candidate must change
one mechanism and each added cell needs a fresh exact-parent full-300 A/B.

The broad negatives remain binding:

- `n <= 1000` varied slot is a measured aggregate loss and must not be retried.
- unrestricted slot 3 is retired after the official hidden-cap kill.
- a fifth stream is not cap-neutral under the four-worker executor; it is ignored or
  requires extra scheduling and work.
- P35's mid-budget density floor and the new leader's top-K TELOS changes are separate
  mechanisms. They must be restacked and tested independently, never folded into this
  slot-policy experiment without a new causal A/B.

The code belongs inside `search_par_default_seeds`: that is the only selector for the
accepted PEP seed portfolio. Putting the gate in `order_with_options` would duplicate
PEP selection and make it easier to accidentally affect RGS; putting it inside the
general `search_par` path would change a different accepted policy. Localizing the
substitution at array construction keeps the exact scope auditable.

## Verification and handoff

The network-disabled release suite enumerated 95 tests and exited zero; the new-source
research probes remain ignored, while every active test passed. The sandboxed offline
production build completed and staged `ssi-candidate-worker`. Production code differs
from exact source 4270 only in `src/ordering/rgreedy.rs`; the top-K TELOS, hub trim,
pair descent, scorer, worker protocol, corpus, dependencies, and limit files are
unchanged.

Before any official use, compare the live source hash again. If it is no longer
`4270fd72`, restack only the conditional slot substitution and repeat the full-300
baseline/candidate comparison. Do not submit this worktree automatically.

## Links

- RGREEDY mechanism and policy-diversity evidence:
  [0022](0022-rgreedy-elimination-game-ils.md)
- New parent top-K TELOS mechanism:
  [0022 top-K](0022-topk-telos-seeding.md)
