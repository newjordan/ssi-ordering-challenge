# 0026 - P16 cap-headroom plus independent post-final score cell

- Date: 2026-08-29
- Parent: `9c4f7a9aa804af14366ebd241ca1afaec8cc6079`
- Parent policy: official cap-passing P9 hub-heavy relabel-budget deletion
- Candidate: one post-final relabelled AMF pass, `dense_alpha=1.0`, seed `24`
- Acceptance: exact symbolic flops, strict `<`
- Status: validation candidate; no submission from this worktree

## 1. What structure was blocked

The current promoted source `5141ad0` has the best official SSI score,
`0.833396`, but recent attempts to add a strictly better final candidate have
failed the hidden two-second per-matrix cap. The failure was not caused by an
invalid permutation or approximate scoring. P11 through P14 placed a new AMF
candidate after the complete inherited pipeline and retained it only when its
exact symbolic flop count was smaller. On the public corpus that seam was
monotone, but each official run was killed before producing a hidden score.

P13 identified the strongest public AMF cell in that family: relabel seed 24
and `dense_alpha=1.0`. It reduced `crudeoil_pooling_ct3` from 1,021,136 to
972,973 flops, a strict reduction of 48,163. The full public A/B had one win,
zero losses, one changed permutation, and moved the development estimate from
`0.842222684` to `0.842105234`, an absolute gain of approximately `0.000117450`.
However, P13's broad `n=2,000..4,000, nnz<=20,000` envelope failed the hidden
cap. P14 narrowed the gate to a moderate-sparse, moderate-hub graph class and
also failed the hidden cap. Those outcomes retired the AMF cell when it was
stacked directly on the exact promoted source: a score-only addition had no
proven room under the benchmark's wall-clock tail.

P9 exposed the complementary fact. It did not add an ordering family. It
deleted suffix relabel restarts when `nnz <= 150,000 && max_deg > n/50`, using
the same hub discriminator already present in the promoted policy. On the
public corpus it removed exactly 315 AMD/AMF/no-dense-AMD passes and reduced
whole-corpus wall time directionally. More importantly, official submission
`8e25fd0e-b564-4f75-9fa4-9c508d59fac5` completed the hidden 500-matrix
benchmark and all time-cap checks. P9 is therefore the only recent source with
official evidence that its work-deletion envelope clears the cap.

P9 did not promote because its hidden score was `0.833436`, which is
`0.000040` worse than the `0.833396` frontier. Its public score moved slightly
in the good direction, `0.842222684 -> 0.842221471`, but an upstream deletion
can change the incumbent that seeds TELOS and therefore is not score-monotone
on unseen graphs. The structural block is consequently two-dimensional:

1. exact promoted source plus the post-final AMF cell has enough public score
   gain but repeatedly lacks hidden cap margin;
2. P9 has official cap margin but a small hidden quality regression.

Neither component alone can promote. P16 tests whether their complementary
properties can be composed without mixing their causal roles.

## 2. How the block can be opened

P16 starts from the exact P9 commit, not from a manually reconstructed source.
Every P9 production change remains intact: the relabel budget is 150,000 only
for hub-heavy graphs below the 150,000-nnz ceiling and 300,000 elsewhere, and
the selected budget is reused by the inherited relabelled AMD, AMF, and
no-dense AMD families. No P9 seed, gate, phase, or budget is changed.

After the entire inherited portfolio has completed, including TELOS, MINL,
ORBIT, and PHARMAKOS, P16 evaluates one independent candidate only under the
P14 structural envelope:

```rust
(2_400..=3_000).contains(&n)
    && 4 * n <= nnz && nnz <= 6 * n
    && (32..=128).contains(&max_deg)
```

The gate is evaluated before the relabel vector, permuted pattern, i32 arrays,
or AMF state are allocated. It is a graph-class predicate based only on
dimension, stored structural work, and maximum column degree. It contains no
matrix name, fingerprint, corpus index, exact public dimensions, timer,
environment variable, or result-dependent branch. The public census of the
former broad gate admits one row: `crudeoil_pooling_ct3`, with `n=2,644`,
`nnz=11,426`, and `max_deg=84`. High-hub and regular low-degree rows are
excluded before added work.

Inside the gate P16 performs one fixed operation:

1. construct the deterministic Fisher-Yates relabeling for seed 24;
2. permute the original scoring pattern by that relabeling;
3. run `feral_amf` once with `dense_alpha=1.0`;
4. compose the AMF permutation back to original vertex numbering;
5. reject any non-bijection;
6. score the candidate with the existing exact `ScoreWorkspace` path;
7. replace the inherited P9 final only if `candidate_flops < best_flops`.

This location is the critical causal seam. P16 cannot change which upstream
candidates are evaluated, cannot change the incumbent supplied to TELOS,
cannot change TELOS's basin, and cannot alter MINL, ORBIT, or PHARMAKOS. On a
tie or loss it returns the exact inherited P9 permutation. The added score
cell is therefore source-level monotone relative to P9, even though P9 itself
is not monotone relative to the promoted source.

