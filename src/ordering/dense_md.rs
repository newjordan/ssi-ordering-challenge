//! Exact-metric greedy elimination on a dense bitset graph, for small n.
//!
//! Minimum-degree / minimum-fill codes at scale must *approximate* their
//! metrics; at n ≤ ~1500 we can afford the exact elimination game: maintain the
//! filled graph as row bitsets, pick the alive vertex minimising an exact
//! metric (ties broken by a seeded PRNG), eliminate it (turn its neighbourhood
//! into a clique), repeat. Cost ≈ O(n·d³·W + n²) words — milliseconds at
//! n ≈ 1000, so it is a cheap, genuinely different candidate family there.

use super::csc::Csc;
use super::rng::SplitMix64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Metric {
    /// Exact degree (current number of alive neighbours).
    Degree,
    /// Exact deficiency: number of non-adjacent pairs in the neighbourhood
    /// (= fill edges created by eliminating the vertex).
    Fill,
    /// Mean local fill: deficiency / degree (scaled), Rothberg–Eisenstat AMMF.
    MeanFill,
    /// Increase in neighbour degree: 2·deficiency − degree (AMIND).
    Mind,
    /// Deficiency + degree (a smooth compromise).
    FillPlusDeg,
}

struct Bits {
    w: usize,
    data: Vec<u64>,
}

impl Bits {
    fn new(n: usize) -> Bits {
        let w = (n + 63) / 64;
        Bits { w, data: vec![0u64; n * w] }
    }
    #[inline]
    fn row(&self, v: usize) -> &[u64] {
        &self.data[v * self.w..(v + 1) * self.w]
    }
    #[inline]
    fn row_mut(&mut self, v: usize) -> &mut [u64] {
        &mut self.data[v * self.w..(v + 1) * self.w]
    }
    #[inline]
    fn set(&mut self, v: usize, u: usize) {
        self.data[v * self.w + (u >> 6)] |= 1u64 << (u & 63);
    }
    #[inline]
    fn get(&self, v: usize, u: usize) -> bool {
        (self.data[v * self.w + (u >> 6)] >> (u & 63)) & 1 == 1
    }
}

#[inline]
fn popcount_and(a: &[u64], b: &[u64]) -> usize {
    a.iter().zip(b).map(|(x, y)| (x & y).count_ones() as usize).sum()
}

fn iter_bits(row: &[u64], mut f: impl FnMut(usize)) {
    for (wi, &word) in row.iter().enumerate() {
        let mut x = word;
        while x != 0 {
            let t = x.trailing_zeros() as usize;
            f(wi * 64 + t);
            x &= x - 1;
        }
    }
}

/// Exact deficiency of `v`: pairs of alive neighbours that are not adjacent.
fn deficiency(adj: &Bits, alive: &[u64], nb_buf: &mut [u64], v: usize) -> (usize, usize) {
    // nb = N(v) ∩ alive
    let w = adj.w;
    let row = adj.row(v);
    for i in 0..w {
        nb_buf[i] = row[i] & alive[i];
    }
    let d: usize = nb_buf.iter().map(|x| x.count_ones() as usize).sum();
    if d < 2 {
        return (d, 0);
    }
    // edges among neighbours = ½ Σ_{u∈nb} |N(u) ∩ nb|
    let mut twice_edges = 0usize;
    iter_bits(nb_buf, |u| {
        twice_edges += popcount_and(adj.row(u), nb_buf);
    });
    let edges = twice_edges / 2;
    (d, d * (d - 1) / 2 - edges)
}

#[inline]
fn metric_value(m: Metric, d: usize, def: usize) -> u64 {
    match m {
        Metric::Degree => d as u64,
        Metric::Fill => def as u64,
        // scale so integer comparison works: def/d ≈ (def << 16) / max(d,1)
        Metric::MeanFill => ((def as u64) << 16) / (d.max(1) as u64),
        Metric::Mind => (2 * def as u64 + 1_000_000) - (d as u64).min(1_000_000),
        Metric::FillPlusDeg => (def + d) as u64,
    }
}

