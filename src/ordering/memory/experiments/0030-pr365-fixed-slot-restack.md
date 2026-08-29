# PR365 carrier plus fixed-slot structural routing

## Dispatch status

This candidate is a direct composition on the current promoted source
`5f024a7d77810577a461e8a43dcf92db1c138f74`, whose official primary score is
`0.810201`.  The immediate carrier is the exact source from submission
`cc3e3679-c426-409b-9c0c-fe9325ba272b`, GitHub head
`5c11f4874df7a788f9b102c63c48ecf4724802f0`.  That carrier completed the
official benchmark at `0.809835` with fill ratio `0.925514`.  It was rejected
because the improvement was smaller than the required ten relative basis
points, not because of purity, correctness, determinism, build, or the
per-matrix time cap.

The ten-relative-bip cutoff from the current displayed frontier is roughly
`0.809391`.  The carrier therefore establishes a real remote improvement of
about `0.000366`, but needs another independent mover before it can promote.
This revision adds only fixed-count candidate substitutions that previously
survived the official runner as part of submission
`9f791d50-e913-4453-8d73-3fcd3f87886d`.  That older composition scored
`0.814339` from the `0.815285` source, a real absolute reduction of `0.000946`.
It was rejected after a concurrent promotion moved the frontier to `0.813547`;
it did not fail validation.

The old aggregate delta is evidence for the package and not a claim for the
marginal value of these five substitutions on the newer carrier.  Several
other mechanisms in that older package have since landed in promoted source,
so the present port explicitly removes those overlaps instead of adding their
historical deltas arithmetically.

## Carrier mechanisms retained unchanged

The `cc3e367` carrier already contains the current promoted dense selectors,
giant sparse-hub selector, hash tie refinements, sparse MID800 floor,
conditional third RGSUB round, equal-work round-two diversification, guarded
TELOS runner-up plan, terminal completion descent, and the medium
AMF-2-to-DegSqrt replacement.  It additionally contains the remotely measured
PR365 complements:

- bounded small adjacent-pair cleanup and exact three-pivot windows;
- later-round RGSUB seed diversification without increasing stream count;
- bounded terminal simplicial promotion;
- dense low-alpha relabel complements;
- two dual-start exact-count terminal peel schedules; and
- one D_WIDE duplicate AMF-2 slot replaced by
  `extra_deg2_div_nv_wf05` at alpha `2.5`.

Those carrier mechanisms are copied byte-for-byte from the officially scored
head.  This port does not widen any PR365 gate, operation budget, thread count,
stream count, peel schedule, or terminal-search envelope.

## Absorption audit

The older `9f791d5` source cannot be overlaid wholesale.  Its base predates four
promotions, and doing so would erase later dense strip routing and terminal
completion work.  A symbol and branch audit against `5f024a7` and the PR365
carrier found these important overlaps:

1. The old narrow MID800 change, conditional third RGSUB round, and round-two
   stream diversification are already present.
2. The promoted source already replaces the medium AMF alpha-2 slot by
   DegSqrt alpha `0.75` when
   `max_degree <= 16 || max_degree^2 <= 4*n`.  That slot is retained exactly
   and is not replaced again.
3. PR365 already recovers the duplicate D_WIDE AMF alpha-2 position for
   `extra_deg2_div_nv_wf05(2.5)`.  This port leaves that replacement untouched.
4. The current giant sparse-hub branch routes the sole heavy metric task to
   `DegDivNvWfP15(0.75)` for very large hubs and `SqPure(1.5)` for smaller
   hubs.  The new dense-giant refinement is placed only in the complementary
   non-sparse-hub branch, so it cannot erase that promoted selector.

After removing those overlaps, five old task positions remain independently
replaceable on structure-only predicates.

For the four medium replacements, the predicate is also the duplicate proof.
Feral's dense threshold is `max(16, alpha*sqrt(n))`.  If maximum structural
degree is below the smaller task's threshold, neither it nor the already-paid
larger-alpha anchor classifies any vertex as dense, so their other identical
options produce the same task.  Thus AMF `2.5` and `0.5` are covered by the
existing AMF `5` anchor under `d25` and `d05`; aggressive AMD `5` and `2` are
covered by the existing aggressive AMD `10` anchor under `d5` and `d2`.
The giant alpha route is different: it deliberately replaces one non-duplicate
basin and therefore carries hidden-regression risk.

## New fixed-count substitutions

All gates use only `n`, structural `nnz`, and maximum column degree.  Squared
degree comparisons use `u128`; density products use saturating integer
arithmetic where the existing giant path already requires it.  There is no
matrix name, corpus index, clock, entropy, filesystem input, environment
switch, or cross-call state.

### D base alpha-2.5 slot

Within the existing `nnz < 400,000` D block, when `nnz >= 200,000` and

```text
max_degree <= 16 OR 4*max_degree^2 <= 25*n
```

