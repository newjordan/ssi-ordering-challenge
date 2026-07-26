# Log

Chronological record, one line per session. **Append-only, newest at the
bottom.** Never rewrite past entries — if a claim turns out wrong, add a new
line correcting it.

Format:

```
YYYY-MM-DD | score before→after | what you tried | outcome (+ link to experiment page)
```

Scores are geomean flop ratio vs AMD (lower is better; AMD = 1.00). They are
only comparable within the same corpus round — the corpus rebaselines per
round, so note the round if you know it.

---

<!-- newest entries below this line -->
2026-06-30 | ≫1.00 → 0.9992 | ported quotient-graph AMD (cs_amd style) into src/ordering/, replacing the identity stub | win — matches feral's AMD baseline as expected; no headroom by doing AMD-vs-AMD, next gain is nested dissection (see [0001](experiments/0001-amd-quotient-graph.md))
2026-07-26 | 0.888132→0.883906 | measured the actual time budget (new test-only `probe.rs`), then spent the slack: added 5 METIS *shape* variants (max_imbalance, nd_to_amd_switch, one seed) at n<30k/nnz<60k and KaHIP seed-2 + Eco at n<12k/nnz<45k | win — gates chosen from per-variant measurement so the global worst case is unmoved. Two findings matter more than the score: worst `order()` is **1.019 s of the 2 s cap** (the module header claimed 0.313 s), and a blanket 12-variant sweep improved only 7 of 260 matrices while costing up to 2.4 s on one — partitioner-parameter tuning is near its ceiling ([0002](experiments/0002-measured-gates-metis-kahip.md))
