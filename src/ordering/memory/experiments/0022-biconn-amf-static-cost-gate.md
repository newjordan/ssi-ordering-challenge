# 0022 — static-cost gate for the additive biconn AMF slot

- **Date:** 2026-08-28
- **Baseline:** official frontier source `5141ad0`, score `0.833396`
- **Predecessor:** [0021](0021-biconn-additive-amf-restack.md)
- **Status:** local positive; deliberately not submitted by this experiment

## Failure being corrected

0021 added one independent plain-AMF candidate to the largest eligible
biconnected block while preserving every promoted relabelled-AMD slot and both
restart floors.  Its official workflow failed before score upload because a
hidden `order()` call exceeded the 2.0 s per-matrix cap.  The `nnz <= 10_000`
gate was not a sufficient end-to-end cost bound: it limited the induced AMF
problem's edge count, but did not bound the full matrix's vertex count or the
cost of the already-live portfolio around the extra pass.

The local audit behind 0021 also measured the wrong population.  It selected
186 JSONL rows using the serialized `nnz` field, while production gates on
`Pattern::nnz()` after diagonal removal.  The real public gate population is
191.  Five matrices were omitted, exactly by one diagonal entry per vertex:

| matrix | n | JSON `nnz` | `Pattern::nnz()` | max degree |
|---|---:|---:|---:|---:|
| `oil` | 3270 | 11700 | 8430 | 123 |
| `slay09h` | 2718 | 10206 | 7488 | 24 |
| `rsyn0805m03m` | 2868 | 11138 | 8270 | 17 |
| `rsyn0815m02hfsg` | 3054 | 11022 | 7968 | 17 |
| `rsyn0840m02m` | 3486 | 13062 | 9576 | 17 |

All five produced the same output/flops with and without the additive AMF
slot, but their omission invalidates 0021's claim that the complete production
gate population had been audited.  Future gate subsets must be formed from the
parsed `Pattern`, not the serialized census field.

## Static discriminator

The replacement gate is:

```text
n <= 192 && Pattern::nnz() <= 1_200 && max_deg <= 64
```

These quantities are already computed by the live portfolio before the biconn
call, so the gate adds no decomposition or feature-extraction pass.  The three
bounds have separate cost meanings: `n` caps every whole-pattern O(n) buffer
and scan, `nnz` caps the induced block edges and the existing sparse passes,
and `max_deg` excludes a small but hub-dominated graph whose elimination walk
can be disproportionate to total nnz.  The AMF block has at most 192 vertices
and 1,200 full-pattern nonzeros; it cannot reproduce the old large-sparse
hidden shape.

The established behavior is otherwise unchanged.  The 24-token/seven-per-
block relabelled-AMD schedule is allocated first, both official restart-floor
regimes remain live, and the one AMF token is still strictly additive.

## Exact full-corpus A/B

A test-only switch ran the exact current portfolio with the AMF token disabled
and enabled.  Outside the new gate the boolean cannot affect the budget, so the
same measured call is byte-identical by construction.  Inside the gate both
sides were executed and rescored independently with the exact symbolic flop
objective.

| metric | exact `5141ad0` | narrowed AMF gate | delta |
|---|---:|---:|---:|
| full 300 weighted flop score | 0.842222684 | **0.841958705** | **-0.000263979** |
| gate population | 86 | 86 | 0 |
| gate outputs changed | — | 2 | — |
| strict flop wins | — | 1 | — |
| gate-population total wall time | 1.984651 s | 1.950888 s | -0.033763 s (noise) |
| gate-population worst wall time | 0.103610 s | **0.102568 s** | -0.001042 s (noise) |

The only strict flop win is `pooling_adhya4pq` (`n=170`, `nnz=932`,
`max_deg=21`): `21,640 -> 18,740`, `ln(candidate/base)=-0.143883177168`.
`pooling_adhya4tp` is the second changed permutation but ties at 23,070 flops.
Every other matrix is output-identical.

The Windows full-corpus timing run was contended: the unchanged incumbent alone
reported 3.792964 s on `acopf_case9241pegase_qcqp`, with several other
unchanged paths above 2 s.  Absolute full-corpus wall times from that run cannot
be used as cap evidence.  They do establish the relevant comparative fact:
the full-corpus worst is exactly identical on both sides, because it is outside
the gate; within the causal gate all 86 public matrices remain around 0.1 s or
below on this slower/noisier run.

## Decision

Keep the narrowed gate as the only defensible P4 retry candidate.  It preserves
the complete public score win while changing the risk model from an nnz-only
proxy to direct bounds on the whole pattern and hub size.  Do not resubmit the
old `nnz <= 10_000` form.  This experiment does not submit; an orchestrator may
consider the narrowed version only after the sandboxed build and fixed security
and time-cap tests pass.

## Reproduction and validation protocol

The final source was prepared and tested from commit `8de916d`. The temporary
A/B switch and probe output files were removed before the commit; the committed
runtime delta is limited to the three production gate predicates plus this
memory record and its index entries. To reproduce the submission boundary on a
Linux host with bubblewrap and the public corpus present:

```sh
bash scripts/prepare-build.sh
cargo build --release --offline --locked -p ssi-ordering-challenge
bash scripts/local-candidate-build.sh
cargo test --release --offline --locked -p ssi-ordering-challenge \
  security_boundary -- --nocapture
cargo test --release --offline --locked -p ssi-ordering-challenge \
  time_cap -- --nocapture
```

The preparation/vendor scan was clean, the trusted build passed, and the
candidate build ran inside bubblewrap with network denied. Candidate unit tests
passed 35/35, `security_boundary` passed 4/4, and `time_cap` passed 5/5,
including the normal sample run and the deliberate slow-order kill. An initial
LF-native clone contained Git LFS pointer text instead of the real development
corpus, causing only the normal-corpus fixture test to fail. That result was
discarded rather than attributed to the ordering. Copying the real 99 MB public
corpus into the native clone and rerunning produced the final 5/5 result.

The exact score A/B used a test-only boolean at the AMF-token allocation point:
one run forced zero new cross-family tokens and the other enabled precisely the
committed static gate. Both used the same parsed patterns, current official
portfolio, exact flop scorer, and deterministic candidate ordering. The switch,
probe instrumentation, PID files, raw outputs, and generated lockfile noise are
not in the submitted tree. The A/B therefore measures the one causal choice
without shipping a benchmark hook.

## Caveats and next step

Public Windows timing was useful only inside the causal 86-matrix gate; machine
contention made unchanged large matrices exceed the official wall and cannot be
used to predict hidden absolute time. The safety case instead comes from the
static size bounds, the approximately 0.103 s public gate maximum, the unchanged
full-corpus worst path, and the official-equivalent Linux timeout suite. The
hidden corpus remains authoritative. If official validation still hits the cap,
retire this AMF addition rather than widen or resubmit it. If it scores but does
not promote, use the per-matrix official delta to decide whether the sole public
win generalizes; do not stack a second cross-family token without a new isolated
A/B.
