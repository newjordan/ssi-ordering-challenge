//! Reusable primitive: build a NESTED SEQUENCE of vertex-induced subgraphs by
//! incrementally restoring vertices, instead of rebuilding each subgraph from
//! the full pattern.
//!
//! Motivating case (the "strip-k" candidate family, being integrated
//! separately — see `.../sbx-integrate`): remove the `k` densest vertices
//! from the pattern by some ranking, run AMD on the induced subgraph of the
//! rest, then append the removed vertices last in elimination order —
//! repeated for several `k` (e.g. 6, 24, 48, 64, 96). Because every removed
//! set is a PREFIX of the same descending ranking, the removed sets nest:
//! `removed(k1) ⊆ removed(k2)` whenever `k1 <= k2`, so the KEPT (induced)
//! sets nest the other way: `kept(k_max) ⊆ kept(k_{...}) ⊆ ... ⊆ kept(k_min)`.
//!
//! [`NestedSubgraphBuilder`] builds `kept(k_max)` — the SMALLEST subgraph,
//! excluding the MOST vertices — directly from the pattern in O(nnz) via
//! [`NestedSubgraphBuilder::new`]. Every subsequent, SMALLER `k` RESTORES
//! exactly the vertices no longer excluded (`ranking[k..k_prev]`) via
//! [`NestedSubgraphBuilder::advance_to`], touching only their incident edges
//! against the CURRENTLY kept set. Every vertex is restored AT MOST ONCE
//! across the whole sequence (once marked kept, it stays kept for every
//! later, smaller-k call), so the total incremental work across `m` steps is
//! bounded by the edges touching `ranking[0..k_max]` ONCE — not by
//! `m` independent O(nnz) rebuilds. For the shipped strip-k schedule
//! (k_max = 96, five steps down to k = 6) that is roughly a 6× reduction in
//! the subgraph-build work, though the exact ratio depends on how much of
//! nnz sits on the densest ~96 vertices.
//!
//! ## Why restoring in any within-batch order is still correct
//! When a batch `ranking[k..k_prev]` contains an edge between two vertices
//! BOTH being restored in that same batch, whichever endpoint is processed
//! LAST (in iteration order) is the one that actually records the edge: at
//! the time the FIRST endpoint is processed, the second is still marked
//! removed, so the neighbor scan skips it; by the time the SECOND endpoint is
//! processed, the first has already been marked kept, so the scan records the
//! edge (into BOTH adjacency lists at once). Net effect: every edge between
//! two eventually-kept vertices is recorded exactly once, regardless of
//! within-batch order — proven by the cross-check tests below, which compare
//! against an independent from-scratch rebuild for many `(pattern, ranking,
//! k-schedule)` combinations.
//!
//! ## Output convention
//! Each subgraph is emitted as an i32 CSC pattern in LOCAL vertex numbering
//! (kept vertices renumbered 0..m in ASCENDING GLOBAL id order — the natural
//! choice, and the one a straightforward from-scratch filter would also
//! produce), ready for `feral_ordering_core::CscPattern::new`, plus
//! `local_to_global` to translate an ordering on the subgraph back to the
//! full matrix's vertex numbers. Within each local column, row indices are
//! sorted ascending (mirrors `Pattern`'s own invariant and vendor
//! `permute_pattern`'s sorted-output convention), so incremental and
//! from-scratch construction are BYTE-IDENTICAL, not just equal as sets —
//! required for this to be a drop-in, provably output-neutral replacement
//! wherever the strip-k integration builds these subgraphs today.

use crate::Pattern;

/// One induced subgraph in the nested sequence.
#[allow(dead_code)]
pub(crate) struct InducedSubgraph {
    /// CSC column pointer, local numbering, length `local_to_global.len()+1`.
    pub col_ptr: Vec<i32>,
    /// CSC row indices, local numbering, sorted ascending within each column.
    pub row_idx: Vec<i32>,
    /// `local_to_global[i]` = original vertex id of local vertex `i`, in
    /// ascending original-id order.
    pub local_to_global: Vec<usize>,
}

/// Rank all `n` vertices by descending degree (ties broken by ascending
/// index, for a deterministic total order) — the ranking the strip-k family
/// uses for its degree-based candidates. `ranking[0]` is the single densest
/// vertex. A separate ranking (e.g. by L-column-count under some ordering)
/// can be supplied directly to [`NestedSubgraphBuilder`] instead; the builder
/// itself is ranking-agnostic.
#[allow(dead_code)]
pub(crate) fn degree_ranking(pattern: &Pattern) -> Vec<usize> {
    let n = pattern.n;
    let mut ranking: Vec<usize> = (0..n).collect();
    let degree = |v: usize| pattern.col_ptr[v + 1] - pattern.col_ptr[v];
    ranking.sort_unstable_by(|&a, &b| degree(b).cmp(&degree(a)).then(a.cmp(&b)));
    ranking
}

