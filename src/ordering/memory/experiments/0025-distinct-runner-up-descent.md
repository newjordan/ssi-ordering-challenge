<!-- 0025 -->
# 0025 — Distinct-runner-up second TELOS descent (gap > 0.1%, nnz ≤ 100k): shipping 0022's procurement1large lead behind a distinctness gate

- **Date:** 2026-08-29
- **Score:** 0.842223 → **0.841594** (fill 0.948942 → 0.948853)
- **Status:** **WIN, shipped** — first scoring win since 0019; all three
  pre-registered measures passed (probe / full run / interleaved timing);
  35/35 tests green

## Hypothesis

0022 implemented the 0020-named top-K (K=2) TELOS fix and killed it on its own
motivating matrix — but measured one decisive cross-basin case worth shipping on
its own: `procurement1large`, where the runner-up seed (0.35% behind the best)
descended 0.636532 → 0.584669 while the best seed sat unmoved, final 0.581426 vs
the incumbent's 0.634285 (−0.0529 single-matrix in gt_10k ≈ −0.0006 weighted).
The kill mechanism was equally precise: top-2-by-flops usually degenerates to a
same-basin near-duplicate (the demoted previous best on ONE improvement path),
whose descent accepts zero moves — pure waste, ~0.1–0.25 s in the 40–90k band.

This stage re-applies the exact 0022 implementation (runner-up tracking via
`accept_top2` at ALL accept sites: consider, hill-climb, post-hoc PEEL,
COUNT-RELABEL, SHOT-3 `try_cand`; per-seed TELOS descent loop thresholding
against its own seed; strict-keep best-of so no matrix can regress) and adds a
**DISTINCTNESS GATE**: the second descent runs only when the relative flops gap
`(second − best) / best` at TELOS entry is **> `TELOS_TOPK_GAP_MIN` = 0.001**
(and `nnz ≤ TELOS_TOPK_MAX_NNZ` = 100_000, unchanged from 0022). Intent: skip
the measured degenerate near-duplicates, keep the measured basin-crosser.

**Pre-registered:** (a) probe — `procurement1large` passes the gate and reaches
≈0.5814; `chp_shorttermplan2d`/`mpbp_48`/`chimera_lga-01` are skipped (recorded
as gap ≤ 0.0005); (b) full dev run strictly < 0.842223, expect ≤ 0.8417, else
revert + negative page; (c) interleaved A/B timing (0021's
`probe_permute_wire_ab` protocol): candidate ≤ incumbent + its own added
descent time; `ringpack_30_2` untouched.

## What changed

`src/ordering/mod.rs` — the 0022 diff (preserved as `0022-topk-diff.patch`)
re-applied verbatim: `accept_top2` + runner-up wiring at every accept site, the
TELOS seed loop (descent 0 from `best_perm` exactly as shipped, descent 1 from
`second_perm`, each thresholding against its own seed, final = strict best of
the two finals with ties to the best-seed descent), `TELOS_TOPK_MAX_NNZ`,
test-only `TELOS_SEED_RESULTS`. New in 0025: `TELOS_TOPK_GAP_MIN = 0.001` and
the gap condition on `seed_count`, plus test-only `TELOS_TOPK_GATE` capturing
`(best, second, verdict)` at descent-loop entry.

`src/ordering/probe.rs` — `probe_telos_topk` resurrected and extended to print
the gate inputs (relative gap, PASS/skip) and an `ALL` census mode (every
matrix under the nnz gate, single rep) to enumerate exactly which matrices pay
a second descent; `probe_permute_wire_ab` gains an `SSI_AB_NAMES` env override
so the A/B protocol can target this stage's matrices.

## Result

### Probe (pre-registered (a)) — gate verdicts on the 0022 gap table

| matrix | nnz | gap at TELOS entry | gate | runner-up descent | final ratio |
|---|---|---|---|---|---|
| `procurement1large` | 41 068 | **0.003542** | **PASS** | 0.636532 → **0.584669** | **0.581426** (incumbent 0.634285) |
| `mpbp_48` | 91 016 | 0.000049 | skip | — (0 added descents) | 0.506909 |
| `chimera_lga-01` | 6 400 | 0.000276 | skip | — (0 added descents) | 0.803396 |
| `chp_shorttermplan2d` | 52 108 | **0.014545** | PASS (see below) | 0.535533 → 0.535533 (zero moves) | 0.514660 (= incumbent) |

