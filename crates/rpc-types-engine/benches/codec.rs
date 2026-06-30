//! Codec benchmarks for Engine API execution payloads and payload envelopes.

use alloy_consensus::{Blob, Block, Bytes48, TxEnvelope};
use alloy_eips::{
    eip7594::{Cell, BYTES_PER_CELL, CELLS_PER_EXT_BLOB},
    eip7685::Requests,
};
use alloy_primitives::{hex, Bytes, B128, B256, U256};
use alloy_rpc_types_engine::{
    ssz_engine_types::{
        BlobCellsAndProofs, BlobEntry, BlobsV1Request, BlobsV1Response, BlobsV2Response,
        BlobsV3Response, BlobsV4Request, BlobsV4Response, BodiesByHashRequest, BodiesResponse,
        BodyEntry, BuiltPayloadAmsterdam, BuiltPayloadOsaka, BuiltPayloadParis, BuiltPayloadPrague,
        BuiltPayloadShanghai, ExecutionPayloadBodyAmsterdam, ExecutionPayloadBodyParis,
        ExecutionPayloadBodyShanghai, ExecutionPayloadEnvelopeAmsterdam,
        ExecutionPayloadEnvelopeCancun, ExecutionPayloadEnvelopeOsaka,
        ExecutionPayloadEnvelopeParis, ExecutionPayloadEnvelopePrague,
        ExecutionPayloadEnvelopeShanghai, ExecutionWitnessV1, Optional, PayloadStatus,
        PayloadStatusWithWitness, WitnessCodeV1, WitnessHeaderV1, WitnessNodeV1,
    },
    BlobAndProofV1, BlobAndProofV2, BlobsBundleV1, BlobsBundleV2, ExecutionPayloadEnvelopeV2,
    ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadEnvelopeV5,
    ExecutionPayloadEnvelopeV6, ExecutionPayloadFieldV2, ExecutionPayloadV1, ExecutionPayloadV2,
    ExecutionPayloadV3, ExecutionPayloadV4, PayloadStatusEnum,
};
use serde::{de::DeserializeOwned, Serialize};
use ssz_types::{
    typenum::{U1048576, U128, U32},
    VariableList,
};
use std::{
    boxed::Box,
    hint::black_box,
    time::{Duration, Instant},
};

const BLOCKS_AND_BALS: &str = include_str!("../testdata/payload/reth-blocks-bals-20.json");
const REAL_BLOBS: &str = include_str!("../testdata/payload/reth-real-blobs-8.json");
const SAMPLE_TIME: Duration = Duration::from_secs(1);

