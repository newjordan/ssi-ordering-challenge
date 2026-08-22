//! Windowed re-elimination local search ("WRE").
//!
//! Fact: the fill created by eliminating a SET of vertices is independent of
//! the order within the set. Hence re-ordering the vertices at positions
//! [t0, t1) of an ordering changes ONLY the column counts of those positions:
//! columns before t0 see the same later-set, columns after t1 see the same
//! eliminated-set. So each window is an independent local problem:
//!
//!   minimise Σ_{a ∈ W} c_a²  over orders of W, where the game is played on the
//!   elimination graph H after eliminating [0, t0), restricted to W ∪ S_W
//!   (S_W = later vertices H-adjacent to W), eliminating only W.
//!
//! H-adjacency of a ∈ W is computed exactly from the etree + column structures
//! of the incumbent: A(a) = N_G(a) ∩ [t0,n) ∪ ⋃_C N(C) ∩ [t0,n) over the
//! connected components C of G[prefix] touching a; N(C) ∩ (later) is exactly
//! Struct(L_q) for the etree root q of C, and q is found by climbing the etree
//! from a's prefix neighbours until the parent leaves the prefix.
//!
//! Windows are ranked by their current weight Σc², re-optimised by seeded
//! greedy re-elimination (exact local cost), in parallel; improvements on
//! disjoint windows are accepted together, and the result is confirmed with the
//! trusted scorer before it replaces the incumbent.

use super::csc::Csc;
use super::rng::SplitMix64;
use super::score;
use feral::sparse::csc::CscPattern as ScoringPattern;

const NONE: u32 = u32::MAX;

/// Symbolic data of an ordering: positions, permuted graph, etree, structures.
#[allow(dead_code)]
pub struct Symb {
    pub n: usize,
    /// perm[k] = vertex at position k
    pub perm: Vec<u32>,
    /// gp = graph relabelled so vertex i == position i
    pub gp: Csc,
    pub parent: Vec<u32>,
    /// Struct(L_j) as positions > j (unsorted), CSR: sptr[j]..sptr[j+1]
    pub sptr: Vec<usize>,
    pub sidx: Vec<u32>,
    /// column counts c_j (= |Struct(L_j)| + 1)
    pub cc: Vec<u32>,
}

fn etree(gp: &Csc) -> Vec<u32> {
    let n = gp.n;
    let mut parent = vec![NONE; n];
    let mut ancestor = vec![NONE; n];
    for k in 0..n {
        for &r in gp.col(k) {
            let mut i = r as usize;
            if i >= k {
                continue;
            }
            loop {
                let inext = ancestor[i];
                ancestor[i] = k as u32;
                if inext == NONE {
                    parent[i] = k as u32;
                    break;
                }
                i = inext as usize;
                if i >= k {
                    break;
                }
            }
        }
    }
    parent
}

/// Build symbolic data; returns None if nnz(L) exceeds `max_nnz_l`.
pub fn build(g: &Csc, perm: &[usize], cc: &[usize], max_nnz_l: usize) -> Option<Symb> {
    let n = g.n;
    let total: usize = cc.iter().map(|&c| c.saturating_sub(1)).sum();
    if total > max_nnz_l {
        return None;
    }
    let mut pos = vec![0u32; n];
    for (k, &v) in perm.iter().enumerate() {
        pos[v] = k as u32;
    }
    let gp = g.relabel(&pos);
    let parent = etree(&gp);
    // children lists
    let mut child_head = vec![NONE; n];
    let mut child_next = vec![NONE; n];
    for j in (0..n).rev() {
        let p = parent[j];
        if p != NONE {
            child_next[j] = child_head[p as usize];
            child_head[p as usize] = j as u32;
        }
    }
    let mut sptr = vec![0usize; n + 1];
    for j in 0..n {
        sptr[j + 1] = sptr[j] + cc[j].saturating_sub(1);
    }
    let mut sidx = vec![0u32; sptr[n]];
    let mut mark = vec![NONE; n];
    let mut fill = vec![0usize; n];
    for j in 0..n {
        let base = sptr[j];
        let mut len = 0usize;
        // higher original neighbours
        for &r in gp.col(j) {
            let r = r as usize;
            if r > j && mark[r] != j as u32 {
                mark[r] = j as u32;
                if base + len < sptr[j + 1] {
                    sidx[base + len] = r as u32;
                }
                len += 1;
            }
        }
        // children structures minus j
        let mut c = child_head[j];
        while c != NONE {
            let cu = c as usize;
            for t in sptr[cu]..sptr[cu] + fill[cu] {
                let r = sidx[t] as usize;
                if r != j && mark[r] != j as u32 {
                    mark[r] = j as u32;
                    if base + len < sptr[j + 1] {
                        sidx[base + len] = r as u32;
                    }
                    len += 1;
                }
            }
            c = child_next[cu];
        }
        debug_assert_eq!(len, cc[j].saturating_sub(1), "structure length mismatch at {j}");
        fill[j] = len.min(sptr[j + 1] - base);
    }
    Some(Symb {
        n,
        perm: perm.iter().map(|&v| v as u32).collect(),
        gp,
        parent,
        sptr,
        sidx,
        cc: cc.iter().map(|&c| c as u32).collect(),
    })
}

