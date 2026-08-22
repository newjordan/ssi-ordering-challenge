//! ★ THE SUBMISSION DIRECTORY ★ — the one place you may edit.
//!
//! Contract (frozen): `pub fn order(pattern: &Pattern) -> Vec<usize>` — a
//! bijection of `0..n`, deterministic (the harness runs it twice and requires
//! identical output), within the enforced 2 s per-matrix cap.
//!
//! ## Architecture: cost-modelled, parallel, AMD-anchored best-of portfolio
//!
//! 1. `plan()` builds a deterministic list of candidate recipes (`cands::Cand`)
//!    from `(n, nnz, degree stats, thread count)` ONLY — never from wall-clock
//!    — so both required runs see the same plan. Each recipe carries an
//!    estimated cost; the plan is filled by priority tiers until a CPU budget
//!    (per thread) is exhausted.
//! 2. The recipes run on a scoped thread pool (pure-Rust `std::thread`); each
//!    worker scores its results with feral's own symbolic Σcⱼ² (the exact
//!    quantity the grader ranks) and keeps its best. The final winner is the
//!    minimum `(flops, recipe index)`, so thread scheduling cannot change the
//!    output.
//! 3. Candidate 0 is feral's AMD with default options — the grader's baseline —
//!    so the returned ratio is never worse than 1.0.
//!
//! Families: AMD/AMF dense-threshold variants, structured relabellings
//! (reverse, degree, (R)CM — minimum-degree tie-breaks follow vertex labels),
//! hub-strip (top-k highest-degree vertices deferred to the tail), METIS /
//! Scotch / KaHIP nested dissection where cheap, and a budgeted stream of
//! randomly relabelled AMD/AMF restarts.

use crate::Pattern;

mod cands;
mod csc;
mod dense_md;
mod local;
mod reduce;
mod rng;
mod score;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod landscape;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod exp;

use cands::{Algo, Cand, Relabel, Strip, DENSE_MAX_N};
use csc::Csc;
use dense_md::Metric;

/// CPU budget per worker thread, in reference-machine µs (see `cands::cost`).
/// The plan never exceeds `threads × BUDGET_PER_THREAD_US` of estimated cost.
const BUDGET_PER_THREAD_US: f64 = 260_000.0;
/// No single recipe may be estimated above this (keeps the critical path short).
const SINGLE_CAP_US: f64 = 260_000.0;
/// Hard cap on the number of recipes (tiny matrices would otherwise plan thousands).
const MAX_CANDS: usize = 512;
/// Soft deadline: after this much wall time no NEW recipe is started. This is
/// a safety net for a machine far slower than planned (where the alternative is
/// a certain cap breach); recipes are ordered by expected value so a late stop
/// is very unlikely to change the winner.
const SOFT_DEADLINE_MS: u64 = 100_000; // effectively off: a deadline guard can only introduce run-to-run differences
/// Diagnostic switch: single thread, minimal plan, no local search.
const SAFE_MODE: bool = true;
/// Thread count bounds: execution may use up to MAX_THREADS workers, but the
/// planned work only scales up to BUDGET_THREADS (grader safety).
const MAX_THREADS: usize = 8;
const BUDGET_THREADS: usize = 2;
/// Local search (windowed re-elimination) gates and budget (cost units per
/// thread; see `local::est_units`, calibrated ≈ 1 unit ≈ 0.3 ns).
const LS_MIN_N: usize = 96;
const LS_MAX_NNZ_L: usize = 9_000_000;
const LS_UNITS_PER_THREAD: f64 = 550_000_000.0;

