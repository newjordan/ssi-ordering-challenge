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

/// Shared body of [`ScoreWorkspace::permute_into`] and
/// [`PermuteWorkspace`]'s permute: a faithful port of vendor
/// `feral::ordering::amd::permute_pattern`, parameterized over the buffers so
/// both workspaces reuse the identical logic (single-sourced so the two paths
/// cannot drift apart). Buffer invariants (argued per-field in
/// `permute_into`'s original comments, unchanged here):
///   - `pcol_ptr[0]` is 0 on entry and is never written (the prefix sum only
///     reads it), so it stays 0 across calls;
///   - `pcol_ptr[1..=n]`, `inv_perm`, `offsets`, `prow_idx` are each fully
///     overwritten on every call (perm is a bijection; the bucket-fill writes
///     exactly the `nnz` distinct positions), so no reset is needed.
fn permute_into_bufs(
    n: usize,
    perm: &[usize],
    pat: &ScoringPattern,
    inv_perm: &mut [usize],
    pcol_ptr: &mut [usize],
    prow_idx: &mut [usize],
    offsets: &mut [usize],
) {
    // `inv_perm[old] = new`.
    for (new, &old) in perm.iter().enumerate() {
        inv_perm[old] = new;
    }

    for old_j in 0..n {
        let new_j = inv_perm[old_j];
        let nnz_j = pat.col_ptr[old_j + 1] - pat.col_ptr[old_j];
        pcol_ptr[new_j + 1] = nnz_j;
    }
    for j in 0..n {
        pcol_ptr[j + 1] += pcol_ptr[j];
    }

    offsets[..n].copy_from_slice(&pcol_ptr[..n]);

    for old_j in 0..n {
        let new_j = inv_perm[old_j];
        let start = pat.col_ptr[old_j];
        let end = pat.col_ptr[old_j + 1];
        for k in start..end {
            let new_i = inv_perm[pat.row_idx[k]];
            let pos = offsets[new_j];
            prow_idx[pos] = new_i;
            offsets[new_j] += 1;
        }
    }

    // Sort each column's row indices, exactly as the vendor function does.
    for j in 0..n {
        let start = pcol_ptr[j];
        let end = pcol_ptr[j + 1];
        prow_idx[start..end].sort_unstable();
    }
}

/// Reusable workspace for the relabel/perturb → permute → i32-cast path the
/// relabel multi-starts, hill-climb phase and count-relabel phase run once
/// PER RESTART. As written against the vendor functions, that path allocated
/// a fresh `q` (`relabel`/`perturb`), a fresh 4-`Vec` permuted pattern
/// (`permute_pattern`) and two fresh `Vec<i32>` casts every restart — ~7 heap
/// allocations, two of them O(nnz), times up to ~48 restarts on the matrices
/// that receive every phase. This workspace allocates every buffer exactly
/// ONCE per `order()` call (n and nnz are fixed for the whole call) and
/// reuses them across every restart. It is separate from [`ScoreWorkspace`]
/// (rather than more fields on it) because the relabel loops run INSIDE
/// `consider`, which already holds `ws` mutably borrowed — the generation
/// path needs its own buffers.
///
/// The permute step is literally [`permute_into_bufs`] (the same function the
/// proven-byte-identical scoring path uses), and the i32 cast is the same
/// element-wise map the relabel loops did by hand — so the relabelled pattern
/// fed to `feral_amd::amd_order` / `feral_amf::amf_order_opts` is
/// bit-identical to the vendor-built one (covered by
/// `permute_ws_matches_vendor` below).
pub(crate) struct PermuteWorkspace {
    n: usize,
    inv_perm: Vec<usize>,
    pcol_ptr: Vec<usize>,
    prow_idx: Vec<usize>,
    offsets: Vec<usize>,
    q: Vec<usize>,
    col_ptr_i32: Vec<i32>,
    row_idx_i32: Vec<i32>,
}

impl PermuteWorkspace {
    pub(crate) fn new(n: usize, nnz: usize) -> Self {
        PermuteWorkspace {
            n,
            inv_perm: vec![0; n],
            pcol_ptr: vec![0; n + 1],
            prow_idx: vec![0; nnz],
            offsets: vec![0; n],
            q: Vec::with_capacity(n),
            col_ptr_i32: Vec::with_capacity(n + 1),
            row_idx_i32: Vec::with_capacity(nnz),
        }
    }