the AMF alpha-2.5 task is replaced one-for-one by
`extra_deg2_div_nv_wf002` with alpha `10`.  Outside the predicate the exact
carrier AMF task remains.  Candidate count and source position are unchanged.

### D base alpha-0.5 slot

In the same D block, when `nnz >= 200,000` and

```text
max_degree <= 16 OR 4*max_degree^2 <= n
```

the AMF alpha-0.5 task is replaced one-for-one by `SqDiv(10)`.  The predicate
is deliberately narrower than the other medium gates and keeps the historical
AMF basin everywhere else.

### Medium AMD alpha-5 slot

Inside the pre-existing medium envelope, when `nnz >= 200,000` and

```text
max_degree <= 16 OR max_degree^2 <= 25*n
```

the aggressive AMD alpha-5 position is reused for
`extra_deg2_div_nv_wf05(10)`.  This is a substitution, not a new heavy metric
pass.  The original task stays active outside the gate.

### Medium AMD alpha-2 slot

When `nnz >= 200,000` and

```text
max_degree <= 16 OR max_degree^2 <= 4*n
```

the aggressive AMD alpha-2 position is replaced by `SqPure(5)`.  The adjacent
AMF alpha-2 position is already the promoted DegSqrt substitution and remains
separate.  The two paid positions therefore explore distinct quotient-metric
basins rather than accidentally duplicating one another.

### Dense giant sole-metric alpha

The giant metric block is capped at one task for `nnz >= 700,000`.  The current
promoted sparse-hub selector remains first.  Only in its complementary branch,
when `nnz > 20*n`, the existing sole metric name is
`extra_deg2_div_nv_wf05`, and either `nnz >= 1,000,000` or density is at least
`50`, its dense alpha changes from `10` to `2.5`.  The extra exclusion is
load-bearing: PR365 already queues the identical `wf05(2.5)` result in the
700k-to-1M, density-20-to-50 D_WIDE slice.  Keeping alpha `10` there prevents a
duplicate candidate and preserves two distinct basins.  Outside that overlap,
the route exposes the previously measured dense basin without adding a second
giant pass or affecting the sparse-hub selector.

## Work and cap argument

Relative to the remotely completed PR365 carrier, each new branch consumes an
already existing candidate position.  The number and order of deferred
portfolio tasks, parallel width, scorer calls, RGSUB rounds, terminal phases,
and deterministic operation budgets are unchanged.  A replacement kernel can
have a different constant cost from its displaced AMD/AMF kernel, so this note
does not claim cycle identity.  The stronger available evidence is that the
same five-way routing package completed the official benchmark in `9f791d5`,
while PR365 independently completed with its terminal complements enabled.

Every produced permutation still passes the existing bijection check and the
trusted exact symbolic scorer.  A replacement can update the incumbent only
under strict lower FLOPS; failure or a non-improving result leaves the current
incumbent intact.  Parallel producers remain stored and admitted in source-task
order, so completion timing cannot select the winner.

This does not prove monotonicity relative to PR365.  Replacing the dense-giant
alpha-10 basin can remove a parent winner, and any changed incumbent can seed a
different downstream search trajectory.  The unconditional AMD floor prevents
a result worse than the benchmark baseline, but it cannot prevent a regression
relative to the PR365 portfolio.  Only the official aggregate can resolve that
interaction.

The failed broad composition `157c3d6c` was not used as positive evidence.  It
added repeated terminal completion work and other extra candidates and failed
at the Benchmark step.  This revision deliberately excludes its iterative
terminal expansion, additive METIS tasks, Truck-only AMD task, pedigree strip,
deep seed cells, and additional RGSUB structural cells.  No mechanism from a
currently validating submission is imported before it has a remote result.

## Verification policy

Per campaign policy, no local benchmark, corpus run, unit test, release build,
or timing probe was executed for this restack.  Performance, correctness,
determinism, compilation, and cap behavior are delegated directly to the
official remote validator.  The local preflight was static only:

```text
carrier src/ordering snapshot comparison before graft: exact
git diff --check: clean
changed production file relative to carrier: src/ordering/mod.rs
forbidden low-level escape-path scan: empty
edits outside src/ordering: none
submission note size: checked before dispatch
```

No low-level escape path, foreign/native object, generated build path, new
dependency, or harness change is introduced.  Only the benchmark's editable
`src/ordering/` subtree is in scope.

## Expected interpretation

The official validator is authoritative.  A promotion would show that the
remote-positive PR365 carrier and the non-overlapping fixed-slot package compose
to more than the current ten-bip threshold.  A scored rejection above the
cutoff would still quantify the remaining marginal value and guide the next
task-neutral route.  A Benchmark failure would invalidate the cap-composition
assumption and require removing or narrowing replacement kernels rather than
adding more work.

If the frontier moves before this run completes, the result should be compared
both with its submission-time source and with the new promoted score.  The next
candidate must be rebuilt from the newly promoted source; this archive should
not be blindly resubmitted on a stale base.
