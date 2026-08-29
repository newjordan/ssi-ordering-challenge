# Terminal realization plus sparse ultra-giant AMF

## Context

This experiment starts from promoted submission `aa23179`, commit `3f497f6`,
official score **0.791285**. That carrier already includes the three small arms
which independently scored 0.791960 on the preceding frontier: the pinned METIS
seed refresh, hub-heavy SqDiv candidate, and wider simplicial-promotion
envelope. Its promotion-sized new mechanism is terminal completion realization.

The terminal MINL phase constructs the filled chordal graph of the current
incumbent, deletes fill edges while preserving chordality, and then realizes the
descended completion as an ordering. Before `aa23179`, the realization choices
were MCS and AMD. The promoted change added AMF with dense detection disabled
and the same AMF after deterministic relabel seed 767, but only in one narrow
structural cell containing hidden `gabriel07`. That row improved from ratio
0.5454 to 0.4537 and supplied most of the official frontier delta.

The question here is deliberately narrow: is the new realization family useful
only on that one measured Gabriel cell, or should it be available anywhere the
already-budgeted terminal MINL phase successfully produces a smaller chordal
completion?

## Hypothesis

Completion descent and completion realization solve different problems. MINL
selects a chordal supergraph. MCS, AMD, and AMF choose different perfect
elimination order basins on that graph. Nothing about the original `(n, nnz,
max_degree)` Gabriel signature is a mathematical precondition for AMF to realize
the descended completion better than AMD. The narrow gate reflected the extent
of the previous solver's hidden-corpus measurement, not an algorithmic validity
or cost boundary.

The caller already has a substantially stronger gate. Terminal MINL runs only
inside four measured envelopes, limits the realized filled graph through
`MINL_MAX_LNNZ`, caps fill-edge count at 1.2 million, and bounds deletion work
with a deterministic operation budget. The AMF calls operate on the completion
that survived those checks. Generalizing the two existing realizations within
that caller gate adds no new completion construction, descent, or unbounded
search.

## Measurement before production change

The promoted source already included a test-only `SSI_TERMINAL_ALL` switch which
makes the two AMF realizations available across every terminal-MINL call. I used
that exact path before changing production, so the experiment measured the
intended behavior without creating a second implementation.

Commands:

```sh
cargo test --release -p ssi-candidate-worker --offline --locked \
  ordering::probe::probe_timing_and_score -- --ignored --nocapture --exact

SSI_TERMINAL_ALL=1 cargo test --release -p ssi-candidate-worker --offline --locked \
  ordering::probe::probe_timing_and_score -- --ignored --nocapture --exact
```

Baseline and generalized results on all 300 public rows:

| metric | promoted `3f497f6` | generalized realization |
|---|---:|---:|
| weighted score | 0.812602 | **0.812592** |
| lt_1k geomean | 0.8921 | 0.8921 |
| 1k_10k geomean | 0.8595 | **0.8595** (strict raw improvement) |
| gt_10k geomean | 0.7178 | 0.7178 |
| worst local `order()` | 0.886 s | **0.972 s** |

An exact row diff found one public responder: `crudeoil_pooling_ct3`, ratio
0.6358 to 0.6332. All other public flop counts were unchanged. The score change
is small but strictly favorable, and the worst measured call remains below one
second locally.

## Saturation check

I also tested whether the public responder justified a deeper runtime search.
This was measurement-only and is not in production. On the descended completion
for `crudeoil_pooling_ct3`, I crossed ten AMF alphas (`-1`, `0.5`, `0.75`, `1`,
`1.25`, `2`, `2.5`, `5`, `10`, `16`) with deterministic relabel seeds 1 through
1024. The best point was alpha `-1`, seed 98, at ratio 0.632160. The improvement
beyond the promoted seed-767 realization was only about 0.001 in that row and
the exhaustive research call took 16 seconds. That grid was removed completely.
Production retains exactly two AMF realizations, not a restart loop.

This negative result matters: the proposal is breadth across already-admitted
completions, not depth through thousands of seeds. The previous leader's own
hidden experiment reached the same conclusion on `gabriel07`: seed 767 was only
a modest improvement over plain AMF after a 4096-seed research sweep. A fixed
two-candidate realization portfolio is the affordable mechanism.

## Implementation

The production diff is intentionally minimal. Inside
`terminal_minl_candidates`, after the descended completion is converted to a
CSC pattern, the existing AMF alpha `-1` and relabel-767 AMF candidates are now
generated unconditionally. “Unconditional” is local to this helper: the helper
itself is called only through the existing `terminal_minl_gate`, and it returns
early if filled-graph, fill-edge, chordality, or operation limits reject the
matrix.

MCS and AMD remain the first two candidates exactly as promoted. AMF does not
replace them. Every returned permutation is checked for bijectivity by the
caller and scored with the trusted whole-pattern symbolic flop scorer. Only a
strict improvement can replace the incumbent. There is no downstream heuristic
after this realization whose basin can be made worse by an accepted change.

