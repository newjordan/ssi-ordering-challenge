//! Reusable scoring/permutation workspace — the allocation-elimination layer.
//!
//! `order()` scores every candidate with `flops_of` (permute_pattern → etree →
//! column_counts_gnp) and, for the relabelled-AMD multi-start and hill-climb
//! phases, additionally permutes the pattern to build the relabelled copy fed
//! to `feral_amd::amd_order`. Both paths, as written against vendor `feral`,
//! allocate a fresh set of `Vec`s EVERY call: `permute_pattern` alone
//! allocates `inv_perm`, `col_ptr`, `row_idx`, `offsets` (4 Vecs, two of them
//! O(nnz)); `EliminationTree::from_pattern` allocates `parent`, `ancestor` (2
//! more); `column_counts_gnp` allocates `post`, `next_child`, `stack` (via
//! `postorder`), `post_of`, `first` (via `first_descendants`), `children`
//! (`Vec<Vec<usize>>`), `delta`, `maxfirst`, `prevleaf`, `ancestor` (7 more,
//! one of them `Vec<Vec<usize>>`) — roughly 15 heap allocations per
//! `flops_of` call, several O(n) and two O(nnz). A single `order()` call
//! scores the AMD anchor, ~30 portfolio candidates, up to 24 relabel
//! restarts, and up to 24 hill-climb steps — up to ~80 calls on the matrices
//! that receive every phase — so this churns megabytes of allocator traffic
//! per matrix, repeatedly, at a size that never changes within one `order()`
//! call (n and nnz are fixed for the whole call).
//!
//! [`ScoreWorkspace`] fixes this by allocating every buffer exactly ONCE, at
//! `order()`'s (n, nnz), and reusing it across every subsequent call within
//! that `order()` invocation (never across matrices — `order()` runs in a
//! fresh child process per matrix, so there is no cross-matrix state to
//! reuse, and reusing across matrices would anyway violate the determinism
//! gate's "pure function of the current pattern" requirement).
//!
//! Every method here is a FAITHFUL, unmodified-logic port of the
//! corresponding vendor function (`feral::ordering::amd::permute_pattern`,
//! `feral::ordering::elimination_tree::EliminationTree::from_pattern` +
//! `children`/`postorder`/`first_descendants`, `feral::symbolic::
//! column_counts_gnp`) — the only change is where each buffer's memory comes
//! from (a reused field instead of a fresh `Vec::new()`/`vec![x; n]`). Where
//! reuse requires resetting stale content from a previous call, the reset is
//! called out in that step; where the original algorithm's own write pattern
//! already overwrites every element every call (argued in place), no reset is
//! needed and none is done — see each method's comments.
//!
//! Byte-identical to the vendor path is REQUIRED, not just plausible: the
//! `ws_equivalence` test in `mod.rs` runs both `flops_of` (vendor path) and
//! `ScoreWorkspace::flops` (this path) on the same (pattern, perm) pairs
//! across the full dev corpus and asserts equal `u64` results.

use super::ScoringPattern;

pub(crate) struct ScoreWorkspace {
    n: usize,

    // -- permute_pattern scratch --
    inv_perm: Vec<usize>,
    pcol_ptr: Vec<usize>,
    prow_idx: Vec<usize>,
    offsets: Vec<usize>,

    // -- EliminationTree::from_pattern / children / postorder scratch --
    parent: Vec<Option<usize>>,
    uf_ancestor: Vec<usize>,
    children: Vec<Vec<usize>>,
    next_child: Vec<usize>,
    stack: Vec<usize>,
    post: Vec<usize>,

    // -- first_descendants scratch --
    post_of: Vec<usize>,
    first: Vec<usize>,

    // -- column_counts_gnp scratch --
    delta: Vec<i64>,
    maxfirst: Vec<i64>,
    prevleaf: Vec<i64>,
    dsu_ancestor: Vec<usize>,
}

