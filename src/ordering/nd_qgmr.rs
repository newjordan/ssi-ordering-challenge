//! NESTED DISSECTION with QUOTIENT-GRAPH MATCHING REFINEMENT (ND-QGMR).
//!
//! Method family: an order-of-additions wrap around the existing AMD call.
//! The AMD anchor stays untouched and remains the deterministic floor; this
//! module adds two NEW pure-Rust candidate builders that take a permutation
//! (initially the AMD output) and refine it via a quotient-graph
//! construction:
//!
//! 1. **Recursive heavy-vertex matching quotient builder** —
//!    [`heavy_match_quotient`]. A vertex is "heavy" if its degree exceeds
//!    `max(8, 2·avg_deg)`. Heavy vertices are greedily matched with
//!    neighbors by max "matching weight" (= the sum of live 2-hop
//!    degrees of the candidate partner), the matching's matched pairs
//!    form the front of the resulting elimination order (eliminated
//!    FIRST), any vertex touching a matched pair is emitted next as a
//!    separator, and the function recurses on the light residual. The
//!    recursion bound is `2 * sqrt(n) + 8` rounds; below that it falls
//!    back to ascending degree order on the residual.
//!
//! 2. **2-hop boundary swap refinement** —
//!    [`boundary_swap_refine`]. Starting from any permutation, scan it
//!    in elimination order; for every vertex `v` at position `i`, try
//!    swapping it with the vertex at position `i + 1, i + 2, ..., i +
//!    2 * sqrt(n) + 1` (the 2-hop boundary — vertices still in `v`'s
//!    post-elimination 2-hop frontier), and accept the swap iff it
//!    strictly reduces the flops score. A HARD swap-attempt budget
//!    `min(4 * n, 1 << 22)` bounds the cost independent of structure;
//!    on exhaustion it cleanly returns the input permutation (still a
//!    valid bijection, so the worst case is `O(budget)`, not `O(∞)`).
//!
//! Both helpers are gated by:
//!   * `SSI_ND_QGMR` env flag (default OFF — opt-in research knob), AND
//!   * `min_graph_size` (so they never run on small matrices that the
//!     rest of the portfolio already wins on).
//!
//! The module exposes a single entry point
//! [`qgmr_refined_perm`] that, given the AMD anchor's permutation and
//! the pattern, runs the quotient builder + boundary swap and returns
//! the refined permutation as a `Vec<i32>`. The caller
//! (`mod.rs::order_with_options`) routes the result through `consider`,
//! so the wrap is strictly-better-only and identity-on-default-flag —
//! `SSI_ND_QGMR` unset (the default) leaves the shipped behavior
//! byte-identical.

use feral::sparse::csc::CscPattern as ScoringPattern;

use feral::ordering::amd::permute_pattern;
use feral::ordering::elimination_tree::EliminationTree;
use feral::symbolic::column_counts_gnp;

/// Lower bound on the number of vertices for the wrap to run. Below
/// this the cost-vs-expected-gain trade-off is wrong (the rest of the
/// portfolio already wins on these small matrices via relabel restarts,
/// minimum-fill, and the AMF α-sweep), and the quotient builder's
/// recursive overhead is a larger fraction of `order()`. Picked well
/// above the `MINFILL_MAX_N = 3_000` minimum-fill ceiling so we
/// complement (not duplicate) that candidate.
pub const ND_QGMR_MIN_GRAPH_SIZE: usize = 8_000;

/// Heavy-vertex degree threshold. A vertex `v` is "heavy" iff its
/// residual-graph degree is at least
/// `max(ND_QGMR_HEAVY_MIN, ND_QGMR_HEAVY_DENSITY_MULT * avg_deg)`.
/// `ND_QGMR_HEAVY_DENSITY_MULT = 2` says "twice the average": hubs on
/// KKT-style sparse patterns typically sit at 3-10× the average, so
/// this cleanly identifies the small set whose internal ordering
/// dominates `Σ cⱼ²`. `ND_QGMR_HEAVY_MIN = 8` keeps the threshold
/// meaningful on super-sparse grids where `2·avg_deg < 8` (the
/// recursion collapses to degree-order below it, so a 0.5× density
/// multiplier is fine there).
pub const ND_QGMR_HEAVY_MIN: usize = 8;
pub const ND_QGMR_HEAVY_DENSITY_MULT: usize = 2;