/// A window's local elimination instance.
#[allow(dead_code)]
pub struct Window {
    pub t0: usize,
    pub w: usize,
    /// number of local vertices (W first, then S)
    pub m: usize,
    pub words: usize,
    /// initial adjacency bitsets for the w window vertices (over m locals)
    pub adj: Vec<u64>,
    /// current Σ c² of the window under the incumbent order
    pub cur_cost: u64,
    /// average initial number of window-neighbours per window vertex
    pub dens: f64,
}

/// Extract the local instance for positions [t0, t0+w).
pub fn extract(s: &Symb, t0: usize, w: usize, scratch: &mut Scratch) -> Window {
    let t1 = t0 + w;
    scratch.stamp += 1;
    let st = scratch.stamp;
    // local ids: window positions 0..w ; S vertices appended
    let mut locals: Vec<u32> = Vec::with_capacity(w * 8);
    for a in t0..t1 {
        scratch.lid[a] = (a - t0) as u32;
        scratch.lstamp[a] = st;
        locals.push(a as u32);
    }
    // collect A(a) for each a as lists of positions (dedupe via per-row marks)
    let mut rows: Vec<Vec<u32>> = vec![Vec::new(); w];
    for a in t0..t1 {
        scratch.rstamp_tick += 1;
        let rs = scratch.rstamp_tick;
        let row = &mut rows[a - t0];
        for &r in s.gp.col(a) {
            let r = r as usize;
            if r >= t0 {
                if scratch.rmark[r] != rs {
                    scratch.rmark[r] = rs;
                    row.push(r as u32);
                }
            } else {
                // boundary of r w.r.t. t0 (memoised per window via bstamp)
                let q = boundary(s, r, t0, scratch, st);
                for t in s.sptr[q]..s.sptr[q + 1] {
                    let b = s.sidx[t] as usize;
                    if b >= t0 && b != a && scratch.rmark[b] != rs {
                        scratch.rmark[b] = rs;
                        row.push(b as u32);
                    }
                }
            }
        }
        // remove a itself if present (it is not: no self loops; boundary structs exclude q only)
        row.retain(|&b| b as usize != a);
    }
    // assign local ids to S vertices
    for row in &rows {
        for &b in row {
            let b = b as usize;
            if scratch.lstamp[b] != st {
                scratch.lstamp[b] = st;
                scratch.lid[b] = locals.len() as u32;
                locals.push(b as u32);
            }
        }
    }
    let m = locals.len();
    let words = (m + 63) / 64;
    let mut adj = vec![0u64; w * words];
    for (i, row) in rows.iter().enumerate() {
        for &b in row {
            let l = scratch.lid[b as usize] as usize;
            adj[i * words + (l >> 6)] |= 1u64 << (l & 63);
        }
    }
    let wwords = (w + 63) / 64;
    let mut wdeg_sum = 0usize;
    for i in 0..w {
        for k in 0..wwords.min(words) {
            let mut x = adj[i * words + k];
            if k == wwords - 1 && w % 64 != 0 {
                x &= (1u64 << (w % 64)) - 1;
            }
            wdeg_sum += x.count_ones() as usize;
        }
    }
    let dens = wdeg_sum as f64 / w as f64;
    let mut win = Window { t0, w, m, words, adj, cur_cost: 0, dens };
    // current cost: simulate the incumbent order (0..w)
    let ident: Vec<u32> = (0..w as u32).collect();
    win.cur_cost = simulate(&win, &ident, &mut scratch.sim);
    win
}