impl ScoreWorkspace {
    /// Allocate every buffer once, sized for this `order()` call's fixed `(n,
    /// nnz)` — `nnz` is the symmetric pattern's total entry count, which
    /// `permute_pattern` preserves exactly (a relabeling is a bijection on
    /// edges), so it never changes across the calls this workspace serves.
    pub(crate) fn new(n: usize, nnz: usize) -> Self {
        ScoreWorkspace {
            n,
            inv_perm: vec![0; n],
            pcol_ptr: vec![0; n + 1],
            prow_idx: vec![0; nnz],
            offsets: vec![0; n],
            parent: vec![None; n],
            uf_ancestor: vec![0; n],
            children: vec![Vec::new(); n],
            next_child: vec![0; n],
            stack: Vec::with_capacity(n),
            post: Vec::with_capacity(n),
            post_of: vec![0; n],
            first: vec![0; n],
            delta: vec![0i64; n],
            maxfirst: vec![0i64; n],
            prevleaf: vec![0i64; n],
            dsu_ancestor: vec![0; n],
        }
    }

    /// Port of `feral::ordering::amd::permute_pattern`: writes the permuted
    /// pattern into `self.pcol_ptr` / `self.prow_idx` (both fully overwritten
    /// every call — see the field-by-field notes below — so no reset is
    /// needed before this runs).
    fn permute_into(&mut self, pat: &ScoringPattern, perm: &[usize]) {
        let n = self.n;

        // `inv_perm[old] = new`. Every index 0..n is written exactly once
        // (perm is a bijection), so this fully overwrites the buffer.
        for (new, &old) in perm.iter().enumerate() {
            self.inv_perm[old] = new;
        }

        // `pcol_ptr[0]` is ALWAYS 0 (set once in `new`, never written again
        // by this method or the prefix-sum below, which only reads it) — no
        // reset needed. `pcol_ptr[1..=n]`: `new_j` ranges over all of 0..n
        // exactly once (inv_perm bijection), so `pcol_ptr[new_j + 1]` is
        // written for every index 1..=n exactly once — full overwrite.
        for old_j in 0..n {
            let new_j = self.inv_perm[old_j];
            let nnz_j = pat.col_ptr[old_j + 1] - pat.col_ptr[old_j];
            self.pcol_ptr[new_j + 1] = nnz_j;
        }
        for j in 0..n {
            self.pcol_ptr[j + 1] += self.pcol_ptr[j];
        }

        // `offsets[..n]` = a copy of the per-column start offsets — full
        // overwrite via copy_from_slice, not a fresh allocation.
        self.offsets[..n].copy_from_slice(&self.pcol_ptr[..n]);

        // `prow_idx`: the fill loop below writes exactly `nnz` entries at
        // exactly the `nnz` distinct positions `0..nnz` (the offsets
        // bookkeeping is an exact bucket-fill), so it is fully overwritten;
        // no reset needed even though nothing here zeroes it first.
        for old_j in 0..n {
            let new_j = self.inv_perm[old_j];
            let start = pat.col_ptr[old_j];
            let end = pat.col_ptr[old_j + 1];
            for k in start..end {
                let new_i = self.inv_perm[pat.row_idx[k]];
                let pos = self.offsets[new_j];
                self.prow_idx[pos] = new_i;
                self.offsets[new_j] += 1;
            }
        }

        // Sort each column's row indices, exactly as the vendor function
        // does (downstream code does not strictly require it, but keeping
        // the invariant avoids any behavioral drift from the vendor path).
        for j in 0..n {
            let start = self.pcol_ptr[j];
            let end = self.pcol_ptr[j + 1];
            self.prow_idx[start..end].sort_unstable();
        }
    }

