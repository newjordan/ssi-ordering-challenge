# 0025 — re-admit sparse mid-band rgreedy on the accepted frontier

**Session:** `main` (2026-08-28). **Combined score:** 0.819363 →
**0.819232** (−0.000131), fill 0.934773 → **0.934718**. The full 300-matrix
sandboxed harness passed determinism, bijection, and the local per-matrix cap.

## Context

The promoted frontier moved to `4270fd7` while the DETOUR/MetaFlip shoulder
experiment was validating. That accepted change added quality-ranked top-K
TELOS seeding and a hub gate-wall cost trim, with official score 0.821048. The
two max-min shoulder cells remained independently useful after merging it:
their `n<1000` marginal is exactly two matrices and about −0.000013 of weighted
dev score.

The strongest measured but held-back lead in the accepted tree was a different,
disjoint band. `rgreedy::Game` switches to degree buckets above `n=3000`, which
had reduced the old `3000<n<=10000` engine's worst added time from about 0.408 s
to 0.094 s while improving its search result. The production gate nevertheless
still stopped at 3000 because the original broad proposal also admitted dense
and hub-heavy matrices near the grader tail.

The hypothesis is that most of the score value survives a much narrower gate:

1. `3000 < n <= 10000`;
2. sparse pep arm only, `nnz <= 60000`;
3. hub-safe, `max_deg * 50 <= n`;
4. the base 300M operation budget, with no 600M/800M slack floor;
5. no 60k–250k dense RGS arm.

This class is disjoint from the shoulder archive (`n<1000`) and from the heavy
tail. Its selector and stopping rule are deterministic functions of the input;
no wall clock is read.

## Implementation

At the existing rgreedy call site, define:

```rust
const RGREEDY_MID_MAX_N: usize = 10_000;
const RGREEDY_MID_HUB_DIV: usize = 50;

let rgreedy_mid_gate = n > RGREEDY_MAX_N
    && n <= RGREEDY_MID_MAX_N
    && nnz > 0
    && nnz <= RGREEDY_PEP_MAX_NNZ
    && max_deg.saturating_mul(RGREEDY_MID_HUB_DIV) <= n;
```

The arm calls `rgreedy::search_par_default_seeds` with
`RGREEDY_PEP_BUDGET` directly. It deliberately does not apply
`rgreedy_slack_floor`, whose mid value would double the accepted experiment's
budget. The four fixed streams share the immutable bitset adjacency, use fixed
seeds, reserve the previous run's measured operation count before starting
another run, and hard-stop an exploding run at the engine's existing 1.25×
operation bound.

Every returned permutation follows the existing trust boundary:

- verify it is a bijection;
- recompute exact `Σ c_j²` using `ScoreWorkspace`;
- admit only on strict improvement over `best_flops`.

No score reported by the randomized engine is trusted. The AMD floor and
determinism contract remain unchanged.

## Same-tree A/B

The accepted top-K plus two-shoulder build was run as the baseline, then the
mid-band arm was added and the full corpus was replayed. Exactly nine matrices
change, all in `1k_10k`: seven improve and two regress through downstream seed
sensitivity.

| matrix | n | nnz | exact flops before → after | direction |
|---|---:|---:|---:|---|
| `crudeoil_lee1_07` | 3670 | 19322 | 3,872,747 → **3,687,609** | win |
| `rsyn0840m02m` | 3486 | 9576 | 45,773 → **45,354** | win |
| `rsyn0815m02hfsg` | 3054 | 7968 | 41,783 → **41,649** | win |
| `chp_partload` | 5211 | 16740 | 392,576 → **391,526** | win |
| `rsyn0820m04m` | 6028 | 17372 | 173,097 → **172,738** | win |
| `mpbp_47` | 5310 | 17098 | 654,972 → **654,536** | win |
| `gasprod_sarawak16` | 4596 | 15316 | 210,731 → **210,720** | win |
| `rsyn0830m04m` | 7252 | 20752 | 190,581 → 190,590 | loss |
| `rsyn0815m04m` | 5456 | 15756 | 165,344 → 167,345 | loss |

The bucket and full-score result is:

| metric | accepted top-K + shoulders | plus mid rgreedy | delta |
|---|---:|---:|---:|
| lt_1k | 0.892430 | 0.892430 | 0 |
| 1k_10k | 0.862522 | **0.862085** | **−0.000437** |
| gt_10k | 0.732194 | 0.732194 | 0 |
| weighted score | 0.819363 | **0.819232** | **−0.000131** |
| fill | 0.934773 | **0.934718** | **−0.000055** |

The exact full result also composes as predicted from the independent lane:
accepted-frontier mid rgreedy alone measured 0.819245, and the shoulders'
disjoint lt_1k marginal accounts for the remaining improvement to 0.819232.
This agreement is useful evidence that neither seed policy silently cannibalizes
the other.

## The measured loss and rejected margin gate

Strict acceptance inside the rgreedy phase does not make the complete pipeline
monotone when a later phase is seed-sensitive. The large loss on
`rsyn0815m04m` is the same mechanism documented in 0019/0020: rgreedy finds a
better immediate incumbent, but a later descent follows a different trajectory
and finishes worse than it did from the old seed.

An obvious 1% acceptance-margin mitigation was measured and rejected. The
losing rgreedy candidate itself improves by at least 1%, so the margin does not
remove that loss; instead it deletes four sub-1% wins worth more in aggregate.
The scoped log-sum improvement is about −0.0505 under strict acceptance versus
−0.0438 with the margin. Preserving both trajectories would require another
top-K-shaped downstream change, not a threshold on one candidate.

## Cap boundary

The local full harness passed, but local success is not promoted as proof of
grader safety. The risk reduction is structural:

- `n<=3000` behavior is byte-identical;
- matrices above `n=10000` are excluded;
- the dense 60k–250k arm is excluded;
- hub-heavy patterns are excluded by the same mechanism used by the accepted
  mid/heavy gates;
- the new band receives 300M ops, not the larger slack floors;
- the engine already has deterministic reserve-before-run and hard-stop bounds.

In isolated/scoped measurements on the accepted carrier, the broad bucket
engine's worst added `order()` time was 0.094 s. The final narrowed arm did not
move the global worst matrix: that worst lies outside the gate. The admitted
band's heaviest observed calls remained below the same-run global worst. This
is comparative evidence, not a promise about unknown hidden structure.

Memory remains comfortably inside 4 GiB: at `n=10000`, all four streams share
one immutable adjacency and own bounded game state measured in tens of
megabytes, while dense matrices that would enlarge fill work are gated out by
`nnz` and the hub predicate.

## Conclusion

This is a strict, full-corpus win on top of the currently accepted frontier and
the two-cell structural archive. Its mechanism is not “raise `RGREEDY_MAX_N`”;
the broad raise was correctly held back. The shippable unit is the sparse,
hub-safe, no-floor sub-band with the existing exact certification path.

Solver context: GPT 5.6 Sol through Codex for integration and verification; a
coordinated local lane performed the initial band A/B. No exact effort setting
was exposed, so none is asserted.

## Official postscript — failed cap, withdrawn

Submission `37c36f1` failed the hidden verifier. Public Actions run
`33215750562` reports that one hidden matrix exceeded the 2.0-second
per-matrix cap. The local score gain was real, but the gate was not safe enough
for the official runtime distribution. Commit `017e9ba` removes this mid-band
rgreedy arm completely. Do not re-submit 0025 without a materially cheaper
engine or a hidden-safe structural bound; the terminal adjacent-pair arm in
0026 recovers broader deterministic mid-band gains at less than half the
measured worst local addition.
