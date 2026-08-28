//! DETERMINISTIC PER-MATRIX COST-BUDGET SCHEDULER (`arm/sched`).
//!
//! # Why this exists
//!
//! `order()` is a best-of portfolio: no candidate family can make the score
//! worse, only slower. Every arm therefore has a standing incentive to add
//! families, and the only thing that has ever cost this project a submission
//! is the grader's **2.0 s SIGKILL**. The grader is a 4-vCPU GitHub runner
//! that is ~1.65x slower than this box on AMD/AMF-class work and ~2.6x on
//! bitset work, and its runners vary +/-30% run to run: the promoted
//! submission `1c8eb4e2` measured a worst hidden position of **1.89 s**, and
//! the next round — which added only ~0.1 s local on the same tier — FAILED.
//! (Measured from the workflow logs: the per-worker spawn overhead is ~0.011 s,
//! so the gaps between consecutive `/bin/kill` lines in the Benchmark step ARE
//! `order()` wall time.)
//!
//! Per-family `(n, nnz)` gate constants — the mechanism used everywhere else
//! in `mod.rs` — cannot express the real constraint, because the constraint is
//! on the **sum** of everything that fires on one matrix, not on any single
//! family. A matrix that trips twenty gates at once is exactly the "gate wall"
//! the heavy tier keeps running into, and no single gate constant can see it.
//!
//! # The mechanism
//!
//! [`plan_budget`] is a pure function of `(n, nnz, max_deg)` — the same three
//! statistics every gate in `mod.rs` already keys on — returning a
//! [`PhaseMask`]: one bit per family plus a global pass-count scale. It
//!
//! 1. predicts each family's wall-clock cost on this matrix from a fitted
//!    linear model `secs ~ a*nnz + b*n + c` ([`COST`], measured by
//!    `probe_sched_cost` on 4 threads on this box),
//! 2. sums the predictions of the families whose own `(n, nnz, max_deg)` gate
//!    fires ([`fires`], a deliberate mirror of `mod.rs`'s gates that
//!    `probe_sched_gates` checks against the real thing),
//! 3. while the total exceeds [`budget_secs`], drops the enabled family with
//!    the WORST value-per-second, where "value" is a MEASURED prior ([`VALUE`],
//!    the mean exact `ln(flops_without / flops_with)` that family contributed
//!    over the 25 slowest dev matrices, from `probe_sched_ablate`),
//! 4. and, if dropping every zero-value family still leaves it over budget,
//!    shrinks the internally budget-scheduled families' pass counts stepwise
//!    ([`PhaseMask::scale_count`]) rather than deleting them outright.
//!
//! Because the decision reads nothing but the pattern statistics, the two
//! required `order()` runs take identical decisions, and every matrix whose
//! predicted cost is under budget keeps the FULL portfolio — the output is
//! byte-identical to the unscheduled build everywhere except on the
//! over-budget tier, where the dropped families are by construction the ones
//! with the least measured value there.
//!
//! # Why a *predicted* budget and not a wall-clock deadline
//!
//! A wall-clock check ("stop when 1.2 s have elapsed") is the obvious design
//! and is FORBIDDEN: the rules require byte-identical output across two runs,
//! and a timing-dependent cut is nondeterministic by construction. Predicting
//! the cost from the pattern is the deterministic equivalent, at the price of
//! needing a cost model — which is what [`COST`] is.
//!
//! # What the measurements say (`probe_sched_cost`, 300 matrices, 4 threads)
//!
//! * The portfolio drain's WALL time is `0.2748 * (sum of per-family CPU)`
//!   with R^2 = 0.977 over the whole corpus — i.e. an effective parallelism of
//!   [`PAR_EFF`] = 3.64 out of 4 threads. Portfolio-family costs in [`COST`]
//!   are therefore stored as CPU seconds and divided by `PAR_EFF`; the
//!   sequential post-hoc phases' costs are stored as wall seconds directly.
//! * `BASE` (the AMD anchor + the i32/scoring-pattern construction, which no
//!   plan can remove) is ~6.4e-8*nnz + 1.5e-7*n; 0.145 s on faclay75.
//! * Fit quality splits cleanly by mechanism. Families that are one pass over
//!   the pattern fit well (SLOAN R^2 0.98, ND 0.96, PEEL 0.95, AMF5 0.93);
//!   families with an INTERNAL time budget (`RELABEL`, `TELOSMS`, `HMETRIC`,
//!   `HAMFMS`, `MINL`) are near-constant by construction and fit badly in R^2
//!   terms while still being small in absolute RMSE. Summed, the model's
//!   predicted/actual ratio over the slow tier is median 0.86, p10 0.62,
//!   p90 1.13 — so [`BUDGET_SECS`] is stated in MODEL seconds and calibrated
//!   against measured wall time, not taken at face value. `probe_sched_report`
//!   prints predicted vs measured per matrix.
//!
//! # What the measurements say (`probe_sched_ablate`, 25 slowest, exact flops)
//!
//! Ablating one family at a time over the 25 matrices whose `order()` exceeds
//! 0.40 s locally gives each family's EXACT marginal value there (the flops
//! objective is deterministic, so `ln(flops_off / flops_on) >= 0` is a
//! noise-free measurement, unlike the wall-clock deltas):
//!
//! ```text
//! family     sum d(ln flops)  wall_s  value/s   where the value is
//! STRIP           0.4701       0.152   3.089    ringpack_30_2 +0.304, gams05 +0.153
//! HAMFMS          0.0667       0.067   1.001    transswitch2736spr +0.067
//! MIDX            0.0847       0.172   0.492    crudeoil_lee4_10 +0.059, mpbp_48 +0.026
//! AMFSWEEP        0.2772       0.840   0.330    edgecross24-115 +0.136, crudeoil_lee4_09 +0.074
//! TELOSMS         0.5832       2.216   0.263    nuclear10a +0.251, nuclear104 +0.168
//! HMETRIC         0.0895       0.370   0.242    cont6-qq +0.061, gabriel10 +0.029
//! RELABEL         0.0399       0.576   0.069    acopf +0.025, chp_shorttermplan2d +0.010
//! ROBUST/TELOS/BICONN/FILLG/ORBIT/MINL/RELAMDND/PEEL   <= 0.017 total
//! AMF5, MEDIUM, RCM, SLOAN, ND, NDFM, MINFILL, METIS, SCOTCH, KAHIP,
//! HAMDND, RELAMF, HC, PHARMAKOS, RGREEDY:  EXACTLY ZERO on all 25
//! ```
//!
//! That zero-value set costs 0.026-0.141 s of wall per slow matrix (2.26 s
//! over the 25) and buys nothing on any of them — it is the free saving this
//! module claims first. It also independently reproduces, from a completely
//! different measurement, the heavy arm's finding that the partitioner
//! families (METIS / Scotch / KaHIP) are 1.4x-20x worse than plain AMD on the
//! heavy tier and are never once the argmin there.
//!
//! # How to change the budget
//!
//! Change [`BUDGET_SECS`], the single constant. Larger = fewer matrices
//! trimmed = closer to the unscheduled build. Then re-run
//! `probe_sched_report` (which plan was chosen, predicted vs measured) and the
//! full harness (the score cost of the trim).

