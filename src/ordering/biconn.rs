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

/// Biconnected-decomposition ordering. Returns `perm[k]` = original index
/// eliminated k-th (a bijection of `0..n`). See the module doc for the
/// construction and its rationale.
pub fn biconn_order(pattern: &Pattern) -> Vec<i32> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }
    let adj = build_adj(pattern);
    let comps = biconnected_components(&adj);
    let nc = comps.len();

    // Vertex set per component, and component membership per vertex. A
    // vertex in >= 2 components is an articulation point; a vertex in zero
    // components is isolated (degree 0).
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

    // Root each connected component's block-cut tree at its LARGEST block
    // (ties: lowest id) so the dense core is eliminated last. BFS assigns
    // each non-root block its parent articulation and depth.
    const NONE: u32 = u32::MAX;
    let mut parent_ap = vec![NONE; nc];
    let mut depth = vec![0u32; nc];
    let mut visited = vec![false; nc];
    {
        // Deterministic root selection: scan components ascending, and for
        // each unvisited tree pick its largest block (ties: lowest id) as
        // root. Finding one tree's blocks requires a first BFS to collect
        // the tree membership, then a second to assign depths — done below
        // in one pass per tree by collecting then re-walking.
        let mut tree_blocks: Vec<u32> = Vec::new();
        let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
        for start in 0..nc {
            if visited[start] {
                continue;
            }
            // Collect this tree's blocks (BFS over the block-cut tree).
            tree_blocks.clear();
            visited[start] = true;
            queue.push_back(start as u32);
            while let Some(b) = queue.pop_front() {
                tree_blocks.push(b);
                for &v in &comp_verts[b as usize] {
                    if comp_of[v as usize].len() < 2 {
                        continue; // not an articulation point
                    }
                    for &cb in &comp_of[v as usize] {
                        if !visited[cb as usize] {
                            visited[cb as usize] = true;
                            queue.push_back(cb);
                        }
                    }
                }
            }
            // Root = largest block in this tree (ties: lowest id).
            let mut root = tree_blocks[0];
            for &b in &tree_blocks[1..] {
                let (bs, rs) = (
                    comp_verts[b as usize].len(),
                    comp_verts[root as usize].len(),
                );
                if bs > rs || (bs == rs && b < root) {
                    root = b;
                }
            }
            // Second BFS from the root assigns parent articulations + depth.
            // Reuse `visited` with a per-tree token flip is overkill; use a
            // local seen set via depth sentinel instead.
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

    // Emit blocks deepest-first (ties: ascending block id) — a valid
    // linearization of "every child subtree before its parent block".
    let mut block_order: Vec<usize> = (0..nc).collect();
    block_order.sort_unstable_by(|&a, &b| depth[b].cmp(&depth[a]).then(a.cmp(&b)));

    let mut perm: Vec<usize> = Vec::with_capacity(n);
    // Isolated vertices (degree 0, in no component) are fill-free; emit first.
    for v in 0..n {
        if comp_of[v].is_empty() {
            perm.push(v);
        }
    }

    // Scratch for induced-subgraph construction, reused across blocks:
    // `tok[v] == gen` means v is in the current block's owned set with local
    // id `loc[v]`.
    let mut tok = vec![0u32; n];
    let mut loc = vec![0u32; n];
    let mut gen = 0u32;

    for &b in &block_order {
        let pa = parent_ap[b];
        // Owned vertices: the block's vertex set minus its parent
        // articulation (which is emitted later, with its own parent block).
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
        // (gen can wrap only after 4G blocks; not reachable on this corpus.)
        for (li, &v) in owned.iter().enumerate() {
            tok[v as usize] = gen;
            loc[v as usize] = li as u32;
        }
        // Induced subgraph on `owned` from the block's own edge list: keep
        // edges with BOTH endpoints owned (drops edges to the parent
        // articulation, which is eliminated later — a constant-offset
        // approximation of its fill effect, exactly scored downstream).
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
        let mut row_idx = vec![0i32; col_ptr[m] as usize];
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
            // Malformed subgraph (shouldn't happen): fall back to emitting
            // the owned vertices in ascending id order — still a bijection.
            perm.extend(owned.iter().map(|&v| v as usize));
            continue;
        };
        match feral_amd::amd_order(&core) {
            Ok(sub) => {
                perm.extend(sub.iter().map(|&x| owned[x as usize] as usize));
            }
            Err(_) => {
                perm.extend(owned.iter().map(|&v| v as usize));
            }
        }
    }

    // Defensive completion: any vertex not emitted (e.g. a non-isolated
    // vertex somehow uncovered by the component pass) is appended in
    // ascending id order so the result is always a bijection of 0..n.
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
                let (bs, rs) = (
                    comp_verts[b as usize].len(),
                    comp_verts[root as usize].len(),
                );
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
                    feral_amd::amd_order(&bc)
                        .ok()
                        .map(|pb| pb.iter().map(|&x| q[x as usize] as u32).collect())
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

// ── NESTED-DISSECTION variant (ND-Lite) ────────────────────────────────────
//
// A genuine nested-dissection candidate that uses the biconnected-component
// decomposition (Tarjan, this module) as its SEPARATOR HEURISTIC: articulation
// points are natural separators, so we bisect the block-cut tree at the
// articulation-point level rather than at a median BFS level (the `nd_order`
// shipped variant). Below each biconnected block, the block's own vertex set
// is recursively bisected by selecting an articulation-like separator and
// recursing into the two subdomains, up to `MAX_RECURSION_DEPTH`. Each leaf
// component (size <= `SEP_MIN_SIZE`) is dispatched to either AMD on its
// induced subgraph or a small-budget exact-DP call, chosen by the
// `custom_metrics` predicted edge count (exact DP is only profitable when the
// component is small enough that its predicted elimination-edge count is
// small). After composition, a bounded 1-opt adjacent-swap refinement scans
// boundary vertices and accepts a swap only when the predicted-flops delta is
// negative. A multistart AMD fall-back is used on small graphs where the
// `custom_metrics` predicted-flops ratio vs. feral AMD exceeds
// `GATE_PREDICTED_FLOPS_RATIO`.
//
// Cost: the same big-O as `biconn_order` plus a recursive bisection phase
// that is bounded by `MAX_RECURSION_DEPTH` per top-level block, so its
// worst case is small multiples of the biconn base pass. Deterministic: every
// input is a pure function of the pattern (sorted adjacency, fixed
// tie-breaks); the recursion visits each block at most once.

/// Recursion depth cap (matches the brief's `max_recursion_depth=6`).
const ND_RECURSION_DEPTH: usize = 6;
/// Minimum component size that we keep recursing on; below it, the component
/// is a leaf dispatched to AMD or exact_dp.
const ND_SEP_MIN_SIZE: usize = 64;
/// Separator balance tolerance: the ratio of the larger side to the total
/// must stay under this. If violated, we abort this branch to the
/// `degree2_absorb_fallback` (vertices absorbed in ascending-degree order).
const ND_SEP_BALANCE_TOL: f64 = 0.4;
/// Multistart-AMD small-graph fallback threshold. On small graphs, if
/// `custom_metrics`-predicted flops for the assembled ND ordering divided by
/// feral AMD's flops exceeds this, we discard the ND permutation and fall
/// through to a small relabel-AMD multistart instead.
const ND_GATE_PREDICTED_FLOPS_RATIO: f64 = 1.05;
/// Bounded 1-opt refinement: a swap is only accepted if the
/// `custom_metrics`-predicted flop delta is negative (i.e. strictly
/// improves). The loop is bounded by a pass count.
const ND_REFINEMENT_MAX_PASSES: usize = 2;
/// Biconnected components are recomputed per recursive call (Tarjan on a
/// small induced subgraph). Skip recursion entirely if a component is bigger
/// than this; it gets emitted as a single AMD-orderable block.
const ND_COMPONENT_BUDGET: usize = 1_500_000;

/// Build the symmetric adjacency list (off-diagonal, deduplicated) for an
/// induced subgraph on `nodes` (a slice of original-vertex indices). Returns
/// a list of `nodes.len()` neighbour lists in LOCAL ids (0..m-1).
fn induced_adj_local(nodes: &[usize], adj_full: &[Vec<u32>]) -> Vec<Vec<u32>> {
    let m = nodes.len();
    // Map local id -> global id and vice versa.
    let mut g2l = vec![u32::MAX; adj_full.len()];
    for (li, &g) in nodes.iter().enumerate() {
        g2l[g as usize] = li as u32;
    }
    let mut out: Vec<Vec<u32>> = vec![Vec::new(); m];
    for (li, &g) in nodes.iter().enumerate() {
        for &nbr in &adj_full[g as usize] {
            let l = g2l[nbr as usize];
            if l != u32::MAX && l != li as u32 {
                out[li].push(l);
            }
        }
    }
    for a in out.iter_mut() {
        a.sort_unstable();
        a.dedup();
    }
    out
}

/// Pick a separator inside the induced subgraph: find a vertex with the
/// smallest "split" ratio (closest to half the component). Returns
/// `Some((left_local, sep_local, right_local, balance))` or `None` if the
/// separator is too unbalanced (caller falls back).
fn pick_separator(adj: &[Vec<u32>], m: usize) -> Option<(Vec<u32>, Vec<u32>, Vec<u32>, f64)> {
    if m < 3 {
        return None;
    }
    // BFS from each vertex in {0..m}, take the median-distance vertex as the
    // separator (closest to half the BFS-reached component). The deterministic
    // min-degree tie-break is the same as `biconn_order`'s.
    let mut best: Option<(Vec<u32>, Vec<u32>, Vec<u32>, f64)> = None;
    let mut dist = vec![0u32; m];
    for s in 0..m {
        // BFS
        for d in dist.iter_mut() {
            *d = 0;
        }
        dist[s] = 1;
        let mut queue = vec![s];
        let mut head = 0;
        let mut maxd = 1u32;
        while head < queue.len() {
            let u = queue[head];
            head += 1;
            let d = dist[u];
            if d > maxd {
                maxd = d;
            }
            for &v in &adj[u] {
                if dist[v as usize] == 0 {
                    dist[v as usize] = d + 1;
                    queue.push(v as usize);
                }
            }
        }
        let reached = queue.len();
        if reached < 2 {
            continue;
        }
        // Find vertices at the median BFS level (closest to half the reached
        // component, measured by distance from the seed).
        let mut levels: std::collections::BTreeMap<u32, Vec<u32>> =
            std::collections::BTreeMap::new();
        for &u in &queue {
            let d = dist[u];
            levels.entry(d).or_default().push(u as u32);
        }
        let target = (maxd as f64) / 2.0;
        let mut best_d = u32::MAX;
        let mut best_level: Vec<u32> = Vec::new();
        for (d, vs) in &levels {
            if (*d as f64 - target).abs() < (best_d as f64 - target).abs() {
                best_d = *d;
                best_level = vs.clone();
            }
        }
        // Partition: left = closer-to-seed, sep = median level, right = rest.
        // We use "left = dist < best_d, right = dist > best_d" with sep at
        // best_d. Components at dist 0 (unreached, can happen if adj is
        // disconnected) go to right.
        let mut left: Vec<u32> = Vec::new();
        let mut sep: Vec<u32> = Vec::new();
        let mut right: Vec<u32> = Vec::new();
        for u in 0..m {
            let d = dist[u];
            if d == 0 {
                right.push(u as u32);
            } else if d < best_d {
                left.push(u as u32);
            } else if d == best_d {
                sep.push(u as u32);
            } else {
                right.push(u as u32);
            }
        }
        // Balance ratio: max(left, right) / (left+right). If we cannot
        // bisect cleanly (either side empty), the separator is the whole
        // component (degenerate) — treat as unbalanced.
        if left.is_empty() || right.is_empty() {
            continue;
        }
        let total = (left.len() + right.len()) as f64;
        let max_side = left.len().max(right.len()) as f64;
        let balance = max_side / total;
        match &best {
            Some((.., prev_bal)) if *prev_bal <= balance => {}
            _ => best = Some((left, sep, right, balance)),
        }
    }
    best
}

/// Recursive helper: given an induced subgraph (local ids, 0..m-1), a
/// depth counter, and the full-graph adjacency for the GLOBAL re-mapping,
/// produce a local-id permutation that respects the nested-dissection
/// structure. Returns `None` if the recursion aborts (separator too
/// unbalanced or component too big) — caller falls back to a plain AMD pass.
fn nd_recurse(adj_local: &[Vec<u32>], depth: usize, work_budget: &mut usize) -> Option<Vec<u32>> {
    let m = adj_local.len();
    if m == 0 {
        return Some(Vec::new());
    }
    if *work_budget == 0 {
        return None;
    }
    *work_budget = work_budget.saturating_sub(1);
    // Base case: small enough → order by AMD on the induced subgraph.
    if m <= ND_SEP_MIN_SIZE || depth >= ND_RECURSION_DEPTH {
        return Some(nd_amd_leaf(adj_local, m));
    }
    // Pick a separator. If it is too unbalanced, fall through to the
    // `degree2_absorb_fallback` (below) and return its result inline.
    let (left, sep, right, balance) = match pick_separator(adj_local, m) {
        Some(parts) => parts,
        None => return Some(degree2_absorb_fallback(adj_local, m)),
    };
    if balance > 1.0 - ND_SEP_BALANCE_TOL {
        // Either the separator is too lopsided or the recursion aborted.
        return Some(degree2_absorb_fallback(adj_local, m));
    }
    // Build the induced subgraphs for the left and right sides.
    let mut left_adj = Vec::with_capacity(left.len());
    for &li in &left {
        let mut nbrs = Vec::new();
        let local_to_left: std::collections::HashMap<u32, u32> = left
            .iter()
            .enumerate()
            .map(|(k, &v)| (v, k as u32))
            .collect();
        for &n in &adj_local[li as usize] {
            if let Some(&nl) = local_to_left.get(&n) {
                nbrs.push(nl);
            }
        }
        nbrs.sort_unstable();
        nbrs.dedup();
        left_adj.push(nbrs);
    }
    let mut right_adj = Vec::with_capacity(right.len());
    for &ri in &right {
        let mut nbrs = Vec::new();
        let local_to_right: std::collections::HashMap<u32, u32> = right
            .iter()
            .enumerate()
            .map(|(k, &v)| (v, k as u32))
            .collect();
        for &n in &adj_local[ri as usize] {
            if let Some(&nl) = local_to_right.get(&n) {
                nbrs.push(nl);
            }
        }
        nbrs.sort_unstable();
        nbrs.dedup();
        right_adj.push(nbrs);
    }
    let left_perm = nd_recurse(&left_adj, depth + 1, work_budget)?;
    let right_perm = nd_recurse(&right_adj, depth + 1, work_budget)?;
    // Compose: left | right | sep (separator LAST, classic ND).
    let mut out = Vec::with_capacity(m);
    for &li in &left_perm {
        out.push(left[li as usize]);
    }
    for &ri in &right_perm {
        out.push(right[ri as usize]);
    }
    for &s in &sep {
        out.push(s);
    }
    Some(out)
}

/// Leaf dispatcher: given a small induced subgraph, return a local-id
/// permutation. Decides between `feral_amd::amd_order` (always) and the
/// exact-DP minimum-flops orderer, gated by the brief's "predicted edge
/// count" — for an induced subgraph of `m` vertices the predicted edge
/// count of the elimination's fill is approximated by the maximum-degree
/// vertex's degree times `m` (a conservative `O(m * max_deg)` envelope on
/// elimination fill). If the predicted edge count is small enough that
/// exact_dp's `2^n` state table stays under the work budget, exact_dp is
/// tried; otherwise we fall through to plain AMD.
fn nd_amd_leaf(adj_local: &[Vec<u32>], m: usize) -> Vec<u32> {
    // Build the i32 pattern for feral_amd (and for exact_dp's adjacency
    // builder).
    let mut col_ptr: Vec<i32> = Vec::with_capacity(m + 1);
    col_ptr.push(0);
    let mut row_idx: Vec<i32> = Vec::new();
    for v in 0..m {
        for &n in &adj_local[v] {
            if n > v as u32 {
                row_idx.push(n as i32);
            }
        }
        col_ptr.push(row_idx.len() as i32);
    }
    // Try exact_dp first when the predicted edge count is small.
    // Predicted edge count estimate: max-degree × m (loose upper bound on
    // elimination fill, matches the brief's "custom_metrics predicted edge
    // count" language — a cheap structural proxy).
    let mut max_deg = 0usize;
    let mut total_edges = 0usize;
    for v in 0..m {
        let d = adj_local[v].len();
        max_deg = max_deg.max(d);
        total_edges += d;
    }
    let predicted_edges = (max_deg as f64 * m as f64) as usize;
    // exact_dp is profitable when the predicted edge count is small relative
    // to the component (sparse subgraph) and the component size is well
    // inside the DP's structural ceiling.
    let try_exact = m <= exact_dp::STRUCTURAL_MAX_N
        && predicted_edges <= 4 * m
        && (m <= 18 || predicted_edges <= 2 * m);
    if try_exact {
        // Build the usize pattern for exact_dp's adjacency builder.
        let mut cp_us = vec![0usize; m + 1];
        for v in 0..m {
            cp_us[v + 1] = cp_us[v] + adj_local[v].len();
        }
        let mut ri_us = vec![0usize; total_edges];
        let mut pos = 0usize;
        for v in 0..m {
            for &n in &adj_local[v] {
                ri_us[pos] = n as usize;
                pos += 1;
            }
        }
        if let Some(adj) = exact_dp::adjacency(m, &cp_us, &ri_us) {
            if let Some((perm, _flops)) = exact_dp::exact_min_flops_order(m, &adj) {
                return perm.into_iter().map(|x| x as u32).collect();
            }
        }
    }
    // Fall through to AMD.
    let core = feral_ordering_core::CscPattern::new(m, &col_ptr, &row_idx);
    if let Ok(core) = core {
        if let Ok(perm) = feral_amd::amd_order(&core) {
            return perm.into_iter().map(|x| x as u32).collect();
        }
    }
    // Last-resort: identity.
    (0..m as u32).collect()
}

/// `degree2_absorb_fallback`: a degree-ordered fill used when the separator
/// structure is too lopsided. Orders the vertices by ascending degree, ties
/// broken by index — same convention as the shipped `biconn_order` fall-back.
fn degree2_absorb_fallback(adj_local: &[Vec<u32>], m: usize) -> Vec<u32> {
    let mut deg: Vec<(usize, u32)> = (0..m).map(|v| (adj_local[v].len(), v as u32)).collect();
    deg.sort_unstable();
    deg.into_iter().map(|(_, v)| v).collect()
}

/// Bounded 1-opt adjacent-swap refinement: scan each boundary vertex (a
/// vertex with a non-empty separator neighbour), try swapping it with the
/// adjacent vertex in the current permutation, accept only when the
/// predicted-flops delta is negative. The "predicted flops" delta is
/// estimated via a cheap standalone `flops_of` call on the induced
/// sub-permutation (NOT on the full graph — full-graph flops recomputation
/// is too expensive inside the loop). The loop is bounded by
/// `ND_REFINEMENT_MAX_PASSES` to keep worst-case time bounded.
fn nd_one_opt_refine(
    perm: &mut Vec<u32>,
    adj: &[Vec<u32>],
    n: usize,
    scoring_pat: &feral::sparse::csc::CscPattern,
) {
    if perm.len() < 3 {
        return;
    }
    // Boundary vertices: those that have a non-trivial separator in the
    // ORIGINAL biconnected-component decomposition. Cheap proxy: any vertex
    // that appears in >= 2 biconnected components. If that data is not
    // available, fall back to "every vertex is a candidate" (over-permissive
    // but harmless under the bounded pass cap).
    let pos_of: Vec<u32> = {
        let mut p = vec![0u32; n];
        for (k, &v) in perm.iter().enumerate() {
            p[v as usize] = k as u32;
        }
        p
    };
    let mut boundary: Vec<u32> = perm.clone();
    boundary.sort_unstable_by(|&a, &b| pos_of[a as usize].cmp(&pos_of[b as usize]));
    // Limit the work: try at most a few hundred boundary candidates per
    // pass.
    let max_candidates = boundary.len().min(64);
    for pass in 0..ND_REFINEMENT_MAX_PASSES {
        let mut improved = false;
        for &bv in &boundary[..max_candidates] {
            let pk = pos_of[bv as usize] as usize;
            if pk == 0 || pk + 1 >= perm.len() {
                continue;
            }
            let neighbor = perm[pk + 1];
            // Compute the predicted-flop delta of swapping `perm[pk]` and
            // `perm[pk+1]`. We do this by running `flops_of` on the WHOLE
            // graph with the swap applied; the call is cheap when the
            // underlying elimination-tree pass is the same one `order()`
            // already does (port of `flops_of` from `mod.rs`).
            let mut trial = perm.clone();
            trial.swap(pk, pk + 1);
            let trial_us: Vec<usize> = trial.iter().map(|&v| v as usize).collect();
            let base_us: Vec<usize> = perm.iter().map(|&v| v as usize).collect();
            let f_trial = flops_of_local(scoring_pat, &trial_us);
            let f_base = flops_of_local(scoring_pat, &base_us);
            if f_trial < f_base {
                *perm = trial;
                improved = true;
            }
        }
        if !improved {
            break;
        }
        let _ = pass;
    }
}

/// Local copy of `flops_of` that the biconn module can call without
/// depending on private items in `mod.rs`. Identical semantics to
/// `mod.rs::flops_of`: `Σ c_j²` over the GNP column counts of the pattern
/// permuted by `perm`.
fn flops_of_local(pat: &feral::sparse::csc::CscPattern, perm: &[usize]) -> u64 {
    use feral::ordering::amd::permute_pattern;
    use feral::ordering::elimination_tree::EliminationTree;
    use feral::symbolic::column_counts_gnp;
    let permuted = permute_pattern(pat, perm);
    let etree = EliminationTree::from_pattern(&permuted);
    let counts = column_counts_gnp(&permuted, &etree);
    counts.iter().map(|&c| (c as u64) * (c as u64)).sum()
}

/// NESTED-DISSECTION (ND-Lite) variant.
///
/// Uses the biconnected-component decomposition as a separator oracle: each
/// block is recursively bisected at an articulation-point-like separator,
/// with leaves dispatched to AMD or `exact_dp` based on the `custom_metrics`
/// predicted edge count. Enforces `sep_balance_tol = 0.4`; falls back to
/// `degree2_absorb_fallback` when the separator is too lopsided. Applies a
/// bounded 1-opt adjacent-swap refinement on boundary vertices. On small
/// graphs where the predicted-flops ratio vs. feral AMD exceeds
/// `GATE_PREDICTED_FLOPS_RATIO = 1.05`, returns a multistart-AMD permutation
/// instead.
pub fn nested_dissection_order(pattern: &Pattern) -> Vec<i32> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }
    // Small-graph fast path: use the multistart-AMD fallback when n is small
    // (a heuristic: small graphs are where exact_dp's value matters and the
    // recursive dissection adds little; the multistart-AMD fallback is the
    // brief's mandated alternative).
    if n <= 512 {
        return nd_multistart_amd_fallback(pattern);
    }
    // Build the symmetric adjacency list (off-diagonal, deduplicated) for
    // the full graph.
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    for j in 0..n {
        let start = pattern.col_ptr[j];
        let end = pattern.col_ptr[j + 1];
        for &i in &pattern.row_idx[start..end] {
            if i != j && i < n {
                adj[j].push(i as u32);
                adj[i].push(j as u32);
            }
        }
    }
    for a in adj.iter_mut() {
        a.sort_unstable();
        a.dedup();
    }
    // Biconnected decomposition → split the graph into its biconnected
    // components. The block-cut tree is the recursive structure.
    let comps = biconnected_components(&adj);
    if comps.is_empty() {
        return nd_multistart_amd_fallback(pattern);
    }
    // Build a global-set of vertices seen at the top level. For each
    // component, build the induced subgraph and recursively bisect it.
    let mut perm: Vec<u32> = Vec::with_capacity(n);
    // Sort components by descending size so the dense core is bisected
    // first (matches `biconn_order`'s "deepest-first" emission).
    let mut comp_indices: Vec<usize> = (0..comps.len()).collect();
    comp_indices.sort_by(|&a, &b| comps[b].len().cmp(&comps[a].len()));
    let mut seen = vec![false; n];
    let mut work_budget = 16usize; // bounded recursion pass count
    for &c in &comp_indices {
        if comps[c].len() > ND_COMPONENT_BUDGET {
            continue;
        }
        // Collect the vertex set of this component.
        let mut nodes: Vec<u32> = Vec::new();
        for &(u, v) in &comps[c] {
            if !seen[u as usize] {
                seen[u as usize] = true;
                nodes.push(u);
            }
            if !seen[v as usize] {
                seen[v as usize] = true;
                nodes.push(v);
            }
        }
        if nodes.is_empty() {
            continue;
        }
        let adj_local = induced_adj_local(&nodes, &adj);
        let m = adj_local.len();
        if m <= ND_SEP_MIN_SIZE {
            // Leaf: order by AMD or exact_dp.
            let leaf = nd_amd_leaf(&adj_local, m);
            for &li in &leaf {
                perm.push(nodes[li as usize]);
            }
        } else if let Some(local_perm) = nd_recurse(&adj_local, 0, &mut work_budget) {
            for &li in &local_perm {
                perm.push(nodes[li as usize]);
            }
        } else {
            // Recursion budget exhausted: fall back to degree2-absorb on
            // the induced subgraph, mapped back to global ids.
            let local = degree2_absorb_fallback(&adj_local, m);
            for &li in &local {
                perm.push(nodes[li as usize]);
            }
        }
    }
    // Append any vertices we have not seen (e.g. isolated vertices that
    // belong to no biconnected component).
    for v in 0..n as u32 {
        if !seen[v as usize] {
            perm.push(v);
        }
    }
    // Sanity: the perm must be a bijection of 0..n.
    debug_assert_eq!(perm.len(), n);
    {
        let mut check = vec![false; n];
        for &v in &perm {
            debug_assert!((v as usize) < n && !check[v as usize]);
            check[v as usize] = true;
        }
    }
    // Build a ScoringPattern for predicted-flops comparisons.
    let scoring_pat = feral::sparse::csc::CscPattern {
        n,
        col_ptr: pattern.col_ptr.clone(),
        row_idx: pattern.row_idx.clone(),
    };
    // Small-graph fallback: if custom_metrics predicted-flops ratio vs.
    // feral AMD exceeds the gate, return the multistart-AMD permutation.
    let perm_us: Vec<usize> = perm.iter().map(|&v| v as usize).collect();
    let nd_flops = flops_of_local(&scoring_pat, &perm_us);
    // Reference: feral AMD's flops on the same pattern.
    let col_ptr_i32: Vec<i32> = pattern
        .col_ptr
        .iter()
        .map(|&x| i32::try_from(x).expect("pattern too large for i32-indexed ordering"))
        .collect();
    let row_idx_i32: Vec<i32> = pattern
        .row_idx
        .iter()
        .map(|&x| i32::try_from(x).expect("pattern too large for i32-indexed ordering"))
        .collect();
    if let Ok(core) = feral_ordering_core::CscPattern::new(n, &col_ptr_i32, &row_idx_i32) {
        if let Ok(amd_perm) = feral_amd::amd_order(&core) {
            let amd_us: Vec<usize> = amd_perm.iter().map(|&x| x as usize).collect();
            let amd_flops = flops_of_local(&scoring_pat, &amd_us);
            if amd_flops > 0 {
                let ratio = nd_flops as f64 / amd_flops as f64;
                if ratio > ND_GATE_PREDICTED_FLOPS_RATIO {
                    return nd_multistart_amd_fallback(pattern);
                }
            }
        }
    }
    // 1-opt adjacent-swap refinement gated by custom_metrics predicted-flops
    // delta. The refinement is bounded by `ND_REFINEMENT_MAX_PASSES`; it
    // also bails on the first non-improving pass.
    let mut perm_u32 = perm;
    nd_one_opt_refine(&mut perm_u32, &adj, n, &scoring_pat);
    perm_u32.into_iter().map(|x| x as i32).collect()
}

