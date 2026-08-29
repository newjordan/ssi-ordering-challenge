# 0029 — PR346 plus terminal P50 and pedigree strip

## Status

QUALIFIED ON THE EXACT PROMOTED PR346 SOURCE; READY FOR THE FINAL ATOMIC
LIVE-MAIN PREFLIGHT.

This experiment starts from PR346 commit
`e96699fb20a0710adcabb2d4ee4a9ed388037df9`, which promoted at official score
`0.812114` and fill ratio `0.926641` on top of the prior promoted
`f2624122b5b8f19ec4babf7c38cfbe613b2489f9` source. Its published complete
exact-500 score was `0.812113511967`, matching the official rounding. The
production composition is local commit `c40aa0e` and has direct ancestry from
the exact promoted PR346 source.

The patch adds two mechanisms:

1. the terminal completion-lattice descent from PR342 / commit `55fab1f`;
2. one task-neutral ultra-dense pedigree refinement, replacing the existing
   AMF-1.25 task only in a narrow structural slice.

No harness, scorer, dependency, watchdog, corpus, or limit is modified.

## Live frontier and promotion arithmetic

PR340 promoted at official score `0.813547`, fill ratio `0.927601`, producing
source `f262412`. Its published exact reconstruction predicted
`0.813546642795`, matching the official score after rounding. That independent
match matters: all 500 reconstructed dimensions, off-diagonal structural nnz
counts, and baseline AMD flop counts were exact, and the same runner reproduced
two prior official aggregates. This is stronger evidence than extrapolating a
public 300-row score.

The prior 10-relative-bips cutoff from `0.813547` was approximately `0.812733`.
PR346 promoted at `0.812114`, safely below that cutoff. The new cutoff is
approximately `0.811302`. A useful successor must therefore improve PR346 by roughly
`0.000812`; small same-work seed changes alone are no longer enough.

PR344 supplies one calibration point. Its fixed-count TELOS runner-up swap and
equal-work round-two RGSUB seed diversification completed officially at
`0.813378` from the same `f262412` source, a real `-0.000169` gain. It was
rejected only because PR340 had already moved the relative cutoff. This proves
that the seed substitutions inside PR346 transfer in the expected direction,
but also proves that they cannot clear the next threshold by themselves.

## Mechanism A — terminal completion-lattice descent

The pipeline already constructs many chordal completions through AMD/AMF,
metrics, strips, relabelled variants, MINL/watcher/ORBIT, PHARMAKOS, TELOS,
whole-graph exact-game search, RGSUB, and adjacent-pair descent. The important
phase-order gap is that MINL/watcher/ORBIT run before several late transforms.
The final ordering can therefore induce a completion that has never been
minimalized.

The terminal P50 phase runs after adjacent-pair descent, on the actual final
incumbent. It reconstructs the incumbent completion and considers deleting a
fill edge `uv` only when the common neighbourhood `N(u) intersection N(v)` is a
clique. That is the exact local chordality condition for a safe deletion. A
successful scan moves downward in the partial order of chordal completions;
merely changing perfect-elimination order inside one fixed completion cannot do
this because the symbolic factor cost is completion-invariant.

After deletion, the smaller chordal graph is realized by deterministic MCS and
AMD candidates. Each candidate must be a bijection and is admitted only through
the existing exact symbolic flop scorer under strict improvement. The phase is
therefore objective-monotone even if a realization fails or returns an
unhelpful ordering.

The structural envelope is deliberately finite:

```text
sparse cell: n <= 30,000 and nnz <= 100,000
dense cell:  22,000 <= n <= 25,000 and 800,000 <= nnz <= 1,000,000
```

Resource guards are inherited or explicit: completion adjacency at most
2,400,000 entries, at most 1,200,000 fill edges, eight deletion rounds,
common-neighbour sets at most 2,000, and 60,000,000 charged operations. The
intersection cost is charged before the intersection is built, closing the
original discovery probe's under-accounting bug. Budget exhaustion fails closed
and retains already safe deletions.

PR342 is the authoritative isolated hidden calibration. Restacked on the then
current `6a886223` source, it completed the official benchmark at `0.814329`
versus parent `0.815285`, a real `-0.000956` improvement (about 11.7 relative
bips). It did not promote because PR340 moved the crown before PR342 completed;
the rejection was not a build, purity, correctness, determinism, or cap failure.

## Mechanism B — one ultra-dense pedigree task replacement

The promoted dense low-alpha arm already queues AMF-0.75, AMF-1.25, and a
density-routed AMD/strip/quotient candidate in the `400k <= nnz < 1M`,
`nnz > 20n` window. For the narrower slice

```text
n < 10,000
nnz >= 800,000
nnz > 100n
```

the AMF-1.25 position is replaced one-for-one by:

1. rank vertices by descending structural degree, index as total tiebreak;
2. strip the first 512 vertices;
3. order the induced core with non-aggressive AMD and dense handling disabled;
4. append the stripped vertices through the existing deterministic strip helper.

