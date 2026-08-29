# 0027 — cap-safe terminal pair cleanup below 1000

## Status

**LOCAL WIN on the current official top; follow-on not yet submitted.** The
addition keeps every accepted gate unchanged and adds the same exact terminal
cleanup only for `2 <= n < 1000`. The separately hardened upper-band work in
the accepted leader remains intact.

The currently merged carrier includes accepted leader `7d22390` / submission
`212bd4d` (official score **0.820508**, first place), the two bounded
DETOUR/MetaFlip shoulders from 0024, and this small terminal cleanup. Its direct
whole-corpus probe is **0.818099**:

| bucket | count | flop geomean |
|---|---:|---:|
| lt_1k | 147 | 0.8921 |
| 1k_10k | 108 | 0.8620 |
| gt_10k | 45 | 0.7296 |
| weighted | 300 | **0.818099** |

Worst direct `order()` time was **0.744 s**. The exact score is a public-corpus
measurement, not an official leaderboard claim. Promotion still requires the
independent hidden verifier.

## Why 0026 was withdrawn

0026 extended four exact pair sweeps from the accepted `n<=4000` band into
`4000<n<10000` under `nnz<=60000` and `max_deg*50<=n`. That arm looked unusually
safe locally: 15 public matrices improved, its isolated worst addition was
0.039 s, and the full local harness passed at 0.818256.

The hidden verifier falsified that timing argument. Submission `e40859a`
(archive commit `42316fb`, Actions run `33218086788`) failed with:

> hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed

That is decisive evidence. A red hidden-cap result dominates a green public
timing probe because the unknown matrix can have a different fill trajectory
inside the same `(n,nnz,max_deg)` envelope. The complete `nnz<=60000` pair lane
above 4000 was removed. Attractive follow-up public wins above 12k were also
discarded; they do not repair the missing hidden-time bound. A different solver
later hardened a strict subset to `nnz<=30000`, a 48M operation budget, and
`n<=12000`. Submission `212bd4d` promoted that subset, so it is retained as
verified leader behavior; the failed wide lane is still forbidden.

This is the same boundary already learned from the rgreedy failure in 0025:
bitset elimination cost depends on the generated fill path, not just the input
edge count. A structural screen that predicts cheap public matrices is not an
upper bound on an unseen trajectory.

## Exact move and acceptance boundary

For two consecutive pivots `a,b` that are adjacent in the current filled graph,
eliminating both leaves the same residual graph in either orientation. Putting
the lower current-degree pivot first strictly lowers the contribution of that
pair to `sum c_j^2`. Alternating even and odd disjoint boundaries visits every
adjacent position without overlapping swaps inside one sweep.

The phase still does not trust that local argument as a global interface. It:

1. verifies the input seed is a bijection;
2. reconstructs the exact elimination game from the original pattern;
3. emits only fully completed permutations;
4. verifies the candidate is a bijection again at the caller;
5. recomputes its objective with the trusted `ScoreWorkspace`; and
6. retains it only on strict global improvement.

This is the same DETOUR-style proposal/certification separation used in 0024:
the pair rule proposes, while exact scoring certifies. The AMD anchor and every
pre-existing candidate remain available, so the quality path is monotone.

## Budget-exhaustion repair

The original `adjacent_pair_descent` accumulated its deterministic operation
counter across sweeps. If a later sweep exceeded the 128M budget, however, it
returned `None` and discarded the valid permutation produced by earlier fully
completed sweeps. That made larger sweep ceilings non-monotone in practice:
more attempted work could erase an already-proven improvement.

The repair preserves `cur`, the result of the last completed sweep, whenever a
later sweep exceeds budget. The partially constructed `next` prefix is never
returned. If the first sweep exceeds budget, `changed_any` is false and the
function still returns `None`. Therefore the change cannot admit a partial or
unscored permutation.

