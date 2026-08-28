//! Randomized greedy elimination-game search on the EXACT objective `Σ c_j²`.
//!
//! ## Why this is a different family from the relabelled-AMD multi-start
//!
//! Every search family already in this module explores permutations by feeding
//! a *relabelled* pattern to `feral_amd`/`feral_amf` and re-scoring the result.
//! AMD is an *approximate* minimum-degree code: supervariables, aggressive
//! absorption, approximate external degrees, multiple elimination. Relabelling
//! only perturbs its internal tie-breaks, so the reachable set of orderings is
//! a narrow neighbourhood of "what AMD's approximations happen to produce".
//! `memory/experiments/0018` measured 3000 relabel seeds × 2 objectives on the
//! near-AMD mass and found essentially nothing — but explicitly flagged that it
//! is a *depth* result inside those two objectives, not a proof of optimality.
//!
//! This module searches a strictly larger space: the **exact** elimination game
//! with an arbitrary randomized pivot rule. No supervariables, no approximate
//! degree, no absorption — the true fill graph is maintained as bitsets and the
//! true external degree of every live vertex is known at every step.
//!
//! ## The objective is free
//!
//! In the elimination game, the column count charged when `v` is eliminated out
//! of the already-eliminated set `S` is exactly `c(v,S) = 1 + |N_{G_S}(v)|`
//! (the same identity `exact_dp.rs` is built on, and the same quantity
//! `column_counts_gnp` recovers from the permuted pattern). So a greedy run
//! *accumulates the scored objective as it goes*, at zero extra cost: there is
//! no separate scoring pass, and a partial sum that already exceeds the
//! incumbent lets the run be abandoned mid-flight (`Σ c²` is monotone in the
//! prefix). Both properties are what make tens of thousands of exact objective
//! evaluations affordable inside the per-matrix cap at `n < 1000`.
//!
//! ## Determinism
//!
//! Fixed-seed xorshift64, a fixed policy schedule, a fixed **operation** budget
//! (never wall-clock), and lowest-index tie-breaks everywhere a random draw is
//! not explicitly taken. Two runs on the same pattern return byte-identical
//! output, as the harness requires.

#![allow(dead_code)]

/// Largest `n` this module will allocate for. Memory is `2 · n · ⌈n/64⌉ · 8`
/// bytes (two bitset adjacency copies) = ~`n²/4` bytes; at 4000 that is 4 MB,
/// far inside the 4 GiB worker cap. The SHIPPED gate at the call site is much
/// lower and is chosen for TIME, not memory.
pub(crate) const MAX_N: usize = 12_000;

#[cfg(test)]
thread_local! {
    pub(crate) static ADJACENT_PAIR_DESCENT_CALLS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[inline]
fn xs64(s: &mut u64) -> u64 {
    let mut x = *s;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *s = x;
    x
}

/// Uniform in `0..m` (m > 0), via the standard rejection-free multiply-shift.
#[inline]
fn below(s: &mut u64, m: u32) -> u32 {
    ((xs64(s) >> 32) * (m as u64) >> 32) as u32
}

#[cfg(test)]
thread_local! {
    /// Proves that the shipped `order()` path reaches the final promotion
    /// phase without exposing a production-global mutable counter.
    pub(crate) static SIMPLICIAL_PROMOTION_CALLS: std::cell::Cell<usize> = const {
        std::cell::Cell::new(0)
    };
}

/// The elimination game on a bitset fill graph.
///
/// Invariant: `adj[u]` holds exactly `u`'s neighbours in the CURRENT fill graph
/// restricted to LIVE vertices (never `u` itself, never an eliminated vertex),
/// and `deg[u] == popcount(adj[u])` for every live `u`.
pub(crate) struct Game<'a> {
    n: usize,
    w: usize,
    adj0: &'a [u64],
    adj: Vec<u64>,
    deg: Vec<u32>,
    livelist: Vec<u32>,
    pos: Vec<u32>,
    nlist: Vec<u32>,
    cand: Vec<u32>,
    tmp: Vec<u64>,
    /// Deterministic work counter, in word-operations. The ONLY budget signal —
    /// no wall-clock anywhere in this module.
    pub(crate) ops: i64,
}

impl<'a> Game<'a> {
    /// Build the pristine bitset adjacency ONCE. Shared immutably by every
    /// stream of the fan-out: it is a pure function of the pattern, it is the
    /// single largest allocation this module makes (`n·⌈n/64⌉` words), and
    /// building it costs an O(nnz) scan. Doing that per stream made four
    /// threads take ~4x the wall time of one at n≈5000 — the allocator and the
    /// kernel's page zeroing serialised, so the fan-out bought nothing.
    ///
    /// `None` if `n` is out of range or the pattern references an
    /// out-of-range row (the caller then simply skips this phase).
    pub(crate) fn build_adj(n: usize, col_ptr: &[usize], row_idx: &[usize]) -> Option<Vec<u64>> {
        if n == 0 || n > MAX_N {
            return None;
        }
        let w = n.div_ceil(64);
        let mut adj0 = vec![0u64; n * w];
        for v in 0..n {
            let (lo, hi) = (col_ptr[v], col_ptr[v + 1]);
            for &r in &row_idx[lo..hi] {
                if r >= n {
                    return None;
                }
                if r != v {
                    adj0[v * w + (r >> 6)] |= 1u64 << (r & 63);
                    adj0[r * w + (v >> 6)] |= 1u64 << (v & 63);
                }
            }
        }
        Some(adj0)
    }

