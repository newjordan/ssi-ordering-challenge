//! PREDICTOR–CORRECTOR elimination ordering, with graceful degradation.
//!
//! Motivated by a measurement rather than an analogy: on these KKT patterns
//! roughly **7% of the remaining graph is tied at the minimum deficiency at every
//! pivot** (15 of 211 on `powerflow0009r`, 108 of 1463 on `powerflow0057r`). So
//! minimum deficiency is not a decision rule — it narrows the field and then says
//! nothing, and every shipped method resolves the remainder by *vertex index*,
//! i.e. arbitrarily. That is what the relabelled-AMD multi-start exploits:
//! renumbering re-rolls an arbitrary choice at every pivot, moving the final
//! answer by up to 12 points.
//!
//! This supplies a rule for that choice, in the shape of Mehrotra's
//! predictor–corrector:
//!
//! * **predictor** — exact minimum deficiency, giving the tie set;
//! * **corrector** — among the tied, minimise the induced change in
//!   `Σ (deg+1)²` over the neighbourhood, i.e. the objective we are actually
//!   scored on.
//!
//! The corrector must target the squared cost, not fill. Within a tie set
//! `deficiency(v)` is constant by construction and
//! `Σ_{u∈N(v)} Δdeg(u) = 2·deficiency(v) − deg(v)`, so any *fill*-based induced
//! measure collapses to a monotone function of `deg(v)` and adds nothing — a
//! degeneracy confirmed empirically on power-flow, where two structurally
//! different fill-based correctors gave byte-identical orderings.
//!
//! ## Why the predictor is exact, and affordable anyway
//!
//! An earlier revision approximated the predictor by a shortlist of the smallest
//! -degree vertices. That was wrong and measurably so: the top-32 by degree
//! contains the true minimum-deficiency vertex only 62–80% of the time, and the
//! resulting orderings lost the very wins the rule exists to capture
//! (`camcns` 73 268 → 81 628, `color_lab3_4x0` 192 590 → 231 262).
//!
//! Exactness is kept without the `O(n·d²)` rescan, because eliminating `v` only
//! changes the deficiency of `N(v)` and *their* neighbours. Those are marked
//! dirty and re-evaluated lazily when popped from a binary heap, using an
//! optimistic key so a vertex whose deficiency *fell* is never missed.
//!
//! ## Degradation, not abandonment
//!
//! An earlier revision returned `None` the moment the work budget or a dense
//! pivot was hit, which discarded the whole ordering over a tail the rule cannot
//! afford — it declined 29 of 41 instances at n = 6k–20k. Cost is concentrated
//! at the *end* of the elimination, where the remaining graph has filled in and
//! is close to a clique; that is also precisely where the choice of pivot stops
//! mattering. So the run now steps down through three phases instead of bailing:
//!
//! | phase | rule | entered when |
//! |---|---|---|
//! | A | exact deficiency + corrector | start |
//! | B | AMD on the filled remainder | `work_soft` spent, or pivot degree > `dense_pivot_deg` |
//!
//! The tail must be AMD, not something cheaper. Degrading to plain minimum
//! degree was measured on `families-big`: the 12 fully-exact orderings scored
//! geomean **0.9203** against AMD while the 29 degraded ones scored **1.0347** —
//! the fallback gave back more than the exact prefix won. Completing with AMD
//! instead bounds the tail at the baseline it is scored against.
//!
//! Deterministic: no wall-clock, no entropy, ties resolved by degree then index,
//! so both required `order()` runs agree byte-for-byte.

