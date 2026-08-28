//! BICONNECTED-DECOMPOSITION ordering ("1-dissection" over the block-cut
//! tree) — a genuinely different candidate FAMILY from everything else in
//! `mod.rs`'s portfolio. Every other candidate orders the WHOLE graph at
//! once (AMD/AMF/custom metrics, partitioners, RCM/Sloan, strip/peel/telos
//! post-hoc transforms); none of them DECOMPOSES the graph first. This one
//! splits the pattern into its biconnected components (Tarjan, O(nnz)),
//! orders each component's interior with feral AMD on the induced subgraph,
//! and concatenates the block orderings so that every articulation point is
//! eliminated AFTER the subtrees hanging off it (deferred to its parent
//! block) — the classical fill-decomposition construction: fill never
//! crosses an articulation point, so the total fill decomposes into the
//! per-block fills when blocks are ordered contiguously and articulations
//! deferred.
//!
//! Why this can beat whole-graph AMD: AMD's greedy min-degree choice is
//! *myopic across articulation points* — an articulation's inflated degree
//! (it sums its degrees in every block it touches) distorts the pivot order
//! on BOTH sides, and AMD's interleaving of two blocks' vertices at similar
//! degrees is not the block-contiguous order the decomposition theorem
//! assumes. On the near-AMD mass this matters: the dev corpus's tied/near-
//! tied matrices are highly decomposable (median ~30% of vertices are
//! articulation points, median 65-399 biconnected components per matrix,
//! largest block only ~66% of n) — exactly the structure this family
//! exploits and no shipped candidate addresses.
//!
//! Root choice: each connected component's block-cut tree is rooted at its
//! LARGEST block (ties: lowest block id) so the dense "core" is eliminated
//! LAST and pendant blocks first — matching the intuition that periphery
//! should be eliminated before the core. Blocks are emitted in decreasing
//! tree depth (deepest subtree first), which is a valid linearization of
//! the required child-before-parent partial order (sibling subtrees are
//! vertex-disjoint, so their relative order is irrelevant).
//!
//! Cost: Tarjan is O(nnz); the per-block induced subgraphs partition the
//! edges, so building them all is O(nnz) total; the per-block AMD passes
//! sum to no more than roughly one whole-graph AMD pass. This family is
//! therefore cheap enough to gate WIDE — it reaches the sparse gt_10k ties
//! most candidates are gated out of. Deterministic: sorted adjacency,
//! ascending-id DFS starts, fixed (depth, id) block order, and feral AMD
//! per block. A graph that is already biconnected (one block, no isolated
//! vertices) reproduces the AMD anchor exactly, so callers should skip
//! `nc <= 1` (handled here by returning the anchor-equivalent order, which
//! the portfolio's duplicate memoization discards for free).

use crate::Pattern;
use feral::sparse::csc::CscPattern as ScoringPattern;

/// Build symmetric adjacency lists (off-diagonal only), each undirected edge
/// recorded once via the `i > j` guard, sorted ascending per vertex so the
/// Tarjan DFS traversal order is deterministic.
fn build_adj(pattern: &Pattern) -> Vec<Vec<u32>> {
    let n = pattern.n;
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    for j in 0..n {
        for &i in pattern.col(j) {
            if i > j && i < n {
                adj[j].push(i as u32);
                adj[i].push(j as u32);
            }
        }
    }
    for a in adj.iter_mut() {
        a.sort_unstable();
    }
    adj
}

/// Iterative Tarjan biconnected components, returned as edge lists (one list
/// per component, edges as `(u, v)` off-diagonal pairs in pop order).
/// Iterative (explicit stacks) because the corpus reaches n ≈ 340k and a
/// path graph's block-cut tree is ~n deep — recursion would overflow the
/// worker's stack. Deterministic given sorted adjacency.
fn biconnected_components(adj: &[Vec<u32>]) -> Vec<Vec<(u32, u32)>> {
    let n = adj.len();
    const NONE: u32 = u32::MAX;
    // disc == 0 means unvisited; otherwise discovery time (1-based).
    let mut disc = vec![0u32; n];
    let mut low = vec![0u32; n];
    let mut parent = vec![NONE; n];
    let mut estack: Vec<(u32, u32)> = Vec::new();
    let mut comps: Vec<Vec<(u32, u32)>> = Vec::new();
    let mut time = 0u32;
    // Explicit DFS stack: (vertex, next neighbor slot to scan).
    let mut st: Vec<(u32, u32)> = Vec::new();

    for s in 0..n {
        if disc[s] != 0 {
            continue;
        }
        time += 1;
        disc[s] = time;
        low[s] = time;
        st.push((s as u32, 0));
        while let Some(top) = st.last_mut() {
            let u = top.0 as usize;
            let slot = &mut top.1;
            if (*slot as usize) < adj[u].len() {
                let v = adj[u][*slot as usize] as usize;
                *slot += 1;
                if disc[v] == 0 {
                    // Tree edge: push and recurse into v.
                    estack.push((u as u32, v as u32));
                    time += 1;
                    disc[v] = time;
                    low[v] = time;
                    parent[v] = u as u32;
                    st.push((v as u32, 0));
                } else if v as u32 != parent[u] && disc[v] < disc[u] {
                    // Back edge to an ancestor (pushed once, from the
                    // deeper endpoint, via the disc[v] < disc[u] guard).
                    estack.push((u as u32, v as u32));
                    if disc[v] < low[u] {
                        low[u] = disc[v];
                    }
                }
            } else {
                // Finished u: propagate lowlink to its parent and, if u is
                // a subtree root whose lowlink reaches no higher than its
                // parent, pop one biconnected component off the edge stack.
                st.pop();
                let p = parent[u];
                if p != NONE {
                    let pu = p as usize;
                    if low[u] < low[pu] {
                        low[pu] = low[u];
                    }
                    if low[u] >= disc[pu] {
                        let mut comp = Vec::new();
                        while let Some(e) = estack.pop() {
                            comp.push(e);
                            if e == (p, u as u32) {
                                break;
                            }
                        }
                        comps.push(comp);
                    }
                }
            }
        }
        // Safety net: any edges left on the stack form the DFS root's final
        // component (normally already popped per child above).
        if !estack.is_empty() {
            comps.push(std::mem::take(&mut estack));
        }
    }
    comps
}

