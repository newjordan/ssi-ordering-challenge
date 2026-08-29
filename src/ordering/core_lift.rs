//! CORE-LIFT ??a fixed exact-elimination prefix, then the portfolio on the
//! residual core.
//!
//! Peel degree-?? vertices (pendants) and eliminate every vertex whose live
//! degree is ??`max_row_deg`, in ascending `(degree, index)` order, adding the
//! clique on each eliminated vertex's live neighbours. That is the EXACT
//! elimination step, so the residual graph is the exact fill graph after the
//! prefix, and
//!
//! ```text
//!   誇_j c_j짼  =  誇_{v ??prefix} c_v짼  +  誇_{w ??core} c_w짼
//! ```
//!
//! splits with the first term FIXED (the prefix is one fixed sequence) and the
//! second term computed on the core graph alone. So a candidate ordering of the
//! core can be ranked on the core graph ??~100횞 cheaper than a full-graph
//! scoring pass ??and only the winner needs the trusted global scorer.
//!
//! DETERMINISM. The elimination order is a strict total order on
//! `(degree, index)` (indices are unique, so there are no ties); the edge set
//! is a hash SET that is only ever membership-tested, never iterated, so no
//! hash order reaches the output; the core is numbered by ascending original
//! index. Same input ??same `CoreLift`, byte for byte.

use feral::sparse::csc::CscPattern as ScoringPattern;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};

/// Deterministic multiplicative hasher for the `u64` edge keys.
///
/// The default `RandomState` is deterministic *within* a process too, but it
/// is also ~2횞 the cost of this on the ~2M membership tests the reduction runs
/// on a `gt_10k` giant, and that cost lands directly on the hard time
/// constraint. Fixed constants, no randomness, no crate.
#[derive(Default)]
pub(crate) struct EdgeHasher(u64);

impl Hasher for EdgeHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x1000_0000_01B3);
        }
    }
    #[inline]
    fn write_u64(&mut self, v: u64) {
        let mut x = v ^ 0x9E37_79B9_7F4A_7C15;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        self.0 = x ^ (x >> 31);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

type EdgeSet = HashSet<u64, BuildHasherDefault<EdgeHasher>>;

/// A reduction of a pattern into a fixed elimination prefix plus a residual
/// core, with the core's fill graph as a standalone symmetric CSC.
pub(crate) struct CoreLift {
    /// Eliminated vertices, in the order they were eliminated. Original ids.
    pub(crate) prefix: Vec<usize>,
    /// Surviving vertices, ascending original id. `core_ids[k]` is the
    /// original id of core vertex `k`.
    pub(crate) core_ids: Vec<usize>,
    /// The core's fill graph, symmetric, diagonal omitted (same convention as
    /// `Pattern`/`ScoringPattern`), indexed `0..core_ids.len()`.
    pub(crate) core_col_ptr: Vec<usize>,
    pub(crate) core_row_idx: Vec<usize>,
    /// 誇 c_v짼 over the prefix ??the fixed part of the objective.
    pub(crate) prefix_flops: u64,
}

impl CoreLift {
    #[inline]
    pub(crate) fn core_n(&self) -> usize {
        self.core_ids.len()
    }
    #[inline]
    pub(crate) fn core_nnz(&self) -> usize {
        self.core_row_idx.len()
    }
}

#[inline]
fn key(a: u32, b: u32) -> u64 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    ((lo as u64) << 32) | (hi as u64)
}

