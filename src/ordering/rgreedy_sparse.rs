//! Sparse (quotient-graph) randomized greedy elimination-game search on the
//! EXACT objective `Σ c_j²` — the mid-band extension of `rgreedy.rs`.
//!
//! ## Why a second representation
//!
//! `rgreedy.rs` maintains the true fill graph as n-bit bitsets: every
//! elimination is `O(deg · n/64)` words and every reset copies `n²/64` words.
//! That is unbeatable at `n ≤ 3000` and hopeless at `n = 20 000` (a single
//! reset would be 6.2M words). This module runs the SAME exact elimination
//! game — true column count `c(v) = 1 + |N_{G_S}(v)|` charged at elimination,
//! partial-sum pruning against the incumbent, identical policy ladder and
//! prefix-freezing LNS — on a quotient-graph representation instead:
//!
//! - Eliminated pivots become ELEMENTS with a fixed boundary list `lvars[e]`
//!   (the live clique members). Elements adjacent to a new pivot are absorbed
//!   (they are subsets of the new element) and die.
//! - A live variable `v`'s fill neighbourhood is `avars[v] ∪ ⋃_{e∈aelems[v]}
//!   lvars[e] \ {v}`, with `avars[v]` kept DISJOINT from every adjacent
//!   element's boundary (entries covered by a new element are pruned at the
//!   moment the element forms — the classical quotient-graph invariant), so
//!   the union is over the element lists only.
//! - External degrees are EXACT, recomputed per touched vertex with a stamped
//!   scatter over its element boundaries — this is MMD's exact degree, not
//!   AMD's approximation. The common post-absorption case (no original
//!   neighbours left, exactly one element) is `O(1)`: `deg = |lvars[e]| - 1`.
//! - Pivot selection is by doubly-linked DEGREE BUCKETS (min-scan is
//!   `O(candidates)`, not `O(live)`), with the same reservoir sampling and
//!   lowest-index determinism as the bitset version.
//!
//! ## Determinism and budget
//!
//! Fixed-seed xorshift64, fixed policy schedule, and a deterministic
//! OPERATION counter charged per list element actually touched — never
//! wall-clock. Unlike the bitset game, a single run's cost here is not a
//! priori bounded by `n·w` terms (a hub pivot can be quadratic), so `run()`
//! additionally hard-aborts mid-run once the op counter passes the caller's
//! budget; the abort is a pure function of the pattern + seed, so two runs
//! still agree exactly.
//!
//! The min-fill tie-break policy of the bitset game is NOT carried over: a
//! deficiency count in a quotient graph costs a full pairwise boundary scan.
//! The slack ladder is kept as-is minus that one policy.

#![allow(dead_code)]

/// Hard n ceiling this module will allocate for. Memory is O(n + nnz) lists;
/// the SHIPPED gate at the call site is chosen for TIME, not memory.
pub(crate) const MAX_N: usize = 40_000;

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

const LIVE: u8 = 0;
const ELEM: u8 = 1;
const DEAD: u8 = 2;

/// The exact elimination game on a quotient graph.
pub(crate) struct SparseGame {
    n: usize,
    /// Symmetrized, deduplicated, self-loop-free CSR of the original pattern.
    ptr: Vec<u32>,
    idx: Vec<u32>,
    /// Live original-neighbour lists, disjoint from adjacent element boundaries.
    avars: Vec<Vec<u32>>,
    /// Adjacent element ids (dead ones dropped lazily on scan).
    aelems: Vec<Vec<u32>>,
    /// Element boundaries: fixed at creation, all members live while the
    /// element is live (a member's elimination absorbs the element).
    lvars: Vec<Vec<u32>>,
    /// LIVE (variable) / ELEM (live element) / DEAD (absorbed element).
    state: Vec<u8>,
    /// Exact external degree of each live variable.
    deg: Vec<u32>,
    // Degree buckets: doubly-linked lists threaded through dnext/dprev.
    dhead: Vec<i32>,
    dnext: Vec<i32>,
    dprev: Vec<i32>,
    /// Lower bound on the true minimum degree (advanced lazily at selection).
    dmin: u32,
    nlive: usize,
    // Stamped scatter marks: mark/stamp for L_p membership (one stamp per
    // elimination), mark2/stamp2 for per-vertex degree unions.
    mark: Vec<u32>,
    stamp: u32,
    mark2: Vec<u32>,
    stamp2: u32,
    /// L_p scratch.
    lp: Vec<u32>,
    /// Deterministic work counter, in touched-list-element units. The ONLY
    /// budget signal — no wall-clock anywhere in this module.
    pub(crate) ops: i64,
}

