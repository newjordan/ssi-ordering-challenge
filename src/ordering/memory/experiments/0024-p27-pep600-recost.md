# 0024 — P27 PEP-selector 600M re-cost on promoted `5e96293`

- Date: 2026-08-29 KST
- Exact parent: `5e962931e1a850d22e6b90041e5e37e7e14044a7`
- Parent official score: `0.822002` (lower is better)
- Candidate change: `RGREEDY_PEP_BUDGET: 300_000_000 -> 600_000_000`
- Status: positive full-corpus candidate prepared for orchestration; not submitted

## Decision

Increase only the deterministic operation budget of the promoted PEP rgreedy
selector. The PEP branch keeps its exact promoted reach (`n <= 3000`,
`0 < nnz <= 60000`), its four default seeds, its lack of a density guard, its
parallel merge order, and its trusted strict-lower acceptance. The legacy RGS
branch remains unchanged at `60000 < nnz <= 250000`, density at most `100*n`,
and 200M operations per stream. No candidate family, worker, seed, phase,
watchdog rule, scorer, dependency, or corpus input changes.

## 1. Structure that was limiting progress

The promoted source split whole-graph rgreedy into two deliberately different
policies. Sparse patterns at or below 60k off-diagonal entries take the PEP
selector, which runs four fixed default seeds without the older average-degree
guard. Denser patterns keep the established RGS stream schedule and its 200M
budget. This split matters: the new official source already incorporated the
cap-safe reach decision and seed diversity that earlier experiments tried to
obtain by changing the global rgreedy constants.

The remaining question was depth inside the already admitted PEP region.
At 300M, every stream is bounded by a deterministic word-operation counter.
The search must finish a complete elimination trajectory before it can offer a
permutation, and the global ordering code admits the result only after a
bijection check and exact `ScoreWorkspace::flops` re-score. More deterministic
work can therefore explore a later basin without changing the safety contract
of the returned ordering. It does increase runtime, so the official 2.0-second
per-matrix watchdog remains the binding risk.

This is a re-cost, not a claim that longer search is a mathematical superset.
Budget-dependent randomized-prefix trajectories can finish in a different
basin, and a later terminal state need not contain every terminal state exposed
by a shorter budget. The trusted scorer prevents a candidate from worsening
its own incumbent in one execution, but it does not make two different-budget
executions pointwise ordered. Selection therefore requires a full paired
corpus measurement, not an assumption that 600M must beat 300M.

## 2. How the obstruction is opened

The production delta is one constant in `order_with_options`:

```text
RGREEDY_PEP_BUDGET: 300,000,000 -> 600,000,000 word operations per stream
```

The call remains `rgreedy::search_par_default_seeds`. Its four searches are
pure functions of the pattern, incumbent, operation budget, and seed index.
They run on the existing four-worker critical path and merge in source order,
with strict objective comparison giving deterministic tie behavior. No clock,
machine load, processor count, matrix name, corpus position, or environment
variable enters the search or gate.

The change is intentionally confined to the PEP branch. It does not raise the
legacy `RGREEDY_BUDGET`, widen either nnz gate, add the density guard to PEP,
remove the guard from RGS, or alter `RGREEDY_MAX_N`. It also leaves the promoted
3M low-nnz relabel floor, AMD-seeded TELOS substitution, expanded two-round
RGSUB, terminal adjacent-pair cycle, later TELOS multi-start, and every strict
fallback intact. Thus the measured delta answers one question: whether twice
the operation depth helps the newly promoted four-default-seed selector.

## 3. Full 300-pattern paired measurement

Both runs started from the exact `5e96293` tree. Generated manifests and the
vendored dependency tree were prepared by `scripts/prepare-build.sh`; the
native/FFI scan passed. Each release probe was built and run under bubblewrap
with network, IPC, UTS, and PID sharing disabled, a read-only source tree, an
empty temporary home and Cargo home, and a distinct persistent target. The
ordinary ignored `probe_timing_and_score` test executed all 300 patterns once
and printed the production `order()` score and per-matrix elapsed time.

