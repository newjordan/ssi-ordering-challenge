//! Exact local subset-DP descent over consecutive windows of an incumbent
//! elimination order. The caller applies a final exact global score check, so
//! a locally accepted rewrite can never regress the returned ordering.

use std::collections::{HashMap, HashSet};

#[inline]
fn spend(left: &mut i64, amount: usize) -> bool {
    *left -= amount as i64;
    *left >= 0
}
fn live_graph(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    left: &mut i64,
) -> Option<Vec<HashSet<u32>>> {
    if col_ptr.len() != n + 1 || !spend(left, n) {
        return None;
    }
    let mut graph = vec![HashSet::new(); n];
    for v in 0..n {
        let lo = col_ptr[v];
        let hi = col_ptr[v + 1];
        if lo > hi || hi > row_idx.len() || !spend(left, 2 * (hi - lo)) {
            return None;
        }
        for &u in &row_idx[lo..hi] {
            if u < n && u != v {
                graph[v].insert(u as u32);
                graph[u].insert(v as u32);
            }
        }
    }
    Some(graph)
}

fn eliminate(graph: &mut [HashSet<u32>], pivot: usize, left: &mut i64) -> bool {
    if pivot >= graph.len() {
        return false;
    }
    let neighbors: Vec<u32> = graph[pivot].iter().copied().collect();
    let degree = neighbors.len();
    if !spend(left, degree + degree.saturating_mul(degree) / 2) {
        return false;
    }
    for &u in &neighbors {
        graph[u as usize].remove(&(pivot as u32));
    }
    graph[pivot].clear();
    for a in 0..neighbors.len() {
        for b in a + 1..neighbors.len() {
            let u = neighbors[a] as usize;
            let v = neighbors[b] as usize;
            graph[u].insert(v as u32);
            graph[v].insert(u as u32);
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn sweep(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    perm: &mut [usize],
    k: usize,
    offset: usize,
    max_universe: usize,
    left: &mut i64,
    accepted: &mut usize,
) -> bool {
    let mut graph = match live_graph(n, col_ptr, row_idx, left) {
        Some(g) => g,
        None => return false,
    };
    for &v in perm.iter().take(offset.min(n)) {
        if !eliminate(&mut graph, v, left) {
            return false;
        }
    }

    let states = 1usize << k;
    let mut start = offset;
    while start + k <= n {
        let block = perm[start..start + k].to_vec();
        if block.iter().any(|&v| v >= n) {
            return false;
        }

        // Universe = block vertices plus their current live neighbors.
        let mut universe: Vec<u32> = block.iter().map(|&v| v as u32).collect();
        let mut degree_sum = 0usize;
        for &v in &block {
            degree_sum += graph[v].len();
            universe.extend(graph[v].iter().copied());
        }
        universe.sort_unstable();
        universe.dedup();
        if !spend(left, degree_sum + universe.len()) {
            return false;
        }
        if universe.len() > max_universe {
            for &v in &perm[start..start + k] {
                if !eliminate(&mut graph, v, left) {
                    return false;
                }
            }
            start += k;
            continue;
        }

        let words = universe.len().div_ceil(64);
        // Charge all work for a window before changing the incumbent.
        let estimate =
            k * words + k * k + k * k * words + states.saturating_mul(k).saturating_mul(2 * words);
        if !spend(left, estimate) {
            return false;
        }

        let local: HashMap<u32, usize> =
            universe.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        let mut adjacency = vec![vec![0u64; words]; k];
        let mut inside = vec![0u32; k];
        let mut block_positions = vec![0usize; k];
        for (j, &v) in block.iter().enumerate() {
            let Some(&pos) = local.get(&(v as u32)) else {
                return false;
            };
            block_positions[j] = pos;
            for &u in &graph[v] {
                let Some(&q) = local.get(&u) else {
                    return false;
                };
                adjacency[j][q / 64] |= 1u64 << (q % 64);
            }
        }
        for j in 0..k {
            for q in 0..k {
                if j != q && graph[block[j]].contains(&(block[q] as u32)) {
                    inside[j] |= 1u32 << q;
                }
            }
        }

        let components = |mask: u32| -> Vec<(u32, Vec<u64>)> {
            let mut result = Vec::new();
            let mut remaining = mask;
            while remaining != 0 {
                let first = remaining.trailing_zeros();
                let mut stack = 1u32 << first;
                let mut members = 0u32;
                let mut boundary = vec![0u64; words];
                while stack != 0 {
                    let j = stack.trailing_zeros() as usize;
                    let bit = 1u32 << j;
                    stack &= !bit;
                    if members & bit != 0 {
                        continue;
                    }
                    members |= bit;
                    for w in 0..words {
                        boundary[w] |= adjacency[j][w];
                    }
                    stack |= inside[j] & mask & !members;
                }
                remaining &= !members;
                result.push((members, boundary));
            }
            result
        };

        let pivot_cost = |done: u32, j: usize, comps: &[(u32, Vec<u64>)]| -> u64 {
            let mut reachable = adjacency[j].clone();
            for (members, boundary) in comps {
                if inside[j] & members != 0 {
                    for w in 0..words {
                        reachable[w] |= boundary[w];
                    }
                }
            }
            let mut dead = done | (1u32 << j);
            while dead != 0 {
                let q = dead.trailing_zeros() as usize;
                dead &= dead - 1;
                let pos = block_positions[q];
                reachable[pos / 64] &= !(1u64 << (pos % 64));
            }
            let degree: u64 = reachable.iter().map(|word| word.count_ones() as u64).sum();
            (degree + 1).saturating_mul(degree + 1)
        };

        let mut old_cost = 0u64;
        let mut done = 0u32;
        for j in 0..k {
            old_cost = old_cost.saturating_add(pivot_cost(done, j, &components(done)));
            done |= 1u32 << j;
        }

        let mut dp = vec![u64::MAX; states];
        let mut last = vec![0u8; states];
        dp[0] = 0;
        let mut choices: Vec<usize> = (0..k).collect();
        choices.sort_unstable_by_key(|&j| block[j]);
        for state in 0..states {
            if dp[state] == u64::MAX {
                continue;
            }
            let done = state as u32;
            let comps = components(done);
            for &j in &choices {
                if done & (1u32 << j) != 0 {
                    continue;
                }
                let next = state | (1usize << j);
                let cost = dp[state].saturating_add(pivot_cost(done, j, &comps));
                if cost < dp[next] {
                    dp[next] = cost;
                    last[next] = j as u8;
                }
            }
        }

        if dp[states - 1] < old_cost {
            let mut reversed = Vec::with_capacity(k);
            let mut state = states - 1;
            while state != 0 && reversed.len() < k {
                let j = last[state] as usize;
                if j >= k || state & (1usize << j) == 0 {
                    break;
                }
                reversed.push(block[j]);
                state &= !(1usize << j);
            }
            if state == 0 && reversed.len() == k {
                reversed.reverse();
                perm[start..start + k].copy_from_slice(&reversed);
                *accepted += 1;
            }
        }

        for &v in &perm[start..start + k] {
            if !eliminate(&mut graph, v, left) {
                return false;
            }
        }
        start += k;
    }
    true
}

pub(crate) fn descent(
    n: usize,
    col_ptr: &[usize],
    row_idx: &[usize],
    seed: &[usize],
    k: usize,
    max_universe: usize,
    budget: i64,
) -> Option<Vec<usize>> {
    let k = k.clamp(2, 16);
    if n < k || seed.len() != n || col_ptr.len() != n + 1 || max_universe == 0 || budget <= 0 {
        return None;
    }
    let mut seen = vec![false; n];
    for &v in seed {
        if v >= n || seen[v] {
            return None;
        }
        seen[v] = true;
    }

    let mut left = budget;
    let mut perm = seed.to_vec();
    let mut accepted = 0usize;
    for offset in [0usize, k / 2] {
        if !sweep(
            n,
            col_ptr,
            row_idx,
            &mut perm,
            k,
            offset,
            max_universe,
            &mut left,
            &mut accepted,
        ) {
            break;
        }
    }
    (accepted > 0).then_some(perm)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csc(n: usize, edges: &[(usize, usize)]) -> (Vec<usize>, Vec<usize>) {
        let mut adj = vec![Vec::new(); n];
        for &(u, v) in edges {
            assert!(u < n && v < n && u != v);
            adj[u].push(v);
            adj[v].push(u);
        }
        let mut col_ptr = Vec::with_capacity(n + 1);
        let mut row_idx = Vec::new();
        col_ptr.push(0);
        for row in &mut adj {
            row.sort_unstable();
            row.dedup();
            row_idx.extend(row.iter().copied());
            col_ptr.push(row_idx.len());
        }
        (col_ptr, row_idx)
    }

    fn exact_cost(n: usize, col_ptr: &[usize], row_idx: &[usize], perm: &[usize]) -> u64 {
        let mut left = i64::MAX;
        let mut graph = live_graph(n, col_ptr, row_idx, &mut left).expect("valid graph");
        let mut total = 0u64;
        for &v in perm {
            let degree = graph[v].len() as u64;
            total = total.saturating_add((degree + 1).saturating_mul(degree + 1));
            assert!(eliminate(&mut graph, v, &mut left));
        }
        total
    }

    fn brute_min_cost(
        n: usize,
        col_ptr: &[usize],
        row_idx: &[usize],
        prefix: &mut Vec<usize>,
        used: &mut [bool],
        best: &mut u64,
    ) {
        if prefix.len() == n {
            *best = (*best).min(exact_cost(n, col_ptr, row_idx, prefix));
            return;
        }
        for v in 0..n {
            if !used[v] {
                used[v] = true;
                prefix.push(v);
                brute_min_cost(n, col_ptr, row_idx, prefix, used, best);
                prefix.pop();
                used[v] = false;
            }
        }
    }

    #[test]
    fn one_window_matches_brute_force_oracle() {
        // Eliminating the star center first is deliberately suboptimal.
        let n = 5;
        let edges = [(0, 1), (0, 2), (0, 3), (0, 4)];
        let (col_ptr, row_idx) = csc(n, &edges);
        let seed: Vec<usize> = (0..n).collect();
        let got = descent(n, &col_ptr, &row_idx, &seed, n, n, 10_000)
            .expect("the oracle window must improve the center-first seed");
        let mut oracle = u64::MAX;
        brute_min_cost(
            n,
            &col_ptr,
            &row_idx,
            &mut Vec::new(),
            &mut vec![false; n],
            &mut oracle,
        );
        assert_eq!(exact_cost(n, &col_ptr, &row_idx, &got), oracle);
        assert!(exact_cost(n, &col_ptr, &row_idx, &got) < exact_cost(n, &col_ptr, &row_idx, &seed));
    }

    #[test]
    fn active_gate_shape_is_deterministic() {
        // n=1000 and nnz=14 are inside the production gate. The first K=8
        // window is a star with a bad center-first seed; the remainder is
        // isolated, making this a cheap deterministic active-path check.
        let n = 1_000;
        let edges: Vec<(usize, usize)> = (1..8).map(|v| (0, v)).collect();
        let (col_ptr, row_idx) = csc(n, &edges);
        let seed: Vec<usize> = (0..n).collect();
        let a = descent(n, &col_ptr, &row_idx, &seed, 8, 512, 32_000_000);
        let b = descent(n, &col_ptr, &row_idx, &seed, 8, 512, 32_000_000);
        assert!(a.is_some());
        assert_eq!(a, b);
    }

    #[test]
    fn budget_abort_before_and_after_acceptance_is_fail_closed() {
        let n = 5;
        let edges = [(0, 1), (0, 2), (0, 3), (0, 4)];
        let (col_ptr, row_idx) = csc(n, &edges);
        let seed: Vec<usize> = (0..n).collect();

        // 409 is the exact charge through the first window's DP. At 408 the
        // mutation is never made; at 409 it is accepted, then the following
        // graph-elimination charge exhausts the budget. The accepted prefix is
        // still a valid improving candidate and later global scoring is safe.
        assert_eq!(descent(n, &col_ptr, &row_idx, &seed, n, n, 408), None);
        let after = descent(n, &col_ptr, &row_idx, &seed, n, n, 409)
            .expect("accepted mutation must survive later budget exhaustion");
        assert_eq!(after.len(), n);
        assert!(
            exact_cost(n, &col_ptr, &row_idx, &after) < exact_cost(n, &col_ptr, &row_idx, &seed)
        );
    }
}
