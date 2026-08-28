# 0025 — spend mid-band slack only where exact attribution pays for it

- **Date:** 2026-08-29
- **Exact parent:** `698da697d45d2f86bdff9c81b1048442c78c0e42`
- **Final policy:** keep the 600M mid floor except for PEP rows satisfying
  `nnz <= 2*n || (4*n <= nnz && 2*nnz <= 9*n)`, where the floor is 800M
- **Full-300 score:** `0.819962 -> 0.819768` (`-0.000194`, lower is better)
- **Final affected-row worst:** 1.1403 s across seven eligible visible rows
- **Status:** strict exact-parent local win; no submission performed by this arm

## Orchestrator decision

P35 is the shippable result of this line. It retains the two dominant score
wins discovered by the broad budget experiment and removes the regions that
either regressed visibly or consumed unnecessary cap margin. The code diff is a
policy-layer density gate plus an 800M narrow-floor constant; search mechanics,
seeds, gates, trusted scoring, and admission are unchanged.

If `698da697` is still the live source, this result is ready for submission
coordination. If another source is promoted first, restack the small policy diff
and repeat full-300 A/B. Rgreedy consumes the incumbent produced by all earlier
phases, so a new upstream portfolio can subsume or alter these gains. Never
claim an exact-`698da697` score for a later composition without remeasurement.

The final acceptance gates were:

1. candidate score strictly below exact-parent `0.819962`;
2. final changed-domain worst no greater than 1.15 s;
3. only a deterministic predicate of `(n, nnz)` may select the extra 200M;
4. every visible changed row under the final predicate must be a win;
5. active tests, static boundary preparation, and production build must pass;
6. no stale-parent result and no broader rejected arm may be submitted.

P35 passed: score `0.819768`, two changed rows and both wins, seven eligible
rows, affected worst 1.1403 s. Its whole-corpus worst was 1.417 s on a row
outside this arm. That whole-call observation is below the original 1.55 s
local gate; the more important final check is the 1.15 s affected-row gate.

## 1. What structure had blocked progress?

The rgreedy phase already had a useful algorithm, stable seed plan, trusted
rescoring, and two accepted selectors under `n <= 3000`:

- PEP: `0 < nnz <= 60_000`, without an average-degree guard;
- RGS: `60_000 < nnz <= 250_000` and `nnz <= 100*n`.

At the exact parent, their nominal budgets are 300M and 200M operations. A
shared floor allocates no extra work below `n=64`, 800M for
`64 <= n <= 1000`, and 600M for `1001 <= n <= 3000`. The discontinuity left an
obvious question: the adjacent small band had already justified 800M, so was the
mid band merely underfunded?

The challenge cap is per matrix while the operation budget is flat. Unused time
from one matrix cannot protect a different matrix. Therefore safe mid-band
headroom is stranded unless the selected search spends it. But a flat increase
is also dangerous: visible local headroom is not proof that every hidden matrix
inside a broad `(n, nnz)` envelope has the same cost. The problem was not
"increase a constant"; it was "identify which structural slice converts the
extra work into score without inheriting the whole selector's cap risk."

A second block was parent drift. An earlier exact-`28652c6` experiment raised a
flat PEP budget from 600M to 800M and measured `0.819884 -> 0.819805`, but its
candidate worst was 2.390 s and PEP-gated worst 1.960 s under contaminated
timing. More importantly, `28652c6 -> 698da697` changed pair descent from
2 sweeps / 32M to 4 sweeps / 128M, reset nominal PEP from 600M to 300M, and
introduced the current shared size floors. Since rgreedy starts from the current
incumbent, that stale score did not answer the live question. It was discarded,
with no build and no commit.

## 2. How was the block opened?

### P30: broad MID800 discovery arm

First change only `RGREEDY_SLACK_MID`, 600M to 800M, on exact `698da697`.
The full-300 score improved `0.819962 -> 0.819776` (`-0.000186`). The candidate
whole-call worst was 1.519 s and broad PEP-domain worst was 1.3405 s. Ten printed
matrix ratios changed:

