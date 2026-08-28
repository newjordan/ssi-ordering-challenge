# Open questions

The research queue: leads worth chasing, gaps in the knowledge base, and
hypotheses not yet tested. Add a line whenever you notice one; resolve it by
linking to the page (experiment, technique, or literature note) that answers
it, rather than deleting it — a resolved question is a useful signpost.

## Active

- [x] **Structured relabelings, not random ones.** RESOLVED by
      [0006](experiments/0006-hillclimb-phase.md): at EQUAL budget every
      hill-climb schedule loses to i.i.d. random (0.8775-0.8787 vs 0.876511).
      But appended AFTER the random phase (extra budget, bucket-weighted,
      nnz<=60k gate) it wins: 0.875359. Original text follows.
      (was:) **Structured relabelings, not random ones (top lead).** The shipped
      multi-start uses uniform-random `Q` (Fisher-Yates), which is the dumbest
      possible choice — it works only because AMD's tie-breaking is numbering-
      sensitive. A *structured* `Q` should do better at identical cost: seed the
      relabeling from RCM, from a partitioner's block order, or from a previous
      restart's winning permutation (a hill-climb rather than i.i.d. sampling).
      The budget machinery in [0003](experiments/0003-relabelled-amd-multistart.md)
      already prices candidates, so this is cheap to test — reuse
      `probe_relabel_budget` and swap `relabel`.
- [x] **Does the budget want to be non-uniform across buckets?** ANSWERED for
      the hill-climb phase in [0006](experiments/0006-hillclimb-phase.md):
      yes — full 300k/24 only at n>=10_000, 100k/8 below, costing 0.000184 of
      score vs uniform but excluding `arki0016` (the order()-time worst) from
      the expensive phase. Untested for the RANDOM multi-start budget itself.
      Original text follows.
      (was:) **Does the budget want to be non-uniform across buckets?** The shipped
      `RELABEL_BUDGET` spends the same ~0.3 s everywhere, but `gt_10k` carries
      weight 0.40 over only 45 matrices (~4.4× the per-matrix leverage of
      `lt_1k`). A bucket-weighted budget — more restarts where a win is worth
      more — was never tested. Note `n` is known inside `order()`, so this stays
      a pure function of `(n, nnz)`.
- [x] **Price ideas by RUNNER-UP MARGIN, not tie count.** MEASURED
      (`probe_tie_margins`, via the test-only `BEST_NON_ANCHOR` hook in
      `consider`), with a caveat that reshapes the question: 92 of 94 dev ties
      show margin 1.0000 because the relabelled-AMD restarts (AMD-family) all
      but reproduce the anchor on tied matrices — the instrument cannot
      separate "AMD-family lands on the same value" from "genuinely
      flippable". To be informative it needs PER-FAMILY margins (best
      partitioner-family candidate, best AMF, etc. separately). The two
      readable rows: transswitch2736spr 1.0135 (did flip at relabel pass 4 in
      0007), gabriel10 **1.0275** — much closer than the 2-10× "structurally
      AMD-optimal" guess, so a couple more heavy passes there may pay after
      all. Original text follows.
      (was:) **Price ideas by RUNNER-UP MARGIN, not tie count.** [0004](experiments/0004-heavy-tier-gates.md)
      over-estimated its own value ~5x by assuming heavy ties would fall as deeply
      as mid-size ones did in [0003](experiments/0003-relabelled-amd-multistart.md);
      round-3 wins landed at 0.83–0.92, heavy-tier wins at 0.89–0.98. A tie is not
      a uniform unit of opportunity. For each remaining tie, record the best
      *non-AMD* candidate's ratio: margins of 1.01–1.1 are flippable, 2–10x are
      structurally AMD-optimal and should be abandoned. This is the missing input
      to every EV estimate. Cheap — one probe over the existing `consider` set.
- [x] **`lt_1k` looks structurally done.** CONFIRMED by
      [0010](experiments/0010-parallel-research-session.md), against a hypothesis
      that it was wrong. A deep sweep from 24 to **2000** restarts leaves lt_1k
      **flat** (0.90702 → 0.90642), and 3000 exact objective evaluations per
      matrix moved the bucket only −0.0006 while breaking **1 of 58** ties.
      `lt_1k` is METHOD-limited, not budget-limited: more search there is
      answering a question nobody is asking. Note the near-AMD mass is still real
      (165 of 243 graded small matrices sit at ratio ≥ 0.95) — it just needs a
      different METHOD, not more depth. Original text follows.
      (was:) **`lt_1k` looks structurally done.** 58 of 147 still tie *while already
      receiving the maximum 24 relabel restarts* (vs `1k_10k` falling 40→24 on the
      same change). More search there is answering a question that is not being
      asked. At 0.00185 of score per matrix per unit ln(ratio) it needs ~15 deep
      flips to be worth anything — do not build randomized MinFill or annealing on
      spec.