/// Incremental builder over a nested sequence of induced subgraphs. See the
/// module doc for the algorithm and the nesting requirement on the `k`
/// sequence passed to [`advance_to`](Self::advance_to).
#[allow(dead_code)]
pub(crate) struct NestedSubgraphBuilder<'a> {
    pattern: &'a Pattern,
    n: usize,
    /// `removed[v]` = true iff `v` is currently EXCLUDED from the kept set.
    removed: Vec<bool>,
    /// Adjacency among CURRENTLY KEPT vertices only, in GLOBAL vertex ids.
    adj: Vec<Vec<usize>>,
}

#[allow(dead_code)]
impl<'a> NestedSubgraphBuilder<'a> {
    /// Start at the SMALLEST subgraph in the sequence: exclude
    /// `ranking[0..k_max]` (the `k_max` highest-ranked / densest vertices).
    /// `ranking` must be the SAME slice passed to every later `advance_to`
    /// call. Returns the builder plus that first induced subgraph.
    pub(crate) fn new(pattern: &'a Pattern, ranking: &[usize], k_max: usize) -> (Self, InducedSubgraph) {
        let n = pattern.n;
        let mut removed = vec![false; n];
        for &v in &ranking[..k_max] {
            removed[v] = true;
        }
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        // Build adjacency among kept vertices directly from the pattern,
        // O(nnz): each undirected edge is visited once via the `i > j` guard
        // (mirrors `Pattern`'s own upper/lower symmetry) and pushed to both
        // endpoints' lists.
        for j in 0..n {
            if removed[j] {
                continue;
            }
            for &i in pattern.col(j) {
                if i > j && !removed[i] {
                    adj[j].push(i);
                    adj[i].push(j);
                }
            }
        }
        let b = NestedSubgraphBuilder { pattern, n, removed, adj };
        let sub = b.emit();
        (b, sub)
    }

    /// Advance to the induced subgraph for `k` (strictly less than the `k`
    /// used to reach the CURRENT state — `k_max` on the first call, or the
    /// previous call's `k` thereafter): restore `ranking[k..k_prev]` into the
    /// graph and emit the resulting induced subgraph. `ranking` must be the
    /// same slice used in [`new`](Self::new).
    pub(crate) fn advance_to(&mut self, ranking: &[usize], k: usize, k_prev: usize) -> InducedSubgraph {
        debug_assert!(k < k_prev, "advance_to requires a strictly smaller k (nesting downward)");
        for &v in &ranking[k..k_prev] {
            self.removed[v] = false;
            // Record v's edges to every neighbor ALREADY kept (either kept
            // from the start, or restored in an earlier batch/earlier in
            // this same batch — see the module doc's within-batch argument).
            // A neighbor restored LATER in this same batch will record the
            // edge itself when its own turn comes, so no duplicate.
            for &w in self.pattern.col(v) {
                if !self.removed[w] {
                    self.adj[v].push(w);
                    self.adj[w].push(v);
                }
            }
        }
        self.emit()
    }

    /// Materialize the CURRENT kept set as an i32 CSC pattern in local
    /// numbering (ascending global id), row indices sorted within each
    /// column.
    fn emit(&self) -> InducedSubgraph {
        let mut local_to_global = Vec::new();
        let mut global_to_local = vec![usize::MAX; self.n];
        for v in 0..self.n {
            if !self.removed[v] {
                global_to_local[v] = local_to_global.len();
                local_to_global.push(v);
            }
        }
        let m = local_to_global.len();
        let mut col_ptr = vec![0i32; m + 1];
        for (li, &v) in local_to_global.iter().enumerate() {
            col_ptr[li + 1] = col_ptr[li] + self.adj[v].len() as i32;
        }
        let mut row_idx = vec![0i32; col_ptr[m] as usize];
        for (li, &v) in local_to_global.iter().enumerate() {
            let start = col_ptr[li] as usize;
            let deg = self.adj[v].len();
            for (t, &w) in self.adj[v].iter().enumerate() {
                row_idx[start + t] = global_to_local[w] as i32;
            }
            row_idx[start..start + deg].sort_unstable();
        }
        InducedSubgraph { col_ptr, row_idx, local_to_global }
    }
}

impl InducedSubgraph {
    /// Materialize this induced subgraph as a [`Pattern`] in local numbering,
    /// ready for `nd_order` / `nd_order_impl` / AMD.
    pub(crate) fn to_pattern(&self) -> Pattern {
        Pattern {
            n: self.local_to_global.len(),
            col_ptr: self.col_ptr.iter().map(|&x| x as usize).collect(),
            row_idx: self.row_idx.iter().map(|&x| x as usize).collect(),
        }
    }
}

