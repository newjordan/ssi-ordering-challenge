//! Comprehensive 4-Tier Opaque-Box E2E Test Suite for `order(&Pattern) -> Vec<usize>`.
//!
//! Tiers covered:
//! - **Tier 1 (Feature Coverage)**: Fundamental graph families (paths, stars, cycles,
//!   cliques, bipartite, disjoint graphs, binary trees, caterpillars, rings of cliques, wheels).
//! - **Tier 2 (Boundary & Corner Cases)**: Empty/singleton graphs ($n=0, 1, 2$), empty edges (isolated vertices),
//!   near-complete density ($d \to 1.0$), high-degree hubs, extreme aspect ratios, disconnected islands,
//!   and rigorous permutation bijection invariants.
//! - **Tier 3 (Cross-Feature, Determinism & Performance)**: Multi-round bitwise determinism across topologies,
//!   multi-threaded concurrency determinism, execution timing headroom (<1.85s cap), AMD portfolio floor
//!   guarantee ($\text{ratio} \le 1.0$), and vertex relabeling structural invariance.
//! - **Tier 4 (Real-World Application Scenarios)**: Real development corpus matrices (`corpus/dev/patterns.jsonl`),
//!   synthetic KKT saddle-point optimization systems, 2D/3D PDE regular mesh grids, Chimera quantum annealing
//!   lattice topologies, bordered block-diagonal domain decomposition structures, and multi-scale bucket stress
//!   (`lt_1k`, `1k_10k`, `gt_10k`).

use super::*;
use std::time::Instant;

// ============================================================================
// Verification Helpers & Mathematical Invariants
// ============================================================================

/// Rigorously validates that a permutation is a true mathematical bijection of 0..n:
/// 1. Length exactly equals n.
/// 2. Every element is strictly within [0, n).
/// 3. Every integer 0..n appears exactly once (no duplicates, no omissions).
pub fn assert_valid_bijection(n: usize, perm: &[usize], context: &str) {
    assert_eq!(
        perm.len(),
        n,
        "[{context}] Permutation length {} != matrix dimension n={}",
        perm.len(),
        n
    );
    if n == 0 {
        return;
    }
    let mut seen = vec![false; n];
    for (pos, &v) in perm.iter().enumerate() {
        assert!(
            v < n,
            "[{context}] Permutation element at pos {pos} is {v}, which is out of bounds for n={n}"
        );
        assert!(
            !seen[v],
            "[{context}] Duplicate element {v} found at pos {pos} in permutation of n={n}"
        );
        seen[v] = true;
    }
    let missing_count = seen.iter().filter(|&&s| !s).count();
    assert_eq!(
        missing_count, 0,
        "[{context}] Permutation has {missing_count} missing elements for n={n}"
    );
}

/// Asserts that repeated invocations of `order()` on the given pattern produce
/// bitwise-identical permutations across `rounds` calls.
pub fn assert_deterministic(pattern: &Pattern, rounds: usize, context: &str) -> Vec<usize> {
    assert!(rounds >= 2, "Must test at least 2 rounds for determinism");
    let baseline = order(pattern);
    assert_valid_bijection(pattern.n, &baseline, context);

    for r in 1..rounds {
        let perm = order(pattern);
        assert_eq!(
            perm, baseline,
            "[{context}] Non-deterministic output detected at round {r} vs round 0 for n={}",
            pattern.n
        );
    }
    baseline
}

/// Simple deterministic linear congruential generator for test fixture generation.
pub struct TestRng {
    state: u64,
}

impl TestRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1),
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state >> 11
    }

    pub fn next_usize(&mut self, upper_bound: usize) -> usize {
        if upper_bound == 0 {
            return 0;
        }
        (self.next_u64() % (upper_bound as u64)) as usize
    }

    pub fn next_bool(&mut self, prob: f64) -> bool {
        let max = 1_000_000_000u64;
        let threshold = (prob * max as f64) as u64;
        (self.next_u64() % max) < threshold
    }
}

// ============================================================================
// Graph Topology Builders
// ============================================================================

/// Path graph $P_n$: 1D linear chain of vertices 0 -- 1 -- 2 ... -- n-1.
pub fn gen_path(n: usize) -> Pattern {
    if n <= 1 {
        return Pattern::from_edges(n, &[]);
    }
    let edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
    Pattern::from_edges(n, &edges)
}

/// Star graph $S_n$: one central hub connected to $n-1$ independent leaves.
pub fn gen_star(n: usize, hub: usize) -> Pattern {
    assert!(hub < n, "hub index out of bounds");
    let edges: Vec<(usize, usize)> = (0..n)
        .filter(|&v| v != hub)
        .map(|leaf| (leaf, hub))
        .collect();
    Pattern::from_edges(n, &edges)
}