use super::Pattern;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Tunable budgets. `Default` is the shipped configuration; the probe overrides
/// them to measure where each bound actually binds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PredcorrParams {
    /// Adjacency touches after which the exact rule (phase A) steps down to
    /// minimum degree (phase B).
    pub work_soft: u64,
    /// A pivot wider than this ends phase A: `O(d²)` elimination plus `O(d·d̄²)`
    /// re-evaluation is no longer worth the exactness.
    pub dense_pivot_deg: usize,
    /// At most this many tied candidates are corrected. The tie set can be ~7%
    /// of the live graph; the correction is taken over the lowest-degree
    /// members, which is where the corrector's own tie-break would look anyway.
    ///
    /// NOTE: the lazy heap can hold several entries for one vertex, so this cap
    /// counts ENTRIES, not distinct vertices. Deduplicating looks like a bug fix
    /// and measurably is not: on `families-big` it moved the score 0.976500 ->
    /// 0.980066 and the worst time 0.158 s -> 0.565 s, because a wider distinct
    /// tie set spends the work budget faster and shortens the exact phase.
    /// Widening the corrector is a `work_soft` tuning question, not a free win.
    pub tie_cap: usize,
}

impl Default for PredcorrParams {
    fn default() -> Self {
        Self {
            work_soft: 200_000_000,
            dense_pivot_deg: 512,
            tie_cap: 32,
        }
    }
}

/// What the run actually did — so the caller can tell an exact ordering from one
/// that spent most of its length in the degree dump.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PredcorrStats {
    /// Pivots chosen by the exact predictor + corrector.
    pub exact_pivots: usize,
    /// Vertices ordered by the AMD tail.
    pub amd_tail: usize,
    /// Vertices ordered by the degree dump (only if AMD itself failed).
    pub dumped: usize,
    /// Entries handed to the AMD tail — the tail's own cost driver.
    pub tail_nnz: usize,
    pub work: u64,
    pub max_pivot_deg: usize,
    /// Why phase A ended: 0 = never left (exact ordering), 1 = work_soft,
    /// 2 = dense pivot, 3 = skipped up front (initial scan unaffordable).
    pub exit_a: u8,
}

pub(crate) fn predcorr_order(pat: &Pattern) -> Option<Vec<usize>> {
    predcorr_order_stats(pat, &PredcorrParams::default()).map(|(o, _)| o)
}

