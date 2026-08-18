//! Fill-reducing ordering: a best-of portfolio anchored on the grader's AMD.
//!
//! ## Contract
//!
//! `pub fn order(pattern: &Pattern) -> Vec<usize>` — `perm[k]` is the original
//! index eliminated k-th. Must be a bijection of `0..n`, **deterministic** (the
//! harness runs it twice per matrix and requires identical output), and finish
//! inside the 2 s/matrix cap, which is enforced by SIGKILL on a worker
//! subprocess. A cap violation or a disagreement fails the entire run, so both
//! are hard constraints here, not soft targets.
//!
//! ## Approach
//!
//! Three families, in order of how much they contribute:
//!
//! 1. **Anchor on AMD.** `ssi_scoring::amd_baseline` is the exact permutation
//!    the leaderboard normalizes against, and `ssi_scoring::score` is the exact
//!    flop proxy (`Σ_j c_j²`) it is scored with. Starting from the anchor and
//!    keeping a candidate only when it strictly lowers `flops` makes every
//!    additional family free upside: a candidate can win or be discarded, but
//!    never regress a matrix.
//!
//! 2. **Reverse Cuthill–McKee** as a structurally different family. RCM
//!    optimizes bandwidth rather than fill, so it is usually worse than AMD —
//!    but it wins on banded / grid-like patterns where a small profile already
//!    implies few fill-ins, and under a best-of those wins are kept for free.
//!
//! 3. **Relabelled-AMD multistart.** `feral_amd`'s tie-break reads the vertex
//!    numbering, so `AMD(Q A Qᵀ)` composed back through `Q` is a *different*
//!    minimum-degree ordering for the price of one AMD pass — a
//!    randomized-restart MD without implementing MD.
//!
//! The restart count may not depend on wall clock, which would break the
//! determinism gate, so it is a pure function of `n` in [`restarts`].

use crate::Pattern;
use ssi_scoring::{amd_baseline, score};

/// Fixed seed: the ordering must be reproducible across runs and machines.
const SEED: u64 = 0x5341_5349_4F52_4400;

/// Produce a fill-reducing elimination order for `pattern`.
pub fn order(pattern: &Pattern) -> Vec<usize> {
    let n = pattern.n;
    if n < 3 {
        return (0..n).collect();
    }

    // The anchor. Everything below is kept only if it strictly beats this.
    let mut best = amd_baseline(pattern);
    let mut best_flops = score(pattern, &best).flops;

    let cm = rcm(pattern);
    let cm_flops = score(pattern, &cm).flops;
    if cm_flops < best_flops {
        best = cm;
        best_flops = cm_flops;
    }

    for k in 0..restarts(n) {
        let q = random_perm(n, SEED ^ (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let relabelled = relabel(pattern, &q);
        // AMD on the relabelled pattern, composed back into original labels.
        let candidate: Vec<usize> = amd_baseline(&relabelled).into_iter().map(|i| q[i]).collect();
        let flops = score(pattern, &candidate).flops;
        if flops < best_flops {
            best = candidate;
            best_flops = flops;
        }
    }

    best
}

/// How many relabelled-AMD restarts to attempt. A pure function of `n`: the
/// determinism gate forbids deciding this from elapsed time.
fn restarts(n: usize) -> usize {
    match n {
        0..=200 => 48,
        201..=1_000 => 24,
        1_001..=10_000 => 6,
        10_001..=50_000 => 2,
        _ => 0,
    }
}

/// Reverse Cuthill–McKee. Components are seeded at the lowest-degree unvisited
/// vertex and neighbours are enqueued in increasing degree, with the vertex
/// index as a deterministic tie-break.
fn rcm(a: &Pattern) -> Vec<usize> {
    let n = a.n;
    let deg: Vec<usize> = (0..n).map(|j| a.col(j).len()).collect();
    let mut by_degree: Vec<usize> = (0..n).collect();
    by_degree.sort_unstable_by_key(|&v| (deg[v], v));

    let mut visited = vec![false; n];
    let mut out: Vec<usize> = Vec::with_capacity(n);
    let mut nbrs: Vec<usize> = Vec::new();

    for &seed in &by_degree {
        if visited[seed] {
            continue;
        }
        visited[seed] = true;
        out.push(seed);
        let mut cursor = out.len() - 1;
        while cursor < out.len() {
            let v = out[cursor];
            cursor += 1;
            nbrs.clear();
            nbrs.extend(a.col(v).iter().copied().filter(|&w| !visited[w]));
            nbrs.sort_unstable_by_key(|&w| (deg[w], w));
            for &w in &nbrs {
                visited[w] = true;
                out.push(w);
            }
        }
    }

    out.reverse();
    out
}

/// Build `B` with `B(i, j) = A(q[i], q[j])`. Preserves every `Pattern`
/// invariant: relabelling keeps the pattern symmetric and diagonal-free, and
/// each column is re-sorted here.
fn relabel(a: &Pattern, q: &[usize]) -> Pattern {
    let n = a.n;
    let mut inv = vec![0usize; n];
    for (new, &old) in q.iter().enumerate() {
        inv[old] = new;
    }

    let mut col_ptr = Vec::with_capacity(n + 1);
    let mut row_idx = Vec::with_capacity(a.nnz());
    let mut buf: Vec<usize> = Vec::new();
    col_ptr.push(0);
    for i in 0..n {
        buf.clear();
        buf.extend(a.col(q[i]).iter().map(|&r| inv[r]));
        buf.sort_unstable();
        row_idx.extend_from_slice(&buf);
        col_ptr.push(row_idx.len());
    }

    Pattern { n, col_ptr, row_idx }
}

/// Fisher–Yates driven by splitmix64 — a fixed-seed permutation, so no `rand`
/// dependency is needed and `src/ordering/deps.toml` stays empty.
fn random_perm(n: usize, seed: u64) -> Vec<usize> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };

    let mut q: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        q.swap(i, j);
    }
    q
}