| matrix | n / nnz | 600M | broad 800M | rounded delta |
|---|---:|---:|---:|---:|
| `crudeoil_pooling_ct3` | 2644 / 11426 | 0.6865 | **0.6462** | -0.0403 |
| `lop97icx` | 1073 / 2118 | 0.8519 | **0.8346** | -0.0173 |
| `syn15m04m` | 1908 / 5288 | 0.9733 | **0.9667** | -0.0066 |
| `chimera_selby-c16-01` | 2031 / 10964 | 0.5935 | **0.5918** | -0.0017 |
| `edgecross10-080` | 1053 / 5940 | 0.9644 | **0.9638** | -0.0006 |
| `rsyn0830hfsg` | 1590 / 4100 | 0.9563 | **0.9557** | -0.0006 |
| `blend718` | 1386 / 4310 | 0.8748 | 0.8749 | +0.0001 |
| `rsyn0840m` | 1159 / 3168 | 0.9432 | 0.9439 | +0.0007 |
| `hydroenergy2` | 2092 / 6236 | 0.8579 | 0.8591 | +0.0012 |
| `nuclear25a` | 1942 / 16030 | 0.5643 | 0.5711 | +0.0068 |

This table supplied the missing structure. Extra budget is not empirically
monotone per matrix: the 800M execution is not necessarily the 600M search plus
an appended suffix, because budget can affect stream allocation, stopping, or
per-run limits. The trusted `f < best_flops` admission prevents regression
against the incumbent entering rgreedy, not against a different rgreedy budget.
The aggregate won because two large gains outweighed four regressions.

### Why the broad arm was not shipped

While P30 was being verified, a sibling P32 submission reached the official
benchmark and failed with the signed-in Actions result:

```text
RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed
```

P32 changed a different rgreedy allocation axis, so this is not evidence that
P30 would fail on the same matrix. It is decisive evidence that the live source
does not have enough universal hidden margin to justify broad extra work merely
because a visible aggregate timing looks comfortable. The official failure
changed the risk standard: preserve the attributed wins and remove work not
paying for visible score. Broad MID800 was never submitted.

### P34: attribution-shaped PEP density gate

The six printed broad-arm wins occupy either very sparse density (`nnz <= 2*n`)
or bands between roughly `2.5*n` and `5.7*n`. All four printed losses fall
outside the union `nnz <= 2*n || 4*n <= nnz <= 6*n`. Restricting 800M to that
union and to PEP excludes every visible regression and RGS while retaining four
dominant wins. P34 measured:

- score `0.819760` (`-0.000202` from exact parent, better than broad P30);
- exactly four changed rows, all improvements;
- 18 visible rows eligible for 800M;
- whole-corpus worst 1.380 s;
- eligible-row worst 1.2651 s on `chimera_selby-c16-01`.

P34 proved that structural attribution can improve both score and risk. It
missed the preferred affected-tail target of 1.25 s by 0.0151 s, so it remained
a discovery arm rather than the final arm.

### P35: cap-first final gate

The two largest wins contain essentially all the value:

- `lop97icx`: `nnz <= 2*n`, improvement -0.0173;
- `crudeoil_pooling_ct3`: `4*n <= nnz <= 4.5*n`, improvement -0.0403.

Tightening the second band from `<=6*n` to `<=4.5*n` drops the small chimera and
edgecross wins but removes the 1.2651 s row and cuts visible eligibility from 18
rows to seven. Integer arithmetic expresses 4.5 without floats as
`2*nnz <= 9*n`; saturating multiplication makes the predicate total for all
`usize` inputs. PEP membership remains explicit, so RGS never receives 800M.

P35 full-300 result:

| metric | exact parent | P35 safe gate |
|---|---:|---:|
| `lt_1k`, 147 matrices | 0.8925 | 0.8925 |
| `1k_10k`, 108 matrices | 0.8622 | **0.8616** |
| `gt_10k`, 45 matrices | 0.7338 | 0.7338 |
| overall score | 0.819962 | **0.819768** |
| whole-corpus observed worst | 2.271 s noisy baseline | **1.417 s** |
| final eligible-row observed worst | n/a | **1.1403 s** |
| visible 800M eligible rows | 0 mid boosts | **7** |
| printed changed rows | n/a | **2, both wins** |

The final two printed changes were exactly the predicted large wins:
`crudeoil_pooling_ct3 0.6865 -> 0.6462` and
`lop97icx 0.8519 -> 0.8346`. The eligible-row tail was
`crudeoil_pooling_ct3` at 1.1403 s, passing the predeclared 1.15 s bound by
0.0097 s. Cross-run times are cap observations, not a claim that extra work
makes execution faster; different outside-gate matrices led the whole-call tail
in different runs.

