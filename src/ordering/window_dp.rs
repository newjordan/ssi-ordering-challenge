//! TERMINAL WINDOW SUBSET-DP DESCENT — production port of the validated
//! `probe_window_dp` reference (end of `probe.rs`, session
//! 20260829T1145Z-n1-push).
//!
//! Mechanism: replay the elimination game on a live hash-set adjacency, and
//! for every full window of `k` consecutive positions solve the EXACT
//! minimum-`Σ (1 + live fill degree)²` ordering of those k pivots by subset
//! DP over the window, holding the outside prefix/suffix fixed. The pivot
//! charge uses the `exact_dp.rs` component-boundary formulation widened from
//! u32 masks to word bitsets over `U = B ∪ N_live(B)`, so the charge is the
//! FULL live degree, not an approximation. A window reorder is committed only
//! on STRICT local cost reduction; two tilings (offsets 0 and k/2) cover
//! window-boundary pairs. Tie-break is deterministic and lexicographic:
//! pivots are relaxed in ascending global vertex id with strict `<` updates,
//! so among equal-cost transitions the smallest id wins.
//!
//! Determinism: `HashSet`/`HashMap` iteration order is nondeterministic but
//! only ever feeds set-valued results (bitsets, sorted vecs), exactly as in
//! the probe, so the returned permutation is a pure function of the pattern
//! and seed. The ops budget below is charged in deterministic word-op counts
//! (never wall clock), so early exit is also a pure function of the inputs.

use std::collections::{HashMap, HashSet};

/// Charge `amount` word-ops against the budget; `false` once exhausted.
#[inline]
fn charge(budget: &mut i64, amount: i64) -> bool {
    *budget -= amount;
    *budget >= 0
}

/// Build the live adjacency from CSC, symmetrized, self-loops dropped.
/// Out-of-range row indices are skipped defensively (the vendor pattern is
/// trusted elsewhere, but this phase must not panic).
fn build_live(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    budget: &mut i64,
) -> Option<Vec<HashSet<u32>>> {
    let mut g: Vec<HashSet<u32>> = vec![HashSet::new(); n];
    if !charge(budget, n as i64) {
        return None;
    }
    for v in 0..n {
        let lo = *col_ptr.get(v)?;
        let hi = *col_ptr.get(v + 1)?;
        if lo > hi || hi > row_idx.len() {
            return None;
        }
        if !charge(budget, 2 * (hi - lo) as i64) {
            return None;
        }
        for &r in &row_idx[lo..hi] {
            if r < n && r != v {
                g[v].insert(r as u32);
                g[r].insert(v as u32);
            }
        }
    }
    Some(g)
}

/// Eliminate pivot `v`: remove it from its neighbours and clique them.
fn eliminate(g: &mut [HashSet<u32>], v: usize, budget: &mut i64) -> bool {
    if v >= g.len() {
        return false;
    }
    let nbrs: Vec<u32> = g[v].iter().copied().collect();
    let d = nbrs.len() as i64;
    if !charge(budget, d + d * d / 2) {
        return false;
    }
    for &u in &nbrs {
        if let Some(su) = g.get_mut(u as usize) {
            su.remove(&(v as u32));
        }
    }
    g[v].clear();
    for i in 0..nbrs.len() {
        for j in (i + 1)..nbrs.len() {
            let (a, b) = (nbrs[i] as usize, nbrs[j] as usize);
            if a < g.len() && b < g.len() {
                g[a].insert(nbrs[j]);
                g[b].insert(nbrs[i]);
            }
        }
    }
    true
}

