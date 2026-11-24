//! Engine API wire types for EIP-8288 proof-carrying inclusion lists.
//!
//! Proof verification, dependency derivation, fork checks and retention belong to the execution
//! client. These types preserve proof bytes even when verification fails.
//!
//! See <https://github.com/ethereum/execution-apis/blob/main/src/engine/bogota.md>.

use crate::{BlobsBundleV2, ExecutionPayloadBodyV2, ExecutionPayloadV5};
use alloc::vec::Vec;
use alloy_eips::{eip4895::Withdrawal, eip7685::Requests};
use alloy_primitives::{Address, Bytes, FixedBytes, B256, U256};

/// Maximum SSZ-encoded inclusion-list proof size. Provisional in EIP-8288.
pub const MAX_IL_PROOF_BYTES: u64 = 5 * (1 << 18);
/// Maximum dependency proof size in bytes. Oversized proofs make the payload invalid.
pub const MAX_DEPENDENCY_PROOF_BYTES: u64 = 1 << 20;

/// The proof attached to a signed inclusion list by EIP-8288.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct RecursiveStarkV1 {
    /// Serialized recursive aggregate proof; empty for an empty proof.
    pub stark_proof: Bytes,
    /// Claimed dependencies in the 96-byte EIP-8288 encoding.
    /// Sorting, uniqueness and enabled schemes are checked by the execution client.
    pub proof_deps: Vec<FixedBytes<96>>,
}

impl RecursiveStarkV1 {
    /// Returns the SSZ size, including two four-byte offsets and all proof and dependency bytes.
    pub fn ssz_encoded_len(&self) -> usize {
        8usize
            .saturating_add(self.stark_proof.len())
            .saturating_add(self.proof_deps.len().saturating_mul(96))
    }
}

/// One counted inclusion list with its EIP-8288 proof.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct InclusionListV1 {
    /// EIP-2718 encoded transactions, including undecodable transaction bytes.
    pub transactions: Vec<Bytes>,
    /// The signed proof, passed unchanged even if it does not verify.
    pub recursive_stark: RecursiveStarkV1,
}

/// Attributes for `engine_forkchoiceUpdatedV6`.
///
/// Extends Amsterdam attributes with counted inclusion lists in committee-position order.
/// Required fields are non-optional, including an empty `inclusion_lists` array.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct PayloadAttributesV6 {
    /// Timestamp of the new payload.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub timestamp: u64,
    /// Previous randao value.
    pub prev_randao: B256,
    /// Suggested fee recipient.
    pub suggested_fee_recipient: Address,
    /// Withdrawals of the new payload.
    pub withdrawals: Vec<Withdrawal>,
    /// Root of the parent beacon block.
    pub parent_beacon_block_root: B256,
    /// Slot number of the new payload.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub slot_number: u64,
    /// Target gas limit of the new payload.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub target_gas_limit: u64,
    /// Counted inclusion lists in committee-position order.
    pub inclusion_lists: Vec<InclusionListV1>,
}

/// Response to `engine_getPayloadV7`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadEnvelopeV7 {
    /// Execution payload carrying the dependency proof.
    pub execution_payload: ExecutionPayloadV5,
    /// Expected fee recipient value in wei.
    pub block_value: U256,
    /// Blobs, commitments and cell proofs associated with the payload.
    pub blobs_bundle: BlobsBundleV2,
    /// Whether the execution layer suggests overriding an external builder.
    pub should_override_builder: bool,
    /// Execution layer triggered requests.
    pub execution_requests: Requests,
}

/// Payload body returned by the V3 retrieval methods.
///
/// The dependency proof is transported here but stored separately from the execution block body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct ExecutionPayloadBodyV3 {
    /// EIP-2718 encoded transactions.
    pub transactions: Vec<Bytes>,
    /// Withdrawals, or `None` before Shanghai.
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// RLP-encoded block access list, or `None` before Amsterdam or after pruning.
    pub block_access_list: Option<Bytes>,
    /// Dependency proof, or `None` before EIP-8288 or after pruning.
    /// An available empty proof is `Some(Bytes::new())`, serialized as `"0x"` rather than `null`.
    pub dependency_proof: Option<Bytes>,
}

impl ExecutionPayloadBodyV3 {
    /// Attaches a separately stored dependency proof to a V2 payload body.
    pub fn new(body: ExecutionPayloadBodyV2, dependency_proof: Option<Bytes>) -> Self {
        Self {
            transactions: body.transactions,
            withdrawals: body.withdrawals,
            block_access_list: body.block_access_list,
            dependency_proof,
        }
    }
}

/// V3 payload body response, with `None` for unavailable blocks.
pub type ExecutionPayloadBodiesV3 = Vec<Option<ExecutionPayloadBodyV3>>;