fn boundary(s: &Symb, r: usize, t0: usize, scratch: &mut Scratch, st: u32) -> usize {
    // climb from r while parent < t0; memoise
    let mut path: Vec<usize> = Vec::new();
    let mut p = r;
    let q = loop {
        if scratch.bstamp[p] == st {
            break scratch.bval[p] as usize;
        }
        path.push(p);
        let par = s.parent[p];
        if par == NONE || (par as usize) >= t0 {
            break p;
        }
        p = par as usize;
    };
    for &x in &path {
        scratch.bstamp[x] = st;
        scratch.bval[x] = q as u32;
    }
    q
}

/// Per-thread scratch (sized n).
pub struct Scratch {
    pub stamp: u32,
    pub lid: Vec<u32>,
    pub lstamp: Vec<u32>,
    pub rmark: Vec<u32>,
    pub rstamp_tick: u32,
    pub bstamp: Vec<u32>,
    pub bval: Vec<u32>,
    pub sim: Vec<u64>,
}

impl Scratch {
    pub fn new(n: usize) -> Scratch {
        Scratch {
            stamp: 0,
            lid: vec![0; n],
            lstamp: vec![0; n],
            rmark: vec![0; n],
            rstamp_tick: 0,
            bstamp: vec![0; n],
            bval: vec![0; n],
            sim: Vec::new(),
        }
    }
}

/// Σ c² for eliminating the window vertices in `order` (local ids 0..w).
pub fn simulate(win: &Window, order: &[u32], buf: &mut Vec<u64>) -> u64 {
    let w = win.w;
    let words = win.words;
    buf.clear();
    buf.extend_from_slice(&win.adj);
    let mut cost = 0u64;
    for &a in order {
        let a = a as usize;
        let (ca, _) = split_row(buf, a, words);
        let d: u64 = ca.iter().map(|x| x.count_ones() as u64).sum();
        cost += (d + 1) * (d + 1);
        // clique: for b in cur(a) ∩ W: cur(b) |= cur(a), clear b and a bits
        let row_a: Vec<u64> = ca.to_vec();
        let wbits_words = (w + 63) / 64;
        for wi in 0..wbits_words.min(words) {
            let mut x = row_a[wi];
            while x != 0 {
                let t = x.trailing_zeros() as usize;
                x &= x - 1;
                let b = wi * 64 + t;
                if b >= w || b == a {
                    continue;
                }
                let rb = &mut buf[b * words..(b + 1) * words];
                for k in 0..words {
                    rb[k] |= row_a[k];
                }
                rb[b >> 6] &= !(1u64 << (b & 63));
                rb[a >> 6] &= !(1u64 << (a & 63));
            }
        }
        // a eliminated: clear its bit from everyone (only W rows matter) — done
        // lazily above for neighbours; non-neighbours never had it.
    }
    cost
}

#[inline]
fn split_row(buf: &mut [u64], a: usize, words: usize) -> (&[u64], ()) {
    (&buf[a * words..(a + 1) * words], ())
}

