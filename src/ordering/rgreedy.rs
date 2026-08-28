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
pub(crate) const MAX_N: usize = 4_000;

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

/// The elimination game on a bitset fill graph.
///
/// Invariant: `adj[u]` holds exactly `u`'s neighbours in the CURRENT fill graph
/// restricted to LIVE vertices (never `u` itself, never an eliminated vertex),
/// and `deg[u] == popcount(adj[u])` for every live `u`.
pub(crate) struct Game {
    n: usize,
    w: usize,
    adj0: Vec<u64>,
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

impl Game {
    /// `None` if `n` is out of range or the pattern references an out-of-range
    /// row (the caller then simply skips this phase).
    pub(crate) fn new(n: usize, col_ptr: &[usize], row_idx: &[usize]) -> Option<Game> {
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
        Some(Game {
            n,
            w,
            adj: adj0.clone(),
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
        self.adj.copy_from_slice(&self.adj0);
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

/// A pivot policy: pick uniformly among the live vertices whose degree is
/// within `slack` of the minimum; when `fill_tb` is set, break that set by
/// smallest deficiency first (a min-fill lookahead over a min-degree
/// candidate list).
#[derive(Clone, Copy)]
struct Policy {
    slack: u32,
    fill_tb: bool,
}

impl Game {
    /// One randomized greedy run. `fixed` is a prefix of pivots replayed
    /// verbatim before randomization starts (the LNS operator); pass an empty
    /// slice for a from-scratch run. Returns `None` as soon as the partial
    /// objective reaches `bound` (pruned), since `Σ c²` only grows.
    fn run(
        &mut self,
        fixed: &[usize],
        pol: Policy,
        rng: &mut u64,
        bound: u64,
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
    search_with(n, col_ptr, row_idx, seed, seed_flops, budget, rng_seed, Params::DEFAULT)
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
    };
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn search_with(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    seed: &[usize],
    seed_flops: u64,
    budget: i64,
    rng_seed: u64,
    par: Params,
) -> Option<(Vec<usize>, u64)> {
    let mut g = Game::new(n, col_ptr, row_idx)?;
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
    let phase_a_end = if par.lns {
        budget * par.phase_a_num / par.phase_a_den
    } else {
        budget
    };
    let mut it = 0usize;
    while g.ops < phase_a_end {
        let pol = pols[it % pols.len()];
        it += 1;
        if let Some(f) = g.run(&[], pol, &mut rng, best, &mut out) {
            if f < best {
                best = f;
                best_ord = out.clone();
            }
        }
    }

    // ── Phase B: LNS around the incumbent ───────────────────────────────────
    // Replay a random prefix of the incumbent verbatim, then re-randomize the
    // suffix. `Σ c²` is dominated by the LAST columns eliminated, so freezing a
    // prefix and re-searching the tail is the operator that actually targets
    // the objective's mass; a short prefix degenerates to a fresh run, which is
    // why the prefix is drawn over the whole range.
    let base: Vec<usize> = if best_ord.is_empty() {
        seed.to_vec()
    } else {
        best_ord.clone()
    };
    let mut base = base;
    while par.lns && g.ops < budget {
        let p = if par.prefix_mode == 0 || (par.prefix_mode == 2 && it % 2 == 0) {
            below(&mut rng, n as u32) as usize
        } else {
            // Log-uniform tail: pick an exponent, then a length inside it.
            let bits = usize::BITS - n.leading_zeros();
            let e = below(&mut rng, bits) as u32;
            let k = 1 + below(&mut rng, 1u32 << e) as usize;
            n.saturating_sub(k.min(n))
        };
        let pol = pols[it % pols.len()];
        it += 1;
        let taken = std::mem::take(&mut base);
        let r = g.run(&taken[..p], pol, &mut rng, best, &mut out);
        base = taken;
        if let Some(f) = r {
            if f < best {
                best = f;
                best_ord = out.clone();
                base = best_ord.clone();
            }
        }
    }

    if best < seed_flops && !best_ord.is_empty() {
        Some((best_ord, best))
    } else {
        None
    }
}
