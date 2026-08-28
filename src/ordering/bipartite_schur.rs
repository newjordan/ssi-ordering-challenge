//! Final exact-bipartite Schur-side siblings.
//!
//! For a connected bipartite graph `G = (U, V, E)`, all vertices in either
//! side are mutually non-adjacent. Eliminating an entire side first therefore
//! has a fixed first-stage cost and creates the corresponding Schur graph on
//! the other side, without making two still-live first-side vertices adjacent.
//! This yields two structural siblings that no degree tie-break can express:
//! stable-filter the final incumbent into `U || V` and into `V || U`.
//!
//! This module only builds candidates. The caller remains responsible for the
//! bijection check and the trusted exact-score, strict-improvement admission.

use crate::Pattern;

#[cfg(test)]
thread_local! {
    pub(crate) static BIPARTITE_SCHUR_CALLS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// Return the stable side-0-first and side-1-first siblings. The outer
/// `Option` is `None` unless the input is a connected, non-empty bipartite
/// pattern with a valid seed permutation. Each inner direction is admitted
/// independently by its first-side `sum choose(degree, 2)` work bound.
pub(crate) fn stable_schur_siblings(
    pattern: &Pattern,
    seed: &[usize],
    pair_work_budget: u64,
) -> Option<[Option<Vec<usize>>; 2]> {
    #[cfg(test)]
    BIPARTITE_SCHUR_CALLS.with(|calls| calls.set(calls.get() + 1));

    let n = pattern.n;
    if n < 2
        || pattern.row_idx.is_empty()
        || seed.len() != n
        || pattern.col_ptr.len() != n + 1
        || pattern.col_ptr[0] != 0
        || pattern.col_ptr[n] != pattern.row_idx.len()
        || pattern.col_ptr.windows(2).any(|w| w[0] > w[1])
    {
        return None;
    }

    let mut seen = vec![false; n];
    for &v in seed {
        if v >= n || seen[v] {
            return None;
        }
        seen[v] = true;
    }

    // Deterministic two-colouring from vertex zero. Reversing all colours
    // would only exchange the two returned directions, not their candidate set.
    let uncoloured = u8::MAX;
    let mut colour = vec![uncoloured; n];
    let mut stack = Vec::with_capacity(n);
    colour[0] = 0;
    stack.push(0usize);
    let mut visited = 0usize;
    let mut pair_work = [0u64; 2];
    let work_cap = pair_work_budget.saturating_add(1);

    while let Some(v) = stack.pop() {
        visited += 1;
        let lo = pattern.col_ptr[v];
        let hi = pattern.col_ptr[v + 1];
        let degree = u64::try_from(hi - lo).ok()?;
        let choose_two = degree
            .checked_mul(degree.saturating_sub(1))
            .map(|pairs| pairs / 2)
            .unwrap_or(work_cap);
        let side = colour[v] as usize;
        pair_work[side] = pair_work[side].saturating_add(choose_two).min(work_cap);

        for &u in &pattern.row_idx[lo..hi] {
            if u >= n || u == v {
                return None;
            }
            if colour[u] == uncoloured {
                colour[u] = 1 - colour[v];
                stack.push(u);
            } else if colour[u] == colour[v] {
                return None;
            }
        }
    }
    if visited != n {
        return None;
    }

    let build = |first: u8| {
        seed.iter()
            .copied()
            .filter(|&v| colour[v] == first)
            .chain(seed.iter().copied().filter(|&v| colour[v] != first))
            .collect::<Vec<_>>()
    };

    Some([
        (pair_work[0] <= pair_work_budget).then(|| build(0)),
        (pair_work[1] <= pair_work_budget).then(|| build(1)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{flops_of, is_bijection, order, ScoringPattern};

    fn trusted_flops(pattern: &Pattern, perm: &[usize]) -> u64 {
        let scoring = ScoringPattern {
            n: pattern.n,
            col_ptr: pattern.col_ptr.clone(),
            row_idx: pattern.row_idx.clone(),
        };
        flops_of(&scoring, perm)
    }

    fn k23() -> Pattern {
        Pattern::from_edges(
            5,
            &[(0, 2), (0, 3), (0, 4), (1, 2), (1, 3), (1, 4)],
        )
    }

    #[test]
    fn bipartite_schur_k23_has_exact_directional_scores() {
        let pattern = k23();
        let seed = vec![0, 1, 2, 3, 4];
        let [side0_first, side1_first] =
            stable_schur_siblings(&pattern, &seed, 250_000).unwrap();
        let side0_first = side0_first.unwrap();
        let side1_first = side1_first.unwrap();

        assert_eq!(side0_first, vec![0, 1, 2, 3, 4]);
        assert_eq!(side1_first, vec![2, 3, 4, 0, 1]);
        assert!(is_bijection(&side0_first, pattern.n));
        assert!(is_bijection(&side1_first, pattern.n));
        assert_eq!(trusted_flops(&pattern, &seed), 46);
        assert_eq!(trusted_flops(&pattern, &side0_first), 46);
        assert_eq!(trusted_flops(&pattern, &side1_first), 32);
        for i in 0..seed.len() - 1 {
            let mut adjacent = seed.clone();
            adjacent.swap(i, i + 1);
            assert_eq!(
                trusted_flops(&pattern, &adjacent),
                46,
                "adjacent swap {i} unexpectedly escaped the 46-cost plateau"
            );
        }
        assert_eq!(
            stable_schur_siblings(&pattern, &seed, 250_000),
            Some([Some(side0_first), Some(side1_first)])
        );

        let interleaved = vec![1, 3, 0, 4, 2];
        assert_eq!(
            stable_schur_siblings(&pattern, &interleaved, 250_000),
            Some([
                Some(vec![1, 0, 3, 4, 2]),
                Some(vec![3, 4, 2, 1, 0]),
            ])
        );
    }

    #[test]
    fn bipartite_schur_rejects_odd_cycle_disconnected_and_bad_seed() {
        let odd = Pattern::from_edges(3, &[(0, 1), (1, 2), (2, 0)]);
        assert!(stable_schur_siblings(&odd, &[0, 1, 2], 250_000).is_none());

        let disconnected = Pattern::from_edges(4, &[(0, 1), (2, 3)]);
        assert!(stable_schur_siblings(&disconnected, &[0, 1, 2, 3], 250_000).is_none());

        let pattern = k23();
        assert!(stable_schur_siblings(&pattern, &[0, 1, 2, 3, 3], 250_000).is_none());
    }

    #[test]
    fn bipartite_schur_applies_pair_work_per_direction() {
        // K1,5: center-first costs C(5,2)=10 pair work, while the five-leaf
        // direction costs zero. A budget of five must reject only the former.
        let pattern = Pattern::from_edges(6, &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]);
        let seed: Vec<usize> = (0..6).collect();
        let [center_first, leaves_first] =
            stable_schur_siblings(&pattern, &seed, 5).unwrap();
        assert!(center_first.is_none());
        assert_eq!(leaves_first, Some(vec![1, 2, 3, 4, 5, 0]));
    }

    #[test]
    fn bipartite_schur_is_reached_from_default_order() {
        // K2,23 is deliberately just outside the n<=24 exact-DP axis while
        // remaining far inside the shipped bipartite cost gates.
        let mut edges = Vec::new();
        for u in 0..2 {
            for v in 2..25 {
                edges.push((u, v));
            }
        }
        let pattern = Pattern::from_edges(25, &edges);
        assert!(pattern.n > super::super::exact_dp::STRUCTURAL_MAX_N);
        BIPARTITE_SCHUR_CALLS.with(|calls| calls.set(0));

        let perm = order(&pattern);

        BIPARTITE_SCHUR_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert!(is_bijection(&perm, pattern.n));
    }
}
