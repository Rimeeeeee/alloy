//! Codec benchmarks for Engine API execution payloads and payload envelopes.

use alloy_consensus::{Blob, Block, Bytes48, TxEnvelope};
use alloy_eips::{eip7594::CELLS_PER_EXT_BLOB, eip7685::Requests};
use alloy_primitives::{Bytes, U256};
use alloy_rpc_types_engine::{
    BlobsBundleV1, BlobsBundleV2, ExecutionPayloadEnvelopeV2, ExecutionPayloadEnvelopeV3,
    ExecutionPayloadEnvelopeV4, ExecutionPayloadEnvelopeV5, ExecutionPayloadEnvelopeV6,
    ExecutionPayloadFieldV2, ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3,
    ExecutionPayloadV4,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

const BLOCKS_AND_BALS: &str = include_str!("../testdata/payload/reth-blocks-bals-10.json");
const SAMPLE_TIME: Duration = Duration::from_secs(1);

fn main() {
    let responses: Vec<serde_json::Value> = serde_json::from_str(BLOCKS_AND_BALS).unwrap();
    let mut payload_v1 = Vec::new();
    let mut payload_v2 = Vec::new();
    let mut payload_v3 = Vec::new();
    let mut payload_v4 = Vec::new();
    let mut transaction_counts = Vec::new();
    let mut bal_sizes = Vec::new();

    for index in 0..10 {
        let raw_block = batch_result::<Bytes>(&responses, &format!("raw-{index}"));
        let bal = batch_result::<Bytes>(&responses, &format!("bal-{index}"));
        let mut raw_block = raw_block.as_ref();
        let sealed = Block::<TxEnvelope>::decode_sealed(&mut raw_block).unwrap();
        assert!(raw_block.is_empty());
        let (block, block_hash) = sealed.into_parts();

        transaction_counts.push(block.body.transactions.len());
        bal_sizes.push(bal.len());
        payload_v1.push(ExecutionPayloadV1::from_block_unchecked(block_hash, &block));
        payload_v2.push(ExecutionPayloadV2::from_block_unchecked(block_hash, &block));
        payload_v3.push(ExecutionPayloadV3::from_block_unchecked(block_hash, &block));
        payload_v4.push(ExecutionPayloadV4::from_block_unchecked_with_bal(block_hash, &block, bal));
    }

    println!("source: https://ethereum.reth.rs/rpc");
    println!(
        "sample: {} consecutive blocks, average transactions: {:.1}, average BAL: {}",
        payload_v1.len(),
        average(&transaction_counts),
        format_bytes(average(&bal_sizes) as usize),
    );

    println!("\nnewPayload - serde");
    print_header("version (fork)");
    print_row("V1 (paris)", &payload_v1);
    print_row("V2 (shanghai)", &payload_v2);
    print_row("V3 (cancun-osaka)", &payload_v3);
    print_row("V4 (amsterdam)", &payload_v4);

    let mut envelope_v2 = Vec::new();
    let mut envelope_v3 = Vec::new();
    let mut envelope_v4 = Vec::new();
    let mut envelope_v5 = Vec::new();
    let mut envelope_v6 = Vec::new();
    let mut blob_counts = Vec::new();

    for (((payload_v1, _payload_v2), payload_v3), payload_v4) in payload_v1
        .iter()
        .cloned()
        .zip(payload_v2.iter().cloned())
        .zip(payload_v3.iter().cloned())
        .zip(payload_v4.iter().cloned())
    {
        let blob_count = (payload_v3.blob_gas_used / 0x20_000) as usize;
        let blobs_v1 = blob_bundle_v1(blob_count);
        let blobs_v2 = blob_bundle_v2(blob_count);
        let requests = Requests::default();

        blob_counts.push(blob_count);
        envelope_v2.push(ExecutionPayloadEnvelopeV2 {
            execution_payload: ExecutionPayloadFieldV2::V1(payload_v1),
            block_value: U256::ZERO,
        });
        envelope_v3.push(ExecutionPayloadEnvelopeV3 {
            execution_payload: payload_v3.clone(),
            block_value: U256::ZERO,
            blobs_bundle: blobs_v1,
            should_override_builder: false,
        });
        envelope_v4.push(ExecutionPayloadEnvelopeV4 {
            envelope_inner: envelope_v3.last().unwrap().clone(),
            execution_requests: requests.clone(),
        });
        envelope_v5.push(ExecutionPayloadEnvelopeV5 {
            execution_payload: payload_v3,
            block_value: U256::ZERO,
            blobs_bundle: blobs_v2.clone(),
            should_override_builder: false,
            execution_requests: requests.clone(),
        });
        envelope_v6.push(ExecutionPayloadEnvelopeV6 {
            execution_payload: payload_v4,
            block_value: U256::ZERO,
            blobs_bundle: blobs_v2,
            should_override_builder: false,
            execution_requests: requests,
        });
    }

    println!("\ngetPayload - serde");
    println!(
        "envelope blob data: {:.1} correctly sized zero-filled blobs per block on average",
        average(&blob_counts)
    );
    print_header("version (fork)");
    print_row("V2 (paris form)", &envelope_v2);
    print_row("V3 (cancun)", &envelope_v3);
    print_row("V4 (prague)", &envelope_v4);
    print_row("V5 (osaka)", &envelope_v5);
    print_row("V6 (amsterdam)", &envelope_v6);
}

fn batch_result<T: DeserializeOwned>(responses: &[serde_json::Value], id: &str) -> T {
    let response = responses.iter().find(|response| response["id"] == id).unwrap();
    serde_json::from_value(response["result"].clone()).unwrap()
}

fn blob_bundle_v1(blob_count: usize) -> BlobsBundleV1 {
    BlobsBundleV1 {
        commitments: vec![Bytes48::default(); blob_count],
        proofs: vec![Bytes48::default(); blob_count],
        blobs: vec![Blob::default(); blob_count],
    }
}

fn blob_bundle_v2(blob_count: usize) -> BlobsBundleV2 {
    BlobsBundleV2 {
        commitments: vec![Bytes48::default(); blob_count],
        proofs: vec![Bytes48::default(); blob_count * CELLS_PER_EXT_BLOB],
        blobs: vec![Blob::default(); blob_count],
    }
}

fn print_header(first_column: &str) {
    println!(
        "| {first_column} | average wire JSON -> SSZ | encode JSON | encode SSZ | speedup | decode JSON | decode SSZ | speedup |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
}

fn print_row<T>(name: &str, values: &[T])
where
    T: Serialize + DeserializeOwned + ssz::Encode + ssz::Decode + PartialEq,
{
    let json: Vec<Vec<u8>> =
        values.iter().map(|value| serde_json::to_vec(value).unwrap()).collect();
    let ssz: Vec<Vec<u8>> = values.iter().map(ssz::Encode::as_ssz_bytes).collect();

    for ((value, json), ssz) in values.iter().zip(&json).zip(&ssz) {
        assert!(serde_json::from_slice::<T>(json).unwrap() == *value);
        assert!(<T as ssz::Decode>::from_ssz_bytes(ssz).unwrap() == *value);
    }

    let json_encode = benchmark(|| {
        for value in values {
            black_box(serde_json::to_vec(black_box(value)).unwrap());
        }
    });
    let ssz_encode = benchmark(|| {
        for value in values {
            black_box(ssz::Encode::as_ssz_bytes(black_box(value)));
        }
    });
    let json_decode = benchmark(|| {
        for json in &json {
            black_box(serde_json::from_slice::<T>(black_box(json)).unwrap());
        }
    });
    let ssz_decode = benchmark(|| {
        for ssz in &ssz {
            black_box(<T as ssz::Decode>::from_ssz_bytes(black_box(ssz)).unwrap());
        }
    });
    let count = values.len() as f64;

    println!(
        "| {name} | {} -> {} | {} | {} | {:.1}x | {} | {} | {:.1}x |",
        format_bytes(average_len(&json)),
        format_bytes(average_len(&ssz)),
        format_time(json_encode / count),
        format_time(ssz_encode / count),
        json_encode / ssz_encode,
        format_time(json_decode / count),
        format_time(ssz_decode / count),
        json_decode / ssz_decode,
    );
}

fn benchmark(mut f: impl FnMut()) -> f64 {
    let mut iterations = 1_u64;
    while run(iterations, &mut f) < Duration::from_millis(100) {
        iterations = iterations.saturating_mul(10);
    }

    let elapsed = run(iterations, &mut f);
    let iterations =
        ((iterations as f64 * SAMPLE_TIME.as_secs_f64() / elapsed.as_secs_f64()).ceil() as u64)
            .max(1);
    let elapsed = run(iterations, &mut f);
    elapsed.as_secs_f64() * 1_000_000_000.0 / iterations as f64
}

fn run(iterations: u64, f: &mut impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed()
}

fn average(values: &[usize]) -> f64 {
    values.iter().sum::<usize>() as f64 / values.len() as f64
}

fn average_len(values: &[Vec<u8>]) -> usize {
    average(&values.iter().map(Vec::len).collect::<Vec<_>>()) as usize
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1_000_000 {
        format!("{:.2} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    }
}

fn format_time(nanos: f64) -> String {
    if nanos >= 1_000_000.0 {
        format!("{:.2} ms", nanos / 1_000_000.0)
    } else if nanos >= 1_000.0 {
        format!("{:.1} us", nanos / 1_000.0)
    } else {
        format!("{nanos:.1} ns")
    }
}