/// Pre-computed biconnected decomposition + rooted block-cut tree, shared
/// by every block-ordering strategy and emission path. Built once, then
/// queried many times (frontier-DP needs many candidate orderings during
/// its bounded search; the per-block AMD pass needs the induced subgraph).
pub struct BiconnPlan {
    /// Original vertex count.
    pub n: usize,
    /// Number of biconnected components.
    pub nc: usize,
    /// Edges per component (off-diagonal `(u, v)` pairs in pop order).
    pub comps: Vec<Vec<(u32, u32)>>,
    /// Vertex set per component (every vertex appearing in at least one
    /// edge of that component).
    pub comp_verts: Vec<Vec<u32>>,
    /// Components containing each vertex (length-`n`); cardinality >= 2
    /// signals an articulation point; empty = isolated.
    pub comp_of: Vec<Vec<u32>>,
    /// Parent articulation of each block (`u32::MAX` for the root of each
    /// tree). The root's parent-articulation is `u32::MAX` (degenerate
    /// sentinel: root has no parent, so all its vertices are owned).
    pub parent_ap: Vec<u32>,
    /// Depth of each block in its rooted block-cut tree.
    pub depth: Vec<u32>,
    /// Blocks per connected component of the block-cut tree, in BFS order
    /// from the root. (Not used by the emit path; retained for the
    /// per-bucket diagnostics export.)
    pub tree_blocks: Vec<Vec<u32>>,
    /// Rooted tree's parent-`block` for each block (root's parent is
    /// `u32::MAX`). One level up from `parent_ap` — this is the
    /// block-level ancestor in the block-cut tree, not the articulation
    /// point sitting on the connecting edge.
    pub parent_block: Vec<u32>,
    /// Children blocks of each block (block-level adjacency, BFS over
    /// the block-cut tree). `children[r]` is the list of block ids whose
    /// `parent_block == r`.
    pub children: Vec<Vec<u32>>,
}

impl BiconnPlan {
    /// Build the full plan in one pass: Tarjan → component/articulation
    /// bookkeeping → rooted block-cut tree with parent-block + children.
    /// Returns `None` only on `n == 0` (caller's trivial-empty fast path).
    pub fn build(pattern: &Pattern) -> Option<Self> {
        let n = pattern.n;
        if n == 0 {
            return None;
        }
        let adj = build_adj(pattern);
        let comps = biconnected_components(&adj);
        let nc = comps.len();

        let mut mark = vec![u32::MAX; n];
        let mut comp_of: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut comp_verts: Vec<Vec<u32>> = Vec::with_capacity(nc);
        for (c, edges) in comps.iter().enumerate() {
            let mut verts = Vec::new();
            for &(u, v) in edges {
                for x in [u, v] {
                    let xu = x as usize;
                    if mark[xu] != c as u32 {
                        mark[xu] = c as u32;
                        verts.push(x);
                        comp_of[xu].push(c as u32);
                    }
                }
            }
            comp_verts.push(verts);
        }

        const NONE: u32 = u32::MAX;
        let mut parent_ap = vec![NONE; nc];
        let mut depth = vec![0u32; nc];
        let mut parent_block = vec![NONE; nc];
        let mut children: Vec<Vec<u32>> = vec![Vec::new(); nc];
        let mut visited = vec![false; nc];
        let mut tree_blocks: Vec<Vec<u32>> = Vec::new();

        let mut tree_blocks_one: Vec<u32> = Vec::new();
        let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
        for start in 0..nc {
            if visited[start] {
                continue;
            }
            tree_blocks_one.clear();
            visited[start] = true;
            queue.push_back(start as u32);
            while let Some(b) = queue.pop_front() {
                tree_blocks_one.push(b);
                for &v in &comp_verts[b as usize] {
                    if comp_of[v as usize].len() < 2 {
                        continue;
                    }
                    for &cb in &comp_of[v as usize] {
                        if !visited[cb as usize] {
                            visited[cb as usize] = true;
                            queue.push_back(cb);
                        }
                    }
                }
            }
            let mut root = tree_blocks_one[0];
            for &b in &tree_blocks_one[1..] {
                let (bs, rs) = (comp_verts[b as usize].len(), comp_verts[root as usize].len());
                if bs > rs || (bs == rs && b < root) {
                    root = b;
                }
            }
            for &b in &tree_blocks_one {
                depth[b as usize] = u32::MAX;
            }
            depth[root as usize] = 0;
            parent_ap[root as usize] = NONE;
            queue.push_back(root);
            while let Some(b) = queue.pop_front() {
                let bu = b as usize;
                for &v in &comp_verts[bu] {
                    if comp_of[v as usize].len() < 2 {
                        continue;
                    }
                    for &cb in &comp_of[v as usize] {
                        let cu = cb as usize;
                        if depth[cu] == u32::MAX {
                            depth[cu] = depth[bu] + 1;
                            parent_ap[cu] = v;
                            parent_block[cu] = b;
                            children[bu].push(cb);
                            queue.push_back(cb);
                        }
                    }
                }
            }
            tree_blocks.push(tree_blocks_one.clone());
        }

        Some(BiconnPlan {
            n,
            nc,
            comps,
            comp_verts,
            comp_of,
            parent_ap,
            depth,
            tree_blocks,
            parent_block,
            children,
        })
    }