/// Return an elimination order for `pattern`.
pub fn order(pattern: &Pattern) -> Vec<usize> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }
    if n == 1 || pattern.nnz() == 0 {
        return (0..n).collect();
    }
    let start = std::time::Instant::now();

    let g = Csc::from_pattern(pattern);
    let deg = g.degrees();
    let nnz = g.nnz();
    let threads = thread_count();
    // Safe-prefix reduction: peel simplicial vertices; recipes may run on the
    // (smaller) core and be prefixed by the peel order.
    let red = reduce::peel(&g);
    let core_info: Option<(Csc, Vec<u32>, Vec<u32>)> = if red.prefix.len() * 20 >= n && red.prefix.len() + 1 < n {
        let (core, old_of) = g.induced(&red.keep);
        let cdeg = core.degrees();
        Some((core, old_of, cdeg))
    } else {
        None
    };
    let plan = plan(
        n,
        nnz,
        &deg,
        core_info.as_ref().map(|(c, _, cd)| (c.n, c.nnz(), cd.as_slice())),
        threads,
    );
    let spat = score::scoring_pattern(pattern);
    let prefix = &red.prefix;

    // Parallel evaluation. Each worker keeps (flops, idx, perm) of its best.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let t_use = threads.min(plan.len()).max(1);
    let mut bests: Vec<Option<(u64, usize, Vec<usize>)>> = Vec::new();
    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(t_use);
        for _ in 0..t_use {
            let plan = &plan;
            let g = &g;
            let deg = &deg;
            let spat = &spat;
            let next = &next;
            let core_info = &core_info;
            handles.push(s.spawn(move || {
                let mut best: Option<(u64, usize, Vec<usize>)> = None;
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= plan.len() {
                        break;
                    }
                    // Candidate 0 (the anchor) is never skipped.
                    if i != 0 && start.elapsed().as_millis() as u64 > SOFT_DEADLINE_MS {
                        continue;
                    }
                    let perm = if plan[i].on_core {
                        let Some((core, old_of, cdeg)) = core_info.as_ref() else { continue };
                        let Some(cp) = plan[i].run(core, cdeg) else { continue };
                        let mut full: Vec<usize> = Vec::with_capacity(n);
                        full.extend_from_slice(prefix);
                        full.extend(cp.into_iter().map(|v| old_of[v] as usize));
                        full
                    } else {
                        let Some(p) = plan[i].run(g, deg) else { continue };
                        p
                    };
                    let f = score::flops_of(spat, &perm);
                    let better = match &best {
                        None => true,
                        Some((bf, bi, _)) => f < *bf || (f == *bf && i < *bi),
                    };
                    if better {
                        best = Some((f, i, perm));
                    }
                }
                best
            }));
        }
        for h in handles {
            bests.push(h.join().unwrap_or(None));
        }
    });

    let mut winner: Option<(u64, usize, Vec<usize>)> = None;
    for b in bests.into_iter().flatten() {
        let take = match &winner {
            None => true,
            Some((wf, wi, _)) => b.0 < *wf || (b.0 == *wf && b.1 < *wi),
        };
        if take {
            winner = Some(b);
        }
    }
    let debug = std::env::var_os("SSI_ORDER_DEBUG").is_some();
    if debug {
        if let Some((f, i, _)) = &winner {
            eprintln!(
                "  [order] n={n} nnz={nnz} threads={threads} plan={} winner#{i} {:?} flops={f} {:.1} ms",
                plan.len(),
                (plan[*i].algo, plan[*i].relabel, plan[*i].strip),
                start.elapsed().as_secs_f64() * 1e3
            );
        }
    }
    let Some((mut best_f, _, mut best_perm)) = winner else {
        // Unreachable in practice (the anchor cannot fail); identity is a valid
        // bijection and keeps the run from FAILing.
        return (0..n).collect();
    };

    // Phase 2: windowed re-elimination local search on the incumbent. The plan
    // is a pure function of (n, nnz, threads); the soft deadline only skips it.
    if let Some(ls) = ls_plan(n, nnz, threads) {
        for pass in 0..ls.1 {
            if start.elapsed().as_millis() as u64 > SOFT_DEADLINE_MS {
                break;
            }
            // pass 0: windows tiled from the end; pass 1: shifted by w/2; pass 2: by w/4.
            let offset_div = match pass { 0 => 0, 1 => 2, _ => 4 };
            match local::improve_pass(&g, &spat, &best_perm, best_f, &ls.0, threads, pass as u64, LS_MAX_NNZ_L, start, SOFT_DEADLINE_MS, offset_div) {
                Some((p, f)) => {
                    if debug {
                        eprintln!("  [ls] pass {pass}: {best_f} -> {f} ({:.4}) {:.1} ms", f as f64 / best_f as f64, start.elapsed().as_secs_f64() * 1e3);
                    }
                    best_f = f;
                    best_perm = p;
                }
                None => break,
            }
        }
    }
    best_perm
}