fn main() {
    let responses: Vec<serde_json::Value> = serde_json::from_str(BLOCKS_AND_BALS).unwrap();
    let mut payload_v1 = Vec::new();
    let mut payload_v2 = Vec::new();
    let mut payload_v3 = Vec::new();
    let mut payload_v4 = Vec::new();
    let mut transaction_counts = Vec::new();
    let mut bal_sizes = Vec::new();
    let mut block_hashes = Vec::new();

    for index in 0..20 {
        let raw_block = batch_result::<Bytes>(&responses, &format!("raw-{index}"));
        let bal = batch_result::<Bytes>(&responses, &format!("bal-{index}"));
        let mut raw_block = raw_block.as_ref();
        let sealed = Block::<TxEnvelope>::decode_sealed(&mut raw_block).unwrap();
        assert!(raw_block.is_empty());
        let (block, block_hash) = sealed.into_parts();

        block_hashes.push(block_hash);
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
    let mut rest_new_payload_paris = Vec::new();
    let mut rest_new_payload_shanghai = Vec::new();
    let mut rest_new_payload_cancun = Vec::new();
    let mut rest_new_payload_prague = Vec::new();
    let mut rest_new_payload_osaka = Vec::new();
    let mut rest_new_payload_amsterdam = Vec::new();
    let mut rest_built_paris = Vec::new();
    let mut rest_built_shanghai = Vec::new();
    let mut rest_built_cancun = Vec::new();
    let mut rest_built_prague = Vec::new();
    let mut rest_built_osaka = Vec::new();
    let mut rest_built_amsterdam = Vec::new();
    let mut rest_body_paris = Vec::new();
    let mut rest_body_shanghai = Vec::new();
    let mut rest_body_amsterdam = Vec::new();

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

    for (((payload_v1, payload_v2), payload_v3), payload_v4) in payload_v1
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
        let parent_beacon_block_root = B256::ZERO;

        rest_new_payload_paris.push(ExecutionPayloadEnvelopeParis { payload: payload_v1.clone() });
        rest_new_payload_shanghai
            .push(ExecutionPayloadEnvelopeShanghai { payload: payload_v2.clone() });
        rest_new_payload_cancun.push(ExecutionPayloadEnvelopeCancun {
            payload: payload_v3.clone(),
            parent_beacon_block_root,
        });
        rest_new_payload_prague.push(ExecutionPayloadEnvelopePrague {
            payload: payload_v3.clone(),
            parent_beacon_block_root,
            execution_requests: requests.clone(),
        });
        rest_new_payload_osaka.push(ExecutionPayloadEnvelopeOsaka {
            payload: payload_v3.clone(),
            parent_beacon_block_root,
            execution_requests: requests.clone(),
        });
        rest_new_payload_amsterdam.push(ExecutionPayloadEnvelopeAmsterdam {
            payload: payload_v4.clone(),
            parent_beacon_block_root,
            execution_requests: requests.clone(),
        });

        rest_built_paris
            .push(BuiltPayloadParis { payload: payload_v1.clone(), block_value: U256::ZERO });
        rest_built_shanghai
            .push(BuiltPayloadShanghai { payload: payload_v2.clone(), block_value: U256::ZERO });
        rest_built_cancun.push(envelope_v3.last().unwrap().clone());
        rest_built_prague.push(BuiltPayloadPrague {
            payload: payload_v3.clone(),
            block_value: U256::ZERO,
            blobs_bundle: blobs_v1,
            execution_requests: requests.clone(),
            should_override_builder: false,
        });
        rest_built_osaka.push(BuiltPayloadOsaka {
            payload: payload_v3.clone(),
            block_value: U256::ZERO,
            blobs_bundle: blobs_v2.clone(),
            execution_requests: requests.clone(),
            should_override_builder: false,
        });
        rest_built_amsterdam.push(BuiltPayloadAmsterdam {
            payload: payload_v4.clone(),
            block_value: U256::ZERO,
            blobs_bundle: blobs_v2,
            execution_requests: requests,
            should_override_builder: false,
        });

        rest_body_paris
            .push(ExecutionPayloadBodyParis { transactions: payload_v1.transactions.clone() });
        rest_body_shanghai.push(ExecutionPayloadBodyShanghai {
            transactions: payload_v2.payload_inner.transactions.clone(),
            withdrawals: payload_v2.withdrawals.clone(),
        });
        rest_body_amsterdam.push(ExecutionPayloadBodyAmsterdam {
            transactions: payload_v4.payload_inner.payload_inner.payload_inner.transactions.clone(),
            withdrawals: payload_v4.payload_inner.payload_inner.withdrawals.clone(),
            block_access_list: payload_v4.block_access_list.clone(),
        });
    }

    println!("\nREST newPayload fork containers - serde");
    print_header("version (fork)");
    print_row("Paris", &rest_new_payload_paris);
    print_row("Shanghai", &rest_new_payload_shanghai);
    print_row("Cancun", &rest_new_payload_cancun);
    print_row("Prague", &rest_new_payload_prague);
    print_row("Osaka", &rest_new_payload_osaka);
    print_row("Amsterdam", &rest_new_payload_amsterdam);

    println!("\nREST builtPayload fork containers - serde");
    print_header("version (fork)");
    print_row("Paris", &rest_built_paris);
    print_row("Shanghai", &rest_built_shanghai);
    print_row("Cancun", &rest_built_cancun);
    print_row("Prague", &rest_built_prague);
    print_row("Osaka", &rest_built_osaka);
    print_row("Amsterdam", &rest_built_amsterdam);

    println!("\nREST payload body fork containers - serde");
    print_header("version (fork)");
    print_row("Paris", &rest_body_paris);
    print_row("Shanghai-Cancun-Prague-Osaka", &rest_body_shanghai);
    print_row("Amsterdam", &rest_body_amsterdam);

    let bodies_by_hash = vec![BodiesByHashRequest {
        block_hashes: VariableList::<B256, U32>::new(block_hashes.clone()).unwrap(),
    }];
    let bodies_paris = vec![bodies_response(rest_body_paris.clone())];
    let bodies_shanghai = vec![bodies_response(rest_body_shanghai.clone())];
    let bodies_amsterdam = vec![bodies_response(rest_body_amsterdam.clone())];

    println!("\nREST payload bodies requests/responses - serde");
    print_header("type");
    print_row("BodiesByHashRequest(20)", &bodies_by_hash);
    print_row("BodiesResponse Paris(20)", &bodies_paris);
    print_row("BodiesResponse Shanghai-Cancun-Prague-Osaka(20)", &bodies_shanghai);
    print_row("BodiesResponse Amsterdam(20)", &bodies_amsterdam);

    let real_blobs = real_blob_sidecars();
    let versioned_hashes = VariableList::<B256, U128>::new(
        real_blobs.iter().map(|blob| blob.versioned_hash).collect(),
    )
    .unwrap();
    let blobs_v1_request = vec![BlobsV1Request { versioned_hashes: versioned_hashes.clone() }];
    let blobs_v4_request =
        vec![BlobsV4Request { versioned_hashes, indices_bitarray: B128::repeat_byte(0xff) }];
    let blob_v1: Vec<_> = real_blobs
        .iter()
        .map(|blob| BlobAndProofV1 { blob: Box::new(blob.blob), proof: blob.kzg_proof })
        .collect();
    let blob_v2: Vec<_> = real_blobs
        .iter()
        .map(|blob| BlobAndProofV2 {
            blob: Box::new(blob.blob),
            proofs: vec![blob.kzg_proof; CELLS_PER_EXT_BLOB],
        })
        .collect();
    let blobs_v1_response =
        vec![BlobsV1Response::try_from(blob_v1.into_iter().map(Some).collect::<Vec<_>>()).unwrap()];
    let blobs_v2_response = vec![BlobsV2Response::try_from(blob_v2.clone()).unwrap()];
    let blobs_v3_response =
        vec![BlobsV3Response::try_from(blob_v2.into_iter().map(Some).collect::<Vec<_>>()).unwrap()];
    let blob_cells_and_proofs = real_blob_cells_and_proofs(&real_blobs[0]);
    let blobs_v4_response = vec![BlobsV4Response {
        entries: VariableList::new(vec![BlobEntry {
            available: true,
            contents: blob_cells_and_proofs,
        }])
        .unwrap(),
    }];

    println!("\nREST blob requests/responses - serde");
    print_header("type");
    print_row("BlobsV1Request / V2 / V3 real hashes", &blobs_v1_request);
    print_row("BlobsV4Request", &blobs_v4_request);
    print_row("BlobsV1Response(8 real)", &blobs_v1_response);
    print_row("BlobsV2Response(8 real blobs, proof-shaped)", &blobs_v2_response);
    print_row("BlobsV3Response(8 real blobs, proof-shaped)", &blobs_v3_response);
    print_row("BlobsV4Response(1 partial real cells, proof-shaped)", &blobs_v4_response);

    let witness = ExecutionWitnessV1 {
        state: VariableList::<WitnessNodeV1, U1048576>::new(vec![
            WitnessNodeV1::new(vec![0x11; 512]).unwrap(),
            WitnessNodeV1::new(vec![0x22; 1024]).unwrap(),
        ])
        .unwrap(),
        codes: VariableList::<WitnessCodeV1, U1048576>::new(vec![WitnessCodeV1::new(vec![
            0x33;
            4096
        ])
        .unwrap()])
        .unwrap(),
        headers: VariableList::<WitnessHeaderV1, U1048576>::new(vec![WitnessHeaderV1::new(
            vec![0x44; 512],
        )
        .unwrap()])
        .unwrap(),
    };
    let payload_status = PayloadStatus {
        status: PayloadStatusEnum::Valid,
        latest_valid_hash: Optional::some(block_hashes[0]),
    };
    let witness_responses = vec![
        PayloadStatusWithWitness::new(payload_status.clone(), None),
        PayloadStatusWithWitness::new(payload_status, Some(witness)),
    ];

    println!("\nREST payload witness response - serde");
    print_header("type");
    print_row("PayloadStatusWithWitness none/full", &witness_responses);
}