    /// A working game over a shared pristine adjacency.
    pub(crate) fn new(n: usize, adj0: &'a [u64]) -> Option<Game<'a>> {
        if n == 0 || n > MAX_N {
            return None;
        }
        let w = n.div_ceil(64);
        if adj0.len() < n * w {
            return None;
        }
        Some(Game {
            n,
            w,
            adj: adj0[..n * w].to_vec(),
            adj0,
            deg: vec![0; n],
            livelist: Vec::with_capacity(n),
            pos: vec![0; n],
            nlist: Vec::with_capacity(n),
            cand: Vec::with_capacity(n),
            tmp: vec![0u64; w],
            ops: 0,
        })
    }

    fn reset(&mut self) {
        self.adj.copy_from_slice(&self.adj0[..self.n * self.w]);
        self.livelist.clear();
        for v in 0..self.n {
            let mut d = 0u32;
            for k in 0..self.w {
                d += self.adj[v * self.w + k].count_ones();
            }
            self.deg[v] = d;
            self.pos[v] = self.livelist.len() as u32;
            self.livelist.push(v as u32);
        }
        // Charged to match measured cost: the bitset copy and the per-vertex
        // popcount pass are both `n·w`, plus a fixed per-vertex bookkeeping term
        // (livelist/pos). Without the linear term the budget massively
        // undercharges tiny `n` (where `w == 1`), and a constant ops budget
        // then costs 3x more wall time at n=64 than at n=800.
        self.ops += (2 * self.n * self.w + 8 * self.n) as i64;
    }

    #[inline]
    fn drop_live(&mut self, v: usize) {
        let p = self.pos[v] as usize;
        let last = *self.livelist.last().unwrap();
        self.livelist[p] = last;
        self.pos[last as usize] = p as u32;
        self.livelist.pop();
    }

    /// Eliminate `v`, returning its column count `c = 1 + |N(v)|`.
    fn eliminate(&mut self, v: usize) -> u64 {
        let w = self.w;
        self.tmp.copy_from_slice(&self.adj[v * w..v * w + w]);
        // Materialize N(v).
        self.nlist.clear();
        for k in 0..w {
            let mut word = self.tmp[k];
            while word != 0 {
                let b = word.trailing_zeros() as usize;
                word &= word - 1;
                self.nlist.push((k * 64 + b) as u32);
            }
        }
        let c = self.nlist.len() as u64 + 1;
        let vw = v >> 6;
        let vbit = 1u64 << (v & 63);
        // Clique N(v): each u in N(v) absorbs N(v), minus itself and minus v.
        for i in 0..self.nlist.len() {
            let u = self.nlist[i] as usize;
            let base = u * w;
            let mut d = 0u32;
            for k in 0..w {
                let nv = self.adj[base + k] | self.tmp[k];
                self.adj[base + k] = nv;
                d += nv.count_ones();
            }
            // `tmp` contains u (u ∈ N(v)) and `adj[u]` contained v; both are
            // now set and both must go — hence the `-2`.
            self.adj[base + (u >> 6)] &= !(1u64 << (u & 63));
            self.adj[base + vw] &= !vbit;
            self.deg[u] = d - 2;
        }
        self.ops += ((self.nlist.len() + 1) * (3 * w + 6) + 24) as i64;
        for k in 0..w {
            self.adj[v * w + k] = 0;
        }
        self.deg[v] = 0;
        self.drop_live(v);
        c
    }

    /// Number of fill edges eliminating `v` would create (its deficiency).
    fn deficiency(&mut self, v: usize) -> u32 {
        let w = self.w;
        self.tmp.copy_from_slice(&self.adj[v * w..v * w + w]);
        let mut missing: u32 = 0;
        for k in 0..w {
            let mut word = self.tmp[k];
            while word != 0 {
                let b = word.trailing_zeros() as usize;
                word &= word - 1;
                let u = k * 64 + b;
                let base = u * w;
                let mut m = 0u32;
                for q in 0..w {
                    m += (self.tmp[q] & !self.adj[base + q]).count_ones();
                }
                // `u` itself is in `tmp` and never in `adj[u]`.
                missing += m - 1;
            }
        }
        self.ops += ((self.deg[v] as usize + 1) * (2 * w + 4)) as i64;
        missing / 2
    }

    /// Exact `Σ c_j²` of an arbitrary elimination order, computed by replaying
    /// the elimination game. Used only by probes/tests to cross-check against
    /// the trusted `column_counts_gnp` path.
    #[cfg(test)]
    pub(crate) fn replay_flops(&mut self, order: &[usize]) -> u64 {
        self.reset();
        let mut f = 0u64;
        for &v in order {
            let c = self.eliminate(v);
            f += c * c;
        }
        f
    }
}

/// Promote a provably zero-fill future pivot across a bounded prefix of the
/// incumbent ordering.
///
/// At prefix `k`, a live vertex with deficiency zero is simplicial in the
/// current fill graph: eliminating it now creates no edges.  We promote it only
/// when it is adjacent to the planned pivot and has strictly smaller live
/// degree.  Bubble the simplicial vertex left across the intervening block:
/// non-edge crossings are neutral, edge crossings are non-increasing, and the
/// final crossing over the planned pivot is strictly improving.  The relative
/// order of every other pivot is kept intact.  Ties use
/// `(degree, future position, vertex id)`.
///
/// This is deliberately a candidate builder, not an admission oracle.  The
/// caller must still validate the bijection and re-score the completed order
/// through the trusted symbolic path, accepting only a strict improvement.
/// Every bitset operation after construction is covered by `ops_budget`; if
/// the next atomic deficiency/elimination step would cross it, the WHOLE
/// candidate fails closed.
pub(crate) fn simplicial_promotion(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    seed: &[usize],
    lookahead: usize,
    promotion_cap: usize,
    ops_budget: i64,
) -> Option<Vec<usize>> {
    #[cfg(test)]
    SIMPLICIAL_PROMOTION_CALLS.with(|c| c.set(c.get() + 1));

    if n < 3
        || seed.len() != n
        || col_ptr.len() != n + 1
        || lookahead < 2
        || promotion_cap == 0
        || ops_budget <= 0
    {
        return None;
    }
    if col_ptr[0] != 0 || col_ptr[n] != row_idx.len() || col_ptr.windows(2).any(|w| w[0] > w[1]) {
        return None;
    }
    let mut seen = vec![false; n];
    for &v in seed {
        if v >= n || seen[v] {
            return None;
        }
        seen[v] = true;
    }

    let adj0 = Game::build_adj(n, col_ptr, row_idx)?;
    let mut game = Game::new(n, &adj0)?;
    game.reset();
    if game.ops > ops_budget {
        return None;
    }

    let mut order = seed.to_vec();
    let mut changed = false;
    let mut promotions = 0usize;
    for k in 0..n {
        let current = order[k];
        let current_degree = game.deg[current];
        let end = k
            .saturating_add(1)
            .saturating_add(lookahead)
            .min(n);
        let mut best: Option<(u32, usize, usize)> = None;

        // Distance one is intentionally excluded: this phase is a batch move
        // across an adjacent-swap plateau, never a duplicate adjacent-pair
        // descent step.
        for j in k.saturating_add(2)..end {
            let v = order[j];
            let degree = game.deg[v];
            if degree >= current_degree {
                continue;
            }

            // The adjacency to the current pivot is the strict certificate;
            // charge the one-word membership probe before performing it.
            if game.ops.saturating_add(1) > ops_budget {
                return None;
            }
            game.ops += 1;
            let adjacent = game.adj[v * game.w + (current >> 6)]
                & (1u64 << (current & 63))
                != 0;
            if !adjacent {
                continue;
            }

            // `deficiency`'s charged work is exact and depends only on the
            // current degree and bitset width, so reserve it before entering
            // the atomic operation instead of detecting an overshoot after it.
            let charge = (degree as i64 + 1) * (2 * game.w as i64 + 4);
            if game.ops.saturating_add(charge) > ops_budget {
                return None;
            }
            if game.deficiency(v) == 0 {
                let key = (degree, j, v);
                if best.is_none_or(|old| key < old) {
                    best = Some(key);
                }
            }
        }

        if let Some((_, j, _)) = best {
            order[k..=j].rotate_right(1);
            changed = true;
            promotions += 1;
            if promotions == promotion_cap {
                return Some(order);
            }
        }

        let pivot = order[k];
        let charge = (game.deg[pivot] as i64 + 1) * (3 * game.w as i64 + 6) + 24;
        if game.ops.saturating_add(charge) > ops_budget {
            return None;
        }
        game.eliminate(pivot);
    }

    changed.then_some(order)
}

/// A pivot policy: pick uniformly among the live vertices whose degree is
/// within `slack` of the minimum; when `fill_tb` is set, break that set by
/// smallest deficiency first (a min-fill lookahead over a min-degree
/// candidate list).
#[derive(Clone, Copy)]
struct Policy {
    slack: u32,
    fill_tb: bool,
}

impl Game<'_> {
    /// One randomized greedy run. `fixed` is a prefix of pivots replayed
    /// verbatim before randomization starts (the LNS operator); pass an empty
    /// slice for a from-scratch run. Returns `None` as soon as the partial
    /// objective reaches `bound` (pruned), since `Σ c²` only grows.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        fixed: &[usize],
        pol: Policy,
        rng: &mut u64,
        bound: u64,
        hard_cap: i64,
        out: &mut Vec<usize>,
    ) -> Option<u64> {
        self.reset();
        out.clear();
        let mut f: u64 = 0;
        for &v in fixed {
            let c = self.eliminate(v);
            f += c * c;
            out.push(v);
            if f >= bound {
                return None;
            }
        }
        while !self.livelist.is_empty() {
            // HARD STOP. `last_run` bounds the budget using the PREVIOUS run's
            // cost, which is only a prediction: a randomized pivot sequence can
            // generate far more fill than the incumbent's, and on an unseen
            // matrix a single run can cost several times what the last one did.
            // That is exactly how a locally-fine ordering phase blows a wall
            // clock cap on a hidden corpus, so the run is abandoned outright
            // once total ops pass the cap. Abandoning costs only this run's
            // work — the incumbent is untouched, and the phase as a whole is
            // still strictly-improving.
            if self.ops > hard_cap {
                return None;
            }
            let m = self.livelist.len();
            self.ops += 4 * m as i64;
            let mut dmin = u32::MAX;
            for i in 0..m {
                let d = self.deg[self.livelist[i] as usize];
                if d < dmin {
                    dmin = d;
                }
            }
            let cut = dmin.saturating_add(pol.slack);
            let pick;
            if pol.slack == 0 && !pol.fill_tb {
                // Uniform among the argmin set, via reservoir sampling (no
                // allocation, no index bias).
                let mut cnt = 0u32;
                let mut sel = 0u32;
                for i in 0..m {
                    let v = self.livelist[i];
                    if self.deg[v as usize] == dmin {
                        cnt += 1;
                        if below(rng, cnt) == 0 {
                            sel = v;
                        }
                    }
                }
                pick = sel as usize;
            } else {
                self.cand.clear();
                for i in 0..m {
                    let v = self.livelist[i];
                    if self.deg[v as usize] <= cut {
                        self.cand.push(v);
                    }
                }
                if pol.fill_tb && self.cand.len() > 1 {
                    // Min-deficiency over the (small) degree candidate list,
                    // uniform among deficiency ties.
                    let cands = std::mem::take(&mut self.cand);
                    let mut bestdef = u32::MAX;
                    let mut cnt = 0u32;
                    let mut sel = cands[0];
                    for &v in &cands {
                        let d = self.deficiency(v as usize);
                        if d < bestdef {
                            bestdef = d;
                            cnt = 1;
                            sel = v;
                        } else if d == bestdef {
                            cnt += 1;
                            if below(rng, cnt) == 0 {
                                sel = v;
                            }
                        }
                    }
                    self.cand = cands;
                    pick = sel as usize;
                } else {
                    let k = below(rng, self.cand.len() as u32) as usize;
                    pick = self.cand[k] as usize;
                }
            }
            let c = self.eliminate(pick);
            f += c * c;
            out.push(pick);
            if f >= bound {
                return None;
            }
        }
        Some(f)
    }
}