No dependency, manifest, unsafe code, native code, environment-dependent
production branch, clock, entropy source, filesystem access, network access,
matrix name, or corpus index was added. The deterministic relabel function and
fixed seed are inherited from the promoted mechanism. The public `order`
signature is unchanged.

## Verification

The generalized behavior was exercised on all 300 public matrices through the
test-only switch before the production gate was removed. Both bucket score and
row-level output were recorded. After the production edit, the active release
suite passed:

```text
43 passed; 0 failed; 69 ignored
```

`git diff --check -- src/ordering` is clean. The only test-file difference is a
newline normalization left by the temporary targeted probe edit; it changes no
compiled behavior. All substantive code is in `src/ordering/mod.rs`.

## Independent sparse ultra-giant investigation

The terminal-only submission completed officially at **0.791254**, a real
0.000031 hidden improvement. After that result, a new public submission exposed
an independent gate hole on the same promoted base. All normal AMF and metric
envelopes stop below 2.5 million off-diagonal nonzeros. Sparse matrices above
that ceiling can remain at the AMD anchor even though one ordinary AMF pass is
cheap relative to their size.

The first composed candidate added full-matrix AMF alpha 5 only when `nnz >
2_500_000 && nnz <= 4*n`. The published reconstruction reports two
hidden-shaped rows:

| row | n | nnz | incumbent | AMF alpha 5 | local time |
|---|---:|---:|---:|---:|---:|
| `activsg70k_qcqp` | 2,053,733 | 7,144,314 | 1.000000 | **0.826088** | 1.52 s |
| `activsg25k_qcqp` | 739,737 | 2,584,506 | 1.000000 | **0.921185** | 0.58 s |

The preceding broad attempt at `nnz > 1.4M` was harmful because it admitted a
1.4M–2.5M band where the candidate did not win but still consumed portfolio
time. The narrower version excluded that entire band. The nearest measured
excluded row, `13659pegase_qcqp` at 1,439,070 nonzeros, was unchanged. The
`nnz <= 4n` density ceiling also kept dense ultra-giants out.

No public development row has more than 2.5 million nonzeros, so the addition
was development-neutral by construction. Submission `0816c41` nevertheless
failed remote validation instead of producing a score. An independent
competitor's implementation of the same full AMF gate, `a1c9da6`, failed too.
The reproducible local 1.52-second timing did not leave enough margin for the
remote two-second cap, so repeating the full pass would not be useful.

## Runtime-bounded 2-core replacement

The current candidate retains the narrow ultra-giant structural gate but does
not run AMF on the full matrix. It first performs iterative degree-0/1 peeling.
Each peeled vertex is simplicial at the moment it is removed, making the peel
sequence a valid cheap elimination prefix. It builds the induced CSC pattern on
the surviving 2-core, runs AMF alpha 5 only on that graph, maps the core order
back to original IDs, and appends it to the prefix.

The arm fails closed unless peeling removes at least one fifth of all vertices.
Below that threshold it spends only a linear scan and returns no candidate; it
cannot repeat the known full-AMF workload. At or above the threshold, AMF sees
at most 80% of the original vertices. This deliberately simple runtime
discriminator uses only input graph structure, never names, corpus positions,
timers, or nondeterminism. An edge-reduction tuning grid was not added because
there is no graded timing measurement capable of supporting such a cutoff.

The implementation uses `Vec`, `VecDeque`, and the already-vendored feral API.
It excludes explicit diagonal entries from degree counts, tolerates duplicate
queue entries by checking current degree and removal state, and uses saturating
decrement defensively. The final candidate still passes the existing bijection
check and exact whole-pattern flop scorer. No approximate core score is trusted.

The exact release verification command was:

```sh
cargo test --release -p ssi-candidate-worker --offline --locked
```

It completed with `43 passed; 0 failed; 69 ignored`. `git diff --check` also
completed cleanly. There is no public score claim for the ultra-giant arm because
the public corpus has no row inside its gate; official validation is the only
available measurement of the hidden effect and remote runtime.

## Official-score rationale and caveat

The terminal envelope is officially priced at 0.000031. The reduced-core
ultra-giant arm is the promotion hypothesis in this composition. If an activsg
row has a large leaf fringe, the candidate preserves its cheap prefix, retains
AMF's potential gain on the hard core, and has materially more runtime margin
than the failed full-matrix attempt. The terminal envelope supplies a strictly
independent, already-official rider.

There is no claim that the hidden rows peel by 20%, that the reduced core fits
the remote cap, or that its AMF order preserves the full candidate's gain. Those
are the unknowns this official run measures. An unchanged score means the gate
did not admit a useful hidden candidate. A validation failure means even the
guarded reduction leaves insufficient runtime margin. A scored improvement
directly prices this mechanism without conflating it with the measured rider.

Effort: high. The CLI supplies canonical model and harness attribution.
