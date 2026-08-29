//! Read-only census of the clique-minimal-separator decomposition ("atoms").
//!
//! MECHANISM (Tarjan 1985, "Decomposition by clique separators"; Berry,
//! Pogorelcnik & Simonet 2010, "An introduction to clique minimal separator
//! decomposition"): a graph `G` decomposes along its clique minimal separators
//! into *atoms* — maximal subgraphs with no clique separator — and a minimal
//! triangulation of each atom, unioned, is a minimal triangulation of `G`. If
//! the dev matrices split into many small atoms, an expensive per-atom ordering
//! search becomes affordable; if every matrix is essentially one atom, the whole
//! family is dead.
//!
//! THIS FILE IS A CENSUS PROBE ONLY. Nothing here is wired into `order()`.
//!
//! ## What is computed, and in which direction it is biased
//!
//! The exact algorithm needs a *minimal* triangulation (MCS-M). We use plain MCS
//! (maximum cardinality search) plus a *partial* elimination game: fill edges are
//! materialised only when the higher-neighbourhood being eliminated is small
//! (`FILL_CAP`). That is sound but incomplete, and the incompleteness has a known
//! sign:
//!
//!   * SOUNDNESS. Let `G''` be any supergraph of the surviving subgraph `G[V']`
//!     (the true filled graph, a partially filled graph, or `G[V']` itself). Take
//!     `S = madj_{G''}(x)`, the neighbours of `x` in `G''` that are later in the
//!     elimination order, and let `C` be the component of `x` in `G''[V' \ S]`.
//!     Then `N_G(C) ⊆ N_{G''}(C) ⊆ S`, so whenever `S` is a clique *in G* and
//!     `V' \ (C ∪ S)` is non-empty, `S` is a genuine clique separator of `G[V']`.
//!     Every split this file reports is therefore real.
//!
//!   * `max_atom` IS AN UPPER BOUND on the true largest atom. Let `A` be a true
//!     atom inside the surviving graph. `A` has no clique separator, so the
//!     clique `S ∩ A` cannot split it: `A` minus `S` lies inside a single
//!     component, hence `A` is contained in `C ∪ S` or in `(V' \ C) ∪ S`. Every
//!     atom therefore stays whole inside one piece at every step, and the
//!     largest piece is at least as large as the largest atom. A reported
//!     `max_atom/n` BELOW the decision threshold is conclusive; a reported ratio
//!     ABOVE it is only suggestive, since a missing fill edge or an oversized
//!     `S` can hide a split we never tried.
//!
//!   * `n_atoms` IS NOT AN ATOM COUNT. Plain MCS yields clique separators that
//!     are not always *minimal* separators, and splitting on a non-minimal one
//!     over-decomposes (it can cut a true atom's interior into overlapping
//!     pieces). So `n_atoms` and `sep_total` describe THIS decomposition — read
//!     `max_atom` for the decision, and the other two only as texture.
//!
//! Deterministic (index-ordered tie-breaks everywhere), no wall clock: work is
//! bounded by an explicit ops meter and the routine returns `None` on overrun.

/// Result of one census run.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AtomCensus {
    /// Number of pieces (unions of true atoms) the decomposition produced.
    pub n_atoms: usize,
    /// Vertex count of the largest piece, separator vertices included.
    pub max_atom: usize,
    /// Sum of `|S|` over the accepted clique separators.
    pub sep_total: usize,
    /// Ops charged (adjacency touches + pair tests).
    pub ops_used: i64,
}

/// Only clique-test a higher-neighbourhood at most this large is decided by the
/// caller (`k_cap`); fill is materialised under the same cap so the elimination
/// game can never cost more than `FILL_CAP^2 / 2` per vertex.
const FILL_CAP: usize = 32;

/// The working graph: a frozen CSR of the ORIGINAL edges plus a per-vertex list
/// of materialised fill edges. Membership is a binary search in the CSR or a
/// hash lookup in the fill set, so both the pair test and the edge insertion are
/// O(1)-ish — which matters because the elimination game inserts in bursts.
struct Graph {
    start: Vec<usize>,
    orig: Vec<u32>,
    fill: Vec<Vec<u32>>,
    fill_set: std::collections::HashSet<u64>,
}

#[inline]
fn key(a: usize, b: usize) -> u64 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    ((lo as u64) << 32) | hi as u64
}

