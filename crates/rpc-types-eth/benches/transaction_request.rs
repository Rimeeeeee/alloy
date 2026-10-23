//! Compare streaming request decoding with an owned JSON buffer, including allocation traffic.

use alloy_consensus::TxEip8141;
use alloy_eips::eip8141::{Frame, TransactionFees};
use alloy_primitives::{Bytes, U256};
use alloy_rpc_types_eth::{TransactionInput, TransactionRequest};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Default)]
struct Counts {
    calls: usize,
    requested: usize,
    live: isize,
    peak: isize,
}

thread_local! {
    static COUNTS: Cell<Option<Counts>> = const { Cell::new(None) };
}

struct Allocator;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn record(allocated: usize, freed: usize, calls: usize) {
    let _ = COUNTS.try_with(|counter| {
        if let Some(mut counts) = counter.get() {
            counts.calls += calls;
            counts.requested += allocated;
            counts.live += allocated as isize - freed as isize;
            counts.peak = counts.peak.max(counts.live);
            counter.set(Some(counts));
        }
    });
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let result = unsafe { System.alloc(layout) };
        if !result.is_null() {
            record(layout.size(), 0, 1);
        }
        result
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let result = unsafe { System.alloc_zeroed(layout) };
        if !result.is_null() {
            record(layout.size(), 0, 1);
        }
        result
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            record(size, layout.size(), 1);
        }
        result
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record(0, layout.size(), 0);
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[derive(Clone, Copy, Debug)]
enum Decode {
    Streaming,
    Buffered,
}

impl Decode {
    fn request(self, json: &str) -> TransactionRequest {
        match self {
            Self::Streaming => serde_json::from_str(json).unwrap(),
            Self::Buffered => {
                // A same-version control for buffering the full input before typed decoding.
                // This is not a claim about performance of a different Git revision.
                let value: serde_json::Value = serde_json::from_str(json).unwrap();
                serde_json::from_value(value).unwrap()
            }
        }
    }
}

fn measure(kind: &str, size: usize, json: &str, decode: Decode, smoke: bool) {
    for _ in 0..8 {
        black_box(decode.request(black_box(json)));
    }
    let start = Instant::now();
    let mut iterations = 0_u64;
    loop {
        black_box(decode.request(black_box(json)));
        iterations += 1;
        if smoke || (iterations % 32 == 0 && start.elapsed() >= Duration::from_millis(250)) {
            break;
        }
    }
    let nanos = start.elapsed().as_nanos() as f64 / iterations as f64;
    const SAMPLES: usize = 10;
    COUNTS.with(|counter| counter.set(Some(Counts::default())));
    for _ in 0..SAMPLES {
        black_box(decode.request(black_box(json)));
    }
    let counts = COUNTS.with(|counter| counter.replace(None).unwrap());
    assert_eq!(counts.live, 0, "request allocations retained after drop");
    println!(
        "{kind}/{size}/{decode:?}: {nanos:.0} ns/op; {:.1} allocations/op; {:.1} requested B/op; \
         peak_live={} B; retained_after_drop={} B (calling thread only)",
        counts.calls as f64 / SAMPLES as f64,
        counts.requested as f64 / SAMPLES as f64,
        counts.peak,
        counts.live,
    );
}

fn main() {
    let smoke = std::env::args().any(|argument| argument == "--test");
    for size in [0, 4_096, 65_536] {
        let data = Bytes::from(vec![0xab; size]);
        let ordinary = TransactionRequest::default()
            .input(TransactionInput::new(data.clone()))
            .max_priority_fee_per_gas(1);
        let frame: TransactionRequest = TxEip8141 {
            frames: vec![Frame { data, ..Default::default() }],
            fees: TransactionFees { max_fee: U256::MAX, max_priority_fee_per_gas: U256::MAX },
            ..Default::default()
        }
        .into();
        for (kind, request) in [("ordinary", ordinary), ("frame", frame)] {
            let json = serde_json::to_string(&request).unwrap();
            for decode in [Decode::Streaming, Decode::Buffered] {
                assert_eq!(decode.request(&json), request);
                measure(kind, size, &json, decode, smoke);
            }
        }
    }
}