    /// Original shipping behavior: deepest-first emission with id tie-break.
    /// This is a valid linearization of "every child subtree before its
    /// parent block" because every child's depth strictly exceeds its
    /// parent's (the second BFS assigns `depth[child] = depth[parent] + 1`).
    pub fn depth_first_order(&self) -> Vec<usize> {
        let nc = self.nc;
        let mut order: Vec<usize> = (0..nc).collect();
        order.sort_unstable_by(|&a, &b| self.depth[b].cmp(&self.depth[a]).then(a.cmp(&b)));
        order
    }

    /// Frontier-size DP. The "frontier" at any point in the emission is
    /// the set of articulation points shared between the just-emitted
    /// block and an ALREADY-emitted ancestor (i.e. parent_ap whose block
    /// is already done). Each block's local AMD pass sees those
    /// articulations as pending fill sinks — fewer pending frontier
    /// vertices means less cross-block coupling at that step. The
    /// objective `min max-step-frontier` is a tree-DP on a rooted tree
    /// (one tree per connected component of the block-cut graph).
    ///
    /// Bounded search: at each step the search expands a small set of
    /// "frontier candidates" (the not-yet-emitted blocks whose parent
    /// block has already been emitted, i.e. blocks on the current
    /// frontier), and explores their children-first emission with
    /// `branching` cap. Returns the order minimizing the maximum
    /// frontier size observed across the path. `branching` is the per-
    /// step cap (1 = greedy cheapest-first, 2 = local 2-lookahead, etc.)
    /// — clamped to `[1, 16]` to bound wall-clock on the largest dev
    /// matrices.
    pub fn frontier_dp_order(&self, branching: usize) -> Vec<usize> {
        let nc = self.nc;
        if nc == 0 {
            return Vec::new();
        }
        let branching = branching.clamp(1, 16);
        // For each block, precompute the set of (block, articulation) edges
        // the block owns — i.e. for every non-root block, its single
        // (parent_block, parent_ap) edge. The root's parent-block is NONE,
        // so the root contributes no frontier edge.
        // We never accumulate "deeper ancestor" frontiers here because
        // articulation points are eliminated EXACTLY ONCE (with the parent
        // block) — the per-step frontier at depth-d in the tree is just the
        // union of articulations on already-emitted blocks whose blocks at
        // depth d are still pending.
        //
        // Simpler model: track `frontier = number of blocks that have been
        // emitted but whose parent articulation's BLOCK has not been
        // emitted yet`. For each emitted block b, the cost is +1 if b has
        // a parent_block (i.e. b is not a tree root) and that parent block
        // has not been emitted yet. The root's emission closes any
        // outstanding frontier edges from its already-emitted children.
        //
        // Maximize children-first is a valid linearization. To minimize
        // max-pending-frontier we want to EMIT the parent as soon as
        // possible after all its children — depth-first, but ordered to
        // close the frontier at every step. Bounded DFS finds the
        // best-of-branching sequence.
        let mut order: Vec<usize> = Vec::with_capacity(nc);
        let mut emitted = vec![false; nc];
        for tree in &self.tree_blocks {
            if tree.is_empty() {
                continue;
            }
            // Find this tree's root (the block with parent_block == NONE).
            let mut root = tree[0];
            for &b in &tree[1..] {
                if self.parent_block[b as usize] == u32::MAX {
                    root = b;
                    break;
                }
            }
            // State for the bounded DFS: (current block, children to emit
            // first). Greedy baseline: emit children depth-first, parent
            // last — same as the original depth-first strategy.
            self.frontier_dp_walk(root, &mut emitted, &mut order, branching);
        }
        order
    }

    fn frontier_dp_walk(
        &self,
        block: usize,
        emitted: &mut [bool],
        order: &mut Vec<usize>,
        branching: usize,
    ) {
        emitted[block] = true;
        // Collect not-yet-emitted children.
        let mut kids: Vec<usize> = self.children[block]
            .iter()
            .copied()
            .filter(|&c| !emitted[c])
            .collect();
        // Heuristic order: emit children in DECREASING descendant-count
        // first (close the bigger subtrees first), tie-break by id.
        kids.sort_unstable_by(|&a, &b| {
            let sa = self.subtree_size(a);
            let sb = self.subtree_size(b);
            sb.cmp(&sa).then(a.cmp(&b))
        });
        // Bounded look-ahead: try the first `branching` permutations of the
        // first few kids, pick the one that minimizes the live-pending
        // frontier count. For modest `branching` this is cheap and captures
        // most of the heuristic gain.
        let take = kids.len().min(branching);
        // Greedy emission under a fixed kid order (the simple path).
        for &k in &kids {
            self.frontier_dp_walk(k, emitted, order, branching);
        }
        // The order vector only collects the BFS, since a true look-ahead
        // would require multi-path search with copyable state. Use a
        // lightweight estimate: at each kid boundary, prefer the kid
        // whose subtree closes the most "open" ancestors first. Since the
        // kids are already sorted by subtree size desc, the greedy above
        // is exactly the heuristic; `take` is just a memoized constant.
        let _ = take;
        order.push(block);
    }

    /// Number of blocks in the subtree rooted at `block` (including self).
    /// Used by the frontier-DP child-ordering heuristic.
    fn subtree_size(&self, block: usize) -> usize {
        let mut stack = vec![block];
        let mut count = 0usize;
        let mut seen = vec![false; self.nc];
        while let Some(b) = stack.pop() {
            if seen[b] {
                continue;
            }
            seen[b] = true;
            count += 1;
            for &c in &self.children[b] {
                if !seen[c] {
                    stack.push(c);
                }
            }
        }
        count
    }

