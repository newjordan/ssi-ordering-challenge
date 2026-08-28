//! Bounded, OUTPUT-IDENTICAL parallelism for the candidate portfolio.
//!
//! The graded worker runs on a 4-vCPU GitHub `ubuntu-latest` runner and the
//! rules permit threads (`RLIMIT_NPROC = 4096`), but the whole portfolio in
//! [`super::order_with_options`] was single-threaded. Almost every candidate
//! in it is a PURE function of the pattern — `AMF(α)`, the metric-sweep
//! variants, METIS/Scotch/KaHIP, RCM/Sloan/ND, the strip transforms, and
//! every relabelled-AMD / -AMF / -no-dense-AMD restart — so they can all be
//! GENERATED and SCORED concurrently. What must stay sequential is the
//! ACCEPTANCE decision, and this module is built so that it does.
//!
//! ## Why the output is byte-identical, not merely "usually the same"
//!
//! The sequential `consider` closure was
//!
//! ```ignore
//! if f < best_flops { best_flops = f; best_perm = perm; }
//! ```
//!
//! applied to candidates in a FIXED source order. `best_flops` is therefore a
//! running minimum and the final `best_perm` is exactly *the earliest
//! candidate, in source order, attaining the strict minimum over
//! `{anchor} ∪ {candidates}`* — a quantity that does not depend on the order
//! in which the candidates were computed. [`run_candidates`] computes the
//! `(flops, perm)` pairs in whatever order the threads finish, then
//! [`super::order_with_options`] picks that earliest strict argmin by walking
//! the result vector in the ORIGINAL index order. Thread completion order
//! decides nothing.
//!
//! Determinism of each individual candidate is unchanged: every producer is
//! the same pure closure it was before, no producer reads wall-clock time,
//! process addresses or `HashMap` iteration order, and the feral crates hold
//! no cross-call global state (their only `thread_local!`s are `#[cfg(test)]`
//! instrumentation counters).
//!
//! ## Memory
//!
//! A naive "collect every candidate permutation, then reduce" would hold up
//! to ~130 `Vec<usize>` of length `n` at once. Instead each worker consults a
//! shared monotone `AtomicU64` minimum and DROPS its permutation when its
//! flops already exceed that minimum: such a candidate can never be the final
//! argmin (the atomic only ever decreases, so it is an upper bound on the
//! final minimum), while any candidate whose flops equals the final minimum is
//! provably retained — at its own `fetch_min` the previous value was still
//! `>=` the final minimum, so the `f <= prev` retention test passed. Peak
//! extra memory is therefore a handful of permutations plus one
//! [`ScoreWorkspace`] per worker, which is why the thread count is hard-capped
//! at [`PAR_MAX_THREADS`] = 4 rather than scaled with `available_parallelism`.

use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Mutex;

use super::scoring_ws::ScoreWorkspace;
use super::{is_bijection, ScoringPattern};

/// Hard cap on worker threads. The grader is a 4-vCPU runner, so more threads
/// than this can only add scheduling and memory cost. Deliberately NOT
/// `available_parallelism()`: this box has 20 cores, the grader has 4, and a
/// thread count that varies with the host would make local timing
/// measurements meaningless for the environment that actually enforces the
/// 2 s cap. (It cannot change the OUTPUT — see the module doc — only the
/// wall-clock.)
pub(crate) const PAR_MAX_THREADS: usize = 4;

/// Minimum estimated batch work (`tasks x nnz`) before spawning threads.
/// Every candidate family here costs roughly `c * nnz` for a small structural
/// constant, so `len * nnz` is the natural work proxy; below this the batch
/// runs in well under a millisecond and the ~30-100 µs of thread spawn/join
/// would dominate. Output-neutral, purely a cost switch — and it is a pure
/// function of the pattern and the queue length, so it does not compromise
/// determinism. Sized from the measured floor: the corpus's cheapest matrices
/// that still showed a real parallel win (`wastepaper6`, n=191, nnz=1416,
/// ~200 queued relabel restarts) are ~14x above it, while a 30-vertex matrix
/// with the same restart count is ~10x below.
pub(crate) const PAR_MIN_WORK: usize = 20_000;

/// A deferred candidate producer. Same signature the sequential `consider`
/// closure took, boxed so the portfolio can collect all of them before any
/// runs. `Sync` (not `Send`) is what the workers need: they only ever hold
/// `&CandFn`.
pub(crate) type CandFn<'a> =
    Box<dyn Fn() -> Result<Vec<i32>, feral_ordering_core::OrderingError> + Sync + 'a>;

/// A deferred candidate producer that already returns a `usize` permutation
/// (the post-hoc phases build these directly rather than through the feral
/// `i32` ordering API).
pub(crate) type PermFn<'a> = Box<dyn Fn() -> Option<Vec<usize>> + Sync + 'a>;