fn batch_result<T: DeserializeOwned>(responses: &[serde_json::Value], id: &str) -> T {
    let response = responses.iter().find(|response| response["id"] == id).unwrap();
    serde_json::from_value(response["result"].clone()).unwrap()
}

#[derive(serde::Deserialize)]
struct BlobFixture {
    data: Vec<BlobFixtureEntry>,
}

#[derive(serde::Deserialize)]
struct BlobFixtureEntry {
    versioned_hash: B256,
    blob: String,
    kzg_proof: Bytes48,
}

#[derive(Clone)]
struct RealBlobSidecar {
    versioned_hash: B256,
    blob: Blob,
    kzg_proof: Bytes48,
}

fn real_blob_sidecars() -> Vec<RealBlobSidecar> {
    let fixture: BlobFixture = serde_json::from_str(REAL_BLOBS).unwrap();
    fixture
        .data
        .into_iter()
        .map(|entry| RealBlobSidecar {
            versioned_hash: entry.versioned_hash,
            blob: fixed_hex::<{ Blob::len_bytes() }>(&entry.blob),
            kzg_proof: entry.kzg_proof,
        })
        .collect()
}

fn real_blob_cells_and_proofs(blob: &RealBlobSidecar) -> BlobCellsAndProofs {
    let cells = [0, 1, 2].map(|cell_index| {
        let start = cell_index * BYTES_PER_CELL;
        Optional::some(Cell::from_slice(&blob.blob[start..start + BYTES_PER_CELL]))
    });
    BlobCellsAndProofs {
        blob_cells: VariableList::new(cells.into()).unwrap(),
        proofs: VariableList::new(vec![Optional::some(blob.kzg_proof); 3]).unwrap(),
    }
}

