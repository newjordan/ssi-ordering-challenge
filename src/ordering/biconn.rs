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
                let (bs, rs) = (comp_verts[b as usize].len(), comp_verts[root as usize].len());
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

/// PATH-COVER DECOMPOSITION of the biconnected-component tree.
///
/// Returns a path cover of the block-cut tree(s) of `pattern` as a list of
/// `Vec<usize>` paths, each path being an alternating sequence of block ids
/// and articulation-point ids (rooted in [`biconnected_components`]: block
/// ids 0..`nc`, articulation ids `nc + v` for original vertex `v` in 0..n).
/// Two block-cut trees sharing no vertices are independent; their paths
/// simply accumulate into the same flat list — every path is a contiguous
/// walk through ONE tree.
///
/// ## What this is, and what it is for
///
/// A path cover of a forest decomposes the forest into a small number of
/// vertex-disjoint paths; the classical greedy bound is `≤ ⌈F/2⌉` paths
/// where `F` is the number of tree nodes. The decomposition is built by
/// repeatedly peeling a leaf-to-leaf walk from the still-uncovered tree,
/// which is the textbook "path cover of a forest" construction. Each path
/// therefore captures a CONTIGUOUS subset of the block-cut tree — i.e. a
/// chain of blocks that share only articulation points along the way.
///
/// The block-cut tree already encodes the structural blind spot the
/// portfolio addresses: an articulation's inflated degree crosses blocks,
/// and AMD's tie-break interleaves the two sides of an articulation in
/// arbitrary order. The path cover turns that interleaving into a
/// deterministic, topologically-coherent walk: along one path every block
/// is connected to the next through exactly one articulation, so a
/// path-following orderer (the trust-region AMD phase in `legacy.rs`)
/// can build a sub-front for the WHOLE path and let AMD see the full
/// local structure, instead of AMD's interleaving-at-the-articulation
/// pathology. This is the "collapsed supernode front" referred to in
/// the method family name: a path's articulation sequence IS a supernode
/// chain (a clique chain through the original graph's chordal completion).
///
/// ## Determinism
///
/// The block-cut tree is rooted by [`biconn_order`] (largest block, ties
/// lowest id, per connected component). Peeling always starts at a leaf
/// and walks to the opposite leaf; ties on "which leaf first" / "which
/// neighbour next" go to the lowest id, so two runs on the same pattern
/// produce byte-identical paths. Cost: O(`nc` + `nart`).
pub fn path_cover(pattern: &Pattern) -> Vec<Vec<usize>> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }
    let adj = build_adj(pattern);
    let comps = biconnected_components(&adj);
    let nc = comps.len();
    if nc == 0 {
        return Vec::new();
    }

    // Build the (block, articulation) node set of the block-cut tree. Block
    // ids are 0..nc; articulation ids are nc + v (so the flat id space is
    // 0..(nc + nart), with the rest unused).
    let mut comp_of: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut comp_verts: Vec<Vec<u32>> = Vec::with_capacity(nc);
    for (c, edges) in comps.iter().enumerate() {
        let mut seen = vec![u32::MAX; n];
        let mut verts: Vec<u32> = Vec::new();
        for &(u, v) in edges {
            for &x in &[u, v] {
                let xu = x as usize;
                if seen[xu] != c as u32 {
                    seen[xu] = c as u32;
                    verts.push(x);
                    comp_of[xu].push(c as u32);
                }
            }
        }
        comp_verts.push(verts);
    }

    // Flatten the block-cut tree into a single undirected graph on the flat
    // id space (block u <-> articulation nc + v for every block u containing
    // articulation v), with adjacency in ASCENDING-id order so the path
    // cover's tie-break (lowest id wins) is deterministic.
    let total = nc + n;
    let mut bc_adj: Vec<Vec<u32>> = vec![Vec::new(); total];
    let mut degree: Vec<usize> = vec![0; total];
    for (c, verts) in comp_verts.iter().enumerate() {
        for &v in verts {
            if comp_of[v as usize].len() >= 2 {
                let aid = nc + v as usize;
                bc_adj[c].push(aid as u32);
                bc_adj[aid].push(c as u32);
                degree[c] += 1;
                degree[aid] += 1;
            }
        }
    }
    for a in bc_adj.iter_mut() {
        a.sort_unstable();
    }

    // Articulations with degree < 2 in the block-cut tree (i.e. in exactly
    // one block) are leaves too; treat them like any other leaf so a
    // path through them is emitted. Singletons (isolated blocks with no
    // articulation) are emitted as one-vertex paths.
    let mut covered = vec![false; total];
    let mut paths: Vec<Vec<usize>> = Vec::new();
    for start in 0..total {
        if covered[start] || degree[start] == 0 {
            if !covered[start] && degree[start] == 0 {
                // Singleton path: a block with no articulation neighbours.
                covered[start] = true;
                paths.push(vec![start]);
            }
            continue;
        }
        // Walk from `start` to a leaf. At each step, follow the LOWEST-id
        // un-covered neighbour (deterministic). Stop on a leaf (deg <= 1
        // among un-covered nodes) or at a covered node (so the next outer
        // iteration can pick up the rest of the tree).
        let mut path: Vec<usize> = Vec::new();
        let mut cur = start;
        loop {
            covered[cur] = true;
            path.push(cur);
            // Pick the lowest un-covered neighbour.
            let mut next: Option<usize> = None;
            for &u in &bc_adj[cur] {
                let uu = u as usize;
                if !covered[uu] {
                    next = Some(uu);
                    break;
                }
            }
            match next {
                Some(nx) => cur = nx,
                None => break,
            }
        }
        if !path.is_empty() {
            paths.push(path);
        }
    }

    paths
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

    #[test]
    fn path_cover_path_graph_is_a_single_path() {
        // A path graph has one block-cut tree that IS a path already.
        let n = 25;
        let edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        let pat = Pattern::from_edges(n, &edges);
        let paths = path_cover(&pat);
        // A path's block-cut tree is a chain of n bridges; path-cover emits
        // it as one or two long paths (every internal node is an
        // articulation; leaves are blocks). What matters is that the union
        // covers every (block, articulation) node exactly once.
        let total: usize = paths.iter().map(|p| p.len()).sum();
        let all_flat: std::collections::HashSet<usize> =
            paths.iter().flatten().copied().collect();
        assert_eq!(all_flat.len(), total, "paths must be vertex-disjoint");
    }

    #[test]
    fn path_cover_two_cliques_one_articulation() {
        // K6 + K7 sharing one articulation -> block-cut tree is two
        // blocks joined at one articulation; the tree itself is a path of
        // length 2.
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
        let paths = path_cover(&pat);
        let all_flat: std::collections::HashSet<usize> =
            paths.iter().flatten().copied().collect();
        // All emitted paths should be vertex-disjoint.
        let total: usize = paths.iter().map(|p| p.len()).sum();
        assert_eq!(all_flat.len(), total, "paths must be vertex-disjoint");
        // And every path should be non-empty.
        for p in &paths {
            assert!(!p.is_empty(), "no empty paths");
        }
    }

    #[test]
    fn path_cover_deterministic() {
        let side = 10;
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
        let pat = Pattern::from_edges(side * side, &edges);
        let a = path_cover(&pat);
        let b = path_cover(&pat);
        assert_eq!(a, b, "path cover must be deterministic");
    }
}