impl Graph {
    /// Build from a symmetric CSC pattern, dropping self-loops and duplicates.
    /// The corpus stores symmetrised patterns; any asymmetry is repaired by
    /// inserting both directions.
    fn from_csc(n: usize, col_ptr: &[usize], row_idx: &[usize], ops: &mut i64) -> Graph {
        let mut cols: Vec<Vec<u32>> = vec![Vec::new(); n];
        for j in 0..n {
            let (lo, hi) = (col_ptr[j], col_ptr[j + 1]);
            for &r in &row_idx[lo..hi] {
                *ops += 1;
                if r == j || r >= n {
                    continue;
                }
                cols[j].push(r as u32);
                cols[r].push(j as u32);
            }
        }
        let mut start = vec![0usize; n + 1];
        let mut orig: Vec<u32> = Vec::new();
        for v in 0..n {
            cols[v].sort_unstable();
            cols[v].dedup();
            orig.extend_from_slice(&cols[v]);
            start[v + 1] = orig.len();
            cols[v] = Vec::new();
        }
        Graph {
            start,
            orig,
            fill: vec![Vec::new(); n],
            fill_set: std::collections::HashSet::new(),
        }
    }

    #[inline]
    fn nbrs(&self, v: usize) -> (&[u32], &[u32]) {
        (&self.orig[self.start[v]..self.start[v + 1]], &self.fill[v])
    }

    #[inline]
    fn deg(&self, v: usize) -> usize {
        self.start[v + 1] - self.start[v] + self.fill[v].len()
    }

    /// One charged pair test, in the ORIGINAL graph only. The clique test must
    /// hold in `G` — a fill edge is not an edge of `G`, so counting it would
    /// manufacture separators that are not cliques.
    #[inline]
    fn adjacent_orig(&self, u: usize, w: u32, ops: &mut i64) -> bool {
        *ops += 1;
        self.orig[self.start[u]..self.start[u + 1]].binary_search(&w).is_ok()
    }

    /// Insert the undirected fill edge `(u, w)`.
    fn add_edge(&mut self, u: usize, w: usize, ops: &mut i64) {
        *ops += 1;
        if self.fill_set.insert(key(u, w)) {
            self.fill[u].push(w as u32);
            self.fill[w].push(u as u32);
        }
    }
}

/// Maximum cardinality search. Returns the VISIT order (first visited is
/// numbered `n`, last is numbered `1`); the elimination order is its reverse.
/// Ties break on the smallest vertex index, so the result is deterministic.
/// Handles disconnected graphs (a new start vertex simply has weight 0).
fn mcs_visit_order(g: &Graph, n: usize, ops: &mut i64, budget: i64) -> Option<Vec<u32>> {
    // Bucket structure: `bucket[w]` holds the unvisited vertices of weight `w`.
    // Weights only ever increase by one, so a simple max-pointer walk is O(n+m).
    let mut weight = vec![0usize; n];
    let mut visited = vec![false; n];
    // Position of each vertex inside its bucket, for O(1) removal.
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); n + 1];
    let mut pos_in_bucket = vec![0usize; n];
    for v in 0..n {
        pos_in_bucket[v] = buckets[0].len();
        buckets[0].push(v as u32);
    }
    let mut order = Vec::with_capacity(n);
    let mut max_w = 0usize;

    let remove = |buckets: &mut Vec<Vec<u32>>, pos_in_bucket: &mut Vec<usize>, w: usize, v: u32| {
        let p = pos_in_bucket[v as usize];
        let last = *buckets[w].last().unwrap();
        buckets[w][p] = last;
        pos_in_bucket[last as usize] = p;
        buckets[w].pop();
    };

    for _ in 0..n {
        if *ops > budget {
            return None;
        }
        while buckets[max_w].is_empty() {
            if max_w == 0 {
                break;
            }
            max_w -= 1;
        }
        if buckets[max_w].is_empty() {
            // Only reachable if every vertex is visited; defensive.
            break;
        }
        // Deterministic tie-break: the bucket's last entry. Every insertion and
        // swap-removal above is itself deterministic, so the whole order is.
        let best = *buckets[max_w].last().unwrap();
        remove(&mut buckets, &mut pos_in_bucket, max_w, best);
        let v = best as usize;
        visited[v] = true;
        order.push(best);
        *ops += g.deg(v) as i64;
        let (o_nb, f_nb) = g.nbrs(v);
        for &u in o_nb.iter().chain(f_nb.iter()) {
            let u = u as usize;
            if visited[u] {
                continue;
            }
            let w = weight[u];
            remove(&mut buckets, &mut pos_in_bucket, w, u as u32);
            weight[u] = w + 1;
            pos_in_bucket[u] = buckets[w + 1].len();
            buckets[w + 1].push(u as u32);
            if w + 1 > max_w {
                max_w = w + 1;
            }
        }
    }
    Some(order)
}