This does not add a candidate, scorer invocation, thread, round, or budget. The
gate is the reconstructed `pedigree_sim2000` class. On the exact reconstructed
pattern, the earlier parent measured `1.000000 -> 0.953819`; PR340's accepted
k=1024 DegSqrt candidate reaches `0.971871`, so the k=512 no-dense AMD strip is a
strictly better independent completion on that target. The public 300 corpus
contains no mover in the replacement slice.

The composed PR346+P50+pedigree worker was also run end to end through the
trusted root harness on a protected-copy instance of that exact reconstructed
pattern on Nimbus. It reproduced flop score `0.953819` and fill score
`0.980798`. This confirms that the production selector reaches the intended
replacement after all earlier PR346 phases; it is not only a helper-level or
isolated-candidate result.

The tradeoff is explicit: this replaces rather than supplements AMF-1.25. A
different unseen ultra-dense sub-10k graph could theoretically need the removed
AMF basin. The gate is narrow, the exact selector audit identifies the pedigree
family, the current DegSqrt candidate remains present as a second dense basin,
and no public row changes. That makes the task-neutral replacement preferable
to widening the already saturated batch.

## Same-machine PR346 A/B on Nimbus

Both trees were built and run on the Nimbus Mac Studio in separate worktrees and
separate Cargo target directories. No MLXFast worker was active when either
scored run began.

Exact PR346 baseline:

```text
lt_1k geomean:    0.8923
1k_10k geomean:  0.8610
gt_10k geomean:  0.7293
score:            0.817692
worst order():    0.927 s
```

PR346 plus terminal P50 plus pedigree replacement:

```text
lt_1k geomean:    0.8921
1k_10k geomean:  0.8605
gt_10k geomean:  0.7264
score:            0.816337
worst order():    0.966 s
```

The measured public delta is `-0.001355`, approximately 16.6 relative bips.
Pedigree is absent from public development, so the visible delta is the marginal
terminal P50 value on the full PR346 pipeline rather than an arithmetic sum of
two local experiments. Scaling the measured public factor from PR346's official
score yields about `0.81077`, roughly `0.00053` below the new `0.811302` cutoff.
This is a projection; the official validator is authoritative.

The public worst call increased only `0.039 s`. The important residual risk is
hidden timing interaction: PR346 conditionally adds a third RGSUB round in much
of the same sparse envelope, and an earlier official sparse component recorded
tail calls near 1.96 seconds. The terminal phase is serial and can therefore
stack work on a row whose upstream search is already expensive. This risk is
why a proposed second terminal seed is deliberately excluded even though strict
score acceptance would make its objective safe. One terminal seed has official
standalone cap evidence, a full composed public watchdog pass, and materially
better promotion evidence; two seeds do not.

## Verification

Nimbus paths:

```text
candidate worktree: /Users/Nimbus/work/repos/matrices-pr346-p50-pedigree
candidate target:   /Users/Nimbus/work/build/matrices-pr346-p50-pedigree
baseline worktree:  /Users/Nimbus/work/repos/matrices-pr346-base
baseline target:    /Users/Nimbus/work/build/matrices-pr346-base
```

Completed checks:

```text
release candidate-worker build: passed
candidate unit suite:            38 passed, 0 failed, 57 ignored
exact equivalence:                3 passed, 0 failed
scorer crosscheck:                2 passed, 0 failed
security boundary:                4 passed, 0 failed
full public timing/score probe:   300/300 valid, worst 0.966 s
exact pedigree root-harness run:  flop 0.953819, fill 0.980798
git diff --check:                 passed
```

Nimbus's Rust installation emits a known post-link `rust-objcopy` warning
because its `libLLVM.dylib` rpath is absent. Cargo still finishes successfully,
the test and worker executables are produced, and every check above executes
those binaries. This is a host strip-tool warning, not a source or link failure.

Independent Fable review found no concrete correctness defect. It confirmed
the pedigree task count is unchanged and the terminal phase is fail-closed with
strict exact admission. Its one material concern was the same serial P50 timing
stack described above. Grok's bounded review attempt exhausted its turns while
loading context and supplied no verdict; it is not used as evidence.

## Dispatch rule and next search

PR346 is now promoted. Immediately before dispatch:

1. fetch `origin/main`;
2. confirm PR346's exact commit remains the promoted source, or restack if it is not;
3. read all live and own Yukon submissions;
4. require a free account slot and no stronger queued result that makes this
   projection stale;
5. require direct ancestry from current main, a clean worktree, passing diff
   check, and unchanged production source hashes.

If another PR promotes before dispatch, rebuild this mechanism on that exact
source and rerun the full 300-row A/B. No old-base overlay is
submission-eligible.

The next independent lane is not more terminal work. It is task-neutral basin
replacement on the remaining exact AMD ties, or a new completion sibling whose
existing paid slot can be reclaimed. Public PR347's one-shot
PHARMAKOS(raw-watcher) composition is a useful official oracle for that idea;
wait for its terminal result before importing it.