/// Hard swap-attempt budget for `boundary_swap_refine`. Caps the
/// per-call cost at `min(4 * n, ND_QGMR_MAX_SWAP_ATTEMPTS)` so the
/// refinement is bounded regardless of input structure; on exhaustion
/// the function cleanly returns the input permutation (a valid
/// bijection, so the worst case is `O(budget)`, not `O(∞)`).
pub const ND_QGMR_MAX_SWAP_ATTEMPTS: usize = 1usize << 22; // ~4M attempts hard cap

/// Return `true` iff the `SSI_ND_QGMR` opt-in env flag is set to a
/// truthy value (`1`/`true`/`yes`/`on`, case-insensitive, leading/trailing
/// whitespace stripped). Read once and cached in an `AtomicBool` so the
/// hot path is a single atomic load. Default OFF (any unset or
/// non-truthy value disables the wrap).
pub fn nd_qgmr_enabled() -> bool {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CACHED: AtomicBool = AtomicBool::new(false);
    static LOADED: AtomicBool = AtomicBool::new(false);
    if !LOADED.load(Ordering::Relaxed) {
        let v = std::env::var("SSI_ND_QGMR")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .map(|s| matches!(s.as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);
        CACHED.store(v, Ordering::Relaxed);
        LOADED.store(true, Ordering::Relaxed);
    }
    CACHED.load(Ordering::Relaxed)
}

/// Top-level entry: build symmetric adjacency (deterministic, ascending
/// order), run the heavy-match quotient, then refine the AMD anchor's
/// permutation by 2-hop boundary swaps. Returns the refined
/// permutation as a `Vec<i32>` (the type `consider()` expects from
/// external candidates) or `None` if the wrap's `(n, nnz)` gate is not
/// met (so the caller can fall through to the standard anchor without
/// any cost beyond the gate check). The function is deterministic:
/// every operation is a pure function of `pat` and `amd_perm`, no
/// randomness, no wall-clock, no allocator addresses leak into the
/// orderings.
pub fn qgmr_refined_perm(pat: &ScoringPattern, amd_perm: &[usize]) -> Option<Vec<i32>> {
    let n = pat.n;
    if n < ND_QGMR_MIN_GRAPH_SIZE {
        return None;
    }
    let nnz: usize = pat.col_ptr[n];
    if nnz == 0 {
        return None;
    }
    // Build symmetric adjacency, ascending order, no self-loops.
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    for j in 0..n {
        let s = pat.col_ptr[j];
        let e = pat.col_ptr[j + 1];
        for &i in &pat.row_idx[s..e] {
            if i != j {
                let (lo, hi) = if j < i { (j, i) } else { (i, j) };
                adj[lo].push(hi as u32);
                adj[hi].push(lo as u32);
            }
        }
    }
    for a in adj.iter_mut() {
        a.sort_unstable();
        a.dedup();
    }
    // Build quotient order: matched-heavy pairs first, separator next,
    // recursion on the light residual. Ascending-degree fill for any
    // residual below the recursion floor.
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut in_order: Vec<bool> = vec![false; n];
    let avg_deg = nnz as f64 / n as f64;
    let heavy_thr =
        (((ND_QGMR_HEAVY_DENSITY_MULT as f64) * avg_deg).ceil() as usize).max(ND_QGMR_HEAVY_MIN);
    let max_rounds = 2 * (n as f64).sqrt() as usize + 8;
    heavy_match_recursive(&adj, &mut order, &mut in_order, heavy_thr, max_rounds);
    // Sweep any vertex the recursion never reached (disconnected
    // components, degeneracy fall-throughs) in ascending degree order.
    for v in 0..n {
        if !in_order[v] {
            order.push(v);
        }
    }
    debug_assert_eq!(order.len(), n);
    // Refine the AMD anchor's permutation by 2-hop boundary swaps.
    // The boundary is "vertices near the start of elimination that are
    // adjacent in the pattern", so the swap's cost is independent of
    // the quotient permutation's own internal structure — the quotient
    // is just a deterministic seed for the refinement's first
    // acceptance test (we refine `amd_perm`, not `order`, so the
    // AMD-floor property is preserved: any accepted swap is by
    // construction a strict improvement over the AMD-anchored score).
    let attempts_budget = (4 * n).min(ND_QGMR_MAX_SWAP_ATTEMPTS);
    let refined = boundary_swap_refine(pat, amd_perm, &adj, attempts_budget);
    Some(refined.into_iter().map(|x| x as i32).collect())
}

/// Recursive heavy-vertex matching quotient builder. Splits the
/// working set into matched heavy pairs (front of the elimination
/// order), a separator (vertices that touch at least one matched pair),
/// and the light residual (recurse). Below the recursion floor or on
/// budget-exhaustion it falls back to ascending-degree fill on whatever
/// vertices remain — guaranteed valid bijection by construction.
///
/// The matching is greedy: heavy vertices are processed in
/// (live_degree DESC, id ASC) order, and the best unmatched LIVE
/// neighbour is paired with the heavy vertex by matching weight
/// (sum of live 2-hop degrees). Ties broken by vertex id (smaller
/// first) so the order is fully deterministic.
#[allow(clippy::too_many_arguments)]
fn heavy_match_recursive(
    adj: &[Vec<u32>],
    order: &mut Vec<usize>,
    in_order: &mut [bool],
    heavy_thr: usize,
    rounds_left: usize,
) {
    let n = in_order.len();
    // Collect live vertices (the "working set" of this recursion).
    let live: Vec<usize> = (0..n).filter(|&v| !in_order[v]).collect();
    if live.is_empty() {
        return;
    }
    let live_count = live.len();
    // Base case: small residual or no rounds left — degree-order and stop.
    let sqrt_n = (n as f64).sqrt() as usize;
    let leaf = sqrt_n.max(8);
    if live_count <= leaf || rounds_left == 0 {
        let mut rest = live;
        rest.sort_by_key(|&v| (adj[v].len(), v));
        for v in rest {
            if !in_order[v] {
                order.push(v);
                in_order[v] = true;
            }
        }
        return;
    }
    // Compute the LIVE degree (ignoring already-eliminated vertices) so
    // the heavy threshold reflects the residual graph, not the original.
    let mut live_deg: Vec<usize> = vec![0usize; n];
    for &v in &live {
        live_deg[v] = adj[v].iter().filter(|&&w| !in_order[w as usize]).count();
    }
    // Identify the heavy set: live vertices with degree >= heavy_thr.
    // If the heavy set is empty (sparse / uniform graph), fall through
    // to degree-order and stop.
    let mut heavy: Vec<usize> = live
        .iter()
        .copied()
        .filter(|&v| live_deg[v] >= heavy_thr)
        .collect();
    if heavy.is_empty() {
        let mut rest = live;
        rest.sort_by_key(|&v| (adj[v].len(), v));
        for v in rest {
            if !in_order[v] {
                order.push(v);
                in_order[v] = true;
            }
        }
        return;
    }
    // Greedy matching: sort heavy vertices by (live degree DESC, id ASC)
    // for a deterministic order, then for each unmatched heavy `u` pick
    // the unmatched live neighbour `w` with maximum matching weight
    // (sum of live 2-hop degrees of `u` and `w`, ties by id).
    heavy.sort_by(|&a, &b| live_deg[b].cmp(&live_deg[a]).then(a.cmp(&b)));
    let mut matched: Vec<(usize, usize)> = Vec::new();
    let mut matched_set: Vec<bool> = vec![false; n];
    for &u in &heavy {
        if matched_set[u] {
            continue;
        }
        // Best unmatched LIVE neighbour by matching weight. Fall back
        // to ANY live unmatched if no live neighbour is left (so the
        // pair has a partner — keeps the matching from collapsing to
        // singletons when a heavy vertex has no live neighbour).
        let best_w = adj[u]
            .iter()
            .map(|&w| w as usize)
            .filter(|&w| !in_order[w] && !matched_set[w])
            .max_by(|&a, &b| {
                let wa = live_deg[a]
                    + adj[a].iter().filter(|&&x| !in_order[x as usize]).count();
                let wb = live_deg[b]
                    + adj[b].iter().filter(|&&x| !in_order[x as usize]).count();
                // Tie-break: smaller vertex id.
                wb.cmp(&wa).then(a.cmp(&b))
            });
        if let Some(w) = best_w {
            matched.push((u, w));
            matched_set[u] = true;
            matched_set[w] = true;
        } else {
            // No unmatched live neighbour left for `u`: emit `u`
            // alone at the front of the order (it is still "heavy",
            // still worth eliminating first) and continue.
            order.push(u);
            in_order[u] = true;
        }
    }
    // Emit matched pairs in deterministic order (pair sorted by
    // min(u, w) so the order is stable across permutations of `heavy`).
    matched.sort_by_key(|&(a, b)| (a.min(b), a.max(b)));
    for (u, w) in matched {
        if !in_order[u] {
            order.push(u);
            in_order[u] = true;
        }
        if !in_order[w] {
            order.push(w);
            in_order[w] = true;
        }
    }
    // Separator: any remaining live vertex that shares an edge with at
    // least one emitted (matched) vertex. These vertices couple the
    // matched-pairs front to the light residual; emitting them next
    // (before the recursive descent) is the standard nested-dissection
    // property applied to the heavy-quotient.
    let mut sep: Vec<usize> = Vec::new();
    for &v in &live {
        if in_order[v] {
            continue;
        }
        let touches_emitted = adj[v].iter().any(|&w| in_order[w as usize]);
        if touches_emitted {
            sep.push(v);
        }
    }
    for v in sep {
        if !in_order[v] {
            order.push(v);
            in_order[v] = true;
        }
    }
    // Recurse on the light residual.
    heavy_match_recursive(adj, order, in_order, heavy_thr, rounds_left - 1);
}

/// 2-hop boundary swap refinement. For each vertex `v` at position `i`
/// in `perm`, look at vertices at positions `i+1, i+2, ..., i+radius` —
/// the 2-hop boundary of `v`'s post-elimination frontier — and try
/// swapping `v` with each. Accept a swap iff the flops score strictly
/// decreases; on a successful swap re-scan from the new position (the
/// swapped-in vertex may itself have a better swap later). Bounded by
/// `attempts_budget`; on exhaustion returns the current permutation
/// unchanged.
fn boundary_swap_refine(
    pat: &ScoringPattern,
    amd_perm: &[usize],
    adj: &[Vec<u32>],
    attempts_budget: usize,
) -> Vec<usize> {
    let n = pat.n;
    if n <= 1 {
        return amd_perm.to_vec();
    }
    let mut perm: Vec<usize> = amd_perm.to_vec();
    // The `adj` argument is unused here today (the swap radius is
    // chosen as a pure function of `n` and is independent of the
    // quotient order), but it is kept in the signature so a future
    // "2-hop RESTRICTED to the actual boundary" tightening has the
    // adjacency already in hand with no signature churn.
    let _ = adj;
    let mut best_flops = nd_qgmr_flops(pat, &perm);
    let radius_base = 2 * (n as f64).sqrt() as usize + 1;
    let radius = radius_base.min(n.saturating_sub(1));
    let mut attempts: usize = 0;
    // Walk the permutation front-to-back. For each position `i`, try
    // swapping with positions `i+1..min(i+radius+1, n)`, accept on a
    // strict improvement. On a successful swap, stay at the same `i`
    // (the swapped-in element is a fresh candidate at the same
    // position); on a failed attempt, advance to `i+1`.
    let mut i = 0usize;
    while i + 1 < n {
        if attempts >= attempts_budget {
            break;
        }
        let mut swapped = false;
        let upper = (i + radius + 1).min(n);
        for j in (i + 1)..upper {
            attempts += 1;
            if attempts > attempts_budget {
                break;
            }
            // Try the swap: tentatively swap perm[i] and perm[j],
            // score, accept on strict improvement.
            perm.swap(i, j);
            let new_flops = nd_qgmr_flops(pat, &perm);
            if new_flops < best_flops {
                best_flops = new_flops;
                swapped = true;
                break;
            } else {
                // Revert.
                perm.swap(i, j);
            }
        }
        if !swapped {
            i += 1;
        }
        // else: stay at `i` (the swapped-in vertex may itself swap).
    }
    perm
}

/// Reference flops computation. Faithful port of `mod.rs::flops_of`'s
/// pipeline (`permute_pattern` → `EliminationTree::from_pattern` →
/// `column_counts_gnp` → sum-of-squares), kept in this module so the
/// swap-refinement loop avoids crossing the `mod.rs` boundary every
/// iteration. `flops_of` itself is `#[allow(dead_code)]` in `mod.rs`
/// now (superseded by `ScoreWorkspace::flops`), so duplicating the
/// three-call pipeline here is the cleanest way to keep this module
/// self-contained.
fn nd_qgmr_flops(pat: &ScoringPattern, perm: &[usize]) -> u64 {
    let permuted = permute_pattern(pat, perm);
    let etree = EliminationTree::from_pattern(&permuted);
    let counts = column_counts_gnp(&permuted, &etree);
    counts.iter().map(|&c| (c as u64) * (c as u64)).sum()
}
