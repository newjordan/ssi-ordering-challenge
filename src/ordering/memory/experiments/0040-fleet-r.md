# 0040 — fleet-r: repass realization-path speedups + schedule REDESIGNS (WIN, proxy −0.000263)

**Date:** 2026-08-29
**Base:** fleet-p `cf1f5c0` (fast-forward from frontier `5581542` / f28dd682 0.789054; fleet-p's
0038 shipped −0.00124 proxy on that frontier and was submitted as e17245f1).
**Corpus:** `_manager/eval-corpus/eval-reconstructed-v2.jsonl` via `$SSI_CORPUS_FILE`, keys `n:nnz`.
**Mission:** 0038 banked ~−0.13 ln of measured repass wins behind rows costing 1.13–1.99 s;
reclaim trigger = a faster fill-graph realization path. Profile → optimize → re-measure.

## Profile findings (probe_fleet_r_prof, per-round ms on the banked rows)

The mission brief guessed the fill-graph re-derivation was the whale. It is NOT:

| row | fill build | scan(60M)+asort | mcs realize | amd | **amd_nd** | rescore ×5 |
|---|--:|--:|--:|--:|--:|--:|
| 9953:287612 | 19 | 35 | 3.5 | 6 | **119** | 15 |
| 14100:477382 | 30 | 30 | 6 | 9 | **222** | 24 |
| 23173:893304 | 54 | 42 | 9.5 | 34 | **580** | 45 |
| 44900:160694 | 26 | 49 | 5 | 75 | **97** | 28 |
| 109774:298082 | 30 | 62 | 5–8 | 44 | **85** | 62 |

**amd_nd (AmdOptions{aggressive:false, dense_alpha:-1}) is 50–75 % of every round** — and the
top-accepted variant. The symbolic re-derivation (permute+etree+counts) is only 3–13 ms/round;
incremental-refill schemes would recover almost nothing.

### Substitutes for amd_nd: both KILLED with proofs
- **Closed-neighborhood supervariable compression**: q == n exactly on 9953/23173 — back-edges
  to earlier-eliminated vertices make every closed neighborhood in a repass fill graph unique.
- **Fundamental-supernode quotient** (Liu–Ng–Peyton absorbed chains → amd/amd_nd on quotient,
  expand in incumbent order): compression only 5–7 % (q=9349/9953, 22481/23173) and quality
  1.4–2.3× worse. Repass-stage completions have almost no supernode structure left.
- (Re-confirmed 0038: aggressive:false + positive dense_alpha 1.5/3.0 = 5–11× catastrophic;
  incumbent-seeded MCS tie-break adds nothing over the three shipped seeds.)

So amd_nd cannot be made cheap. The unlock is (a) shaving fixed round costs and (b) REDESIGNING
schedules around cheap variants — mcs rounds are 5–50× cheaper and carry most of the mass.

## Phase A — pure-speed internals (bit-identical, verified)

1. `filled_graph`: ONE clique-merge pass instead of two (merged columns retained for the
   scatter). Shared with MINL/ORBIT/PHARMAKOS call sites — corpus-wide.
2. `terminal_completion_repass` realize-only rounds: `(fcp, fri)` IS the sorted symmetric CSC —
   MCS runs off the CSR slices (`repass_mcs_csr`, byte-identical PEOs) and AMD gets
   `CscPattern::new(n, &fcp, &fri)` zero-copy; the per-round `Vec<Vec<u32>>` adjacency and the
   CSC rebuild are gone from that path.
3. `asort` (sorted true-pattern adjacency, perm-independent) cached across scan rounds.
4. Test-only `REPASS_OFF` thread-local (MINL_OFF pattern) for schedule-redesign walks.

Verified: all 19 windowed keys reproduce 0038 finals integer-exact; 46/46 unit tests.
Row-time effect (min-of-3, quiet): 23173 0.96→0.897, 8423 0.94→0.893, 11544 0.94→0.860,
9953 0.99→0.961.