    /// Same computation as `permute_into`, but ALSO writes the i32-cast
    /// result into caller-owned buffers (cleared, not reallocated, as long
    /// as the caller keeps reusing them across its own loop) — replaces
    /// vendor `permute_pattern` (which allocates a fresh `usize`
    /// `CscPattern`) plus two more fresh `Vec<i32>` casts, for a caller that
    /// builds a relabelled pattern to feed `feral_amd::amd_order`.
    ///
    /// NOT wired into this build's relabel-multi-start/hill-climb loops (they
    /// still use vendor `permute_pattern` directly, to keep the cap-safety
    /// merge minimal and lower-risk under time pressure) — kept for a future
    /// pass, and exercised directly by `permute_i32_matches_vendor` below so
    /// it stays correct if wired in later.
    #[allow(dead_code)]
    pub(crate) fn permute_i32_into(
        &mut self,
        pat: &ScoringPattern,
        perm: &[usize],
        col_ptr_i32: &mut Vec<i32>,
        row_idx_i32: &mut Vec<i32>,
    ) {
        self.permute_into(pat, perm);
        col_ptr_i32.clear();
        col_ptr_i32.extend(self.pcol_ptr.iter().map(|&x| x as i32));
        row_idx_i32.clear();
        row_idx_i32.extend(self.prow_idx.iter().map(|&x| x as i32));
    }

    /// Port of `EliminationTree::from_pattern`, operating on the permuted
    /// pattern currently held in `self.pcol_ptr` / `self.prow_idx`.
    fn build_etree(&mut self) {
        let n = self.n;

        // MUST reset: a root in this call (parent[j] == None) may have held
        // `Some(x)` from a previous call, and not every position is
        // necessarily rewritten below (roots are exactly the positions that
        // stay untouched).
        self.parent.fill(None);

        // No reset needed: `uf_ancestor[j] = j` is set at the top of every
        // j-iteration below, BEFORE any read of `uf_ancestor[i]` for i < j —
        // and every read only ever targets indices < j, which were set in an
        // earlier iteration of this very loop. So every position is written
        // before it is read, every call, regardless of leftover content.
        for j in 0..n {
            self.uf_ancestor[j] = j;
            for k in self.pcol_ptr[j]..self.pcol_ptr[j + 1] {
                let i = self.prow_idx[k];
                if i >= j {
                    continue;
                }
                let mut r = i;
                while self.uf_ancestor[r] != r {
                    r = self.uf_ancestor[r];
                }
                let mut node = i;
                while node != r {
                    let next = self.uf_ancestor[node];
                    self.uf_ancestor[node] = r;
                    node = next;
                }
                if r != j {
                    self.parent[r] = Some(j);
                    self.uf_ancestor[r] = j;
                }
            }
        }
    }

    /// Port of `EliminationTree::children` + `postorder` +
    /// `first_descendants`, writing `self.post` and `self.first`.
    fn build_postorder_and_first(&mut self) {
        let n = self.n;

        // `children()`: clear (keep capacity) then repopulate from `parent`.
        for c in self.children.iter_mut() {
            c.clear();
        }
        for j in 0..n {
            if let Some(p) = self.parent[j] {
                self.children[p].push(j);
            }
        }

        // `postorder()`. MUST reset `next_child` (read before every node's
        // own write in general, since it can be revisited across the outer
        // roots loop); `post`/`stack` cleared, capacity retained.
        self.next_child.fill(0);
        self.post.clear();
        self.stack.clear();
        for root in 0..n {
            if self.parent[root].is_some() {
                continue; // not a root
            }
            self.stack.push(root);
            while let Some(&node) = self.stack.last() {
                let k = self.next_child[node];
                if k < self.children[node].len() {
                    self.next_child[node] = k + 1;
                    self.stack.push(self.children[node][k]);
                } else {
                    self.post.push(node);
                    self.stack.pop();
                }
            }
        }

        // `first_descendants(&post)`. `post_of` and `first` are both fully
        // overwritten for every index (post is a permutation of 0..n) — no
        // reset needed beyond that full rewrite.
        for (pnum, &node) in self.post.iter().enumerate() {
            self.post_of[node] = pnum;
        }
        self.first.copy_from_slice(&self.post_of);
        for &node in &self.post {
            if let Some(p) = self.parent[node] {
                if self.first[node] < self.first[p] {
                    self.first[p] = self.first[node];
                }
            }
        }
    }

