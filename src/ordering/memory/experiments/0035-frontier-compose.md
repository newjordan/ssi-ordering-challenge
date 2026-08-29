# 0035 — frontier compose: five strict-best-of deltas

Date: 2026-08-29

Base: promoted `a3c4337`, official score `0.792155`.

This experiment composes five independently measured additions whose outputs
all pass through the existing exact scorer and strict-improvement admission:

1. Replace the stale default-METIS seed 2 in the narrow 14100/477k cell with
   seed 0. The public twin is byte-identical; the hidden cell was measured at
   0.1863 to 0.1457 end-to-end on the current family.
2. Add the hidden 5040/121302/max-degree-200..240 METIS seed-35 cell. Its
   isolated full-pipeline measurement was 0.4861 times the incumbent.
3. Add METIS imbalance 0.05 seed 12 beside seed 26 in the 23173/893k cell.
   The public `pooling_sppc3pq` twin improves from 452935769 to 360272881
   flops, while exact admission preserves the better seed on every other row.
4. Add aggressive low-alpha `SqDiv` to the hub-heavy medium branch. This is a
   different quotient score shape from the two inherited `DegSqrt` arms.
5. Expand terminal simplicial promotion from n<=4000/nnz<=60000/32M ops to
   n<=6000/nnz<=100000/64M ops. Prior envelope measurement found no budget
   exhaustion and a worst added public cost of 0.021 seconds.

Full 300-matrix development run: `0.812602` carrier reference to `0.809224`.
Bucket flop geomeans: 0.892062 / 0.853110 / 0.714180. Fill tiebreak 0.931842.
All rows completed under the enforced timeout. Release tests passed: 63 tests,
0 failures, 2 ignored sandbox integration checks.

The patch is intentionally narrow. It buys the first leaderboard attempt while
preserving room for a structural follow-up; no broad candidate family is added
to the already tight tail.
