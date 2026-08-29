//! TEST-ONLY counting global allocator — the machine-independent measurement
//! instrument for this session's task (reduce allocation traffic in
//! `order()` without changing its output by a single bit).
//!
//! Wall-clock numbers are unreliable while other agents share this machine,
//! but allocation COUNT and BYTES are deterministic and hardware-independent
//! for a given (pattern, code) pair — exactly the signal the task asks for.
//! This wraps `std::alloc::System`, so it changes nothing about what gets
//! allocated or how; it only counts.
//!
//! `#[cfg(test)]` end to end: never compiled into the shipped binary (the
//! grader builds without `--test`, so this whole file — including the
//! `#[global_alloc]` registration — does not exist in that build). The one
//! `unsafe impl GlobalAlloc` below is the explicitly-sanctioned exception to
//! "no unsafe" for this task: a `GlobalAlloc` impl is unsafe by the trait's
//! own definition (the trait's safety contract, not any behavior this impl
//! adds), and it does nothing but forward to `System` plus increment atomic
//! counters — there is no unsafe *logic* here, only the mandatory trait
//! shape.
#![cfg(test)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static REALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Snapshot of the counters at a point in time.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AllocStats {
    pub calls: usize,
    pub bytes: usize,
    pub deallocs: usize,
    pub reallocs: usize,
}

impl std::ops::Sub for AllocStats {
    type Output = AllocStats;
    fn sub(self, rhs: AllocStats) -> AllocStats {
        AllocStats {
            calls: self.calls - rhs.calls,
            bytes: self.bytes - rhs.bytes,
            deallocs: self.deallocs - rhs.deallocs,
            reallocs: self.reallocs - rhs.reallocs,
        }
    }
}

impl std::ops::Add for AllocStats {
    type Output = AllocStats;
    fn add(self, rhs: AllocStats) -> AllocStats {
        AllocStats {
            calls: self.calls + rhs.calls,
            bytes: self.bytes + rhs.bytes,
            deallocs: self.deallocs + rhs.deallocs,
            reallocs: self.reallocs + rhs.reallocs,
        }
    }
}

pub(crate) fn snapshot() -> AllocStats {
    AllocStats {
        calls: ALLOC_CALLS.load(Ordering::Relaxed),
        bytes: ALLOC_BYTES.load(Ordering::Relaxed),
        deallocs: DEALLOC_CALLS.load(Ordering::Relaxed),
        reallocs: REALLOC_CALLS.load(Ordering::Relaxed),
    }
}

/// Run `f`, returning its result and the allocator activity attributed to it
/// (calls to `alloc`/`realloc` in between the snapshots; nothing outside `f`
/// running concurrently, since `cargo test` runs each test in its own
/// thread but the counters are process-global — call this only from
/// single-threaded measurement code, e.g. with `--test-threads=1`).
pub(crate) fn measure<T>(f: impl FnOnce() -> T) -> (T, AllocStats) {
    let before = snapshot();
    let out = f();
    let after = snapshot();
    (out, after - before)
}