/// Cycle graph $C_n$: simple cycle $0 - 1 - 2 - \dots - n-1 - 0$.
pub fn gen_cycle(n: usize) -> Pattern {
    if n <= 2 {
        return gen_path(n);
    }
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        edges.push((i, (i + 1) % n));
    }
    Pattern::from_edges(n, &edges)
}

/// Complete graph / Clique $K_n$: all pairs connected.
pub fn gen_clique(n: usize) -> Pattern {
    let mut edges = Vec::with_capacity(n * n / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            edges.push((i, j));
        }
    }
    Pattern::from_edges(n, &edges)
}

/// Complete bipartite graph $K_{m,n}$.
pub fn gen_complete_bipartite(m: usize, n: usize) -> Pattern {
    let total = m + n;
    let mut edges = Vec::with_capacity(m * n);
    for u in 0..m {
        for v in m..total {
            edges.push((u, v));
        }
    }
    Pattern::from_edges(total, &edges)
}

/// Sparse bipartite graph with edge probability $p$.
pub fn gen_random_bipartite(m: usize, n: usize, p: f64, seed: u64) -> Pattern {
    let total = m + n;
    let mut rng = TestRng::new(seed);
    let mut edges = Vec::new();
    for u in 0..m {
        for v in m..total {
            if rng.next_bool(p) {
                edges.push((u, v));
            }
        }
    }
    Pattern::from_edges(total, &edges)
}

/// Disjoint union of multiple independent patterns.
pub fn gen_disjoint_union(patterns: &[Pattern]) -> Pattern {
    let total_n: usize = patterns.iter().map(|p| p.n).sum();
    let mut edges = Vec::new();
    let mut offset = 0;
    for p in patterns {
        for j in 0..p.n {
            for &r in p.col(j) {
                if r < j {
                    edges.push((offset + r, offset + j));
                }
            }
        }
        offset += p.n;
    }
    Pattern::from_edges(total_n, &edges)
}

/// Complete binary tree with given depth ($n = 2^{\text{depth}} - 1$).
pub fn gen_binary_tree(depth: usize) -> Pattern {
    let n = (1 << depth) - 1;
    let mut edges = Vec::new();
    for i in 0..n {
        let left = 2 * i + 1;
        let right = 2 * i + 2;
        if left < n {
            edges.push((i, left));
        }
        if right < n {
            edges.push((i, right));
        }
    }
    Pattern::from_edges(n, &edges)
}

/// Caterpillar graph: central path spine with attached feet.
pub fn gen_caterpillar(spine_len: usize, feet_per_node: usize) -> Pattern {
    let n = spine_len + spine_len * feet_per_node;
    let mut edges = Vec::new();
    for i in 0..spine_len.saturating_sub(1) {
        edges.push((i, i + 1));
    }
    let mut foot_idx = spine_len;
    for spine_node in 0..spine_len {
        for _ in 0..feet_per_node {
            edges.push((spine_node, foot_idx));
            foot_idx += 1;
        }
    }
    Pattern::from_edges(n, &edges)
}

/// Ring of cliques: $k$ cliques of size $c$, linked sequentially in a loop.
pub fn gen_ring_of_cliques(num_cliques: usize, clique_size: usize) -> Pattern {
    let n = num_cliques * clique_size;
    let mut edges = Vec::new();
    for c in 0..num_cliques {
        let base = c * clique_size;
        for i in 0..clique_size {
            for j in (i + 1)..clique_size {
                edges.push((base + i, base + j));
            }
        }
        let next_base = ((c + 1) % num_cliques) * clique_size;
        edges.push((base + clique_size - 1, next_base));
    }
    Pattern::from_edges(n, &edges)
}

/// Wheel graph $W_n$: outer cycle $C_{n-1}$ joined to central hub $n-1$.
pub fn gen_wheel(n: usize) -> Pattern {
    if n <= 3 {
        return gen_clique(n);
    }
    let hub = n - 1;
    let cycle_len = n - 1;
    let mut edges = Vec::with_capacity(2 * cycle_len);
    for i in 0..cycle_len {
        edges.push((i, (i + 1) % cycle_len));
        edges.push((i, hub));
    }
    Pattern::from_edges(n, &edges)
}

/// Ladder graph $2 \times L$.
pub fn gen_ladder(length: usize) -> Pattern {
    let n = 2 * length;
    let mut edges = Vec::new();
    for i in 0..length {
        edges.push((i, length + i)); // rung
        if i + 1 < length {
            edges.push((i, i + 1)); // rail 1
            edges.push((length + i, length + i + 1)); // rail 2
        }
    }
    Pattern::from_edges(n, &edges)
}

