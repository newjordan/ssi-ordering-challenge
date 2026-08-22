//! Ignored experiments (never run by the grader).

#[cfg(test)]
mod exp_tests {
    use crate::ordering::csc::Csc;
    use crate::ordering::score::{flops_of, scoring_pattern};
    use crate::Pattern;
    use std::io::BufRead;
    use std::time::Instant;

    fn corpus(min_nnz: usize, max_nnz: usize) -> Vec<(String, Pattern)> {
        let file = std::fs::File::open("corpus/dev/patterns.jsonl").expect("corpus");
        let reader = std::io::BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line.unwrap();
            let (name, pat): (String, Pattern) =
                ssi_scoring::pattern_from_jsonl_line(&line).expect("parse");
            if pat.nnz() >= min_nnz && pat.nnz() <= max_nnz {
                out.push((name, pat));
            }
        }
        out
    }

    /// METIS with a large nd_to_amd_switch: a few separator levels, AMD below.
    #[test]
    #[ignore]
    fn exp_nd_switch() {
        for (name, pat) in corpus(30_000, 700_000) {
            let g = Csc::from_pattern(&pat);
            let spat = scoring_pattern(&pat);
            let amd: Vec<usize> =
                feral_amd::amd_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
            let base = flops_of(&spat, &amd).max(1);
            let n = pat.n;
            let mut line = format!("{name:<28} n={n:<7} nnz={:<8}", pat.nnz());
            for sw in [200u32, 2000, (n / 16) as u32, (n / 4) as u32, (n / 2) as u32] {
                let t = Instant::now();
                let o = feral_metis::MetisOptions { nd_to_amd_switch: sw.max(16), ..Default::default() };
                let r = feral_metis::metis_order_full(&g.view(), &o);
                let ms = t.elapsed().as_secs_f64() * 1e3;
                let ratio = r
                    .ok()
                    .map(|(p, _, _)| {
                        let p: Vec<usize> = p.into_iter().map(|x| x as usize).collect();
                        flops_of(&spat, &p) as f64 / base as f64
                    })
                    .unwrap_or(-1.0);
                line.push_str(&format!(" | sw{sw}: {ratio:.3} ({ms:.0}ms)"));
            }
            println!("{line}");
        }
    }


    /// Incumbent-guided relabelling: relabel by the incumbent's positions
    /// (reversed: AMD picks the highest label among ties) and re-run AMD/AMF;
    /// iterate a few rounds keeping the best.
    #[test]
    #[ignore]
    fn exp_guided() {
        use crate::ordering::csc::{compose_relabel, invert_u32};
        for (name, pat) in corpus(30_000, 700_000) {
            let g = Csc::from_pattern(&pat);
            let spat = scoring_pattern(&pat);
            let n = pat.n;
            let amd: Vec<usize> =
                feral_amd::amd_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
            let base = flops_of(&spat, &amd).max(1);
            let t = Instant::now();
            let mut inc = amd.clone();
            let mut inc_f = base;
            let mut trail = String::new();
            for round in 0..6 {
                // q[v] = label; reversed positions so earlier-in-incumbent gets higher label
                let mut q = vec![0u32; n];
                for (k, &v) in inc.iter().enumerate() {
                    q[v] = (n - 1 - k) as u32;
                }
                let gq = g.relabel(&q);
                let inv = invert_u32(&q);
                let mut best_round: Option<(u64, Vec<usize>)> = None;
                for which in 0..4 {
                    let r = match which {
                        0 => feral_amd::amd_order(&gq.view()),
                        1 => feral_amf::amf_order(&gq.view()),
                        2 => feral_amd::amd_order_opts(&gq.view(), &feral_amd::AmdOptions { aggressive: true, dense_alpha: -1.0 }).map(|(p, _)| p),
                        _ => feral_amf::amf_order_opts(&gq.view(), &feral_amf::AmfOptions { dense_alpha: -1.0 }).map(|(p, _)| p),
                    };
                    if let Ok(p) = r {
                        let perm = compose_relabel(&p, &inv);
                        let f = flops_of(&spat, &perm);
                        if best_round.as_ref().map_or(true, |(bf, _)| f < *bf) {
                            best_round = Some((f, perm));
                        }
                    }
                }
                let (f, perm) = best_round.unwrap();
                trail.push_str(&format!(" r{round}:{:.4}", f as f64 / base as f64));
                if f < inc_f {
                    inc_f = f;
                    inc = perm;
                } else {
                    break;
                }
            }
            let ms = t.elapsed().as_secs_f64() * 1e3;
            println!("{name:<28} n={n:<7} nnz={:<8} final {:.4} ({ms:.0}ms){trail}", pat.nnz(), inc_f as f64 / base as f64);
        }
    }


    /// Share of Σc² held by the last L columns of the AMD ordering.
    #[test]
    #[ignore]
    fn exp_tail_share() {
        use crate::ordering::score::column_counts_of;
        for (name, pat) in corpus(30_000, 2_000_000) {
            let g = Csc::from_pattern(&pat);
            let spat = scoring_pattern(&pat);
            let n = pat.n;
            let amd: Vec<usize> =
                feral_amd::amd_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
            let cc = column_counts_of(&spat, &amd);
            let total: f64 = cc.iter().map(|&c| (c as f64) * (c as f64)).sum();
            let mut line = format!("{name:<28} n={n:<7} nnz={:<8} maxc={:<6}", pat.nnz(), cc.iter().max().unwrap());
            for l in [100usize, 300, 800, 2000, 5000] {
                if l >= n { break; }
                let tail: f64 = cc[n - l..].iter().map(|&c| (c as f64) * (c as f64)).sum();
                line.push_str(&format!(" | L{l}: {:.0}%", 100.0 * tail / total));
            }
            println!("{line}");
        }
    }


    /// LS strategy comparison on heavy windows: gain after k restarts of each
    /// metric (0 deg, 1 lazy fill, 2 exact fill, 3 two-hop fill) and time.
    #[test]
    #[ignore]
    fn exp_ls_strategies() {
        use crate::ordering::local::{build, extract, greedy, Scratch};
        use crate::ordering::score::column_counts_of;
        let names = ["pooling_sppc1pq", "gams05", "crudeoil_lee4_09", "arki0013", "nuclear10a", "unitcommit_200_100_1_mod_8", "methanol400", "transswitch2383wpr", "faclay30", "mpbp_48", "crudeoil_lee4_10", "gabriel09"];
        let mut tot = [[0u64; 6]; 4];   // metric -> gain after 1,2,4,8,12,16 restarts
        let mut tms = [0.0f64; 4];
        for (name, pat) in corpus(30_000, 2_000_000) {
            if !names.contains(&name.as_str()) { continue; }
            let g = Csc::from_pattern(&pat);
            let spat = scoring_pattern(&pat);
            let n = pat.n;
            let perm: Vec<usize> = feral_amf::amf_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
            let cc = column_counts_of(&spat, &perm);
            let Some(s) = build(&g, &perm, &cc, usize::MAX) else { continue };
            // top-24 windows of 256 by weight, non-overlapping tiling from the end
            let w = 256usize;
            if n < 2 * w { continue; }
            let mut cands: Vec<(u64, usize)> = Vec::new();
            let mut t1 = n;
            while t1 >= w { let t0 = t1 - w; cands.push(((t0..t1).map(|j| (cc[j] as u64) * (cc[j] as u64)).sum(), t0)); if t0 == 0 { break; } t1 = t0; }
            cands.sort_by(|a, b| b.0.cmp(&a.0));
            let mut scratch = Scratch::new(n);
            let mut buf = Vec::new();
            let mut line = format!("{name:<26}");
            let mut mg = [[0u64; 6]; 4];
            let mut mt = [0.0f64; 4];
            for &(_, t0) in cands.iter().take(24) {
                let win = extract(&s, t0, w, &mut scratch);
                for metric in 0..4u8 {
                    let t = Instant::now();
                    let mut best = win.cur_cost;
                    for r in 0..16 {
                        let (_, cost) = greedy(&win, 1000 + r as u64, metric, &mut buf);
                        if cost < best { best = cost; }
                        let k = match r { 0 => Some(0), 1 => Some(1), 3 => Some(2), 7 => Some(3), 11 => Some(4), 15 => Some(5), _ => None };
                        if let Some(k) = k { mg[metric as usize][k] += win.cur_cost - best; }
                    }
                    mt[metric as usize] += t.elapsed().as_secs_f64() * 1e3;
                }
            }
            let base: u64 = cands.iter().take(24).map(|c| c.0).sum();
            for metric in 0..4 {
                line.push_str(&format!(" | m{metric}: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}/{:.2}% {:.0}ms", 100.0 * mg[metric][0] as f64 / base as f64, 100.0 * mg[metric][1] as f64 / base as f64, 100.0 * mg[metric][2] as f64 / base as f64, 100.0 * mg[metric][3] as f64 / base as f64, 100.0 * mg[metric][4] as f64 / base as f64, 100.0 * mg[metric][5] as f64 / base as f64, mt[metric]));
                for k in 0..6 { tot[metric][k] += mg[metric][k]; }
                tms[metric] += mt[metric];
            }
            println!("{line}");
        }
        println!("TOTAL gains (abs) per metric after 1/2/4/8/12/16 restarts, and ms:");
        for metric in 0..4 { println!("  m{metric}: {:?} {:.0}ms", tot[metric], tms[metric]); }
    }

    /// Simplicial / low-degree reduction potential: fraction of n and nnz that
    /// a safe-prefix reduction would remove.
    #[test]
    #[ignore]
    fn exp_reduction_potential() {
        for (name, pat) in corpus(20_000, usize::MAX) {
            let g = Csc::from_pattern(&pat);
            let t = Instant::now();
            let (kept_n, kept_nnz, prefix_len) = crate::ordering::reduce::measure(&g);
            let ms = t.elapsed().as_secs_f64() * 1e3;
            println!(
                "{name:<28} n={:<7} nnz={:<8} core_n={kept_n:<7} ({:.0}%) core_nnz={kept_nnz:<8} ({:.0}%) prefix={prefix_len} {ms:.1}ms",
                pat.n,
                pat.nnz(),
                100.0 * kept_n as f64 / pat.n as f64,
                100.0 * kept_nnz as f64 / pat.nnz().max(1) as f64
            );
        }
    }
}