The essential half of (a) holds exactly: `procurement1large` passes and
reproduces 0.584669/0.581426; the two true near-duplicates sit two orders of
magnitude under the threshold and are skipped. **The pre-registration's third
skip was WRONG:** `chp_shorttermplan2d`'s gap is 0.014545 — *larger than the
winner's* — because 0022's "runner-up within 0.05% of best" signature was only
ever measured on mpbp/chimera; chp's runner-up (2 189 795, the demoted
consider-pool best) is 1.5% behind the TELOS-entry incumbent yet still on the
same improvement path. **No gap threshold separates chp from procurement**
(0.0145 > 0.0035), so gap-as-distinctness cannot exclude it; chp pays a ~0.1 s
no-move second descent as the price of keeping the winner. Strict-keep means
this cannot cost score, only time — accepted, and measured in (c).

### Gate census (`ALL` mode) — every second descent the corpus pays

**53 of 272** sub-100k matrices pass the gate (gaps 0.001038 … 0.107120); 219
are skipped, 196 of them at gap exactly 0.000000 (no distinct runner-up at
all, or one at identical flops — `accept_top2` rejects exact-duplicate perms)
and 23 in the (0, 0.001] near-duplicate band the gate exists to kill
(`mpbp_48` 0.000049, `chimera_lga-01` 0.000276, `pinene200` 0.000049, …).
Census wall time 110 s, single rep.

Of the 53 second descents, **12 accepted at least one move** and **5 strictly
beat the best-seed descent** (= the only matrices whose `order()` output
changes vs the incumbent, all improvements by strict-keep):

| matrix | n (bucket) | gap | best-seed TELOS | runner-up TELOS | final (cand) |
|---|---|---|---|---|---|
| `procurement1large` | 14 416 (gt_10k) | 0.003542 | 0.634285 unmoved | 0.636532 → **0.584669** | 0.581426 |
| `st_e31` | 301 (lt_1k) | 0.006110 | 0.974636 unmoved | 0.980591 → **0.954345** | 0.954345 |
| `graphpart_clique-20` | 80 (lt_1k) | 0.023663 | 0.829812 → 0.823850 | 0.849448 → **0.821385** | 0.820095 |
| `maxcsp-langford-3-11` | 660 (lt_1k) | 0.001178 | 0.390986 → 0.390014 | 0.391446 → **0.385088** | 0.385088 |
| `nuclear25a` | 1 942 (1k_10k) | 0.002213 | 0.680452 unmoved | 0.681958 → **0.678753** | 0.652069 |

Moved-but-did-not-win (second descent worked, strict-keep discarded it):
`chp_shorttermplan1a` (0.899561→0.884150, best 0.879972), `chp_partload`
(0.817122→0.813982, best 0.810464), `pooling_digabel19` (0.907816→0.902061,
best 0.896309), `edgecross14-156` (0.956339→0.952013, best 0.946230),
`netmod_kar1` (0.880133→0.876051, best 0.872744), and two that converged to
an exact tie with the best seed's result (`sonet24v5`, `syn40m04hfsg`). The
remaining 41 descents accepted zero moves — the 0022 waste class, now
bounded to matrices whose runner-up at least *looked* distinct. So the 0022
lead was UNDER-scoped: rank-2-by-flops crosses basins on 5 corpus matrices,
not 1, spread over all three buckets.

### Full dev run (pre-registered (b)) — PASS

**0.842223 → 0.841594** (−0.000629; bar was ≤ 0.8417), tiebreak fill
0.948942 → 0.948853. `results.tsv`: `1787933807	OK	0.841594	0.948853`.

| bucket | count | flop geomean (cand) | note |
|---|---|---|---|
| lt_1k | 147 | 0.8993 | st_e31 / clique-20 / langford wins land here |
| 1k_10k | 108 | 0.8754 | nuclear25a |
| gt_10k | 45 | **0.7730** | 0.7748 at baseline (0018 ledger) — procurement1large's −0.0529 |

The −0.000629 is ≈ procurement1large's predicted −0.0006 plus the four
unpredicted small-bucket wins; every changed matrix is an improvement (the
strict-keep construction admits nothing else).

### Interleaved A/B timing (pre-registered (c)) — PASS

0021's protocol: `probe_permute_wire_ab` (now `SSI_AB_NAMES`-overridable),
min-of-5 per matrix, adjacent candidate/incumbent runs (C/I/C/I) from two
separate build trees (incumbent = a `git worktree` at d65a7b4, own `target/`,
same corpus bytes) on a box with background load. min-of-5 seconds:

| matrix | nnz | 2nd descent | C r1 | I r1 | C r2 | I r2 | Δ adjacent |
|---|---|---|---|---|---|---|---|
| `procurement1large` | 41 068 | **WIN descent** | 1.466 | 1.306 | 1.383 | 1.221 | **+0.16 / +0.16** |
| `chp_shorttermplan2d` | 52 108 | zero-move | 1.569 | 1.520 | 1.597 | 1.553 | +0.05 / +0.04 |
| `crudeoil_pooling_dt2` | 75 910 | zero-move | 0.942 | 0.999 | 1.005 | 0.951 | −0.06 / +0.05 |
| `faclay30` | 82 242 | zero-move | 0.975 | 0.983 | 0.912 | 0.851 | −0.01 / +0.06 |
| `ringpack_30_2` | 121 458 | none (>100k gate) | 2.024 | 1.985 | 2.123 | 2.092 | +0.04 / +0.03 |

Every delta is ≤ the matrix's own added descent (the criterion): the paying
matrix costs a reproducible **+0.16 s** (its cross-basin descent, cheaper
than 0022's +0.24 s estimate), zero-move descents cost ≤ 0.06 s and are
inside the noise floor (adjacent-pair sign flips), and `ringpack_30_2` —
identical code path by the nnz gate — differs by +0.03/+0.04 s against a
rep spread of 2.02–2.57 s, i.e. pure box noise. Worst rep anywhere 2.57 s,
far under the 8 s cap; the full run's OK row confirms cap + determinism
gates.

## Why it won

The mechanism is exactly 0022's, and 0022's kill analysis was right about
the failure mode but wrong about its frequency: rank-2-by-flops is USUALLY a
same-basin duplicate, but on 5 of 272 gated matrices it is a genuinely
different basin whose descent beats the best seed's — and the strict-keep
floor converts "usually useless" into "free option, exercised where it
pays". The distinctness gate contributes the cost control, not the win: it
deletes 219 of 272 second descents (196 with no distinct runner-up at all,
23 in the (0, 0.001] near-duplicate band 0022 measured as pure waste),
leaving 53 candidates of which 41 still find nothing — the residual price,
~0.05 s each in the mid band, bounded and paid only where the runner-up at
least looked distinct.

What the gate CANNOT do (measured, important for successors): gap size does
not predict basin difference. `chp_shorttermplan2d`'s runner-up is 1.45%
away — four times the winner's gap — yet still on the incumbent's own
improvement path (0022: the demoted consider-pool best), while
`maxcsp-langford-3-11` crosses basins at gap 0.001178, a hair over the
threshold. The gate is a cheap NECESSARY condition (a same-flops duplicate
cannot be a different basin), not a sufficient one; 0.001 comes from 0022's
degenerate-case table (`mpbp_48` 0.000049 and `chimera_lga-01` 0.000276,
both measured zero-move, sit among the 23 sub-threshold skips). Caveat for
successors: the other 21 skips in (0, 0.001] were never descended, so their
worth is UNMEASURED by construction — lowering GAP_MIN is a cheap census
away if anyone suspects treasure there, but langford at 0.001178 is the
closest measured call and it was already admitted.

## Follow-ups

- The moved-but-lost class (`chp_shorttermplan1a`, `chp_partload`,
  `pooling_digabel19`, `edgecross14-156`, `netmod_kar1`) proves more
  reachable basins exist than the best seed sees; a K=3 or a
  diversity-axis second seed (0020's between-CONFIG idea, e.g. the
  restart-prefix winner or best-of-another-family) might convert some —
  but 0022/0025 together say expect most runner-up basins to LOSE, so any
  successor needs the same strict-keep + census-first protocol.
- `chp_shorttermplan2d`'s ~0.484 basin (0020) remains unreached — its
  second descent runs and finds nothing; rank-2 is structurally the wrong
  seed there, do not re-tune GAP_MIN hoping to fix it.
- The `ALL`-mode gate census (110 s) is the cheap instrument this stage
  lacked in 0022; re-run it before touching `TELOS_TOPK_GAP_MIN` or the
  100k gate, and against the CURRENT incumbent (0023's staleness rule).

## Links

- Mechanism + the measured lead this ships: [0022](0022-telos-topk-runner-up.md)
- Seed-sensitivity diagnosis: [0020](0020-telos-seed-sensitivity.md)
- Gate rationale (130k is not cheap): [0013](0013-gate-covariate-audit.md)
