//! Safe-prefix reduction: repeatedly eliminate *simplicial* vertices (alive
//! neighbourhood is a clique). Eliminating a simplicial vertex creates no fill,
//! so the remaining graph is just the induced subgraph on the survivors — the
//! "core" — and any ordering of the core can be prefixed by the peeled vertices.
//! Because the core is smaller (often much smaller on sparse KKT patterns),
//! every candidate recipe run on it is cheaper, which buys more candidates
//! under the time cap.

use super::csc::Csc;

/// Only vertices with alive degree ≤ this are tested (high-degree simplicial
/// vertices are rare and the clique test is O(Σ deg) per attempt).
const MAX_TEST_DEG: usize = 48;

pub struct Reduction {
    /// Peeled vertices in elimination order (original indices).
    pub prefix: Vec<usize>,
    /// `true` for vertices that remain in the core.
    pub keep: Vec<bool>,
}

/// Peel simplicial vertices, lowest alive degree first (bucket queue), so
/// the prefix mimics a minimum-degree elimination of the simplicial part.
/// Deterministic.
pub fn peel(g: &Csc) -> Reduction {
    let n = g.n;
    let mut alive = vec![true; n];
    let mut adeg: Vec<u32> = g.degrees();
    let mut mark = vec![u32::MAX; n];
    let mut stamp: u32 = 0;
    // buckets[d] holds (possibly stale) candidates with alive degree d ≤ MAX_TEST_DEG
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); MAX_TEST_DEG + 1];
    let mut tested_at: Vec<u32> = vec![u32::MAX; n]; // degree at which v was last found non-simplicial
    for v in 0..n {
        let d = adeg[v] as usize;
        if d <= MAX_TEST_DEG {
            buckets[d].push(v as u32);
        }
    }
    let mut prefix: Vec<usize> = Vec::new();
    let mut nb: Vec<u32> = Vec::with_capacity(MAX_TEST_DEG + 1);
    let mut d = 0usize;
    while d <= MAX_TEST_DEG {
        let Some(v32) = buckets[d].pop() else {
            d += 1;
            continue;
        };
        let v = v32 as usize;
        if !alive[v] || adeg[v] as usize != d {
            continue; // stale entry
        }
        if tested_at[v] == adeg[v] {
            continue; // already known non-simplicial at this degree
        }
        let simplicial = if d <= 1 {
            true
        } else {
            stamp = stamp.wrapping_add(1);
            if stamp == u32::MAX {
                for m in mark.iter_mut() {
                    *m = u32::MAX;
                }
                stamp = 0;
            }
            nb.clear();
            for &r in g.col(v) {
                if alive[r as usize] {
                    mark[r as usize] = stamp;
                    nb.push(r as u32);
                }
            }
            let mut ok = true;
            for &u in &nb {
                let cu = g.col(u as usize);
                if cu.len() <= 8 * nb.len() {
                    let mut cnt = 0usize;
                    for &r in cu {
                        if mark[r as usize] == stamp {
                            cnt += 1;
                        }
                    }
                    if cnt + 1 < nb.len() {
                        ok = false;
                        break;
                    }
                } else {
                    // high-degree neighbour (hub): binary-search each other
                    // neighbour in its sorted column instead of scanning it
                    for &w in &nb {
                        if w != u && cu.binary_search(&(w as i32)).is_err() {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        break;
                    }
                }
            }
            ok
        };
        if !simplicial {
            tested_at[v] = adeg[v];
            continue;
        }
        alive[v] = false;
        prefix.push(v);
        for &r in g.col(v) {
            let r = r as usize;
            if alive[r] {
                adeg[r] -= 1;
                let dr = adeg[r] as usize;
                if dr <= MAX_TEST_DEG {
                    buckets[dr].push(r as u32);
                    if dr < d {
                        d = dr;
                    }
                }
            }
        }
    }
    Reduction { prefix, keep: alive }
}

/// (core_n, core_nnz, prefix_len) for diagnostics.
#[allow(dead_code)]
pub fn measure(g: &Csc) -> (usize, usize, usize) {
    let r = peel(g);
    let core_n = r.keep.iter().filter(|&&k| k).count();
    let mut core_nnz = 0usize;
    for v in 0..g.n {
        if r.keep[v] {
            core_nnz += g.col(v).iter().filter(|&&u| r.keep[u as usize]).count();
        }
    }
    (core_n, core_nnz, r.prefix.len())
}