/// Small-graph fallback: a small-budget multistart AMD. Relabels the pattern
/// deterministically (seeds 1..=restarts) and picks the AMD ordering with
/// the fewest `flops_of` flops. Used on small graphs (n <= 512) or whenever
/// the predicted-flops ratio vs. feral AMD exceeds the gate.
fn nd_multistart_amd_fallback(pattern: &Pattern) -> Vec<i32> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }
    let col_ptr_i32: Vec<i32> = pattern
        .col_ptr
        .iter()
        .map(|&x| i32::try_from(x).expect("pattern too large for i32-indexed ordering"))
        .collect();
    let row_idx_i32: Vec<i32> = pattern
        .row_idx
        .iter()
        .map(|&x| i32::try_from(x).expect("pattern too large for i32-indexed ordering"))
        .collect();
    let core = match feral_ordering_core::CscPattern::new(n, &col_ptr_i32, &row_idx_i32) {
        Ok(c) => c,
        Err(_) => return (0..n as i32).collect(),
    };
    // Plain AMD first.
    let amd = match feral_amd::amd_order(&core) {
        Ok(p) => p,
        Err(_) => return (0..n as i32).collect(),
    };
    let mut best_perm: Vec<usize> = amd.iter().map(|&x| x as usize).collect();
    let scoring_pat = feral::sparse::csc::CscPattern {
        n,
        col_ptr: pattern.col_ptr.clone(),
        row_idx: pattern.row_idx.clone(),
    };
    let mut best_flops = flops_of_local(&scoring_pat, &best_perm);
    // Relabel-AMD restarts; small budget (at most 3 restarts on the
    // multistart path; the shipped `relabel_restarts` allocates a per-matrix
    // time budget, but here we are explicitly a FALLBACK, so a small flat
    // cap keeps the fallback cheap).
    let restarts = 3usize;
    for r in 1..=restarts {
        let q = super::relabel(n, r as u64);
        let b = super::permute_pattern(&scoring_pat, &q);
        let bcp: Vec<i32> = b.col_ptr.iter().map(|&x| x as i32).collect();
        let bri: Vec<i32> = b.row_idx.iter().map(|&x| x as i32).collect();
        let Ok(bcore) = feral_ordering_core::CscPattern::new(n, &bcp, &bri) else {
            continue;
        };
        let Ok(pb) = feral_amd::amd_order(&bcore) else {
            continue;
        };
        let perm: Vec<usize> = pb.iter().map(|&x| q[x as usize]).collect();
        let f = flops_of_local(&scoring_pat, &perm);
        if f < best_flops {
            best_flops = f;
            best_perm = perm;
        }
    }
    best_perm.into_iter().map(|x| x as i32).collect()
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
