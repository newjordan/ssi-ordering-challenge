# 0025 — restack the cap-neutral varied slot onto promoted PEP600

- **Date:** 2026-08-29
- **Exact parent:** `28652c6c76d2f306d2766fec3f59a6585baaae0e`
- **Parent official:** FLOPS **0.821800**, fill **0.931791**, promoted
- **Production delta:** one slot replacement in `src/ordering/rgreedy.rs`
- **Exact public A/B:** 0.819884 → **0.819790** (−0.000094)
- **Status:** clean restack; prepared for orchestration, not submitted here

## 1. What changed in the live structure

P27 has now promoted the PEP budget increase. The live source already runs four
fixed default-policy PEP streams at 600M deterministic word-operations per
stream inside `n <= 3000`, `0 < nnz <= 60000`. The official grader accepted it
at 0.821800 versus the preceding 0.822002 frontier, and promotion completed to
source `28652c6`. That official terminal result is important for two reasons:

1. the 600M depth improves the hidden score by 0.000202;
2. the exact four-stream PEP600 envelope clears the official per-matrix cap.

The P29 research checkout had been built independently from `5e96293` while P27
was validating. It combined the same PEP600 budget with a cap-neutral policy
mix. With P27 promoted, the budget line is no longer part of the candidate.
Only the measured slot replacement should remain.

## 2. Production delta on the promoted parent

The parent creates four PEP specs from four fixed seeds and
`Params::DEFAULT`. The restack preserves default slots 0, 1 and 2 and replaces
slot 3 with the existing varied schedule's slot 3:

```text
specs[3] = (stream_rng(3), stream_params(3), budget)
```

`stream_params(3)` keeps the default alternating uniform-prefix/log-tail mode
but narrows the slack-policy mask to `0b0011_0111`. It therefore changes the
policy basin and PRNG trajectory without changing the number of streams or the
budget passed to any stream.

There is no `mod.rs` production change in this restack. P27 already owns
`RGREEDY_PEP_BUDGET = 600_000_000`. The legacy RGS branch above 60k remains at
200M with its existing density guard. RGSUB, relabel multistart, TELOS,
adjacent-pair descent, scorer, watchdog, dependencies and gates are untouched.

## 3. Why this result is not assumed additive

The 300M slot mix and 600M depth operate on the same trajectories, so their
deltas cannot be summed. The required experiment used default PEP600 as its
baseline and changed only slot 3 at that same depth. On exact source `5e96293`,
which differs from the promoted P27 production tree only by the now-promoted
budget line, the full 300-pattern comparison measured:

| bucket | PEP600 default | PEP600 + varied slot 3 | delta |
|---|---:|---:|---:|
| `lt_1k` | **0.8922** | 0.8924 | +0.0002 |
| `1k_10k` | 0.8623 | **0.8617** | −0.0006 |
| `gt_10k` | **0.7338** | **0.7338** | 0 |
| **total** | **0.819884** | **0.819790** | **−0.000094** |

At displayed row precision, 12 matrices improve, 10 regress and 278 are
unchanged. The decisive moves are:

- `crudeoil_pooling_ct3`: 0.6865 → **0.6499**
- `chimera_k64ising-02`: 0.6131 → **0.6047**
- `multiplants_stg1b`: 0.6154 → **0.6104**
- `chimera_mgw-c16-2031-01`: 0.8445 → **0.8407**
- `pooling_adhya4pq`: 0.7930 → **0.8198** (largest loss)
- `ndcc13`: 0.6370 → **0.6442** (second-largest loss)

The mix intentionally trades some small-tier default-seed coverage for a larger
mid-tier gain. The heavy bucket is outside the PEP gate and remains unchanged.
The causal claim is −0.000094 relative to PEP600, not the larger difference
relative to the older PEP300 source.

## 4. Exact object audit and verification transfer

This restack was audited against the already-tested P29 production objects,
commit `009fad2f7e91817bd7864290495021e15130576d`:

```text
file                         promoted+restack blob
src/ordering/mod.rs          a3fff2c75cda418e4a2c52d9e9b4fcbc125c3d8c
src/ordering/rgreedy.rs      3112d09621ec99e2b0273461e01d54d9614f925b
```

Both hashes exactly equal the corresponding P29 blobs. A whole production-path
diff against P29, excluding research memory documents, is empty. P27's own
`5e96293..28652c6` diff changes only the PEP budget line plus its research note;
there is no hidden executable drift to absorb the slot result.

Because the executable source object is byte-identical, the existing P29
verification transfers exactly rather than probabilistically:

- network-denied release suite: **38 passed, 0 failed, 54 ignored**;
- network-denied offline locked candidate build: passed;
- full-300 PEP600 baseline and mixed candidate artifacts completed normally;
- `git diff --check`: passed.

Rebuilding an identical source object would add no new code evidence. The new
evidence since P29 is stronger: P27's official validation proves the shared
PEP600 envelope itself clears the hidden cap.

## 5. Cap, memory and determinism boundary

The candidate retains exactly four PEP streams × 600M operations. Slot 3 is a
replacement, not a fifth stream, extra serial pass or wider gate. Shared
adjacency construction, per-stream game state, four-worker limit and asymptotic
memory are identical to the promoted parent.

In the local P29 A/B, global worst `order()` was 1.279 s for default PEP600 and
1.292 s for the mix. Both maxima were `crudeoil_lee4_10`, outside the PEP gate,
so that 0.013 s movement is shared-host noise. Within the changed envelope, the
slowest row was 1.241 s in the baseline and 1.239 s in the mix. The policy
replacement did not consume measured tail headroom.

All seeds and params are constants. Each worker is a pure function of pattern,
incumbent, starting FLOPS, seed, params and deterministic op budget. Results are
joined in source order and merged by strict FLOPS comparison. Completion order,
clock, environment and scheduler do not affect the returned permutation.

P27's cap pass transfers the safety of the *600M envelope*, while the exact
same-work comparison transfers the marginal slot cost. It cannot prove every
hidden trajectory has identical wall time, so the official result remains the
final authority for the mixed policy.

## 6. How the next rise can proceed

Submit only this one-slot diff from the promoted parent. Do not re-include the
budget line in a patch or restore the old `5e96293` tree. If the hidden result
promotes, the next experiment should keep 4×600M and test one slot marginal at
a time. If it rejects, retire this exact slot-3 mix on the PEP600 carrier; the
public margin is small enough for hidden reversal.

Potential follow-ups, after a terminal official result:

1. replace a different default slot with the same varied policy;
2. compare uniform-only or log-tail-only params as separate experiments;
3. preserve the 60k PEP gate and legacy RGS branch;
4. avoid four-varied 600M, which the P300 evidence already dominated;
5. rebase every test onto the then-current promoted source before action.

## 7. Orchestrator decision rule

The durable selection rule is: when one axis promotes while a composition was
measured in parallel, reconstruct the composition from the new promoted source
and keep only the remaining axis. Verify executable object identity, preserve
the exact baseline comparison, add official cap evidence, and refuse to assume
that old deltas add.

This checkout satisfies that rule: exact promoted parent, one production file,
one same-cost slot replacement, byte-identical tested executable objects,
full-corpus positive evidence, official PEP600 cap pass, and no submission or
cancellation performed locally.
