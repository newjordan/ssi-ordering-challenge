//! Ignored-by-default probe: run `order()` on every dev matrix in-process,
//! measure wall time and flop ratio vs the AMD baseline, print the slowest
//! matrices, every per-matrix ratio (optional), and the bucketed score.
//!
//!   SSI_ORDER_THREADS=2 cargo test --release --offline --locked probe_all \
//!       -- --ignored --nocapture --test-threads=1
//!
//! Env: PROBE_CSV=/path/out.csv writes per-matrix rows; PROBE_MIN_NNZ filters.

#[cfg(test)]
mod probe_tests {
    use crate::Pattern;
    use std::io::{BufRead, Write};
    use std::time::Instant;

    #[test]
    #[ignore]
    fn probe_all() {
        let min_nnz: usize = std::env::var("PROBE_MIN_NNZ")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let max_n: usize = std::env::var("PROBE_MAX_N")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(usize::MAX);
        let name_filter = std::env::var("PROBE_NAME").ok();
        let csv_path = std::env::var("PROBE_CSV").ok();
        let mut csv = csv_path.map(|p| std::fs::File::create(p).expect("csv"));
        if let Some(f) = csv.as_mut() {
            writeln!(f, "name,n,nnz,ms,ratio,amd_flops,our_flops").unwrap();
        }
        let corpus_path = std::env::var("PROBE_CORPUS").unwrap_or_else(|_| "corpus/dev/patterns.jsonl".to_string());
        let file = std::fs::File::open(&corpus_path).expect("corpus");
        let reader = std::io::BufReader::new(file);
        let mut rows: Vec<(f64, String, usize, usize, f64)> = Vec::new();
        let mut acc = [(0.0f64, 0usize); 3];
        let t_all = Instant::now();
        for line in reader.lines() {
            let line = line.unwrap();
            let (name, pat): (String, Pattern) =
                ssi_scoring::pattern_from_jsonl_line(&line).expect("parse");
            if pat.nnz() < min_nnz || pat.n > max_n || name_filter.as_ref().map_or(false, |f| !name.contains(f.as_str())) {
                continue;
            }
            let spat = crate::ordering::score::scoring_pattern(&pat);
            let base = ssi_scoring::amd_baseline(&pat);
            let base_fl = crate::ordering::score::flops_of(&spat, &base).max(1);
            let t = Instant::now();
            let perm = crate::ordering::order(&pat);
            let ms = t.elapsed().as_secs_f64() * 1e3;
            assert!(crate::ordering::csc::is_bijection(&perm, pat.n), "{name}: not a bijection");
            let fl = crate::ordering::score::flops_of(&spat, &perm);
            let ratio = fl as f64 / base_fl as f64;
            let b = if pat.n < 1000 { 0 } else if pat.n < 10_000 { 1 } else { 2 };
            acc[b].0 += ratio.ln();
            acc[b].1 += 1;
            if let Some(f) = csv.as_mut() {
                writeln!(f, "{name},{},{},{ms:.1},{ratio:.5},{base_fl},{fl}", pat.n, pat.nnz()).unwrap();
            }
            eprintln!("{name:<30} n={:<7} nnz={:<8} {ms:>8.1} ms  ratio {ratio:.4}", pat.n, pat.nnz());
            rows.push((ms, name, pat.n, pat.nnz(), ratio));
        }
        rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        println!("\nslowest 15 order() calls:");
        for r in rows.iter().take(15) {
            println!("  {:>8.1} ms  n={:<8} nnz={:<8} ratio={:.4}  {}", r.0, r.2, r.3, r.4, r.1);
        }
        let w = [0.3, 0.3, 0.4];
        let mut score = 0.0;
        let mut wsum = 0.0;
        let names = ["lt_1k", "1k_10k", "gt_10k"];
        for b in 0..3 {
            if acc[b].1 > 0 {
                let g = (acc[b].0 / acc[b].1 as f64).exp();
                println!("  {:<7} count={:<4} geomean={g:.4}", names[b], acc[b].1);
                score += w[b] * g;
                wsum += w[b];
            }
        }
        println!("SCORE = {:.4}   (total wall {:.1} s)", score / wsum, t_all.elapsed().as_secs_f64());
    }
}
