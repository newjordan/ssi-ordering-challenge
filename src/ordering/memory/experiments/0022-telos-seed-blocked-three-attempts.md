<!-- 0022 -->
# 0022 - RESOLVED: every submission failure here is a 2.0s cap breach on a HIDDEN matrix

- **Date:** 2026-08-28
- **Score:** no change shipped. Promoted candidate remains graded `0.833396` (#1).
- **Status:** **CAUSE RESOLVED, five for five.** The grader's verdict is public — read it with
  `~/ssi-ordering-LEARNINGS/bin/ssi-grader-log.sh` before theorising about any `failed` row.
  Every packaged failure ends `RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix
  cap and was killed`. The binding matrix is NOT in the dev corpus, passes `max_deg <= n / 50`,
  and sits within ~0.1-0.3 s of the cap — so the promoted candidate is itself near the cap on it.
- **Acceptance criterion this establishes:** a candidate must REDUCE per-matrix time, or add time
  only where the matrix is provably fast (small `nnz`, as the shipped 600k floor does). Net dev
  score, unchanged dev worst-case, unchanged worst-case round count, and per-matrix
  zero-downside are each INSUFFICIENT — all four were tried and failed. See the RESOLVED
  section at the end for the evidence table and the next lever (a genuine speedup, paired with a
  score gain in one submission).
- **Reading note:** the two analyses in the body below (a "TELOS code region" cause, then an
  "added time on mid-distribution matrices" cause) are superseded in that order. The first was
  wrong; the second was directionally right and is now confirmed precisely. They are kept
  because the sequence of refutations is the useful part.

## What is being attempted

Exploit 0020's finding that `TELOS-GRADIENT DESCENT` is seed-sensitive, by descending from
the AMD anchor as well as the incumbent. Measured value on dev: **6 matrices better, 0
worse**, dev score `0.842223 -> 0.842032` (additive form) or `-> 0.842015` (budget-split
form). Both forms are real gains. Neither can be shipped.

## Three attempts, one controlled variable

| submission | form | worst dev `order()` | total bench time | outcome |
|:--|:--|--:|--:|:--|
| `b904857` | additive 2nd descent, gate `nnz < TELOS_MAX_NNZ` | 1.828 s | ~221 s | **failed** |
| `f6e0c2d` | **no TELOS change** (restart floors only) | 1.784 s | 216 s | **promoted 0.833396** |
| `3689919` | additive 2nd descent, gate `nnz <= 40_000` | 1.849 s | ~221 s | **failed** |
| `d390ef6` | budget-SPLIT: 1 round incumbent + 1 round anchor | 1.818 s | ~221 s | **failed** |

**That inference was initially UNSOUND, and an adviser was right to challenge it.** This
grader fails a large fraction of submissions regardless of content. Counting every row on the
benchmark:

| solver | failed | total | rate |
|:--|--:|--:|--:|
| fkiene | 12 | 31 | 39% |
| Inah-choi | 1 | 2 | 50% |
| **all solvers** | **16** | **40** | **40%** |

At a ~40% independent failure rate, three consecutive failures have probability `0.4^3` =
**6.4%**, and the one non-TELOS success had a ~60% chance of passing whatever it contained.
So the original three-attempt argument rested on ruling out a 1-in-16 coincidence.

**Resolved by a free discriminator:** a failed submission cannot demote a promoted one, so
byte-identical content was resubmitted (`e1bf08e`, identical to `d390ef6`). It failed again.

| attempt | content | outcome |
|:--|:--|:--|
| `d390ef6` | budget-split | failed |
| `e1bf08e` | **byte-identical to `d390ef6`** | failed |

Now 4 failures on TELOS-containing variants against 1 success without, including a direct
replication on identical bytes. `P(4 consecutive | flaky at 40%)` = `0.4^4` = **2.6%**, and
that 40% is the TOTAL observed failure rate - it includes other solvers' content-caused
failures, so the pure-infrastructure rate is at most 40% and 2.6% is a conservative upper
bound. Content-determinism is now the reasonable reading, and the park is justified on
evidence rather than on a coincidence that had not been excluded.

## Hypotheses refuted, each by measurement not argument

1. **Global wall-clock budget on the benchmark command.** Timed back to back on an idle
   machine: promoted **216 s**, TELOS variant **221 s**. `+2.3%`. Dead. (My earlier
   180->250-330 s figures were contaminated by an included load-wait loop - my instrumentation
   error, not a property of the build.)
2. **Per-matrix cap via added time.** Attempt `d390ef6` splits the EXISTING round budget:
   `rounds_per_seed = max_rounds / seeds`, so the total number of descent rounds is
   unchanged and the worst case is exactly what the promoted build already does. It failed
   anyway. This kills added-time as the mechanism.
3. **Corrupted / stateful `ws.flops`.** Re-scored **every** `consider` candidate against the
   workspace-free `flops_of` path across all 300 matrices: **0 mismatches**. `flops_cache` is
   a per-call local `HashMap` (`mod.rs:1574`), so no cross-matrix contamination either.
4. **Fatal warnings.** No `-D warnings` anywhere in the harness; the 7 warnings the crate
   emits are pre-existing on the promoted build too.
5. **The `#[ignore]`d byte-identical-to-`legacy` equivalence test.** The benchmark command is
   `bash scripts/local-candidate-build.sh && cargo run --release` - no `cargo test` at all.
6. **Build sandbox.** `local-candidate-build.sh` builds under bubblewrap with network denied
   and writes confined to target/cargo/tmp. The change interacts with none of it.
7. **Memory.** Peak extra allocation is ~4 permutations of `n` (tier A caps `n` well under
   150k), i.e. single-digit MB against a 4 GiB per-matrix cap.

## What is still standing

Something about the restructured loop fails in the grader environment on input we cannot
see. The residual suspects, none testable from here:

- a graded matrix that makes the anchor-seeded descent take a pathological path;
- a panic reachable only on a matrix shape absent from the 300-matrix dev corpus (the
  `std::mem::take(&mut best_perm)` in the no-anchor branch leaves `best_perm` empty until the
  loop's first assignment - locally always restored, and `telos_out` is unconditionally set
  once per iteration, but this is the one novel invariant the change introduces);
- an interaction with the parent process's per-matrix supervision that the local
  `yukon run` path does not exercise identically.

## Decision, and the discipline behind it

Stopped after four attempts. Expected value is `-0.0002` of dev score against a 3-for-3
failure rate and no diagnostic channel. A failed submission cannot demote a promoted one, so
the attempts were positionally free - but they are not free in time, and the same hours buy
more elsewhere.

Preserved so a future session with a grader log can finish it in one step: the exact
budget-split patch is the cheapest form to retry, because its worst-case work is provably
identical to the shipped build, which removes an entire class of explanation up front.

## Method note worth keeping

The measurement harness (`~/ssi-ordering-LEARNINGS/bin/ssi-measure.sh`) earned its keep here.
`diff promoted` proved the tree was byte-equivalent to the promoted submission after each
revert (0 better, 0 worse over 300 matrices) - which is a far stronger check than "the score
matches", and it is what let three risky variants be tried and backed out without ever
endangering the live #1.


## SUPERSEDED CONCLUSION (2026-08-28, later the same day)

Two further submissions changed the picture, and the "TELOS code region" framing above is now
the wrong generalisation. Read this section instead.

### A whole failure class was mis-read: `commit` tells you if the candidate ever ran

Three attempts failed within seconds. They carry **no commit** (`-`) in `yukon submissions`,
where every submission that actually executed carries a hash. Cause was purely local:
`origin/main` had advanced to `5141ad0` - the commit of our own promoted submission - leaving
the checkout one behind, and `yukon submit` builds its commit on the current base. Fixed with
`git stash push -u` + `git pull --ff-only` + re-apply (NOT `yukon sync --force`, which discards
editable paths).

**Standing rule: after your own submission is promoted, your checkout is one commit behind and
every later submission fails to package until you catch up.** It looks exactly like a candidate
failure. Always check the commit column before drawing any conclusion from a `failed` row.

### The real pattern is ADDED TIME ON MID-DISTRIBUTION MATRICES, not a code region

Restated over every submission that packaged and ran:

| submission | change | added per-matrix time | outcome |
|:--|:--|:--|:--|
| `f6e0c2d` | 600k low-nnz floor | ~0.25 s, only `nnz <= 20_000` | **promoted** |
| `b842115` | 1.5M low-nnz floor | ~0.62 s, same class | failed |
| `b904857`, `3689919` | additive 2nd TELOS descent | up to +0.314 s incl. mid-pack | failed |
| `d390ef6`, `e1bf08e` | budget-split TELOS | +0.12 s on mid-pack (`crudeoil_lee4_10` 1.354 -> 1.473) | failed |

The one change that shipped adds time **only to small, fast matrices**. Every change that
touched mid-distribution matrices failed, including one whose worst-case round count was
provably unchanged and one that never touched TELOS at all. So the discriminator is not which
code region was edited - it is whether any matrix in the middle of the time distribution got
slower.

That is consistent with the graded corpus holding at least one matrix close enough to the 2.0 s
cap that a ~0.1-0.3 s addition breaches it, which dev worst-case cannot reveal because dev worst
is `ringpack_30_2` and every gate here excludes it.

### Consequence for the next attempt

A future increment on this benchmark should either **reduce** per-matrix time, or add time only
where the matrix is provably fast (small `nnz`, as the promoted floor does). "Net dev score
improves" is not sufficient, and neither is "dev worst-case is unchanged". The parked TELOS
increment fails this test as written, which is a better reason to park it than the code-region
argument above.


## RESOLVED (2026-08-28): the grader log is public, and every failure is one cap breach

An adviser pointed at 0010's reconstructed eval corpus as a way to test the near-cap theory
locally. That artifact is gone from disk (`~/ssi-ordering-LEARNINGS/data/` does not exist) and
would have been stale anyway - 0010 calibrated it against a round that graded `0.8844`, while
this round grades `~0.834`, i.e. before the corpus rebaseline. But the same note contained the
thing that actually mattered: **the grader's per-matrix output goes to a PUBLIC GitHub Actions
log.** That is the diagnostic channel I spent the whole session assuming did not exist.

```sh
gh run list -R Layr-Labs/ssi-ordering-challenge --limit 60 \
  --json databaseId,name,headBranch
gh run view <databaseId> -R Layr-Labs/ssi-ordering-challenge --log-failed
```

The `benchmark` workflow run for each submission is on a branch named
`submissions/<submission-uuid>`. Every one of our packaged failures ends with the same line:

```
RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed
```

| submission | change | verdict |
|:--|:--|:--|
| `3689919` | additive 2nd TELOS descent, `nnz <= 40_000` | cap breach, hidden matrix |
| `d390ef6` | budget-split TELOS | cap breach, hidden matrix |
| `e1bf08e` | byte-identical retry of `d390ef6` | cap breach, hidden matrix |
| `b842115` | 1.5M low-nnz floor (no TELOS involvement) | cap breach, hidden matrix |

So the near-cap theory is confirmed outright, and every competing explanation is dead: not the
TELOS code region, not a global time budget, not the scorer, not flaky infrastructure. Four
different candidates, one cause.

### What the word "hidden" tells us

The grader withholds the matrix name, so this matrix is **not in the 300-matrix dev corpus** and
cannot be measured locally. Two further deductions:

1. It is **slower than any comparable dev matrix**, sitting within ~0.1-0.3 s of the 2.0 s cap.

2. **CORRECTION (an adviser caught this):** an earlier version of this section claimed the matrix
   must pass `max_deg <= n / 50` because "every gate we ship is conditioned on that predicate".
   That is false. The mid-band floor and the TELOS second seed are hub-gated; the **low-nnz floor
   is deliberately NOT** - it is plain `if nnz <= 20_000`, with no `max_deg` term, precisely so
   that `chimera_lga-01` (max degree ~1119 of `n = 1120`) is admitted (see 0019). So the `1.5M`
   breach need not involve a hub-passing matrix at all, and **"hidden matrix" may be plural**: a
   small-`nnz` one breached by the floor, and a hub-passing mid-pack one breached by TELOS.

3. **The cost bound is weaker than claimed, in the same place.** The `~4.1e-7 s/nnz` per pass was
   fitted on dev matrices and is LINEAR in `nnz`. AMD's cost is not linear in `nnz` on a dense
   pattern, and 0005 records exactly that class in eval - `pb302035` at `n=670, nnz=333040`,
   `nnz/n ~= 497`, with the note that "AMD on a pattern with average degree 497 is the textbook
   blowup". The shipped `small_multi` hands a matrix up to **200 restarts** with no density gate,
   so a dense small-`nnz` eval matrix could be near the cap **under the promoted build itself**.
   That is an open risk to the current #1, not merely to future candidates.

### The eval census is no longer available (0005's route is closed)

0005 mined the per-matrix table (name, n, nnz, flops, ratio) from the public Actions log, and
that is how it identified `telecomsp_metro` by name after a cap FAIL. That leak is gone. Our own
promoted run (`33174088716`) prints only the aggregate line
`score (weighted mean of per-bucket geomean flop ratios, lower is better): 0.8334` and uploads a
209-byte `score.zip`; the log contains **zero** occurrences of `nnz`. This is also why the failure
message now says "hidden matrix" where 0005's said `telecomsp_metro`. Do not plan around mining
the census; it has to be inferred or reconstructed.

That also means our own promoted candidate is running close to the cap on it, so a slower grader
or a noisy run could time out a submission that has already been promoted.

### The acceptance criterion this establishes

For this benchmark, a candidate must **reduce per-matrix time, or add time only where the matrix
is provably fast** (small `nnz`, which is exactly what the promoted 600k floor does and why it is
the only thing that shipped). The following are all insufficient, and each was tried:

- net dev score improves (`1.5M floor`, `-0.000240`, failed);
- dev worst-case unchanged (`ringpack_30_2` dominates dev worst and every gate excludes it);
- worst-case round count provably unchanged (`budget-split TELOS`, failed);
- per-matrix zero-downside on score (`additive TELOS`, 6 better / 0 worse, failed).

### Cheapest next step, for whoever picks this up

Stop proposing score-adding increments until there is headroom. The lever is a **time reduction**, and per the
correction above it may belong in the **dense small-`nnz` class** rather than only the hub-heavy
large one - `small_multi` is the one shipped path with no density gate, and 0005's precedent is
that a density re-gate is what fixes an eval cap FAIL. Note such a re-gate is protective, i.e.
score-neutral-or-slightly-worse on dev, so it cannot ship alone (it would grade at 0.00% and be
rejected); it has to ride with a score gain. The other candidate remains a reduction which buys margin on the hidden matrix and simultaneously protects
the promoted #1 from a flaky timeout. The recorded `RELABEL_BUDGET 300_000 -> 150_000` free rider
(score-neutral, `-0.026 s` worst case) is the only measured candidate of that shape so far and is
too small alone - it would grade at `0.00%` and be rejected. Pair a real speedup with a score
gain in one submission.


## Quantified: inside `small_multi`'s gate the risk is large-`n` LOW density, not dense-small

The dense-small worry above is real in principle and bounded in practice. Measured over the 221
dev matrices in `small_multi`'s range (`nnz <= 20_000`) under the promoted build:

| | matrix | n | nnz | nnz/n | time |
|:--|:--|--:|--:|--:|--:|
| densest | `graphpart_clique-70` | 280 | 14910 | 53.2 | **0.176 s** |
| 2nd densest | `autocorr_bern40-30` | 43 | 1534 | 35.7 | 0.034 s |
| slowest | `crudeoil_lee1_07` | 3670 | 19322 | 5.3 | **1.197 s** [see correction] |
| 2nd slowest | `chimera_rfr-02` | 2032 | 15140 | 7.5 | 0.968 s |

Time tracks `n`, not density, and the reason is structural: `nnz <= 20_000` together with high
density FORCES small `n` (at `nnz/n = 100`, `n <= 200`), and a 200-node pattern is cheap however
dense it is. So a `pb302035`-class blowup (`nnz/n ~= 497`) cannot occur inside this gate, and
`pb302035` itself has `nnz = 333_040` and is outside it regardless. Dev's densest small matrix is
`53.2` and it is one of the fastest in the class.

The exposure is the opposite shape: **large `n`, low density, near the gate's `nnz` ceiling.**
`crudeoil_lee1_07` is the dev proxy at `1.197 s`, and it is exactly the matrix the `3_000_000`
sweep drove to `1.832 s` (`+1.226 s` for 155 passes). That is consistent with `b842115` (the
`1.5M` floor, `~+0.62 s`) breaching on an eval matrix of that shape.

### So the lever, sharpened again

A density re-gate on `small_multi` (0005's shape of fix) would be aimed at the wrong risk here.
The right protective bound for this gate is on **`n`**, not on `nnz/n`: cost in this class rises
with `n` at fixed `nnz`, and the matrices that pay for the floor's gains are small-`n`. A gate of
the form "raise the budget only where `n` is small" would keep the measured gains while removing
the near-cap exposure the `1.5M` attempt demonstrated - and unlike a density gate it is not merely
protective, so it has a chance of being score-positive rather than a 0.00% rejection.

Untested. Recorded as the concrete next candidate rather than another guess, and it should be
measured against `crudeoil_lee1_07`'s time specifically, since that is the dev matrix standing in
for the hidden one.


## CORRECTION to the table above, and the gain/risk split

The `1.197 s` I recorded for `crudeoil_lee1_07` was measured under the **1.5M** configuration,
not the promoted one - `measurements/last-time.log` still held the `b842115` run when I did that
analysis. Under the shipped 600k budget the same matrix measures **0.734 s**. The
density-vs-`n` conclusion is unaffected (time tracks `n`, not `nnz/n`), but the number was
mis-attributed, and the corrected figures make the cost model look better rather than worse:
1.5M added `~0.46 s` to it against a predicted `~0.62 s`.

### 65% of that gain was ON the matrices that break the run

Splitting the `1.5M` budget by `n` and re-measuring:

| configuration | dev score | delta vs promoted | share of the 1.5M gain |
|:--|--:|--:|--:|
| promoted (600k everywhere) | 0.842223 | - | - |
| 1.5M everywhere (`b842115`, FAILED) | 0.841983 | `-0.000240` | 100% |
| **1.5M only where `n <= 1_500`** | **0.842138** | `-0.000085` | **35%** |

So two thirds of the gain lives on the large-`n`, low-density matrices near the gate's `nnz`
ceiling - which are precisely the ones whose time the 1.5M budget inflated into a cap breach. The
gain and the risk are the SAME matrices. That is worth stating plainly, because it means this
particular lever cannot be made both safe and fully profitable: bounding it by `n` keeps a third
of the value and gives up the rest by construction.

The `n <= 1_500` form is safe by construction on the demonstrated failure: `crudeoil_lee1_07`
(`n = 3670`) keeps the shipped 600k budget and its time is unchanged, and worst-case plus the
near-cap census (`>1.4 s`: 2, `>1.7 s`: 1) are identical to the promoted build.
