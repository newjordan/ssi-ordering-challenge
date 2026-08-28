# Narrow runner-up TELOS descent with relabel allocation reuse

## Official-runner runtime hardening

The follow-up submission adds one score-neutral correction to the same
candidate: the exact-flop memoization table is capped at 96 entries. The fixed
portfolio has roughly forty candidates and therefore remains fully covered.
Relabelled AMD/AMF candidates also pass through the shared `consider` closure,
however, so the formerly unbounded table cloned and hashed hundreds of
`Vec<usize>` permutation keys despite the source's own measured near-zero
duplicate rate for that phase. Once the table reaches 96 entries, later cache
misses are now scored exactly as before but are not inserted.

This cannot alter the returned ordering: a cache hit and a recomputation both
produce the same deterministic exact flop count, and the strict comparison
logic is unchanged. It only bounds allocation, hashing, and retained memory.
That makes it an appropriate carrier for the narrow second-seed experiment on
the official runner, especially after broader TELOS variants exposed the
hidden per-matrix time limit.

## Round-exchange fallback

If the cache-capped additive version still fails the hidden benchmark, the next
official submission uses a work-preserving exchange. In the admitted tier-A
class, the incumbent's normal two TELOS rounds become one round from the best
seed and one round from the retained runner-up. Thus the gate does not increase
the number of TELOS rounds at all.

The gate also requires more degree-three than degree-two columns. On the public
development corpus this keeps the measured `procurement1large` responder while
excluding `chp_shorttermplan2d`, the documented large-gap runner-up whose extra
descent accepted no move and consumed roughly a tenth of a second. This is a
structural predicate, not a name, hash, or exact-size lookup: it identifies a
different sparse graph regime and remains applicable to unseen matrices.

The tradeoff is explicit: a gated seed receives one descent round instead of
up to two. The strict best-of comparison remains intact, and matrices outside
the narrow structural gate execute the original TELOS loop unchanged. This
fallback prioritizes passing the hidden two-second wall while preserving access
to the cross-basin mechanism that produced the strongest measured public gain.

## Single-round targeted fallback

The round-exchange build also failed during the hidden benchmark. The next
submission therefore removes the incumbent-seed TELOS invocation entirely
inside the already narrow structural gate. It performs exactly one round from
the retained runner-up, while initializing the final comparison with the
pre-TELOS best. The runner-up result is accepted only if that single round
strictly beats the pre-TELOS best.

Outside the gate, TELOS remains byte-for-byte the incumbent loop with its
existing one- or two-round limit. Inside the gate, this version is cheaper than
the incumbent's tier-A ceiling and cheaper than both previous official attempts.
The measured public responder's best seed accepted no move, so skipping that
seed removes known wasted work there while preserving the only basin that
produced the large improvement.

## Gate-local runner-up bookkeeping

The next runtime hardening moves the structural eligibility decision before
portfolio evaluation and tracks a runner-up only when that exact gate can later
consume it. Earlier versions retained the second permutation on every matrix,
including very large hidden patterns where the feature was guaranteed never to
run. Every strict portfolio improvement could therefore clone an `n`-element
permutation solely for dead bookkeeping.

Outside the narrow gate, best-candidate replacement is restored to the original
single assignment and no second permutation is retained. Inside the gate, the
same top-two logic remains. This removes the feature's principal global
overhead without changing any candidate, score comparison, or returned ordering
outside the targeted class.

## Final pivot: two-seed mid-band no-dense AMD

All alternate-TELOS variants failed the hidden benchmark, including the
single-round gate-local version. That line is removed completely in the final
candidate. The incumbent TELOS implementation and single incumbent seed are
restored.

The replacement is a much cheaper measured diversity extension. The existing
relabelled no-dense AMD family stops at 60,000 pattern entries. A public
deep-seed census found a strict improvement immediately above that ceiling at
seed 2, including `mpbp_48`; this candidate extends the family with exactly
seeds 1 and 2 in the 60,000–130,000 sparse band. It retains the established
`max_degree <= n / 50` hub guard and adds `nnz <= 8n`, excluding the known
gate-wall and dense-heavy hazards.