/// Reduce `sp` to (prefix, core). Returns `None` when the pattern is empty, is
/// too large for the `u32` vertex ids used internally, or when the residual
/// core misses the size gates ??every `None` path leaves the caller's
/// incumbent untouched.
pub(crate) fn reduce(
    sp: &ScoringPattern,
    max_row_deg: usize,
    max_core_n: usize,
    max_core_edges: usize,
) -> Option<CoreLift> {
    let n = sp.n;
    if n == 0 || n > u32::MAX as usize {
        return None;
    }

    let mut nbrs: Vec<Vec<u32>> = Vec::with_capacity(n);
    let mut deg: Vec<u32> = vec![0; n];
    let mut edges: EdgeSet =
        EdgeSet::with_capacity_and_hasher(sp.row_idx.len() / 2 + 16, Default::default());
    for j in 0..n {
        let s = sp.col_ptr[j];
        let e = sp.col_ptr[j + 1];
        let list: Vec<u32> = sp.row_idx[s..e].iter().map(|&x| x as u32).collect();
        deg[j] = list.len() as u32;
        for &w in &list {
            if (w as usize) > j {
                edges.insert(key(j as u32, w));
            }
        }
        nbrs.push(list);
    }

    let mut alive = vec![true; n];
    let mut prefix: Vec<usize> = Vec::new();
    let mut prefix_flops: u64 = 0;

    // Ascending (degree, index). Pendants (degree ??1) therefore always come
    // out before any degree-2/3 vertex, at every point in the run ??the
    // "peel pendants first, to exhaustion" phase is the head of this order,
    // and it re-runs automatically whenever an elimination creates a new one.
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
    for v in 0..n {
        if deg[v] as usize <= max_row_deg {
            heap.push(Reverse((deg[v], v as u32)));
        }
    }

    let mut live: Vec<u32> = Vec::with_capacity(max_row_deg + 1);
    while let Some(Reverse((d, v))) = heap.pop() {
        let vu = v as usize;
        // Stale entry (the vertex was eliminated, or its degree moved since
        // this entry was pushed).
        if !alive[vu] || deg[vu] != d || d as usize > max_row_deg {
            continue;
        }
        live.clear();
        for &w in &nbrs[vu] {
            let wu = w as usize;
            if wu != vu && alive[wu] && edges.contains(&key(v, w)) {
                live.push(w);
            }
        }
        live.sort_unstable();
        live.dedup();
        debug_assert_eq!(live.len(), deg[vu] as usize);
        // c_v = its live degree at elimination time (Liu): the column count of
        // v in the factor is exactly |live| + 1 counting the diagonal. The
        // grader's 誇 c_j짼 counts the diagonal, matching `column_counts_gnp`.
        let cv = live.len() as u64 + 1;
        prefix_flops = prefix_flops.saturating_add(cv * cv);
        // Close the neighbourhood into a clique ??the exact elimination step.
        for i in 0..live.len() {
            for j in (i + 1)..live.len() {
                let (a, b) = (live[i], live[j]);
                if edges.insert(key(a, b)) {
                    nbrs[a as usize].push(b);
                    nbrs[b as usize].push(a);
                    deg[a as usize] += 1;
                    deg[b as usize] += 1;
                }
            }
        }
        for &w in live.iter() {
            edges.remove(&key(v, w));
            deg[w as usize] -= 1;
        }
        alive[vu] = false;
        deg[vu] = 0;
        nbrs[vu] = Vec::new();
        prefix.push(vu);
        for &w in live.iter() {
            if deg[w as usize] as usize <= max_row_deg {
                heap.push(Reverse((deg[w as usize], w)));
            }
        }
    }

    let core_ids: Vec<usize> = (0..n).filter(|&v| alive[v]).collect();
    let core_n = core_ids.len();
    if core_n == 0 || core_n > max_core_n {
        return None;
    }
    let mut pos_of: Vec<u32> = vec![u32::MAX; n];
    for (k, &v) in core_ids.iter().enumerate() {
        pos_of[v] = k as u32;
    }
    let mut core_col_ptr: Vec<usize> = Vec::with_capacity(core_n + 1);
    let mut core_row_idx: Vec<usize> = Vec::new();
    core_col_ptr.push(0);
    let mut row: Vec<u32> = Vec::new();
    for &v in core_ids.iter() {
        row.clear();
        for &w in &nbrs[v] {
            let wu = w as usize;
            if wu != v && alive[wu] && edges.contains(&key(v as u32, w)) {
                row.push(pos_of[wu]);
            }
        }
        row.sort_unstable();
        row.dedup();
        core_row_idx.extend(row.iter().map(|&x| x as usize));
        core_col_ptr.push(core_row_idx.len());
        if core_row_idx.len() > 2 * max_core_edges {
            return None;
        }
    }

    Some(CoreLift {
        prefix,
        core_ids,
        core_col_ptr,
        core_row_idx,
        prefix_flops,
    })
}

/// prefix ++ core, mapped back to original vertex ids. `core_perm[k]` is the
/// core vertex eliminated `k`-th.
pub(crate) fn splice(cl: &CoreLift, core_perm: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::with_capacity(cl.prefix.len() + core_perm.len());
    out.extend_from_slice(&cl.prefix);
    for &k in core_perm {
        out.push(cl.core_ids[k]);
    }
    out
}