/// Greedy re-elimination with seeded tie-breaking. Metric 0: exact current
/// degree (maintained incrementally). Metric 1: degree·4 + W-fill, where the
/// W-fill of `a` (new pairs created for window neighbours) is recomputed lazily
/// only for the window-neighbours of the vertex just eliminated — an
/// approximation of the exact metric that keeps each step cheap; the cost of
/// the resulting order is still computed exactly.
pub fn greedy(win: &Window, seed: u64, metric: u8, buf: &mut Vec<u64>) -> (Vec<u32>, u64) {
    let w = win.w;
    let words = win.words;
    buf.clear();
    buf.extend_from_slice(&win.adj);
    let mut rng = SplitMix64::new(seed);
    let mut alive: Vec<bool> = vec![true; w];
    let mut deg: Vec<u32> = (0..w)
        .map(|a| buf[a * words..(a + 1) * words].iter().map(|x| x.count_ones()).sum())
        .collect();
    let wwords = (w + 63) / 64;
    let fill_of = |buf: &[u64], a: usize, alive: &[bool]| -> u64 {
        let ra = &buf[a * words..(a + 1) * words];
        let mut f = 0u64;
        for wi in 0..wwords.min(words) {
            let mut x = ra[wi];
            while x != 0 {
                let t = x.trailing_zeros() as usize;
                x &= x - 1;
                let b = wi * 64 + t;
                if b >= w || b == a || !alive[b] {
                    continue;
                }
                let rb = &buf[b * words..(b + 1) * words];
                let mut cnt = 0u64;
                for k in 0..words {
                    cnt += (ra[k] & !rb[k]).count_ones() as u64;
                }
                f += cnt.saturating_sub(1);
            }
        }
        f
    };
    let mut val: Vec<u64> = if metric == 0 {
        deg.iter().map(|&d| d as u64).collect()
    } else {
        (0..w).map(|a| (deg[a] as u64) * 4 + fill_of(buf, a, &alive)).collect()
    };
    let mut order: Vec<u32> = Vec::with_capacity(w);
    let mut cost = 0u64;
    let mut tmp: Vec<u64> = vec![0; words];
    let mut nbrs: Vec<usize> = Vec::with_capacity(w);
    for _ in 0..w {
        let mut best = u64::MAX;
        let mut pick = usize::MAX;
        let mut ties = 0usize;
        for a in 0..w {
            if !alive[a] {
                continue;
            }
            let v = val[a];
            if v < best {
                best = v;
                pick = a;
                ties = 1;
            } else if v == best {
                ties += 1;
                if rng.below(ties) == 0 {
                    pick = a;
                }
            }
        }
        let a = pick;
        alive[a] = false;
        order.push(a as u32);
        tmp.copy_from_slice(&buf[a * words..(a + 1) * words]);
        let d = deg[a] as u64;
        cost += (d + 1) * (d + 1);
        // window neighbours of a
        nbrs.clear();
        for wi in 0..wwords.min(words) {
            let mut x = tmp[wi];
            while x != 0 {
                let t = x.trailing_zeros() as usize;
                x &= x - 1;
                let b = wi * 64 + t;
                if b < w && b != a && alive[b] {
                    nbrs.push(b);
                }
            }
        }
        for &b in &nbrs {
            let rb = &mut buf[b * words..(b + 1) * words];
            for k in 0..words {
                rb[k] |= tmp[k];
            }
            rb[b >> 6] &= !(1u64 << (b & 63));
            rb[a >> 6] &= !(1u64 << (a & 63));
            deg[b] = rb.iter().map(|x| x.count_ones()).sum();
        }
        // every other alive vertex adjacent to a loses a from its row? Rows of
        // non-neighbours never contained a. Non-window locals have no rows.
        match metric {
            0 => {
                for &b in &nbrs {
                    val[b] = deg[b] as u64;
                }
            }
            1 => {
                for &b in &nbrs {
                    val[b] = (deg[b] as u64) * 4 + fill_of(buf, b, &alive);
                }
            }
            2 => {
                for b in 0..w {
                    if alive[b] {
                        val[b] = (deg[b] as u64) * 4 + fill_of(buf, b, &alive);
                    }
                }
            }
            _ => {
                // 2-hop: neighbours and their window neighbours
                let mut touched = vec![false; w];
                for &b in &nbrs {
                    touched[b] = true;
                    let rb = &buf[b * words..(b + 1) * words];
                    for wi in 0..wwords.min(words) {
                        let mut x = rb[wi];
                        while x != 0 {
                            let t = x.trailing_zeros() as usize;
                            x &= x - 1;
                            let c = wi * 64 + t;
                            if c < w && alive[c] {
                                touched[c] = true;
                            }
                        }
                    }
                }
                for b in 0..w {
                    if touched[b] && alive[b] {
                        val[b] = (deg[b] as u64) * 4 + fill_of(buf, b, &alive);
                    }
                }
            }
        }
    }
    (order, cost)
}

/// Plan for one improvement pass.
pub struct LsPlan {
    pub win_sizes: Vec<usize>,
    pub max_windows: usize,
    pub restarts: usize,
    pub fill_restarts: usize,
    /// total work budget in cost units (see `est_units`), all threads together
    pub unit_budget: f64,
    /// cap on total extracted adjacency load (Σ over windows of 2·Σc + w²)
    pub extract_cap: usize,
}