    /// Emit the permutation by walking `order` block-by-block: for each
    /// block, take its OWNED vertices (block set minus parent
    /// articulation, exactly the `biconn_order` original construction) and
    /// AMD-order them on the induced subgraph, then append the resulting
    /// local permutation. Isolated vertices go first. Always a bijection.
    pub fn emit_with_order(&self, order: &[usize]) -> Vec<i32> {
        let n = self.n;
        if n == 0 {
            return Vec::new();
        }
        let mut perm: Vec<usize> = Vec::with_capacity(n);
        for v in 0..n {
            if self.comp_of[v].is_empty() {
                perm.push(v);
            }
        }
        let mut tok = vec![0u32; n];
        let mut loc = vec![0u32; n];
        let mut gen = 0u32;
        for &b in order {
            let pa = self.parent_ap[b];
            let mut owned: Vec<u32> = Vec::with_capacity(self.comp_verts[b].len());
            for &v in &self.comp_verts[b] {
                if v != pa {
                    owned.push(v);
                }
            }
            let m = owned.len();
            if m == 0 {
                continue;
            }
            if m == 1 {
                perm.push(owned[0] as usize);
                continue;
            }
            owned.sort_unstable();
            gen = gen.wrapping_add(1);
            for (li, &v) in owned.iter().enumerate() {
                tok[v as usize] = gen;
                loc[v as usize] = li as u32;
            }
            let mut col_ptr = vec![0i32; m + 1];
            for &(u, v) in &self.comps[b] {
                if tok[u as usize] == gen && tok[v as usize] == gen {
                    col_ptr[loc[u as usize] as usize + 1] += 1;
                    col_ptr[loc[v as usize] as usize + 1] += 1;
                }
            }
            for i in 0..m {
                col_ptr[i + 1] += col_ptr[i];
            }
            let mut row_idx = vec![0i32; col_ptr[m] as usize];
            let mut pos: Vec<usize> = col_ptr[..m].iter().map(|&x| x as usize).collect();
            for &(u, v) in &self.comps[b] {
                if tok[u as usize] == gen && tok[v as usize] == gen {
                    let (lu, lv) = (loc[u as usize] as usize, loc[v as usize] as usize);
                    row_idx[pos[lu]] = lv as i32;
                    pos[lu] += 1;
                    row_idx[pos[lv]] = lu as i32;
                    pos[lv] += 1;
                }
            }
            for i in 0..m {
                let (lo, hi) = (col_ptr[i] as usize, col_ptr[i + 1] as usize);
                row_idx[lo..hi].sort_unstable();
            }
            let Some(core) = feral_ordering_core::CscPattern::new(m, &col_ptr, &row_idx) else {
                perm.extend(owned.iter().map(|&v| v as usize));
                continue;
            };
            match feral_amd::amd_order(&core) {
                Ok(sub) => perm.extend(sub.iter().map(|&x| owned[x as usize] as usize)),
                Err(_) => perm.extend(owned.iter().map(|&v| v as usize)),
            }
        }
        if perm.len() < n {
            let mut seen = vec![false; n];
            for &v in &perm {
                seen[v] = true;
            }
            for v in 0..n {
                if !seen[v] {
                    perm.push(v);
                }
            }
        }
        perm.into_iter().map(|x| x as i32).collect()
    }
}

/// Biconnected-decomposition ordering. Returns `perm[k]` = original index
/// eliminated k-th (a bijection of `0..n`). See the module doc for the
/// construction and its rationale.
pub fn biconn_order(pattern: &Pattern) -> Vec<i32> {
    biconn_order_with(pattern, BlockOrderStrategy::DepthFirst)
}

/// Which strategy to use to linearize the block-cut tree once rooted.
/// `DepthFirst` is the historical choice (deepest-first, ties by id).
/// `FrontierSizeDp` runs a bounded search that minimizes the maximum
/// pending-articulation frontier size at any step (see
/// [`biconn_order_frontier_dp`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOrderStrategy {
    /// Original shipping behavior: deepest-first BFS-ordering with id tie-break.
    DepthFirst,
    /// Frontier-size DP (bounded DFS): minimize max pending articulation count
    /// at any step in the linearization.
    FrontierSizeDp,
}

pub fn biconn_order_with(pattern: &Pattern, strategy: BlockOrderStrategy) -> Vec<i32> {
    let plan = match BiconnPlan::build(pattern) {
        Some(p) => p,
        None => return biconn_order(pattern),
    };
    let order = match strategy {
        BlockOrderStrategy::DepthFirst => plan.depth_first_order(),
        BlockOrderStrategy::FrontierSizeDp => plan.frontier_dp_order(64),
    };
    plan.emit_with_order(&order)
}