/// One bit per schedulable family, plus a global pass-count scale for the
/// families whose cost is set by an internal pass/restart budget. `mod.rs`
/// ANDs `plan.on(F_X)` into each family's existing `(n, nnz)` gate, so a set
/// bit never *widens* a gate — it can only leave it alone or close it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PhaseMask {
    bits: u32,
    /// Pass-count scale in [`SCALE_FULL`]ths. `SCALE_FULL` = untouched.
    scale: u32,
}

/// Denominator of [`PhaseMask::scale`]. A scale of `SCALE_FULL` leaves every
/// pass count exactly as the unscheduled build computes it.
pub(crate) const SCALE_FULL: u32 = 8;

impl PhaseMask {
    /// Is this family allowed to run at all on this matrix?
    #[inline]
    pub(crate) fn on(self, f: u32) -> bool {
        self.bits & f != 0
    }

    /// Scale an internally budget-scheduled family's pass/restart count.
    /// Monotone (never increases `k`) and never reduces a positive count to
    /// zero — the SCHEDULER decides whether a family runs at all, via its bit;
    /// this knob only makes a surviving family cheaper.
    #[inline]
    pub(crate) fn scale_count(self, k: usize) -> usize {
        if self.scale >= SCALE_FULL || k == 0 {
            return k;
        }
        (k * self.scale as usize / SCALE_FULL as usize).max(1)
    }

    #[inline]
    pub(crate) fn bits(self) -> u32 {
        self.bits
    }

    #[inline]
    pub(crate) fn scale(self) -> u32 {
        self.scale
    }
}