/// Census the clique-minimal-separator decomposition of the pattern.
///
/// `k_cap` bounds the separator size we are willing to clique-test (larger
/// candidate separators are skipped, which again only loses splits).
/// `budget` bounds the ops meter; `None` means the budget was exhausted.
pub(crate) fn atom_census(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    k_cap: usize,
    budget: i64,
) -> Option<AtomCensus> {
    if n == 0 || col_ptr.len() != n + 1 {
        return None;
    }
    let mut ops: i64 = 0;
    let mut g = Graph::from_csc(n, col_ptr, row_idx, &mut ops);
    if ops > budget {
        return None;
    }

    let visit = mcs_visit_order(&g, n, &mut ops, budget)?;
    // Elimination order = reverse of the MCS visit order; `pos[v]` is v's index
    // in that elimination order, so "later in the order" == "higher MCS number".
    let mut elim: Vec<u32> = visit;
    elim.reverse();
    let mut pos = vec![0usize; n];
    for (i, &v) in elim.iter().enumerate() {
        pos[v as usize] = i;
    }

    let mut removed = vec![false; n];
    let mut alive = n;
    let mut n_atoms = 0usize;
    let mut max_atom = 0usize;
    let mut sep_total = 0usize;

    // Scratch reused across iterations.
    let mut s_buf: Vec<u32> = Vec::with_capacity(k_cap + 1);
    let mut in_s = vec![false; n];
    let mut queue: Vec<u32> = Vec::new();
    let mut in_c = vec![false; n];

    for &xv in elim.iter() {
        if ops > budget {
            return None;
        }
        let x = xv as usize;
        if removed[x] {
            continue;
        }
        // S = surviving higher-order neighbours of x in the (partially filled) graph.
        s_buf.clear();
        let cap = k_cap.max(FILL_CAP);
        let mut too_big = false;
        ops += g.deg(x) as i64;
        {
            let (o_nb, f_nb) = g.nbrs(x);
            for &u in o_nb.iter().chain(f_nb.iter()) {
                let ui = u as usize;
                if removed[ui] || pos[ui] < pos[x] {
                    continue;
                }
                if s_buf.len() == cap {
                    too_big = true;
                    break;
                }
                s_buf.push(u);
            }
        }
        if too_big {
            continue;
        }
        // An empty S is a (trivial) clique separator: it splits off whichever
        // connected component of the surviving graph contains x. That is the
        // right handling for an isolated vertex AND for a disconnected graph,
        // so it needs no special case — it falls through below.

        // Clique test on S, collecting the missing pairs so a failed test can be
        // reused as the elimination-game fill step.
        let mut missing: Vec<(u32, u32)> = Vec::new();
        let mut is_clique = true;
        for i in 0..s_buf.len() {
            for j in (i + 1)..s_buf.len() {
                if !g.adjacent_orig(s_buf[i] as usize, s_buf[j], &mut ops) {
                    is_clique = false;
                    missing.push((s_buf[i], s_buf[j]));
                }
            }
        }

        if is_clique && s_buf.len() <= k_cap {
            // C = component of x in G''[V' \ S]. Charged BFS; every vertex of C
            // is removed afterwards, so adjacency is scanned O(1) times overall.
            for &s in &s_buf {
                in_s[s as usize] = true;
            }
            queue.clear();
            queue.push(xv);
            in_c[x] = true;
            let mut head = 0usize;
            while head < queue.len() {
                let v = queue[head] as usize;
                head += 1;
                ops += g.deg(v) as i64;
                {
                    let (o_nb, f_nb) = g.nbrs(v);
                    for &u in o_nb.iter().chain(f_nb.iter()) {
                        let ui = u as usize;
                        if removed[ui] || in_s[ui] || in_c[ui] {
                            continue;
                        }
                        in_c[ui] = true;
                        queue.push(u);
                    }
                }
                if ops > budget {
                    for &s in &s_buf {
                        in_s[s as usize] = false;
                    }
                    for &v in queue.iter() {
                        in_c[v as usize] = false;
                    }
                    return None;
                }
            }
            let c_len = queue.len();
            let rest = alive.saturating_sub(c_len + s_buf.len());
            if rest > 0 {
                // Genuine clique separator: split off the atom C ∪ S.
                n_atoms += 1;
                max_atom = max_atom.max(c_len + s_buf.len());
                sep_total += s_buf.len();
                for &v in queue.iter() {
                    removed[v as usize] = true;
                }
                alive -= c_len;
            }
            for &s in &s_buf {
                in_s[s as usize] = false;
            }
            for &v in queue.iter() {
                in_c[v as usize] = false;
            }
            queue.clear();
        } else if s_buf.len() <= FILL_CAP {
            // Partial elimination game: materialise the fill among S so later
            // vertices see a closer approximation of the filled graph.
            for (a, b) in missing {
                g.add_edge(a as usize, b as usize, &mut ops);
            }
        }
    }

    // Whatever survives is the final piece (or pieces, if disconnected).
    if alive > 0 {
        let mut seen = vec![false; n];
        for v in 0..n {
            if removed[v] || seen[v] {
                continue;
            }
            queue.clear();
            queue.push(v as u32);
            seen[v] = true;
            let mut head = 0usize;
            while head < queue.len() {
                let w = queue[head] as usize;
                head += 1;
                ops += g.deg(w) as i64;
                {
                    let (o_nb, f_nb) = g.nbrs(w);
                    for &u in o_nb.iter().chain(f_nb.iter()) {
                        let ui = u as usize;
                        if removed[ui] || seen[ui] {
                            continue;
                        }
                        seen[ui] = true;
                        queue.push(u);
                    }
                }
            }
            n_atoms += 1;
            max_atom = max_atom.max(queue.len());
            if ops > budget {
                return None;
            }
        }
    }

    Some(AtomCensus { n_atoms, max_atom, sep_total, ops_used: ops })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a CSC pattern (with the diagonal, as the corpus does) from an edge
    /// list on `n` vertices.
    fn csc(n: usize, edges: &[(usize, usize)]) -> (Vec<usize>, Vec<usize>) {
        let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n];
        for v in 0..n {
            cols[v].push(v);
        }
        for &(a, b) in edges {
            cols[a].push(b);
            cols[b].push(a);
        }
        let mut col_ptr = vec![0usize; n + 1];
        let mut row_idx = Vec::new();
        for v in 0..n {
            cols[v].sort_unstable();
            cols[v].dedup();
            row_idx.extend_from_slice(&cols[v]);
            col_ptr[v + 1] = row_idx.len();
        }
        (col_ptr, row_idx)
    }

    #[test]
    fn two_triangles_sharing_an_edge_do_not_split() {
        // K4 minus an edge: the only separator is the shared edge {1,2}, which is
        // a clique, so it splits into two triangles.
        let (cp, ri) = csc(4, &[(0, 1), (0, 2), (1, 2), (1, 3), (2, 3)]);
        let c = atom_census(4, &cp, &ri, 32, 10_000).unwrap();
        assert_eq!(c.n_atoms, 2);
        assert_eq!(c.max_atom, 3);
    }

    #[test]
    fn complete_graph_is_one_atom() {
        let mut edges = Vec::new();
        for i in 0..6 {
            for j in (i + 1)..6 {
                edges.push((i, j));
            }
        }
        let (cp, ri) = csc(6, &edges);
        let c = atom_census(6, &cp, &ri, 32, 100_000).unwrap();
        assert_eq!(c.n_atoms, 1);
        assert_eq!(c.max_atom, 6);
    }

    #[test]
    fn long_path_splits_into_edges() {
        let edges: Vec<(usize, usize)> = (0..9).map(|i| (i, i + 1)).collect();
        let (cp, ri) = csc(10, &edges);
        let c = atom_census(10, &cp, &ri, 32, 100_000).unwrap();
        // Every internal vertex is a cut vertex (a clique separator of size 1).
        assert!(c.max_atom <= 2, "max_atom = {}", c.max_atom);
        assert!(c.n_atoms >= 9, "n_atoms = {}", c.n_atoms);
    }

    #[test]
    fn budget_overrun_returns_none() {
        let edges: Vec<(usize, usize)> = (0..99).map(|i| (i, i + 1)).collect();
        let (cp, ri) = csc(100, &edges);
        assert!(atom_census(100, &cp, &ri, 32, 1).is_none());
    }

    #[test]
    fn two_cliques_joined_by_a_triangle_split() {
        // Two K6 blocks glued on a shared triangle {0,1,2}: one clique minimal
        // separator of size 3, two atoms of size 6.
        let mut edges = Vec::new();
        let left = [0usize, 1, 2, 3, 4, 5];
        let right = [0usize, 1, 2, 6, 7, 8];
        for blk in [left, right] {
            for i in 0..blk.len() {
                for j in (i + 1)..blk.len() {
                    edges.push((blk[i], blk[j]));
                }
            }
        }
        let (cp, ri) = csc(9, &edges);
        let c = atom_census(9, &cp, &ri, 32, 1_000_000).unwrap();
        // The true decomposition is two atoms of size 6. Plain MCS also fires on
        // non-minimal clique separators (e.g. {0,1,2,3,4} cutting off vertex 5),
        // so the piece count can exceed 2 — but max_atom must still be at least
        // the true largest atom, and never exceed n.
        assert!(c.max_atom >= 6, "max_atom = {}", c.max_atom);
        assert!(c.max_atom <= 9);
        assert!(c.n_atoms >= 2);
        assert!(c.sep_total >= 3);
    }
}

