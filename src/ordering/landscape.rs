//! Ignored-by-default landscape measurement: for every dev matrix, run each
//! candidate ordering family once, record wall time and flop ratio vs AMD.
//! Writes CSV to $LANDSCAPE_OUT (default landscape.csv in cwd).
//!
//!   LANDSCAPE_OUT=/path/landscape.csv cargo test --release --offline --locked \
//!       landscape_all -- --ignored --nocapture --test-threads=1

#[cfg(test)]
mod landscape_tests {
    use crate::Pattern;
    use std::io::{BufRead, Write};
    use std::time::Instant;

    use feral::ordering::amd::permute_pattern;
    use feral::ordering::elimination_tree::EliminationTree;
    use feral::sparse::csc::CscPattern as ScoringPattern;
    use feral::symbolic::column_counts_gnp;

    fn flops_of(pat: &ScoringPattern, perm: &[usize]) -> u64 {
        let permuted = permute_pattern(pat, perm);
        let etree = EliminationTree::from_pattern(&permuted);
        let counts = column_counts_gnp(&permuted, &etree);
        counts.iter().map(|&c| (c as u64) * (c as u64)).sum()
    }

    fn to_usize(p: Vec<i32>) -> Vec<usize> {
        p.into_iter().map(|x| x as usize).collect()
    }

    #[test]
    #[ignore]
    fn landscape_all() {
        let out_path =
            std::env::var("LANDSCAPE_OUT").unwrap_or_else(|_| "landscape.csv".to_string());
        let mut out = std::fs::File::create(&out_path).expect("create csv");
        writeln!(
            out,
            "name,n,nnz,amd_ms,sym_ms,amf_ms,amf_r,amdm1_ms,amdm1_r,amfm1_ms,amfm1_r,amd4_ms,amd4_r,amdna_ms,amdna_r,metis_ms,metis_r,metis2_ms,metis2_r,scotch_ms,scotch_r,kahip_ms,kahip_r"
        )
        .unwrap();
        let file = std::fs::File::open("corpus/dev/patterns.jsonl").expect("corpus");
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            let (name, pat): (String, Pattern) =
                ssi_scoring::pattern_from_jsonl_line(&line).expect("parse");
            let n = pat.n;
            let nnz = pat.nnz();
            let col_ptr_i32: Vec<i32> = pat.col_ptr.iter().map(|&x| x as i32).collect();
            let row_idx_i32: Vec<i32> = pat.row_idx.iter().map(|&x| x as i32).collect();
            let core =
                feral_ordering_core::CscPattern::new(n, &col_ptr_i32, &row_idx_i32).unwrap();
            let spat = ScoringPattern {
                n,
                col_ptr: pat.col_ptr.clone(),
                row_idx: pat.row_idx.clone(),
            };
            let t = Instant::now();
            let amd = to_usize(feral_amd::amd_order(&core).unwrap());
            let amd_ms = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let amd_fl = flops_of(&spat, &amd).max(1);
            let sym_ms = t.elapsed().as_secs_f64() * 1e3;

            let mut cells: Vec<(f64, f64)> = Vec::new();
            let mut run = |enabled: bool,
                           f: &dyn Fn() -> Result<Vec<i32>, feral_ordering_core::OrderingError>| {
                if !enabled {
                    cells.push((-1.0, -1.0));
                    return;
                }
                let t = Instant::now();
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
                let ms = t.elapsed().as_secs_f64() * 1e3;
                let ratio = match r {
                    Ok(Ok(p)) => {
                        let p = to_usize(p);
                        if p.len() == n {
                            flops_of(&spat, &p) as f64 / amd_fl as f64
                        } else {
                            -2.0
                        }
                    }
                    _ => -3.0,
                };
                cells.push((ms, ratio));
            };
            // AMF
            run(true, &|| feral_amf::amf_order(&core));
            // AMD alpha -1
            run(true, &|| {
                feral_amd::amd_order_opts(
                    &core,
                    &feral_amd::AmdOptions { aggressive: true, dense_alpha: -1.0 },
                )
                .map(|(p, _)| p)
            });
            // AMF alpha -1
            run(true, &|| {
                feral_amf::amf_order_opts(&core, &feral_amf::AmfOptions { dense_alpha: -1.0 })
                    .map(|(p, _)| p)
            });
            // AMD alpha 4
            run(true, &|| {
                feral_amd::amd_order_opts(
                    &core,
                    &feral_amd::AmdOptions { aggressive: true, dense_alpha: 4.0 },
                )
                .map(|(p, _)| p)
            });
            // AMD non-aggressive alpha 10
            run(true, &|| {
                feral_amd::amd_order_opts(
                    &core,
                    &feral_amd::AmdOptions { aggressive: false, dense_alpha: 10.0 },
                )
                .map(|(p, _)| p)
            });
            // METIS default
            run(nnz < 2_000_000, &|| {
                feral_metis::metis_order_full(&core, &feral_metis::MetisOptions::default())
                    .map(|(p, _, _)| p)
            });
            // METIS seed 2
            run(nnz < 700_000, &|| {
                let o = feral_metis::MetisOptions { seed: 2, ..Default::default() };
                feral_metis::metis_order_full(&core, &o).map(|(p, _, _)| p)
            });
            // Scotch
            run(nnz < 700_000, &|| feral_scotch::scotch_order(&core));
            // KaHIP fast
            run(nnz < 300_000, &|| feral_kahip::kahip_order(&core));

            let mut row = format!("{name},{n},{nnz},{amd_ms:.2},{sym_ms:.2}");
            for (ms, r) in &cells {
                row.push_str(&format!(",{ms:.2},{r:.5}"));
            }
            writeln!(out, "{row}").unwrap();
            out.flush().unwrap();
            eprintln!("{row}");
        }
    }
}