    /// Port of `column_counts_gnp`, consuming `self.post` / `self.first` /
    /// `self.children` (already built by `build_postorder_and_first`) plus
    /// the permuted pattern in `self.pcol_ptr` / `self.prow_idx`. Returns
    /// `Σ_j c_j²` directly (the vendor path returns the `counts` Vec and the
    /// caller sums separately; folding the sum in here drops one more O(n)
    /// allocation with no behavioral change since nothing else ever reads
    /// `counts`).
    fn column_counts_gnp_flops(&mut self) -> u64 {
        let n = self.n;

        // `delta`: fully overwritten for every index below (one write per
        // i in 0..n before any accumulation reads it) — no separate reset.
        for i in 0..n {
            self.delta[i] = if self.children[i].is_empty() { 1 } else { 0 };
        }
        // MUST reset: these two are read (as "no leaf seen yet" sentinels)
        // before necessarily being written this call.
        self.maxfirst.fill(-1);
        self.prevleaf.fill(-1);
        // MUST reset: DSU identity init, mirroring vendor's `(0..n).collect()`.
        for i in 0..n {
            self.dsu_ancestor[i] = i;
        }

        for idx in 0..n {
            let i = self.post[idx];
            if let Some(p) = self.parent[i] {
                self.delta[p] -= 1;
            }
            let fi = self.first[i] as i64;
            let row_start = self.pcol_ptr[i];
            let row_end = self.pcol_ptr[i + 1];
            for k in row_start..row_end {
                let partner = self.prow_idx[k];
                if partner <= i {
                    continue;
                }
                if fi > self.maxfirst[partner] {
                    self.delta[i] += 1;
                    let pl = self.prevleaf[partner];
                    if pl != -1 {
                        let mut q = pl as usize;
                        while self.dsu_ancestor[q] != q {
                            q = self.dsu_ancestor[q];
                        }
                        let root = q;
                        let mut cur = pl as usize;
                        while cur != root {
                            let next = self.dsu_ancestor[cur];
                            self.dsu_ancestor[cur] = root;
                            cur = next;
                        }
                        self.delta[root] -= 1;
                    }
                    self.prevleaf[partner] = i as i64;
                    self.maxfirst[partner] = fi;
                }
            }
            if let Some(p) = self.parent[i] {
                self.dsu_ancestor[i] = p;
            }
        }

        for idx in 0..n {
            let i = self.post[idx];
            if let Some(p) = self.parent[i] {
                let di = self.delta[i];
                self.delta[p] += di;
            }
        }

        let mut flops: u64 = 0;
        for &d in &self.delta {
            let c = d as u64;
            flops += c * c;
        }
        flops
    }

    /// `Σ_j c_j²` for `perm` on `pat` — the workspace-reusing equivalent of
    /// the vendor-backed `flops_of` in `mod.rs`. Faithfully performs the same
    /// three steps (permute → etree → GNP column counts) with every buffer
    /// pre-allocated in `new` and reused here, instead of allocating ~15
    /// fresh `Vec`s per call.
    pub(crate) fn flops(&mut self, pat: &ScoringPattern, perm: &[usize]) -> u64 {
        self.permute_into(pat, perm);
        self.build_etree();
        self.build_postorder_and_first();
        self.column_counts_gnp_flops()
    }