## Verification and build boundary

All benchmark and verification commands ran offline inside bubblewrap with a
network namespace unshared, unique writable target/log directories, and the
source tree read-only to the process. Timing probes were serialized with every
sibling SSI arm.

- `scripts/prepare-build.sh` regenerated the manifests/vendor preparation.
- The native/FFI scan was clean before measurement.
- Final P35 full-300 probe: 300/300 rows, score 0.819768, worst 1.417 s.
- Final active release tests: **38 passed, 0 failed, 54 ignored**.
- Production `ssi-candidate-worker` release build: offline, locked, optimized,
  successful.

The local Rust 1.94.0 toolchain twice hit a compiler internal error while
rendering an existing warning in `ordering::biconn`
(`check_mod_deathness` / `annotate_snippets`). It was not a source diagnostic
and reproduced with one Cargo job. Suppressing warning emission with
`RUSTFLAGS=-Awarnings` avoided only that diagnostic-render path; the same
non-test production binary, release optimization, lockfile, package, and
offline dependency graph then built successfully in 32.33 s. The official P32
failure occurred after build at the hidden per-matrix cap, so this local rustc
frontend issue is separate from that official runtime failure.

## 3. How can the next ascent proceed?

1. **Restack before extending.** A newly promoted earlier phase can change the
   rgreedy incumbent. Repeat exact-parent A/B instead of cherry-picking a score.
2. **Instrument budget prefix stability.** Record stream-level best flops, run
   reservation, and exhaustion on the ten P30 changers. A prefix-stable schedule
   could retain extra work without the four broad-arm regressions.
3. **Use exact flop deltas, not four-decimal ratios.** The next selector should
   quantify value per operation and per local millisecond for every eligible
   row. Stop adding bands when marginal weighted log-score becomes negligible.
4. **Keep PEP and RGS evidence separate.** The broad RGS slice changed no printed
   ratio and was removed from P35. It needs independent positive evidence before
   receiving more than 600M.
5. **Treat official hidden-cap failures as composition evidence.** A sibling
   failure does not identify the same matrix, but it invalidates universal
   headroom assumptions. Recompute risk whenever work is added elsewhere.
6. **Prefer convergence features over ever finer density bins.** The final bins
   are justified by exact attribution, but stream convergence or improvement
   rate may generalize better to hidden data than additional hand-cut ratios.

## 4. Where can structural change happen?

- **Portfolio policy (`order_with_options`):** owns selectors and cap allocation.
  P35 changes only this layer by choosing the mid floor from `(n, nnz)`.
- **Search scheduling (`rgreedy`):** owns stream budgets, reservation, stopping,
  and prefix behavior. The broad non-monotonicity should be investigated here,
  but that would be a separate experiment.
- **Candidate admission:** bijection validation and trusted strict flop scoring
  remain unchanged. This is the quality firewall and must not be weakened.
- **Orchestration/evidence:** exact-parent worktrees, unique targets, serialized
  full-300 timing, official-log review, and post-promotion restacks prevent a
  true local win from becoming a false live-frontier claim.

The general rule is: spend per-matrix slack at the selector boundary only where
exact attribution shows that additional work pays. A broad safe-looking local
tail is insufficient once the official composition has exposed a hidden cap
failure. Narrow on observable structure, remeasure the entire corpus, and keep
the trusted admission invariants unchanged.

## Reproduction and hand-off checklist

- Check out `698da697d45d2f86bdff9c81b1048442c78c0e42`.
- Prepare generated manifests/vendor state, then deny network access.
- Run exact-parent full-300 in a unique target while owning the timing host.
- Add `RGREEDY_SLACK_MID_NARROW = 800M`; retain base MID at 600M.
- Select the narrow floor only for PEP, `n > 1000`, and either
  `nnz <= 2*n` or `4*n <= nnz <= 4.5*n` (the outer rgreedy gate already bounds
  `n <= 3000`).
- Repeat full-300 alone; require strict score win and eligible worst <=1.15 s.
- Confirm every printed changed row, not only the aggregate score.
- Run active release tests and a separate offline locked production build.
- Audit that the diff contains only the policy change and this evidence note.
- Restack and repeat if the live frontier changed; submission is an orchestrator
  action, not part of this experiment.