- [ ] **The big tied matrices are gated out of everything.** RESOLVED IN PART by
      [0004](experiments/0004-heavy-tier-gates.md) — `acopf_case9241pegase_qcqp`
      now gets a relabel restart (1.0000→0.9737) and `gabriel10` a no-dense AMD
      pass (1.0000→0.9756). Still open for the rest: `faclay75`
      (n=272878), `acopf_case9241pegase_qcqp` (n=313068), `gabriel10` (n=244056),
      `unitcommit_200_100_1_mod_8` (n=146830) all tie at 1.000 and receive only
      AMD plus at most one AMF pass, because the candidate gates are capped on
      `n`. But cost tracks nnz, not n, so some may have unused budget —
      `acopf_case9241pegase_qcqp` gets literally nothing but the baseline. These
      are the highest-leverage matrices on the corpus (gt_10k weight 0.40 over
      only 45 matrices). `probe_large` is written to measure exactly this.
- [ ] **How fast is the grader, really?** Partly answered and partly reopened by
      [0003](experiments/0003-relabelled-amd-multistart.md): the header's "3-5×
      slower than local" claim is false (a 1.019 s local revision passed), and
      repeat local runs vary ~1.6×, so we are tuning against a number we know to
      one significant figure. Nothing in the harness output exposes grader
      timing. Until it does, the only defensible rule is comparative — stay at or
      below the worst case of a revision known to have passed.
- [x] Do any ML/RL-guided ordering ideas fit a stdlib-only, deterministic,
      2 s/matrix `order()`? **NO** — literature sweep in
      [0010](experiments/0010-parallel-research-session.md) found no published
      result beating AMD anywhere near n=340k; the one paper tested at comparable
      scale doesn't compare to AMD and loses to METIS. Closed.
- [ ] **Does candidate diversity saturate?** [0010](experiments/0010-parallel-research-session.md)
      establishes that a bad-but-DIFFERENT ordering beats a good-but-similar one
      (four metrics, all worse than AMD standalone, bought −0.0088 together). A
      25-variant curve was still climbing — **but it was measured against an
      AMD-only baseline, not the real portfolio, and at 0.8801 it is worse than
      the portfolio already achieves.** So diversity past FOUR variants is
      unmeasured against what we actually ship. Re-run the curve with the real
      `order()` in the loop. If it keeps climbing, candidate generation is the
      whole game and the budget should move there.
- [ ] **What method (not depth) moves the near-AMD mass?** 309 of 500 graded
      matrices sit at ratio ≥ 0.95, concentrated in `lt_1k`. Search depth is
      proven not to touch it. The STRIP transform is the first thing found that
      behaves like a genuine method (new coverage of a class the portfolio was
      blind to) — are there others of that shape? Look for portfolio *blind
      spots*, i.e. structures no current candidate family addresses at all,
      rather than better versions of families we already have.
- [ ] The hand-rolled `nd_order` / `ndfm_order` use a plain **degree sort** at
      their leaves (`ND_LEAF=200`, `NDFM_LEAF=100`) and for unsplittable
      separators. The textbook hybrid hands leaves to minimum degree instead.
      Cheap to try (AMD on the induced subgraph) — but note their gate is nearly
      a subset of the METIS gate, so the upside may be small.

## Resolved

- [x] *"Where is the real headroom — is it nested dissection on the larger
      families?"* Partly answered by
      [0002](experiments/0002-measured-gates-metis-kahip.md): a 12-variant
      partitioner sweep (METIS/Scotch/KaHIP seeds, imbalance, ND→AMD switch,
      dense-quotient) improved only **7 of 260** matrices. Partitioner-parameter
      tuning is near its ceiling; the headroom is not there.
- [x] *"What density threshold should gate an expensive path? Measure, don't
      guess."* Measured — cost tracks **nnz**, not n (`qapw`, n=705/nnz=87k,
      costs 0.539 s; matrices 300× larger cost less). Per-variant costs are
      tabulated in [0002](experiments/0002-measured-gates-metis-kahip.md); use
      `probe_family` to extend the table rather than guessing a new gate.
- [x] *"Port the demo ND+AMD hybrid's exact-MD inner loop to a quotient-graph
      MD."* Obsolete as written: the portfolio now calls library METIS/Scotch/
      KaHIP, all of which already do multilevel ND with an AMD base case, and
      none breach the cap under their gates.