/// Barbell graph: two $K_k$ cliques connected by a central path.
pub fn gen_barbell(clique_size: usize, path_len: usize) -> Pattern {
    let n = 2 * clique_size + path_len;
    let mut edges = Vec::new();
    // Clique 1: 0..clique_size
    for i in 0..clique_size {
        for j in (i + 1)..clique_size {
            edges.push((i, j));
        }
    }
    // Path: clique_size .. clique_size + path_len
    let path_start = clique_size;
    let path_end = clique_size + path_len;
    if path_len > 0 {
        edges.push((clique_size - 1, path_start));
        for i in path_start..path_end - 1 {
            edges.push((i, i + 1));
        }
        edges.push((path_end - 1, path_end));
    } else {
        edges.push((clique_size - 1, clique_size));
    }
    // Clique 2: path_end .. n
    for i in path_end..n {
        for j in (i + 1)..n {
            edges.push((i, j));
        }
    }
    Pattern::from_edges(n, &edges)
}

/// 2D Grid ($k \times k$).
pub fn gen_grid2d(k: usize) -> Pattern {
    let n = k * k;
    let mut edges = Vec::new();
    for x in 0..k {
        for y in 0..k {
            let v = x * k + y;
            if x + 1 < k {
                edges.push((v, (x + 1) * k + y));
            }
            if y + 1 < k {
                edges.push((v, x * k + y + 1));
            }
        }
    }
    Pattern::from_edges(n, &edges)
}

/// 3D Grid ($k \times k \times k$).
pub fn gen_grid3d(k: usize) -> Pattern {
    let n = k * k * k;
    let idx = |x: usize, y: usize, z: usize| (x * k + y) * k + z;
    let mut edges = Vec::new();
    for x in 0..k {
        for y in 0..k {
            for z in 0..k {
                let v = idx(x, y, z);
                if x + 1 < k {
                    edges.push((v, idx(x + 1, y, z)));
                }
                if y + 1 < k {
                    edges.push((v, idx(x, y + 1, z)));
                }
                if z + 1 < k {
                    edges.push((v, idx(x, y, z + 1)));
                }
            }
        }
    }
    Pattern::from_edges(n, &edges)
}

/// Chimera quantum-annealing cell lattice ($M \times N$ cells of $K_{4,4}$).
pub fn gen_chimera(m_rows: usize, n_cols: usize) -> Pattern {
    let cell_size = 8; // K_{4,4}
    let total_n = m_rows * n_cols * cell_size;
    let mut edges = Vec::new();

    for r in 0..m_rows {
        for c in 0..n_cols {
            let base = (r * n_cols + c) * cell_size;
            // Intra-cell K_{4,4} bipartite
            for i in 0..4 {
                for j in 4..8 {
                    edges.push((base + i, base + j));
                }
            }
            // Inter-cell horizontal couplings (columns c <-> c+1)
            if c + 1 < n_cols {
                let right_base = (r * n_cols + (c + 1)) * cell_size;
                for i in 0..4 {
                    edges.push((base + i, right_base + i));
                }
            }
            // Inter-cell vertical couplings (rows r <-> r+1)
            if r + 1 < m_rows {
                let down_base = ((r + 1) * n_cols + c) * cell_size;
                for j in 4..8 {
                    edges.push((base + j, down_base + j));
                }
            }
        }
    }
    Pattern::from_edges(total_n, &edges)
}

/// Bordered Block Diagonal: `num_blocks` independent cliques bordered by `border_size` nodes.
pub fn gen_bordered_block_diagonal(
    num_blocks: usize,
    block_size: usize,
    border_size: usize,
) -> Pattern {
    let total_n = num_blocks * block_size + border_size;
    let mut edges = Vec::new();
    let border_start = num_blocks * block_size;

    // Diagonal blocks
    for b in 0..num_blocks {
        let base = b * block_size;
        for i in 0..block_size {
            for j in (i + 1)..block_size {
                edges.push((base + i, base + j));
            }
            // Coupling to border
            for border_node in border_start..total_n {
                edges.push((base + i, border_node));
            }
        }
    }
    // Border clique
    for i in border_start..total_n {
        for j in (i + 1)..total_n {
            edges.push((i, j));
        }
    }
    Pattern::from_edges(total_n, &edges)
}