/// Predictor–corrector ordering with degradation. Returns a full permutation for
/// every non-degenerate pattern; `None` only if the pattern itself is malformed.
pub(crate) fn predcorr_order_stats(
    pat: &Pattern,
    p: &PredcorrParams,
) -> Option<(Vec<usize>, PredcorrStats)> {
    let n = pat.n;
    let mut st = PredcorrStats::default();
    if n == 0 {
        return Some((Vec::new(), st));
    }

    let mut adj: Vec<Vec<u32>> = (0..n)
        .map(|v| {
            pat.row_idx[pat.col_ptr[v]..pat.col_ptr[v + 1]]
                .iter()
                .filter(|&&x| x != v)
                .map(|&x| x as u32)
                .collect()
        })
        .collect();
    let mut alive = vec![true; n];
    let mut dirty = vec![false; n];
    let mut def = vec![0u64; n];

    let mut stamp = vec![u32::MAX; n];
    let mut epoch: u32 = 0;
    let mut work: u64 = 0;

    // Exact deficiency of `v`, and the induced change in `Σ (deg+1)²` over `N(v)`.
    // One pass: mark `N(v)`, then for each `u ∈ N(v)` count `|N(u) ∩ N(v)|`, which
    // yields both the internal edge count and `u`'s degree change.
    macro_rules! evaluate {
        ($v:expr) => {{
            let vu: usize = $v;
            let d = adj[vu].len() as i64;
            epoch = epoch.wrapping_add(1);
            for &u in &adj[vu] {
                stamp[u as usize] = epoch;
            }
            let mut internal: u64 = 0;
            let mut induced: i64 = 0;
            for &u in &adj[vu] {
                let uu = u as usize;
                let mut common: i64 = 0;
                for &w in &adj[uu] {
                    if stamp[w as usize] == epoch {
                        common += 1;
                    }
                }
                work += adj[uu].len() as u64;
                internal += common as u64;
                let old = adj[uu].len() as i64 + 1;
                let new = old - 1 + (d - 1 - common);
                induced += new * new - old * old;
            }
            let dd = d as u64;
            (dd * dd.saturating_sub(1) / 2 - internal / 2, induced)
        }};
    }

    // Eliminate `v`: N(v) becomes a clique. Returns the pivot degree.
    macro_rules! eliminate {
        ($v:expr) => {{
            let v: usize = $v;
            let nb: Vec<u32> = std::mem::take(&mut adj[v]);
            for &u in &nb {
                let uu = u as usize;
                epoch = epoch.wrapping_add(1);
                for &w in &adj[uu] {
                    stamp[w as usize] = epoch;
                }
                adj[uu].retain(|&x| x != v as u32);
                for &w in &nb {
                    if w != u && stamp[w as usize] != epoch {
                        adj[uu].push(w);
                        stamp[w as usize] = epoch;
                    }
                }
                work += (adj[uu].len() + nb.len()) as u64;
            }
            alive[v] = false;
            if nb.len() > st.max_pivot_deg {
                st.max_pivot_deg = nb.len();
            }
            nb
        }};
    }

    let mut order: Vec<usize> = Vec::with_capacity(n);

    // ── PHASE A: exact predictor + corrector ──────────────────────────────────
    // The initial exact scan costs Σ_u deg(u)², which is computable in O(n). If
    // that alone would blow the soft budget, skip phase A rather than spend it.
    let init_cost: u64 = adj
        .iter()
        .map(|a| (a.len() as u64) * (a.len() as u64))
        .sum();
    let mut heap: BinaryHeap<Reverse<(u64, u32, u32)>> = BinaryHeap::new();

    if init_cost > p.work_soft {
        st.exit_a = 3;
    } else {
        heap.reserve(2 * n);
        for v in 0..n {
            let (dfc, _) = evaluate!(v);
            def[v] = dfc;
            heap.push(Reverse((dfc, adj[v].len() as u32, v as u32)));
        }

        let mut ties: Vec<(u32, u32)> = Vec::with_capacity(p.tie_cap);
        let mut stash: Vec<Reverse<(u64, u32, u32)>> = Vec::with_capacity(p.tie_cap);

        while order.len() < n {
            if work > p.work_soft {
                st.exit_a = 1;
                break;
            }
            // predictor: pop the true minimum deficiency, refreshing stale keys
            let mut dmin: Option<u64> = None;
            ties.clear();
            stash.clear();
            loop {
                let Some(&Reverse((key, _, v))) = heap.peek() else {
                    break;
                };
                let vu = v as usize;
                if !alive[vu] {
                    heap.pop();
                    continue;
                }
                if dirty[vu] {
                    heap.pop();
                    let (dfc, _) = evaluate!(vu);
                    def[vu] = dfc;
                    dirty[vu] = false;
                    heap.push(Reverse((dfc, adj[vu].len() as u32, v)));
                    continue;
                }
                if key != def[vu] {
                    heap.pop();
                    continue;
                }
                dmin = Some(key);
                break;
            }
            let Some(dmin) = dmin else { break };

            // collect the tie set at `dmin` (bounded)
            while ties.len() < p.tie_cap {
                let Some(&Reverse((key, dg, v))) = heap.peek() else {
                    break;
                };
                if key != dmin {
                    break;
                }
                let vu = v as usize;
                heap.pop();
                if !alive[vu] || dirty[vu] || key != def[vu] {
                    if alive[vu] && dirty[vu] {
                        let (dfc, _) = evaluate!(vu);
                        def[vu] = dfc;
                        dirty[vu] = false;
                        heap.push(Reverse((dfc, adj[vu].len() as u32, v)));
                    }
                    continue;
                }
                ties.push((dg, v));
                stash.push(Reverse((key, dg, v)));
            }
            for e in stash.drain(..) {
                heap.push(e);
            }
            if ties.is_empty() {
                break;
            }

            // corrector: least induced damage to Σ (deg+1)², then degree, index
            ties.sort_unstable();
            let chosen = if ties.len() == 1 {
                ties[0].1 as usize
            } else {
                let mut best: Option<(i64, u32, u32)> = None;
                for &(dg, v) in &ties {
                    let (_, induced) = evaluate!(v as usize);
                    let key = (induced, dg, v);
                    if best.is_none() || key < best.unwrap() {
                        best = Some(key);
                    }
                }
                best?.2 as usize
            };

            // A pivot this wide ends the exact phase rather than the whole run.
            if adj[chosen].len() > p.dense_pivot_deg {
                st.exit_a = 2;
                break;
            }

            let nb = eliminate!(chosen);
            order.push(chosen);
            st.exact_pivots += 1;

            // Deficiency changes only within distance 2 of the pivot. Mark those
            // dirty and re-admit them with an optimistic key, so a vertex whose
            // deficiency FELL is still popped early enough to be seen.
            for &u in &nb {
                let uu = u as usize;
                if alive[uu] && !dirty[uu] {
                    dirty[uu] = true;
                    heap.push(Reverse((0, adj[uu].len() as u32, u)));
                }
                for &w in &adj[uu] {
                    let wu = w as usize;
                    if alive[wu] && !dirty[wu] {
                        dirty[wu] = true;
                        heap.push(Reverse((0, adj[wu].len() as u32, w)));
                    }
                }
                work += adj[uu].len() as u64;
            }
        }
    }

    // ── PHASE B: hand the remainder to AMD ───────────────────────────────────
    // Degrading to plain minimum degree was measurably the wrong tail: on
    // `families-big` the fully-exact orderings scored geomean 0.9203 against
    // AMD while the degraded ones scored 1.0347, i.e. the fallback gave back
    // more than the exact prefix won. AMD is the baseline we are scored
    // against, so completing with AMD on the filled remainder makes the tail
    // cost ~nothing and keeps the prefix's gain.
    if order.len() < n {
        let mut new_id = vec![u32::MAX; n];
        let mut old_id: Vec<u32> = Vec::with_capacity(n - order.len());
        for v in 0..n {
            if alive[v] {
                new_id[v] = old_id.len() as u32;
                old_id.push(v as u32);
            }
        }
        let m = old_id.len();
        let mut scp: Vec<i32> = Vec::with_capacity(m + 1);
        let mut sri: Vec<i32> = Vec::with_capacity(m * 8);
        scp.push(0);
        // Fill edges are appended, so each list is unsorted; AMD rejects a
        // pattern whose column indices are not ascending.
        let mut col: Vec<i32> = Vec::new();
        for &v in &old_id {
            col.clear();
            // include the diagonal, matching the input convention
            col.push(new_id[v as usize] as i32);
            for &w in &adj[v as usize] {
                col.push(new_id[w as usize] as i32);
            }
            col.sort_unstable();
            sri.extend_from_slice(&col);
            scp.push(sri.len() as i32);
        }
        st.amd_tail = m;
        st.tail_nnz = sri.len();

        let sub = feral_ordering_core::CscPattern::new(m, &scp, &sri)
            .and_then(|core| feral_amd::amd_order(&core).ok());
        match sub {
            Some(p_sub) => {
                for x in p_sub {
                    let v = old_id[x as usize] as usize;
                    alive[v] = false;
                    order.push(v);
                }
            }
            None => {
                // last resort: degree order, so a malformed remainder still
                // yields a valid permutation rather than no ordering at all
                let mut rest: Vec<(u32, u32)> = old_id
                    .iter()
                    .map(|&v| (adj[v as usize].len() as u32, v))
                    .collect();
                rest.sort_unstable();
                st.dumped = rest.len();
                for (_, v) in rest {
                    alive[v as usize] = false;
                    order.push(v as usize);
                }
            }
        }
    }

    st.work = work;
    if order.len() != n {
        return None;
    }
    Some((order, st))
}
