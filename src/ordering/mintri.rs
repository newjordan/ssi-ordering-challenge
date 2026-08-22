//! Minimal-triangulation descent on the incumbent's filled graph.
//!
//! Σⱼcⱼ² of an elimination order depends only on the filled graph H (a chordal
//! supergraph of G): with T = #triangles of H, Σc² = 2T + 3|E(H)| + n, and any
//! perfect elimination order of H realises it. An edge of a chordal graph can be
//! deleted without breaking chordality iff the common neighbourhood of its
//! endpoints is a clique; deleting a fill edge therefore strictly lowers Σc².
//! This greedily deletes deletable fill edges (largest common neighbourhood
//! first) and returns a perfect elimination order (maximum cardinality search)
//! of the reduced graph. Bitset adjacency: intended for n ≤ ~6000.

use super::csc::Csc;

/// Returns a new elimination order if at least one fill edge was deleted.
/// `max_fill` bounds the number of fill edges considered (cost guard).
pub fn improve(g: &Csc, perm: &[usize], max_fill: usize, max_passes: usize) -> Option<Vec<usize>> {
    let n = g.n;
    if n < 8 || perm.len() != n {
        return None;
    }
    let words = (n + 63) / 64;
    let mut adj: Vec<u64> = vec![0u64; n * words];
    for v in 0..n {
        for &u in g.col(v) {
            let u = u as usize;
            if u == v {
                continue;
            }
            adj[v * words + (u >> 6)] |= 1u64 << (u & 63);
            adj[u * words + (v >> 6)] |= 1u64 << (v & 63);
        }
    }
    let orig = adj.clone();
    // Symbolic elimination in `perm` order: each pivot's later neighbours become a clique.
    let mut later: Vec<u64> = vec![!0u64; words];
    if n % 64 != 0 {
        later[words - 1] = (1u64 << (n % 64)) - 1;
    }
    let mut madj: Vec<u64> = vec![0u64; words];
    let mut list: Vec<usize> = Vec::new();
    let mut fill_count = 0usize;
    for &v in perm {
        later[v >> 6] &= !(1u64 << (v & 63));
        for w in 0..words {
            madj[w] = adj[v * words + w] & later[w];
        }
        list.clear();
        for w in 0..words {
            let mut x = madj[w];
            while x != 0 {
                list.push(w * 64 + x.trailing_zeros() as usize);
                x &= x - 1;
            }
        }
        for &u in &list {
            for w in 0..words {
                let add = madj[w] & !adj[u * words + w];
                fill_count += add.count_ones() as usize;
                adj[u * words + w] |= madj[w];
            }
            adj[u * words + (u >> 6)] &= !(1u64 << (u & 63));
        }
        if fill_count / 2 > max_fill {
            return None;
        }
    }
    // Fill edges (u < v).
    let mut fill: Vec<(u32, u32)> = Vec::new();
    for u in 0..n {
        for w in 0..words {
            let mut x = adj[u * words + w] & !orig[u * words + w];
            while x != 0 {
                let v = w * 64 + x.trailing_zeros() as usize;
                if v > u {
                    fill.push((u as u32, v as u32));
                }
                x &= x - 1;
            }
        }
    }
    if fill.is_empty() || fill.len() > max_fill {
        return None;
    }
    let mut removed = 0usize;
    let mut cbuf: Vec<u64> = vec![0u64; words];
    let mut clist: Vec<usize> = Vec::new();
    for _ in 0..max_passes.max(1) {
        let mut changed = false;
        let mut order: Vec<(u32, u32, u32)> = fill
            .iter()
            .map(|&(u, v)| {
                let (u, v) = (u as usize, v as usize);
                let mut c = 0u32;
                for w in 0..words {
                    c += (adj[u * words + w] & adj[v * words + w]).count_ones();
                }
                (c, u as u32, v as u32)
            })
            .collect();
        order.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut keep: Vec<(u32, u32)> = Vec::with_capacity(fill.len());
        for &(_, u, v) in &order {
            let (u, v) = (u as usize, v as usize);
            if adj[u * words + (v >> 6)] & (1u64 << (v & 63)) == 0 {
                continue;
            }
            for w in 0..words {
                cbuf[w] = adj[u * words + w] & adj[v * words + w];
            }
            clist.clear();
            for w in 0..words {
                let mut x = cbuf[w];
                while x != 0 {
                    clist.push(w * 64 + x.trailing_zeros() as usize);
                    x &= x - 1;
                }
            }
            let mut clique = true;
            'outer: for &x in &clist {
                for w in 0..words {
                    let mut need = cbuf[w] & !adj[x * words + w];
                    if w == (x >> 6) {
                        need &= !(1u64 << (x & 63));
                    }
                    if need != 0 {
                        clique = false;
                        break 'outer;
                    }
                }
            }
            if clique {
                adj[u * words + (v >> 6)] &= !(1u64 << (v & 63));
                adj[v * words + (u >> 6)] &= !(1u64 << (u & 63));
                removed += 1;
                changed = true;
            } else {
                keep.push((u as u32, v as u32));
            }
        }
        fill = keep;
        if !changed || fill.is_empty() {
            break;
        }
    }
    if removed == 0 {
        return None;
    }
    // Perfect elimination order of the reduced chordal graph: maximum
    // cardinality search numbers vertices n-1..0; eliminating in numbering
    // order is a PEO.
    let mut weight = vec![0u32; n];
    let mut numbered = vec![false; n];
    let mut peo = vec![0usize; n];
    for k in (0..n).rev() {
        let mut best = usize::MAX;
        let mut bw = 0u32;
        for v in 0..n {
            if !numbered[v] && (best == usize::MAX || weight[v] > bw) {
                best = v;
                bw = weight[v];
            }
        }
        numbered[best] = true;
        peo[k] = best;
        for w in 0..words {
            let mut x = adj[best * words + w];
            while x != 0 {
                let u = w * 64 + x.trailing_zeros() as usize;
                if !numbered[u] {
                    weight[u] += 1;
                }
                x &= x - 1;
            }
        }
    }
    Some(peo)
}