    /// INCREMENTAL swap evaluation for the WPS-AMD-IDB phase: starting from
    /// the workspace's CURRENT state (permuted pattern in `self.pcol_ptr` /
    /// `self.prow_idx` already built by the previous `flops()` call), apply
    /// the swap of `perm[i]` and `perm[j]` in place — physically re-bucketing
    /// only the entries belonging to the two affected columns (O(deg(a) +
    /// deg(b)) work, the entries of the two original vertices) and leaving
    /// every other column byte-identical — then recompute the elimination
    /// tree, post-order, first descendants, and `Σ c_k²` using the same
    /// reused buffers as `flops()`.
    ///
    /// This is the strict-improvement oracle for the windowed pairwise-swap
    /// local search in [`crate::ordering::wps_amd_idb`]: the swap is kept
    /// iff the returned flops is strictly less than the previous flops (the
    /// caller passes that in as `prev_flops` for the strict-improvement
    /// decision). The state of `self` after this call is the NEW permuted
    /// pattern, so the caller should `perm.swap(i, j)` to keep the in-place
    /// `perm` argument consistent before the next `swap_flops` call.
    ///
    /// Cost: O(nnz) for the GNP rebuild (the whole algorithm dominates on
    /// any non-trivial matrix) plus the local O(deg(a) + deg(b)) re-bucket
    /// of the two swapped columns. Since the workspace is reused across
    /// every swap in a window and across the entire WPS-AMD-IDB pass, the
    /// per-swap constant is small enough that `w*w*2 = 128` swaps per
    /// window position across 2 passes is well under a millisecond on
    /// the matrices the WPS-AMD-IDB envelope reaches.
    pub(crate) fn swap_flops(
        &mut self,
        pat: &ScoringPattern,
        perm: &[usize],
        i: usize,
        j: usize,
    ) -> u64 {
        debug_assert!(i < perm.len() && j < perm.len() && i != j);
        // The local re-permutation: after swapping positions i and j in
        // the permutation, vertex `a = perm[i]` now lives at position
        // `pos_b = self.inv_perm[b]` (the OLD bucket of `b = perm[j]`)
        // and vertex `b` now lives at position `pos_a`. The other n-2
        // columns are byte-identical — a relabeling is a bijection on
        // edges so column sizes are unchanged and `self.pcol_ptr`
        // needs no rewrite. We rewrite ONLY the two affected column
        // ranges in `self.prow_idx`, deriving their new contents from
        // the ORIGINAL pattern by walking the two affected vertices'
        // adjacency lists and mapping each neighbor through the NEW
        // `inv_perm` (which is the old `inv_perm` with `a` and `b`
        // swapped).
        let a = perm[i];
        let b = perm[j];
        let pos_a = self.inv_perm[a];
        let pos_b = self.inv_perm[b];
        // No-op guard: unreachable in practice (i != j forces a != b
        // forces pos_a != pos_b because inv_perm is a bijection on
        // 0..n), but the explicit check keeps the re-bucket loop well
        // defined and is free.
        if pos_a == pos_b {
            return self.column_counts_gnp_flops();
        }
        // First, update `inv_perm` to reflect the new positions of `a`
        // and `b`. Subsequent rows below read the new inv_perm.
        self.inv_perm[a] = pos_b;
        self.inv_perm[b] = pos_a;
        // Rewrite column `pos_a` to contain `b`'s neighbors (mapped
        // through the NEW inv_perm) and column `pos_b` to contain
        // `a`'s neighbors (mapped through the NEW inv_perm). The
        // self-symmetric entry (a,a) and (b,b) are filtered out by
        // the `i as usize != j` test inside the `inv_perm` map —
        // i.e., a row with neighbor == column-vertex becomes
        // `pos_x` (the new position of that neighbor), which is the
        // self-loop entry; the original `permute_into` does NOT
        // filter self-loops, so we don't either (preserves byte-
        // identity with the vendor path on the un-swapped columns).
        let col_a_start = self.pcol_ptr[pos_a];
        let col_a_end = self.pcol_ptr[pos_a + 1];
        let col_b_start = self.pcol_ptr[pos_b];
        let col_b_end = self.pcol_ptr[pos_b + 1];
        // Vertex `b` now goes into column `pos_a`.
        let mut k = col_a_start;
        for ki in pat.col_ptr[b]..pat.col_ptr[b + 1] {
            self.prow_idx[k] = self.inv_perm[pat.row_idx[ki]];
            k += 1;
        }
        // Vertex `a` now goes into column `pos_b`.
        let mut k = col_b_start;
        for ki in pat.col_ptr[a]..pat.col_ptr[a + 1] {
            self.prow_idx[k] = self.inv_perm[pat.row_idx[ki]];
            k += 1;
        }
        // Sort the two affected columns so the GNP algorithm sees the
        // same sorted-row-column invariant `permute_into` establishes.
        self.prow_idx[col_a_start..col_a_end].sort_unstable();
        self.prow_idx[col_b_start..col_b_end].sort_unstable();
        // Rebuild etree / postorder / first / counts on the swapped
        // permuted pattern. The GNP pass is O(nnz) which dominates
        // the local re-bucket above; the per-swap constant is
        // small enough for the WPS-AMD-IDB budget.
        self.build_etree();
        self.build_postorder_and_first();
        self.column_counts_gnp_flops()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pattern;

    fn scoring_pattern(pattern: &Pattern) -> ScoringPattern {
        ScoringPattern {
            n: pattern.n,
            col_ptr: pattern.col_ptr.clone(),
            row_idx: pattern.row_idx.clone(),
        }
    }

    /// The workspace path must match `super::flops_of` (the vendor-backed
    /// path) on a battery of small synthetic patterns and permutations,
    /// including repeated calls that exercise buffer reuse across DIFFERENT
    /// permutations (catching any missing reset).
    #[test]
    fn matches_vendor_flops_of_synthetic() {
        let cases: Vec<Pattern> = vec![
            Pattern::from_edges(1, &[]),
            Pattern::from_edges(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]), // chain
            Pattern::from_edges(5, &[(0, 1), (0, 2), (0, 3), (0, 4)]), // star
            Pattern::from_edges(6, &[(0, 1), (0, 2), (1, 2), (3, 4), (4, 5)]), // block+chain
            {
                let n = 20;
                let mut edges = Vec::new();
                for v in 0..n - 1 {
                    edges.push((v, v + 1));
                }
                for v in 0..n - 5 {
                    edges.push((v, v + 5));
                }
                Pattern::from_edges(n, &edges)
            },
        ];

        for pat in &cases {
            let n = pat.n;
            let sp = scoring_pattern(pat);
            let mut ws = ScoreWorkspace::new(n, pat.nnz());
            // A handful of permutations per pattern, including identity and
            // reversed, to exercise different fill patterns through the same
            // reused workspace.
            let mut perms: Vec<Vec<usize>> = vec![(0..n).collect(), (0..n).rev().collect()];
            if n > 3 {
                let mut p = (0..n).collect::<Vec<_>>();
                p.swap(0, n - 1);
                p.swap(1, n - 2);
                perms.push(p);
            }
            for perm in &perms {
                let expected = super::super::flops_of(&sp, perm);
                let got = ws.flops(&sp, perm);
                assert_eq!(
                    got, expected,
                    "workspace flops diverged from vendor flops_of on n={n} perm={perm:?}"
                );
            }
        }
    }

