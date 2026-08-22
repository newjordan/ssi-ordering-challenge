//! Candidate orderings: a `Cand` is a pure, deterministic recipe
//! (base algorithm × relabelling × hub-strip) that turns a pattern into a
//! permutation. The plan (mod.rs) decides WHICH recipes run; this module
//! decides HOW each one runs and what it costs.

use super::csc::{compose_relabel, invert_u32, is_bijection, rcm_labels, Csc};
use super::dense_md::{dense_order, Metric};
use super::rng::SplitMix64;

/// Largest n handled by the exact-metric dense engine.
pub const DENSE_MAX_N: usize = 1_500;

/// Base ordering algorithm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Algo {
    /// feral AMD with (aggressive absorption, dense_alpha).
    Amd { aggressive: bool, alpha: f64 },
    /// feral AMF (approximate minimum fill) with dense_alpha.
    Amf { alpha: f64 },
    /// feral METIS-class multilevel ND.
    Metis { seed: u64, switch: u32, niparts: u32 },
    /// feral Scotch-class ND.
    Scotch { seed: u64 },
    /// feral KaHIP-class ND.
    Kahip { seed: u64, mode: u8 },
    /// Exact-metric greedy elimination on a dense bitset graph (small n only).
    Dense { metric: Metric, seed: u64 },
}

/// How the graph is relabelled before the base algorithm runs. Minimum-degree
/// codes break ties by vertex index, so a relabelling is a different ordering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Relabel {
    None,
    Reverse,
    Random(u64),
    /// Degree-ascending (ties by index), or descending.
    Degree { ascending: bool },
    /// (Reverse) Cuthill–McKee BFS labelling.
    Rcm { reverse: bool },
}