/// Search for an elimination order with a smaller `Σ c_j²` than the incumbent.
///
/// `seed` / `seed_flops` are the portfolio's current best (used both as the
/// pruning bound and as the LNS base). `budget` is in word-operations — a
/// deterministic proxy for time, calibrated at the call site. Returns the
/// improved order and its exact objective, or `None` if nothing beat the seed.
///
/// The caller MUST re-score the returned order through the trusted scorer; this
/// function's own accumulator is the elimination-game identity, not the shipped
/// `column_counts_gnp` path.
pub(crate) fn search(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    seed: &[usize],
    seed_flops: u64,
    budget: i64,
    rng_seed: u64,
) -> Option<(Vec<usize>, u64)> {
    let adj0 = Game::build_adj(n, col_ptr, row_idx)?;
    search_with(n, &adj0, seed, seed_flops, budget, rng_seed, Params::DEFAULT)
}

/// Tunable knobs. `Params::DEFAULT` is what ships; the probe overrides them to
/// attribute the phases against each other.
#[derive(Clone, Copy)]
pub(crate) struct Params {
    /// Phase-A (from-scratch restarts) share of the budget, as `num/den`.
    pub(crate) phase_a_num: i64,
    pub(crate) phase_a_den: i64,
    /// Bitmask over `POLICIES`.
    pub(crate) pol_mask: u8,
    /// Run the prefix-freezing LNS phase with the remaining budget.
    pub(crate) lns: bool,
    /// How the LNS draws the frozen prefix length. `0` = uniform over
    /// `0..n`; `1` = log-uniform TAIL length (short perturbations of the
    /// incumbent's tail are far more common, long ones still reachable).
    pub(crate) prefix_mode: u8,
    /// Consecutive rejected kicks before the ILS restarts the walk from a
    /// fresh from-scratch randomized greedy. `0` disables restarts, which is
    /// what SHIPS: measured at every setting from 1 to 2000 at the shipped
    /// budget, a restart is either never reached (>= 30, identical score) or
    /// catastrophic (limit 1: -0.00104 vs -0.00258). The plateau walk must not
    /// be evicted from its basin.
    pub(crate) stall_limit: usize,
    /// Threshold accepting: the ILS walk may move to any solution within
    /// `accept_num/accept_den` of the GLOBAL best (0 = sideways only, which is
    /// what SHIPS). The threshold is relative to `best`, not to the current
    /// point, so the walk cannot drift away without bound. Measured: 0.1% /
    /// 0.5% / 2% / 5% thresholds all score WORSE than pure sideways
    /// (-0.00213 / -0.00231 / -0.00234 / -0.00208 vs -0.00258). Accepting
    /// worse solutions is not what this landscape needs; drifting across
    /// equal-cost plateaus is.
    pub(crate) accept_num: u64,
    pub(crate) accept_den: u64,
    /// Number of independent ILS walks kept in rotation. Measured a wash at
    /// the shipped budget (1 / 2 / 4 / 8 walks: -0.00258 / -0.00214 /
    /// -0.00255 / -0.00227, with 63 / 64 / 66 / 65 matrices improved) — the
    /// spread is single-matrix instance noise, so 1 ships.
    pub(crate) walks: usize,
}