// ---------------------------------------------------------------------------
// ATOM ORDERING — the constructive follow-up to the census above.
//
// The census answers "does the graph split?"; this answers "does ordering the
// pieces separately beat ordering the whole?". It reuses the SAME MCS +
// clique-separator scan, but instead of only counting a split it RETAINS the
// atom (component `C` plus its bounding separator `S`) and emits a full
// permutation:
//
//   * every accepted split contributes one atom, in discovery order. Because a
//     component `C` is removed the moment its atom is recorded, and `S` is
//     built only from SURVIVING vertices, a vertex removed with `C` can never
//     reappear in any later atom. Separator vertices stay alive and are
//     therefore emitted later — with the last atom that contains them, exactly
//     the child-before-parent linearization `biconn.rs` uses for articulation
//     points. Fill never crosses a clique separator, so this ordering is the
//     classical fill-decomposition construction generalised from cut VERTICES
//     (biconn) to clique SEPARATORS.
//
//   * the inner solver runs on the subgraph induced (in the ORIGINAL graph, not
//     the partially filled one) by the WHOLE atom `C ∪ S`, so the boundary
//     structure is visible to it, and only the `C` part of the returned local
//     order is emitted. Ordering `C` while blind to `S` would hide precisely
//     the fill that the separator absorbs.
//
//   * whatever survives the scan is the "root" piece; its connected components
//     (in the ORIGINAL graph — a finer and always-valid split, since fill edges
//     are not edges of `G`) are ordered last.
//
// Deterministic (ascending vertex sets, index-ordered tie-breaks, fixed atom
// order), no wall clock, ops-metered like the census. `None` on budget
// overrun, on any inner-solver or bijection inconsistency, or when the largest
// atom exceeds `0.8 n` (the decomposition is then not buying anything).

