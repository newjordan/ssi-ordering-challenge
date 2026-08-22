//! Owned i32 CSC pattern utilities: relabelling (Q·A·Qᵀ), induced subgraphs,
//! degrees, and permutation composition. All O(n + nnz), allocation-light.

use crate::Pattern;

/// Owned, i32-indexed, full-symmetric, diagonal-free CSC pattern with sorted
/// row indices per column (the invariants every feral ordering crate requires).
#[derive(Clone, Debug)]
pub struct Csc {
    pub n: usize,
    pub col_ptr: Vec<i32>,
    pub row_idx: Vec<i32>,
}

impl Csc {
    pub fn from_pattern(p: &Pattern) -> Csc {
        Csc {
            n: p.n,
            col_ptr: p.col_ptr.iter().map(|&x| x as i32).collect(),
            row_idx: p.row_idx.iter().map(|&x| x as i32).collect(),
        }
    }

    #[inline]
    pub fn nnz(&self) -> usize {
        self.row_idx.len()
    }

    #[inline]
    pub fn col(&self, j: usize) -> &[i32] {
        &self.row_idx[self.col_ptr[j] as usize..self.col_ptr[j + 1] as usize]
    }

    /// Borrowed view for the feral ordering crates.
    pub fn view(&self) -> feral_ordering_core::CscPattern<'_> {
        feral_ordering_core::CscPattern::new(self.n, &self.col_ptr, &self.row_idx)
            .expect("Csc invariants hold")
    }

    pub fn degrees(&self) -> Vec<u32> {
        (0..self.n)
            .map(|j| (self.col_ptr[j + 1] - self.col_ptr[j]) as u32)
            .collect()
    }

    /// Relabel: vertex `i` becomes `q[i]` (q is a bijection of 0..n). Returns
    /// the pattern of Q·A·Qᵀ with sorted rows (counting-sort construction).
    pub fn relabel(&self, q: &[u32]) -> Csc {
        let n = self.n;
        debug_assert_eq!(q.len(), n);
        let mut col_ptr = vec![0i32; n + 1];
        for j in 0..n {
            let deg = self.col_ptr[j + 1] - self.col_ptr[j];
            col_ptr[q[j] as usize + 1] = deg;
        }
        for j in 0..n {
            col_ptr[j + 1] += col_ptr[j];
        }
        let nnz = self.nnz();
        let mut row_idx = vec![0i32; nnz];
        let mut fill: Vec<i32> = col_ptr[..n].to_vec();
        // Emit entries in increasing NEW row order so each column's rows come
        // out sorted: iterate old columns in order of their new label. That is
        // a scatter over inv(q); build inv once.
        let mut inv = vec![0u32; n];
        for (old, &new) in q.iter().enumerate() {
            inv[new as usize] = old as u32;
        }
        for new_r in 0..n {
            let old_r = inv[new_r] as usize;
            // old_r's column lists its neighbours old_c; symmetric pattern, so
            // (old_r, old_c) present ⇔ (old_c, old_r) present. For each
            // neighbour old_c, the new column q[old_c] receives new row new_r.
            for &old_c in self.col(old_r) {
                let nc = q[old_c as usize] as usize;
                row_idx[fill[nc] as usize] = new_r as i32;
                fill[nc] += 1;
            }
        }
        Csc { n, col_ptr, row_idx }
    }

    /// Induced subgraph on the vertices with `keep[v]`. Returns the subgraph
    /// (vertices renumbered in increasing original order) and the map
    /// new → old.
    pub fn induced(&self, keep: &[bool]) -> (Csc, Vec<u32>) {
        let n = self.n;
        let mut new_of = vec![u32::MAX; n];
        let mut old_of: Vec<u32> = Vec::new();
        for v in 0..n {
            if keep[v] {
                new_of[v] = old_of.len() as u32;
                old_of.push(v as u32);
            }
        }
        let m = old_of.len();
        let mut col_ptr = Vec::with_capacity(m + 1);
        let mut row_idx = Vec::with_capacity(self.nnz());
        col_ptr.push(0i32);
        for &ov in &old_of {
            for &r in self.col(ov as usize) {
                let nr = new_of[r as usize];
                if nr != u32::MAX {
                    row_idx.push(nr as i32);
                }
            }
            col_ptr.push(row_idx.len() as i32);
        }
        (Csc { n: m, col_ptr, row_idx }, old_of)
    }
}

/// Inverse of a bijection given as `q[old] = new`: returns `inv[new] = old`.
pub fn invert_u32(q: &[u32]) -> Vec<u32> {
    let mut inv = vec![0u32; q.len()];
    for (old, &new) in q.iter().enumerate() {
        inv[new as usize] = old as u32;
    }
    inv
}

/// Compose a permutation returned on a relabelled graph (`perm_new[k]` = new
/// index eliminated k-th) back to original indices.
pub fn compose_relabel(perm_new: &[i32], inv_q: &[u32]) -> Vec<usize> {
    perm_new.iter().map(|&x| inv_q[x as usize] as usize).collect()
}

/// Whether `perm` is a bijection of `0..n`.
pub fn is_bijection(perm: &[usize], n: usize) -> bool {
    if perm.len() != n {
        return false;
    }
    let mut seen = vec![false; n];
    for &v in perm {
        if v >= n || seen[v] {
            return false;
        }
        seen[v] = true;
    }
    true
}

/// Reverse Cuthill–McKee-style BFS labelling (pseudo-peripheral start per
/// component, neighbours visited in ascending degree). Returns `q[old] = new`.
pub fn rcm_labels(g: &Csc, deg: &[u32], reverse: bool) -> Vec<u32> {
    let n = g.n;
    let mut order: Vec<u32> = Vec::with_capacity(n);
    let mut visited = vec![false; n];
    let mut by_deg: Vec<usize> = (0..n).collect();
    by_deg.sort_unstable_by_key(|&v| (deg[v], v));
    let mut nbrs: Vec<u32> = Vec::new();
    for &start in &by_deg {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let head = order.len();
        order.push(start as u32);
        let mut i = head;
        while i < order.len() {
            let v = order[i] as usize;
            i += 1;
            nbrs.clear();
            for &r in g.col(v) {
                let r = r as usize;
                if !visited[r] {
                    visited[r] = true;
                    nbrs.push(r as u32);
                }
            }
            nbrs.sort_unstable_by_key(|&u| (deg[u as usize], u));
            order.extend_from_slice(&nbrs);
        }
    }
    let mut q = vec![0u32; n];
    if reverse {
        for (k, &v) in order.iter().enumerate() {
            q[v as usize] = (n - 1 - k) as u32;
        }
    } else {
        for (k, &v) in order.iter().enumerate() {
            q[v as usize] = k as u32;
        }
    }
    q
}
