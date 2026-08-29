<!-- 0014 -->
# 0014 — AMD leaves for `ndfm_order` (the untried half of the ND-leaf lead)

- **Date:** 2026-08-28
- **Score:** 0.842722 → 0.842722 (dev, 300 matrices; byte-identical in all three buckets)
- **Status:** loss / reverted

## Hypothesis

`open-questions.md` records this as untried: the hand-rolled `nd_order` and
`ndfm_order` both use a plain global-degree sort at their leaves (`ND_LEAF=200`,
`NDFM_LEAF=100`) and for unsplittable separators, whereas the textbook
nested-dissection hybrid hands leaves to minimum degree instead.

On inspection only **half** of that was still true. `nd_order_impl` already carries
an `amd_leaves: bool` and the portfolio already registers *both* forms
(`nd_order_impl(pattern, true)` is a live candidate). `ndfm_order` was the half
still doing a degree sort at both sites. So the remaining question was narrow and
cheap: does the same switch pay on the GGGP-separator variant?

## What changed

- `ndfm_order` split into `ndfm_order` + `ndfm_order_impl(pattern, amd_leaves)`,
  mirroring the existing `nd_order` / `nd_order_impl` pair exactly.
- Both leaf sites switched: the `sz <= NDFM_LEAF || budget < 0` base case and the
  degenerate "separator is the whole subset" case, each calling the existing
  `amd_fill_induced` helper (same signature, same `g2l` scratch discipline).
- `ndfm_order_impl(pattern, true)` registered as a **distinct** candidate under the
  identical gate, so the best-of floor made it structurally zero-downside on score.

## Result

**Zero.** Score 0.842722 before and after, and identical to six decimal places in
every bucket (`lt_1k` 0.899476, `1k_10k` 0.876533, `gt_10k` 0.774798). Not a small
win rounded away — the same numbers.

## Why it lost

The open question anticipated it: *"their gate is nearly a subset of the METIS
gate, so the upside may be small."* Sharper version after this run: `nd_order`
**already** ships AMD leaves, and its gate nearly subsumes ndfm's, so on every
matrix where an AMD-ordered leaf beats a degree-sorted one, the best-of floor
already held an equal-or-better candidate from `nd_order_impl(_, true)`. The new
candidate was never the argmin anywhere. This is 0010b's "diversity beats quality"
read in reverse: the change added *quality within a family already covered*, not a
new blind spot, so it bought nothing.

**Reverted** rather than kept. Under the best-of floor it was harmless to score,
but it is pure added time — one more GGGP pass plus O(nnz) induced-graph work per
gated matrix — and time is the binding constraint (see below). A zero-gain,
nonzero-cost candidate is strictly negative when the cap is the scarce resource.

## Two operational findings, more valuable than the experiment

### 1. The timing headroom in `PROVENANCE.txt` is badly stale

`probe_timing_and_score` on the unmodified frontier checkout reports
**WORST `order()` = 1.852 s against the enforced 2.0 s cap** — 8% headroom, not the
"worst ~0.55 s" the top of `PROVENANCE.txt` still claims. The worst matrix is
`ringpack_30_2` (n=17999) at 1.852 s, and it is *earning* that time: ratio 0.3017.

That makes it the single largest score-at-risk item on the corpus. If a slower
grader times it out, it falls back to AMD 1.0000, which moves the `gt_10k` geomean
0.775 → 0.796 and the weighted score by **+0.0084** — a bigger regression than most
increments on this challenge are wins. Priced against
`open-questions.md`'s standing rule ("stay at or below the worst case of a revision
known to have passed"), the current tree is already at the edge of what is
defensible, which is why 0014 was reverted on cost grounds alone.

By contrast, `faclay75` (1.260 s) and `nuclear104` (1.133 s) spend >1.1 s each for
ratio **exactly 1.0000** — pure cost, zero return. If headroom is ever needed, that
is where to reclaim it, not from `ringpack_30_2`.

### 2. The 2 s cap makes a scored run invalid on a loaded machine

A run of this change first came back `RUN FAILED: ex8_4_1: order() exceeded the
2.0s per-matrix cap`, on a matrix of **n=32, nnz=100**. That is not a real cap
breach: an isolated probe times `ndfm_order_impl(ex8_4_1, true)` at **0.0000 s**
with a valid bijection, and the baseline whole-`order()` time for that matrix is
**0.0012 s**. The machine's 15-minute load average was **19.07** at the time
(unrelated concurrent work on the same box); re-running the identical tree on an
idle machine passed and produced the score above.

So the cap is wall-clock and unguarded: **any** scored run competing with other load
can fail spuriously on an arbitrary matrix, and the failure message will confidently
misattribute it to the ordering code. Two consequences:

- Never trust a single `RUN FAILED: exceeded the cap` — reproduce the specific
  matrix in an isolated probe before believing it, or a healthy change gets reverted
  for someone else's CPU usage.
- Scored runs must be serialised against other work on the host. On this machine the
  neighbouring flock fleet already publishes `bus/machine-quiet.lock` and a heavy-slot
  semaphore for exactly this; acquiring both around `yukon run` is what made the
  measurement reproducible.
