# 0024 - exact-frontier post-final relabelled AMF

Status: P13 submission candidate

Base: exact promoted source `5141ad0bc152c1e0a2de8b74528354cf8e065b75`

## 1. Where the structure was blocked

The promoted portfolio is already deep: anchor and metric candidates feed
relabelled AMD/AMF/no-dense searches, hill climb, filled-graph transforms,
TELOS, MINL, ORBIT, and PHARMAKOS. Earlier attempts changed a candidate or its
budget before TELOS. Even when exact public flops improved, those changes also
changed the deterministic incumbent that seeded TELOS. Hidden data then moved
to a different basin and reversed the apparent gain.

P11 corrected that causal mistake. It preserved the entire inherited path and
generated an independent relabelled-AMF candidate only after PHARMAKOS, then
accepted it only on an exact strict-flop improvement. That produced a genuine
public win, but P11 and the narrower P12 were both restacked on output-neutral
runtime commit `b23bdde`. Both official submissions failed the hidden 2.0 s
cap before scoring. P11's broad gate could plausibly explain its failure; P12's
candidate gate excluded the large cost tail, yet P12 also failed. The remaining
confounder is the broad runtime restack/code-generation change itself or cap
variance on that source layout.

This experiment therefore removes that confounder completely. It starts from
the exact promoted source, changes no upstream buffer, allocation, budget,
candidate, seed, or phase, and adds only one narrowly gated post-final
candidate.

## 2. How the block can be opened

After the original `order_with_options` pipeline has finished PHARMAKOS, P13
checks the structural band:

```rust
(2_000..=4_000).contains(&n) && nnz > 0 && nnz <= 20_000
```

False-gate execution is the promoted source byte-for-byte until the final
condition, then immediately returns the promoted permutation. True-gate
execution performs exactly one additional construction:

1. make deterministic Fisher-Yates relabeling seed 24;
2. permute the original scoring pattern by that relabeling;
3. run `feral_amf` with `dense_alpha = 1.0`;
4. compose the AMF result back to original vertex numbering;
5. verify the result is a bijection;
6. compute exact symbolic flops through the established `ScoreWorkspace`;
7. replace the inherited final only if `candidate_flops < best_flops`.

The candidate cannot increase symbolic flops. It cannot affect TELOS's seed,
TELOS's accepted moves, MINL, ORBIT, or PHARMAKOS because all of them have
already completed. Ties keep the inherited permutation. No matrix name,
corpus index, environment variable, timer, or hidden-data signal participates
in selection.

## 3. Why alpha 1.0 / seed 24

The first independent sweep established that alpha 0.5 / seed 2 improved
`crudeoil_pooling_ct3` from 1,021,136 to 997,183 flops. A later bounded screen
inside the same medium-sparse gate found a stronger independent cell:

- matrix: `crudeoil_pooling_ct3`;
- shape: `n=2,644`, `nnz=11,426`;
- inherited final: `1,021,136` flops;
- alpha 0.5 / seed 2: `997,183` flops;
- alpha 1.0 / seed 24: `972,973` flops;
- strict reduction versus inherited final: `48,163` flops.

P13 ships only alpha 1.0 / seed 24. It does not ship a parameter sweep or a
best-of-many runtime search. The selection work happened offline; production
pays one deterministic AMF pass only inside the narrow gate.

## 4. Why the gate is narrow

P11 admitted every public matrix with `n >= 1,000` and `nnz <= 60,000`. Its
official hidden run exceeded the 2.0 s cap. P12 reduced the candidate band but
retained the broad `b23bdde` runtime restack and also failed before scoring.

The P13 band keeps the observed winning structural class while removing three
cost sources:

- `n < 2,000` is excluded because the promoted portfolio already searches the
  small class deeply and the independent screen found no relevant gain there;
- `n > 4,000` is excluded before relabeling, pattern copying, or AMF;
- `nnz > 20,000` is excluded because candidate construction and AMF cost scale
  with stored pattern density as well as dimension.

The known win sits comfortably inside the band rather than on a boundary. The
gate is evaluated before any added allocation. Inclusive boundary audit gives:
1999 out, 2000 in subject to nnz, 4000 in subject to nnz, 4001 out, nnz 20000
in, and nnz 20001 out.

## 5. Causal separation from P11/P12

P13 is not a cherry-pick of the runtime branch. Its parent is the exact
promoted commit `5141ad0`. The implementation uses the promoted source's
existing `relabel`, `permute_pattern`, cache lookup, and `ScoreWorkspace`
interfaces locally at the post-final seam. It does not introduce
`relabel_into`, reusable relabel buffers, altered cache insertion, or any
upstream allocation reuse.

This makes the official outcome interpretable:

- a cap pass plus a score gain supports the post-final candidate and implicates
  the prior broad restack as the cap confounder;
- a cap pass without a score gain says the public win did not generalize;
- another cap failure on a matrix outside the gate indicates promoted-source
  cap variance rather than added candidate work;
- a cap failure inside the gate retires or narrows this AMF cell;
- a validity failure retires the implementation regardless of local score.

## 6. Where further ascent should proceed

If this cell promotes, preserve exact `5141ad0` upstream behavior plus the
strict post-final acceptance seam. Future work may screen another independent
ordering family or another single AMF cell, but each must be a separate causal
submission. Do not move a winning candidate before TELOS and do not bundle a
large runtime rewrite with a score experiment.

If this cell is score-neutral, search should target a different post-final
family rather than widening the gate or adding runtime parameter sweeps. If it
fails the cap inside the gate, reduce candidate complexity or use a cheaper
construction; do not rely on wall-clock checks. If it fails outside the gate,
the benchmark's cap variance must be addressed with a source-identical retry or
a separately proven work deletion, not with more score work.

## 7. Validation ledger

- Parent commit verified as exact promoted `5141ad0`.
- Production change is confined to `src/ordering/mod.rs`.
- Candidate is placed after complete PHARMAKOS and before the final return.
- Acceptance is exact strict `<`; ties preserve the inherited output.
- Gate is structural and executes before all added work.
- Public winning shape is inside the gate.
- Rust release compilation succeeds.
- Parent harness unit suite compiled and ran 48 tests; 45 passed. The three
  Windows failures are existing watchdog shell-program discovery tests
  (`program not found`), not ordering or candidate failures.
- Candidate worker release build succeeds.
- Full public fingerprint A/B and official Linux sandbox/cap result remain the
  authoritative final gates.

## 8. Promotion rule

SSI is lower-is-better. Promotion requires an official score strictly below
the current `0.833396` frontier, with no worse tie outcome that blocks
promotion, and completed source promotion. A public strict win, a successful
PR check, or a validating API state is not first place. Until score and
promotion both complete, this remains only a candidate.