    /// Scratch buffer for the caller's relabel/perturb generation (filled by
    /// `relabel_into`/`perturb_into` in `mod.rs`, then consumed by
    /// [`permute_q_i32`](Self::permute_q_i32)).
    pub(crate) fn q_scratch(&mut self) -> &mut Vec<usize> {
        &mut self.q
    }

    /// Permute `pat` by the permutation currently held in `self.q`, returning
    /// the i32-cast permuted pattern plus a read-only view of `q` itself (for
    /// the compose-back step `pb[k] -> q[pb[k]]`). One `&mut self` borrow in,
    /// three shared reborrows out — no per-call allocation.
    pub(crate) fn permute_q_i32(&mut self, pat: &ScoringPattern) -> (&[i32], &[i32], &[usize]) {
        permute_into_bufs(
            self.n,
            &self.q,
            pat,
            &mut self.inv_perm,
            &mut self.pcol_ptr,
            &mut self.prow_idx,
            &mut self.offsets,
        );
        self.col_ptr_i32.clear();
        self.col_ptr_i32.extend(self.pcol_ptr.iter().map(|&x| x as i32));
        self.row_idx_i32.clear();
        self.row_idx_i32.extend(self.prow_idx.iter().map(|&x| x as i32));
        (&self.col_ptr_i32, &self.row_idx_i32, &self.q)
    }

    /// Same as [`permute_q_i32`](Self::permute_q_i32) but permuting by an
    /// explicit `perm` (for phases that build their relabeling in their own
    /// buffer, e.g. count-relabel's sorted `q`).
    #[allow(dead_code)]
    pub(crate) fn permute_i32(&mut self, pat: &ScoringPattern, perm: &[usize]) -> (&[i32], &[i32]) {
        permute_into_bufs(
            self.n,
            perm,
            pat,
            &mut self.inv_perm,
            &mut self.pcol_ptr,
            &mut self.prow_idx,
            &mut self.offsets,
        );
        self.col_ptr_i32.clear();
        self.col_ptr_i32.extend(self.pcol_ptr.iter().map(|&x| x as i32));
        self.row_idx_i32.clear();
        self.row_idx_i32.extend(self.prow_idx.iter().map(|&x| x as i32));
        (&self.col_ptr_i32, &self.row_idx_i32)
    }
}

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
        // Single-sourced with `PermuteWorkspace`'s permute — see
        // `permute_into_bufs` for the per-field overwrite/reset invariants
        // (they are the same invariants the original inline body documented
        // here; the logic is unchanged, only its location moved).
        permute_into_bufs(
            self.n,
            perm,
            pat,
            &mut self.inv_perm,
            &mut self.pcol_ptr,
            &mut self.prow_idx,
            &mut self.offsets,
        );
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

    /// `PermuteWorkspace` must produce the same permuted pattern as vendor
    /// `permute_pattern` (cast to i32), across repeated calls into the SAME
    /// reused buffers (catching any missing reset), for both the `q`-driven
    /// and explicit-perm entry points.
    #[test]
    fn permute_ws_matches_vendor() {
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
        let mut pws = PermuteWorkspace::new(n, pat.nnz());

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
            // Explicit-perm entry point.
            let (cp, ri) = pws.permute_i32(&sp, perm);
            let expected = feral::ordering::amd::permute_pattern(&sp, perm);
            let expected_cp: Vec<i32> = expected.col_ptr.iter().map(|&x| x as i32).collect();
            let expected_ri: Vec<i32> = expected.row_idx.iter().map(|&x| x as i32).collect();
            assert_eq!(cp, expected_cp.as_slice(), "col_ptr mismatch for perm {perm:?}");
            assert_eq!(ri, expected_ri.as_slice(), "row_idx mismatch for perm {perm:?}");

            // q-driven entry point: load q with the same perm, expect the
            // same permuted pattern AND the same q back for compose-back.
            pws.q_scratch().clear();
            pws.q_scratch().extend_from_slice(perm);
            let (cp2, ri2, q2) = pws.permute_q_i32(&sp);
            assert_eq!(cp2, expected_cp.as_slice(), "q col_ptr mismatch for perm {perm:?}");
            assert_eq!(ri2, expected_ri.as_slice(), "q row_idx mismatch for perm {perm:?}");
            assert_eq!(q2, perm.as_slice(), "q view mismatch for perm {perm:?}");
        }
    }
}
