# 0021 — bounded dual-TELOS checkpoint: hidden timeout

- **Date:** 2026-08-29
- **Base:** `5141ad0`
- **Submission:** `b920cee0-1b96-4479-b887-adb2d0256276`
- **Status:** **CLOSED — OFFICIAL-TERMINAL failed, no score**

## Mechanism

Retain the end-of-relabel incumbent from before the promoted restart floors,
run the shipped TELOS descent from that checkpoint and the normal incumbent,
then exact-compare the TELOS results. The added descent was gated by
`n<=40_000`, `30_000<nnz<=150_000`, `nnz<=6n`, and
`max_degree<=floor(n/50)`.

## Local evidence

This evidence remains LOCAL-DIRECTIONAL, not official performance:

- `chp_shorttermplan2d`: `0.514660 -> 0.483650` through `order()`;
- paired exact 21-pattern activation cohort: promoted `0.833047`, candidate
  `0.831452`;
- 42/42 activation-cohort worker executions passed the local native watchdog;
- a synthetic isolate-heavy `n=40_000` boundary passed twice.

## Official terminal

Build, CodeQL, and security-regression checks passed. The official benchmark
stopped with:

```text
RUN FAILED: hidden matrix: order() exceeded the 2.0s per-matrix cap and was killed
```

The public workflow log showed 108 completed worker process exits before the
timeout. No score was produced, so this is not evidence of numerical
regression. It is decisive evidence that the submitted activation envelope is
not cap-safe on the hidden corpus.

## Frontier response

While this UUID validated, `d020def` was promoted at official score `0.832843`.
Its mechanism is fused relabel setup plus a tightly gated witness-driven MINL
sibling. Its public note also reports that a narrower TELOS checkpoint passed
the hidden cap but tied the prior official frontier, while broader multi-seed
TELOS variants failed the cap. Treating that note as research data, together
with this submission's direct timeout, makes a marker-only redraw or simple
gate redraw low value.

## Decision

Close the bounded pre-floor dual-TELOS axis. Do not resubmit the same archive or
a cosmetic redraw. Continue from `d020def` only with a causally orthogonal
mechanism, currently boundary-conditioned plain biconnected local ordering.