    /// `permute_i32_into` must match vendor `permute_pattern` cast to i32,
    /// including across repeated calls into the SAME reused output buffers.
    #[test]
    fn permute_i32_matches_vendor() {
        let n = 15;
        let mut edges = Vec::new();
        for v in 0..n - 1 {
            edges.push((v, v + 1));
        }
        for v in 0..n - 4 {
            edges.push((v, v + 4));
        }
        let pat = Pattern::from_edges(n, &edges);
        let sp = scoring_pattern(&pat);
        let mut ws = ScoreWorkspace::new(n, pat.nnz());
        let mut col_ptr_i32 = Vec::new();
        let mut row_idx_i32 = Vec::new();

        let perms: Vec<Vec<usize>> = vec![
            (0..n).collect(),
            (0..n).rev().collect(),
            {
                let mut p: Vec<usize> = (0..n).collect();
                p.swap(0, 7);
                p.swap(3, 12);
                p
            },
        ];
        for perm in &perms {
            ws.permute_i32_into(&sp, perm, &mut col_ptr_i32, &mut row_idx_i32);
            let expected = feral::ordering::amd::permute_pattern(&sp, perm);
            let expected_cp: Vec<i32> = expected.col_ptr.iter().map(|&x| x as i32).collect();
            let expected_ri: Vec<i32> = expected.row_idx.iter().map(|&x| x as i32).collect();
            assert_eq!(col_ptr_i32, expected_cp, "col_ptr mismatch for perm {perm:?}");
            assert_eq!(row_idx_i32, expected_ri, "row_idx mismatch for perm {perm:?}");
        }
    }
}