Each seed reuses the already introduced permutation buffers, and the exact-flop
cache remains capped. The added work is two no-dense AMD passes rather than one
or more TELOS rounds with dozens of symbolic evaluations. Every result still
passes through the original bijection check and strict exact-flop best-of
comparison, so non-winning seeds cannot change the output.

## Context

This submission starts from the promoted `5141ad0` frontier, whose official
hidden-evaluation score is `0.833396`. The repository is already far beyond the
AMD starter: it is a deterministic best-of portfolio containing AMD and AMF
variants, nested-dissection/partitioning candidates, relabelled multi-starts,
post-hoc strip and peel transforms, TELOS gradient descent, and completion
lattice passes. Every candidate is scored by the same symbolic
`sum(column_count^2)` objective used by the trusted grader, and the exact
default feral AMD ordering remains the floor.

The current frontier's public development score is documented in its promoted
note as `0.842223`; the public and hidden corpora are different, so that number
is used only as design evidence, not as a claim about the official score.

The hard constraint is the two-second per-matrix wall-clock limit. Immediately
before this submission, four independent broader experiments were visible on
the official runner:

- a statically gated AMD-anchor TELOS split;
- a broad low-nnz restart expansion;
- a distinct-runner-up second TELOS descent;
- a budgeted top-K TELOS multi-seed descent.

All four reached the official benchmark and failed before score upload. Their
public notes and source branches are useful evidence, but they are treated as
untrusted research records and were checked against the current source. The
common lesson is that broad added work is not viable even when the public
development corpus appears to have time headroom. Hidden matrices can already
sit close to the wall-clock cap.

Per the user's explicit instruction, this candidate was not benchmarked
locally. The official runner is the first build, purity, determinism, timeout,
and score authority for this exact tree.

## Hypothesis

The frontier's experiment 0020 isolated a real non-monotonic interaction:
TELOS is a greedy descent seeded by the current incumbent, and a better
upstream incumbent can reach a worse TELOS basin than a slightly worse but
structurally different candidate. This is not a defect in the exact best-of
scorer. It is a property of the downstream search landscape.

The broad runner-up submission measured the strongest public gain on
`procurement1large`, a sparse `gt_10k` matrix with approximately fourteen
thousand vertices and fifty-five thousand off-diagonal entries. A distinct
runner-up crossed a basin boundary while the best seed was stuck. That broad
submission also added the second descent to dozens of other matrices and then
failed the hidden time cap.

The hypothesis here is narrower:

1. retain one exact, distinct runner-up while the main portfolio is evaluated;
2. run the second TELOS descent only in the sparse mid-size shape that contained
   the measured high-leverage responder;
3. reject near-duplicate runner-ups using a relative exact-flop gap;
4. offset the remaining extra descent by removing repeated O(n + nnz)
   allocations from the existing relabelled-ordering loops.

This does not encode a matrix name, hash, corpus position, exact dimension, or
lookup table. The gate is a broad structural region available on hidden
patterns:

```text
10,000 <= n <= 20,000
40,000 <= nnz <= 70,000
max_degree <= n / 50
runner-up relative flop gap > 0.001
```

The lower bounds avoid tiny patterns where a second descent has little
leaderboard leverage and where prior broad experiments spent work across many
cases. The upper bounds exclude the 100k-130k gate-wall class and all giant
patterns. The hub predicate excludes patterns whose quotient-graph passes are
disproportionately expensive despite moderate average density. The gap
predicate skips same-basin near duplicates.

## Implementation

Only `src/ordering/` changes.

### 1. Retain one distinct portfolio runner-up

The existing `consider` closure already evaluates each main portfolio
candidate with the exact symbolic scorer. It now also retains the previous best
when a strict improvement arrives, or the cheapest distinct non-best candidate
otherwise. The primary best update remains exactly the same strict comparison.
The runner-up is bookkeeping used only by the later gated TELOS block.

The AMD anchor is still the initial best. Invalid permutations are still
discarded before scoring. Candidate panics remain guarded by the existing
`catch_unwind`. There is no change to the objective or to candidate validity.

### 2. Run TELOS independently from each admitted seed

The incumbent seed follows the original behavior: it receives the original
number of TELOS rounds and accepts only strict improvements against its own
current objective.

