# 0026 — exact adjacent-pair descent in the sparse mid band

**Session:** `main` (2026-08-28). **Status:** win, ready to submit on accepted
leader `c7d5a8b`. **Combined dev score:** 0.818322 → **0.818256**
(−0.000066). The 1k–10k bucket moves 0.862522 → **0.862302**; the
lt-1k and gt-10k buckets are byte-identical. The final 300-matrix sandboxed
harness completed at 0.818256 / fill 0.934126.

## Context and failure-driven scope

The prior experiment, 0025, re-admitted randomized exact-elimination search in
the sparse 3k–10k band. It was a real local score win, but official submission
`37c36f1` failed the hidden per-matrix 2-second cap. The public Actions run
`33215750562` ended with:

> RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed

That is decisive evidence against shipping the 300M-operation mid-band
rgreedy arm. It has been removed completely. This experiment retains only a
much smaller deterministic local move which uses the exact elimination-game
identity but does not perform randomized whole-order searches.

While that submission was validating, the accepted leaderboard advanced to
`c7d5a8b`, official 0.820722. Its change ranks RGSUB elimination-tree blocks
for `n >= 10000`. The present arm is disjoint by construction:

- `4000 < n < 10000`;
- `nnz <= 60000`;
- hub-safe, `max_deg * 50 <= n`;
- four adjacent-pair sweeps under the existing 128M deterministic operation
  ceiling.

The accepted heavy RGSUB change, the two DETOUR/MetaFlip shoulders (`n<1000`),
and this mid arm therefore occupy non-overlapping size regimes. They merged
without a semantic conflict and were replayed together from a clean sandboxed
worker build.

## Exact move and certification

For two consecutive pivots `a,b`, eliminating both leaves the same residual
filled graph regardless of their internal orientation. Their contribution to
the objective is therefore minimized by eliminating the pivot with smaller
current exact fill-graph degree first. `adjacent_pair_descent` reconstructs the
current elimination game from the candidate permutation, visits disjoint
adjacent pairs, and swaps only when:

1. the two current pivots are adjacent in the exact current fill graph; and
2. the second has strictly smaller current degree than the first.

Alternating parity between sweeps exposes both even and odd boundaries. Every
choice and tie is deterministic. The move is local and exact, but its internal
reasoning is still treated as a proposal rather than as the trusted score.
Following the same proposal/certification boundary used throughout the
ordering stack, the returned permutation must pass both checks before it can
replace the incumbent:

- it must be a bijection of `0..n`;
- the shared trusted `ScoreWorkspace` must recompute a strictly smaller
  `sum(c_j^2)` value.

Thus no accumulator or local delta is trusted as the final objective. The AMD
floor remains intact, and a failed, exhausted, tied, or worse descent leaves
the previous result byte-for-byte unchanged.

## Why this gate is safer than the withdrawn arm

The failed rgreedy arm built a shared bitset graph and launched four randomized
search streams at a 300M-operation allowance. Its measured worst addition was
0.094 seconds locally, but the grader failure demonstrates that the composed
pipeline did not have enough hidden-matrix margin.

The pair arm is substantially narrower in mechanism and work:

- no random streams and no restart loop;
- exactly four deterministic passes at most;
- no candidate generation outside the final incumbent's adjacent boundaries;
- no dense arm;
- no `n >= 10000` work;
- no hub-heavy matrix;
- operation exhaustion discards the proposal rather than returning a partial
  ordering;
- strict trusted rescoring is the only admission path.

On the final accepted-leader carrier, an isolated all-in-gate probe measured
0.325 seconds total over the whole public band and 0.039 seconds worst added on
one matrix. That maximum is less than half the already-too-risky rgreedy
addition, and it does not land on the accepted heavy RGSUB regime. This is
comparative evidence rather than a claim about the hidden corpus; the hidden
cap remains the ultimate test.

## Sweep calibration

The new probe `probe_pair_descent_mid_tail` evaluates the arm after the full
shipped ordering, re-scores every proposal with the trusted objective, and
reports exact per-matrix changes plus isolated wall time.

