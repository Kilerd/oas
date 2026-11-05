//! Regression: decoding a schema must not copy its subtree at every level.
//!
//! The derived deserializer buffered the whole schema object into Serde's
//! `Content` at each nesting level, so a leaf reached through `items` was copied
//! once per level. A document 1.8 MB large peaked at about 5.9 GB when a
//! 900k-entry `enum` sat 100 levels down. This test compares a shallow and a
//! deep document with the same leaf and bounds the deep/shallow heap ratio.

use std::alloc::{GlobalAlloc, Layout, System};
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

fn document(depth: usize, leaf: usize) -> serde_json::Value {
    let mut schema = serde_json::json!({
        "type": "integer",
        "enum": vec![0u64; leaf],
    });
    for _ in 0..depth {
        schema = serde_json::json!({"type": "array", "items": schema});
    }
    serde_json::json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {},
        "components": {"schemas": {"Deep": schema}},
    })
}

/// Decode `value` and return the peak heap allocated above the value's own
/// footprint. The value is passed in so its allocations are outside the window.
fn measure(value: serde_json::Value) -> (usize, oas::OpenAPIV3) {
    let base = LIVE.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let decoded = serde_json::from_value(value).expect("the document decodes");
    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(base);
    (peak, decoded)
}

#[test]
fn deep_schema_decode_stays_linear() {
    const LEAF: usize = 50_000;
    let expected = document(60, LEAF);
    // Warm up lazy allocator and deserializer state outside the measurements.
    drop(measure(document(2, LEAF)).1);

    let (shallow, _) = measure(document(2, LEAF));
    let (deep, decoded) = measure(expected.clone());
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        expected,
        "the deep document must still round-trip unchanged"
    );
    assert!(
        deep <= shallow.saturating_mul(2).max(4 << 20),
        "a schema 60 levels deep used {deep} bytes against {shallow} at depth 2; \
         the decoder is copying the leaf subtree once per level"
    );
}