/// One candidate's outcome. `flops == None` means the producer panicked,
/// errored, or returned a non-bijection — exactly the cases the sequential
/// `consider` dropped on the floor. `perm == None` with `flops == Some(_)`
/// means the candidate scored, but was already known not to be the argmin and
/// its permutation was released (see the module doc's memory section).
#[derive(Default)]
pub(crate) struct CandOut {
    pub(crate) flops: Option<u64>,
    pub(crate) perm: Option<Vec<usize>>,
}

/// Generate + score `tasks` on up to [`PAR_MAX_THREADS`] threads.
///
/// `incumbent` is `best_flops` at phase entry; it seeds the shared minimum so
/// candidates that cannot beat the incumbent release their permutations
/// immediately. The returned vector is indexed by TASK INDEX, not by
/// completion order — that is what makes the caller's sequential acceptance
/// walk reproduce the original semantics exactly.
pub(crate) fn run_candidates(
    tasks: &[CandFn<'_>],
    sp: &ScoringPattern,
    n: usize,
    nnz: usize,
    incumbent: u64,
) -> Vec<CandOut> {
    run_generic(
        tasks.len(),
        sp,
        n,
        nnz,
        incumbent,
        false,
        0,
        &|i: usize| match std::panic::catch_unwind(AssertUnwindSafe(|| (tasks[i])())) {
            Ok(Ok(perm_i32)) => Some(perm_i32.into_iter().map(|x| x as usize).collect()),
            _ => None,
        },
    )
}

/// Generate and score the same candidate batch while retaining the raw top
/// `keep_top` candidates by deterministic `(flops, task_index)` rank. No new
/// candidate is generated or scored; this only keeps the few permutations a
/// downstream seed selector may consume. The monotone top-rank threshold means
/// a candidate dropped here can never enter the final top `keep_top`.
pub(crate) fn run_candidates_keep_top(
    tasks: &[CandFn<'_>],
    sp: &ScoringPattern,
    n: usize,
    nnz: usize,
    incumbent: u64,
    keep_top: usize,
) -> Vec<CandOut> {
    run_generic(
        tasks.len(),
        sp,
        n,
        nnz,
        incumbent,
        false,
        keep_top,
        &|i: usize| match std::panic::catch_unwind(AssertUnwindSafe(|| (tasks[i])())) {
            Ok(Ok(perm_i32)) => Some(perm_i32.into_iter().map(|x| x as usize).collect()),
            _ => None,
        },
    )
}

/// Same as [`run_candidates`] for producers that already speak `Vec<usize>`.
pub(crate) fn run_perms(
    tasks: &[PermFn<'_>],
    sp: &ScoringPattern,
    n: usize,
    nnz: usize,
    incumbent: u64,
) -> Vec<CandOut> {
    run_perms_inner(tasks, sp, n, nnz, incumbent, false)
}

/// [`run_perms`] with the permutation-release optimization DISABLED, so every
/// scored candidate keeps its permutation.
///
/// Required by the hill-climb phase: that phase does NOT take the batch's
/// argmin. Its sequential form re-perturbed the NEW incumbent the moment a
/// restart improved, so within a speculatively-computed wave only the FIRST
/// improving candidate may be accepted — and the first improver need not be
/// the wave's minimum, so the argmin-only retention invariant does not cover
/// it. Waves are at most [`PAR_MAX_THREADS`] long, so keeping all of them
/// costs at most 4 permutations.
pub(crate) fn run_perms_keep_all(
    tasks: &[PermFn<'_>],
    sp: &ScoringPattern,
    n: usize,
    nnz: usize,
    incumbent: u64,
) -> Vec<CandOut> {
    run_perms_inner(tasks, sp, n, nnz, incumbent, true)
}

fn run_perms_inner(
    tasks: &[PermFn<'_>],
    sp: &ScoringPattern,
    n: usize,
    nnz: usize,
    incumbent: u64,
    keep_all: bool,
) -> Vec<CandOut> {
    run_generic(
        tasks.len(),
        sp,
        n,
        nnz,
        incumbent,
        keep_all,
        0,
        &|i: usize| match std::panic::catch_unwind(AssertUnwindSafe(|| (tasks[i])())) {
            Ok(v) => v,
            Err(_) => None,
        },
    )
}

/// The shared engine: a lock-free `AtomicUsize` work queue over `len` tasks,
/// one lazily-created [`ScoreWorkspace`] per worker, per-worker result buffers
/// merged back into task-index order after the scope joins.
fn run_generic(
    len: usize,
    sp: &ScoringPattern,
    n: usize,
    nnz: usize,
    incumbent: u64,
    keep_all: bool,
    keep_top: usize,
    produce: &(dyn Fn(usize) -> Option<Vec<usize>> + Sync),
) -> Vec<CandOut> {
    let mut results: Vec<CandOut> = Vec::with_capacity(len);
    for _ in 0..len {
        results.push(CandOut::default());
    }
    if len == 0 {
        return results;
    }

    // Monotone shared upper bound on the final minimum. Only ever decreases.
    let gmin = AtomicU64::new(incumbent);
    let ranked_floor: Mutex<Vec<(u64, usize)>> = Mutex::new(Vec::new());

    let eval = |i: usize, ws: &mut Option<ScoreWorkspace>| -> CandOut {
        let Some(perm) = produce(i) else {
            return CandOut::default();
        };
        if !is_bijection(&perm, n) {
            return CandOut::default();
        }
        let w = ws.get_or_insert_with(|| ScoreWorkspace::new(n, nnz));
        let f = w.flops(sp, &perm);
        let prev = gmin.fetch_min(f, AtomicOrdering::Relaxed);
        let retained_rank = if keep_top == 0 {
            false
        } else {
            let key = (f, i);
            let mut top = ranked_floor.lock().unwrap_or_else(|e| e.into_inner());
            let pos = top.partition_point(|&prior| prior < key);
            if pos < keep_top {
                top.insert(pos, key);
                top.truncate(keep_top);
                true
            } else {
                false
            }
        };
        CandOut {
            flops: Some(f),
            // Retain only while this candidate can still be the argmin.
            perm: if keep_all || retained_rank || f <= prev {
                Some(perm)
            } else {
                None
            },
        }
    };
    // Nothing inside `eval` should be able to unwind (the producers are
    // already `catch_unwind`-wrapped by the callers above, and the scoring
    // path is trusted internal code), but a worker thread that DID unwind
    // would silently lose a whole batch of results, which is exactly the kind
    // of nondeterminism the rules forbid. Wrapping here makes a worker
    // panic-free by construction.
    let eval = |i: usize, ws: &mut Option<ScoreWorkspace>| -> CandOut {
        std::panic::catch_unwind(AssertUnwindSafe(|| eval(i, ws))).unwrap_or_default()
    };

    if len == 1 || len.saturating_mul(nnz.max(1)) < PAR_MIN_WORK || force_sequential() {
        let mut ws: Option<ScoreWorkspace> = None;
        for (i, slot) in results.iter_mut().enumerate() {
            *slot = eval(i, &mut ws);
        }
        return results;
    }

    let next = AtomicUsize::new(0);
    let nthreads = PAR_MAX_THREADS.min(len);
    let collected: Vec<Vec<(usize, CandOut)>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..nthreads)
            .map(|_| {
                s.spawn(|| {
                    let mut ws: Option<ScoreWorkspace> = None;
                    let mut out: Vec<(usize, CandOut)> = Vec::new();
                    loop {
                        let i = next.fetch_add(1, AtomicOrdering::Relaxed);
                        if i >= len {
                            break;
                        }
                        out.push((i, eval(i, &mut ws)));
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_default())
            .collect()
    });
    for part in collected {
        for (i, o) in part {
            results[i] = o;
        }
    }
    results
}

/// Sequential acceptance over [`run_generic`]'s task-indexed results,
/// reproducing the running-minimum semantics of the original `consider`
/// closure exactly: return the EARLIEST task index attaining the strict
/// minimum below `best_flops`, or `None` if no candidate beats the incumbent.
///
/// `on_flops` is invoked, in task-index order, for every candidate that
/// scored — that is where the `#[cfg(test)]` runner-up census hooks in.
pub(crate) fn accept<F: FnMut(u64)>(
    results: &mut [CandOut],
    best_flops: &mut u64,
    best_perm: &mut Vec<usize>,
    mut on_flops: F,
) -> bool {
    let mut win: Option<(usize, u64)> = None;
    for (i, r) in results.iter().enumerate() {
        let Some(f) = r.flops else { continue };
        on_flops(f);
        if f < *best_flops && win.is_none_or(|(_, bf)| f < bf) {
            win = Some((i, f));
        }
    }
    let Some((i, f)) = win else { return false };
    // The retention invariant (module doc) guarantees the winner kept its
    // permutation; `else` is unreachable, and declining is the safe response
    // (the incumbent is always a valid ordering).
    let Some(p) = results[i].perm.take() else {
        debug_assert!(false, "argmin candidate dropped its permutation");
        return false;
    };
    *best_flops = f;
    *best_perm = p;
    true
}

/// TEST-ONLY switch (never compiled into the shipped binary) forcing every
/// batch onto the sequential path, so `probe_parallel_identity` can run the
/// SAME `order()` both ways on the same process and compare the returned
/// permutations element-by-element across the whole corpus. Production builds
/// compile [`force_sequential`] down to a constant `false`.
#[cfg(test)]
thread_local! {
    pub(crate) static FORCE_SEQUENTIAL: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

#[cfg(test)]
#[inline]
fn force_sequential() -> bool {
    FORCE_SEQUENTIAL.with(|c| c.get())
}

#[cfg(not(test))]
#[inline(always)]
fn force_sequential() -> bool {
    false
}