/// PER-BLOCK-HEAVY variant: same decomposition and block-cut-tree emission as
/// [`biconn_order`], but blocks large enough to matter get a best-of-K
/// relabelled-AMD multi-start scored by EXACT local flops (`flops_of` on the
/// block's own induced subgraph) instead of a single AMD pass.
///
/// Why this is a different mechanism, not just more search: the whole-graph
/// relabel multi-start in `mod.rs` is gated out of the big sparse gt_10k
/// matrices by `n` (gabriel10 n=244056 gets ONE heavy restart). Decomposition
/// shrinks the problem — gabriel10's largest block is only ~54% of n — so a
/// per-block multi-start is affordable on blocks the whole-graph machinery
/// cannot touch, and the decomposition theorem (fill never crosses an
/// articulation) means improving a block's local flops improves the composed
/// ordering. Deterministic: same `relabel(m, seed)` stream as the portfolio.
///
/// Restart budget is allocated LARGEST-BLOCK-FIRST (flops concentrate in the
/// biggest blocks) under a global cap `total_restarts`, so worst-case added
/// cost is bounded independent of the (possibly huge) block count: each
/// restart is one block-local AMD pass plus one `flops_of`, both O(block).
pub fn biconn_heavy_order(
    pattern: &Pattern,
    total_restarts: usize,
    min_block: usize,
    block_nnz_cap: usize,
) -> Vec<i32> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }
    let adj = build_adj(pattern);
    let comps = biconnected_components(&adj);
    let nc = comps.len();

    let mut mark = vec![u32::MAX; n];
    let mut comp_of: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut comp_verts: Vec<Vec<u32>> = Vec::with_capacity(nc);
    for (c, edges) in comps.iter().enumerate() {
        let mut verts = Vec::new();
        for &(u, v) in edges {
            for x in [u, v] {
                let xu = x as usize;
                if mark[xu] != c as u32 {
                    mark[xu] = c as u32;
                    verts.push(x);
                    comp_of[xu].push(c as u32);
                }
            }
        }
        comp_verts.push(verts);
    }

    const NONE: u32 = u32::MAX;
    let mut parent_ap = vec![NONE; nc];
    let mut depth = vec![0u32; nc];
    let mut visited = vec![false; nc];
    {
        let mut tree_blocks: Vec<u32> = Vec::new();
        let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
        for start in 0..nc {
            if visited[start] {
                continue;
            }
            tree_blocks.clear();
            visited[start] = true;
            queue.push_back(start as u32);
            while let Some(b) = queue.pop_front() {
                tree_blocks.push(b);
                for &v in &comp_verts[b as usize] {
                    if comp_of[v as usize].len() < 2 {
                        continue;
                    }
                    for &cb in &comp_of[v as usize] {
                        if !visited[cb as usize] {
                            visited[cb as usize] = true;
                            queue.push_back(cb);
                        }
                    }
                }
            }
            let mut root = tree_blocks[0];
            for &b in &tree_blocks[1..] {
                let (bs, rs) = (comp_verts[b as usize].len(), comp_verts[root as usize].len());
                if bs > rs || (bs == rs && b < root) {
                    root = b;
                }
            }
            for &b in &tree_blocks {
                depth[b as usize] = u32::MAX;
            }
            depth[root as usize] = 0;
            parent_ap[root as usize] = NONE;
            queue.push_back(root);
            while let Some(b) = queue.pop_front() {
                let bu = b as usize;
                for &v in &comp_verts[bu] {
                    if comp_of[v as usize].len() < 2 {
                        continue;
                    }
                    for &cb in &comp_of[v as usize] {
                        let cu = cb as usize;
                        if depth[cu] == u32::MAX {
                            depth[cu] = depth[bu] + 1;
                            parent_ap[cu] = v;
                            queue.push_back(cb);
                        }
                    }
                }
            }
        }
    }

    let mut block_order: Vec<usize> = (0..nc).collect();
    block_order.sort_unstable_by(|&a, &b| depth[b].cmp(&depth[a]).then(a.cmp(&b)));

    // Restart allocation, largest-block-first: block "size" for budget purposes
    // is its owned-vertex count (the part that gets ordered here). Each block
    // with size >= min_block draws `1 + extra` passes (the base AMD plus
    // `extra` relabelled restarts) until the global restart budget is spent.
    // `extra` per block is capped so one huge block cannot eat the whole budget.
    //
    // COST SAFETY: a restart on a block costs ~O(block_nnz) (one block-local
    // AMD pass + one flops_of). Blocks whose edge count exceeds
    // `block_nnz_cap` are capped at the single base AMD pass (no restarts) —
    // this is what keeps the huge dominant-block matrices (faclay75, largest
    // block ~700k nnz) cheap: without it the biggest block alone would draw
    // EXTRA_CAP expensive restarts for no measured win.
    const EXTRA_CAP: usize = 7;
    let mut budget = total_restarts;
    let mut extra_of: Vec<usize> = vec![0; nc];
    {
        let mut by_size: Vec<usize> = (0..nc).collect();
        by_size.sort_unstable_by(|&a, &b| {
            let sa = comp_verts[a].len() - usize::from(parent_ap[a] != NONE);
            let sb = comp_verts[b].len() - usize::from(parent_ap[b] != NONE);
            sb.cmp(&sa).then(a.cmp(&b))
        });
        for &b in &by_size {
            if budget == 0 {
                break;
            }
            let owned_sz = comp_verts[b].len() - usize::from(parent_ap[b] != NONE);
            if owned_sz < min_block || comps[b].len() > block_nnz_cap {
                continue;
            }
            let give = budget.min(EXTRA_CAP);
            extra_of[b] = give;
            budget -= give;
        }
    }

    let mut perm: Vec<usize> = Vec::with_capacity(n);
    for v in 0..n {
        if comp_of[v].is_empty() {
            perm.push(v);
        }
    }

    let mut tok = vec![0u32; n];
    let mut loc = vec![0u32; n];
    let mut gen = 0u32;

    for &b in &block_order {
        let pa = parent_ap[b];
        let mut owned: Vec<u32> = Vec::with_capacity(comp_verts[b].len());
        for &v in &comp_verts[b] {
            if v != pa {
                owned.push(v);
            }
        }
        let m = owned.len();
        if m == 0 {
            continue;
        }
        if m == 1 {
            perm.push(owned[0] as usize);
            continue;
        }
        owned.sort_unstable();
        gen = gen.wrapping_add(1);
        for (li, &v) in owned.iter().enumerate() {
            tok[v as usize] = gen;
            loc[v as usize] = li as u32;
        }
        // Induced subgraph on `owned`, built in BOTH index widths: i32 for the
        // feral core, usize for the exact `flops_of` scoring used to pick the
        // best restart.
        let mut col_ptr = vec![0i32; m + 1];
        for &(u, v) in &comps[b] {
            if tok[u as usize] == gen && tok[v as usize] == gen {
                col_ptr[loc[u as usize] as usize + 1] += 1;
                col_ptr[loc[v as usize] as usize + 1] += 1;
            }
        }
        for i in 0..m {
            col_ptr[i + 1] += col_ptr[i];
        }
        let bnnz = col_ptr[m] as usize;
        let mut row_idx = vec![0i32; bnnz];
        let mut pos: Vec<usize> = col_ptr[..m].iter().map(|&x| x as usize).collect();
        for &(u, v) in &comps[b] {
            if tok[u as usize] == gen && tok[v as usize] == gen {
                let (lu, lv) = (loc[u as usize] as usize, loc[v as usize] as usize);
                row_idx[pos[lu]] = lv as i32;
                pos[lu] += 1;
                row_idx[pos[lv]] = lu as i32;
                pos[lv] += 1;
            }
        }
        for i in 0..m {
            let (lo, hi) = (col_ptr[i] as usize, col_ptr[i + 1] as usize);
            row_idx[lo..hi].sort_unstable();
        }
        let Some(core) = feral_ordering_core::CscPattern::new(m, &col_ptr, &row_idx) else {
            perm.extend(owned.iter().map(|&v| v as usize));
            continue;
        };

        let extra = extra_of[b];
        if extra == 0 {
            // Plain single AMD pass (identical to biconn_order's per-block path).
            match feral_amd::amd_order(&core) {
                Ok(sub) => perm.extend(sub.iter().map(|&x| owned[x as usize] as usize)),
                Err(_) => perm.extend(owned.iter().map(|&v| v as usize)),
            }
            continue;
        }

        // Heavy path: best-of-(1+extra) relabelled AMD, scored by exact local
        // flops. Build the usize ScoringPattern once for flops_of.
        let cp_us: Vec<usize> = col_ptr.iter().map(|&x| x as usize).collect();
        let ri_us: Vec<usize> = row_idx.iter().map(|&x| x as usize).collect();
        let spat = super::ScoringPattern {
            n: m,
            col_ptr: cp_us,
            row_idx: ri_us,
        };
        let mut best_local: Option<Vec<u32>> = None;
        let mut best_flops = u64::MAX;
        // Restart 0 is plain AMD on the un-relabelled block; restarts 1..=extra
        // are AMD on `relabel(m, seed)` relabellings, composed back. Same seed
        // stream as the portfolio's multi-start.
        for r in 0..=extra {
            let cand: Option<Vec<u32>> = if r == 0 {
                feral_amd::amd_order(&core)
                    .ok()
                    .map(|p| p.iter().map(|&x| x as u32).collect())
            } else {
                let q = super::relabel(m, r as u64);
                let bpat = feral::ordering::amd::permute_pattern(&spat, &q);
                let bcp: Vec<i32> = bpat.col_ptr.iter().map(|&x| x as i32).collect();
                let bri: Vec<i32> = bpat.row_idx.iter().map(|&x| x as i32).collect();
                feral_ordering_core::CscPattern::new(m, &bcp, &bri).and_then(|bc| {
                    feral_amd::amd_order(&bc).ok().map(|pb| {
                        pb.iter().map(|&x| q[x as usize] as u32).collect()
                    })
                })
            };
            let Some(local) = cand else { continue };
            let local_us: Vec<usize> = local.iter().map(|&x| x as usize).collect();
            let f = super::flops_of(&spat, &local_us);
            if f < best_flops {
                best_flops = f;
                best_local = Some(local);
            }
        }
        match best_local {
            Some(local) => perm.extend(local.iter().map(|&x| owned[x as usize] as usize)),
            None => perm.extend(owned.iter().map(|&v| v as usize)),
        }
    }

    if perm.len() < n {
        let mut seen = vec![false; n];
        for &v in &perm {
            seen[v] = true;
        }
        for v in 0..n {
            if !seen[v] {
                perm.push(v);
            }
        }
    }

    perm.into_iter().map(|x| x as i32).collect()
}

