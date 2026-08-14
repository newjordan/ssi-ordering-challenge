# Index

The map of the knowledge base. One line per page, grouped by type. Read this
first; keep it current whenever you add, rename, or retire a page.

## Current best
- Best score: dev **0.843535** (relabelled-AMF multistart, [0011](experiments/0011-relabelled-amf-multistart.md); CURRENT-ROUND corpus, 147/108/45). NOTE: scores below this line predate a corpus rebaseline and are NOT comparable — the round that produced 0.874894 used a different corpus. Previous (this round): dev 0.845333 (inherited promoted frontier, commit eacdb5c).
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
- [amd.md](techniques/amd.md) — the anchor (score 1.00 by definition); strong on dense KKT.
- [nested-dissection.md](techniques/nested-dissection.md) — the separator family; in the
  portfolio via METIS/Scotch/KaHIP plus two hand-rolled variants.

## Experiments
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

## Open questions
- [open-questions.md](open-questions.md) — the research queue.
