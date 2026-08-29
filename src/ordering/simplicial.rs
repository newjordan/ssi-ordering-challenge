//! SIMPLICIAL PRE-PEELING (Exact deterministic zero-fill deduction).
//!
//! Mathematically proven (Dirac 1961, Rose 1972):
//! If a vertex v is simplicial in G (i.e. N(v) induces a clique), eliminating
//! v first creates ZERO fill-in (Def(v) = ∅) and leaves the fill of all other
//! vertices unchanged or reduced.
//!
//! This module iteratively peels:
//! 1. Isolated vertices (deg 0): c_v = 1.
//! 2. Pendant vertices / leaves (deg 1): c_v = 2.
//! 3. Triangle nodes (deg 2 with adjacent neighbors): c_v = 3.
//! 4. Low-degree simplicial cliques (deg <= 4 where N(v) is a complete graph).

use crate::Pattern;

/// Peel simplicial vertices and return an elimination ordering using AMD on the core.
pub fn simplicial_amd_order(pattern: &Pattern) -> Vec<i32> {
    simplicial_order_core(pattern, false, 1.0)
}

/// Peel simplicial vertices and return an elimination ordering using AMF on the core.
pub fn simplicial_amf_order(pattern: &Pattern, alpha: f64) -> Vec<i32> {
    simplicial_order_core(pattern, true, alpha)
}

fn simplicial_order_core(pattern: &Pattern, use_amf: bool, alpha: f64) -> Vec<i32> {
    let n = pattern.n;
    if n <= 1 {
        return (0..n as i32).collect();
    }

    // Build symmetric adjacency (sorted ascending per vertex, excluding self-loops)
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    for j in 0..n {
        for &i in pattern.col(j) {
            if i != j && i < n {
                adj[j].push(i as u32);
            }
        }
    }
    for list in adj.iter_mut() {
        list.sort_unstable();
        list.dedup();
    }

    let mut deg: Vec<usize> = adj.iter().map(|l| l.len()).collect();
    let mut in_graph: Vec<bool> = vec![true; n];
    let mut peeled: Vec<i32> = Vec::with_capacity(n);

    // Queue of candidates to check for simplicial elimination
    let mut queue: Vec<usize> = Vec::new();
    for v in 0..n {
        if deg[v] <= 2 {
            queue.push(v);
        }
    }

    // Helper closure: check if neighbor set of v in active graph forms a clique
    let is_clique = |v: usize, adj: &[Vec<u32>], in_graph: &[bool]| -> bool {
        let active_neighbors: Vec<usize> = adj[v]
            .iter()
            .map(|&u| u as usize)
            .filter(|&u| in_graph[u])
            .collect();
        let k = active_neighbors.len();
        if k <= 1 {
            return true;
        }
        if k > 6 {
            return false;
        }
        for i in 0..k {
            let u1 = active_neighbors[i];
            for j in (i + 1)..k {
                let u2 = active_neighbors[j];
                if adj[u1].binary_search(&(u2 as u32)).is_err() {
                    return false;
                }
            }
        }
        true
    };

    let mut q_head = 0;
    while q_head < queue.len() {
        let v = queue[q_head];
        q_head += 1;

        if !in_graph[v] {
            continue;
        }

        let d = deg[v];
        if d == 0 {
            in_graph[v] = false;
            peeled.push(v as i32);
        } else if d == 1 {
            in_graph[v] = false;
            peeled.push(v as i32);
            for &u in &adj[v] {
                let u = u as usize;
                if in_graph[u] {
                    deg[u] = deg[u].saturating_sub(1);
                    if deg[u] <= 2 {
                        queue.push(u);
                    }
                }
            }
        } else if d <= 4 && is_clique(v, &adj, &in_graph) {
            in_graph[v] = false;
            peeled.push(v as i32);
            for &u in &adj[v] {
                let u = u as usize;
                if in_graph[u] {
                    deg[u] = deg[u].saturating_sub(1);
                    if deg[u] <= 2 {
                        queue.push(u);
                    }
                }
            }
        }
    }

    if peeled.len() == n {
        return peeled;
    }

    // Build induced subgraph on remaining active vertices (V_core)
    let core_vertices: Vec<usize> = (0..n).filter(|&v| in_graph[v]).collect();
    let n_core = core_vertices.len();
    let mut global_to_local = vec![usize::MAX; n];
    for (loc, &glob) in core_vertices.iter().enumerate() {
        global_to_local[glob] = loc;
    }

    let mut core_col_ptr: Vec<i32> = Vec::with_capacity(n_core + 1);
    let mut core_row_idx: Vec<i32> = Vec::new();
    core_col_ptr.push(0);

    for &glob in &core_vertices {
        let mut count = 0;
        for &i in pattern.col(glob) {
            if i < n && in_graph[i] {
                let local_row = global_to_local[i];
                core_row_idx.push(local_row as i32);
                count += 1;
            }
        }
        core_col_ptr.push(core_col_ptr.last().unwrap() + count);
    }

    let mut final_perm = peeled;
    if let Some(core_pat) = feral_ordering_core::CscPattern::new(n_core, &core_col_ptr, &core_row_idx) {
        if use_amf {
            let opts = feral_amf::AmfOptions {
                dense_alpha: alpha,
                ..Default::default()
            };
            if let Ok((amf_core, ..)) = feral_amf::amf_order_opts(&core_pat, &opts) {
                for loc in amf_core {
                    final_perm.push(core_vertices[loc as usize] as i32);
                }
                return final_perm;
            }
        } else if let Ok(amd_core) = feral_amd::amd_order(&core_pat) {
            for loc in amd_core {
                final_perm.push(core_vertices[loc as usize] as i32);
            }
            return final_perm;
        }
    }

    for v in core_vertices {
        final_perm.push(v as i32);
    }
    final_perm
}