// ── FAMILY BITS ─────────────────────────────────────────────────────────────
// Portfolio families (queued, drained once by `parallel::run_candidates`).
// Their [`COST`] rows are CPU seconds and are divided by [`PAR_EFF`].
pub(crate) const F_AMF5: u32 = 1 << 0;
pub(crate) const F_MEDIUM: u32 = 1 << 1;
pub(crate) const F_AMFSWEEP: u32 = 1 << 2;
pub(crate) const F_ROBUST: u32 = 1 << 3;
pub(crate) const F_RCM: u32 = 1 << 4;
pub(crate) const F_SLOAN: u32 = 1 << 5;
pub(crate) const F_BICONN: u32 = 1 << 6;
pub(crate) const F_ND: u32 = 1 << 7;
pub(crate) const F_NDFM: u32 = 1 << 8;
pub(crate) const F_MINFILL: u32 = 1 << 9;
pub(crate) const F_METIS: u32 = 1 << 10;
pub(crate) const F_SCOTCH: u32 = 1 << 11;
pub(crate) const F_KAHIP: u32 = 1 << 12;
pub(crate) const F_STRIP: u32 = 1 << 13;
pub(crate) const F_HMETRIC: u32 = 1 << 14;
pub(crate) const F_HAMDND: u32 = 1 << 15;
pub(crate) const F_RELABEL: u32 = 1 << 16;
pub(crate) const F_RELAMF: u32 = 1 << 17;
pub(crate) const F_HAMFMS: u32 = 1 << 18;
pub(crate) const F_RELAMDND: u32 = 1 << 19;
pub(crate) const F_MIDX: u32 = 1 << 20;
// Sequential post-hoc phases. Their [`COST`] rows are WALL seconds.
pub(crate) const F_HC: u32 = 1 << 21;
pub(crate) const F_PEEL: u32 = 1 << 22;
pub(crate) const F_FILLG: u32 = 1 << 23;
pub(crate) const F_TELOS: u32 = 1 << 24;
pub(crate) const F_MINL: u32 = 1 << 25;
pub(crate) const F_ORBIT: u32 = 1 << 26;
pub(crate) const F_PHARMAKOS: u32 = 1 << 27;
pub(crate) const F_RGREEDY: u32 = 1 << 28;
pub(crate) const F_TELOSMS: u32 = 1 << 29;

pub(crate) const NUM_FAMILIES: usize = 30;
pub(crate) const F_ALL: u32 = (1u32 << NUM_FAMILIES) - 1;
/// The first [`PORTFOLIO_FAMILIES`] bits are portfolio families.
const PORTFOLIO_FAMILIES: usize = 21;

/// Families whose cost is set by an internal pass/restart budget and which
/// therefore respond to [`PhaseMask::scale_count`] instead of only to their
/// bit. Their predicted cost is scaled by `scale / SCALE_FULL`.
const SCALED: u32 =
    F_HMETRIC | F_RELABEL | F_RELAMF | F_HAMFMS | F_RELAMDND | F_MIDX | F_TELOSMS;

/// Human-readable names, indexed by bit position. Used by the probes and the
/// arm report; kept next to the bits so the two cannot drift.
pub(crate) const FAMILY_NAMES: [&str; NUM_FAMILIES] = [
    "AMF5", "MEDIUM", "AMFSWEEP", "ROBUST", "RCM", "SLOAN", "BICONN", "ND", "NDFM", "MINFILL",
    "METIS", "SCOTCH", "KAHIP", "STRIP", "HMETRIC", "HAMDND", "RELABEL", "RELAMF", "HAMFMS",
    "RELAMDND", "MIDX", "HC", "PEEL", "FILLG", "TELOS", "MINL", "ORBIT", "PHARMAKOS", "RGREEDY",
    "TELOSMS",
];

// ── COST MODEL ──────────────────────────────────────────────────────────────

/// Effective parallelism of the portfolio drain, measured: the drain's wall
/// time is `0.2748 * (sum of per-task CPU)` with R^2 = 0.977 over all 300 dev
/// matrices (`probe_sched_cost`). Deliberately not 4.0 — the queue is not
/// perfectly balanced and the giants are memory-bound.
const PAR_EFF: f64 = 3.64;

