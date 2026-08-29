//! Twin compression and simplicial peeling — exact preprocessing for Σ c_j².
//!
//! True twins (identical open neighborhoods) and false twins (same neighborhood,
//! non-adjacent) merge into supervertices; simplicial vertices (neighborhood a
//! clique among live vertices) are peeled to the front. Expanding twin classes
//! consecutively preserves the exact column-count objective.

use feral_ordering_core::OrderingError;

use super::ScoringPattern;

/// Result of preprocessing: a smaller pattern plus maps to expand orderings.
#[derive(Clone)]
pub(crate) struct CompressResult {
    pub(crate) orig_n: usize,
    pub(crate) peel_prefix: Vec<usize>,
    pub(crate) compressed: ScoringPattern,
    /// `groups[c]` lists original vertex indices for compressed vertex `c`.
    pub(crate) groups: Vec<Vec<usize>>,
}

/// Build sorted adjacency lists (no self-loops).
fn build_adj(sp: &ScoringPattern) -> Vec<Vec<u32>> {
    let n = sp.n;
    let mut adj = vec![Vec::new(); n];
    for j in 0..n {
        for k in sp.col_ptr[j]..sp.col_ptr[j + 1] {
            let i = sp.row_idx[k];
            if i != j {
                adj[j].push(i as u32);
                adj[i].push(j as u32);
            }
        }
    }
    for a in adj.iter_mut() {
        a.sort_unstable();
        a.dedup();
    }
    adj
}

/// True when every pair of active neighbors of `v` is adjacent.
fn is_simplicial(v: usize, adj: &[Vec<u32>], active: &[bool]) -> bool {
    let mut nbrs: Vec<u32> = adj[v]
        .iter()
        .copied()
        .filter(|&u| active[u as usize])
        .collect();
    if nbrs.len() <= 1 {
        return true;
    }
    nbrs.sort_unstable();
    for i in 0..nbrs.len() {
        for j in (i + 1)..nbrs.len() {
            let a = nbrs[i] as usize;
            let b = nbrs[j] as usize;
            if adj[a].binary_search(&nbrs[j]).is_err() {
                return false;
            }
        }
    }
    true
}

/// Repeatedly peel all simplicial vertices; return peel order and live mask.
fn peel_simplicial(adj: &[Vec<u32>], n: usize) -> (Vec<usize>, Vec<bool>) {
    let mut active = vec![true; n];
    let mut peel = Vec::new();
    loop {
        let mut batch = Vec::new();
        for v in 0..n {
            if active[v] && is_simplicial(v, adj, &active) {
                batch.push(v);
            }
        }
        if batch.is_empty() {
            break;
        }
        batch.sort_unstable();
        for v in batch {
            active[v] = false;
            peel.push(v);
        }
    }
    (peel, active)
}

/// Group live vertices with identical neighbor lists among live vertices.
fn twin_groups(adj: &[Vec<u32>], active: &[bool]) -> Vec<Vec<usize>> {
    use std::collections::HashMap;
    let mut sig_to_group: HashMap<Vec<u32>, usize> = HashMap::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for v in 0..adj.len() {
        if !active[v] {
            continue;
        }
        let sig: Vec<u32> = adj[v]
            .iter()
            .copied()
            .filter(|&u| active[u as usize])
            .collect();
        if let Some(&gi) = sig_to_group.get(&sig) {
            groups[gi].push(v);
        } else {
            let gi = groups.len();
            sig_to_group.insert(sig, gi);
            groups.push(vec![v]);
        }
    }
    groups
}

/// Build CSC pattern on compressed vertex set (one supervertex per twin group).
fn build_compressed_pattern(adj: &[Vec<u32>], groups: &[Vec<usize>], active: &[bool]) -> ScoringPattern {
    let nc = groups.len();
    if nc == 0 {
        return ScoringPattern {
            n: 0,
            col_ptr: vec![0],
            row_idx: Vec::new(),
        };
    }
    let mut orig_to_comp = vec![usize::MAX; adj.len()];
    for (c, g) in groups.iter().enumerate() {
        for &v in g {
            orig_to_comp[v] = c;
        }
    }
    let mut col_ptr = vec![0usize; nc + 1];
    let mut row_idx = Vec::new();
    for c in 0..nc {
        let mut nbrs: Vec<usize> = Vec::new();
        for &v in &groups[c] {
            for &u in &adj[v] {
                let u = u as usize;
                if active[u] {
                    let cu = orig_to_comp[u];
                    if cu != c {
                        nbrs.push(cu);
                    }
                }
            }
        }
        nbrs.sort_unstable();
        nbrs.dedup();
        for u in nbrs {
            row_idx.push(u);
        }
        col_ptr[c + 1] = row_idx.len();
    }
    ScoringPattern {
        n: nc,
        col_ptr,
        row_idx,
    }
}

/// Preprocess pattern: simplicial peel + twin merge. Returns `None` only when
/// compression yields no reduction (all groups size 1 and no peel).
pub(crate) fn preprocess(sp: &ScoringPattern) -> Option<CompressResult> {
    let n = sp.n;
    if n == 0 {
        return None;
    }
    let adj = build_adj(sp);
    let (peel_prefix, active) = peel_simplicial(&adj, n);
    let groups = twin_groups(&adj, &active);
    let nc = groups.len();
    if nc == 0 {
        return None;
    }
    let nontrivial = peel_prefix.len() > 0 || groups.iter().any(|g| g.len() > 1) || nc < n;
    if !nontrivial {
        return None;
    }
    let compressed = build_compressed_pattern(&adj, &groups, &active);
    Some(CompressResult {
        orig_n: n,
        peel_prefix,
        compressed,
        groups,
    })
}

impl CompressResult {
    /// Map compressed elimination order back to original indices (twins consecutive).
    pub(crate) fn expand_perm_i32(&self, compressed_perm: &[usize]) -> Result<Vec<i32>, OrderingError> {
        if compressed_perm.len() != self.compressed.n {
            return Err(OrderingError::MalformedInput);
        }
        let mut seen = vec![false; self.orig_n];
        let mut perm = Vec::with_capacity(self.orig_n);
        for &v in &self.peel_prefix {
            if v >= self.orig_n || seen[v] {
                return Err(OrderingError::MalformedInput);
            }
            seen[v] = true;
            perm.push(v as i32);
        }
        for &cv in compressed_perm {
            if cv >= self.groups.len() {
                return Err(OrderingError::MalformedInput);
            }
            for &ov in &self.groups[cv] {
                if ov >= self.orig_n || seen[ov] {
                    return Err(OrderingError::MalformedInput);
                }
                seen[ov] = true;
                perm.push(ov as i32);
            }
        }
        if perm.len() != self.orig_n || !seen.iter().all(|&s| s) {
            return Err(OrderingError::MalformedInput);
        }
        Ok(perm)
    }
}