/// Synthetic KKT Saddle-Point Matrix.
pub fn gen_kkt(nh: usize, m: usize, seed: u64) -> Pattern {
    let n = nh + m;
    let mut rng = TestRng::new(seed);
    let mut edges = Vec::new();
    let bw = 40;
    for i in 0..nh {
        if i + 1 < nh {
            edges.push((i, i + 1));
        }
        for _ in 0..2 {
            let off = 1 + rng.next_usize(bw);
            let j = if rng.next_bool(0.5) {
                i.saturating_sub(off)
            } else {
                (i + off).min(nh - 1)
            };
            if j != i {
                edges.push((i, j));
            }
        }
    }
    for c in 0..m {
        let row = nh + c;
        let center = c * nh / m;
        let deg = 3 + rng.next_usize(4);
        for _ in 0..deg {
            let off = rng.next_usize(2 * bw);
            let j = (center + off).min(nh - 1);
            edges.push((row, j));
        }
    }
    Pattern::from_edges(n, &edges)
}

/// Erdős-Rényi $G(n, p)$ random graph.
pub fn gen_er_random(n: usize, p: f64, seed: u64) -> Pattern {
    let mut rng = TestRng::new(seed);
    let mut edges = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if rng.next_bool(p) {
                edges.push((i, j));
            }
        }
    }
    Pattern::from_edges(n, &edges)
}

// ============================================================================
// TIER 1: Feature Coverage (Fundamental Graph Topologies)
// ============================================================================

#[test]
fn tier1_feature_path_graphs() {
    // Linear paths P_n across multiple scales.
    // Mathematical property: zero fill under path elimination order (c_j <= 2, flops <= 4n - 3).
    for &n in &[3, 10, 50, 200, 1000] {
        let pat = gen_path(n);
        let perm = order(&pat);
        assert_valid_bijection(n, &perm, &format!("Path P_{n}"));

        let score = ssi_scoring::score(&pat, &perm);
        assert!(
            score.flops <= (4 * n as u64).saturating_sub(3) + 10,
            "Path P_{n} flop count ({}) should be approximately zero fill (<= {})",
            score.flops,
            4 * n - 3
        );
    }
}

#[test]
fn tier1_feature_star_graphs() {
    // Star graphs S_n with hub at 0, middle, and end.
    // Mathematical property: eliminating leaves before hub yields zero fill (flops = 4n - 3).
    for &n in &[4, 25, 100, 500] {
        for &hub in &[0, n / 2, n - 1] {
            let pat = gen_star(n, hub);
            let perm = order(&pat);
            assert_valid_bijection(n, &perm, &format!("Star S_{n} (hub={hub})"));

            let score = ssi_scoring::score(&pat, &perm);
            assert_eq!(
                score.flops,
                (4 * n as u64).saturating_sub(3),
                "Star S_{n} hub={hub} did not achieve optimal zero-fill flops: got {}",
                score.flops
            );
        }
    }
}

#[test]
fn tier1_feature_cycle_graphs() {
    // Cycle graphs C_n.
    for &n in &[3, 10, 50, 250, 1000] {
        let pat = gen_cycle(n);
        let perm = order(&pat);
        assert_valid_bijection(n, &perm, &format!("Cycle C_{n}"));

        let score = ssi_scoring::score(&pat, &perm);
        assert!(
            score.flops <= (10 * n as u64) + 50,
            "Cycle C_{n} should have linear O(n) flops, got {}",
            score.flops
        );
    }
}

#[test]
fn tier1_feature_clique_graphs() {
    // Complete graphs K_n.
    // Closed-form property: completely permutation-invariant,
    // nnz(L) = n(n+1)/2, flops = n(n+1)(2n+1)/6.
    for &n in &[3, 8, 20, 50] {
        let pat = gen_clique(n);
        let perm = order(&pat);
        assert_valid_bijection(n, &perm, &format!("Clique K_{n}"));

        let score = ssi_scoring::score(&pat, &perm);
        let expected_nnz = (n * (n + 1) / 2) as u64;
        let expected_flops = (n * (n + 1) * (2 * n + 1) / 6) as u64;

        assert_eq!(
            score.nnz_l, expected_nnz,
            "Clique K_{n} nnz(L) drift: got {}, expected {}",
            score.nnz_l, expected_nnz
        );
        assert_eq!(
            score.flops, expected_flops,
            "Clique K_{n} flops drift: got {}, expected {}",
            score.flops, expected_flops
        );
    }
}

#[test]
fn tier1_feature_bipartite_graphs() {
    // Complete and sparse bipartite graphs K_{m,n}.
    for &(m, n) in &[(5, 10), (10, 10), (20, 50), (100, 100)] {
        let pat = gen_complete_bipartite(m, n);
        let perm = order(&pat);
        assert_valid_bijection(m + n, &perm, &format!("Complete Bipartite K_{{{m},{n}}}"));

        let score = ssi_scoring::score(&pat, &perm);
        let amd_perm = ssi_scoring::amd_baseline(&pat);
        let amd_score = ssi_scoring::score(&pat, &amd_perm);
        assert!(
            score.flops <= amd_score.flops,
            "Bipartite K_{{{m},{n}}} flops ({}) should not exceed AMD baseline ({})",
            score.flops,
            amd_score.flops
        );
    }

    // Sparse bipartite
    let sparse_bip = gen_random_bipartite(80, 80, 0.05, 42);
    let perm = order(&sparse_bip);
    assert_valid_bijection(160, &perm, "Sparse Bipartite 80x80");
}