/// `secs ~ a*nnz + b*n + c` per family, indexed by BIT POSITION. Rows for the
/// first [`PORTFOLIO_FAMILIES`] entries are CPU seconds (divided by
/// [`PAR_EFF`] in [`family_cost`]); the rest are wall seconds. Fitted by
/// weighted least squares (weight `1 + nnz/40000`, so the heavy tier — the
/// only place a plan is ever trimmed — dominates) over the matrices where the
/// family's own gate fires. `N`, `R2`, `rmse` and the largest observed cost
/// are recorded per row so a later session can see which rows are trustworthy.
#[rustfmt::skip]
const COST: [[f64; 3]; NUM_FAMILIES] = [
    // AMF5       N=300 R2=0.930 rmse=0.0100 max=0.3746s
    [2.09723e-07, 1.28189e-07, 2.289566e-05],
    // MEDIUM     N=290 R2=0.685 rmse=0.0086 max=0.1505s
    [2.40108e-07, 6.14716e-07, -3.300695e-03],
    // AMFSWEEP   N=279 R2=0.744 rmse=0.0330 max=0.5139s
    [1.58567e-06, 3.96895e-06, -3.364604e-03],
    // ROBUST     N=279 R2=0.703 rmse=0.0055 max=0.0916s
    [3.68776e-07, -1.08770e-07, -1.021469e-03],
    // RCM        N=286 R2=0.891 rmse=0.0007 max=0.0195s
    [3.05599e-08, 1.84636e-07, -1.658632e-04],
    // SLOAN      N=286 R2=0.978 rmse=0.0012 max=0.0496s
    [1.64836e-07, 4.62892e-07, -3.059177e-05],
    // BICONN     N=220 R2=0.861 rmse=0.0108 max=0.2554s
    [1.19943e-07, 2.63484e-07, 2.809868e-03],
    // ND         N=286 R2=0.964 rmse=0.0012 max=0.0497s
    [4.33698e-08, 8.06369e-07, 3.692284e-05],
    // NDFM       N=286 R2=0.920 rmse=0.0018 max=0.0451s
    [1.13140e-07, 4.37820e-07, -2.652658e-04],
    // MINFILL    N=197 R2=0.775 rmse=0.0052 max=0.0522s
    [3.08773e-06, 6.55501e-07, -1.022413e-03],
    // METIS      N=290 R2=0.405 rmse=0.0217 max=0.1480s
    [5.82834e-08, 1.88061e-06, 1.744331e-02],
    // SCOTCH     N=262 R2=0.722 rmse=0.0072 max=0.0625s
    [8.71952e-08, 3.46122e-06, 3.486041e-03],
    // KAHIP      N=250 R2=0.869 rmse=0.0100 max=0.1607s
    [8.56435e-07, 7.40360e-06, 1.168254e-04],
    // STRIP      N=267 R2=0.811 rmse=0.0056 max=0.1599s
    [5.83582e-07, -1.23971e-06, -1.322156e-03],
    // HMETRIC    N= 19 R2=0.315 rmse=0.0593 max=0.2275s
    [1.19885e-07, -2.75426e-08, 7.980576e-02],
    // HAMDND     N=  5 R2=0.930 rmse=0.0084 max=0.0961s
    [-1.78464e-08, 5.73043e-07, -2.524321e-02],
    // RELABEL    N=296 R2=0.483 rmse=0.0241 max=0.2917s
    [3.83441e-08, 6.27596e-07, 2.109871e-02],
    // RELAMF     N=279 R2=0.388 rmse=0.0230 max=0.1658s
    [-3.99593e-07, 4.24931e-06, 2.512986e-02],
    // HAMFMS     N=  9 R2=-0.039 rmse=0.0422 max=0.1858s
    [-5.46227e-08, -8.95115e-07, 1.451871e-01],
    // RELAMDND   N=261 R2=0.510 rmse=0.0192 max=0.1796s
    [2.16198e-07, 4.54733e-06, 1.895465e-02],
    // MIDX       N=251 R2=0.642 rmse=0.0120 max=0.2289s
    [2.19872e-06, -5.33015e-06, -5.765681e-03],
    // HC         N=284 R2=0.701 rmse=0.0026 max=0.0357s
    [4.94567e-08, 4.00072e-07, 8.348990e-04],
    // PEEL       N=296 R2=0.951 rmse=0.0031 max=0.1365s
    [8.59608e-08, 5.73174e-07, 3.549386e-04],
    // FILLG      N=296 R2=0.861 rmse=0.0045 max=0.0980s
    [9.61200e-08, 2.78904e-07, 2.282766e-03],
    // TELOS      N=291 R2=0.502 rmse=0.0092 max=0.1008s
    [1.73318e-07, -2.32802e-08, 4.502141e-03],
    // MINL       N=296 R2=0.300 rmse=0.0211 max=0.1534s
    // Fitted JOINTLY over the whole `6.minl_orbit` phase (MINL and ORBIT are
    // queued as two sibling tasks and run concurrently, so the phase's WALL
    // time is essentially MINL's; ORBIT's marginal is inside it).
    [9.57331e-08, 2.39503e-07, 1.712936e-02],
    // ORBIT      marginal wall over MINL is ~0: it is MINL's parallel sibling,
    // so removing it does not shorten the phase. Charged 0, which also keeps
    // the drop loop from ever picking it (cost <= COST_EPS).
    [0.0, 0.0, 0.0],
    // PHARMAKOS  N=260 R2=0.867 rmse=0.0019 max=0.0257s
    [1.99506e-07, 7.52592e-07, 6.985608e-05],
    // RGREEDY    N=211 R2=0.467 rmse=0.0091 max=0.1029s
    [-9.15703e-08, -9.69805e-06, 7.031520e-02],
    // TELOSMS    N=280 R2=0.276 rmse=0.0302 max=0.1832s
    [3.96288e-07, -4.03618e-07, 3.241247e-02],
];