/// Greedy exact-metric elimination order of `g` (all vertices). `seed` drives
/// tie-breaking. Returns None if `n` is too large for the dense representation.
pub fn dense_order(g: &Csc, metric: Metric, seed: u64, max_n: usize) -> Option<Vec<usize>> {
    let n = g.n;
    if n == 0 {
        return Some(Vec::new());
    }
    if n > max_n {
        return None;
    }
    let mut adj = Bits::new(n);
    for j in 0..n {
        for &r in g.col(j) {
            adj.set(j, r as usize);
        }
    }
    let w = adj.w;
    let mut alive = vec![!0u64; w];
    if n % 64 != 0 {
        alive[w - 1] = (1u64 << (n % 64)) - 1;
    }
    let mut rng = SplitMix64::new(seed);
    let mut nb = vec![0u64; w];
    let mut nb2 = vec![0u64; w];
    // metric cache + dirty flags
    let mut val = vec![0u64; n];
    let mut degv = vec![0usize; n];
    let needs_def = metric != Metric::Degree;
    for v in 0..n {
        let (d, def) = if needs_def {
            deficiency(&adj, &alive, &mut nb, v)
        } else {
            (popcount_and(adj.row(v), &alive), 0)
        };
        degv[v] = d;
        val[v] = metric_value(metric, d, def);
    }
    let mut order = Vec::with_capacity(n);
    let mut alive_list: Vec<usize> = (0..n).collect();
    for _step in 0..n {
        // pick min metric among alive, random tie-break (reservoir).
        let mut best = u64::MAX;
        let mut pick = usize::MAX;
        let mut ties = 0u64;
        for &v in &alive_list {
            let x = val[v];
            if x < best {
                best = x;
                pick = v;
                ties = 1;
            } else if x == best {
                ties += 1;
                if rng.below(ties as usize) == 0 {
                    pick = v;
                }
            }
        }
        let v = pick;
        order.push(v);
        // neighbourhood of v
        {
            let row = adj.row(v);
            for i in 0..w {
                nb[i] = row[i] & alive[i];
            }
        }
        // remove v
        alive[v >> 6] &= !(1u64 << (v & 63));
        // make N(v) a clique: adj[a] |= nb for a in nb (a's own bit is harmless;
        // we always mask with alive and never read the diagonal for counts
        // because a ∉ N(a) is guaranteed by construction — clear it explicitly).
        let mut nbs: Vec<usize> = Vec::with_capacity(degv[v]);
        iter_bits(&nb, |a| nbs.push(a));
        for &a in &nbs {
            let row = adj.row_mut(a);
            for i in 0..w {
                row[i] |= nb[i];
            }
            row[a >> 6] &= !(1u64 << (a & 63));
        }
        // update metrics: all of N(v) changed; for Fill-type metrics, vertices
        // adjacent to two newly-joined neighbours also change (their pair count
        // grew). Recompute exactly for the union of N(v) and N(N(v)) ∩ alive —
        // cheap at this scale and always correct.
        if needs_def {
            // nb2 = union of N(a) for a in nb (alive), plus nb
            for i in 0..w {
                nb2[i] = nb[i];
            }
            for &a in &nbs {
                let row = adj.row(a);
                for i in 0..w {
                    nb2[i] |= row[i] & alive[i];
                }
            }
            let mut todo: Vec<usize> = Vec::new();
            iter_bits(&nb2, |u| todo.push(u));
            for u in todo {
                let (d, def) = deficiency(&adj, &alive, &mut nb, u);
                degv[u] = d;
                val[u] = metric_value(metric, d, def);
            }
        } else {
            for &a in &nbs {
                let d = popcount_and(adj.row(a), &alive);
                degv[a] = d;
                val[a] = metric_value(metric, d, 0);
            }
        }
        // drop v from alive_list (swap_remove keeps O(1); order of scan does not
        // matter because ties are broken by the PRNG reservoir).
        if let Some(pos) = alive_list.iter().position(|&x| x == v) {
            alive_list.swap_remove(pos);
        }
    }
    debug_assert!(!adj.get(0, 0));
    Some(order)
}