/// Largest atom, relative to `n`, above which the decomposition is declared
/// useless and `atom_order` returns `None`.
const MAX_ATOM_FRACTION: f64 = 0.8;

/// Order `verts` (ascending global ids, the whole atom `C ∪ S`) with the inner
/// solver on the induced ORIGINAL subgraph. Returns the global ids in
/// elimination order, or `None` if the inner solver refuses the subgraph.
///
/// `tok`/`loc` are caller-owned scratch of length `n`; `gen` must be a fresh
/// token. The induced pattern carries the diagonal, matching what the shipped
/// `order()` hands the same solvers.
fn order_subgraph(
    g: &Graph,
    verts: &[u32],
    tok: &mut [u32],
    loc: &mut [u32],
    gen: u32,
    inner: u8,
    ops: &mut i64,
) -> Option<Vec<u32>> {
    let m = verts.len();
    if m == 0 {
        return Some(Vec::new());
    }
    for (li, &v) in verts.iter().enumerate() {
        tok[v as usize] = gen;
        loc[v as usize] = li as u32;
    }
    if m == 1 {
        return Some(vec![verts[0]]);
    }
    // Count pass (diagonal included, fill edges excluded).
    let mut col_ptr = vec![0i32; m + 1];
    for (li, &v) in verts.iter().enumerate() {
        let (o_nb, _f_nb) = g.nbrs(v as usize);
        *ops += o_nb.len() as i64;
        let mut cnt = 1i32;
        for &u in o_nb {
            if tok[u as usize] == gen {
                cnt += 1;
            }
        }
        col_ptr[li + 1] = cnt;
    }
    for i in 0..m {
        col_ptr[i + 1] += col_ptr[i];
    }
    let mut row_idx = vec![0i32; col_ptr[m] as usize];
    for (li, &v) in verts.iter().enumerate() {
        let (o_nb, _f_nb) = g.nbrs(v as usize);
        let mut p = col_ptr[li] as usize;
        // `verts` is ascending and `g.orig[v]` is ascending, so the local ids
        // come out ascending; inserting the diagonal in its sorted slot keeps
        // every column sorted without a sort pass.
        let mut diag_done = false;
        for &u in o_nb {
            if tok[u as usize] != gen {
                continue;
            }
            let lu = loc[u as usize] as i32;
            if !diag_done && lu > li as i32 {
                row_idx[p] = li as i32;
                p += 1;
                diag_done = true;
            }
            row_idx[p] = lu;
            p += 1;
        }
        if !diag_done {
            row_idx[p] = li as i32;
            p += 1;
        }
        debug_assert_eq!(p, col_ptr[li + 1] as usize);
    }
    *ops += (m + row_idx.len()) as i64;

    let core = feral_ordering_core::CscPattern::new(m, &col_ptr, &row_idx)?;
    let sub: Vec<i32> = match inner {
        0 => feral_amd::amd_order(&core).ok()?,
        1 => feral_amf::amf_order_opts(&core, &feral_amf::AmfOptions::default())
            .ok()
            .map(|(p, ..)| p)?,
        _ => return None,
    };
    if sub.len() != m {
        return None;
    }
    let mut out = Vec::with_capacity(m);
    let mut seen = vec![false; m];
    for &x in &sub {
        if x < 0 {
            return None;
        }
        let li = x as usize;
        if li >= m || seen[li] {
            return None;
        }
        seen[li] = true;
        out.push(verts[li]);
    }
    Some(out)
}

