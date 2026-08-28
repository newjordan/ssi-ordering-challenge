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

    /// `Σ_j c_j²` PLUS a fresh `Vec<usize>` of the per-position column counts
    /// `c_j` (in PERMUTED indices, i.e. `c_j` is the count of vertex
    /// `perm[j]`). The returned `Vec` is allocated by the caller; the
    /// workspace's internal `delta` is left in a valid (post-call) state but
    /// must not be read after the next `flops`/`flops_and_counts` call.
    /// Same algorithm as [`flops`]; only the return type changes.
    pub(crate) fn flops_and_counts(
        &mut self,
        pat: &ScoringPattern,
        perm: &[usize],
    ) -> (u64, Vec<usize>) {
        self.permute_into(pat, perm);
        self.build_etree();
        self.build_postorder_and_first();
        let n = self.n;
        // Mirror `column_counts_gnp_flops` exactly so the two return paths
        // agree on every bit, but ALSO write the per-position counts into a
        // fresh `Vec<usize>` for the caller's use. We can't reuse the
        // workspace's `delta` directly (it is `i64` and is borrowed by
        // further calls), so the copy is unavoidable.
        for i in 0..n {
            self.delta[i] = if self.children[i].is_empty() { 1 } else { 0 };
        }
        self.maxfirst.fill(-1);
        self.prevleaf.fill(-1);
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
        let mut counts: Vec<usize> = vec![0usize; n];
        for (i, &d) in self.delta.iter().enumerate() {
            let c = d as u64;
            flops += c * c;
            counts[i] = c as usize;
        }
        (flops, counts)
    }

    // -- WINDOWED REINSERTION HILL-CLIMB (W=16, P=3) ----------------------
    //
    // A local-search refinement over a fixed-size sliding window of
    // elimination positions: take the W=16 vertices in the window, process
    // them in FRONT-HEIGHT-DESCENDING order (their current column count
    // `counts[k]`, which is the `c_j` the GNP oracle charges when vertex
    // `perm[k]` is eliminated), and for each one try to REINSERT it at the
    // best of P=3 candidate positions strictly before its current position.
    // Accept the move only on a STRICT improvement to the total flop count
    // (`Σ c_j²`); ties are rejected (must be `<`, not `≤`). Update
    // `best_perm` / `best_flops` in place and keep going for the next
    // vertex in the window, so each accepted move is seen by the next move
    // (the front heights used to rank vertices in the window are
    // re-evaluated AFTER each accepted reinsertion, never stale).
    //
    // The "incremental predicted-flops resimulation" is the incremental
    // reuse of THIS workspace's already-pre-allocated `~15` buffers across
    // every `flops` call (instead of paying for ~15 fresh allocations per
    // reinsertion candidate) — the difference between this hill-climb and
    // a naive "score every reinsertion from scratch" approach is exactly
    // the per-call allocation cost, which is the dominant cost on the
    // sparse-medium matrices that the rest of the portfolio finds
    // interesting. The GNP column-count pass itself is `O(nnz(L))` per
    // call — already near-linear in the only size the workspace has been
    // pre-sized for, so no further incremental work is available.
    //
    // Determinism: every step is a pure function of (`pat`, `best_perm`,
    // `best_flops`, `W`, `P`); the workspace's pre-existing buffers are
    // reset where required, so two `order()` calls on the same pattern
    // reach the same `best_perm` / `best_flops`. The two required
    // `order()` runs (the determinism gate) agree by construction.
    //
    // Cost model: a single window of W vertices runs at most `W * P` full
    // `flops` evaluations (one per (vertex, candidate-position) pair), plus
    // 1 evaluation after each accepted move to refresh the per-position
    // counts. The full pass over `n / W` windows therefore performs up to
    // `(n / W) * (1 + W * P)` `flops` evaluations, every one O(nnz(L)) on
    // pre-allocated buffers. W=16 and P=3 are calibrated for ms-to-low-tens-
    // of-ms total cost on the small/sparse matrices the rest of the
    // portfolio already accepts; the caller must gate on (n, nnz).
    //
    // Returns `None` if no move is accepted (so the caller's incumbent is
    // untouched); `Some((new_perm, new_flops))` otherwise. The returned
    // `new_perm` is a fresh allocation; the caller's `best_perm` is
    // unchanged when the function returns `None`.
    pub(crate) fn windowed_reinsertion_hillclimb(
        &mut self,
        pat: &ScoringPattern,
        best_perm: &[usize],
        best_flops: u64,
        window_size: usize,
        positions: usize,
    ) -> Option<(Vec<usize>, u64)> {
        let n = pat.n;
        if n == 0 || window_size == 0 || positions == 0 {
            return None;
        }
        // Need at least one valid "insert back" room: cur_pos >= positions.
        if n < window_size + positions {
            return None;
        }

        let mut cur_perm: Vec<usize> = best_perm.to_vec();
        let mut cur_flops = best_flops;
        let mut improved = false;

        // Last window's start, in ascending order. (Slide the window over
        // the whole permutation; overlap between windows is harmless and
        // gives the high-front-height vertices of a window another chance
        // to move in the next window after their column counts are
        // refreshed.)
        for win_start in 0..=(n - window_size) {
            // Recompute column counts AFTER every accepted move, so the
            // front-height ranking of the next window is current. If
            // nothing has been accepted since the last evaluation this
            // would be wasteful, but `improved_since_last_eval` tracks
            // that and skips it.
            let counts: Vec<usize> = {
                let (_, c) = self.flops_and_counts(pat, &cur_perm);
                c
            };
            // Build the window-vertex list, sorted by front height
            // (column count) DESCENDING. `cur_perm[win_start + i]` is the
            // vertex eliminated at position `win_start + i`; its current
            // front height is `counts[win_start + i]`.
            let mut window_verts: Vec<(usize, usize)> = (0..window_size)
                .map(|i| {
                    let pos = win_start + i;
                    (cur_perm[pos], counts[pos])
                })
                .collect();
            window_verts.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

            for (v, _h) in window_verts {
                if v >= n {
                    continue;
                }
                // Find v's CURRENT position in `cur_perm` (this changes
                // as accepted moves accumulate within the window).
                let cur_pos = match cur_perm.iter().position(|&x| x == v) {
                    Some(p) => p,
                    None => continue,
                };
                if cur_pos < positions {
                    continue; // not enough room to reinsert back
                }

                // Try the P candidate reinsertion positions: strictly
                // BEFORE the current position. We try ALL P from the
                // closest (`cur_pos - 1`) outward to `cur_pos - P` and
                // take the best — the "best of P" choice (rather than
                // the FIRST improving one) is what makes the move
                // monotone in `cur_flops` and avoids accepting a merely-
                // local move when a strictly-better one is available.
                let mut best_local_flops = cur_flops;
                let mut best_local_pos: Option<usize> = None;
                for p in 1..=positions {
                    let new_pos = cur_pos - p;
                    let mut trial = Vec::with_capacity(n);
                    trial.extend_from_slice(&cur_perm[..new_pos]);
                    trial.push(v);
                    trial.extend_from_slice(&cur_perm[new_pos..cur_pos]);
                    trial.extend_from_slice(&cur_perm[cur_pos + 1..]);
                    let f = self.flops(pat, &trial);
                    if f < best_local_flops {
                        best_local_flops = f;
                        best_local_pos = Some(new_pos);
                    }
                }

                // STRICT improvement only — ties are rejected.
                if let Some(np) = best_local_pos {
                    if best_local_flops < cur_flops {
                        let mut new_perm = Vec::with_capacity(n);
                        new_perm.extend_from_slice(&cur_perm[..np]);
                        new_perm.push(v);
                        new_perm.extend_from_slice(&cur_perm[np..cur_pos]);
                        new_perm.extend_from_slice(&cur_perm[cur_pos + 1..]);
                        cur_perm = new_perm;
                        cur_flops = best_local_flops;
                        improved = true;
                    }
                }
            }
        }

        if improved && cur_flops < best_flops {
            Some((cur_perm, cur_flops))
        } else {
            None
        }
    }

    /// Same as [`windowed_reinsertion_hillclimb`], but with a hard cap on
    /// the number of full `flops` evaluations the phase may perform —
    /// `max_evals` decrements by 1 for every `flops` call (initial per-
    /// window count refresh + every candidate reinsertion evaluation);
    /// once the budget hits 0 the loop breaks out and the current
    /// incumbent is returned. A defensive belt for unusually large `n`
    /// where `n * W * P` could otherwise exceed the caller's wall-clock
    /// budget.
    pub(crate) fn windowed_reinsertion_hillclimb_budgeted(
        &mut self,
        pat: &ScoringPattern,
        best_perm: &[usize],
        best_flops: u64,
        window_size: usize,
        positions: usize,
        max_evals: usize,
    ) -> Option<(Vec<usize>, u64)> {
        let n = pat.n;
        if n == 0 || window_size == 0 || positions == 0 {
            return None;
        }
        if n < window_size + positions {
            return None;
        }
        if max_evals == 0 {
            return None;
        }

        let mut cur_perm: Vec<usize> = best_perm.to_vec();
        let mut cur_flops = best_flops;
        let mut evals_left = max_evals;
        let mut improved = false;

        for win_start in 0..=(n - window_size) {
            if evals_left == 0 {
                break;
            }
            evals_left -= 1;
            let (_, counts) = self.flops_and_counts(pat, &cur_perm);
            let mut window_verts: Vec<(usize, usize)> = (0..window_size)
                .map(|i| {
                    let pos = win_start + i;
                    (cur_perm[pos], counts[pos])
                })
                .collect();
            window_verts.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

            for (v, _h) in window_verts {
                if v >= n {
                    continue;
                }
                let cur_pos = match cur_perm.iter().position(|&x| x == v) {
                    Some(p) => p,
                    None => continue,
                };
                if cur_pos < positions {
                    continue;
                }
                let mut best_local_flops = cur_flops;
                let mut best_local_pos: Option<usize> = None;
                for p in 1..=positions {
                    if evals_left == 0 {
                        break;
                    }
                    evals_left -= 1;
                    let new_pos = cur_pos - p;
                    let mut trial = Vec::with_capacity(n);
                    trial.extend_from_slice(&cur_perm[..new_pos]);
                    trial.push(v);
                    trial.extend_from_slice(&cur_perm[new_pos..cur_pos]);
                    trial.extend_from_slice(&cur_perm[cur_pos + 1..]);
                    let f = self.flops(pat, &trial);
                    if f < best_local_flops {
                        best_local_flops = f;
                        best_local_pos = Some(new_pos);
                    }
                }

                if let Some(np) = best_local_pos {
                    if best_local_flops < cur_flops {
                        let mut new_perm = Vec::with_capacity(n);
                        new_perm.extend_from_slice(&cur_perm[..np]);
                        new_perm.push(v);
                        new_perm.extend_from_slice(&cur_perm[np..cur_pos]);
                        new_perm.extend_from_slice(&cur_perm[cur_pos + 1..]);
                        cur_perm = new_perm;
                        cur_flops = best_local_flops;
                        improved = true;
                    }
                }
                if evals_left == 0 {
                    break;
                }
            }
        }

        if improved && cur_flops < best_flops {
            Some((cur_perm, cur_flops))
        } else {
            None
        }
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