/// The cost no plan can remove: the AMD anchor, the i32 pattern conversion and
/// the scoring workspace. Same fit, over all 300 matrices.
const BASE: [f64; 3] = [6.40979e-08, 1.51383e-07, 2.246586e-03];

/// MEASURED value prior: the mean exact `ln(flops_without / flops_with)` this
/// family contributed per matrix over the 25 slowest dev matrices
/// (`probe_sched_ablate`; sum over the 25 divided by 25). Zero means the
/// family was measured to change NOTHING on any of them. Indexed by bit
/// position. This is the numerator of the value-per-second the drop order
/// sorts on; it is a REGIONAL prior for the over-budget tier, which is the
/// only tier `plan_budget` ever trims.
#[rustfmt::skip]
const VALUE: [f64; NUM_FAMILIES] = [
    0.000000e+00, // AMF5
    0.000000e+00, // MEDIUM
    1.108800e-02, // AMFSWEEP
    1.920000e-04, // ROBUST
    0.000000e+00, // RCM
    0.000000e+00, // SLOAN
    1.400000e-04, // BICONN
    0.000000e+00, // ND
    0.000000e+00, // NDFM
    0.000000e+00, // MINFILL
    0.000000e+00, // METIS
    0.000000e+00, // SCOTCH
    0.000000e+00, // KAHIP
    1.880400e-02, // STRIP
    3.580000e-03, // HMETRIC
    0.000000e+00, // HAMDND
    1.596000e-03, // RELABEL
    0.000000e+00, // RELAMF
    2.668000e-03, // HAMFMS
    7.200000e-05, // RELAMDND
    3.388000e-03, // MIDX
    0.000000e+00, // HC
    6.800000e-05, // PEEL
    2.280000e-04, // FILLG
    6.600000e-04, // TELOS
    1.360000e-04, // MINL
    1.760000e-04, // ORBIT
    0.000000e+00, // PHARMAKOS
    0.000000e+00, // RGREEDY
    2.332800e-02, // TELOSMS
];

/// CALIBRATION of the summed model against measured wall time. The per-family
/// fits are unbiased least squares, so their SUM sits below the measured total
/// on the slow tier (median predicted/actual 0.86 over the 85 dev matrices
/// above 0.20 s). Multiplying by `1/0.86` makes the prediction track measured
/// LOCAL 4-thread seconds in the median, which is what makes
/// [`BUDGET_SECS`] readable as a wall-clock number rather than as an
/// arbitrary model unit. It does NOT make the model exact: the p10/p90 of the
/// calibrated ratio is 0.72/1.31, so the budget still needs measurement
/// (`probe_sched_report`) behind it.
const MODEL_CALIB: f64 = 1.16;