impl SparseGame {
    /// `None` if `n` is out of range or the pattern references an out-of-range
    /// row (the caller then simply skips this phase).
    pub(crate) fn new(n: usize, col_ptr: &[usize], row_idx: &[usize]) -> Option<SparseGame> {
        if n == 0 || n > MAX_N {
            return None;
        }
        // Symmetrize + dedup into CSR (counting sort by endpoint; dedup via a
        // per-row seen-stamp since rows arrive grouped).
        let mut cnt = vec![0u32; n + 1];
        for v in 0..n {
            let (lo, hi) = (col_ptr[v], col_ptr[v + 1]);
            for &r in &row_idx[lo..hi] {
                if r >= n {
                    return None;
                }
                if r != v {
                    cnt[v + 1] += 1;
                    cnt[r + 1] += 1;
                }
            }
        }
        for i in 0..n {
            cnt[i + 1] += cnt[i];
        }
        let ptr = cnt;
        let mut idx = vec![0u32; ptr[n] as usize];
        let mut fill = ptr.clone();
        for v in 0..n {
            let (lo, hi) = (col_ptr[v], col_ptr[v + 1]);
            for &r in &row_idx[lo..hi] {
                if r != v {
                    idx[fill[v] as usize] = r as u32;
                    fill[v] += 1;
                    idx[fill[r] as usize] = v as u32;
                    fill[r] += 1;
                }
            }
        }
        // Dedup each row in place (sort_unstable keeps determinism and makes
        // every later list order canonical).
        let mut w = 0usize;
        let mut newptr = vec![0u32; n + 1];
        for v in 0..n {
            let (lo, hi) = (ptr[v] as usize, ptr[v + 1] as usize);
            idx[lo..hi].sort_unstable();
            let start = w;
            let mut last = u32::MAX;
            for k in lo..hi {
                let u = idx[k];
                if u != last {
                    idx[w] = u;
                    w += 1;
                    last = u;
                }
            }
            // (start..w) may overlap (lo..hi) but w <= lo + written, and we
            // only ever read ahead of where we write.
            let _ = start;
            newptr[v + 1] = w as u32;
        }
        idx.truncate(w);
        Some(SparseGame {
            n,
            ptr: newptr,
            idx,
            avars: vec![Vec::new(); n],
            aelems: vec![Vec::new(); n],
            lvars: vec![Vec::new(); n],
            state: vec![LIVE; n],
            deg: vec![0; n],
            dhead: vec![-1; n + 1],
            dnext: vec![-1; n],
            dprev: vec![-1; n],
            dmin: 0,
            nlive: 0,
            mark: vec![0; n],
            stamp: 0,
            mark2: vec![0; n],
            stamp2: 0,
            lp: Vec::new(),
            ops: 0,
        })
    }

    #[inline]
    fn bucket_remove(&mut self, v: usize) {
        let nx = self.dnext[v];
        let pv = self.dprev[v];
        if pv >= 0 {
            self.dnext[pv as usize] = nx;
        } else {
            self.dhead[self.deg[v] as usize] = nx;
        }
        if nx >= 0 {
            self.dprev[nx as usize] = pv;
        }
    }

    #[inline]
    fn bucket_insert(&mut self, v: usize, d: u32) {
        self.deg[v] = d;
        let h = self.dhead[d as usize];
        self.dnext[v] = h;
        self.dprev[v] = -1;
        if h >= 0 {
            self.dprev[h as usize] = v as i32;
        }
        self.dhead[d as usize] = v as i32;
        if d < self.dmin {
            self.dmin = d;
        }
    }

    fn reset(&mut self) {
        let n = self.n;
        for h in self.dhead.iter_mut() {
            *h = -1;
        }
        self.dmin = u32::MAX.min(n as u32);
        for v in 0..n {
            self.state[v] = LIVE;
            self.aelems[v].clear();
            self.lvars[v].clear();
            let (lo, hi) = (self.ptr[v] as usize, self.ptr[v + 1] as usize);
            self.avars[v].clear();
            self.avars[v].extend_from_slice(&self.idx[lo..hi]);
            let d = (hi - lo) as u32;
            self.bucket_insert(v, d);
        }
        self.nlive = n;
        self.ops += (8 * n + self.idx.len()) as i64;
    }