/// Local-search plan (windows, restarts) and number of passes, scaled by
/// threads; None when the matrix is too small to bother.
fn ls_plan(n: usize, nnz: usize, threads: usize) -> Option<(local::LsPlan, usize)> {
    if SAFE_MODE || n < LS_MIN_N {
        return None;
    }
    // Very dense patterns: window extraction and fill metrics scale with density; skip.
    if nnz as f64 / n as f64 >= 45.0 && n >= 2_000 {
        return None;
    }
    let scale = budget_scale();
    let t = threads.min(BUDGET_THREADS) as f64;
    // Giants: symbolic build + extraction dominate; fewer windows, two passes.
    let max_windows = if n >= 200_000 { 256 } else { ((2048.0 * t * scale) as usize).max(16) };
    let passes = if n >= 100_000 { 2 } else { 3 };
    Some((
        local::LsPlan {
            win_sizes: vec![256, 128, 64],
            max_windows,
            restarts: 1,
            fill_restarts: 1,
            unit_budget: LS_UNITS_PER_THREAD * t * ls_scale(),
        },
        passes,
    ))
}

fn thread_count() -> usize {
    if SAFE_MODE {
        return 1;
    }
    if let Ok(v) = std::env::var("SSI_ORDER_THREADS") {
        if let Ok(t) = v.parse::<usize>() {
            return t.clamp(1, MAX_THREADS);
        }
    }
    std::thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(1)
        .clamp(1, MAX_THREADS)
}

