//! List-based exact elimination game for the 3k–10k sparse band.
//!
//! Mechanism ported from the public submission notes (tekkac, note 2669ba8,
//! "sparse list-based elimination game on the 3k–10k band"): the bitset
//! [`super::rgreedy::Game`] pays `(deg+1)·3⌈n/64⌉` WORDS per neighbour per
//! pivot — at n = 3k–10k that is 47–157 words where the matrix's actual
//! degrees are single digits. This module keeps the fill graph as SORTED
//! adjacency lists and cliques N(v) via two-pointer merges, so a pivot costs
//! `deg(u)+deg(v)` ELEMENTS — an order of magnitude less work at this band —
//! which is exactly what makes prefix-replay LNS affordable above n = 3000.
//!
//! Same discipline as `rgreedy`: strictly-improving candidates only, the
//! result is re-scored through the trusted scorer by the CALLER, the budget
//! is a deterministic op count (never wall-clock) and is enforced MID-RUN —
//! per pivot and per prefix step, not only between kicks, because one kick's
//! replay can cost several times the whole budget (the measured trap from
//! the same note).

use super::rgreedy::{below, Policy};

/// Band ceiling. `n ≤ 12_000` matches [`super::rgreedy::MAX_N`].
const SG_MAX_N: usize = 12_000;
/// Hard cap on ops consumed by ONE run (from-scratch or LNS kick incl.
/// replay). Abandoning a runaway run costs only that run's work; the phase
/// stays strictly-improving because the incumbent is never replaced except
/// on a strict win re-scored by the caller.
const SG_RUN_CAP: i64 = 40_000_000;

/// Sorted adjacency lists for the CURRENT fill graph, plus a pristine copy.
pub(crate) struct ListGame {
    n: usize,
    adj0: Vec<Vec<u32>>,
    adj: Vec<Vec<u32>>,
    deg: Vec<u32>,
    live: Vec<u32>,
    pos: Vec<u32>,
    scratch: Vec<u32>,
    /// Cost of the current LNS walk point (sideways acceptance bound).
    last_cost: u64,
    /// Deterministic work counter, element operations. The ONLY budget
    /// signal — no wall-clock anywhere in this module.
    pub(crate) ops: i64,
}

impl ListGame {
    /// Build the pristine adjacency from the pattern. `None` if out of range
    /// or a row index is out of range (caller then simply skips the phase).
    pub(crate) fn build(n: usize, col_ptr: &[usize], row_idx: &[usize]) -> Option<ListGame> {
        if n == 0 || n > SG_MAX_N {
            return None;
        }
        let mut adj0: Vec<Vec<u32>> = vec![Vec::new(); n];
        for v in 0..n {
            for &r in &row_idx[col_ptr[v]..col_ptr[v + 1]] {
                if r >= n {
                    return None;
                }
                if r != v {
                    adj0[v].push(r as u32);
                    adj0[r].push(v as u32);
                }
            }
        }
        for a in adj0.iter_mut() {
            a.sort_unstable();
            a.dedup();
        }
        Some(ListGame {
            n,
            scratch: Vec::with_capacity(64),
            adj0,
            adj: Vec::new(),
            deg: vec![0; n],
            live: Vec::with_capacity(n),
            pos: vec![0; n],
            ops: 0,
            last_cost: u64::MAX,
        })
    }

    fn reset(&mut self) {
        self.adj.clone_from(&self.adj0);
        self.live.clear();
        for v in 0..self.n {
            let d = self.adj[v].len() as u32;
            self.deg[v] = d;
            self.pos[v] = self.live.len() as u32;
            self.live.push(v as u32);
        }
        self.ops += 3 * self.n as i64;
    }

    #[inline]
    fn live_remove(&mut self, v: usize) {
        let p = self.pos[v] as usize;
        let last = *self.live.last().unwrap() as usize;
        self.live[p] = last as u32;
        self.pos[last] = p as u32;
        self.live.pop();
    }

    /// Merge sorted `b` into sorted `a`, dropping `drop1`/`drop2`, result
    /// sorted + deduped in `out`. Op cost = len(a)+len(b) element steps.
    fn merge_without(a: &[u32], b: &[u32], drop1: u32, drop2: u32, out: &mut Vec<u32>) {
        out.clear();
        let mut i = 0;
        let mut j = 0;
        while i < a.len() && j < b.len() {
            let x = a[i];
            let y = b[j];
            if x < y {
                if x != drop1 && x != drop2 {
                    out.push(x);
                }
                i += 1;
            } else if y < x {
                if y != drop1 && y != drop2 {
                    out.push(y);
                }
                j += 1;
            } else {
                if x != drop1 && x != drop2 {
                    out.push(x);
                }
                i += 1;
                j += 1;
            }
        }
        while i < a.len() {
            let x = a[i];
            if x != drop1 && x != drop2 {
                out.push(x);
            }
            i += 1;
        }
        while j < b.len() {
            let y = b[j];
            if y != drop1 && y != drop2 {
                out.push(y);
            }
            j += 1;
        }
    }

    /// Eliminate `v`: clique N(v) into every neighbour, remove v. Returns the
    /// column count `c = 1 + |N(v)|`.
    fn eliminate(&mut self, v: usize) -> u64 {
        let nv: Vec<u32> = self.adj[v].clone();
        let c = nv.len() as u64 + 1;
        self.live_remove(v);
        self.ops += (nv.len() as i64) * 2 + 8;
        for &u in nv.iter() {
            let u = u as usize;
            let a = std::mem::take(&mut self.adj[u]);
            let alen = a.len();
            Self::merge_without(&a, &nv, u as u32, v as u32, &mut self.scratch);
            let ndlen = self.scratch.len();
            let nd = std::mem::replace(&mut self.scratch, a);
            self.adj[u] = nd;
            self.deg[u] = ndlen as u32;
            self.ops += (alen + nv.len()) as i64 + 8;
        }
        self.adj[v].clear();
        c
    }