    /// Eliminate `v` (must be live), returning its column count `c = 1 + |N(v)|`.
    fn eliminate(&mut self, v: usize) -> u64 {
        debug_assert_eq!(self.state[v], LIVE);
        // ── Materialize L_v = avars[v] ∪ ⋃ lvars[e] \ {v}, stamped. ─────────
        self.stamp += 1;
        let st = self.stamp;
        self.mark[v] = st;
        let mut lp = std::mem::take(&mut self.lp);
        lp.clear();
        let mut scanned = 0usize;
        let avs = std::mem::take(&mut self.avars[v]);
        for &u in &avs {
            // avars holds only live vertices (pruned on every element
            // formation that could have eliminated them — see invariant note
            // in `new`'s doc), but a neighbour eliminated as a PIVOT leaves a
            // stale entry; skip those lazily.
            if self.state[u as usize] == LIVE && self.mark[u as usize] != st {
                self.mark[u as usize] = st;
                lp.push(u);
            }
        }
        scanned += avs.len();
        self.avars[v] = avs;
        let aes = std::mem::take(&mut self.aelems[v]);
        for &e in &aes {
            if self.state[e as usize] != ELEM {
                continue;
            }
            for &u in &self.lvars[e as usize] {
                if self.mark[u as usize] != st {
                    debug_assert_eq!(self.state[u as usize], LIVE);
                    self.mark[u as usize] = st;
                    lp.push(u);
                }
            }
            scanned += self.lvars[e as usize].len();
            // Absorbed: the new element's boundary is a superset.
            self.state[e as usize] = DEAD;
        }
        self.aelems[v] = aes;
        let c = lp.len() as u64 + 1;

        // ── v becomes an element with boundary L_v. ─────────────────────────
        self.bucket_remove(v);
        self.state[v] = ELEM;
        self.nlive -= 1;
        self.avars[v].clear();
        self.aelems[v].clear();
        self.lvars[v].clear();
        self.lvars[v].extend_from_slice(&lp);

        // ── Update every boundary vertex: prune, relink, exact degree. ──────
        for i in 0..lp.len() {
            let u = lp[i] as usize;
            // avars[u] := avars[u] \ ({v} ∪ L_v ∪ eliminated) — covered by the
            // new element (or gone). Keeps the disjointness invariant.
            let mut av = std::mem::take(&mut self.avars[u]);
            scanned += av.len();
            av.retain(|&x| self.state[x as usize] == LIVE && self.mark[x as usize] != st);
            let d_a = av.len() as u32;
            self.avars[u] = av;
            // aelems[u] := live(aelems[u]) ∪ {v}.
            let mut ae = std::mem::take(&mut self.aelems[u]);
            scanned += ae.len();
            ae.retain(|&e| self.state[e as usize] == ELEM);
            ae.push(v as u32);
            // Exact external degree.
            let d = if d_a == 0 && ae.len() == 1 {
                // Single-element shortcut: N(u) is exactly that boundary.
                self.lvars[ae[0] as usize].len() as u32 - 1
            } else {
                self.stamp2 += 1;
                let s2 = self.stamp2;
                self.mark2[u] = s2;
                let mut d = d_a;
                for &x in &self.avars[u] {
                    self.mark2[x as usize] = s2;
                }
                for &e in &ae {
                    for &x in &self.lvars[e as usize] {
                        if self.mark2[x as usize] != s2 {
                            self.mark2[x as usize] = s2;
                            d += 1;
                        }
                    }
                    scanned += self.lvars[e as usize].len();
                }
                d
            };
            self.aelems[u] = ae;
            self.bucket_remove(u);
            self.bucket_insert(u, d);
            scanned += 6;
        }
        self.ops += (scanned + lp.len() + 16) as i64;
        self.lp = lp;
        c
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
/// within `slack` of the minimum. (No min-fill tie-break in the sparse tier —
/// deficiency is a pairwise boundary scan here.)
#[derive(Clone, Copy)]
struct Policy {
    slack: u32,
}

impl SparseGame {
    /// One randomized greedy run. `fixed` is a prefix of pivots replayed
    /// verbatim before randomization starts (the LNS operator). Returns `None`
    /// as soon as the partial objective reaches `bound` (pruned — `Σ c²` only
    /// grows) or the op counter passes `hard_ops` (a single sparse run has no
    /// a-priori cost bound, unlike the bitset game).
    fn run(
        &mut self,
        fixed: &[usize],
        pol: Policy,
        rng: &mut u64,
        bound: u64,
        hard_ops: i64,
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
        while self.nlive > 0 {
            if self.ops >= hard_ops {
                return None;
            }
            // Advance dmin to the first non-empty bucket (lazy lower bound).
            let mut d = self.dmin as usize;
            while self.dhead[d] < 0 {
                d += 1;
            }
            self.dmin = d as u32;
            let cut = (d as u32).saturating_add(pol.slack).min(self.n as u32 - 1);
            // Reservoir-sample uniformly over all vertices in buckets
            // dmin ..= cut (deterministic traversal order).
            let mut cnt = 0u32;
            let mut sel = -1i32;
            let mut scanned = 0usize;
            let mut b = d;
            while b <= cut as usize {
                let mut x = self.dhead[b];
                while x >= 0 {
                    cnt += 1;
                    scanned += 1;
                    if below(rng, cnt) == 0 {
                        sel = x;
                    }
                    x = self.dnext[x as usize];
                }
                b += 1;
            }
            self.ops += (scanned + 8) as i64;
            let pick = sel as usize;
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

/// Tunable knobs, mirroring `rgreedy::Params` (minus the min-fill policy).
#[derive(Clone, Copy)]
pub(crate) struct Params {
    /// Phase-A (from-scratch restarts) share of the budget, as `num/den`.
    pub(crate) phase_a_num: i64,
    pub(crate) phase_a_den: i64,
    /// Bitmask over `POLICIES`.
    pub(crate) pol_mask: u8,
    /// Run the prefix-freezing LNS phase with the remaining budget.
    pub(crate) lns: bool,
    /// Prefix draw: see `rgreedy::Params::prefix_mode` (same encoding).
    pub(crate) prefix_mode: u8,
}

impl Params {
    pub(crate) const DEFAULT: Params = Params {
        // Same shape that won in the bitset tier (0021-rgreedy line): almost
        // the whole budget in the LNS, wide slack ladder.
        phase_a_num: 1,
        phase_a_den: 16,
        pol_mask: 0b0111_1111,
        lns: true,
        prefix_mode: 2,
    };
}

const POLICIES: [Policy; 7] = [
    Policy { slack: 0 },
    Policy { slack: 1 },
    Policy { slack: 2 },
    Policy { slack: 3 },
    Policy { slack: 5 },
    Policy { slack: 8 },
    Policy { slack: 16 },
];

/// Search for an elimination order with a smaller `Σ c_j²` than the incumbent.
/// Same contract as `rgreedy::search`: `seed`/`seed_flops` are the portfolio's
/// current best (pruning bound + LNS base), `budget` is in deterministic ops,
/// and the caller MUST re-score any returned order through the trusted scorer.
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
    let mut g = SparseGame::new(n, col_ptr, row_idx)?;
    let mut rng = rng_seed | 1;
    let mut best = seed_flops;
    let mut best_ord: Vec<usize> = Vec::new();
    let mut out: Vec<usize> = Vec::with_capacity(n);

    let pols: Vec<Policy> = (0..POLICIES.len())
        .filter(|&i| par.pol_mask & (1 << i) != 0)
        .map(|i| POLICIES[i])
        .collect();
    if pols.is_empty() {
        return None;
    }

    // ── Phase A: from-scratch randomized greedy, cycling the slack ladder ───
    let phase_a_end = if par.lns {
        budget * par.phase_a_num / par.phase_a_den
    } else {
        budget
    };
    let mut it = 0usize;
    while g.ops < phase_a_end {
        let pol = pols[it % pols.len()];
        it += 1;
        if let Some(f) = g.run(&[], pol, &mut rng, best, budget, &mut out) {
            if f < best {
                best = f;
                best_ord = out.clone();
            }
        }
    }

    // ── Phase B: prefix-freezing LNS with sideways (plateau) acceptance ─────
    // Identical acceptance discipline to the bitset tier: the walk may move to
    // any solution with `f <= cur` (plateau drift), the GLOBAL best is updated
    // only on strict improvement, so the returned answer is monotone.
    let start: Vec<usize> = if best_ord.is_empty() {
        seed.to_vec()
    } else {
        best_ord.clone()
    };
    let mut cur: Vec<usize> = start;
    let mut cur_f: u64 = best;
    while par.lns && g.ops < budget {
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
        it += 1;
        let bound = cur_f + 1;
        let taken = std::mem::take(&mut cur);
        let r = g.run(&taken[..p.min(taken.len())], pol, &mut rng, bound, budget, &mut out);
        cur = taken;
        if let Some(f) = r {
            if f < best {
                best = f;
                best_ord = out.clone();
            }
            cur_f = f;
            std::mem::swap(&mut cur, &mut out);
        }
    }

    if best < seed_flops && !best_ord.is_empty() {
        Some((best_ord, best))
    } else {
        None
    }
}