/// One tiling pass at `offset`: DP-optimize every full k-window, committing
/// only strict local improvements. Returns `false` once the budget is
/// exhausted (the perm then holds every window committed so far — commits
/// are atomic per window, so the state is always fully committed).
#[allow(clippy::too_many_arguments)]
fn sweep(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    perm: &mut [usize],
    k: usize,
    offset: usize,
    max_u: usize,
    budget: &mut i64,
    committed: &mut usize,
) -> bool {
    let mut g = match build_live(n, col_ptr, row_idx, budget) {
        Some(g) => g,
        None => return false,
    };
    for i in 0..offset.min(n) {
        if !eliminate(&mut g, perm[i], budget) {
            return false;
        }
    }
    let states = 1usize << k;
    let mut p = offset;
    while p + k <= n {
        let b: Vec<usize> = perm[p..p + k].to_vec();
        if b.iter().any(|&v| v >= n) {
            return false;
        }
        // U = B ∪ N_live(B), compressed to ascending local ids.
        let mut u: Vec<u32> = b.iter().map(|&x| x as u32).collect();
        let mut deg_sum = 0usize;
        for &v in &b {
            deg_sum += g[v].len();
            u.extend(g[v].iter().copied());
        }
        u.sort_unstable();
        u.dedup();
        if !charge(budget, (deg_sum + u.len()) as i64) {
            return false;
        }
        if u.len() > max_u {
            for i in p..p + k {
                if !eliminate(&mut g, perm[i], budget) {
                    return false;
                }
            }
            p += k;
            continue;
        }
        let words = (u.len() + 63) / 64;
        // Everything below (setup + seed cost + full subset DP) is charged
        // up front; if the remainder cannot cover this window, stop between
        // windows with the perm fully committed so far.
        let win_cost =
            (k * words + k * k) as i64 + (k * k * words) as i64 + (states * k * 2 * words) as i64;
        if !charge(budget, win_cost) {
            return false;
        }
        let lid: HashMap<u32, usize> = u.iter().enumerate().map(|(i, &x)| (x, i)).collect();
        // Per window vertex: U-bitset of live neighbours, k-bit mask of
        // window-internal adjacency, and its own bit position in U.
        let mut adjw = vec![vec![0u64; words]; k];
        let mut badj = vec![0u32; k];
        let mut bpos = vec![0usize; k];
        let mut ok = true;
        for (j, &v) in b.iter().enumerate() {
            match lid.get(&(v as u32)) {
                Some(&q) => bpos[j] = q,
                None => {
                    ok = false;
                    break;
                }
            }
            for &w in &g[v] {
                match lid.get(&w) {
                    Some(&q) => adjw[j][q / 64] |= 1u64 << (q % 64),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                break;
            }
        }
        if !ok {
            // Unreachable by construction; skip the window defensively.
            for i in p..p + k {
                if !eliminate(&mut g, perm[i], budget) {
                    return false;
                }
            }
            p += k;
            continue;
        }
        for j in 0..k {
            for j2 in 0..k {
                if j != j2 && g[b[j]].contains(&(b[j2] as u32)) {
                    badj[j] |= 1u32 << j2;
                }
            }
        }
        // Connected components of G[S] among the window vertices, with
        // their U-bitset boundaries — the exact_dp.rs component-boundary
        // formulation, widened from u32 masks to word bitsets over U.
        let comps_of = |smask: u32| -> Vec<(u32, Vec<u64>)> {
            let mut out = Vec::new();
            let mut rem = smask;
            while rem != 0 {
                let start = rem.trailing_zeros();
                let mut stack = 1u32 << start;
                let mut comp = 0u32;
                let mut nbr = vec![0u64; words];
                while stack != 0 {
                    let j = stack.trailing_zeros() as usize;
                    let jb = 1u32 << j;
                    stack &= !jb;
                    if comp & jb != 0 {
                        continue;
                    }
                    comp |= jb;
                    for w in 0..words {
                        nbr[w] |= adjw[j][w];
                    }
                    stack |= badj[j] & smask & !comp;
                }
                rem &= !comp;
                out.push((comp, nbr));
            }
            out
        };
        // Exact pivot charge (1 + live fill degree)² for eliminating window
        // vertex j out of eliminated window subset smask. All fill
        // neighbours of j after eliminating S ⊆ B lie inside U, so the
        // U-bitsets carry the FULL live degree, not an approximation.
        let trans = |smask: u32, j: usize, comps: &[(u32, Vec<u64>)]| -> u64 {
            let mut reach = adjw[j].clone();
            for (cm, cn) in comps {
                if badj[j] & cm != 0 {
                    for w in 0..words {
                        reach[w] |= cn[w];
                    }
                }
            }
            // Drop eliminated window vertices and the pivot itself.
            let mut dead = smask | (1u32 << j);
            while dead != 0 {
                let j2 = dead.trailing_zeros() as usize;
                dead &= dead - 1;
                reach[bpos[j2] / 64] &= !(1u64 << (bpos[j2] % 64));
            }
            let deg: u32 = reach.iter().map(|w| w.count_ones()).sum();
            let c = (deg + 1) as u64;
            c * c
        };
        // Exact cost of the seed order inside this window.
        let mut seed_cost = 0u64;
        {
            let mut smask = 0u32;
            for j in 0..k {
                let comps = comps_of(smask);
                seed_cost = seed_cost.saturating_add(trans(smask, j, &comps));
                smask |= 1u32 << j;
            }
        }
        // Subset DP over the window; ascending-global-id relaxation with
        // strict `<` gives the lexicographic tie-break.
        let mut f = vec![u64::MAX; states];
        let mut choice = vec![0u8; states];
        f[0] = 0;
        let mut order_js: Vec<usize> = (0..k).collect();
        order_js.sort_by_key(|&j| b[j]);
        for s in 0..states {
            if f[s] == u64::MAX {
                continue;
            }
            let smask = s as u32;
            let comps = comps_of(smask);
            for &j in &order_js {
                if smask & (1u32 << j) != 0 {
                    continue;
                }
                let cost = f[s].saturating_add(trans(smask, j, &comps));
                let ns = s | (1usize << j);
                if cost < f[ns] {
                    f[ns] = cost;
                    choice[ns] = j as u8;
                }
            }
        }
        if f[states - 1] < seed_cost {
            let mut rev = Vec::with_capacity(k);
            let mut s = states - 1;
            let mut steps = 0usize;
            while s != 0 && steps <= k {
                let j = choice[s] as usize;
                if j >= k || s & (1usize << j) == 0 {
                    break;
                }
                rev.push(b[j]);
                s &= !(1usize << j);
                steps += 1;
            }
            if s == 0 && rev.len() == k {
                rev.reverse();
                perm[p..p + k].copy_from_slice(&rev);
                *committed += 1;
            }
        }
        // Advance the game past the window (any order gives the same
        // residual; use the committed one).
        for i in p..p + k {
            if !eliminate(&mut g, perm[i], budget) {
                return false;
            }
        }
        p += k;
    }
    true
}

/// Two-tiling window subset-DP descent over `seed`. Returns `Some(perm)`
/// only when at least one window committed a strict local improvement
/// (the caller re-scores through the canonical `ScoreWorkspace` and accepts
/// strictly); `None` when nothing committed, on invalid input, or when the
/// deterministic word-op `budget` is exhausted before any commit. On
/// mid-run exhaustion the perm returned holds every window committed so far
/// — commits are atomic per window, so it is always a fully-committed state.
/// No wall-clock reads anywhere; the result is a pure function of the
/// pattern and seed.
pub(crate) fn window_dp_descent(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    seed: &[usize],
    k: usize,
    max_u: usize,
    budget: i64,
) -> Option<Vec<usize>> {
    let k = k.clamp(2, 16);
    if n < k || seed.len() != n || col_ptr.len() != n + 1 || max_u == 0 || budget <= 0 {
        return None;
    }
    let mut seen = vec![false; n];
    for &v in seed {
        if v >= n || seen[v] {
            return None;
        }
        seen[v] = true;
    }
    let mut budget = budget;
    let mut perm = seed.to_vec();
    let mut committed = 0usize;
    for &off in &[0usize, k / 2] {
        if !sweep(
            n,
            col_ptr,
            row_idx,
            &mut perm,
            k,
            off,
            max_u,
            &mut budget,
            &mut committed,
        ) {
            break;
        }
    }
    if committed == 0 {
        None
    } else {
        Some(perm)
    }
}