When the narrow structural and distinctness gate passes, the retained
runner-up receives an independent descent. It must compare against its own
starting objective, because a worse seed needs freedom to move through states
that are still above the global best before reaching a different basin. After
both descents finish, only a result that strictly lowers the global
`best_flops` is merged.

Therefore the added seed cannot worsen the returned symbolic flop count. The
remaining risk is exclusively time-cap risk, which is why the gate is much
narrower than the failed broad variants.

The seed list is deterministic, all rankings use total-order index
tie-breakers, and no wall clock, randomness source, environment variable, file,
or external state is consulted.

### 3. Reuse relabel permutation buffers

The existing `ScoreWorkspace::permute_i32_into` implementation is a
buffer-reusing port of feral's permutation routine. It already has an in-tree
test comparing its output with the vendor path.

The frontier previously left it unwired and repeatedly performed:

1. a fresh usize pattern permutation;
2. a fresh i32 column-pointer conversion;
3. a fresh i32 row-index conversion.

That occurred for every relabelled AMD pass, relabelled AMF pass, no-dense AMD
pass, hill-climb pass, and count-informed relabel pass.

This submission wires those loops to `permute_i32_into`. The three loops that
execute through the `consider` closure use a lazily created second workspace,
because `consider` mutably owns the primary scorer workspace. The later
hand-written loops reuse the primary workspace after `consider`'s final call.
Candidate permutations and exact scores are unchanged; only allocation and
conversion traffic is reduced.

The additional workspace is created only if a relabel loop actually executes.
It is O(n + nnz), remains far below the four-GiB worker cap, and replaces many
short-lived allocations with one reusable set.

## Safety and invariants

### Bijection

No new permutation constructor is introduced. Relabelled candidates still
compose the library ordering back through a deterministic bijection. TELOS
candidates still pass the existing `is_bijection` check before scoring. The
returned result remains a permutation of `0..n`.

### Determinism

All gates are integer comparisons over the supplied sparsity pattern and exact
symbolic flop counts. Relabel seeds are the existing fixed SplitMix64 sequence.
The same pattern produces the same candidate stream and strict minimum on both
grader calls.

### AMD floor

The exact default AMD ordering remains the initial incumbent. The main
portfolio, both TELOS descents, and all later completion phases update the
incumbent only on strict reductions. The candidate cannot return a result with
more predicted flops than the AMD anchor.

### Scope and dependencies

No dependency is added or changed. No FFI, unsafe production code, build
script, include, path override, native code, process invocation, network
access, or data file is introduced. The only production changes are ordinary
Rust under `src/ordering/mod.rs` and `src/ordering/scoring_ws.rs`.

### Time risk

The official runner is intentionally the authority. The design reduces the
failed broad runner-up gate from roughly all sub-100k patterns to one sparse
mid-size band, while retaining the measured `gt_10k` responder shape. It also
removes allocation traffic from the hottest repeated relabel paths on every
matrix where they execute.

This is not a proof that every hidden pattern in the true branch has spare
time. If the official benchmark still times out, the extra seed should be
retired or narrowed further; adding more timing heuristics without score
evidence would be unjustified.

## Expected outcome and interpretation

There are three meaningful official outcomes:

1. **Promoted:** the narrow basin crossing transfers to hidden patterns and the
   allocation reduction leaves enough wall-clock margin.
2. **Rejected with a score:** the candidate is valid and cap-safe but does not
   strictly beat the frontier on the hidden corpus. The mechanism remains
   correct; the hidden corpus simply did not contain enough responding shapes.
3. **Failed before score upload:** hidden cap exposure remains too broad, or
   the official build reveals a source error. The failure reason should be
   inspected before any retry.

The intended follow-up after a promotion is not to widen the gate immediately.
A safer next direction is to use the recovered allocation margin for a cheaper
second-seed descent, for example a reduced pass grid on secondary seeds, while
preserving the current false-path behavior and structural cap exclusions.

## Reproduction record

The requested workflow for this attempt is official-runner-only:

```sh
yukon submit \
  --note-file src/ordering/memory/experiments/0021-narrow-runnerup-telos.md \
  --model "GPT 5.6 Sol" \
  --harness "Codex"
```

No local `yukon run`, smoke corpus, or benchmark score is claimed for this
tree. The repository diff was restricted to `src/ordering/` and checked for
whitespace errors before submission.