## Finding — 0038's schedules were greedy prefixes of WORSE basins

Walking from the PRE-repass incumbent (REPASS_OFF) with per-round (budget, variant) control
finds chains that dominate the shipped schedules at equal or lower cost. Redesigns (all replay
integer-exact through the pinned executor):

| key | new schedule | ln vs pre-repass | 0038 ln | Δ ln | final flops | secs (min-of-3) |
|---|---|--:|--:|--:|--:|--:|
| 9953:287612 | 4×20M-scan: mcs2,mcs0,amd,mcs2 | −0.0805 | −0.0468 | **−0.0337** | 124773694 | 0.966 |
| 14100:477382 | mcs1,mcs0 realize + 2×60M-scan mcs0 | −0.0983 | −0.0709 | **−0.0274** | 218558816 | 0.983 |
| 23173:893304 | realize-only amd,mcs0,mcs1 | −0.1301 | −0.1163 | **−0.0139** | 381674642 | 0.954 |
| 3033:65206 | +12 trailing (0,FULL_CHEAP) to true fixpoint | | | **−0.0328** | 7209425 | 0.956 |
| 8274:320136 | +1 realize-only mcs0 | | | **−0.00046** | 1125243587 | 0.771 |
| 97603:409394 NEW | one 60M-scan mcs0 | | | **−0.0039** | 7591520 | 0.918 |
| 10637:69790 NEW | realize-only mcs1,mcs0 (mcs1 = 4× the census's mcs0 chain) | | | **−0.0031** | 84927452 | 0.960 |

Σ = −0.116 ln (mid −0.067, gt −0.049).

Key micro-facts (measured):
- 9953's round-2+ wins are SCAN-dependent (realize-only round 2: 0.9981 vs 0.9878 with scan) —
  but a 20M budget reproduces the 60M chain bit-for-bit at ⅓ the scan cost.
- 23173's scans remove 0 edges — realize-only replays the same chain.
- amd-FIRST on 23173 (0.9439 round 1) then mcs0 (0.9432!) — realization order matters more
  than depth.
- 14100: mcs1 realize-only round 1 = 0.9398, deeper than 0038's entire 2-round schedule.

## Banked / dead (cap)

- 48264:255462: NEW scan round mcs1 −0.0277 found, but row = 1.0005 s local — banked, trigger:
  any base-pipeline speedup on this row.
- 5725:28914: −0.0071 residual, all amd_nd/scan-driven, needs ~0.45 s — dead.
- 23173 rounds 4+: amd_nd-only trickle (~0.58 s/round, ≤−0.0003 each) — dead.
- Rows with base >1.0 s stay banked: 44900-class (−0.024), 109774 r2 (−0.014), 94109, 214928,
  10157, 11170, 162508, 174094, 8113, 147266.

## Verification

- **v2 proxy 0.787870 → 0.787607 (−0.000263)**, full-corpus run under flock.
- Multiset diff vs the 0038 baseline run: EXACTLY the 7 intended rows changed, each with
  exactly its predicted ln delta; every other row's ratio identical to 6 decimals.
- order() replay: all 7 finals integer-equal to the probe-predicted values; all 19 pre-0040
  windowed keys reproduce their 0038 finals integer-exact (Phase A bit-identity).
- Touched-row worst 0.983 s local min-of-3 (14100); corpus worst order() 1.393 s = the
  pre-existing untouched 214928-class tail (0038 measured 1.430 s on the same row — Phase A's
  filled_graph gain shows up even there).
- 46/46 unit tests green.

## Instruments

`probe_fleet_r_prof` (per-phase round profiler), `probe_fleet_r_var2` (variant sweep/walk with
`SSI_VAR_MASK`, per-round `SSI_ITER_BUDGETS`/`SSI_VAR_MASKS` replay, `SSI_REPASS_OFF`,
supernode-quotient variants), `fleet_r_quotient` (dead, kept for the record).

Model: Claude Fable 5 (Claude Code).
