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

    /// Recipe census on selected matrices: ratio vs AMD and ms for a fixed list
    /// of single recipes. Env: PROBE_CORPUS (path), PROBE_NAMES (comma list).
    #[test]
    #[ignore]
    fn exp_recipes() {
        use crate::ordering::cands::{Algo, Cand, Relabel, Strip};
        let path = std::env::var("PROBE_CORPUS").unwrap_or_else(|_| "corpus/dev/patterns.jsonl".to_string());
        let names: Option<Vec<String>> = std::env::var("PROBE_NAMES").ok().map(|v| v.split(',').map(|x| x.trim().to_string()).collect());
        let file = std::fs::File::open(&path).expect("corpus");
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            let (name, pat): (String, Pattern) = ssi_scoring::pattern_from_jsonl_line(&line).expect("parse");
            if let Some(ns) = &names { if !ns.iter().any(|x| *x == name) { continue; } }
            let g = Csc::from_pattern(&pat);
            let deg = g.degrees();
            let spat = scoring_pattern(&pat);
            let n = pat.n; let nnz = pat.nnz();
            let maxdeg = *deg.iter().max().unwrap_or(&0) as usize;
            let amd: Vec<usize> = feral_amd::amd_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
            let base = flops_of(&spat, &amd).max(1);
            println!("### {name} n={n} nnz={nnz} maxdeg={maxdeg} amd_flops={base}");
            let amd = |alpha: f64| Algo::Amd { aggressive: true, alpha };
            let amdna = |alpha: f64| Algo::Amd { aggressive: false, alpha };
            let amf = |alpha: f64| Algo::Amf { alpha };
            let mut recipes: Vec<(String, Cand)> = Vec::new();
            for a in [10.0, 4.0, 2.0, 1.0, 0.5, 0.25, -1.0] {
                recipes.push((format!("amd({a})"), Cand::new(amd(a), Relabel::None, Strip::None, nnz)));
            }
            recipes.push(("amdna(10)".into(), Cand::new(amdna(10.0), Relabel::None, Strip::None, nnz)));
            recipes.push(("amdna(-1)".into(), Cand::new(amdna(-1.0), Relabel::None, Strip::None, nnz)));
            for a in [10.0, 4.0, 1.0, 0.5, -1.0] {
                recipes.push((format!("amf({a})"), Cand::new(amf(a), Relabel::None, Strip::None, nnz)));
            }
            recipes.push(("amd rev".into(), Cand::new(amd(10.0), Relabel::Reverse, Strip::None, nnz)));
            recipes.push(("amd rcm".into(), Cand::new(amd(10.0), Relabel::Rcm { reverse: true }, Strip::None, nnz)));
            recipes.push(("amf rev".into(), Cand::new(amf(10.0), Relabel::Reverse, Strip::None, nnz)));
            for k in [8u32, 32, 128, 512, 2048] {
                if (k as usize) * 4 > n { break; }
                recipes.push((format!("amd top{k}"), Cand::new(amd(10.0), Relabel::None, Strip::TopK(k), nnz)));
                recipes.push((format!("amf top{k}"), Cand::new(amf(10.0), Relabel::None, Strip::TopK(k), nnz)));
                recipes.push((format!("amd-1 top{k}"), Cand::new(amd(-1.0), Relabel::None, Strip::TopK(k), nnz)));
            }
            let avg = nnz as f64 / n as f64;
            for (lbl, t) in [("1.5avg", 1.5 * avg), ("2avg", 2.0 * avg), ("3avg", 3.0 * avg), ("6avg", 6.0 * avg), ("12avg", 12.0 * avg)] {
                let t = t.ceil() as u32;
                let cnt = deg.iter().filter(|&&d| d > t).count();
                if cnt == 0 || cnt * 3 > n { continue; }
                recipes.push((format!("amd thr{lbl}({cnt})"), Cand::new(amd(10.0), Relabel::None, Strip::Threshold(t), nnz)));
                recipes.push((format!("amf thr{lbl}({cnt})"), Cand::new(amf(10.0), Relabel::None, Strip::Threshold(t), nnz)));
                recipes.push((format!("amd-1 thr{lbl}({cnt})"), Cand::new(amd(-1.0), Relabel::None, Strip::Threshold(t), nnz)));
            }
            if nnz < 3_000_000 {
                recipes.push(("metis sw200".into(), Cand::new(Algo::Metis { seed: 1, switch: 200, niparts: 7 }, Relabel::None, Strip::None, nnz)));
                recipes.push(("metis sw n/2".into(), Cand::new(Algo::Metis { seed: 1, switch: (n / 2).max(16) as u32, niparts: 7 }, Relabel::None, Strip::None, nnz)));
                recipes.push(("metis sw n/8".into(), Cand::new(Algo::Metis { seed: 1, switch: (n / 8).max(16) as u32, niparts: 7 }, Relabel::None, Strip::None, nnz)));
            }
            if nnz < 400_000 {
                recipes.push(("scotch".into(), Cand::new(Algo::Scotch { seed: 0 }, Relabel::None, Strip::None, nnz)));
            }
            for (lbl, c) in recipes {
                let t = Instant::now();
                let r = c.run(&g, &deg);
                let ms = t.elapsed().as_secs_f64() * 1e3;
                match r {
                    Some(p) => {
                        let f = flops_of(&spat, &p);
                        println!("  {lbl:<22} ratio={:.4} {ms:8.1} ms", f as f64 / base as f64);
                    }
                    None => println!("  {lbl:<22} FAILED {ms:8.1} ms"),
                }
            }
        }
    }

    /// Minimal-triangulation descent experiment: delete fill edges of the
    /// incumbent's filled graph while it stays chordal (an edge of a chordal
    /// graph is removable iff the common neighbourhood of its endpoints is a
    /// clique). Σc² = 2T + 3|E| + n is a function of the filled graph, so every
    /// deletion is a strict gain. Reports the gain and time per matrix.
    /// Env: PROBE_CORPUS, PROBE_MAX_N (default 6000), PROBE_NAMES.
    #[test]
    #[ignore]
    fn exp_mintri() {
        let path = std::env::var("PROBE_CORPUS").unwrap_or_else(|_| "corpus/dev/patterns.jsonl".to_string());
        let max_n: usize = std::env::var("PROBE_MAX_N").ok().and_then(|v| v.parse().ok()).unwrap_or(6000);
        let names: Option<Vec<String>> = std::env::var("PROBE_NAMES").ok().map(|v| v.split(',').map(|x| x.trim().to_string()).collect());
        let file = std::fs::File::open(&path).expect("corpus");
        let reader = std::io::BufReader::new(file);
        let mut acc_old = 0.0f64;
        let mut acc_new = 0.0f64;
        let mut cnt = 0usize;
        let mut t_sum = 0.0f64;
        for line in reader.lines() {
            let line = line.unwrap();
            let (name, pat): (String, Pattern) = ssi_scoring::pattern_from_jsonl_line(&line).expect("parse");
            if let Some(ns) = &names { if !ns.iter().any(|x| *x == name) { continue; } }
            if pat.n > max_n || pat.n < 8 { continue; }
            let n = pat.n;
            let g = Csc::from_pattern(&pat);
            let spat = scoring_pattern(&pat);
            let amd: Vec<usize> = feral_amd::amd_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
            let base = flops_of(&spat, &amd).max(1) as f64;
            let perm = crate::ordering::order(&pat);
            let f0 = flops_of(&spat, &perm) as f64;
            let t = Instant::now();
            let words = (n + 63) / 64;
            let mut adj: Vec<u64> = vec![0u64; n * words];
            let set = |adj: &mut Vec<u64>, u: usize, v: usize| { adj[u * words + (v >> 6)] |= 1u64 << (v & 63); };
            for v in 0..n {
                for &u in g.col(v) {
                    set(&mut adj, v, u as usize);
                    set(&mut adj, u as usize, v);
                }
            }
            let orig = adj.clone();
            // elimination in perm order: clique among later neighbours
            let mut pos = vec![0usize; n];
            for (k, &v) in perm.iter().enumerate() { pos[v] = k; }
            let mut later_mask: Vec<u64> = vec![0u64; words]; // vertices with pos > current
            // build later_mask incrementally from the end: start with all, remove as we go
            for w in 0..words { later_mask[w] = !0u64; }
            if n % 64 != 0 { later_mask[words - 1] = (1u64 << (n % 64)) - 1; }
            let mut madj: Vec<u64> = vec![0u64; words];
            let mut madj_list: Vec<usize> = Vec::new();
            for k in 0..n {
                let v = perm[k];
                later_mask[v >> 6] &= !(1u64 << (v & 63));
                for w in 0..words { madj[w] = adj[v * words + w] & later_mask[w]; }
                madj_list.clear();
                for w in 0..words {
                    let mut x = madj[w];
                    while x != 0 { let b = x.trailing_zeros() as usize; madj_list.push(w * 64 + b); x &= x - 1; }
                }
                for &u in &madj_list {
                    for w in 0..words { adj[u * words + w] |= madj[w]; }
                    adj[u * words + (u >> 6)] &= !(1u64 << (u & 63));
                }
            }
            // fill edges
            let mut fill: Vec<(usize, usize)> = Vec::new();
            for u in 0..n {
                for w in 0..words {
                    let mut x = adj[u * words + w] & !orig[u * words + w];
                    while x != 0 { let b = x.trailing_zeros() as usize; let v = w * 64 + b; if v > u { fill.push((u, v)); } x &= x - 1; }
                }
            }
            let n_fill = fill.len();
            let mut removed = 0usize;
            let mut cbuf: Vec<u64> = vec![0u64; words];
            let mut clist: Vec<usize> = Vec::new();
            for _pass in 0..6 {
                let mut changed = false;
                // order: larger common neighbourhood first (bigger gain)
                let mut order: Vec<(usize, usize, usize)> = fill.iter().map(|&(u, v)| {
                    let mut c = 0usize;
                    for w in 0..words { c += (adj[u * words + w] & adj[v * words + w]).count_ones() as usize; }
                    (c, u, v)
                }).collect();
                order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
                let mut keep: Vec<(usize, usize)> = Vec::with_capacity(fill.len());
                for &(_, u, v) in &order {
                    if adj[u * words + (v >> 6)] & (1u64 << (v & 63)) == 0 { continue; }
                    for w in 0..words { cbuf[w] = adj[u * words + w] & adj[v * words + w]; }
                    clist.clear();
                    for w in 0..words { let mut x = cbuf[w]; while x != 0 { let b = x.trailing_zeros() as usize; clist.push(w * 64 + b); x &= x - 1; } }
                    let mut clique = true;
                    'outer: for &x in &clist {
                        for w in 0..words {
                            let mut need = cbuf[w] & !adj[x * words + w];
                            if w == (x >> 6) { need &= !(1u64 << (x & 63)); }
                            if need != 0 { clique = false; break 'outer; }
                        }
                    }
                    if clique {
                        adj[u * words + (v >> 6)] &= !(1u64 << (v & 63));
                        adj[v * words + (u >> 6)] &= !(1u64 << (u & 63));
                        removed += 1;
                        changed = true;
                    } else {
                        keep.push((u, v));
                    }
                }
                fill = keep;
                if !changed { break; }
            }
            // PEO of H via MCS (number from n-1 down to 0; elimination order = numbering order)
            let mut weight = vec![0usize; n];
            let mut numbered = vec![false; n];
            let mut peo = vec![0usize; n];
            for k in (0..n).rev() {
                let mut best = usize::MAX; let mut bw = 0usize;
                for v in 0..n { if !numbered[v] && (best == usize::MAX || weight[v] > bw) { best = v; bw = weight[v]; } }
                numbered[best] = true; peo[k] = best;
                for w in 0..words { let mut x = adj[best * words + w]; while x != 0 { let b = x.trailing_zeros() as usize; let u = w * 64 + b; if !numbered[u] { weight[u] += 1; } x &= x - 1; } }
            }
            let ms = t.elapsed().as_secs_f64() * 1e3;
            let f1 = flops_of(&spat, &peo) as f64;
            let f_best = f0.min(f1);
            acc_old += (f0 / base).ln(); acc_new += (f_best / base).ln(); cnt += 1; t_sum += ms;
            println!("{name:<28} n={n:<6} fill={n_fill:<7} removed={removed:<6} mine={:.4} mintri={:.4} {ms:8.1} ms", f0 / base, f1 / base);
        }
        println!("count={cnt} geomean mine={:.4} with-mintri={:.4} avg ms {:.1}", (acc_old / cnt as f64).exp(), (acc_new / cnt as f64).exp(), t_sum / cnt.max(1) as f64);
    }
}
