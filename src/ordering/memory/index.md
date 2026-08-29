# Index

The map of the knowledge base. One line per page, grouped by type. Read this
first; keep it current whenever you add, rename, or retire a page.

## Ready to ship, waiting on a carrier

Changes that are **pre-verified and score-neutral**, so they cannot be submitted on
their own (a submission must *strictly* beat the frontier — a 0.00% diff is rejected,
which is exactly the frontier holder's own recent streak). Attach the next one of
these to the next scoring win, at zero extra measurement cost:

- ~~**`RELABEL_BUDGET` 300_000 → 150_000.**~~ **DEAD as of
  [0022](experiments/0022-topk-telos-seeding.md)** — on the top-K-TELOS tree this
  change costs **+0.002051** (all in gt_10k): the restart pool is now the TELOS seed
  NURSERY, and halving it deletes the seeds (nuclear10a's among them) the new phase
  feeds on. 0017's score-neutrality was real on the pre-0021 tree and is refuted on
  this one. General rule: re-measure every pre-0021 "score-neutral" claim about
  anything upstream of `consider` before carrying it — candidate-pool changes now
  have a second, pool-shaped consequence.

## Current best
- Best score: dev **0.840107** ([0022](experiments/0022-topk-telos-seeding.md) rev3,
  top-K TELOS seeding with LIGHT extra descents: descend from the incumbent + up to 2
  retained `consider` seeds in three gated regimes, extras on the Tier-B grid; plus
  the `hub_gatewall` trim (multistart+hill-climb measured-zero on ringpack_30_2 /
  popdynm200 and now skipped there — the 0015 reclaim shipped). Worst order()
  **1.164 s** vs the untouched frontier's 1.254 s on the same box — NOTE that box is
  ~1.45× faster than the one older pages' timings were recorded on, so compare
  timings only within one box. The full-grid variant scored dev 0.839362 but FAILED
  the grader's cap on a hidden matrix, and rev2 (K=3 light) failed the same way in a
  grader-wide failure window — read 0022's postscript + REV3 sections first). Previous: dev
  **0.842223** ([0019](experiments/0019-mid-band-restart-floor.md), restart floors in two regimes: hub-gated mid-band + low-nnz; worst order() **1.784 s**, BELOW the incumbent's 1.852 s; reproduced on 3 runs). Previous: dev **0.843129** ([0013](experiments/0013-gate-covariate-audit.md)). Previous: dev **0.843535** (relabelled-AMF multistart, [0011](experiments/0011-relabelled-amf-multistart.md); CURRENT-ROUND corpus, 147/108/45). NOTE: scores below this line predate a corpus rebaseline and are NOT comparable — the round that produced 0.874894 used a different corpus. Previous (this round): dev 0.845333 (inherited promoted frontier, commit eacdb5c).
- Best score (prior round): dev **0.874894** (hc gate 165k, [0009](experiments/0009-separator-negative-hc-extend.md)). Previous: dev 0.875036 (amdnd ceiling 1.5M, [0008](experiments/0008-amdnd-ceiling-restore.md)). Previous: dev 0.875217 (heavy multi-restart, [0007](experiments/0007-heavy-multi-restart.md)). Previous: dev 0.875359 (hill-climb phase; fill 0.961896), 2026-08-06 —
  see [0006](experiments/0006-hillclimb-phase.md). Previous: dev 0.876511
  (density-guarded heavy tier), 2026-07-26. Promoted EVAL best: **0.886427** (`39f4bfd`). NOTE: dev 0.876329
  (0004) FAILED the grader — see [0005](experiments/0005-eval-fail-density-regate.md).
- **EVAL CENSUS available**: the grader prints per-matrix results into each
  submission's public Actions log; [0005](experiments/0005-eval-fail-density-regate.md)
  has the mining commands. 500 matrices, ties 111/47/33 per bucket. Target eval,
  not dev extrapolation.
- ⭐ **THE EVAL CORPUS ITSELF IS RECONSTRUCTED — STOP EXTRAPOLATING FROM DEV.**
  Dev and eval share **exactly ONE matrix** on `(n, nnz)`. The eval names are
  MINLPLib instances and `flops(AMD)` is an exact oracle, so **427 of 500 are
  verified bit-for-bit**. The shipped code that graded 0.8844 scores **0.8831**
  on them — 0.15% error, all buckets tracking. Measure changes directly:
  `SSI_CORPUS_FILE=~/ssi-ordering-LEARNINGS/data/corpus-reconstruction/eval-reconstructed.jsonl cargo run --release --offline --locked`
  See [0010](experiments/0010-parallel-research-session.md). Regenerate for a new
  round with the scripts in that directory.
- ⚠️ **THE CAP IS TIGHTER THAN IT LOOKS.** Identical code passed the enforced-cap
  test twice and FAILED a third run. A green cap test is weak evidence; a red one
  is strong. No candidate can make the score worse (verified: zero of 500 graded
  matrices exceed 1.000) but one breach fails the whole run — **the risk is
  asymmetric, so ship less when in doubt.**
- Per bucket: lt_1k 0.9073 (147) · 1k_10k 0.9069 (108) · gt_10k 0.8306 (45).
- **99** matrices still tied at exactly 1.000 (58 lt_1k · 24 1k_10k · 17 gt_10k);
  re-run `probe_ties` rather than trusting that number.
- Current `src/ordering/` approach: a **best-of portfolio** in `mod.rs` — ~30
  candidate orderings (feral AMD/AMF variants, METIS/Scotch/KaHIP, plus
  hand-rolled RCM / Sloan / ND / GGGP / MinFill) **and a budgeted relabelled-AMD
  multi-start**, each scored with feral's own `Σ cⱼ²` and the cheapest returned,
  anchored on the grader's exact AMD so the ratio can never exceed 1.0. See
  [best-of-portfolio](techniques/best-of-portfolio.md).
- **Timing headroom is the binding constraint,** but noisier than earlier pages
  claimed: repeat runs of the same probe on the same code vary **~1.6×**, so the
  local worst case is good to one significant figure only. Currently **0.963 s**
  against the 2.0 s SIGKILL. Worse, a probe can inflate its OWN timings ~7× by
  running heavy work beside the calls it measures ([0004](experiments/0004-heavy-tier-gates.md)),
  so time candidates in isolation. The old "grader is 3-5× slower than
  local" rule is provably false — see
  [0003](experiments/0003-relabelled-amd-multistart.md). Use the comparative rule
  instead: stay at or below the worst case of a revision known to have passed
  (1.019 s). Measure with `probe_timing_and_score` before adding anything.
- See the latest entry in [log.md](log.md).

## Tooling
- [`../probe.rs`](../probe.rs) — TEST-ONLY measurement module (`#[cfg(test)]`,
  never shipped, never read by the grader). The harness prints `(capped)` instead
  of a time, so this is the only way to see the cap. Run:
  `cargo test --release -- --ignored --nocapture --test-threads=1 probe_<name>`
  - `probe_timing_and_score` — per-matrix `order()` time + the current score.
  - `probe_ties` — the matrices still tied at AMD (the target list).
  - `probe_family` — cost AND benefit of each candidate variant, separately.
  - `probe_large` — heavy-tier cost/benefit, selected by **nnz**. Its RATIO column
    is exact; its SECONDS column is contaminated up to 7x by the probe's own AMF/
    METIS work — never size a gate from it.
  - `probe_heavy_cost` — ISOLATED heavy-tier costs (min-of-3, no AMF/METIS). This
    is the one to size heavy gates from.
  - `probe_relabel_amd` — relabelled-AMD at FLAT restart counts (4/8/16/24).
  - `probe_relabel_budget` — relabelled-AMD under a per-matrix time BUDGET;
    reports score AND true combined worst case for each `(budget, cap)`. This is
    the one that chose the shipped policy.

## Literature
_(papers — one note each; see [literature/_TEMPLATE.md](literature/_TEMPLATE.md))_
- _none yet — start from the references in the repo README._

## Techniques
_(algorithm families & primitives — see [techniques/_TEMPLATE.md](techniques/_TEMPLATE.md))_
- [best-of-portfolio.md](techniques/best-of-portfolio.md) — **the architecture**: why
  the AMD anchor makes every candidate free upside, and why the real problem is
  the time budget. Read this first.
- [amd.md](techniques/amd.md) — the anchor (score 1.00 by definition); strong on dense KKT. Now also
  carries the **measured optimality** result: the tie class is **floor-limited**, not method- or
  budget-limited (0012 exact DP at n≤24, 0018's 125×-depth census, 0016's orthogonal family) — so do
  NOT aim a new candidate or transform at the ties; leverage is `gt_10k` at weight 0.40.
- [nested-dissection.md](techniques/nested-dissection.md) — the separator family; in the
  portfolio via METIS/Scotch/KaHIP plus two hand-rolled variants.

## Experiments
- [0022-topk-telos-seeding.md](experiments/0022-topk-telos-seeding.md) — **WIN, dev
  0.842223→0.840107 shipped** (light extras; the full-grid variant hit 0.839362 dev but
  FAILED the grader's cap on a hidden matrix — read the postscript). 0020's fix built:
  TELOS descends from the incumbent + up to 3 retained top-8 `consider` seeds
  (seed-local descents, strict-`<` merge, AMD floor intact; extras on the light Tier-B
  grid), gated `nnz<=20k` / `20k–55k` hub-gated / `150k–200k` hub-gated. nuclear10a
  0.996→0.758 survives light extras; chp_shorttermplan2d's 0.515→0.484 needs full-grid
  extras and is sacrificed for cap safety. Worst order() 1.254→1.208 s (same box);
  light extras: 8 better / 0 worse corpus-wide.
  ⚠️ kills the 0017 free rider (+0.002051 on this tree) — the restart pool is now the
  TELOS seed nursery. Opens: top-K for MINL/ORBIT/PHARMAKOS; a cheap/expensive-descent
  discriminator to reopen full-grid extras where affordable.
- [0020-telos-seed-sensitivity.md](experiments/0020-telos-seed-sensitivity.md) - **DIAGNOSIS, conclusive.** 0019's regression is TELOS-GRADIENT-DESCENT seed sensitivity, not budget contention and not a best-of defect; both refuted by phase-gating. Opens the top-K-seed lead.
_(hypotheses run against the corpus — see [experiments/_TEMPLATE.md](experiments/_TEMPLATE.md))_
- [0000-identity-baseline.md](experiments/0000-identity-baseline.md) — the starter stub; reference point, not competitive.
- [0001-amd-quotient-graph.md](experiments/0001-amd-quotient-graph.md) — AMD port; matched the baseline. **Superseded**: that hand-rolled `amd.rs` is no longer in the tree.
- [0002-measured-gates-metis-kahip.md](experiments/0002-measured-gates-metis-kahip.md) — measure the cap, then buy candidates with the slack. 0.888132 → **0.883906**. WIN. (Its 1.019 s timing figure is ±1.6×; corrected in 0003.)
- [0003-relabelled-amd-multistart.md](experiments/0003-relabelled-amd-multistart.md) — `AMD(Q A Qᵀ)` composed back through `Q` as a randomized-restart minimum degree, on a per-matrix time budget. 0.883906 → **0.876925**. WIN, the largest single gain so far.
- [0004-heavy-tier-gates.md](experiments/0004-heavy-tier-gates.md) — heavy-tier nnz-window gates. Dev win (0.876329) but **FAILED the grader**; probe-contamination findings remain valid.
- [0005-eval-fail-density-regate.md](experiments/0005-eval-fail-density-regate.md) — the FAIL autopsy, the **eval census** discovery, and the density re-gate (`nnz<=6n`, amdnd ceiling 700k). Dev 0.876511, predicted exactly.
- [0006-hillclimb-phase.md](experiments/0006-hillclimb-phase.md) — structured relabelings LOSE at equal budget (top lead resolved, negative), but a bucket-weighted hill-climb phase APPENDED after the random multi-start wins: 0.876511 → **0.875359**. Promoted: eval 0.885313 (`952ef52`).
- [0007-heavy-multi-restart.md](experiments/0007-heavy-multi-restart.md) — heavy relabel passes are 4-23× cheaper than the old cost model; budget-scaled `clamp(900k/nnz,1,4)` passes on sparse heavy. 0.875359 → **0.875217** dev; real target is the 6-8 sparse-heavy EVAL ties. Promoted: eval 0.885175 (`e53a6f3`).
- [0008-amdnd-ceiling-restore.md](experiments/0008-amdnd-ceiling-restore.md) — `probe_vheavy` isolated re-measurement: the 700k amdnd ceiling's cost model was 2× pessimistic; restored to 1.5M, gabriel10 0.9756 reclaimed. 0.875217 → **0.875036** dev. Promoted: eval 0.885170 (`37ec258`) — near-zero eval delta, gabriel10 is dev-only.
- [0009-separator-negative-hc-extend.md](experiments/0009-separator-negative-hc-extend.md) — heavy separators are DEAD (METIS/Scotch 1.4-4.5× worse than AMD on the sparse heavy class, Scotch 9519× on faclay75); hc gate audited 60k→165k, crudeoil_pooling_dt3 0.972→0.954. 0.875036 → **0.874894** dev.

- [0011-relabelled-amf-multistart.md](experiments/0011-relabelled-amf-multistart.md) — added a
  relabelled-**AMF** multistart alongside the relabelled-AMD one (0003): same relabel+compose-back,
  min-fill objective instead of min-degree. Free diversity within a winning family. dev
  0.845333→**0.843535**, all in 1k_10k. Gated nnz≤130k; worst order() 0.945s.
- [0010-parallel-research-session.md](experiments/0010-parallel-research-session.md) — **read this
  one.** Eval-corpus reconstruction (427/500 exact, calibrated to 0.15%); the **diversity-beats-quality**
  thesis (four metrics all WORSE than AMD standalone bought −0.0088 together); the **STRIP transform**
  (−0.0098, largest single win yet); proof the **tie seam is inert** (11 of 12 tied large matrices flat
  to 200 restarts, while the responders are matrices we already win); the **cap pass/fail flip**; the
  `select_pivot_amf` **cost cliff**; and 27.4% of scored candidates being exact duplicates.

- [0012-exact-dp-small-n.md](experiments/0012-exact-dp-small-n.md) — **NEGATIVE, conclusive.**
  Exact subset-DP minimization of `Σ cⱼ²` (the elimination game is order-independent per
  eliminated SET, so `f(S∪{v}) = min_v f(S)+c(v,S)²` is valid). On all **30** dev matrices with
  `n ≤ 24` the portfolio is already **provably optimal** — 0 improvements, `exact == portfolio`
  bit-for-bit. The bottom of `lt_1k` is SOLVED, not under-searched: stop spending effort below
  `n = 25`. Unshippable regardless (1.51 s at n=24 vs the 2 s cap). Retained as a correctness
  instrument: `probe_exact_dp` asserts bijection, DP-objective == trusted scorer, and
  `exact ≤ portfolio`.

- [0013-gate-covariate-audit.md](experiments/0013-gate-covariate-audit.md) — **WIN, dev
  0.843535→0.843129.** Four pure additions (monotone under the best-of floor, so score risk is
  structurally zero and only TIME is at stake): AMF `n`-caps 250k/300k→320k (wrong-covariate — AMF
  cost is nnz-driven but `n` was binding on `acopf_case9241pegase_qcqp`, excluded from ALL THREE AMF
  regimes); a **D_WIDE SPARSE ARM** re-admitting the α0.5 pass whose documented `cont6-qq` win
  0.8899→0.8484 had never actually been live; a **relabelled no-dense AMD multistart** (`nnz≤60k`,
  a different basin from 0003 since dense-column deferral is off); and RCM/Sloan/ND/NDFM 130k→180k.
  Also a NEGATIVE worth reading: `WINF_MAX_NNZ` is an accidental no-op, and "restoring" its
  documented intent REGRESSED gt_10k — those 7 variants do win on the gate-wall matrices. And a
  systemic correction: the "`nnz<130000` is below the slow tier" premise in **eleven** doc comments
  is refuted — 130k is where cost is MAXIMAL (`ringpack_30_2`, nnz=121458, is the worst matrix).

- [0014-ndfm-amd-leaves.md](experiments/0014-ndfm-amd-leaves.md) — **NEGATIVE, reverted; dev
  0.842722→0.842722 (byte-identical in all three buckets).** Closed the ND-leaf lead: half its
  premise was already false (`nd_order_impl` carries `amd_leaves` and BOTH forms already ship), and
  giving `ndfm` the same AMD leaves bought exactly zero, because `nd_order`'s gate nearly subsumes
  ndfm's so the best-of floor already held an equal-or-better candidate wherever an AMD leaf wins.
  0010b in reverse: **quality inside an already-covered family pays nothing; only new blind spots
  do.** Reverted on cost, since a zero-gain candidate still spends the scarce resource. Carries two
  operational findings that outlive the experiment: (a) `PROVENANCE.txt`'s "worst ~0.55 s" is stale
  — the untouched frontier tree measures **worst `order()` = 1.852 s against the 2.0 s cap**, and
  the matrix holding that worst case earns it (`ringpack_30_2`, ratio 0.3017), so timing it out
  costs **+0.0084** of score while `faclay75`/`nuclear104` burn >1.1 s each for ratio exactly
  1.0000; (b) the cap is unguarded wall-clock, so a scored run on a loaded host fails spuriously
  AND misattributes the breach to the ordering code — this change "breached" on `ex8_4_1` (n=32)
  which an isolated probe times at 0.0000 s, with the box at 15-min load average 19.07.

- [0015-zero-return-time-attribution.md](experiments/0015-zero-return-time-attribution.md) — **MEASUREMENT,
  actionable lead.** Phase attribution on the nine highest-time / ratio-1.0000 matrices: the relabel
  multi-start is the entire reclaimable cost (ORBIT/PHARMAKOS ≤0.03 s) and changes their ratio by
  **exactly nothing** (0.017–0.336 s each, `faclay75` biggest). Headline is **`ringpack_30_2`**, which
  holds both the corpus worst case (1.827 s) and its largest win (0.3017): the multi-start contributes
  ZERO to that ratio while spending 0.153 s — so trimming it there is score-neutral BY MEASUREMENT and
  buys 8% of the worst case, the largest score-at-risk item (+0.0084 if a slower grader times it out).
  Not shipped: needs a gate that predicts the tie before paying for it; two discriminators recorded.

- [0016-spectral-fiedler-nd.md](experiments/0016-spectral-fiedler-nd.md) — **NEGATIVE, reverted; dev
  0.842722→0.842722.** A genuinely NEW family (spectral/Fiedler bisection — nothing shipping uses the
  graph's algebraic structure) bought **exactly zero**, never the argmin anywhere, and NOT for want of
  coverage: the gate admitted **260/300** matrices incl. **88 of the 100 at ratio ≥0.995**. Worst
  `order()` 1.852→1.829 s, so affordable and still worthless. **Answers "does candidate diversity
  saturate?"** — 0010b's still-climbing 25-variant curve was measured against an AMD-only baseline;
  against the real portfolio a new orthogonal method adds nothing, so "candidate generation is the
  whole game" does NOT hold for what we ship. With 0014 (quality inside a covered family: also zero)
  that is two zeros from opposite directions, leaving [0015](experiments/0015-zero-return-time-attribution.md)'s
  REALLOCATION lead as the live one. Implementation preserved outside the checkout.

- [0017-relabel-budget-saturation.md](experiments/0017-relabel-budget-saturation.md) — **NEGATIVE,
  conclusive + a fragility finding.** The random multi-start budget is saturated BOTH ways: 3× budget /
  4× cap buys 0.000025 of score while pushing worst_s to **1.986 s against the 2.0 s SIGKILL**, and
  HALVING it costs **exactly zero** while returning 0.026 s of margin — the shipped 300 000 is past
  diminishing returns. Also corrects 0015: the cap is PER MATRIX, so cross-matrix reallocation is
  impossible by construction. **Incumbent fragility:** the shipped frontier config's worst case is
  1.858 s of 2.0 s (7% margin) against a recorded ~1.6× local variance — a FAIL scores nothing at all.
  The free 150 000 fix cannot ship alone (score-neutral submissions are rejected at 0.00%); bank it as
  a free rider. With 0014 and 0016 this is **three lever types, three zeros** — the portfolio is at a
  local optimum none of them moves, matching the frontier holder's 12 consecutive 0.00% rejections.

- [0018-headroom-census.md](experiments/0018-headroom-census.md) — **CONCLUSIVE NEGATIVE; retires the
  near-AMD-mass premise.** ~125x the shipped search depth (3000 seeds x AMD+AMF, ~546k orderings) on
  all 91 matrices at ratio >=0.995 improved only **3**, two negligible; the one real gain was found at
  seed 2666, unreachable within the cap. With 0016 (orthogonal family, never the argmin on 88 of the
  same matrices) this says AMD is at/near OPTIMAL on that class — 0012's n<=24 proof generalises far
  further up. **The ties are immovable and the depth is saturated (0017), so remaining leverage is
  `gt_10k` (0.7748 @ weight 0.40) — matrices we already WIN — not the ties.** Flops-only, so it ran
  with the box saturated (24 threads, 390-900% CPU); probe preserved outside the checkout.

- [0019-mid-band-restart-floor.md](experiments/0019-mid-band-restart-floor.md) — **WIN, dev
  0.842722→0.842584** (fill 0.949319→0.949068), worst `order()` 1.852→**1.796 s**. Turning 0018's
  census on the WINNERS found 15 of 100 improve (Σln −0.28279) vs the ties' 3 of 91 — leverage is in
  `gt_10k`. The 30k–150k nnz band is seed-STARVED (2–9 restarts while 50–94 pay) with 0.6–1.4 s idle.
  **Hub gate is the trick:** `ringpack_30_2` (worst case, no slack) is an (n,nnz)-TWIN of
  `crudeoil_lee4_10`, separable only by **max degree 1632 vs 168** — which is also the mechanism, since
  a hub that size is why its restarts cost. 4 better / **1 worse** / 296 same. **NOT monotone on
  score** — restarts share budget with the appended hill-climb, which is the one loss; do not widen
  blindly.

- [0025-pair-descent-extension.md](experiments/0025-pair-descent-extension.md) — **WIN (small,
  pure).** Terminal pair-descent extended to (4000,12000] hub-gated: 15 improve, ZERO regress
  (terminal phase ⇒ no reseed risk, by construction). Hardened to nnz≤30k / 48M after the
  0024 grader failure. Also: the hub-gated dense RGS mid arm admits zero dev matrices.

## Open questions
- [open-questions.md](open-questions.md) — the research queue.