```text
                         baseline 300M   candidate 600M   delta
weighted score             0.820071          0.819884   -0.000187
lt_1k geomean                 0.8923            0.8922
1k_10k geomean                0.8628            0.8623
gt_10k geomean                0.7338            0.7338
probe total order time      142.6630 s        175.8662 s  +33.2032 s
PEP-gated row time           80.8284 s        114.7198 s  +33.8914 s
```

The bucket printout is rounded to four decimals; the weighted six-decimal
score is the authoritative aggregate emitted by the shared probe. The gain is
concentrated below 10k, exactly where the `n <= 3000` PEP gate can act, while
the large bucket remains unchanged at printed precision. The 600M run is
therefore positive, but the extra work is real rather than runtime-neutral.

## 4. Cap evidence and failure boundary

The baseline whole-`order()` maximum was 1.3204s on
`acopf_case9241pegase_qcqp`. The candidate maximum was 1.3304s on
`crudeoil_lee4_10`, leaving about 0.6696s below the local 2.0-second threshold.
Those two matrices are outside the PEP gate, so the more relevant gated tail is
also recorded: the slowest PEP-eligible row moved from 1.0758s to 1.1612s on
`chimera_rfr-02`, an observed increase of 0.0854s and local headroom of 0.8388s.

These are isolated local observations, not a proof about the official runner.
The full candidate probe took about 33.2s more across the corpus, almost all of
it inside the 207 rows satisfying the static PEP size gate. A hidden sparse
matrix can have a more expensive fill trajectory than any public pattern, and
the grader may execute bitset work at a different speed. The candidate is
therefore suitable for official cap validation only after orchestration checks
that the live source is still `5e96293` and no stronger frontier has replaced
it. A cap failure would invalidate the candidate regardless of its public gain.

P24 is the essential warning and is not treated as transferable success.
P24 changed the old global rgreedy reach from 250k/200M to 60k/600M and also
added a second terminal adjacent-pair cycle on parent `37131d5`. Its official
benchmark produced no score because a hidden matrix exceeded the 2.0-second
cap. That failure did not name the matrix and cannot isolate RG600 from the
extra pair cycle or their interaction. P27 differs structurally: it starts from
the later promoted source, raises only the new PEP branch from 300M to 600M,
retains the legacy 200M branch, and adds no pair cycle. P24 nevertheless proves
that local timing optimism around 600M is insufficient; official execution is
the only cap authority.

## 5. Where future structure may change

The correct control point is the PEP/RGS split immediately before rgreedy in
`order_with_options`. Future work should preserve the two branches rather than
reverting to one global budget. If 600M transfers and clears the cap, the next
question is seed quality at equal cost, not a further budget increase: earlier
screens showed that deeper deterministic ILS can become worse, and the present
33-second aggregate increase already consumes meaningful envelope. If it
fails the cap, the defensible fallback is the promoted 300M selector, or a
narrower structural sub-gate justified by the failing row class if disclosed.

Any later restack must audit four boundaries separately:

1. PEP reach and its deliberate absence of the density guard;
2. legacy RGS reach, density guard, seed family, and 200M budget;
3. downstream RGSUB, TELOS, and adjacent-pair phase order;
4. the then-current relabel floor and other promoted cap costs.

Changing two of those axes in one submission would destroy the causal value of
this A/B and could recreate P24's attribution problem. In particular, a second
adjacent cycle or a wider PEP gate must not be composed into this candidate
without a fresh full-corpus and isolated tail measurement.

## Verification and handoff

The exact implementation diff against `5e96293` is the one constant above plus
this research note. `git diff --check` passes. The ordinary network-denied
release suite passes 38 tests with zero failures and 54 intentionally ignored
research probes. A separate network-denied, offline, locked release build of
`ssi-candidate-worker` also passes. Generated manifests, vendor contents,
target artifacts, probe logs, scorer outputs, and corpus results are ignored
and do not enter the commit.

Do not submit or cancel from this worktree. Before any official action, fetch
the live promoted source and compare it byte-for-byte with the recorded parent;
if it moved, restack only the one constant and re-audit all four boundaries.