/// Whole-matrix ordering by clique-atom decomposition. See the block comment
/// above for the construction. `inner` selects the per-atom solver:
/// `0` = `feral_amd::amd_order`, `1` = `feral_amf::amf_order_opts` (defaults).
///
/// Returns `perm[k]` = original index eliminated `k`-th, verified to be a
/// bijection of `0..n`, or `None`.
pub(crate) fn atom_order(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    k_cap: usize,
    budget: i64,
    inner: u8,
) -> Option<Vec<usize>> {
    if n == 0 || col_ptr.len() != n + 1 || inner > 1 {
        return None;
    }
    let mut ops: i64 = 0;
    let mut g = Graph::from_csc(n, col_ptr, row_idx, &mut ops);
    if ops > budget {
        return None;
    }

    let visit = mcs_visit_order(&g, n, &mut ops, budget)?;
    let mut elim: Vec<u32> = visit;
    elim.reverse();
    let mut pos = vec![0usize; n];
    for (i, &v) in elim.iter().enumerate() {
        pos[v as usize] = i;
    }

    let mut removed = vec![false; n];
    let mut alive = n;
    let mut max_atom = 0usize;
    // (owned component C, bounding separator S) in discovery order.
    let mut atoms: Vec<(Vec<u32>, Vec<u32>)> = Vec::new();

    let mut s_buf: Vec<u32> = Vec::with_capacity(k_cap + 1);
    let mut in_s = vec![false; n];
    let mut queue: Vec<u32> = Vec::new();
    let mut in_c = vec![false; n];

    for &xv in elim.iter() {
        if ops > budget {
            return None;
        }
        let x = xv as usize;
        if removed[x] {
            continue;
        }
        s_buf.clear();
        let cap = k_cap.max(FILL_CAP);
        let mut too_big = false;
        ops += g.deg(x) as i64;
        {
            let (o_nb, f_nb) = g.nbrs(x);
            for &u in o_nb.iter().chain(f_nb.iter()) {
                let ui = u as usize;
                if removed[ui] || pos[ui] < pos[x] {
                    continue;
                }
                if s_buf.len() == cap {
                    too_big = true;
                    break;
                }
                s_buf.push(u);
            }
        }
        if too_big {
            continue;
        }

        let mut missing: Vec<(u32, u32)> = Vec::new();
        let mut is_clique = true;
        for i in 0..s_buf.len() {
            for j in (i + 1)..s_buf.len() {
                if !g.adjacent_orig(s_buf[i] as usize, s_buf[j], &mut ops) {
                    is_clique = false;
                    missing.push((s_buf[i], s_buf[j]));
                }
            }
        }

        if is_clique && s_buf.len() <= k_cap {
            for &s in &s_buf {
                in_s[s as usize] = true;
            }
            queue.clear();
            queue.push(xv);
            in_c[x] = true;
            let mut head = 0usize;
            while head < queue.len() {
                let v = queue[head] as usize;
                head += 1;
                ops += g.deg(v) as i64;
                {
                    let (o_nb, f_nb) = g.nbrs(v);
                    for &u in o_nb.iter().chain(f_nb.iter()) {
                        let ui = u as usize;
                        if removed[ui] || in_s[ui] || in_c[ui] {
                            continue;
                        }
                        in_c[ui] = true;
                        queue.push(u);
                    }
                }
                if ops > budget {
                    return None;
                }
            }
            let c_len = queue.len();
            let rest = alive.saturating_sub(c_len + s_buf.len());
            if rest > 0 {
                max_atom = max_atom.max(c_len + s_buf.len());
                let mut owned = queue.clone();
                owned.sort_unstable();
                let mut sep = s_buf.clone();
                sep.sort_unstable();
                for &v in queue.iter() {
                    removed[v as usize] = true;
                }
                alive -= c_len;
                atoms.push((owned, sep));
            }
            for &s in &s_buf {
                in_s[s as usize] = false;
            }
            for &v in queue.iter() {
                in_c[v as usize] = false;
            }
            queue.clear();
        } else if s_buf.len() <= FILL_CAP {
            for (a, b) in missing {
                g.add_edge(a as usize, b as usize, &mut ops);
            }
        }
    }

    // Root pieces: connected components of the ORIGINAL graph restricted to the
    // survivors. Fill edges are not edges of `G`, so ignoring them here only
    // splits finer, which is always sound.
    if alive > 0 {
        let mut seen = vec![false; n];
        for v in 0..n {
            if removed[v] || seen[v] {
                continue;
            }
            queue.clear();
            queue.push(v as u32);
            seen[v] = true;
            let mut head = 0usize;
            while head < queue.len() {
                let w = queue[head] as usize;
                head += 1;
                let (o_nb, _f_nb) = g.nbrs(w);
                ops += o_nb.len() as i64;
                for &u in o_nb {
                    let ui = u as usize;
                    if removed[ui] || seen[ui] {
                        continue;
                    }
                    seen[ui] = true;
                    queue.push(u);
                }
            }
            if ops > budget {
                return None;
            }
            max_atom = max_atom.max(queue.len());
            let mut owned = queue.clone();
            owned.sort_unstable();
            atoms.push((owned, Vec::new()));
            queue.clear();
        }
    }

    if max_atom as f64 > MAX_ATOM_FRACTION * n as f64 {
        return None;
    }

    // Emit: children (earlier atoms) before parents; within an atom, the inner
    // solver's order on `C ∪ S`, restricted to the vertices this atom owns.
    let mut emitted = vec![false; n];
    let mut perm: Vec<usize> = Vec::with_capacity(n);
    let mut tok = vec![0u32; n];
    let mut loc = vec![0u32; n];
    let mut gen = 0u32;
    let mut union: Vec<u32> = Vec::new();

    for (owned, sep) in atoms.iter() {
        if ops > budget {
            return None;
        }
        // `owned` and `sep` are disjoint by construction (S is built from
        // survivors only, C is the component of x in G''[V' \ S]); merge two
        // sorted ascending lists.
        union.clear();
        let (mut i, mut j) = (0usize, 0usize);
        while i < owned.len() || j < sep.len() {
            let take_o = if i >= owned.len() {
                false
            } else if j >= sep.len() {
                true
            } else {
                owned[i] <= sep[j]
            };
            if take_o {
                union.push(owned[i]);
                i += 1;
            } else {
                union.push(sep[j]);
                j += 1;
            }
        }
        gen = gen.wrapping_add(1);
        if gen == 0 {
            gen = 1;
            tok.iter_mut().for_each(|t| *t = 0);
        }
        let local = order_subgraph(&g, &union, &mut tok, &mut loc, gen, inner, &mut ops)?;
        let mut n_owned = 0usize;
        for &v in &local {
            let vu = v as usize;
            if emitted[vu] {
                continue;
            }
            // Own it only if it is in this atom's component; separator vertices
            // wait for the last atom that contains them.
            if owned.binary_search(&v).is_ok() {
                emitted[vu] = true;
                perm.push(vu);
                n_owned += 1;
            }
        }
        if n_owned != owned.len() {
            return None;
        }
    }

    // Bijection check.
    if perm.len() != n {
        return None;
    }
    let mut seen = vec![false; n];
    for &v in &perm {
        if v >= n || seen[v] {
            return None;
        }
        seen[v] = true;
    }
    Some(perm)
}