/// The local:grader slowdown is NOT uniform across code kinds — measured
/// across three graded submissions: ~1.65x for AMD/AMF/partitioner-style work
/// and ~2.6x for the bitset phases (`rgreedy`'s elimination game and the
/// filled-graph completion descents MINL/ORBIT). Round 4's worst hidden
/// position (1.84 s) was a SMALL dense matrix where the sequential frontier
/// took 1.02 s — i.e. the cap risk there is the bitset tax, not nnz.
///
/// Costs are therefore expressed in GRADER-WEIGHTED local seconds: a bitset
/// family is charged `2.6/1.65 = 1.576x` its local cost, everything else 1x.
/// `BUDGET_SECS` is in those units, so one constant covers both classes.
const BITSET: u32 = F_RGREEDY | F_MINL | F_ORBIT;
const BITSET_MULT: f64 = 1.576;

/// Costs below this are not worth a drop decision (and dropping them cannot
/// close a meaningful budget gap).
const COST_EPS: f64 = 1e-4;

/// **THE ONE CONSTANT TO TURN.** The per-matrix budget, in GRADER-WEIGHTED
/// LOCAL 4-thread seconds (see [`MODEL_CALIB`] and [`BITSET_MULT`]): the model
/// is calibrated to track measured local wall time in the median, and bitset
/// phases are charged 1.576x because the grader punishes them that much
/// harder. Divide by ~1.65 and multiply by the runner's +/-30% to read it as a
/// grader-side number: 0.45 here is ~0.75 s of typical grader time and ~1.0 s
/// on a bad runner, against the 2.0 s SIGKILL.
///
/// Larger = fewer matrices trimmed = closer to the unscheduled build. Turn it,
/// then re-run `probe_sched_report` (which plan was chosen, predicted vs
/// measured, per-matrix flops both ways) and the full harness (the score cost
/// of the trim). The measured curve on the dev corpus is in `ARM_REPORT.md`.
/// The anchor's exact fill `Σ amd_counts` for the CURRENT `order()` matrix,
/// published by `order_with_options` before `plan_budget` is consulted so
/// `fires(F_TELOSMS)` can mirror the shipped `telos_ms_count(.., lnnz)` gate
/// exactly. Pure per matrix: both required grader runs compute it from the
/// same deterministic anchor and agree.
thread_local! {
    pub(crate) static ANCHOR_LNNZ: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(crate) fn set_anchor_lnnz(v: usize) {
    ANCHOR_LNNZ.with(|c| c.set(v));
}
pub(crate) const BUDGET_SECS: f64 = 0.45;

#[cfg(test)]
fn budget_secs() -> f64 {
    // TEST-ONLY sweep hook so `probe_sched_report` can price several budgets
    // in one build. Production reads the constant.
    std::env::var("SSI_SCHED_BUDGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(BUDGET_SECS)
}

#[cfg(not(test))]
#[inline(always)]
fn budget_secs() -> f64 {
    BUDGET_SECS
}

// ── GATE MIRROR ─────────────────────────────────────────────────────────────

/// Does family `bit_pos`'s own `(n, nnz, max_deg)` gate in `mod.rs` fire on
/// this matrix?
///
/// This is a deliberate MIRROR of the gates in `order_with_options`, not a
/// second source of truth: every threshold below is the same named constant
/// the gate itself reads (`super::AMF_MAX_N`, ...), so widening a gate cannot
/// silently desynchronise the two — only restructuring one can. The mirror
/// covers each family's OUTER gate; sub-variants inside a block are folded
/// into that family's fitted cost. `probe_sched_gates` checks this function
/// against the per-family timings actually observed on all 300 dev matrices
/// (agreement is exact wherever the family costs more than 0.1 ms; the
/// remaining disagreements are all sub-0.1 ms and cannot move a plan).
fn fires(bit_pos: usize, n: usize, nnz: usize, max_deg: usize) -> bool {
    use super::*;
    if nnz == 0 || n == 0 {
        return false;
    }
    let rr = relabel_restarts(RELABEL_BUDGET, RELABEL_MAX_RESTARTS, n, nnz, max_deg);
    match 1u32 << bit_pos {
        F_AMF5 => n < AMF_MAX_N && nnz < AMF_MAX_NNZ,
        F_MEDIUM => n < MEDIUM_MAX_N && nnz < MEDIUM_MAX_NNZ,
        F_AMFSWEEP => n < AMF_SWEEP_MAX_N && nnz < 130_000,
        F_ROBUST => n < ROBUST_MAX_N && nnz < ROBUST_MAX_NNZ,
        F_RCM => n < RCM_MAX_N && nnz < RCM_MAX_NNZ,
        F_SLOAN => n < SLOAN_MAX_N && nnz < SLOAN_MAX_NNZ,
        F_BICONN => nnz < BICONN_MAX_NNZ && nnz <= HEAVY_SPARSE_MAX_AVG_DEG * n,
        F_ND => n < ND_MAX_N && nnz < ND_MAX_NNZ,
        F_NDFM => n < NDFM_MAX_N && nnz < NDFM_MAX_NNZ,
        F_MINFILL => n < MINFILL_MAX_N && nnz < MINFILL_MAX_NNZ,
        // Any of the four METIS blocks; the widest one subsumes the rest.
        F_METIS => n < METIS_MAX_N && nnz < METIS_MAX_NNZ,
        F_SCOTCH => n < SCOTCH_MAX_N && nnz < SCOTCH_MAX_NNZ,
        F_KAHIP => {
            (n < KAHIP_MAX_N && nnz < KAHIP_MAX_NNZ)
                || (n < KAHIP_MULTI_MAX_N && nnz < KAHIP_MULTI_MAX_NNZ)
        }
        F_STRIP => n < STRIP_MAX_N && nnz < STRIP_MAX_NNZ && nnz < (STRIP_MAX_AVG_DEG + 1) * n,
        F_HMETRIC => {
            // Mirrors the heavy arm's dead-window + low-band sparsity trim.
            let dead = (nnz >= HEAVY_METRIC_DEAD_MIN_NNZ && nnz <= HEAVY_METRIC_DEAD_MAX_NNZ)
                || (nnz < HEAVY_METRIC_DEAD_MIN_NNZ && nnz > HEAVY_SPARSE_MAX_AVG_DEG * n);
            nnz >= HEAVY_METRIC_MIN_NNZ && nnz <= HEAVY_METRIC_MAX_NNZ && !dead
        }
        F_HAMDND => {
            let window = nnz >= HEAVY_AMDND_MIN_NNZ && nnz < HEAVY_AMDND_MAX_NNZ;
            window && nnz <= HEAVY_SPARSE_MAX_AVG_DEG * n
        }
        F_RELABEL => rr > 0,
        F_RELAMF => nnz <= 130_000 && rr > 0,
        F_HAMFMS => {
            if nnz <= 130_000 {
                false
            } else if nnz <= HEAVY_SPARSE_MAX_AVG_DEG * n {
                nnz <= HEAVY_RELABEL_AMF_SPARSE_MAX_NNZ
            } else {
                nnz >= HEAVY_RELABEL_AMF_DENSE_MIN_NNZ && nnz <= HEAVY_RELABEL_AMF_DENSE_MAX_NNZ
            }
        }
        F_RELAMDND => nnz <= 60_000,
        F_MIDX => {
            let main = nnz <= MIDX_MAX_AVG_DEG * n
                && max_deg <= n / MIDX_HUB_DIV
                && nnz > 60_000
                && nnz <= MIDX_AMF_MAX_NNZ;
            main || nnz <= MIDX_AMF05_MAX_NNZ
        }
        F_HC => nnz <= HC_MAX_NNZ,
        F_PEEL => nnz < PEEL_CNTREL_MAX_NNZ,
        F_FILLG => nnz < FILLG_STRIPV2_MAX_NNZ,
        F_TELOS => nnz < TELOS_MAX_NNZ,
        F_MINL => nnz < MINL_MAX_NNZ,
        // ORBIT only ever runs alongside MINL (they are queued as siblings).
        F_ORBIT => nnz < MINL_MAX_NNZ && n <= ORBIT_MAX_N,
        F_PHARMAKOS => nnz < PHARMAKOS_MAX_NNZ,
        // `RGREEDY_MAX_N` / `RGREEDY_MAX_NNZ` are FUNCTION-LOCAL consts in
        // `order_with_options`, so they cannot be named from here; these two
        // literals are the mirror. `probe_sched_gates` is what catches a
        // drift.
        F_RGREEDY => n <= 3_000 && nnz <= 250_000 && nnz <= 100 * n,
        F_TELOSMS => {
            let lnnz = ANCHOR_LNNZ.with(std::cell::Cell::get);
            telos_ms_count(usize::MAX, n, nnz, max_deg, lnnz) > 0
        }
        _ => false,
    }
}

/// Predicted seconds of WALL time this family adds to `order()` on this
/// matrix under `scale`, or 0.0 if its gate does not fire.
fn family_cost(bit_pos: usize, scale: u32, n: usize, nnz: usize, max_deg: usize) -> f64 {
    if !fires(bit_pos, n, nnz, max_deg) {
        return 0.0;
    }
    let r = COST[bit_pos];
    let mut c = (r[0] * nnz as f64 + r[1] * n as f64 + r[2]) * MODEL_CALIB;
    if bit_pos < PORTFOLIO_FAMILIES {
        c /= PAR_EFF;
    }
    if (1u32 << bit_pos) & SCALED != 0 && scale < SCALE_FULL {
        c = c * scale as f64 / SCALE_FULL as f64;
    }
    if (1u32 << bit_pos) & BITSET != 0 {
        c *= BITSET_MULT;
    }
    if c > 0.0 {
        c
    } else {
        0.0
    }
}

/// Predicted total wall time of `order()` on this matrix under this plan.
pub(crate) fn predict_secs(bits: u32, scale: u32, n: usize, nnz: usize, max_deg: usize) -> f64 {
    let mut t = ((BASE[0] * nnz as f64 + BASE[1] * n as f64 + BASE[2]) * MODEL_CALIB).max(0.0);
    for b in 0..NUM_FAMILIES {
        if bits & (1u32 << b) != 0 {
            t += family_cost(b, scale, n, nnz, max_deg);
        }
    }
    t
}

// ── THE PLAN ────────────────────────────────────────────────────────────────

/// Deterministic plan for one matrix. Pure function of the pattern statistics
/// — no wall-clock, no entropy, no iteration over a hash container — so both
/// required `order()` runs decide identically. See the module doc.
pub(crate) fn plan_budget(n: usize, nnz: usize, max_deg: usize) -> PhaseMask {
    #[cfg(test)]
    {
        if let Some(m) = OVERRIDE.with(|c| c.get()) {
            return PhaseMask {
                bits: m,
                scale: SCALE_FULL,
            };
        }
    }
    let budget = budget_secs();
    let mut bits = F_ALL;
    let scale = SCALE_FULL;
    let mut total = predict_secs(bits, scale, n, nnz, max_deg);
    if total <= budget {
        // The common case: full portfolio, output byte-identical to the
        // unscheduled build.
        return PhaseMask { bits, scale };
    }

    // PHASE 1 — drop whole families, worst value-per-second first.
    //
    // The comparison is done by CROSS MULTIPLICATION (`v_i * c_j` vs
    // `v_j * c_i`) rather than by forming the ratio, so a zero-cost family can
    // never produce a division by zero and the order is a genuine total order.
    // Ties (which is every pair of measured-zero-value families) break on
    // LARGER cost first — dropping the most expensive worthless family closes
    // the budget gap in the fewest drops — and then on bit position, which
    // makes the whole loop reproducible.
    loop {
        if total <= budget {
            break;
        }
        let mut pick: Option<(usize, f64, f64)> = None; // (bit, value, cost)
        for b in 0..NUM_FAMILIES {
            if bits & (1u32 << b) == 0 {
                continue;
            }
            let c = family_cost(b, scale, n, nnz, max_deg);
            if c <= COST_EPS {
                continue;
            }
            let v = VALUE[b];
            pick = Some(match pick {
                None => (b, v, c),
                Some((pb, pv, pc)) => {
                    // worse value-per-second wins the pick
                    let lhs = v * pc;
                    let rhs = pv * c;
                    if lhs < rhs || (lhs == rhs && c > pc) {
                        (b, v, c)
                    } else {
                        (pb, pv, pc)
                    }
                }
            });
        }
        match pick {
            Some((b, _, c)) => {
                bits &= !(1u32 << b);
                total -= c;
            }
            None => break,
        }
    }

    // PHASE 2 — if every family that could be dropped has been and the matrix
    // is STILL over budget (nothing on the dev corpus reaches here; this is
    // the valve that keeps a future arm's additions, or an unseen hidden
    // matrix, off the cap), shrink the internally budget-scheduled families'
    // pass counts instead of removing more families.
    let mut scale = scale;
    while total > budget && scale > 1 {
        scale -= 1;
        total = predict_secs(bits, scale, n, nnz, max_deg);
    }

    PhaseMask { bits, scale }
}

/// TEST-ONLY plan override, so `probe_sched_ablate` can run the SAME
/// `order()` with any family switched off and measure its marginal wall time
/// and marginal flops. Never compiled into the shipped binary.
#[cfg(test)]
thread_local! {
    pub(crate) static OVERRIDE: std::cell::Cell<Option<u32>> =
        const { std::cell::Cell::new(None) };
}