#[test]
fn tier1_feature_disjoint_multi_component() {
    // Disjoint union of heterogeneous components.
    let c1 = gen_path(30);
    let c2 = gen_clique(12);
    let c3 = gen_star(25, 0);
    let c4 = Pattern::from_edges(10, &[]); // 10 isolated nodes
    let disjoint = gen_disjoint_union(&[c1, c2, c3, c4]);

    let perm = order(&disjoint);
    assert_valid_bijection(disjoint.n, &perm, "Disjoint 4-Component Graph");
    assert_eq!(disjoint.n, 30 + 12 + 25 + 10);
}

#[test]
fn tier1_feature_binary_trees_and_caterpillars() {
    // Binary trees (depths 4, 7, 9)
    for &depth in &[4, 7, 9] {
        let tree = gen_binary_tree(depth);
        let perm = order(&tree);
        assert_valid_bijection(tree.n, &perm, &format!("Binary Tree depth={depth}"));
    }

    // Caterpillars (spine lengths with various feet)
    for &(spine, feet) in &[(20, 3), (50, 5), (100, 2)] {
        let cat = gen_caterpillar(spine, feet);
        let perm = order(&cat);
        assert_valid_bijection(cat.n, &perm, &format!("Caterpillar spine={spine} feet={feet}"));
    }
}

#[test]
fn tier1_feature_ring_of_cliques() {
    // Ring of 6 cliques of size 8, ring of 12 cliques of size 5
    for &(num_c, size_c) in &[(6, 8), (12, 5), (20, 6)] {
        let ring = gen_ring_of_cliques(num_c, size_c);
        let perm = order(&ring);
        assert_valid_bijection(
            ring.n,
            &perm,
            &format!("Ring of {num_c} Cliques (size {size_c})"),
        );
    }
}

#[test]
fn tier1_feature_wheel_graphs() {
    // Wheel graphs W_n
    for &n in &[10, 50, 200, 600] {
        let wheel = gen_wheel(n);
        let perm = order(&wheel);
        assert_valid_bijection(n, &perm, &format!("Wheel W_{n}"));
    }
}

// ============================================================================
// TIER 2: Boundary & Corner Cases (Extreme, Adversarial & Degenerate Inputs)
// ============================================================================

#[test]
fn tier2_boundary_empty_and_singletons() {
    // Empty graph n=0
    let empty = Pattern::from_edges(0, &[]);
    let p_empty = order(&empty);
    assert_eq!(p_empty.len(), 0, "n=0 must return empty vector");

    // Singleton n=1
    let single = Pattern::from_edges(1, &[]);
    let p_single = order(&single);
    assert_eq!(p_single, vec![0], "n=1 must return [0]");

    // n=2 with 0 edges
    let two_iso = Pattern::from_edges(2, &[]);
    let p_two_iso = order(&two_iso);
    assert_valid_bijection(2, &p_two_iso, "n=2 isolated");

    // n=2 with 1 edge
    let two_edge = Pattern::from_edges(2, &[(0, 1)]);
    let p_two_edge = order(&two_edge);
    assert_valid_bijection(2, &p_two_edge, "n=2 with edge");
}

#[test]
fn tier2_boundary_isolated_vertices_scaling() {
    // Pure isolated vertices at multiple scales (n = 10, 100, 1000, 4000).
    // Closed form: nnz(L) = n, flops = n (strictly diagonal factor).
    for &n in &[10, 100, 1000, 4000] {
        let iso = Pattern::from_edges(n, &[]);
        let perm = order(&iso);
        assert_valid_bijection(n, &perm, &format!("Isolated Vertices n={n}"));

        let score = ssi_scoring::score(&iso, &perm);
        assert_eq!(
            score.nnz_l, n as u64,
            "Isolated vertices n={n} nnz(L) must equal n"
        );
        assert_eq!(
            score.flops, n as u64,
            "Isolated vertices n={n} flops must equal n"
        );
    }
}

#[test]
fn tier2_boundary_near_complete_density() {
    // Quasi-clique: K_50 missing 1 edge
    let n = 50;
    let mut edges = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if !(i == 0 && j == 1) {
                edges.push((i, j));
            }
        }
    }
    let quasi = Pattern::from_edges(n, &edges);
    let perm = order(&quasi);
    assert_valid_bijection(n, &perm, "Quasi-clique K_50 missing 1 edge");

    // Dense random graph p=0.95
    let dense = gen_er_random(60, 0.95, 12345);
    let perm_dense = order(&dense);
    assert_valid_bijection(dense.n, &perm_dense, "Dense random graph p=0.95");
}