/// Induced subgraph on an explicit kept set, via [`NestedSubgraphBuilder`]:
/// excluded vertices are a prefix of a deterministic ranking (ascending id),
/// `k_max = n - |keep|`, so the builder's first emit is exactly the vertex-
/// induced subgraph of `keep`. Local numbering is ascending global id.
pub(crate) fn induced_keep(pattern: &Pattern, keep: &[bool]) -> InducedSubgraph {
    let n = pattern.n;
    debug_assert_eq!(keep.len(), n);
    let mut ranking = Vec::with_capacity(n);
    for v in 0..n {
        if !keep[v] {
            ranking.push(v);
        }
    }
    let k_excl = ranking.len();
    for v in 0..n {
        if keep[v] {
            ranking.push(v);
        }
    }
    let (_builder, sub) = NestedSubgraphBuilder::new(pattern, &ranking, k_excl);
    sub
}

/// From-scratch reference builder: filter the full pattern for `ranking[..k]`
/// excluded, independent of [`NestedSubgraphBuilder`]'s incremental state.
/// Used ONLY to cross-check the incremental builder; not on any hot path.
#[cfg(test)]
fn naive_induced(pattern: &Pattern, ranking: &[usize], k: usize) -> InducedSubgraph {
    let n = pattern.n;
    let mut removed = vec![false; n];
    for &v in &ranking[..k] {
        removed[v] = true;
    }
    let mut local_to_global = Vec::new();
    let mut global_to_local = vec![usize::MAX; n];
    for v in 0..n {
        if !removed[v] {
            global_to_local[v] = local_to_global.len();
            local_to_global.push(v);
        }
    }
    let m = local_to_global.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); m];
    for (li, &v) in local_to_global.iter().enumerate() {
        for &w in pattern.col(v) {
            if !removed[w] {
                adj[li].push(global_to_local[w]);
            }
        }
    }
    let mut col_ptr = vec![0i32; m + 1];
    for i in 0..m {
        col_ptr[i + 1] = col_ptr[i] + adj[i].len() as i32;
    }
    let mut row_idx = vec![0i32; col_ptr[m] as usize];
    for i in 0..m {
        let start = col_ptr[i] as usize;
        for (t, &w) in adj[i].iter().enumerate() {
            row_idx[start + t] = w as i32;
        }
        row_idx[start..start + adj[i].len()].sort_unstable();
    }
    InducedSubgraph { col_ptr, row_idx, local_to_global }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_subgraph_eq(a: &InducedSubgraph, b: &InducedSubgraph, ctx: &str) {
        assert_eq!(a.local_to_global, b.local_to_global, "{ctx}: local_to_global mismatch");
        assert_eq!(a.col_ptr, b.col_ptr, "{ctx}: col_ptr mismatch");
        assert_eq!(a.row_idx, b.row_idx, "{ctx}: row_idx mismatch");
    }

    /// Run the shipped strip-k schedule (descending) against the naive
    /// rebuild at every step, on one pattern/ranking.
    fn check_schedule(pattern: &Pattern, ranking: &[usize], ks: &[usize]) {
        assert!(ks.windows(2).all(|w| w[0] > w[1]), "ks must be strictly descending");
        let k_max = ks[0];
        let (mut builder, first) = NestedSubgraphBuilder::new(pattern, ranking, k_max);
        assert_subgraph_eq(&first, &naive_induced(pattern, ranking, k_max), &format!("k={k_max}"));
        let mut k_prev = k_max;
        for &k in &ks[1..] {
            let sub = builder.advance_to(ranking, k, k_prev);
            assert_subgraph_eq(&sub, &naive_induced(pattern, ranking, k), &format!("k={k}"));
            k_prev = k;
        }
    }

    #[test]
    fn matches_naive_on_synthetic_graphs() {
        // A mesh-like grid (regular degree, so degree ties matter for the
        // ranking's deterministic tie-break).
        let side = 20usize;
        let mut edges = Vec::new();
        for r in 0..side {
            for c in 0..side {
                let v = r * side + c;
                if c + 1 < side {
                    edges.push((v, v + 1));
                }
                if r + 1 < side {
                    edges.push((v, v + side));
                }
            }
        }
        let grid = Pattern::from_edges(side * side, &edges);
        let grid_ranking = degree_ranking(&grid);
        check_schedule(&grid, &grid_ranking, &[96, 64, 48, 24, 6, 0]);
        check_schedule(&grid, &grid_ranking, &[50, 10, 0]);

        // A star-heavy graph: a few very dense hubs plus a sparse tail —
        // exercises the "densest vertices are adjacent to almost everything"
        // case where restoring a hub touches many edges at once.
        let n = 150;
        let mut hub_edges = Vec::new();
        for v in 1..n {
            hub_edges.push((0, v));
            hub_edges.push((1, v));
        }
        for v in 2..n - 1 {
            hub_edges.push((v, v + 1));
        }
        let hubs = Pattern::from_edges(n, &hub_edges);
        let hub_ranking = degree_ranking(&hubs);
        check_schedule(&hubs, &hub_ranking, &[96, 64, 48, 24, 6, 0]);

        // A disconnected graph (several small components) — exercises
        // subgraphs with isolated vertices and multiple roots.
        let mut disc_edges = Vec::new();
        for block in 0..10 {
            let base = block * 12;
            for v in 0..11 {
                disc_edges.push((base + v, base + v + 1));
            }
        }
        let disc = Pattern::from_edges(120, &disc_edges);
        let disc_ranking = degree_ranking(&disc);
        check_schedule(&disc, &disc_ranking, &[40, 20, 8, 0]);
    }

    /// Same cross-check, driven off the real dev corpus, for the exact
    /// strip-k schedule described in the integration brief (k ∈ {96, 64, 48,
    /// 24, 6}, descending, plus k=0 as a trivial final step). `#[ignore]`d
    /// like the other measurement probes — run with
    /// `cargo test --release -- --ignored --nocapture matches_naive_on_corpus`.
    #[test]
    #[ignore]
    fn matches_naive_on_corpus() {
        let corpus = crate::corpus::corpus();
        let ks = [96usize, 64, 48, 24, 6, 0];
        let mut checked = 0usize;
        for (_name, pat) in &corpus {
            if pat.n <= ks[0] {
                continue; // k_max must leave at least one vertex kept to be meaningful
            }
            let ranking = degree_ranking(pat);
            check_schedule(pat, &ranking, &ks);
            checked += 1;
        }
        println!("nested_subgraph_corpus: cross-checked {checked} matrices");
        assert!(checked > 0);
    }

    /// Measured allocation reduction for the shipped strip-k schedule
    /// (k ∈ {96, 64, 48, 24, 6}, descending): six independent from-scratch
    /// rebuilds vs one `NestedSubgraphBuilder::new` + five `advance_to`
    /// calls, under the counting global allocator (see `alloc_probe`) —
    /// the same machine-independent proxy used for the `ScoreWorkspace`
    /// measurement. Run with:
    /// `cargo test --release -- --ignored --nocapture --test-threads=1 nested_subgraph_alloc`
    #[test]
    #[ignore]
    fn nested_subgraph_alloc_reduction() {
        let corpus = crate::corpus::corpus();
        let ks = [96usize, 64, 48, 24, 6, 0];
        let mut tot_naive = super::super::alloc_probe::AllocStats { calls: 0, bytes: 0, deallocs: 0, reallocs: 0 };
        let mut tot_incr = tot_naive;
        let mut checked = 0usize;

        for (_name, pat) in &corpus {
            if pat.n <= ks[0] {
                continue;
            }
            let ranking = degree_ranking(pat);

            let (_out, naive_stats) = super::super::alloc_probe::measure(|| {
                let mut out = Vec::with_capacity(ks.len());
                for &k in &ks {
                    out.push(naive_induced(pat, &ranking, k));
                }
                out
            });

            let (_out, incr_stats) = super::super::alloc_probe::measure(|| {
                let mut out = Vec::with_capacity(ks.len());
                let (mut builder, first) = NestedSubgraphBuilder::new(pat, &ranking, ks[0]);
                out.push(first);
                let mut k_prev = ks[0];
                for &k in &ks[1..] {
                    out.push(builder.advance_to(&ranking, k, k_prev));
                    k_prev = k;
                }
                out
            });

            tot_naive = tot_naive + naive_stats;
            tot_incr = tot_incr + incr_stats;
            checked += 1;
        }

        println!("\n--- nested_subgraph: naive (6x independent) vs incremental ---");
        println!("matrices checked: {checked}");
        println!("naive:       {} calls, {} bytes", tot_naive.calls, tot_naive.bytes);
        println!("incremental: {} calls, {} bytes", tot_incr.calls, tot_incr.bytes);
        println!(
            "reduction: {:.1}% fewer allocator calls, {:.1}% fewer bytes allocated",
            100.0 * (1.0 - tot_incr.calls as f64 / tot_naive.calls as f64),
            100.0 * (1.0 - tot_incr.bytes as f64 / tot_naive.bytes as f64)
        );
        assert!(checked > 0);
    }
}
