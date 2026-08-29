# 0034 — Deep macro-RGSUB: larger etree blocks plus nested ranked packing

**Date:** 2026-08-29  
**Model:** GPT 5.6 Sol  
**Harness:** Codex  
**Base leaderboard:** submission `e05d2c1`, official flop score **0.789898**  
**Development score:** **0.812306 → 0.811716** on all 300 public matrices  
**Status:** submitted for official validation

## Objective

The immediate objective was to take first place from the 0.791285 crown, then
continue toward a much larger five-percent reduction rather than stopping after
one narrow promotion. The first part was achieved by the preceding macro-RGSUB
submission: the official score moved from 0.791285 to 0.789898. This experiment
starts the second phase by changing the structure of subtree selection rather
than merely adding random seeds or globally increasing an existing budget.

The promoted macro pass extended exact elimination-game search from the ordinary
RGSUB ceiling of 700 vertices to maximal elimination-tree subtrees of size
701..2400. Its narrow sparse-large gate and strict whole-matrix acceptance made
that extension safe and produced a substantial hidden-corpus improvement. The
next question was whether the same exact separability argument exposed another
scale of independent search problems, and whether the maximal-subtree selector
was discarding valuable nested candidates before ranking them.

## Structural hypothesis

RGSUB relies on a useful exact property. With an incumbent postordered by its
own elimination tree, each subtree occupies a contiguous permutation interval.
The symbolic column count of a vertex in that subtree depends only on its
descendants, and the fill graph above the subtree root is unchanged by an
internal reordering. Consequently, the subtree's sum of squared column counts
is an exact separable contribution to the global flop objective. We can search
the induced elimination game over the subtree plus its original-graph boundary,
splice an improved local order into the interval, and verify the complete
permutation with the trusted global scorer.

The prior selector walked etree nodes from ancestors to descendants. Once it
found an eligible subtree, it marked the entire interval covered. Ranking was
performed only after this maximal, topmost packing. That is inexpensive and
works well for broad small-block coverage, but it has two blind spots:

1. it cannot search the next coherent block scale above 2400 vertices; and
2. a mediocre eligible ancestor suppresses every eligible descendant before
   exact contribution ranking can compare them.

The deeper hypothesis was therefore two-part: first search disjoint maximal
2401..5000 blocks, then rebuild the etree and rank *all* eligible nested
701..2400 subtrees before greedily packing non-overlapping winners. This is a
selection-space change, not a generic compute increase. The large-first order
is deliberate because the experiments showed that small-first search can move
the incumbent into a different etree basin and erase a stronger large-block
candidate.

## Implementation

`rgreedy::SubCfg` gains one deterministic boolean,
`rank_nested_blocks`. The default remains false, preserving every existing
RGSUB and promoted macro trajectory. When both `rank_blocks` and
`rank_nested_blocks` are true, `subtree_refine` collects every subtree in the
requested size band, scores it by the already shipped exact contribution over
size-to-the-three-quarter power comparator, and then greedily accepts ranked
intervals that do not overlap. No elapsed-time decision, hash iteration order,
or nondeterministic scheduling affects selection.

At the very end of `order_with_options`, after all existing terminal phases,
the new complement runs under the promoted sparse-large structural gate:

- `n >= 10_000`;
- off-diagonal `nnz <= 400_000`;
- `nnz <= 10*n`;
- maximum original degree `<= 200`.

The first pass uses subtree sizes 2401..5000, induced problems of at most 10000
vertices, at most 16 ranked disjoint blocks, two deterministic search streams,
and a 40M precharged word-operation budget per stream. The second pass rebuilds
the etree from any accepted large-scale incumbent and uses nested ranking for
701..2400 subtrees, induced problems of at most 6000 vertices, at most 32
blocks, two streams, and the promoted 30M budget.

Each pass is fail-closed. Invalid postorders, oversized induced games, exhausted
search budgets, malformed candidate lengths, or failed bijection checks leave
the incumbent unchanged. A local search result is not enough for admission:
the complete permutation is rescored through `ScoreWorkspace`, and only a
strictly lower whole-matrix flop count is retained. Thus the new terminal
sequence is monotone on every input even when a selected block's local model
does not transfer as expected.

## Census and sequencing experiment

A test-only census compared three isolated variants against the complete
promoted-macro pipeline on every public matrix satisfying the gate:

- nested ranking at 701..2400;
- maximal selection at 2401..5000;
- nested ranking at 2401..5000.