The public effects are expected to compose cleanly. P9's only public output
change was `pooling_sppa0pq`; its narrow-budget rule did not change the final
permutation of the P13 winner. For `crudeoil_pooling_ct3`, the inherited
low-nnz restart floor dominates both the 300k and 150k ordinary budgets, so
P9 removes no effective restart and leaves the P13 baseline at 1,021,136.
The post-final cell can therefore reproduce the independent 972,973 result.
Using the measured public score deltas, the combined estimate is about
`0.842104021`: P9's `-0.000001213` plus the post-final cell's approximately
`-0.000117450`. That public gain is almost three times the magnitude of P9's
official `+0.000040` hidden regression. It does not guarantee a hidden win,
but it clears the requested plausibility threshold for an official test.

## 3. How further ascent can proceed

P16 is deliberately one binary decision, not a parameter sweep. If it passes
the cap and scores below `0.833396`, the promoted state should retain both
causal layers: P9 as the proven work-deletion envelope and P16 as a strict
post-final quality layer. Future score experiments should be attached only
after the complete inherited pipeline and should use the same strict exact
comparison. That preserves the ability to attribute a regression to one new
family rather than to a changed TELOS seed basin.

If P16 passes the cap but does not beat `0.833396`, the interpretation is also
clear. Because the post-final cell cannot worsen P9's output, its hidden score
must be no worse than P9's `0.833436` on each evaluated graph. A score between
those values means the seed-24 AMF cell generalized but did not find enough
weighted hidden improvement to erase all of P9's upstream regression. The
next ascent should keep P9 and add a different independently strict post-final
family, preferably one whose public win lies in a different size bucket or
graph class. Widening the AMF gate or shipping multiple nearby AMF seeds would
spend cap margin without independent evidence and is not justified.

If P16 fails the hidden cap, the P14 AMF family is retired even under the P9
envelope. The next candidate must be cheaper by construction: a bounded local
permutation repair, a score-neutral representation deletion that has official
cap evidence, or a different ordering family with substantially lower
complexity than AMF. The response must not be a narrower identity-like gate.

If P16 is invalid, nondeterministic, or fails purity, the implementation is
rejected regardless of public score. Correctness and deterministic execution
remain hard gates. A local win is evidence for a submission, not evidence of
first place.

## 4. Where structural changes should happen

There are four stable seams in the ordering pipeline, and each has a distinct
role:

- **Before TELOS:** only auditable work deletions with official cap evidence.
  These changes may affect quality through seed-basin movement, so they need
  an independent post-final score layer and official validation.
- **After PHARMAKOS, before return:** new quality candidates. They must be
  structurally gated, deterministic, scored by exact symbolic flops, and
  accepted only on strict improvement. This is the preferred ascent seam.
- **Inside scoring/allocation utilities:** output-neutral performance work.
  Such changes require byte-identical permutation proofs and should not be
  bundled with a new heuristic unless the combination is the explicit causal
  question, as it is here with already-official P9 evidence.
- **Orchestrator memory and experiment notes:** evidence, retirement rules,
  and next decisions. The orchestrator should read official score and cap
  outcomes first, then the public A/B ledger, and only then decide whether to
  promote, retry, narrow, or switch families.

P16 changes only the second seam. Its parent already embodies the first seam.
It does not touch the scoring utility, corpus, harness, dependencies, security
boundary, or benchmark logic.

## Validation and decision ledger

- Parent SHA is exactly `9c4f7a9aa804af14366ebd241ca1afaec8cc6079`.
- P9 official hidden cap result: pass through full benchmark and upload.
- P9 official score: `0.833436`, `+0.000040` versus frontier.
- P9 public A/B: `0.842222684 -> 0.842221471`, 1 win / 0 losses.
- P13 cell public A/B: `0.842222684 -> 0.842105234`, 1 win / 0 losses.
- P13 winner: `crudeoil_pooling_ct3`, 1,021,136 -> 972,973 flops.
- P16 gate is the P14 moderate structural gate and is checked before work.
- Candidate placement is after complete PHARMAKOS.
- Candidate acceptance is exact strict `<`; ties and losses keep P9.
- Production code change is confined to `src/ordering/mod.rs`.
- No dependency, manifest, harness, scoring, or corpus change is included.
- Expected combined public estimate: approximately `0.842104021`.
- Expected public gain magnitude exceeds P9's hidden regression magnitude.

Before submission, the release source must compile and the final commit diff
against P9 must contain only this note and the intended `mod.rs` block. The
official Linux purity, security, deterministic double-run, bijection, and
time-cap checks remain authoritative. Repeated local wall timings are
directional only because the cap uses noisy shared-runner wall time.

## Promotion rule

SSI is lower-is-better. P16 is promoted only if the official hidden score is
strictly below `0.833396` and the source promotion completes. A local public
estimate, successful compile, PR creation, passing security workflow, or API
state of `validating` is not first place. Until both official score and source
promotion are recorded, this file describes a candidate, not a victory.
