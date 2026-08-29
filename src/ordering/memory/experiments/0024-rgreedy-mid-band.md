
## GRADER FAILURE and hardening (submission 37c36f1b)

The 300M-budget, nnz<=60k version FAILED the grader: "hidden matrix: order()
exceeded the 2.0s per-matrix cap", at hidden matrix ~#458 of 500. Aligning the
watchdog kill timestamps of that run against the promoted run's, call by call:

- the fan-out added **+0.41-0.43 s** per hidden in-gate matrix (4 streams x
  300M ops; the ~2.6x grader:local bitset ratio, exactly as memory records);
- the hidden corpus carries band-class members at **1.59 s** under the
  frontier — my run hit 2.003 s there and survived by ~3 ms — and at
  **~1.93 s**, which died;
- dev's band members are ALL cheap (<0.45 s local under load). **Dev-class
  timing extrapolation is invalid for the hidden corpus** — the same trap that
  killed the shoulder submission (87c6490) the same evening.

Hardening shipped in the resubmission: `RGREEDY_MID_MAX_NNZ = 30_000` (all
nine dev wins are below 30k; band cost tracks nnz, so the hot hidden members
are excluded), `RGREEDY_MID_BUDGET = 150M` (~0.2 s grader), and the
pair-descent extension region narrowed the same way (nnz<=30k, 48M ops).
Rule to carry: **assume the hidden tail of ANY class you add time to is at
~1.9 s, regardless of what dev shows.**