    /// Deficiency of `v` (fill created): count of pairs (u,w) ⊂ N(v) with
    /// w ∉ N(u). Two binary searches per pair — sorted lists make this exact
    /// and cheap at these degrees.
    fn deficiency(&mut self, v: usize) -> u32 {
        let nv = self.adj[v].clone();
        let mut missing: u64 = 0;
        for (i, &u) in nv.iter().enumerate() {
            let au = self.adj[u as usize].clone();
            self.ops += (au.len() + nv.len()) as i64;
            for (j, &w) in nv.iter().enumerate() {
                if i != j && au.binary_search(&w).is_err() {
                    missing += 1;
                }
            }
        }
        self.ops += (nv.len().saturating_sub(1) as i64) * (nv.len() as i64);
        (missing / 2) as u32
    }

    /// One randomized greedy run; `fixed` = LNS prefix replayed verbatim.
    /// Returns `None` once the objective reaches `bound` (Σc² only grows) or
    /// ops pass [`SG_RUN_CAP`].
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
            if f >= bound || self.ops > SG_RUN_CAP {
                return None;
            }
        }
        while !self.live.is_empty() {
            if self.ops > SG_RUN_CAP {
                return None;
            }
            let m = self.live.len();
            self.ops += 2 * m as i64;
            let mut d0 = u32::MAX;
            for &u in self.live.iter() {
                let d = self.deg[u as usize];
                if d < d0 {
                    d0 = d;
                }
            }
            let cut = d0.saturating_add(pol.slack);
            let mut cand: Vec<u32> = Vec::new();
            for &u in self.live.iter() {
                if self.deg[u as usize] <= cut {
                    cand.push(u);
                    if cand.len() >= 64 {
                        break;
                    }
                }
            }
            let pick = if cand.is_empty() {
                self.live[0] as usize
            } else if pol.fill_tb {
                // Deficiency tiebreak over the candidate list.
                let mut bestu = cand[0];
                let mut bestd = u32::MAX;
                for &u in cand.iter() {
                    let d = self.deficiency(u as usize);
                    if d < bestd {
                        bestd = d;
                        bestu = u;
                    }
                }
                bestu as usize
            } else {
                cand[below(rng, cand.len() as u32) as usize] as usize
            };
            let c = self.eliminate(pick);
            f += c * c;
            out.push(pick);
            if f >= bound {
                return None;
            }
        }
        Some(f)
    }

    /// The list-game search shell: phase-A from-scratch restarts on 1/16 of
    /// the budget, then prefix-freezing LNS with sideways acceptance and a
    /// mixed uniform / log-uniform tail draw. Mirrors `rgreedy::search_with`'s
    /// measured shipped parameter set (phase_a 1/16, 8-policy ladder,
    /// sideways-only acceptance, single walk).
    pub(crate) fn search(
        &mut self,
        seed: &[usize],
        seed_flops: u64,
        budget: i64,
        mut rng: u64,
    ) -> Option<(Vec<usize>, u64)> {
        const POLICIES: [Policy; 8] = [
            Policy { slack: 0, fill_tb: false },
            Policy { slack: 1, fill_tb: false },
            Policy { slack: 2, fill_tb: false },
            Policy { slack: 4, fill_tb: false },
            Policy { slack: 8, fill_tb: false },
            Policy { slack: 1, fill_tb: true },
            Policy { slack: 2, fill_tb: true },
            Policy { slack: 4, fill_tb: true },
        ];
        rng |= 1;
        let mut best = seed_flops;
        let mut best_ord: Vec<usize> = Vec::new();
        let mut out: Vec<usize> = Vec::with_capacity(self.n);
        // ── phase A ────────────────────────────────────────────────────────
        let a_end = budget / 16;
        let mut it = 0usize;
        while self.ops < a_end {
            let pol = POLICIES[it % POLICIES.len()];
            it += 1;
            if let Some(f) = self.run(&[], pol, &mut rng, best, &mut out) {
                if f < best {
                    best = f;
                    best_ord = out.clone();
                }
            }
        }
        // ── LNS ────────────────────────────────────────────────────────────
        let mut cur: Vec<usize> = if best_ord.is_empty() {
            seed.to_vec()
        } else {
            best_ord.clone()
        };
        self.last_cost = best;
        let mut it2 = 0usize;
        while self.ops < budget {
            let ne = cur.len();
            let p = if it2 % 2 == 0 {
                below(&mut rng, ne as u32) as usize
            } else {
                // Log-uniform tail length: pick an exponent, then a length
                // inside it. Long tails stay reachable, short ones dominate.
                let bits = usize::BITS - (ne as u32).leading_zeros();
                let e = below(&mut rng, bits) as usize;
                let k = 1 + below(&mut rng, 1u32 << e.min(30)) as usize;
                ne.saturating_sub(k.min(ne))
            };
            let pol = POLICIES[it2 % POLICIES.len()];
            it2 += 1;
            if let Some(f) = self.run(&cur[..p.min(cur.len())], pol, &mut rng, u64::MAX, &mut out) {
                if f < best {
                    best = f;
                    best_ord = out.clone();
                }
                // Sideways/threshold acceptance of the WALK point: any
                // completed run ≤ the current point's cost becomes the next
                // prefix source. The global best is only the strict-min
                // track, so the returned answer stays monotone.
                if f <= self.last_cost {
                    self.last_cost = f;
                    cur = out.clone();
                }
            }
        }
        if best < seed_flops {
            Some((best_ord, best))
        } else {
            None
        }
    }
}
