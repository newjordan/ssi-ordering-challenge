//! Final-stage fusion across disconnected graph components.
//!
//! For a structurally symmetric pattern, the trees of any elimination forest
//! are exactly the connected components: original edges never cross trees,
//! and parent links never leave an original component.  The scored objective
//! is additive across those components, so the AMD anchor and final incumbent
//! can be compared component by component and their better projections spliced
//! into one ordering.

pub(super) struct Components {
    of_vertex: Vec<usize>,
    sizes: Vec<usize>,
}

pub(super) struct Fused {
    pub(super) perm: Vec<usize>,
    pub(super) predicted_flops: u64,
}

impl Components {
    /// Build the original-vertex component map from the AMD-permuted
    /// elimination forest.  Two components of size at least three are a
    /// necessary condition for a strict fusion win; smaller connected graphs
    /// have order-invariant elimination cost.
    pub(super) fn from_amd_forest(
        amd_perm: &[usize],
        parent: &[Option<usize>],
    ) -> Option<Self> {
        let n = amd_perm.len();
        if n == 0 || parent.len() != n {
            return None;
        }
        let mut by_pos = vec![usize::MAX; n];
        let mut sizes = Vec::<usize>::new();
        for pos in (0..n).rev() {
            let component = match parent[pos] {
                Some(p) if p > pos && p < n => {
                    let c = by_pos[p];
                    if c == usize::MAX {
                        return None;
                    }
                    c
                }
                Some(_) => return None,
                None => {
                    sizes.push(0);
                    sizes.len() - 1
                }
            };
            by_pos[pos] = component;
            sizes[component] += 1;
        }
        if sizes.iter().filter(|&&size| size >= 3).count() < 2 {
            return None;
        }

        let mut of_vertex = vec![usize::MAX; n];
        for (pos, &vertex) in amd_perm.iter().enumerate() {
            if vertex >= n || of_vertex[vertex] != usize::MAX {
                return None;
            }
            of_vertex[vertex] = by_pos[pos];
        }
        Some(Self { of_vertex, sizes })
    }

    /// Choose the lower-cost projection (AMD anchor or final incumbent) in
    /// every component. `final_counts` is indexed by original vertex;
    /// `amd_counts` is indexed by AMD elimination position.
    pub(super) fn fuse(
        &self,
        amd_perm: &[usize],
        amd_counts: &[usize],
        final_perm: &[usize],
        final_counts: &[usize],
        final_flops: u64,
    ) -> Option<Fused> {
        let n = self.of_vertex.len();
        if amd_perm.len() != n
            || amd_counts.len() != n
            || final_perm.len() != n
            || final_counts.len() != n
        {
            return None;
        }
        let nc = self.sizes.len();
        let mut amd_local = vec![0u64; nc];
        let mut final_local = vec![0u64; nc];
        let square = |count: usize| -> Option<u64> {
            let c = u64::try_from(count).ok()?;
            c.checked_mul(c)
        };
        for (pos, &v) in amd_perm.iter().enumerate() {
            if v >= n {
                return None;
            }
            let c = self.of_vertex[v];
            amd_local[c] = amd_local[c].checked_add(square(amd_counts[pos])?)?;
        }
        let mut seen = vec![false; n];
        for &v in final_perm {
            if v >= n || seen[v] {
                return None;
            }
            seen[v] = true;
            let c = self.of_vertex[v];
            final_local[c] = final_local[c].checked_add(square(final_counts[v])?)?;
        }
        if final_local.iter().try_fold(0u64, |s, &x| s.checked_add(x))? != final_flops {
            return None;
        }

        let use_amd: Vec<bool> = (0..nc).map(|c| amd_local[c] < final_local[c]).collect();
        let predicted = (0..nc).try_fold(0u64, |s, c| {
            s.checked_add(amd_local[c].min(final_local[c]))
        })?;
        if predicted >= final_flops {
            return None;
        }

        let mut starts = vec![0usize; nc + 1];
        for c in 0..nc {
            starts[c + 1] = starts[c].checked_add(self.sizes[c])?;
        }
        if starts[nc] != n {
            return None;
        }
        let mut cursor = starts[..nc].to_vec();
        let mut perm = vec![usize::MAX; n];
        for &v in final_perm {
            let c = self.of_vertex[v];
            if !use_amd[c] {
                perm[cursor[c]] = v;
                cursor[c] += 1;
            }
        }
        for &v in amd_perm {
            let c = self.of_vertex[v];
            if use_amd[c] {
                perm[cursor[c]] = v;
                cursor[c] += 1;
            }
        }
        if (0..nc).any(|c| cursor[c] != starts[c + 1]) {
            return None;
        }
        Some(Fused { perm, predicted_flops: predicted })
    }
}

#[cfg(test)]
thread_local! {
    static DEFAULT_REACH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn mark_default_reach() {
    DEFAULT_REACH.with(|v| v.set(v.get() + 1));
}

#[cfg(test)]
pub(super) fn take_default_reach() -> usize {
    DEFAULT_REACH.with(|v| {
        let n = v.get();
        v.set(0);
        n
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn causal_fusion_is_deterministic_and_bijective() {
        // Two 3-vertex components. Anchor is worse on the first and better on
        // the second; final has the opposite profile (14+9 versus 9+14).
        let amd = [1, 0, 2, 3, 4, 5];
        let parent = [Some(1), Some(2), None, Some(4), Some(5), None];
        let components = Components::from_amd_forest(&amd, &parent).unwrap();
        let amd_counts = [3, 2, 1, 2, 2, 1];
        let final_perm = [0, 1, 2, 4, 3, 5];
        let final_counts = [2, 2, 1, 2, 3, 1];
        let a = components
            .fuse(&amd, &amd_counts, &final_perm, &final_counts, 23)
            .unwrap();
        let b = components
            .fuse(&amd, &amd_counts, &final_perm, &final_counts, 23)
            .unwrap();
        assert_eq!(a.predicted_flops, 18);
        assert_eq!(a.perm, b.perm);
        let mut sorted = a.perm.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..6).collect::<Vec<_>>());
    }

    #[test]
    fn malformed_and_non_opportunity_forests_fail_closed_without_panic() {
        let result = std::panic::catch_unwind(|| {
            assert!(Components::from_amd_forest(&[0, 1, 2], &[Some(7), None, None]).is_none());
            assert!(Components::from_amd_forest(&[0, 0, 2], &[Some(1), Some(2), None]).is_none());
            // One nontrivial component plus isolates cannot have opposing
            // component winners, so it does not pay for a final rescore.
            assert!(Components::from_amd_forest(
                &[0, 1, 2, 3, 4],
                &[Some(1), Some(2), None, None, None],
            )
            .is_none());
        });
        assert!(result.is_ok());
    }
}