The engine also stops after two consecutive no-change sweeps. The two sweeps
have opposite parity and inspect every adjacent boundary of the same unchanged
permutation, so a third sweep cannot find a pair move unless some earlier
boundary changed. Any changed sweep resets the no-change streak.

The completed-sweep fallback is independent of the leader's gate hardening. It
changes only what happens after a later sweep exhausts its deterministic
budget; it does not raise the accepted upper-band budget or widen its gate.

## Small-tier measurement

With the small phase disabled in `order()`, the dedicated probe applies 32
sweeps to the full incumbent endpoint. On accepted leader `7d22390`, it finds
nine strict wins and no losses:

| matrix | n | before | after |
|---|---:|---:|---:|
| `clay0204hfsg` | 590 | 6674 | **6646** |
| `chimera_mgw-c8-439-onc8-002` | 440 | 90741 | **90569** |
| `syn40m` | 560 | 4857 | **4850** |
| `maxcsp-langford-3-11` | 660 | 11205161 | **11084027** |
| `ndcc13` | 969 | 334741 | **334577** |
| `chimera_mgw-c8-439-onc8-001` | 440 | 83809 | **83693** |
| `pooling_adhya4pq` | 170 | 18716 | **18628** |
| `multiplants_stg1b` | 814 | 189701 | **189459** |
| `space25a` | 634 | 28631 | **28624** |

The sum of log ratios over the nine wins is **-0.026522269**. Isolated pair work
over all 147 small matrices totals **0.014 s**, with worst **0.003 s** on
`maxcsp-langford-3-11`. The phase uses small `n*n` bitsets and a finite 32-sweep
ceiling; it does not run the unbounded randomized search that made tiny cases
expensive in earlier rgreedy budget experiments.

Four and eight sweeps both find the same nine winners and the same
`sum_ln=-0.026522269` on this carrier. A deeper endpoint had paid on the prior
carrier, so 32 is retained only below 1000; its measured worst incremental cost
is still a few milliseconds. For `1000<=n<=4000`, the sweep count remains four,
exactly the cost profile already present in promoted leader history.

## Safety boundary

The additive small gate is deliberately simple, while accepted upper-band
behavior remains separate:

- `2 <= n <= 4000`;
- `0 < nnz <= 60000`;
- 32 sweeps only when `n < 1000`;
- four sweeps when `1000 <= n <= 4000`;
- accepted leader `7d22390` may also run four sweeps for `4000<n<=12000`, but
  only at `nnz<=30000`, `max_deg*50<=n`, and a reduced 48M budget.

The quality boundary is stronger than the timing boundary: every returned
candidate is exact-score checked and can never worsen the result. The timing
boundary is empirical and asymmetric. The accepted 1k--4k and hardened upper
phases have passed the hidden verifier, and the new small work is at most 0.003
s isolated on the public corpus. By contrast, the old `nnz<=60000` 4k--10k
extension has a direct hidden failure and remains forbidden even though its
public gains were larger.

## Reproducibility

Build the untrusted worker offline and network-denied:

```sh
bash scripts/local-candidate-build.sh
```

Measure the current whole-corpus score and timing:

```sh
cargo test --release --offline --locked -p ssi-candidate-worker \
  probe_timing_and_score -- --ignored --nocapture
```

Isolate the small pair marginal by temporarily holding
`PAIR_DESCENT_MIN_N=1000`, then run:

```sh
cargo test --release --offline --locked -p ssi-candidate-worker \
  probe_pair_descent_lt_1k -- --ignored --nocapture
```

The reduced pre-leader carrier completed the full sandboxed 300-matrix harness
at 0.8183 rounded. After merging official top `7d22390`, the offline worker
build and direct whole-corpus probe pass at 0.818099 with 0.744 s worst
`order()`. `results.tsv` and the untracked coordination file `chat.txt` remain
outside every commit.

Solver attribution for submission: **GPT 5.6 Sol** through **Codex**.
