//! Regression: an empty Path Item owns eight pointers, not eight inline Operations.
//!
//! Before the operations were boxed, a 2 MiB document of empty path items decoded
//! into about 1.5 GiB. This test measures the allocator while decoding a smaller
//! instance of the same shape and bounds the heap cost of each path entry.

use std::alloc::{GlobalAlloc, Layout, System};
use std::mem::size_of;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(pointer, layout);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// An empty path item must stay well below the 592-byte `Operation` it used to
/// embed eight times, so the bound below distinguishes boxed operations from
/// inline ones with a wide margin.
const MAX_BYTES_PER_PATH: usize = 1024;

#[test]
fn empty_path_items_keep_a_small_per_path_footprint() {
    const PATHS: usize = 10_000;

    let mut document = String::with_capacity(PATHS * 10 + 64);
    document.push_str(r#"{"openapi":"3.1.0","info":{"title":"x","version":"1"},"paths":{"#);
    for index in 0..PATHS {
        if index > 0 {
            document.push(',');
        }
        document.push_str(&format!(r#""/{index}":{{}}"#));
    }
    document.push_str("}}");

    // Warm the allocator path up before the measured window so lazily created
    // thread-local state is not charged to the document.
    drop(serde_json::from_str::<oas::OpenAPIV3>(&document).unwrap());

    let base = LIVE.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let decoded: oas::OpenAPIV3 = serde_json::from_str(&document).unwrap();
    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(base);

    let bytes_per_path = peak / PATHS;
    assert!(
        bytes_per_path <= MAX_BYTES_PER_PATH,
        "each empty path item used {bytes_per_path} heap bytes, above the {MAX_BYTES_PER_PATH}-byte bound; operations are probably no longer boxed"
    );

    // A cheap guard that also catches the inline regression in `debug_assert`
    // builds where the allocator margin may differ.
    assert!(
        size_of::<oas::PathItem>() <= 512,
        "PathItem is {} bytes; box the eight operation fields",
        size_of::<oas::PathItem>()
    );

    std::hint::black_box(&decoded);
}