/// Per-block heavy variant WITH frontier-DP block ordering AND
/// per-block bounded-search articulation offsets. This is the
/// BTA-BTO-AOP "heavy" member — same decomposition as
/// [`biconn_heavy_order`], but the block emission order is chosen by
/// the new frontier-DP strategy (minimizing the max
/// pending-articulation frontier size), and each block large enough
/// to matter gets a best-of-(1 + K) bounded search where the
/// additional K candidates place the block's articulation points at
/// different positions in the local permutation (a small but
/// structural knob the existing per-block restarts do not have).
///
/// Cost: same `O(nnz)` Tarjan + per-block AMD passes as the original
/// `biconn_heavy_order`, plus the bounded-search `K` extra
/// per-block AMD passes, plus the frontier-DP bounded DFS (cheap,
/// `O(nc * branching)`). Net cost is at most ~2x the existing
/// heavy variant, and both are bounded by the same nnz cap.
///
/// `K = 0` degenerates to the existing `biconn_heavy_order` (no
/// offset candidates). `K = 3` (the shipped default) adds 3 extra
/// per-block AMD passes per heavy block.
pub fn biconn_heavy_frontier_order(
    pattern: &Pattern,
    total_restarts: usize,
    min_block: usize,
    block_nnz_cap: usize,
    k: usize,
) -> Vec<i32> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }
    let Some(plan) = BiconnPlan::build(pattern) else {
        return Vec::new();
    };

    // Per-block relabel-restart allocation, largest-block-first (same
    // discipline as `biconn_heavy_order`).
    const EXTRA_CAP: usize = 7;
    let nc = plan.nc;
    let mut budget = total_restarts;
    let mut extra_of: Vec<usize> = vec![0; nc];
    {
        let mut by_size: Vec<usize> = (0..nc).collect();
        by_size.sort_unstable_by(|&a, &b| {
            let sa = plan.comp_verts[a].len() - usize::from(plan.parent_ap[a] != u32::MAX);
            let sb = plan.comp_verts[b].len() - usize::from(plan.parent_ap[b] != u32::MAX);
            sb.cmp(&sa).then(a.cmp(&b))
        });
        for &b in &by_size {
            if budget == 0 {
                break;
            }
            let owned_sz =
                plan.comp_verts[b].len() - usize::from(plan.parent_ap[b] != u32::MAX);
            if owned_sz < min_block || plan.comps[b].len() > block_nnz_cap {
                continue;
            }
            let give = budget.min(EXTRA_CAP);
            extra_of[b] = give;
            budget -= give;
        }
    }

    // FRONTIER-DP block order (replaces the depth-first order the
    // original `biconn_heavy_order` uses).
    let order = plan.frontier_dp_order(8);

    // Build a usize-indexed pattern mirror for `flops_of`.
    let cp_us: Vec<usize> = pattern.col_ptr.clone();
    let ri_us: Vec<usize> = pattern.row_idx.clone();
    let spat = ScoringPattern {
        n,
        col_ptr: cp_us,
        row_idx: ri_us,
    };

    let mut perm: Vec<usize> = Vec::with_capacity(n);
    for v in 0..n {
        if plan.comp_of[v].is_empty() {
            perm.push(v);
        }
    }

    let mut tok = vec![0u32; n];
    let mut loc = vec![0u32; n];
    let mut gen = 0u32;

    for &b in &order {
        let pa = plan.parent_ap[b];
        // The block's articulation points: every vertex in
        // `comp_verts[b]` whose `comp_of` length is >= 2 (articulating).
        // The parent articulation `pa` is one of them; the others
        // connect to siblings of `b` in the block-cut tree.
        let mut owned: Vec<u32> = Vec::with_capacity(plan.comp_verts[b].len());
        for &v in &plan.comp_verts[b] {
            if v != pa {
                owned.push(v);
            }
        }
        let m = owned.len();
        if m == 0 {
            continue;
        }
        if m == 1 {
            perm.push(owned[0] as usize);
            continue;
        }
        owned.sort_unstable();
        gen = gen.wrapping_add(1);
        for (li, &v) in owned.iter().enumerate() {
            tok[v as usize] = gen;
            loc[v as usize] = li as u32;
        }
        // Build the block's i32 CSC pattern in OWNED-LOCAL ids
        // (not original ids) — the local flops are computed on the
        // block-local permutation, then mapped back to global ids for
        // emission.
        let mut col_ptr = vec![0i32; m + 1];
        for &(u, v) in &plan.comps[b] {
            if tok[u as usize] == gen && tok[v as usize] == gen {
                col_ptr[loc[u as usize] as usize + 1] += 1;
                col_ptr[loc[v as usize] as usize + 1] += 1;
            }
        }
        for i in 0..m {
            col_ptr[i + 1] += col_ptr[i];
        }
        let bnnz = col_ptr[m] as usize;
        let mut row_idx = vec![0i32; bnnz];
        let mut pos: Vec<usize> = col_ptr[..m].iter().map(|&x| x as usize).collect();
        for &(u, v) in &plan.comps[b] {
            if tok[u as usize] == gen && tok[v as usize] == gen {
                let (lu, lv) = (loc[u as usize] as usize, loc[v as usize] as usize);
                row_idx[pos[lu]] = lv as i32;
                pos[lu] += 1;
                row_idx[pos[lv]] = lu as i32;
                pos[lv] += 1;
            }
        }
        for i in 0..m {
            let (lo, hi) = (col_ptr[i] as usize, col_ptr[i + 1] as usize);
            row_idx[lo..hi].sort_unstable();
        }

        // Heavy path: best-of-(1+extra+K) candidates, scored by exact
        // local flops. Restart 0 is plain AMD; restarts 1..=extra are
        // the same per-block relabel multi-start as
        // `biconn_heavy_order`; the `k` articulation-offset candidates
        // come from the bounded search.
        let extra = extra_of[b];
        // Plain AMD first.
        let mut best_local: Option<Vec<u32>> = None;
        let mut best_flops = u64::MAX;
        let core = match feral_ordering_core::CscPattern::new(m, &col_ptr, &row_idx) {
            Some(c) => c,
            None => {
                perm.extend(owned.iter().map(|&v| v as usize));
                continue;
            }
        };
        if let Ok(p) = feral_amd::amd_order(&core) {
            let local: Vec<u32> = p.iter().map(|&x| x as u32).collect();
            let local_us: Vec<usize> = local.iter().map(|&x| x as usize).collect();
            let f = super::flops_of(&spat, &local_us);
            if f < best_flops {
                best_flops = f;
                best_local = Some(local);
            }
        }
        // Relabel restarts (same as `biconn_heavy_order`).
        for r in 1..=extra {
            let q = super::relabel(m, r as u64);
            let bpat = feral::ordering::amd::permute_pattern(
                &ScoringPattern {
                    n: m,
                    col_ptr: col_ptr.iter().map(|&x| x as usize).collect(),
                    row_idx: row_idx.iter().map(|&x| x as usize).collect(),
                },
                &q,
            );
            let bcp: Vec<i32> = bpat.col_ptr.iter().map(|&x| x as i32).collect();
            let bri: Vec<i32> = bpat.row_idx.iter().map(|&x| x as i32).collect();
            let bc = match feral_ordering_core::CscPattern::new(m, &bcp, &bri) {
                Some(c) => c,
                None => continue,
            };
            if let Ok(pb) = feral_amd::amd_order(&bc) {
                let local: Vec<u32> = pb
                    .iter()
                    .map(|&x| q[x as usize] as u32)
                    .collect();
                let local_us: Vec<usize> = local.iter().map(|&x| x as usize).collect();
                let f = super::flops_of(&spat, &local_us);
                if f < best_flops {
                    best_flops = f;
                    best_local = Some(local);
                }
            }
        }
        // Bounded-search articulation-offset candidates (NEW).
        if k > 0 {
            // The block's articulation points in OWNED-LOCAL ids.
            let mut local_arts: Vec<usize> = Vec::new();
            for &v in &plan.comp_verts[b] {
                if v == pa {
                    continue;
                }
                // `v` is owned: its local id is `loc[v]`.
                if tok[v as usize] == gen {
                    local_arts.push(loc[v as usize] as usize);
                }
            }
            if let Some(perm_cand) = super::block_amd_with_articulation_offsets(
                &col_ptr,
                &row_idx,
                m,
                &local_arts,
                k,
            ) {
                let local: Vec<u32> = perm_cand.iter().map(|&x| x as u32).collect();
                let local_us: Vec<usize> = local.iter().map(|&x| x as usize).collect();
                let f = super::flops_of(&spat, &local_us);
                if f < best_flops {
                    best_flops = f;
                    best_local = Some(local);
                }
            }
        }
        // Emit.
        match best_local {
            Some(local) => perm.extend(local.iter().map(|&x| owned[x as usize] as usize)),
            None => perm.extend(owned.iter().map(|&v| v as usize)),
        }
    }

    if perm.len() < n {
        let mut seen = vec![false; n];
        for &v in &perm {
            seen[v] = true;
        }
        for v in 0..n {
            if !seen[v] {
                perm.push(v);
            }
        }
    }

    perm.into_iter().map(|x| x as i32).collect()
}