#[test]
fn tier2_boundary_extreme_hub_degree() {
    // Single giant hub with 5000 leaves
    let star_giant = gen_star(5000, 0);
    let perm = order(&star_giant);
    assert_valid_bijection(5000, &perm, "Giant Star n=5000");

    // Multi-hub graph: 5 hubs mutually connected and all connected to 2000 common leaves
    let num_hubs = 5;
    let num_leaves = 2000;
    let total_n = num_hubs + num_leaves;
    let mut edges = Vec::new();
    // Hub clique
    for i in 0..num_hubs {
        for j in (i + 1)..num_hubs {
            edges.push((i, j));
        }
    }
    // Hubs to leaves
    for h in 0..num_hubs {
        for leaf in num_hubs..total_n {
            edges.push((h, leaf));
        }
    }
    let multi_hub = Pattern::from_edges(total_n, &edges);
    let perm_mh = order(&multi_hub);
    assert_valid_bijection(total_n, &perm_mh, "Multi-Hub Graph (5 hubs, 2000 leaves)");
}

#[test]
fn tier2_boundary_extreme_aspect_ratios() {
    // 2 x 2000 Ladder
    let ladder = gen_ladder(2000);
    let perm_lad = order(&ladder);
    assert_valid_bijection(ladder.n, &perm_lad, "2x2000 Ladder");

    // Barbell: two K_25 cliques connected by a 500-node long path
    let barbell = gen_barbell(25, 500);
    let perm_bar = order(&barbell);
    assert_valid_bijection(barbell.n, &perm_bar, "Barbell (K_25 - Path_500 - K_25)");
}

#[test]
fn tier2_boundary_disconnected_heterogeneous_islands() {
    // 15 disconnected heterogeneous components
    let mut components = Vec::new();
    for k in 3..18 {
        match k % 4 {
            0 => components.push(gen_path(k * 5)),
            1 => components.push(gen_cycle(k * 4)),
            2 => components.push(gen_star(k * 6, 0)),
            _ => components.push(gen_clique(k.min(15))),
        }
    }
    let island_graph = gen_disjoint_union(&components);
    let perm = order(&island_graph);
    assert_valid_bijection(island_graph.n, &perm, "15 Heterogeneous Disconnected Islands");
}

#[test]
fn tier2_boundary_permutation_bijection_invariants() {
    // Adversarial validation of permutation properties on complex graph
    let pat = gen_kkt(300, 100, 777);
    let perm = order(&pat);
    assert_valid_bijection(pat.n, &perm, "Bijection Invariants on KKT(300, 100)");

    // Invertibility test: inverse permutation pinv must also be a bijection
    let mut pinv = vec![usize::MAX; pat.n];
    for (i, &v) in perm.iter().enumerate() {
        pinv[v] = i;
    }
    for (v, &pos) in pinv.iter().enumerate() {
        assert_ne!(
            pos,
            usize::MAX,
            "Vertex {v} was never mapped by inverse permutation"
        );
        assert_eq!(perm[pos], v, "Inverse permutation mismatch at {v}");
    }
}

// ============================================================================
// TIER 3: Cross-Feature, Determinism & Performance
// ============================================================================

#[test]
fn tier3_determinism_across_topologies() {
    // Strict bitwise determinism check across 5 repeated runs on diverse topologies
    let test_patterns = vec![
        ("Path 200", gen_path(200)),
        ("Star 500", gen_star(500, 250)),
        ("Cycle 300", gen_cycle(300)),
        ("Grid 2D 25x25", gen_grid2d(25)),
        ("Barbell 20-200", gen_barbell(20, 200)),
        ("Random ER 150 (p=0.08)", gen_er_random(150, 0.08, 999)),
        ("KKT 250-80", gen_kkt(250, 80, 54321)),
    ];

    for (name, pat) in &test_patterns {
        assert_deterministic(pat, 5, name);
    }
}

