//! The trusted objective: predicted factorization flops Σⱼ cⱼ² via feral's own
//! pattern-pure symbolic building blocks — byte-identical to what the grader
//! ranks (`permute_pattern → EliminationTree → column_counts_gnp`).

use feral::ordering::amd::permute_pattern;
use feral::ordering::elimination_tree::EliminationTree;
use feral::sparse::csc::CscPattern as ScoringPattern;
use feral::symbolic::column_counts_gnp;

use crate::Pattern;

pub fn scoring_pattern(p: &Pattern) -> ScoringPattern {
    ScoringPattern {
        n: p.n,
        col_ptr: p.col_ptr.clone(),
        row_idx: p.row_idx.clone(),
    }
}

/// Σⱼ cⱼ² for `perm` on `pat` (exact quantity the grader computes).
pub fn flops_of(pat: &ScoringPattern, perm: &[usize]) -> u64 {
    let permuted = permute_pattern(pat, perm);
    let etree = EliminationTree::from_pattern(&permuted);
    let counts = column_counts_gnp(&permuted, &etree);
    counts.iter().map(|&c| (c as u64) * (c as u64)).sum()
}

/// Column counts of L for `perm` (same trusted path), useful for diagnostics
/// and refinement heuristics.
#[allow(dead_code)]
pub fn column_counts_of(pat: &ScoringPattern, perm: &[usize]) -> Vec<usize> {
    let permuted = permute_pattern(pat, perm);
    let etree = EliminationTree::from_pattern(&permuted);
    column_counts_gnp(&permuted, &etree)
}
