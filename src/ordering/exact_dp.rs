//! EXACT minimum-flop elimination ordering by subset dynamic programming.
//!
//! For a graph small enough to index by a bitmask, the elimination ordering
//! that minimizes the scored objective `Σ cⱼ²` can be computed EXACTLY rather
//! than heuristically. The whole portfolio elsewhere in this module is a
//! collection of heuristics; this is the one place we can stop guessing.
//!
//! ## Why this is a valid DP
//!
//! In the elimination game, the fill graph obtained after eliminating a SET of
//! vertices does not depend on the order in which that set was eliminated
//! (the classic result underlying every elimination-tree argument). So the
//! state of a partial elimination is fully described by the eliminated SET `S`,
//! and the column count charged when `v` is eliminated out of `S` is
//!
//!   `c(v, S) = 1 + |N_{G_S}(v)|`
//!
//! where `N_{G_S}(v)` is `v`'s neighbourhood in the fill graph on `V \ S`.
//! The objective is additive over eliminations, giving
//!
//!   `f(S ∪ {v}) = min_v [ f(S) + c(v, S)² ]`,  `f(∅) = 0`
//!
//! and `f(V)` is the true minimum of `Σ cⱼ²` over ALL `n!` orderings.
//!
//! ## Computing `c(v, S)` without a per-transition BFS
//!
//! `v`'s fill neighbourhood is its own neighbours outside `S`, plus the
//! outside-neighbours of every connected component of `G[S]` that `v` touches
//! (eliminating a component couples everything on its boundary). The components
//! of `G[S]` depend only on `S`, so they are computed ONCE per state and reused
//! for all `n` candidate pivots — turning an O(n) BFS per transition into a few
//! word operations. Distinct components of `G[S]` are non-adjacent by
//! definition, so no chaining between them is possible.
//!
//! ## Cost
//!
//! `2^n` states × `n` pivots, each a handful of `u32` ops, plus an O(n)
//! component pass per state. Memory is `5 · 2^n` bytes (a `u32` cost table and
//! a `u8` back-pointer). Both are why the caller must gate on `n` — see
//! `EXACT_DP_MAX_N` at the call site. This module never allocates per
//! transition and never recurses.
//!
//! Deterministic by construction: ties are broken by the lowest vertex index,
//! and the state sweep is a plain ascending loop, so two runs on the same
//! pattern return byte-identical output (the harness requires this).

/// Hard structural ceiling: the bitmask is a `u32`, and the back-pointer a
/// `u8`. Beyond this the tables stop fitting in cache and then in memory; the
/// SHIPPED gate is much lower and lives at the call site.
pub const STRUCTURAL_MAX_N: usize = 24;

/// Build the adjacency bitmask table for a symmetric pattern, dropping the
/// diagonal. Returns `None` if `n` exceeds the structural ceiling or the
/// pattern references an out-of-range row.
pub fn adjacency(n: usize, col_ptr: &[usize], row_idx: &[usize]) -> Option<Vec<u32>> {
    if n == 0 || n > STRUCTURAL_MAX_N {
        return None;
    }
    let mut adj = vec![0u32; n];
    for v in 0..n {
        let (lo, hi) = (col_ptr[v], col_ptr[v + 1]);
        for &r in &row_idx[lo..hi] {
            if r >= n {
                return None;
            }
            if r != v {
                adj[v] |= 1u32 << r;
                adj[r] |= 1u32 << v; // symmetrize defensively
            }
        }
    }
    Some(adj)
}

/// The exact minimizer of `Σ cⱼ²`. Returns `perm` with `perm[k]` = the original
/// vertex eliminated `k`-th, and the achieved flop count.
///
/// Returns `None` when `n` is outside the structural range, so the caller can
/// simply fall through to the heuristic portfolio.
pub fn exact_min_flops_order(n: usize, adj: &[u32]) -> Option<(Vec<usize>, u64)> {
    if n == 0 || n > STRUCTURAL_MAX_N || adj.len() < n {
        return None;
    }
    let states: usize = 1usize << n;
    let full: u32 = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };

    let mut f = vec![u32::MAX; states];
    let mut choice = vec![0u8; states];
    f[0] = 0;

    // Component scratch, reused across every state — no per-state allocation.
    let mut comp_mask = [0u32; STRUCTURAL_MAX_N];
    let mut comp_nbr = [0u32; STRUCTURAL_MAX_N];

    for s in 0..states {
        let cur = f[s];
        if cur == u32::MAX {
            continue; // unreachable state
        }
        let smask = s as u32;

        // ── connected components of G[S], with their outside neighbourhoods ──
        let mut ncomp = 0usize;
        let mut rem = smask;
        while rem != 0 {
            let start = rem.trailing_zeros();
            let mut stack = 1u32 << start;
            let mut comp = 0u32;
            let mut nbr = 0u32;
            while stack != 0 {
                let u = stack.trailing_zeros();
                let ub = 1u32 << u;
                stack &= !ub;
                if comp & ub != 0 {
                    continue;
                }
                comp |= ub;
                let a = adj[u as usize];
                nbr |= a;
                stack |= a & smask & !comp;
            }
            rem &= !comp;
            comp_mask[ncomp] = comp;
            comp_nbr[ncomp] = nbr;
            ncomp += 1;
        }

        // ── relax every pivot out of this state ─────────────────────────────
        for v in 0..n {
            let vb = 1u32 << v;
            if smask & vb != 0 {
                continue;
            }
            let av = adj[v];
            let mut reach = av;
            for i in 0..ncomp {
                if av & comp_mask[i] != 0 {
                    reach |= comp_nbr[i];
                }
            }
            // Uneliminated neighbours in the fill graph, plus the diagonal.
            let c = (reach & !smask & !vb).count_ones() + 1;
            let cost = cur + c * c;
            let ns = s | (vb as usize);
            if cost < f[ns] {
                f[ns] = cost;
                choice[ns] = v as u8;
            }
        }
    }

    let goal = full as usize;
    if f[goal] == u32::MAX {
        return None;
    }

    // Walk the back-pointers: `choice[S]` is the vertex eliminated LAST to
    // reach `S`, so this yields the order reversed.
    let mut perm = Vec::with_capacity(n);
    let mut s = full;
    while s != 0 {
        let v = choice[s as usize] as usize;
        perm.push(v);
        s &= !(1u32 << v);
    }
    perm.reverse();
    debug_assert_eq!(perm.len(), n);

    Some((perm, f[goal] as u64))
}