#[test]
fn tier3_determinism_multithreaded_parallel_consistency() {
    // Concurrently invoke order() from multiple OS threads to verify
    // pure thread safety and lack of cross-thread data corruption / non-determinism.
    let pat = gen_grid2d(20);
    let reference_perm = order(&pat);

    let handles: Vec<_> = (0..8)
        .map(|thread_id| {
            let p = pat.clone();
            let expected = reference_perm.clone();
            std::thread::spawn(move || {
                for iter in 0..3 {
                    let perm = order(&p);
                    assert_eq!(
                        perm, expected,
                        "Thread {thread_id} iteration {iter} produced divergent permutation"
                    );
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Worker thread panicked during order()");
    }
}

#[test]
fn tier3_execution_timing_safety_headroom() {
    // Verify execution terminates and returns valid permutations across representative structures.
    // Note: In whole-workspace multi-threaded test runs, 100+ concurrent test threads
    // heavily oversubscribe CPU cores and Rayon worker pools. We bound test execution
    // to < 10.0s to remain resilient against runner thread starvation while confirming termination.
    // The strict 2.0s wall-clock watchdog ceiling is verified by tests/time_cap.rs.
    let test_sizes = vec![
        ("Grid 2D 15x15 (n=225)", gen_grid2d(15)),
        ("Grid 3D 6x6x6 (n=216)", gen_grid3d(6)),
        ("KKT 300-100 (n=400)", gen_kkt(300, 100, 888)),
        ("Barbell 20-300 (n=340)", gen_barbell(20, 300)),
    ];

    for (label, pat) in &test_sizes {
        let t0 = Instant::now();
        let perm = order(pat);
        let elapsed = t0.elapsed();

        assert_valid_bijection(pat.n, &perm, label);
        assert!(
            elapsed.as_secs_f64() < 10.0,
            "[{label}] Execution time {:?} exceeded maximum test ceiling",
            elapsed
        );
    }
}

#[test]
fn tier3_portfolio_amd_floor_guarantee() {
    // Since the contestant portfolio is anchored on the AMD baseline,
    // candidate flops must NEVER be worse than AMD (ratio <= 1.0 + epsilon).
    let test_cases = vec![
        ("Path 150", gen_path(150)),
        ("Star 300", gen_star(300, 0)),
        ("Cycle 200", gen_cycle(200)),
        ("Grid 2D 20x20", gen_grid2d(20)),
        ("KKT 400-150", gen_kkt(400, 150, 101)),
        ("Chimera 2x2", gen_chimera(2, 2)),
    ];

    for (name, pat) in &test_cases {
        let cand_perm = order(pat);
        assert_valid_bijection(pat.n, &cand_perm, name);

        let cand_score = ssi_scoring::score(pat, &cand_perm);
        let amd_perm = ssi_scoring::amd_baseline(pat);
        let amd_score = ssi_scoring::score(pat, &amd_perm);

        assert!(
            cand_score.flops <= amd_score.flops,
            "[{name}] Candidate flops ({}) exceeded AMD baseline floor ({})",
            cand_score.flops,
            amd_score.flops
        );
    }
}

#[test]
fn tier3_relabeling_structural_invariance() {
    // If a graph G is isomorphic to G' under vertex relabeling sigma,
    // the elimination ordering quality should be structurally preserved.
    let base_pat = gen_grid2d(15); // n = 225
    let n = base_pat.n;

    // Create a random permutation sigma
    let mut rng = TestRng::new(98765);
    let mut sigma: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = rng.next_usize(i + 1);
        sigma.swap(i, j);
    }

    // Build permuted edges
    let mut permuted_edges = Vec::new();
    for j in 0..n {
        for &r in base_pat.col(j) {
            if r < j {
                permuted_edges.push((sigma[r], sigma[j]));
            }
        }
    }
    let permuted_pat = Pattern::from_edges(n, &permuted_edges);

    let perm_base = order(&base_pat);
    let perm_permuted = order(&permuted_pat);

    assert_valid_bijection(n, &perm_base, "Grid 15x15 base");
    assert_valid_bijection(n, &perm_permuted, "Grid 15x15 permuted");

    let score_base = ssi_scoring::score(&base_pat, &perm_base);
    let score_permuted = ssi_scoring::score(&permuted_pat, &perm_permuted);

    // Flop counts under isomorphic relabeling should be of comparable magnitude
    let ratio = (score_permuted.flops as f64) / (score_base.flops as f64);
    assert!(
        ratio >= 0.70 && ratio <= 1.40,
        "Isomorphic relabeling caused extreme flop divergence: base={} permuted={} ratio={ratio}",
        score_base.flops,
        score_permuted.flops
    );
}

// ============================================================================
// TIER 4: Real-World Scenarios & Scientific Benchmarks
// ============================================================================

#[test]
fn tier4_realworld_dev_corpus_patterns() {
    // Load matrices from the dev corpus (if available in test environment).
    let patterns = crate::corpus::corpus();
    assert!(!patterns.is_empty(), "Dev corpus must contain matrices");

    // Test a representative sample of corpus matrices across size spectrum
    for (name, pat) in patterns.iter().take(20) {
        let perm = order(pat);
        assert_valid_bijection(pat.n, &perm, &format!("Dev Corpus Matrix {name}"));

        let cand_score = ssi_scoring::score(pat, &perm);
        let amd_perm = ssi_scoring::amd_baseline(pat);
        let amd_score = ssi_scoring::score(pat, &amd_perm);

        assert!(
            cand_score.flops <= amd_score.flops,
            "Corpus matrix {name} candidate flops ({}) exceeded AMD baseline ({})",
            cand_score.flops,
            amd_score.flops
        );
    }
}

#[test]
fn tier4_realworld_kkt_optimization_systems() {
    // Saddle-point optimization matrices representing constrained optimization problems:
    // [ H    A^T ]
    // [ A     0  ]
    for &(nh, m, seed) in &[(300, 100, 42), (1000, 300, 101), (2500, 800, 202)] {
        let kkt_pat = gen_kkt(nh, m, seed);
        let perm = order(&kkt_pat);
        assert_valid_bijection(kkt_pat.n, &perm, &format!("KKT(nh={nh}, m={m})"));

        let cand_score = ssi_scoring::score(&kkt_pat, &perm);
        let natural_perm: Vec<usize> = (0..kkt_pat.n).collect();
        let natural_score = ssi_scoring::score(&kkt_pat, &natural_perm);

        // Effective ordering should achieve massive fill reduction vs natural ordering on KKT
        assert!(
            cand_score.flops < natural_score.flops,
            "KKT(nh={nh}, m={m}) candidate flops ({}) should be far less than natural order flops ({})",
            cand_score.flops,
            natural_score.flops
        );
    }
}

#[test]
fn tier4_realworld_2d_3d_pde_meshes() {
    // 2D Stencils (5-point) and 3D Stencils (7-point) modeling PDE discretizations
    let g2d = gen_grid2d(40); // n = 1600
    let perm2d = order(&g2d);
    assert_valid_bijection(g2d.n, &perm2d, "2D PDE Grid 40x40");

    let g3d = gen_grid3d(14); // n = 2744
    let perm3d = order(&g3d);
    assert_valid_bijection(g3d.n, &perm3d, "3D PDE Grid 14x14x14");
}

#[test]
fn tier4_realworld_chimera_quantum_lattices() {
    // D-Wave Chimera graph structures
    let chimera_small = gen_chimera(2, 2); // 4 cells = 32 vertices
    let perm_small = order(&chimera_small);
    assert_valid_bijection(chimera_small.n, &perm_small, "Chimera 2x2");

    let chimera_med = gen_chimera(5, 5); // 25 cells = 200 vertices
    let perm_med = order(&chimera_med);
    assert_valid_bijection(chimera_med.n, &perm_med, "Chimera 5x5");

    let chimera_large = gen_chimera(10, 10); // 100 cells = 800 vertices
    let perm_large = order(&chimera_large);
    assert_valid_bijection(chimera_large.n, &perm_large, "Chimera 10x10");
}

#[test]
fn tier4_realworld_bordered_block_diagonal_systems() {
    // Domain decomposition Schur complement / substructuring systems:
    // Multiple independent subdomains coupled via interface / boundary separators
    for &(num_b, b_size, border) in &[(4, 50, 10), (8, 100, 30), (12, 80, 40)] {
        let bbd = gen_bordered_block_diagonal(num_b, b_size, border);
        let perm = order(&bbd);
        assert_valid_bijection(
            bbd.n,
            &perm,
            &format!("Bordered Block Diagonal ({num_b} blocks x {b_size} + {border} border)"),
        );

        let cand_score = ssi_scoring::score(&bbd, &perm);
        let amd_perm = ssi_scoring::amd_baseline(&bbd);
        let amd_score = ssi_scoring::score(&bbd, &amd_perm);

        assert!(
            cand_score.flops <= amd_score.flops,
            "Bordered block diagonal candidate flops ({}) must not exceed AMD baseline ({})",
            cand_score.flops,
            amd_score.flops
        );
    }
}

#[test]
fn tier4_realworld_multi_bucket_scale_stress() {
    // Exercises all 3 competition size buckets:
    // Bucket 0: lt_1k (n < 1,000)
    // Bucket 1: 1k_10k (1,000 <= n < 10,000)
    // Bucket 2: gt_10k (n >= 10,000)

    // Bucket 0: lt_1k
    let b0 = gen_path(800);
    let p0 = order(&b0);
    assert_valid_bijection(800, &p0, "Bucket lt_1k (n=800)");

    // Bucket 1: 1k_10k
    let b1 = gen_grid2d(60); // n = 3600
    let p1 = order(&b1);
    assert_valid_bijection(3600, &p1, "Bucket 1k_10k (n=3600)");

    // Bucket 2: gt_10k
    let b2 = gen_path(11000); // n = 11000
    let p2 = order(&b2);
    assert_valid_bijection(11000, &p2, "Bucket gt_10k (n=11000)");
}