/// Hub-strip: remove a hub set, order the remainder, then append the hubs.
/// `TopK(k)`: the k highest-degree vertices (ties by index), appended in
/// ascending degree. `Threshold(t)`: every vertex with degree > t, appended in
/// AMD order of the hubs' induced subgraph (degree order if that fails).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Strip {
    None,
    TopK(u32),
    Threshold(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct Cand {
    pub algo: Algo,
    pub relabel: Relabel,
    pub strip: Strip,
    /// Run on the reduced core (prefix ++ core order) instead of the full graph.
    pub on_core: bool,
    /// Estimated single-thread cost in µs on the reference machine (used only
    /// for planning; never measured at run time).
    pub est_us: f64,
}

/// Cost model (µs per nnz, reference machine, conservative ≈ observed max).
pub mod cost {
    pub const AMD: f64 = 0.075;
    pub const AMF: f64 = 0.11;
    pub const METIS: f64 = 0.65;
    pub const SCOTCH: f64 = 0.85;
    pub const KAHIP: f64 = 1.6;
    /// Trusted symbolic scoring pass (permute + etree + column counts).
    pub const SYM: f64 = 0.035;
    /// Relabel / induced-subgraph construction.
    pub const RELABEL: f64 = 0.03;
    /// Fixed overhead per candidate (allocation, thread hand-off).
    pub const BASE: f64 = 40.0;
}

/// Provisional cost model for the dense engine (µs): degree-only is ~n²-ish
/// scans; fill metrics add d³·W work per elimination. Calibrated on the dev
/// corpus; conservative.
pub fn dense_cost_us(nnz: usize, metric: Metric) -> f64 {
    // Per elimination the fill metrics recompute deficiencies over N(N(v)),
    // O(d²·W) words each ⇒ total ≈ n·d²·W = nnz²/64 word-ops; measured ≈ 1e-4
    // µs per nnz² on the reference machine. Degree-only is ~15× cheaper.
    let x = nnz as f64;
    match metric {
        Metric::Degree => 150.0 + 0.7e-5 * x * x + 0.05 * x,
        _ => 300.0 + 1.0e-4 * x * x + 0.1 * x,
    }
}

impl Cand {
    pub fn new(algo: Algo, relabel: Relabel, strip: Strip, nnz: usize) -> Cand {
        let per = match algo {
            Algo::Amd { .. } => cost::AMD,
            Algo::Amf { .. } => cost::AMF,
            Algo::Metis { .. } => cost::METIS,
            Algo::Scotch { .. } => cost::SCOTCH,
            Algo::Kahip { .. } => cost::KAHIP,
            Algo::Dense { .. } => 0.0,
        };
        let mut est = cost::BASE + (per + cost::SYM) * nnz as f64;
        if let Algo::Dense { metric, .. } = algo {
            // Calibrated separately (see `dense_cost_us`); nnz stands in for n·d.
            est += dense_cost_us(nnz, metric);
        }
        if relabel != Relabel::None {
            est += cost::RELABEL * nnz as f64;
        }
        if strip != Strip::None {
            est += cost::RELABEL * nnz as f64;
        }
        Cand { algo, relabel, strip, on_core: false, est_us: est }
    }

    pub fn core(mut self) -> Cand {
        self.on_core = true;
        self
    }

    /// Run the base algorithm on `g`; returns an elimination order of `g`'s
    /// vertices (i32), or None on any failure.
    fn run_algo(&self, g: &Csc) -> Option<Vec<i32>> {
        if let Algo::Dense { metric, seed } = self.algo {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                dense_order(g, metric, seed, DENSE_MAX_N)
            }));
            return match r {
                Ok(Some(p)) if p.len() == g.n => Some(p.into_iter().map(|x| x as i32).collect()),
                _ => None,
            };
        }
        let view = g.view();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match self.algo {
            Algo::Amd { aggressive, alpha } => feral_amd::amd_order_opts(
                &view,
                &feral_amd::AmdOptions { aggressive, dense_alpha: alpha },
            )
            .map(|(p, _)| p),
            Algo::Amf { alpha } => {
                feral_amf::amf_order_opts(&view, &feral_amf::AmfOptions { dense_alpha: alpha })
                    .map(|(p, _)| p)
            }
            Algo::Metis { seed, switch, niparts } => {
                let o = feral_metis::MetisOptions {
                    seed,
                    nd_to_amd_switch: switch,
                    niparts,
                    ..feral_metis::MetisOptions::default()
                };
                feral_metis::metis_order_full(&view, &o).map(|(p, _, _)| p)
            }
            Algo::Scotch { seed } => {
                let o = feral_scotch::ScotchOptions { seed, ..feral_scotch::ScotchOptions::default() };
                feral_scotch::scotch_order_full(&view, &o).map(|(p, _, _)| p)
            }
            Algo::Kahip { seed, mode } => {
                let mode = match mode {
                    0 => feral_kahip::KahipMode::Fast,
                    1 => feral_kahip::KahipMode::Eco,
                    _ => feral_kahip::KahipMode::Strong,
                };
                let o = feral_kahip::KahipOptions { seed, mode };
                feral_kahip::kahip_order_full(&view, &o).map(|(p, _, _)| p)
            }
            Algo::Dense { .. } => unreachable!("handled above"),
        }));
        match r {
            Ok(Ok(p)) if p.len() == g.n => Some(p),
            _ => None,
        }
    }

    /// Run relabel + algorithm on `g` (a graph whose vertices are 0..m), and
    /// return an elimination order in `g`'s own vertex numbering (usize).
    fn run_on(&self, g: &Csc, deg: &[u32]) -> Option<Vec<usize>> {
        let m = g.n;
        match self.relabel {
            Relabel::None => self.run_algo(g).map(|p| p.into_iter().map(|x| x as usize).collect()),
            Relabel::Reverse => {
                let q: Vec<u32> = (0..m).map(|i| (m - 1 - i) as u32).collect();
                let gq = g.relabel(&q);
                let p = self.run_algo(&gq)?;
                Some(compose_relabel(&p, &invert_u32(&q)))
            }
            Relabel::Random(seed) => {
                let mut q: Vec<u32> = (0..m as u32).collect();
                SplitMix64::new(seed).shuffle(&mut q);
                let gq = g.relabel(&q);
                let p = self.run_algo(&gq)?;
                Some(compose_relabel(&p, &invert_u32(&q)))
            }
            Relabel::Degree { ascending } => {
                let mut order: Vec<u32> = (0..m as u32).collect();
                if ascending {
                    order.sort_unstable_by_key(|&v| (deg[v as usize], v));
                } else {
                    order.sort_unstable_by_key(|&v| (std::cmp::Reverse(deg[v as usize]), v));
                }
                // order[k] = vertex getting label k  ⇒ q[vertex] = k
                let mut q = vec![0u32; m];
                for (k, &v) in order.iter().enumerate() {
                    q[v as usize] = k as u32;
                }
                let gq = g.relabel(&q);
                let p = self.run_algo(&gq)?;
                Some(compose_relabel(&p, &invert_u32(&q)))
            }
            Relabel::Rcm { reverse } => {
                let q = rcm_labels(g, deg, reverse);
                let gq = g.relabel(&q);
                let p = self.run_algo(&gq)?;
                Some(compose_relabel(&p, &invert_u32(&q)))
            }
        }
    }

    /// Full recipe on the original graph. Returns a bijection of 0..n or None.
    pub fn run(&self, g: &Csc, deg: &[u32]) -> Option<Vec<usize>> {
        let n = g.n;
        let perm = match self.strip {
            Strip::None => self.run_on(g, deg)?,
            Strip::Threshold(t) => {
                let mut keep = vec![true; n];
                let mut hubs: Vec<u32> = Vec::new();
                for v in 0..n {
                    if deg[v] > t {
                        keep[v] = false;
                        hubs.push(v as u32);
                    }
                }
                if hubs.is_empty() || hubs.len() + 1 >= n {
                    self.run_on(g, deg)?
                } else {
                    let (sub, old_of) = g.induced(&keep);
                    let sub_deg = sub.degrees();
                    let inner = self.run_on(&sub, &sub_deg)?;
                    let mut perm: Vec<usize> = Vec::with_capacity(n);
                    perm.extend(inner.into_iter().map(|v| old_of[v] as usize));
                    // Tail: AMD on the hubs' induced subgraph (keeps whatever
                    // structure the hub block has), else ascending degree.
                    let mut inv_keep = vec![false; n];
                    for &h in &hubs {
                        inv_keep[h as usize] = true;
                    }
                    let (hsub, hold) = g.induced(&inv_keep);
                    let tail: Vec<usize> = match feral_amd::amd_order(&hsub.view()) {
                        Ok(p) if p.len() == hsub.n => p.into_iter().map(|v| hold[v as usize] as usize).collect(),
                        _ => {
                            hubs.sort_unstable_by_key(|&h| (deg[h as usize], h));
                            hubs.iter().map(|&h| h as usize).collect()
                        }
                    };
                    perm.extend(tail);
                    perm
                }
            }
            Strip::TopK(k) => {
                let k = (k as usize).min(n.saturating_sub(1));
                if k == 0 {
                    self.run_on(g, deg)?
                } else {
                    // hubs = k highest-degree vertices (ties: lower index first).
                    let mut idx: Vec<u32> = (0..n as u32).collect();
                    idx.select_nth_unstable_by_key(k - 1, |&v| {
                        (std::cmp::Reverse(deg[v as usize]), v)
                    });
                    let mut hubs: Vec<u32> = idx[..k].to_vec();
                    let mut keep = vec![true; n];
                    for &h in &hubs {
                        keep[h as usize] = false;
                    }
                    let (sub, old_of) = g.induced(&keep);
                    let sub_deg = sub.degrees();
                    let inner = self.run_on(&sub, &sub_deg)?;
                    let mut perm: Vec<usize> = Vec::with_capacity(n);
                    perm.extend(inner.into_iter().map(|v| old_of[v] as usize));
                    hubs.sort_unstable_by_key(|&h| (deg[h as usize], h));
                    perm.extend(hubs.into_iter().map(|h| h as usize));
                    perm
                }
            }
        };
        if is_bijection(&perm, n) {
            Some(perm)
        } else {
            None
        }
    }
}
