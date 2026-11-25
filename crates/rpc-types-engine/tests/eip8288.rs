//! EIP-8288 Engine API JSON compatibility tests.

#![cfg(feature = "serde")]

use alloy_consensus::{Block, BlockBody, Header, TxEnvelope};
use alloy_primitives::{bytes, keccak256, Bytes, FixedBytes, B256};
use alloy_rpc_types_engine::*;
use serde_json::{json, Value};

fn payload() -> ExecutionPayloadV5 {
    let block = Block::<TxEnvelope>::new(Header::default(), BlockBody::default());
    ExecutionPayloadV5 {
        payload_inner: ExecutionPayloadV4::from_block_unchecked(B256::ZERO, &block),
        dependency_proof: Bytes::new(),
    }
}

#[test]
fn required_dependency_proof_and_strict_fields() {
    let original = payload();
    let mut value = serde_json::to_value(&original).unwrap();
    assert_eq!(value["dependencyProof"], "0x");
    assert_eq!(serde_json::from_value::<ExecutionPayloadV5>(value.clone()).unwrap(), original);
    value.as_object_mut().unwrap().remove("dependencyProof");
    assert!(serde_json::from_value::<ExecutionPayloadV5>(value.clone()).is_err());
    value["dependencyProof"] = Value::Null;
    assert!(serde_json::from_value::<ExecutionPayloadV5>(value.clone()).is_err());
    value["dependencyProof"] = json!("0x");
    value["starkProof"] = json!("0x01");
    assert!(serde_json::from_value::<ExecutionPayloadV5>(value.clone()).is_err());
    value["starkProof"] = Value::Null;
    assert!(serde_json::from_value::<ExecutionPayloadV5>(value).is_ok());
    assert_eq!(original.dependency_proof_hash(), keccak256([]));
}

#[test]
fn oversized_payload_proof_is_not_a_parameter_decoding_error() {
    let mut original = payload();
    original.dependency_proof = vec![1; MAX_DEPENDENCY_PROOF_BYTES + 1].into();
    let encoded = serde_json::to_string(&original).unwrap();
    assert_eq!(serde_json::from_str::<ExecutionPayloadV5>(&encoded).unwrap(), original);
}

#[test]
fn inclusion_proof_width_size_and_opaque_transactions() {
    let list = InclusionListV1 {
        transactions: vec![bytes!("ff")],
        recursive_stark: RecursiveStarkV1 {
            stark_proof: bytes!("1234"),
            proof_deps: vec![FixedBytes::<96>::ZERO],
        },
    };
    assert_eq!(list.recursive_stark.ssz_encoded_len(), 8 + 2 + 96);
    let mut value = serde_json::to_value(&list).unwrap();
    assert_eq!(serde_json::from_value::<InclusionListV1>(value.clone()).unwrap(), list);
    value["recursiveStark"]["proofDeps"] = json!(["0x00"]);
    assert!(serde_json::from_value::<InclusionListV1>(value).is_err());
    assert!(serde_json::from_value::<InclusionListV1>(json!({"transactions": []})).is_err());
    assert_eq!(RecursiveStarkV1::default().ssz_encoded_len(), 8);
}

#[test]
fn nullable_body_proof_and_unavailable_blocks() {
    let bodies = vec![Some(ExecutionPayloadBodyV3::default()), None];
    let mut value = serde_json::to_value(&bodies).unwrap();
    assert_eq!(value[0]["dependencyProof"], Value::Null);
    assert_eq!(value[1], Value::Null);
    value[0]["dependencyProof"] = json!("0x");
    let decoded: ExecutionPayloadBodiesV3 = serde_json::from_value(value).unwrap();
    assert_eq!(decoded[0].as_ref().unwrap().dependency_proof, Some(Bytes::new()));
    assert!(decoded[1].is_none());
}

#[test]
fn v6_attributes_require_all_fields_and_use_quantities() {
    let attrs = PayloadAttributesV6 {
        timestamp: 15,
        slot_number: 16,
        target_gas_limit: 30_000_000,
        ..Default::default()
    };
    let value = serde_json::to_value(&attrs).unwrap();
    assert_eq!(value["timestamp"], "0xf");
    assert_eq!(value["slotNumber"], "0x10");
    assert_eq!(value["targetGasLimit"], "0x1c9c380");
    assert_eq!(value["inclusionLists"], json!([]));
    for key in
        ["inclusionLists", "withdrawals", "parentBeaconBlockRoot", "slotNumber", "targetGasLimit"]
    {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<PayloadAttributesV6>(missing).is_err(), "{key}");
    }
    assert_eq!(serde_json::from_value::<PayloadAttributesV6>(value).unwrap(), attrs);
}

#[test]
fn v7_envelope_preserves_proof() {
    let mut execution_payload = payload();
    execution_payload.dependency_proof = bytes!("010203");
    let envelope = ExecutionPayloadEnvelopeV7 {
        execution_payload,
        block_value: Default::default(),
        blobs_bundle: Default::default(),
        should_override_builder: false,
        execution_requests: Default::default(),
    };
    let value = serde_json::to_value(&envelope).unwrap();
    assert_eq!(value["executionPayload"]["dependencyProof"], "0x010203");
    assert_eq!(serde_json::from_value::<ExecutionPayloadEnvelopeV7>(value).unwrap(), envelope);
}

#[test]
fn v5_conversion_derives_both_commitments_and_bounds_before_hashing() {
    let original = payload();
    let block = ExecutionPayload::V5(original.clone()).into_block_raw().unwrap();
    assert_eq!(block.header.dependency_proof_hash, Some(keccak256([])));
    assert_eq!(block.header.block_deps_hash, Some(alloy_eips::eip8288::dependency_digest(&[])));
    let mut oversized = original;
    oversized.dependency_proof = vec![0; MAX_DEPENDENCY_PROOF_BYTES + 1].into();
    assert!(matches!(
        ExecutionPayload::V5(oversized).into_block_raw(),
        Err(PayloadError::DependencyProofTooLarge(_))
    ));
}

#[cfg(feature = "ssz")]
#[test]
fn v5_ssz_and_generic_json_keep_proof_and_attributes() {
    use ssz::{Decode, Encode};
    let mut original = payload();
    original.dependency_proof = bytes!("010203");
    let encoded = original.as_ssz_bytes();
    assert_eq!(encoded.len(), original.ssz_bytes_len());
    assert_eq!(ExecutionPayloadV5::from_ssz_bytes(&encoded).unwrap(), original);
    let generic: ExecutionPayload =
        serde_json::from_value(serde_json::to_value(&original).unwrap()).unwrap();
    assert_eq!(generic.as_v5(), Some(&original));
    let attrs: PayloadAttributes = PayloadAttributesV6 {
        inclusion_lists: vec![InclusionListV1::default()],
        ..Default::default()
    }
    .into();
    assert_eq!(PayloadAttributes::from_ssz_bytes(&attrs.as_ssz_bytes()).unwrap(), attrs);
}