fn fixed_hex<const N: usize>(value: &str) -> alloy_primitives::FixedBytes<N> {
    let bytes = hex::decode(value.strip_prefix("0x").unwrap_or(value)).unwrap();
    alloy_primitives::FixedBytes::<N>::from_slice(&bytes)
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

fn bodies_response<T>(bodies: Vec<T>) -> BodiesResponse<T> {
    BodiesResponse {
        entries: VariableList::new(bodies.into_iter().map(BodyEntry::available).collect()).unwrap(),
    }
}

fn print_header(first_column: &str) {
    println!(
        "| {first_column} | case | wire JSON -> SSZ | encode JSON | encode SSZ | speedup | decode JSON | decode SSZ | speedup |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
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
        "| {name} | avg | {} -> {} | {} | {} | {:.1}x | {} | {} | {:.1}x |",
        format_bytes(average_len(&json)),
        format_bytes(average_len(&ssz)),
        format_time(json_encode / count),
        format_time(ssz_encode / count),
        json_encode / ssz_encode,
        format_time(json_decode / count),
        format_time(ssz_decode / count),
        json_decode / ssz_decode,
    );

    let max_index =
        json.iter().enumerate().max_by_key(|(_, json)| json.len()).map(|(index, _)| index).unwrap();
    let value = &values[max_index];
    let json = &json[max_index];
    let ssz = &ssz[max_index];
    let json_encode = benchmark(|| {
        black_box(serde_json::to_vec(black_box(value)).unwrap());
    });
    let ssz_encode = benchmark(|| {
        black_box(ssz::Encode::as_ssz_bytes(black_box(value)));
    });
    let json_decode = benchmark(|| {
        black_box(serde_json::from_slice::<T>(black_box(json)).unwrap());
    });
    let ssz_decode = benchmark(|| {
        black_box(<T as ssz::Decode>::from_ssz_bytes(black_box(ssz)).unwrap());
    });

    println!(
        "| {name} | max | {} -> {} | {} | {} | {:.1}x | {} | {} | {:.1}x |",
        format_bytes(json.len()),
        format_bytes(ssz.len()),
        format_time(json_encode),
        format_time(ssz_encode),
        json_encode / ssz_encode,
        format_time(json_decode),
        format_time(ssz_decode),
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