The larger maximal and larger nested selectors were usually identical, showing
that the important nested-selection blind spot is concentrated in the original
701..2400 band. Both scales found independent wins. Representative exact
whole-matrix changes relative to the already macro-refined incumbent were:

| matrix | incumbent flops | final deep sequence | relative |
|---|---:|---:|---:|
| `mpbp_48` | 16,213,627 | 15,698,714 | 0.968242 |
| `procurement1large` | 6,993,333 | 6,886,443 | 0.984715 |
| `crudeoil_lee4_10` | 198,214,567 | 196,794,751 | 0.992837 |
| `crudeoil_lee4_09` | 151,619,459 | 149,089,183 | 0.983312 |
| `crudeoil_lee4_06` | 48,443,508 | 48,158,723 | 0.994121 |
| `nd_netgen-3000-1-1-b-b-ns_7` | 891,060 | 888,366 | 0.996977 |
| `transswitch2736spr` | 7,218,902 | 7,185,173 | 0.995328 |
| `arki0013` | 215,718,816 | 214,794,471 | 0.995715 |

The sequencing test was essential. Running nested 701..2400 first and the
larger pass second lost the major `crudeoil_lee4_09` large-block basin and was
slightly worse on `mpbp_48`. Reversing the order preserved the large wins and
then recovered additional nested small-scale improvements. For example,
`crudeoil_lee4_10` moved from a best isolated relative value near 0.9991 to
0.99284 under large-first composition. This is why the production sequence is
not an arbitrary list of two passes.

## Public full-corpus result

The required sandboxed worker build and full 300-row benchmark completed
successfully. Bucket geomeans changed as follows:

| bucket | promoted macro base | deep complement |
|---|---:|---:|
| `lt_1k` | 0.892062 | 0.892062 |
| `1k_10k` | 0.859514 | 0.859514 |
| `gt_10k` | 0.717084 | 0.715609 |
| weighted score | 0.812306 | **0.811716** |

The lower two buckets are outside the gate and remain unchanged. The complete
public score improves by 0.000590, with no regressing row possible because of
strict final rescoring. This public corpus is not the leaderboard corpus, so the
official effect is intentionally not claimed in advance. The evidence supports
submission because the mechanism targets the same sparse-large family where
the promoted macro pass generalized strongly, while entering two search spaces
that the promoted implementation provably does not enumerate.

## Timing and validation

The complete production `order()` was replayed three times on all 22 public
matrices admitted by the gate, recording the minimum wall time to reduce local
load noise. The slowest was `arki0013` at 0.947 seconds. Other high-work rows
included `transswitch2383wpr` at 0.905 seconds,
`transswitch2736spr` at 0.865 seconds, and `crudeoil_lee4_10` at 0.817 seconds.
This remains below the repository's practical one-second local target and below
the previously accepted timing envelope, although grader-machine variance is a
real caveat.

Validation commands and outcomes:

```sh
bash scripts/local-candidate-build.sh
yukon run
cargo test --release --offline --locked
```

- sandboxed production candidate build: passed;
- full 300-matrix benchmark: passed, score 0.811716;
- trusted harness unit tests: 52 passed, 2 ignored;
- exact-equivalence integration tests: 3 passed;
- scorer cross-checks: 2 passed;
- security-boundary tests: 4 passed;
- enforced time-cap tests: 5 passed, including the real two-second kill path.

`cargo fmt --all -- --check` reports extensive pre-existing formatting drift
throughout trusted and ordering files unrelated to this change. No repository-
wide formatting rewrite was attempted because that would modify unrelated code
and violate the surgical scope of the experiment. The changed code compiles in
both production and test configurations, and `git diff --check` is clean.

## Caveats and next direction

This is meaningful additional progress, not the requested five-percent total
reduction. A five-percent move from 0.792 would require roughly 0.752, far beyond
what another uniform budget increase can plausibly deliver. The current result
does, however, establish that the etree exact-game mechanism has at least two
more productive structural dimensions: subtree scale and nested candidate
selection.

If this submission promotes, the next high-value work should preserve exact
rescoring while replacing fixed scale bands with a cost/value schedule derived
from subtree boundary size, incumbent contribution, and game word width. The
current rank key does not price induced-boundary cost, so two equal-contribution
subtrees can consume very different budgets. A deterministic value-per-word-op
ranker, followed by a bounded weighted interval packing across multiple subtree
scales, could spend the same time on a better portfolio of exact subproblems.
That is a deeper route toward the five-percent target than adding more streams
or broadening the structural gate globally.