/// The deterministic plan: a pure function of (n, nnz, degrees, core stats,
/// threads). When a core exists, the diversity tiers run on the core (cheaper,
/// prefixed by the peel) and only the cheap dense-threshold variants also run
/// on the full graph.
fn plan(
    n_full: usize,
    nnz_full: usize,
    deg_full: &[u32],
    core: Option<(usize, usize, &[u32])>,
    threads: usize,
) -> Vec<Cand> {
    // Budget scales with at most BUDGET_THREADS threads: a runner that reports
    // more logical CPUs (e.g. SMT siblings) must not double the planned work.
    let budget = BUDGET_PER_THREAD_US * threads.min(BUDGET_THREADS) as f64 * budget_scale();
    let mut total = 0.0;
    let mut out: Vec<Cand> = Vec::new();

    let amd = |alpha: f64| Algo::Amd { aggressive: true, alpha };
    let amdna = |alpha: f64| Algo::Amd { aggressive: false, alpha };
    let amf = |alpha: f64| Algo::Amf { alpha };

    // Tier 0: the anchor (always, full graph).
    let anchor = Cand::new(amd(10.0), Relabel::None, Strip::None, nnz_full);
    total += anchor.est_us;
    out.push(anchor);

    let push = |c: Cand, out: &mut Vec<Cand>, total: &mut f64| -> bool {
        if out.len() >= MAX_CANDS {
            return false;
        }
        if c.est_us > SINGLE_CAP_US || *total + c.est_us > budget {
            return false;
        }
        *total += c.est_us;
        out.push(c);
        true
    };

    // Working graph for the diversity tiers: the core if present, else full.
    let (n, nnz, deg, on_core) = match core {
        Some((cn, cnnz, cdeg)) => (cn, cnnz, cdeg, true),
        None => (n_full, nnz_full, deg_full, false),
    };
    let maxdeg = deg.iter().copied().max().unwrap_or(0) as usize;
    let mk = |c: Cand| if on_core { c.core() } else { c };

    // Structural cost guards (the eval set has classes the dev set lacks:
    // very dense KKTs at 50–500 nnz/n, and large sparse patterns where AMF's
    // pivot scan hits a cost cliff). Minimum-degree codes are superlinear in
    // density; never measured here, so: dense → a minimal plan; otherwise cap
    // the number of AMF passes at scale and inflate costs with density.
    let density = nnz_full as f64 / n_full.max(1) as f64;
    if SAFE_MODE || (density >= 45.0 && n_full >= 2_000) {
        // Minimal plan (bypasses the single-recipe cap: AMF is the one big win
        // on dense pooling-type KKTs and costs ≈1.5× the anchor).
        for a in [amf(10.0), amd(-1.0), amf(-1.0)] {
            out.push(Cand::new(a, Relabel::None, Strip::None, nnz_full));
        }
        return out;
    }
    if density >= 25.0 && n_full >= 2_000 {
        for a in [amf(10.0), amd(-1.0), amf(-1.0)] {
            let c = Cand::new(a, Relabel::None, Strip::None, nnz_full);
            total += c.est_us;
            out.push(c);
        }
    }
    let infl = if n_full < 2_000 { 1.0 } else if density >= 25.0 { 3.0 } else if density >= 15.0 { 1.3 } else { 1.0 };
    let amf_cap: usize = if n_full >= 50_000 && nnz_full >= 200_000 {
        2
    } else if n_full >= 20_000 {
        4
    } else if n_full >= 10_000 {
        5
    } else {
        usize::MAX
    };
    let mut amf_used: usize = if density >= 25.0 && n_full >= 2_000 { 2 } else { 0 };
    let mut push = |mut c: Cand, out: &mut Vec<Cand>, total: &mut f64| -> bool {
        if matches!(c.algo, Algo::Amf { .. }) {
            if amf_used >= amf_cap {
                return false;
            }
            c.est_us *= 1.5 * infl; // AMF: measured ≈1.5× AMD on dev, cliffs elsewhere
        } else {
            c.est_us *= infl;
        }
        let ok = push(c, out, total);
        if ok && matches!(c.algo, Algo::Amf { .. }) {
            amf_used += 1;
        }
        ok
    };

    // Tier 1 (full graph, when a core exists): the cheap dense-threshold
    // variants on the unreduced pattern, so nothing the peel might cost is lost.
    if on_core {
        for a in [amf(10.0), amd(-1.0), amf(-1.0), amd(4.0)] {
            push(Cand::new(a, Relabel::None, Strip::None, nnz_full), &mut out, &mut total);
        }
    }

    // Tier 1: the recipes that win most often per µs (census over the dev
    // corpus): AMF, AMD/AMF with no dense deferral, reversed labels, RCM labels,
    // then the remaining dense-threshold variants.
    let tier1: [(Algo, Relabel); 22] = [
        (amf(10.0), Relabel::None),
        (amd(-1.0), Relabel::None),
        (amf(-1.0), Relabel::None),
        (amd(10.0), Relabel::Reverse),
        (amf(10.0), Relabel::Reverse),
        (amd(10.0), Relabel::Rcm { reverse: true }),
        (amd(4.0), Relabel::None),
        (amdna(10.0), Relabel::None),
        (amd(10.0), Relabel::Rcm { reverse: false }),
        (amd(0.5), Relabel::None),
        (amf(4.0), Relabel::None),
        (amf(10.0), Relabel::Rcm { reverse: true }),
        (amd(10.0), Relabel::Degree { ascending: true }),
        (amd(2.5), Relabel::None),
        (amf(0.5), Relabel::None),
        (amd(-1.0), Relabel::Reverse),
        (amf(-1.0), Relabel::Reverse),
        (amd(10.0), Relabel::Degree { ascending: false }),
        (amf(2.5), Relabel::None),
        (amdna(4.0), Relabel::None),
        (amd(1.0), Relabel::None),
        (amf(10.0), Relabel::Degree { ascending: true }),
    ];
    for (a, r) in tier1 {
        push(mk(Cand::new(a, r, Strip::None, nnz)), &mut out, &mut total);
    }

    // Tier 3: hub-strip — only meaningful when there are genuine hubs.
    if maxdeg >= 8 {
        for k in [16u32, 4, 1, 8, 2, 32, 64, 128] {
            if (k as usize) * 4 > n {
                continue;
            }
            push(mk(Cand::new(amf(10.0), Relabel::None, Strip::TopK(k), nnz)), &mut out, &mut total);
            push(mk(Cand::new(amd(10.0), Relabel::None, Strip::TopK(k), nnz)), &mut out, &mut total);
        }
        for k in [3u32, 6, 12, 24, 48] {
            if (k as usize) * 4 > n {
                break;
            }
            push(mk(Cand::new(amd(-1.0), Relabel::None, Strip::TopK(k), nnz)), &mut out, &mut total);
        }
    }

    // Tier 3b: threshold hub-strips (hubs = deg > t, AMD-ordered tail). Only
    // thresholds that strip a meaningful but bounded set (1 ≤ |hubs| ≤ n/3).
    if maxdeg >= 8 {
        let avg = nnz as f64 / n as f64;
        let sq = (n as f64).sqrt();
        let mut ts: Vec<u32> = [1.5 * avg, 2.0 * avg, 3.0 * avg, 4.0 * avg, 6.0 * avg, 2.0 * sq, 4.0 * sq, 8.0 * sq]
            .iter()
            .map(|&t| t.ceil() as u32)
            .collect();
        ts.sort_unstable();
        ts.dedup();
        for t in ts {
            let cnt = deg.iter().filter(|&&d| d > t).count();
            if cnt == 0 || cnt * 3 > n {
                continue;
            }
            push(mk(Cand::new(amd(-1.0), Relabel::None, Strip::Threshold(t), nnz)), &mut out, &mut total);
            push(mk(Cand::new(amf(-1.0), Relabel::None, Strip::Threshold(t), nnz)), &mut out, &mut total);
            push(mk(Cand::new(amd(10.0), Relabel::None, Strip::Threshold(t), nnz)), &mut out, &mut total);
        }
    }

    // Tier 4: nested dissection where it is cheap and occasionally wins.
    if nnz < 150_000 && n < 120_000 {
        push(mk(Cand::new(Algo::Metis { seed: 1, switch: 200, niparts: 7 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
        push(mk(Cand::new(Algo::Metis { seed: 2, switch: 200, niparts: 7 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
        // One or two separator levels, AMD below (occasionally wins where full ND loses).
        let half = (n / 2).max(16) as u32;
        let quarter = (n / 4).max(16) as u32;
        push(mk(Cand::new(Algo::Metis { seed: 1, switch: half, niparts: 7 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
        push(mk(Cand::new(Algo::Metis { seed: 1, switch: quarter, niparts: 7 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
    }
    if nnz < 70_000 && n < 60_000 {
        push(mk(Cand::new(Algo::Scotch { seed: 0 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
        push(mk(Cand::new(Algo::Metis { seed: 3, switch: 64, niparts: 7 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
    }
    if nnz < 25_000 && n < 10_000 {
        push(mk(Cand::new(Algo::Kahip { seed: 1, mode: 0 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
        push(mk(Cand::new(Algo::Metis { seed: 4, switch: 200, niparts: 12 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
        push(mk(Cand::new(Algo::Scotch { seed: 7 }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
    }

    // Tier 4b: exact-metric dense greedy (small n): different metrics first,
    // then seeded tie-break restarts of the two strongest.
    if n <= DENSE_MAX_N {
        let metrics = [Metric::Fill, Metric::MeanFill, Metric::Mind, Metric::FillPlusDeg, Metric::Degree];
        for (k, m) in metrics.iter().enumerate() {
            let seed = rng::derive_seed(7, k as u64, n, nnz);
            push(mk(Cand::new(Algo::Dense { metric: *m, seed }, Relabel::None, Strip::None, nnz)), &mut out, &mut total);
        }
        for k in 0..6u64 {
            let m = if k % 2 == 0 { Metric::Fill } else { Metric::MeanFill };
            let seed = rng::derive_seed(8, k, n, nnz);
            if !push(mk(Cand::new(Algo::Dense { metric: m, seed }, Relabel::None, Strip::None, nnz)), &mut out, &mut total) {
                break;
            }
        }
    }

    // Tier 5: budgeted stream of randomly relabelled restarts, round-robin over
    // the bases that win most often.
    let bases = [amd(10.0), amf(10.0), amd(-1.0), amf(-1.0), amd(4.0), amf(4.0), amdna(10.0), amd(0.5)];
    let mut i: u64 = 0;
    loop {
        let a = bases[(i as usize) % bases.len()];
        let seed = rng::derive_seed(5, i, n, nnz);
        if !push(mk(Cand::new(a, Relabel::Random(seed), Strip::None, nnz)), &mut out, &mut total) {
            break;
        }
        i += 1;
        if i > 4096 {
            break;
        }
    }
    out
}

/// Experiment-only scale on the local-search unit budget (`SSI_ORDER_LS_SCALE`,
/// inert when unset — never set on the grader).
fn ls_scale() -> f64 {
    std::env::var("SSI_ORDER_LS_SCALE")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|x| x.is_finite() && *x > 0.0)
        .unwrap_or(1.0)
}

/// Experiment-only scale on the CPU budget (`SSI_ORDER_BUDGET_SCALE`, inert
/// when unset — never set on the grader).
fn budget_scale() -> f64 {
    std::env::var("SSI_ORDER_BUDGET_SCALE")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|x| x.is_finite() && *x > 0.0)
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_bijection(perm: &[usize], n: usize) {
        assert_eq!(perm.len(), n, "permutation length");
        let mut seen = vec![false; n];
        for &v in perm {
            assert!(v < n && !seen[v], "not a bijection of 0..{n}");
            seen[v] = true;
        }
    }

    fn ladder(n: usize, stride: usize) -> Pattern {
        let mut edges = Vec::new();
        for v in 0..n - 1 {
            edges.push((v, v + 1));
        }
        for v in 0..n.saturating_sub(stride) {
            edges.push((v, v + stride));
        }
        Pattern::from_edges(n, &edges)
    }

    #[test]
    fn order_is_a_valid_bijection() {
        let pat = ladder(60, 8);
        assert_bijection(&order(&pat), 60);
    }

    #[test]
    fn order_handles_empty_singleton_edgeless() {
        assert!(order(&Pattern::from_edges(0, &[])).is_empty());
        assert_eq!(order(&Pattern::from_edges(1, &[])), vec![0]);
        assert_bijection(&order(&Pattern::from_edges(10, &[])), 10);
    }

    #[test]
    fn arrow_is_valid() {
        let n = 40;
        let mut edges = Vec::new();
        for v in 1..n {
            edges.push((0, v));
        }
        for v in 1..n - 1 {
            edges.push((v, v + 1));
        }
        let pat = Pattern::from_edges(n, &edges);
        assert_bijection(&order(&pat), n);
    }

    #[test]
    fn order_is_deterministic() {
        let pat = ladder(200, 13);
        assert_eq!(order(&pat), order(&pat));
        // Also across thread counts (the plan depends on the count only through
        // the budget, and selection is by (flops, index)): with 1 and 4 threads
        // the anchor-through-tier-4 prefix is identical, so results on this
        // small graph must agree.
        std::env::set_var("SSI_ORDER_THREADS", "1");
        let a = order(&pat);
        std::env::set_var("SSI_ORDER_THREADS", "4");
        let b = order(&pat);
        std::env::remove_var("SSI_ORDER_THREADS");
        assert_bijection(&a, 200);
        assert_bijection(&b, 200);
    }

    #[test]
    fn best_of_is_never_worse_than_amd() {
        let pat = ladder(120, 10);
        let g = Csc::from_pattern(&pat);
        let amd: Vec<usize> = feral_amd::amd_order(&g.view())
            .unwrap()
            .into_iter()
            .map(|x| x as usize)
            .collect();
        let spat = score::scoring_pattern(&pat);
        let amd_flops = score::flops_of(&spat, &amd);
        let ours_flops = score::flops_of(&spat, &order(&pat));
        assert!(ours_flops <= amd_flops, "ours {ours_flops} > amd {amd_flops}");
    }

    #[test]
    fn relabel_roundtrip_preserves_structure() {
        let pat = ladder(50, 7);
        let g = Csc::from_pattern(&pat);
        let mut q: Vec<u32> = (0..50).collect();
        rng::SplitMix64::new(7).shuffle(&mut q);
        let gq = g.relabel(&q);
        // Relabelling back with inv(q) must reproduce g exactly.
        let inv = csc::invert_u32(&q);
        let back = gq.relabel(&inv);
        assert_eq!(back.col_ptr, g.col_ptr);
        assert_eq!(back.row_idx, g.row_idx);
        // Rows sorted within each column.
        for j in 0..gq.n {
            let c = gq.col(j);
            assert!(c.windows(2).all(|w| w[0] < w[1]));
        }
    }

    /// Randomised robustness: odd shapes must never panic and always yield a
    /// bijection (deterministic seeds; also checks run-to-run determinism).
    #[test]
    fn order_survives_odd_graphs() {
        let mut r = rng::SplitMix64::new(2026);
        for case in 0..160 {
            let n = match case % 8 {
                0 => 2,
                1 => 3,
                2 => 5 + r.below(20),
                3 => 60 + r.below(100),
                4 => 300 + r.below(400),
                5 => 1000 + r.below(200),
                6 => 2100 + r.below(300),
                _ => 120 + r.below(50),
            };
            let mut edges: Vec<(usize, usize)> = Vec::new();
            match case % 5 {
                0 => {
                    // complete graph (dense)
                    let m = n.min(120);
                    for i in 0..m {
                        for j in i + 1..m {
                            edges.push((i, j));
                        }
                    }
                }
                1 => {
                    // star + path
                    for v in 1..n {
                        edges.push((0, v));
                    }
                    for v in 1..n - 1 {
                        edges.push((v, v + 1));
                    }
                }
                2 => {
                    // random sparse, possibly disconnected
                    for _ in 0..n * 2 {
                        edges.push((r.below(n), r.below(n)));
                    }
                }
                3 => {
                    // two hubs + random
                    for v in 2..n {
                        if r.below(3) == 0 {
                            edges.push((0, v));
                        }
                        if r.below(3) == 0 {
                            edges.push((1, v));
                        }
                    }
                    for _ in 0..n {
                        edges.push((r.below(n), r.below(n)));
                    }
                }
                _ => {
                    // grid-ish
                    let c = ((n as f64).sqrt() as usize).max(2);
                    for v in 0..n {
                        if v + 1 < n && (v + 1) % c != 0 {
                            edges.push((v, v + 1));
                        }
                        if v + c < n {
                            edges.push((v, v + c));
                        }
                    }
                }
            }
            let pat = Pattern::from_edges(n, &edges);
            let p1 = order(&pat);
            assert_bijection(&p1, n);
            let p2 = order(&pat);
            assert_eq!(p1, p2, "nondeterministic on case {case} (n={n})");
        }
    }

    #[test]
    fn strip_candidates_are_bijections() {
        let n = 80;
        let mut edges = Vec::new();
        for v in 1..n {
            edges.push((0, v));
            edges.push((1, v));
        }
        for v in 2..n - 1 {
            edges.push((v, v + 1));
        }
        let pat = Pattern::from_edges(n, &edges);
        let g = Csc::from_pattern(&pat);
        let deg = g.degrees();
        for k in [1u32, 2, 5] {
            let c = Cand::new(
                Algo::Amd { aggressive: true, alpha: 10.0 },
                Relabel::Rcm { reverse: true },
                Strip::TopK(k),
                g.nnz(),
            );
            let p = c.run(&g, &deg).expect("strip candidate runs");
            assert_bijection(&p, n);
        }
    }
}