impl Params {
    pub(crate) const DEFAULT: Params = Params {
        // Measured on the full dev corpus at a 600M-op budget
        // (`probe_rgreedy`, phase/policy attribution sweep):
        //   LNS off                          -0.000402
        //   phase_a 1/4, slacks {0,1,2,fill} -0.001655
        //   phase_a 1/16, all 8 policies,
        //     mixed prefix draw              -0.002434   <- shipped
        // The two levers that matter are (a) spending almost the whole budget
        // in the LNS phase rather than on from-scratch restarts (4x), and
        // (b) a WIDE slack ladder — the best single slack is never the best
        // portfolio, because different matrices want different amounts of
        // greedy myopia.
        phase_a_num: 1,
        phase_a_den: 16,
        pol_mask: 0b1111_1111,
        lns: true,
        prefix_mode: 2,
        stall_limit: 0,
        accept_num: 0,
        accept_den: 1,
        walks: 1,
    };
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn search_with(
    n: usize,
    adj0: &[u64],
    seed: &[usize],
    seed_flops: u64,
    budget: i64,
    rng_seed: u64,
    par: Params,
) -> Option<(Vec<usize>, u64)> {
    let mut g = Game::new(n, adj0)?;
    let mut rng = rng_seed | 1;
    let mut best = seed_flops;
    let mut best_ord: Vec<usize> = Vec::new();
    let mut out: Vec<usize> = Vec::with_capacity(n);

    // ── Phase A: from-scratch randomized greedy, cycling four policies ──────
    // The first quarter of the budget establishes whether the exact-degree
    // family reaches the incumbent at all; the LNS phase then works from
    // whichever of (seed, phase-A best) is better.
    const POLICIES: [Policy; 8] = [
        Policy { slack: 0, fill_tb: false },
        Policy { slack: 1, fill_tb: false },
        Policy { slack: 2, fill_tb: false },
        Policy { slack: 1, fill_tb: true },
        Policy { slack: 3, fill_tb: false },
        Policy { slack: 5, fill_tb: false },
        Policy { slack: 8, fill_tb: false },
        Policy { slack: 16, fill_tb: false },
    ];
    let pols: Vec<Policy> = (0..POLICIES.len())
        .filter(|&i| par.pol_mask & (1 << i) != 0)
        .map(|i| POLICIES[i])
        .collect();
    if pols.is_empty() {
        return None;
    }
    // Total-ops ceiling: 1.25x the budget, so an over-long run can overshoot
    // by at most a quarter of the budget instead of by a whole run.
    let hard_cap = budget + budget / 4;
    let phase_a_end = if par.lns {
        budget * par.phase_a_num / par.phase_a_den
    } else {
        budget
    };
    let mut it = 0usize;
    // A single greedy run is ATOMIC: it cannot be stopped halfway and still
    // yield an ordering, and at n=6000 one run already costs ~130M ops. So the
    // loop guard must reserve the cost of the run it is about to start, using
    // the previous run's measured cost, or the budget is a lower bound rather
    // than an upper one and the added wall time overshoots by a whole run.
    let mut last_run: i64 = 0;
    while g.ops + last_run <= phase_a_end {
        let before = g.ops;
        let pol = pols[it % pols.len()];
        it += 1;
        if let Some(f) = g.run(&[], pol, &mut rng, best, hard_cap, &mut out) {
            if f < best {
                best = f;
                best_ord = out.clone();
            }
        }
        last_run = last_run.max(g.ops - before);
    }

    // ── Phase B: iterated local search around the incumbent ────────────────
    // Replay a prefix of the CURRENT solution verbatim, then re-randomize the
    // suffix. `Σ c²` is dominated by the LAST columns eliminated, so freezing a
    // prefix and re-searching the tail is the operator that actually targets
    // the objective's mass.
    //
    // Acceptance is SIDEWAYS (`f <= cur`), not strictly improving. The
    // objective is massively degenerate — huge plateaus of equal-cost orders
    // differing only in the elimination-tree postorder — and a strict-descent
    // walk freezes on the first one it lands in. Sideways moves let it drift
    // across the plateau to a point that has a downhill neighbour. The GLOBAL
    // best is tracked separately and only ever updated on a strict improvement,
    // so the returned answer is still monotone.
    //
    // After `stall_limit` consecutive rejected kicks the walk is restarted from
    // a fresh from-scratch randomized greedy (a real ILS restart, not a
    // re-seed from the incumbent, which would just re-enter the same basin).
    let start: Vec<usize> = if best_ord.is_empty() {
        seed.to_vec()
    } else {
        best_ord.clone()
    };
    let nwalk = par.walks.max(1);
    let mut cur: Vec<Vec<usize>> = vec![start; nwalk];
    let mut cur_f: Vec<u64> = vec![best; nwalk];
    let mut stall = 0usize;
    let mut kick: Vec<usize> = Vec::new();
    while par.lns && g.ops + last_run <= budget {
        let before = g.ops;
        let p = match par.prefix_mode {
            0 => below(&mut rng, n as u32) as usize,
            3 => below(&mut rng, (n as u32).div_ceil(2)) as usize,
            4 => below(&mut rng, (n as u32).div_ceil(4)) as usize,
            2 if it % 2 == 0 => below(&mut rng, n as u32) as usize,
            _ => {
                // Log-uniform tail: pick an exponent, then a length inside it.
                let bits = usize::BITS - n.leading_zeros();
                let e = below(&mut rng, bits);
                let k = 1 + below(&mut rng, 1u32 << e) as usize;
                n.saturating_sub(k.min(n))
            }
        };
        let pol = pols[it % pols.len()];
        let wi = it % nwalk;
        it += 1;
        let thresh = best + best / par.accept_den * par.accept_num;
        let bound = if thresh > cur_f[wi] { thresh } else { cur_f[wi] } + 1;
        let taken = std::mem::take(&mut cur[wi]);
        let r = g.run(&taken[..p.min(taken.len())], pol, &mut rng, bound, hard_cap, &mut out);
        cur[wi] = taken;
        last_run = last_run.max(g.ops - before);
        match r {
            Some(f) => {
                if f < best {
                    best = f;
                    best_ord = out.clone();
                }
                // `f <= bound` by the pruning rule, so every returned run is
                // accepted: this is the sideways / threshold drift.
                cur_f[wi] = f;
                std::mem::swap(&mut cur[wi], &mut out);
                stall = 0;
            }
            None => {
                stall += 1;
                #[allow(clippy::needless_late_init)]
                if par.stall_limit != 0 && stall >= par.stall_limit {
                    stall = 0;
                    let pol = pols[it % pols.len()];
                    it += 1;
                    if let Some(f) = g.run(&[], pol, &mut rng, u64::MAX, hard_cap, &mut kick) {
                        if f < best {
                            best = f;
                            best_ord = kick.clone();
                        }
                        cur_f[wi] = f;
                        cur[wi].clear();
                        cur[wi].extend_from_slice(&kick);
                    }
                }
            }
        }
    }

    if best < seed_flops && !best_ord.is_empty() {
        Some((best_ord, best))
    } else {
        None
    }
}

/// The four parameter configurations the parallel fan-out runs, one per
/// thread. Measured (`probe_rgreedy`, RG_SEEDS/RG_MULTI): four INDEPENDENT
/// PRNG seeds at the same parameters recover -0.00309 of dev score where one
/// stream recovers -0.00258, and varying the LNS prefix draw per stream on top
/// of that reaches -0.00327 with 74 matrices improved instead of 63. The
/// prefix draw is the parameter worth varying because the two extremes win on
/// DIFFERENT matrices — uniform-over-`0..n` prefixes score better in total,
/// log-uniform tails improve more matrices — and a fan-out can have both
/// instead of choosing.
/// ONE stream of the fan-out, addressed by index — the unit the parallel
/// arm's task queue schedules. PURE: the result is a function of
/// `(pattern, seed, seed_flops, budget, k)` and nothing else (no wall-clock,
/// no shared state, no thread identity), so the caller can run these in any
/// order, on any number of threads, and merge by `(flops, k)` argmin to get a
/// byte-identical answer. `k` selects both the PRNG seed and the parameter
/// variant (see [`stream_params`]).
pub(crate) fn search_seed(
    n: usize,
    adj0: &[u64],
    seed: &[usize],
    seed_flops: u64,
    budget: i64,
    k: usize,
) -> Option<(Vec<usize>, u64)> {
    search_with(
        n,
        adj0,
        seed,
        seed_flops,
        budget,
        stream_rng(k),
        stream_params(k),
    )
}

/// The PRNG seed for stream `k`. Fixed constant, no entropy, no clock.
fn stream_rng(k: usize) -> u64 {
    0x9E37_79B9_7F4A_7C15u64.wrapping_mul(2 * k as u64 + 1) ^ (k as u64) << 32
}

fn stream_params(k: usize) -> Params {
    let mut p = Params::DEFAULT;
    match k % 4 {
        0 => {}
        1 => p.prefix_mode = 0,
        2 => p.prefix_mode = 1,
        _ => {
            p.prefix_mode = 2;
            p.pol_mask = 0b0011_0111;
        }
    }
    p
}

/// Run `threads` INDEPENDENT searches concurrently and return the best.
///
/// Each stream is a pure function of `(pattern, seed, params, budget)` with no
/// shared state whatsoever — no shared incumbent, no work stealing, no
/// wall-clock — so the set of results is fixed before any thread starts, and
/// the merge below (strict argmin, ties broken by the LOWEST stream index)
/// picks the same one regardless of completion order. Byte-identical output
/// across runs, as the rules require.
///
/// The point is WALL TIME, not throughput: the grader has 4 vCPUs, so four
/// streams of `budget` ops each cost the same wall time as one, and buy 4x the
/// search. The thread cap is [`super::parallel::PAR_MAX_THREADS`] so the whole
/// candidate binary has a single source of truth for it.
pub(crate) fn search_par(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    seed: &[usize],
    seed_flops: u64,
    budget: i64,
    rng_seed: u64,
) -> Option<(Vec<usize>, u64)> {
    let _ = rng_seed;
    let adj0 = Game::build_adj(n, col_ptr, row_idx)?;
    let adj0: &[u64] = &adj0;
    let streams = super::parallel::PAR_MAX_THREADS.max(1);
    if streams == 1 {
        return search_seed(n, adj0, seed, seed_flops, budget, 0);
    }
    let results: Vec<Option<(Vec<usize>, u64)>> = std::thread::scope(|sc| {
        let handles: Vec<_> = (0..streams)
            .map(|k| {
                sc.spawn(move || {
                    // A stream that panicked would silently drop a result and
                    // make the merge depend on which thread died — wrap it, so
                    // the worst case is a missing (never a differing) result.
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        search_seed(n, adj0, seed, seed_flops, budget, k)
                    }))
                    .unwrap_or(None)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or(None))
            .collect()
    });
    let mut best: Option<(Vec<usize>, u64)> = None;
    for r in results.into_iter().flatten() {
        // Strict `<` walking streams in index order = lowest index wins ties.
        if best.as_ref().is_none_or(|(_, bf)| r.1 < *bf) {
            best = Some(r);
        }
    }
    best
}

/// Exact adjacent-transposition descent around a completed ordering.
///
/// At an elimination state, consider consecutive pivots `a, b`. If they are
/// non-adjacent, their eliminations commute and swapping them is score-neutral.
/// If they are adjacent, after either first pivot is eliminated the survivor's
/// live neighbourhood is the same union
/// `(N(a) union N(b)) - {a, b}`. The residual graph after both pivots is also
/// identical. Consequently only the first column count differs, and placing
/// the lower-current-degree endpoint first is the exact better choice for the
/// pair. Disjoint pair choices in one sweep are independent because each pair
/// leaves the same eliminated set at the next boundary.
///
/// Sweeps alternate pair parity, so two sweeps examine every adjacent boundary.
/// The operation budget is deterministic and cumulative. Exhaustion discards
/// the whole candidate instead of returning a partially explored result.
pub(crate) fn adjacent_pair_descent(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    seed: &[usize],
    sweeps: usize,
    budget: i64,
) -> Option<Vec<usize>> {
    #[cfg(test)]
    ADJACENT_PAIR_DESCENT_CALLS.with(|calls| calls.set(calls.get() + 1));

    if n < 2 || seed.len() != n || sweeps == 0 || budget <= 0 {
        return None;
    }
    let mut seen = vec![false; n];
    for &v in seed {
        if v >= n || seen[v] {
            return None;
        }
        seen[v] = true;
    }

    let adj0 = Game::build_adj(n, col_ptr, row_idx)?;
    let mut game = Game::new(n, &adj0)?;
    let mut cur = seed.to_vec();
    let mut next = Vec::with_capacity(n);
    let mut changed_any = false;

    for sweep in 0..sweeps {
        game.reset();
        if game.ops > budget {
            return None;
        }
        next.clear();

        let mut k = 0usize;
        if sweep & 1 == 1 {
            let v = cur[0];
            next.push(v);
            game.eliminate(v);
            if game.ops > budget {
                return None;
            }
            k = 1;
        }

        let mut changed = false;
        while k + 1 < n {
            let a = cur[k];
            let b = cur[k + 1];
            let adjacent = game.adj[a * game.w + (b >> 6)] & (1u64 << (b & 63)) != 0;
            let swap = adjacent && game.deg[b] < game.deg[a];
            let (first, second) = if swap { (b, a) } else { (a, b) };
            changed |= swap;
            next.push(first);
            next.push(second);
            game.eliminate(first);
            if game.ops > budget {
                return None;
            }
            game.eliminate(second);
            if game.ops > budget {
                return None;
            }
            k += 2;
        }
        if k < n {
            let v = cur[k];
            next.push(v);
            game.eliminate(v);
            if game.ops > budget {
                return None;
            }
        }

        if changed {
            changed_any = true;
            std::mem::swap(&mut cur, &mut next);
        }
    }

    changed_any.then_some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pattern;

    fn trusted_flops(pattern: &Pattern, perm: &[usize]) -> u64 {
        let scoring = super::super::ScoringPattern {
            n: pattern.n,
            col_ptr: pattern.col_ptr.clone(),
            row_idx: pattern.row_idx.clone(),
        };
        super::super::flops_of(&scoring, perm)
    }

    #[test]
    fn adjacent_pair_descent_improves_star_exactly_and_deterministically() {
        let pattern = Pattern::from_edges(4, &[(0, 1), (0, 2), (0, 3)]);
        let seed = vec![0, 1, 2, 3];
        let first = adjacent_pair_descent(
            pattern.n,
            &pattern.col_ptr,
            &pattern.row_idx,
            &seed,
            2,
            1_000_000,
        )
        .expect("the center-first star contains strict adjacent inversions");
        let second = adjacent_pair_descent(
            pattern.n,
            &pattern.col_ptr,
            &pattern.row_idx,
            &seed,
            2,
            1_000_000,
        )
        .expect("the deterministic replay must find the same inversions");

        assert_eq!(first, second);
        assert_eq!(first.len(), pattern.n);
        let mut sorted = first.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..pattern.n).collect::<Vec<_>>());
        assert!(trusted_flops(&pattern, &first) < trusted_flops(&pattern, &seed));
    }

    #[test]
    fn adjacent_pair_descent_matches_trusted_score_on_small_graphs() {
        let mut state = 0xD1B5_4A32_D192_ED03u64;
        let mut improved = 0usize;
        for case in 0..96 {
            let n = 3 + case % 9;
            let mut edges = Vec::new();
            for u in 0..n {
                for v in u + 1..n {
                    if xs64(&mut state) % 5 <= 1 {
                        edges.push((u, v));
                    }
                }
            }
            let pattern = Pattern::from_edges(n, &edges);
            let mut seed: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = xs64(&mut state) as usize % (i + 1);
                seed.swap(i, j);
            }

            if let Some(cand) = adjacent_pair_descent(
                n,
                &pattern.col_ptr,
                &pattern.row_idx,
                &seed,
                2,
                2_000_000,
            ) {
                improved += 1;
                let mut sorted = cand.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..n).collect::<Vec<_>>());
                assert!(
                    trusted_flops(&pattern, &cand) < trusted_flops(&pattern, &seed),
                    "case {case} returned a non-improving exchange"
                );
            }
        }
        assert!(improved > 0, "property sweep did not exercise any exchange");
    }

    #[test]
    fn adjacent_pair_descent_fails_closed_on_edge_cases() {
        let singleton = Pattern::from_edges(1, &[]);
        assert!(adjacent_pair_descent(
            singleton.n,
            &singleton.col_ptr,
            &singleton.row_idx,
            &[0],
            2,
            1_000_000,
        )
        .is_none());

        let edge = Pattern::from_edges(2, &[(0, 1)]);
        assert!(adjacent_pair_descent(
            edge.n,
            &edge.col_ptr,
            &edge.row_idx,
            &[0, 0],
            2,
            1_000_000,
        )
        .is_none());
        assert!(adjacent_pair_descent(
            edge.n,
            &edge.col_ptr,
            &edge.row_idx,
            &[0, 1],
            2,
            1,
        )
        .is_none());
    }

    #[test]
    fn adjacent_pair_descent_is_reached_from_default_order() {
        let n = 1_000;
        let edges: Vec<(usize, usize)> = (0..n - 1).map(|v| (v, v + 1)).collect();
        let pattern = Pattern::from_edges(n, &edges);
        ADJACENT_PAIR_DESCENT_CALLS.with(|calls| calls.set(0));

        let perm = super::super::order(&pattern);

        ADJACENT_PAIR_DESCENT_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(perm.len(), n);
        let mut sorted = perm;
        sorted.sort_unstable();
        assert_eq!(sorted, (0..n).collect::<Vec<_>>());
    }
}