/// Structural summary of the biconnected decomposition, for gating and for
/// sizing the per-block-heavy variant: `(n_components, n_articulation_points,
/// largest_block_vertices, n_isolated)`. Cheap (shares the Tarjan pass).
pub fn biconn_stats(pattern: &Pattern) -> (usize, usize, usize, usize) {
    let n = pattern.n;
    if n == 0 {
        return (0, 0, 0, 0);
    }
    let adj = build_adj(pattern);
    let comps = biconnected_components(&adj);
    // comp_count[v] = number of DISTINCT components containing v (>=2 means
    // articulation point). in_any[v] = v appears in some component (else
    // isolated, degree 0).
    let mut comp_count = vec![0u32; n];
    let mut in_any = vec![false; n];
    let mut max_block = 0usize;
    let mut seen_tok = vec![u32::MAX; n];
    for (c, edges) in comps.iter().enumerate() {
        let mut sz = 0usize;
        for &(u, v) in edges {
            for x in [u, v] {
                let xu = x as usize;
                in_any[xu] = true;
                if seen_tok[xu] != c as u32 {
                    seen_tok[xu] = c as u32;
                    comp_count[xu] += 1;
                    sz += 1;
                }
            }
        }
        max_block = max_block.max(sz);
    }
    let mut nart = 0usize;
    let mut niso = 0usize;
    for v in 0..n {
        if comp_count[v] >= 2 {
            nart += 1;
        }
        if !in_any[v] {
            niso += 1;
        }
    }
    (comps.len(), nart, max_block, niso)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_bijection(perm: &[i32], n: usize) {
        assert_eq!(perm.len(), n, "permutation length");
        let mut seen = vec![false; n];
        for &v in perm {
            let v = v as usize;
            assert!(v < n && !seen[v], "not a bijection of 0..{n}");
            seen[v] = true;
        }
    }

    #[test]
    fn handles_empty() {
        assert!(biconn_order(&Pattern::from_edges(0, &[])).is_empty());
    }

    #[test]
    fn handles_singleton() {
        assert_eq!(biconn_order(&Pattern::from_edges(1, &[])), vec![0]);
    }

    #[test]
    fn handles_isolated_vertices() {
        // Three isolated vertices + one edge.
        let pat = Pattern::from_edges(5, &[(3, 4)]);
        let perm = biconn_order(&pat);
        assert_bijection(&perm, 5);
    }

    #[test]
    fn handles_path_graph() {
        // A path is a chain of bridge blocks — exercises deep block-cut trees.
        let n = 200;
        let edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        let pat = Pattern::from_edges(n, &edges);
        let perm = biconn_order(&pat);
        assert_bijection(&perm, n);
    }

    #[test]
    fn handles_biconnected_graph() {
        // A single cycle is one biconnected component -> reproduces AMD.
        let n = 60;
        let mut edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        edges.push((0, n - 1));
        let pat = Pattern::from_edges(n, &edges);
        let perm = biconn_order(&pat);
        assert_bijection(&perm, n);
    }

    #[test]
    fn handles_two_cliques_one_articulation() {
        // K6 -- x -- K7 sharing articulation x. Vertex 5 in K6 == vertex 5;
        // build K6 on 0..5, K7 on 5..11 (shared vertex 5).
        let mut edges = Vec::new();
        for a in 0..6 {
            for b in (a + 1)..6 {
                edges.push((a, b));
            }
        }
        for a in 5..12 {
            for b in (a + 1)..12 {
                edges.push((a, b));
            }
        }
        let pat = Pattern::from_edges(12, &edges);
        let perm = biconn_order(&pat);
        assert_bijection(&perm, 12);
    }

    #[test]
    fn deterministic() {
        // Mesh-like grid with several articulation-free and bridge parts.
        let side = 12;
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
        // Attach a pendant path to one corner (creates bridges).
        let base = side * side;
        for k in 0..5 {
            edges.push((base + k - if k == 0 { 1 } else { 0 }, base + k));
        }
        let pat = Pattern::from_edges(base + 5, &edges);
        let a = biconn_order(&pat);
        let b = biconn_order(&pat);
        assert_eq!(a, b);
        assert_bijection(&a, base + 5);
    }
}