One sweep produced 13 wins, `sum_ln = -0.006619`, and 0.015 seconds worst
added on the pre-leader carrier. Two sweeps produced 16 wins,
`sum_ln = -0.019347`, and 0.027 seconds worst. Four sweeps produced the best
risk/value point: 15 wins, `sum_ln = -0.025616`, and 0.040 seconds worst.
Eight sweeps were worse because some matrices exhausted the fixed operation
budget and correctly discarded the entire proposal; the extra work did not
buy a deeper accepted result. Four is therefore the measured saturation point
under the existing budget, not an arbitrary multiplier.

After removing the failed mid-band rgreedy arm and merging accepted leader
`c7d5a8b`, the authoritative four-sweep probe measured:

```text
PAIR_MID  wins=15  sum_ln=-0.027576230  total_s=0.325  worst_s=0.039
```

Applying that log delta over the 108 matrices in the 1k–10k bucket predicts
0.862522 → 0.862302, exactly matching the full harness.

## Exact public-corpus ledger

All 15 changed matrices improve. There are no losses because the phase is
terminal and uses strict exact rescoring.

| matrix | n | nnz | exact flops before → after |
|---|---:|---:|---:|
| `rsyn0810m04m` | 4772 | 13836 | 123255 → **122697** |
| `rsyn0830m04m` | 7252 | 20752 | 190581 → **190120** |
| `mpbp_07` | 9532 | 30810 | 1231192 → **1230952** |
| `rsyn0815m04m` | 5456 | 15756 | 165344 → **164979** |
| `syn40m04m` | 4960 | 13792 | 63726 → **63613** |
| `crudeoil_li05` | 4225 | 14628 | 586236 → **584135** |
| `chp_partload` | 5211 | 16740 | 392576 → **391864** |
| `rsyn0820m04m` | 6028 | 17372 | 173097 → **172283** |
| `chp_shorttermplan1a` | 4952 | 15700 | 322819 → **322597** |
| `rsyn0840m04m` | 8508 | 24272 | 205851 → **205708** |
| `mpbp_46` | 4518 | 14492 | 455167 → **454644** |
| `chp_shorttermplan2a` | 8992 | 26288 | 585144 → **584905** |
| `mpbp_15` | 9858 | 31692 | 1207295 → **1206727** |
| `mpbp_47` | 5310 | 17098 | 654972 → **654748** |
| `syn40m04hfsg` | 6328 | 16896 | 74192 → **74002** |

The two matrices which regressed through downstream seed sensitivity in 0025
now become direct wins: `rsyn0830m04m` reaches 190120 versus the pre-rgreedy
190581, and `rsyn0815m04m` reaches 164979 versus 165344. This is a cleaner fix
than retaining a whole alternate trajectory: orient the exact local inversions
at the terminal boundary, after all seed-sensitive phases have completed.

## Final combined result

The final tree contains:

1. accepted leader `c7d5a8b`, including ranked heavy RGSUB blocks;
2. the two bounded `n<1000` DETOUR/MetaFlip structural shoulders from 0024;
3. this sparse, hub-safe mid-band adjacent-pair extension;
4. no 3k–10k whole-matrix rgreedy arm.

The sandboxed 300-matrix harness completed successfully:

| bucket / metric | final value |
|---|---:|
| lt_1k flop geomean | 0.892430 |
| 1k_10k flop geomean | **0.862302** |
| gt_10k flop geomean | 0.729592 |
| weighted flop score | **0.818256** |
| weighted fill tiebreak | 0.934126 |

The pair-only weighted marginal is −0.000066 on this carrier. The score is a
local public-corpus result, not an official leaderboard claim. Promotion
requires the independent Yukon verifier to complete the hidden run within its
per-matrix cap and report a score below the current official 0.820722.

## Reproducibility and trust boundary

The candidate worker was built with the repository's sandboxed build script,
offline and network-denied. The full harness invoked the frozen trusted parent,
which checks determinism, permutation validity, exact symbolic score, process
isolation, and the local per-matrix cap. Only files under `src/ordering/` are
part of this change. The dirty local `results.tsv` and untracked coordination
`chat.txt` were not staged or modified as implementation inputs.

Solver context: GPT 5.6 Sol through Codex. The earlier rgreedy band was first
measured in a coordinated local lane; this pair extension, failure diagnosis,
leader merge, integration, and final verification were performed in this task.