/// Estimated work (cost units ≈ inner-loop word operations) for one window
/// with `nd` degree restarts and `nf` fill restarts.
pub fn est_units(win: &Window, nd: usize, nf: usize) -> f64 {
    let w = win.w as f64;
    let words = win.words as f64;
    let dens = win.dens.max(1.0);
    // elimination makes the window denser; effective neighbourhood size
    let deff = (2.0 * dens + 8.0).min(w);
    let deg_r = w * (w + deff * words);
    // 2-hop lazy fill: per step ≈ (touched ≈ deff·min(deff,w)/2) × fill_of (≈ deff·words);
    // measured ≈ 0.14 ns per unit (median) on the reference machine with the 1/3 factor.
    let fill_r = w * (deff * deff.min(w) * 0.5) * (deff * words) / 3.0;
    (nd as f64) * deg_r + (nf as f64) * fill_r + w * words * 2.0
}

/// One pass of windowed re-elimination over the incumbent. Returns an
/// improved (perm, flops) if found (confirmed with the trusted scorer).
pub fn improve_pass(
    g: &Csc,
    spat: &ScoringPattern,
    perm: &[usize],
    flops: u64,
    plan: &LsPlan,
    threads: usize,
    seed: u64,
    max_nnz_l: usize,
    start: std::time::Instant,
    deadline_ms: u64,
    offset_div: usize,
) -> Option<(Vec<usize>, u64)> {
    let n = g.n;
    let debug = std::env::var_os("SSI_ORDER_DEBUG").is_some();
    let t_start = std::time::Instant::now();
    let cc = score::column_counts_of(spat, perm);
    let nnz_l: usize = cc.iter().map(|&c| c.saturating_sub(1)).sum();
    let s = build(g, perm, &cc, max_nnz_l)?;
    let t_build = t_start.elapsed().as_secs_f64() * 1e3;
    // candidate windows: all sizes, tiled from the end (heaviest first later)
    let mut cands: Vec<(u64, usize, usize)> = Vec::new(); // (weight, t0, w)
    for &w in &plan.win_sizes {
        if w >= n || w < 4 {
            continue;
        }
        // tile from the end, optionally shifted by w/offset_div (second pass)
        let shift = if offset_div > 0 { w / offset_div } else { 0 };
        let mut t1 = n.saturating_sub(shift);
        while t1 >= w {
            let t0 = t1 - w;
            let weight: u64 = (t0..t1).map(|j| (s.cc[j] as u64) * (s.cc[j] as u64)).sum();
            cands.push((weight, t0, w));
            if t0 == 0 {
                break;
            }
            t1 = t0;
        }
    }
    // choose the heaviest, non-overlapping windows (greedy by weight)
    cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut taken = vec![false; n];
    let mut chosen: Vec<(usize, usize)> = Vec::new();
    for &(_, t0, w) in &cands {
        if chosen.len() >= plan.max_windows {
            break;
        }
        if (t0..t0 + w).any(|p| taken[p]) {
            continue;
        }
        for p in t0..t0 + w {
            taken[p] = true;
        }
        chosen.push((t0, w));
    }
    if chosen.is_empty() {
        return None;
    }
    // Phase A: extract the chosen windows (parallel), in weight order, with a
    // deterministic cap on the total extracted adjacency (window vertices ×
    // local vertices) — extraction cost is otherwise unbounded on patterns with
    // very large prefix-root structures.
    // Serial, deterministic decision of which windows get extracted: charge an
    // upper bound of each window's load (2·Σc + w², from the column counts) in
    // weight order and stop at the cap.
    let mut take = vec![false; chosen.len()];
    {
        let mut acc = 0usize;
        for (i, &(t0, w)) in chosen.iter().enumerate() {
            let est: usize = (t0..t0 + w).map(|j| s.cc[j] as usize).sum::<usize>() * 2 + w * w;
            if acc + est > plan.extract_cap {
                break;
            }
            acc += est;
            take[i] = true;
        }
    }
    let mut slots: Vec<Option<Window>> = (0..chosen.len()).map(|_| None).collect();
    {
        let next = std::sync::atomic::AtomicUsize::new(0);
        let slot_mutexes: Vec<std::sync::Mutex<Option<Window>>> =
            (0..chosen.len()).map(|_| std::sync::Mutex::new(None)).collect();
        let t_use = threads.max(1).min(chosen.len());
        std::thread::scope(|sc| {
            for _ in 0..t_use {
                let s = &s;
                let chosen = &chosen;
                let take = &take;
                let next = &next;
                let slot_mutexes = &slot_mutexes;
                sc.spawn(move || {
                    let mut scratch = Scratch::new(n);
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= chosen.len() {
                            break;
                        }
                        if !take[i] {
                            continue;
                        }
                        let (t0, w) = chosen[i];
                        let win = extract(s, t0, w, &mut scratch);
                        *slot_mutexes[i].lock().unwrap() = Some(win);
                    }
                });
            }
        });
        for (i, m) in slot_mutexes.into_iter().enumerate() {
            slots[i] = m.into_inner().unwrap();
        }
    }
    // Phase B: deterministic restart allocation within the unit budget, in
    // weight order (chosen is already sorted heaviest first).
    let mut alloc: Vec<(usize, usize)> = Vec::with_capacity(chosen.len()); // (nd, nf)
    let mut used = 0.0f64;
    for win in slots.iter() {
        let Some(win) = win else { alloc.push((0, 0)); continue };
        let mut nd = plan.restarts;
        let mut nf = plan.fill_restarts;
        // shrink fill restarts, then degree restarts, until it fits
        loop {
            let u = est_units(win, nd, nf);
            if used + u <= plan.unit_budget || (nd == 0 && nf == 0) {
                if nd + nf > 0 {
                    used += u;
                }
                break;
            }
            if nf > 0 {
                nf -= 1;
            } else if nd > 1 {
                nd -= 1;
            } else {
                nd = 0;
            }
        }
        alloc.push((nd, nf));
    }
    // Phase C: run restarts (parallel).
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<(usize, usize, Vec<u32>, u64, u64)>> =
        std::sync::Mutex::new(Vec::new());
    let win_stats: std::sync::Mutex<Vec<(usize, usize, f64)>> = std::sync::Mutex::new(Vec::new());
    let t_use = threads.max(1).min(chosen.len());
    std::thread::scope(|sc| {
        for _ in 0..t_use {
            let chosen = &chosen;
            let slots = &slots;
            let alloc = &alloc;
            let next = &next;
            let results = &results;
            let win_stats = &win_stats;
            sc.spawn(move || {
                let mut buf: Vec<u64> = Vec::new();
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= chosen.len() {
                        break;
                    }
                    let (t0, w) = chosen[i];
                    let (nd, nf) = alloc[i];
                    if nd + nf == 0 {
                        continue;
                    }
                    // Last-resort guard (see SOFT_DEADLINE_MS): never start a
                    // window past the deadline.
                    if start.elapsed().as_millis() as u64 > deadline_ms {
                        continue;
                    }
                    let Some(win) = slots[i].as_ref() else { continue };
                    let tw = std::time::Instant::now();
                    let mut best_cost = win.cur_cost;
                    let mut best_order: Option<Vec<u32>> = None;
                    let mut best_r = usize::MAX;
                    for r in 0..nd + nf {
                        let metric: u8 = if r >= nd { 3 } else { 0 };
                        let sd = super::rng::derive_seed(11, seed ^ ((t0 as u64) << 20) ^ (r as u64), n, w);
                        let (ord, cost) = greedy(win, sd, metric, &mut buf);
                        if cost < best_cost {
                            best_cost = cost;
                            best_order = Some(ord);
                            best_r = r;
                        }
                    }
                    if debug {
                        let ms = tw.elapsed().as_secs_f64() * 1e3;
                        win_stats.lock().unwrap().push((w, win.m, ms));
                        eprintln!(
                            "  [win] w={w} m={} dens={:.1} nd={nd} nf={nf} units={:.0} ms={ms:.2} gain={} ({:.3}%) best_r={best_r}",
                            win.m, win.dens, est_units(win, nd, nf), win.cur_cost - best_cost,
                            100.0 * (win.cur_cost - best_cost) as f64 / win.cur_cost.max(1) as f64
                        );
                    }
                    if let Some(ord) = best_order {
                        results.lock().unwrap().push((t0, w, ord, win.cur_cost, best_cost));
                    }
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    if debug {
        let wstats = win_stats.lock().unwrap();
        let mmax = wstats.iter().map(|x| x.1).max().unwrap_or(0);
        let mavg = if wstats.is_empty() { 0 } else { wstats.iter().map(|x| x.1).sum::<usize>() / wstats.len() };
        let tsum: f64 = wstats.iter().map(|x| x.2).sum();
        let tmax: f64 = wstats.iter().map(|x| x.2).fold(0.0, f64::max);
        eprintln!(
            "  [ls-stats] n={n} nnzL={nnz_l} build={t_build:.1}ms windows={} m_avg={mavg} m_max={mmax} win_ms_sum={tsum:.1} win_ms_max={tmax:.1} total={:.1}ms improved={}",
            chosen.len(), t_start.elapsed().as_secs_f64() * 1e3, results.len()
        );
    }
    if results.is_empty() {
        return None;
    }
    results.sort_by_key(|r| r.0);
    // apply all improvements
    let mut new_perm: Vec<usize> = perm.to_vec();
    for (t0, w, ord, _, _) in &results {
        let old: Vec<usize> = perm[*t0..*t0 + *w].to_vec();
        for (k, &l) in ord.iter().enumerate() {
            new_perm[*t0 + k] = old[l as usize];
        }
    }
    let f = score::flops_of(spat, &new_perm);
    if f < flops {
        Some((new_perm, f))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pattern;

    fn grid(rows: usize, cols: usize) -> Pattern {
        let mut edges = Vec::new();
        let id = |r: usize, c: usize| r * cols + c;
        for r in 0..rows {
            for c in 0..cols {
                if c + 1 < cols {
                    edges.push((id(r, c), id(r, c + 1)));
                }
                if r + 1 < rows {
                    edges.push((id(r, c), id(r + 1, c)));
                }
            }
        }
        Pattern::from_edges(rows * cols, &edges)
    }

    /// The local simulation must reproduce the trusted column counts exactly.
    #[test]
    fn window_simulation_matches_trusted_counts() {
        let pat = grid(30, 31);
        let g = Csc::from_pattern(&pat);
        let spat = score::scoring_pattern(&pat);
        let n = pat.n;
        let perm: Vec<usize> =
            feral_amd::amd_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
        let cc = score::column_counts_of(&spat, &perm);
        let s = build(&g, &perm, &cc, usize::MAX).unwrap();
        let mut scratch = Scratch::new(n);
        for &(t0, w) in &[(0usize, 16usize), (100, 32), (500, 64), (n - 40, 40), (n - 200, 128)] {
            let win = extract(&s, t0, w, &mut scratch);
            let expected: u64 = (t0..t0 + w).map(|j| (cc[j] as u64) * (cc[j] as u64)).sum();
            assert_eq!(win.cur_cost, expected, "window [{t0},{}) cost mismatch", t0 + w);
        }
    }

    /// Applying a greedy re-order must give exactly the simulated cost when
    /// re-scored by the trusted path (the rest of the ordering is unaffected).
    #[test]
    fn window_reorder_is_exact_and_local() {
        let pat = grid(25, 27);
        let g = Csc::from_pattern(&pat);
        let spat = score::scoring_pattern(&pat);
        let n = pat.n;
        let perm: Vec<usize> =
            feral_amd::amd_order(&g.view()).unwrap().into_iter().map(|x| x as usize).collect();
        let cc = score::column_counts_of(&spat, &perm);
        let s = build(&g, &perm, &cc, usize::MAX).unwrap();
        let mut scratch = Scratch::new(n);
        let mut buf = Vec::new();
        for &(t0, w) in &[(n - 64, 64usize), (300, 48), (50, 32)] {
            let win = extract(&s, t0, w, &mut scratch);
            let (ord, cost) = greedy(&win, 5, 1, &mut buf);
            let mut np = perm.clone();
            let old: Vec<usize> = perm[t0..t0 + w].to_vec();
            for (k, &l) in ord.iter().enumerate() {
                np[t0 + k] = old[l as usize];
            }
            let cc2 = score::column_counts_of(&spat, &np);
            let win_cost: u64 = (t0..t0 + w).map(|j| (cc2[j] as u64) * (cc2[j] as u64)).sum();
            assert_eq!(win_cost, cost, "simulated vs trusted window cost");
            for j in 0..n {
                if j < t0 || j >= t0 + w {
                    assert_eq!(cc[j], cc2[j], "column {j} outside window changed");
                }
            }
        }
    }
}
