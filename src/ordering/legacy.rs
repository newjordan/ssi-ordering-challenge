//! TEST-ONLY: the baseline `feral_amd::amd_order` ordering, with library-default
//! options — the exact ordering the grader anchors the score against. This is
//! what the rest of the portfolio `best_of`s against; the score function is
//! `flops(ours) / flops(this)`.
//!
//! The previous incarnation of this file held a FROZEN SNAPSHOT of a much
//! earlier portfolio, kept around only so a byte-identity test could prove
//! that a cost-reduction merge had not changed observable behavior. That
//! snapshot was deleted: keeping it required ~700 lines of carefully-
//! preserved orchestration that no longer serves the current code, and
//! nothing in the shipping pipeline (`order()`, the harness, the grader) ever
//! read it. The byte-identity test has been replaced by per-bucket score
//! measurements as the contract for "did this change the score?", which is
//! what the harness actually reports.
//!
//! What stays here: the *baseline* ordering itself, in the simplest possible
//! form. Kept as a separate, `#[cfg(test)]`-only module so a downstream
//! experiment can compare an arbitrary candidate against the anchor
//! (`legacy::legacy_order(pat)` is `feral_amd::amd_order` on the raw
//! full-symmetric pattern) without depending on the rest of the portfolio.
//! Never compiled into the shipped binary; the harness reads only `order()`
//! from `mod.rs` at runtime.
//!
//! Deterministic by construction: feral AMD with default options is pattern-
//! only and bit-stable across runs, and the two required `order()` calls
//! (the determinism gate) both call it on the same `core` and get the same
//! answer.
//!
//! Cost: O(nnz) per call, ms-scale on the whole dev corpus (it is the
//! baseline, so its cost is the same one the harness budgets in the
//! cap-safety headers of `mod.rs`).

#![cfg(test)]

use super::*;

/// Return the grader's baseline `feral_amd::amd_order` ordering. The returned
/// `Vec<usize>` is a valid bijection of `0..n`, deterministic across runs, and
/// identical (element-for-element) to the permutation the harness computes as
/// the AMD anchor when it scores `order(pat)`.
pub(crate) fn legacy_order(pattern: &Pattern) -> Vec<usize> {
    let n = pattern.n;
    if n == 0 {
        return Vec::new();
    }

    // i32-indexed borrowed pattern: the public signature every feral ordering
    // crate accepts. Cast is `expect`ed to fail loudly (matrix too large for
    // i32 indexing) — that would be a corpus-format bug, not a recoverable
    // runtime error.
    let col_ptr_i32: Vec<i32> = pattern
        .col_ptr
        .iter()
        .map(|&x| i32::try_from(x).expect("matrix too large for i32-indexed ordering"))
        .collect();
    let row_idx_i32: Vec<i32> = pattern
        .row_idx
        .iter()
        .map(|&x| i32::try_from(x).expect("matrix too large for i32-indexed ordering"))
        .collect();
    let core = feral_ordering_core::CscPattern::new(n, &col_ptr_i32, &row_idx_i32)
        .expect("malformed CscPattern (bug in Pattern invariants)");

    feral_amd::amd_order(&core)
        .expect("feral_amd::amd_order failed on a well-formed pattern")
        .into_iter()
        .map(|x| x as usize)
        .collect()
}