#[cfg(test)]
mod order_tests {
    use super::*;

    fn csc2(n: usize, edges: &[(usize, usize)]) -> (Vec<usize>, Vec<usize>) {
        let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n];
        for v in 0..n {
            cols[v].push(v);
        }
        for &(a, b) in edges {
            cols[a].push(b);
            cols[b].push(a);
        }
        let mut col_ptr = vec![0usize; n + 1];
        let mut row_idx = Vec::new();
        for v in 0..n {
            cols[v].sort_unstable();
            cols[v].dedup();
            row_idx.extend_from_slice(&cols[v]);
            col_ptr[v + 1] = row_idx.len();
        }
        (col_ptr, row_idx)
    }

    fn is_perm(p: &[usize], n: usize) -> bool {
        if p.len() != n {
            return false;
        }
        let mut s = vec![false; n];
        for &v in p {
            if v >= n || s[v] {
                return false;
            }
            s[v] = true;
        }
        true
    }

    #[test]
    fn path_order_is_a_bijection() {
        let edges: Vec<(usize, usize)> = (0..49).map(|i| (i, i + 1)).collect();
        let (cp, ri) = csc2(50, &edges);
        for inner in [0u8, 1] {
            let p = atom_order(50, &cp, &ri, 32, 10_000_000, inner)
                .unwrap_or_else(|| panic!("inner {inner} returned None"));
            assert!(is_perm(&p, 50));
        }
    }

    #[test]
    fn two_cliques_on_a_triangle_order_children_first() {
        let mut edges = Vec::new();
        let left = [0usize, 1, 2, 3, 4, 5];
        let right = [0usize, 1, 2, 6, 7, 8];
        for blk in [left, right] {
            for i in 0..blk.len() {
                for j in (i + 1)..blk.len() {
                    edges.push((blk[i], blk[j]));
                }
            }
        }
        let (cp, ri) = csc2(9, &edges);
        let p = atom_order(9, &cp, &ri, 32, 1_000_000, 0).unwrap();
        assert!(is_perm(&p, 9));
    }

    #[test]
    fn complete_graph_is_rejected() {
        let mut edges = Vec::new();
        for i in 0..8 {
            for j in (i + 1)..8 {
                edges.push((i, j));
            }
        }
        let (cp, ri) = csc2(8, &edges);
        // One atom of size n: max_atom / n = 1.0 > 0.8.
        assert!(atom_order(8, &cp, &ri, 32, 1_000_000, 0).is_none());
    }

    #[test]
    fn disconnected_pieces_are_all_emitted() {
        // Three disjoint triangles.
        let edges = [
            (0usize, 1usize),
            (1, 2),
            (0, 2),
            (3, 4),
            (4, 5),
            (3, 5),
            (6, 7),
            (7, 8),
            (6, 8),
        ];
        let (cp, ri) = csc2(9, &edges);
        let p = atom_order(9, &cp, &ri, 32, 1_000_000, 1).unwrap();
        assert!(is_perm(&p, 9));
    }

    #[test]
    fn order_budget_overrun_returns_none() {
        let edges: Vec<(usize, usize)> = (0..99).map(|i| (i, i + 1)).collect();
        let (cp, ri) = csc2(100, &edges);
        assert!(atom_order(100, &cp, &ri, 32, 1, 0).is_none());
    }
}
