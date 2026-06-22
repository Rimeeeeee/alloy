use alloy_eips::{
    eip4844::{Blob, Bytes48},
    eip7594::Cell,
};
use alloy_primitives::{Address, Bloom, Bytes, B128, B256, B64, U256};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use ssz::Encode;
use std::boxed::Box;

#[derive(Deserialize)]
#[serde(transparent)]
struct Uint8(#[serde(deserialize_with = "decimal_u8")] u8);

impl Encode for Uint8 {
    fn is_ssz_fixed_len() -> bool {
        true
    }

    fn ssz_fixed_len() -> usize {
        1
    }

    fn ssz_bytes_len(&self) -> usize {
        1
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.0.ssz_append(buf);
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct Uint64(#[serde(deserialize_with = "decimal_u64")] u64);

impl Encode for Uint64 {
    fn is_ssz_fixed_len() -> bool {
        true
    }

    fn ssz_fixed_len() -> usize {
        8
    }

    fn ssz_bytes_len(&self) -> usize {
        8
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.0.ssz_append(buf);
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct Uint256(#[serde(deserialize_with = "decimal_u256")] U256);

impl Encode for Uint256 {
    fn is_ssz_fixed_len() -> bool {
        true
    }

    fn ssz_fixed_len() -> usize {
        32
    }

    fn ssz_bytes_len(&self) -> usize {
        32
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.0.ssz_append(buf);
    }
}

fn decimal_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
}

fn decimal_u8<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
}

fn decimal_u256<'de, D: Deserializer<'de>>(deserializer: D) -> Result<U256, D::Error> {
    U256::from_str_radix(&String::deserialize(deserializer)?, 10).map_err(serde::de::Error::custom)
}

#[derive(Deserialize, ssz_derive::Encode)]
struct Withdrawal {
    index: Uint64,
    validator_index: Uint64,
    address: Address,
    amount: Uint64,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ExecutionPayloadParis {
    parent_hash: B256,
    fee_recipient: Address,
    state_root: B256,
    receipts_root: B256,
    logs_bloom: Bloom,
    prev_randao: B256,
    block_number: Uint64,
    gas_limit: Uint64,
    gas_used: Uint64,
    timestamp: Uint64,
    extra_data: Bytes,
    base_fee_per_gas: Uint256,
    block_hash: B256,
    transactions: Vec<Bytes>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ExecutionPayloadShanghai {
    parent_hash: B256,
    fee_recipient: Address,
    state_root: B256,
    receipts_root: B256,
    logs_bloom: Bloom,
    prev_randao: B256,
    block_number: Uint64,
    gas_limit: Uint64,
    gas_used: Uint64,
    timestamp: Uint64,
    extra_data: Bytes,
    base_fee_per_gas: Uint256,
    block_hash: B256,
    transactions: Vec<Bytes>,
    withdrawals: Vec<Withdrawal>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ExecutionPayloadCancun {
    parent_hash: B256,
    fee_recipient: Address,
    state_root: B256,
    receipts_root: B256,
    logs_bloom: Bloom,
    prev_randao: B256,
    block_number: Uint64,
    gas_limit: Uint64,
    gas_used: Uint64,
    timestamp: Uint64,
    extra_data: Bytes,
    base_fee_per_gas: Uint256,
    block_hash: B256,
    transactions: Vec<Bytes>,
    withdrawals: Vec<Withdrawal>,
    blob_gas_used: Uint64,
    excess_blob_gas: Uint64,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ExecutionPayloadAmsterdam {
    parent_hash: B256,
    fee_recipient: Address,
    state_root: B256,
    receipts_root: B256,
    logs_bloom: Bloom,
    prev_randao: B256,
    block_number: Uint64,
    gas_limit: Uint64,
    gas_used: Uint64,
    timestamp: Uint64,
    extra_data: Bytes,
    base_fee_per_gas: Uint256,
    block_hash: B256,
    transactions: Vec<Bytes>,
    withdrawals: Vec<Withdrawal>,
    blob_gas_used: Uint64,
    excess_blob_gas: Uint64,
    block_access_list: Bytes,
    slot_number: Uint64,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct PayloadAttributesParis {
    timestamp: Uint64,
    prev_randao: B256,
    suggested_fee_recipient: Address,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct PayloadAttributesShanghai {
    timestamp: Uint64,
    prev_randao: B256,
    suggested_fee_recipient: Address,
    withdrawals: Vec<Withdrawal>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct PayloadAttributesCancun {
    timestamp: Uint64,
    prev_randao: B256,
    suggested_fee_recipient: Address,
    withdrawals: Vec<Withdrawal>,
    parent_beacon_block_root: B256,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct PayloadAttributesAmsterdam {
    timestamp: Uint64,
    prev_randao: B256,
    suggested_fee_recipient: Address,
    withdrawals: Vec<Withdrawal>,
    parent_beacon_block_root: B256,
    slot_number: Uint64,
    target_gas_limit: Uint64,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobsBundleV1 {
    commitments: Vec<Bytes48>,
    proofs: Vec<Bytes48>,
    blobs: Vec<Blob>,
}

type BlobsBundleV2 = BlobsBundleV1;

#[derive(Deserialize, ssz_derive::Encode)]
struct BuiltPayloadParis {
    payload: ExecutionPayloadParis,
    block_value: Uint256,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BuiltPayloadShanghai {
    payload: ExecutionPayloadShanghai,
    block_value: Uint256,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BuiltPayloadCancun {
    payload: ExecutionPayloadCancun,
    block_value: Uint256,
    blobs_bundle: BlobsBundleV1,
    should_override_builder: bool,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BuiltPayloadPrague {
    payload: ExecutionPayloadCancun,
    block_value: Uint256,
    blobs_bundle: BlobsBundleV1,
    execution_requests: Vec<Bytes>,
    should_override_builder: bool,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BuiltPayloadOsaka {
    payload: ExecutionPayloadCancun,
    block_value: Uint256,
    blobs_bundle: BlobsBundleV2,
    execution_requests: Vec<Bytes>,
    should_override_builder: bool,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BuiltPayloadAmsterdam {
    payload: ExecutionPayloadAmsterdam,
    block_value: Uint256,
    blobs_bundle: BlobsBundleV2,
    execution_requests: Vec<Bytes>,
    should_override_builder: bool,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct EnvelopeParis {
    payload: ExecutionPayloadParis,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct EnvelopeShanghai {
    payload: ExecutionPayloadShanghai,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct EnvelopeCancun {
    payload: ExecutionPayloadCancun,
    parent_beacon_block_root: B256,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct EnvelopePrague {
    payload: ExecutionPayloadCancun,
    parent_beacon_block_root: B256,
    execution_requests: Vec<Bytes>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct EnvelopeAmsterdam {
    payload: ExecutionPayloadAmsterdam,
    parent_beacon_block_root: B256,
    execution_requests: Vec<Bytes>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ForkchoiceState {
    head_block_hash: B256,
    safe_block_hash: B256,
    finalized_block_hash: B256,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ForkchoiceParis {
    forkchoice_state: ForkchoiceState,
    payload_attributes: Vec<PayloadAttributesParis>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ForkchoiceShanghai {
    forkchoice_state: ForkchoiceState,
    payload_attributes: Vec<PayloadAttributesShanghai>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ForkchoiceCancun {
    forkchoice_state: ForkchoiceState,
    payload_attributes: Vec<PayloadAttributesCancun>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ForkchoiceAmsterdam {
    forkchoice_state: ForkchoiceState,
    payload_attributes: Vec<PayloadAttributesAmsterdam>,
    custody_columns: Vec<B128>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ExecutionPayloadBodyParis {
    transactions: Vec<Bytes>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ExecutionPayloadBodyShanghai {
    transactions: Vec<Bytes>,
    withdrawals: Vec<Withdrawal>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ExecutionPayloadBodyAmsterdam {
    transactions: Vec<Bytes>,
    withdrawals: Vec<Withdrawal>,
    block_access_list: Bytes,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BodyEntry {
    available: bool,
    body: ExecutionPayloadBodyAmsterdam,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BodiesResponse {
    entries: Vec<BodyEntry>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BodiesByHashRequest {
    block_hashes: Vec<B256>,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct BlobValue(Box<Blob>);

impl Encode for BlobValue {
    fn is_ssz_fixed_len() -> bool {
        Blob::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        Blob::ssz_fixed_len()
    }

    fn ssz_bytes_len(&self) -> usize {
        self.0.ssz_bytes_len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.0.as_ref().ssz_append(buf);
    }
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobAndProofV1 {
    blob: BlobValue,
    proof: Bytes48,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobAndProofV2 {
    blob: BlobValue,
    proofs: Vec<Bytes48>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobCellsAndProofs {
    blob_cells: Vec<Vec<Cell>>,
    proofs: Vec<Vec<Bytes48>>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobEntry<T: Encode> {
    available: bool,
    contents: T,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobsResponse<T: Encode> {
    entries: Vec<BlobEntry<T>>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobsRequest {
    versioned_hashes: Vec<B256>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct BlobsV4Request {
    versioned_hashes: Vec<B256>,
    indices_bitarray: B128,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct PayloadStatus {
    status: Uint8,
    latest_valid_hash: Vec<B256>,
    validation_error: Vec<Bytes>,
}

#[derive(Deserialize, ssz_derive::Encode)]
struct ForkchoiceUpdateResponse {
    payload_status: PayloadStatus,
    payload_id: Vec<B64>,
}

fn encode<T: DeserializeOwned + Encode>(yaml: &str) -> Result<Vec<u8>, String> {
    serde_yaml::from_str::<T>(yaml).map_err(|err| err.to_string()).map(|value| value.as_ssz_bytes())
}

pub(super) fn encode_yaml(key: &str, yaml: &str) -> Result<Vec<u8>, String> {
    match key {
        "BlobAndProofV1" => encode::<BlobAndProofV1>(yaml),
        "BlobAndProofV2" => encode::<BlobAndProofV2>(yaml),
        "BlobCellsAndProofs" => encode::<BlobCellsAndProofs>(yaml),
        "BlobsBundleV1" => encode::<BlobsBundleV1>(yaml),
        "BlobsBundleV2" => encode::<BlobsBundleV2>(yaml),
        "BlobsV1Request" | "BlobsV2Request" => encode::<BlobsRequest>(yaml),
        "BlobsV1Response" => encode::<BlobsResponse<BlobAndProofV1>>(yaml),
        "BlobsV2Response" | "BlobsV3Response" => encode::<BlobsResponse<BlobAndProofV2>>(yaml),
        "BlobsV4Request" => encode::<BlobsV4Request>(yaml),
        "BlobsV4Response" => encode::<BlobsResponse<BlobCellsAndProofs>>(yaml),
        "BlobV1Entry" => encode::<BlobEntry<BlobAndProofV1>>(yaml),
        "BlobV2Entry" => encode::<BlobEntry<BlobAndProofV2>>(yaml),
        "BlobV4Entry" => encode::<BlobEntry<BlobCellsAndProofs>>(yaml),
        "BodiesByHashRequest" => encode::<BodiesByHashRequest>(yaml),
        "BodiesResponse" => encode::<BodiesResponse>(yaml),
        "BodyEntry" => encode::<BodyEntry>(yaml),
        "BuiltPayload/Amsterdam" => encode::<BuiltPayloadAmsterdam>(yaml),
        "BuiltPayload/Cancun" => encode::<BuiltPayloadCancun>(yaml),
        "BuiltPayload/Osaka" => encode::<BuiltPayloadOsaka>(yaml),
        "BuiltPayload/Paris" => encode::<BuiltPayloadParis>(yaml),
        "BuiltPayload/Prague" => encode::<BuiltPayloadPrague>(yaml),
        "BuiltPayload/Shanghai" => encode::<BuiltPayloadShanghai>(yaml),
        "ExecutionPayload/Amsterdam" => encode::<ExecutionPayloadAmsterdam>(yaml),
        "ExecutionPayload/Cancun" | "ExecutionPayload/Osaka" | "ExecutionPayload/Prague" => {
            encode::<ExecutionPayloadCancun>(yaml)
        }
        "ExecutionPayload/Paris" => encode::<ExecutionPayloadParis>(yaml),
        "ExecutionPayload/Shanghai" => encode::<ExecutionPayloadShanghai>(yaml),
        "ExecutionPayloadBody/Amsterdam" => encode::<ExecutionPayloadBodyAmsterdam>(yaml),
        "ExecutionPayloadBody/Cancun"
        | "ExecutionPayloadBody/Osaka"
        | "ExecutionPayloadBody/Prague"
        | "ExecutionPayloadBody/Shanghai" => encode::<ExecutionPayloadBodyShanghai>(yaml),
        "ExecutionPayloadBody/Paris" => encode::<ExecutionPayloadBodyParis>(yaml),
        "ExecutionPayloadEnvelope/Amsterdam" => encode::<EnvelopeAmsterdam>(yaml),
        "ExecutionPayloadEnvelope/Cancun" => encode::<EnvelopeCancun>(yaml),
        "ExecutionPayloadEnvelope/Osaka" | "ExecutionPayloadEnvelope/Prague" => {
            encode::<EnvelopePrague>(yaml)
        }
        "ExecutionPayloadEnvelope/Paris" => encode::<EnvelopeParis>(yaml),
        "ExecutionPayloadEnvelope/Shanghai" => encode::<EnvelopeShanghai>(yaml),
        "ForkchoiceState" => encode::<ForkchoiceState>(yaml),
        "ForkchoiceUpdate/Amsterdam" => encode::<ForkchoiceAmsterdam>(yaml),
        "ForkchoiceUpdate/Cancun" | "ForkchoiceUpdate/Osaka" | "ForkchoiceUpdate/Prague" => {
            encode::<ForkchoiceCancun>(yaml)
        }
        "ForkchoiceUpdate/Paris" => encode::<ForkchoiceParis>(yaml),
        "ForkchoiceUpdate/Shanghai" => encode::<ForkchoiceShanghai>(yaml),
        "ForkchoiceUpdateResponse" => encode::<ForkchoiceUpdateResponse>(yaml),
        "PayloadAttributes/Amsterdam" => encode::<PayloadAttributesAmsterdam>(yaml),
        "PayloadAttributes/Cancun" | "PayloadAttributes/Osaka" | "PayloadAttributes/Prague" => {
            encode::<PayloadAttributesCancun>(yaml)
        }
        "PayloadAttributes/Paris" => encode::<PayloadAttributesParis>(yaml),
        "PayloadAttributes/Shanghai" => encode::<PayloadAttributesShanghai>(yaml),
        "PayloadStatus" => encode::<PayloadStatus>(yaml),
        "Withdrawal" => encode::<Withdrawal>(yaml),
        _ => Err(format!("unhandled YAML vector family {key}")),
    }
}
