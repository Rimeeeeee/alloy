use crate::Provider;
use alloy_eips::eip7685::RequestsOrHash;
use alloy_network::Network;
use alloy_primitives::{B128, B256, BlockHash, Bytes, U64};
use alloy_rpc_types_engine::{
    BlobAndProofV1, BlobAndProofV2, ClientVersionV1, ExecutionPayloadBodiesV1,
    ExecutionPayloadBodiesV2, ExecutionPayloadBodiesV3, ExecutionPayloadEnvelopeV2,
    ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4, ExecutionPayloadEnvelopeV5,
    ExecutionPayloadEnvelopeV6, ExecutionPayloadEnvelopeV7, ExecutionPayloadInputV2,
    ExecutionPayloadV1, ExecutionPayloadV3, ExecutionPayloadV4, ExecutionPayloadV5,
    ForkchoiceState, ForkchoiceUpdated, ForkchoiceUpdatedResponseV2, InclusionListV1,
    PayloadAttributes, PayloadAttributesV6, PayloadId, PayloadStatus, PayloadStatusV2,
};
use alloy_transport::TransportResult;

/// Extension trait that gives access to engine API RPC methods.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait EngineApi<N>: Send + Sync {
    /// Submits a payload and proof-carrying inclusion lists under EIP-8288.
    async fn new_payload_v7(
        &self,
        payload: ExecutionPayloadV5,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
        inclusion_lists: Vec<InclusionListV1>,
    ) -> TransportResult<PayloadStatusV2>;

    /// Updates fork choice with EIP-8288 attributes and EIP-8070 custody columns.
    async fn fork_choice_updated_v6(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributesV6>,
        custody_columns: Option<B128>,
    ) -> TransportResult<ForkchoiceUpdatedResponseV2>;

    /// Retrieves a built payload with its dependency proof.
    async fn get_payload_v7(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV7>;

    /// Retrieves payload bodies and separately stored proofs by hash.
    async fn get_payload_bodies_by_hash_v3(
        &self,
        block_hashes: Vec<BlockHash>,
    ) -> TransportResult<ExecutionPayloadBodiesV3>;

    /// Retrieves payload bodies and proofs for a block range.
    async fn get_payload_bodies_by_range_v3(
        &self,
        start: u64,
        count: u64,
    ) -> TransportResult<ExecutionPayloadBodiesV3>;

    /// Retrieves an inclusion list with its inline EIP-8288 proof.
    async fn get_inclusion_list_v2(&self) -> TransportResult<InclusionListV1>;

    /// Sends the given payload to the execution layer client, as specified for the Paris fork.
    ///
    /// Caution: This should not accept the `withdrawals` field
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/paris.md#engine_newpayloadv1>
    async fn new_payload_v1(&self, payload: ExecutionPayloadV1) -> TransportResult<PayloadStatus>;

    /// Sends the given payload to the execution layer client, as specified for the Shanghai fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/584905270d8ad665718058060267061ecfd79ca5/src/engine/shanghai.md#engine_newpayloadv2>
    async fn new_payload_v2(
        &self,
        payload: ExecutionPayloadInputV2,
    ) -> TransportResult<PayloadStatus>;

    /// Sends the given payload to the execution layer client, as specified for the Cancun fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#engine_newpayloadv3>
    async fn new_payload_v3(
        &self,
        payload: ExecutionPayloadV3,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
    ) -> TransportResult<PayloadStatus>;

    /// Sends the given payload to the execution layer client, as specified for the Prague fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/03911ffc053b8b806123f1fc237184b0092a485a/src/engine/prague.md#engine_newpayloadv4>
    async fn new_payload_v4(
        &self,
        payload: ExecutionPayloadV3,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: Vec<Bytes>,
    ) -> TransportResult<PayloadStatus>;

    /// Sends the given payload to the execution layer client, as specified for the Prague fork.
    ///
    /// This is a variant of [`Self::new_payload_v4`] that accepts [`RequestsOrHash`] for the
    /// execution requests, allowing either the full requests or just a precomputed hash.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/03911ffc053b8b806123f1fc237184b0092a485a/src/engine/prague.md#engine_newpayloadv4>
    async fn new_payload_v4_requests(
        &self,
        payload: ExecutionPayloadV3,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
    ) -> TransportResult<PayloadStatus>;

    /// Sends the given payload to the execution layer client, as specified for the Amsterdam fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/7b4d9f62a3fe62b9b8dcb355f1c5a38b5ff084f6/src/engine/amsterdam.md#engine_newpayloadv5>
    async fn new_payload_v5(
        &self,
        payload: ExecutionPayloadV4,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
    ) -> TransportResult<PayloadStatus>;

    /// Sends the given payload to the execution layer client, as specified for the Bogota fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/bogota.md#engine_newpayloadv6>
    async fn new_payload_v6(
        &self,
        payload: ExecutionPayloadV4,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
        inclusion_list_transactions: Vec<Bytes>,
    ) -> TransportResult<PayloadStatusV2>;

    /// Updates the execution layer client with the given fork choice, as specified for the Paris
    /// fork.
    ///
    /// Caution: This should not accept the `withdrawals` field in the payload attributes.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/paris.md#engine_forkchoiceupdatedv1>
    async fn fork_choice_updated_v1(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated>;

    /// Updates the execution layer client with the given fork choice, as specified for the Shanghai
    /// fork.
    ///
    /// Caution: This should not accept the `parentBeaconBlockRoot` field in the payload attributes.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/shanghai.md#engine_forkchoiceupdatedv2>
    async fn fork_choice_updated_v2(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated>;

    /// Updates the execution layer client with the given fork choice, as specified for the Cancun
    /// fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#engine_forkchoiceupdatedv3>
    async fn fork_choice_updated_v3(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated>;

    /// Updates the execution layer client with the given fork choice, as specified for the
    /// Amsterdam fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/amsterdam.md#engine_forkchoiceupdatedv4>
    async fn fork_choice_updated_v4(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated>;

    /// Updates the execution layer client with the given fork choice, as specified for the Bogota
    /// fork.
    ///
    /// `custody_columns` is the custody-column bitmask used for [EIP-8070] sparse blobpool
    /// signaling. It must be 16 bytes when set.
    ///
    /// [EIP-8070]: https://eips.ethereum.org/EIPS/eip-8070
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/bogota.md#engine_forkchoiceupdatedv5>
    async fn fork_choice_updated_v5(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
        custody_columns: Option<B128>,
    ) -> TransportResult<ForkchoiceUpdatedResponseV2>;

    /// Retrieves an execution payload from a previously started build process, as specified for the
    /// Paris fork.
    ///
    /// Caution: This should not return the `withdrawals` field
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/paris.md#engine_getpayloadv1>
    ///
    /// Note:
    /// > Provider software MAY stop the corresponding build process after serving this call.
    async fn get_payload_v1(&self, payload_id: PayloadId) -> TransportResult<ExecutionPayloadV1>;

    /// Retrieves an execution payload from a previously started build process, as specified for the
    /// Shanghai fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/engine/shanghai.md#engine_getpayloadv2>
    ///
    /// Note:
    /// > Provider software MAY stop the corresponding build process after serving this call.
    async fn get_payload_v2(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV2>;

    /// Retrieves an execution payload from a previously started build process, as specified for the
    /// Cancun fork.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#engine_getpayloadv3>
    ///
    /// Note:
    /// > Provider software MAY stop the corresponding build process after serving this call.
    async fn get_payload_v3(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV3>;

    /// Returns the most recent version of the payload that is available in the corresponding
    /// payload build process at the time of receiving this call.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/prague.md#engine_getpayloadv4>
    ///
    /// Note:
    /// > Provider software MAY stop the corresponding build process after serving this call.
    async fn get_payload_v4(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV4>;

    /// Returns the most recent version of the payload that is available in the corresponding
    /// payload build process at the time of receiving this call.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/osaka.md#engine_getpayloadv5>
    ///
    /// Note:
    /// > Provider software MAY stop the corresponding build process after serving this call.
    async fn get_payload_v5(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV5>;

    /// Returns the most recent version of the payload that is available in the corresponding
    /// payload build process at the time of receiving this call.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/7b4d9f62a3fe62b9b8dcb355f1c5a38b5ff084f6/src/engine/amsterdam.md#engine_getpayloadv6>
    ///
    /// Note:
    /// > Provider software MAY stop the corresponding build process after serving this call.
    async fn get_payload_v6(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV6>;

    /// Returns the execution payload bodies by the given hash.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6452a6b194d7db269bf1dbd087a267251d3cc7f8/src/engine/shanghai.md#engine_getpayloadbodiesbyhashv1>
    async fn get_payload_bodies_by_hash_v1(
        &self,
        block_hashes: Vec<BlockHash>,
    ) -> TransportResult<ExecutionPayloadBodiesV1>;

    /// Returns the execution payload bodies by the range starting at `start`, containing `count`
    /// blocks.
    ///
    /// WARNING: This method is associated with the BeaconBlocksByRange message in the consensus
    /// layer p2p specification, meaning the input should be treated as untrusted or potentially
    /// adversarial.
    ///
    /// Implementers should take care when acting on the input to this method, specifically
    /// ensuring that the range is limited properly, and that the range boundaries are computed
    /// correctly and without panics.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6452a6b194d7db269bf1dbd087a267251d3cc7f8/src/engine/shanghai.md#engine_getpayloadbodiesbyrangev1>
    async fn get_payload_bodies_by_range_v1(
        &self,
        start: u64,
        count: u64,
    ) -> TransportResult<ExecutionPayloadBodiesV1>;

    /// Returns the execution payload bodies by the given hash.
    ///
    /// This is a V2 variant that includes the `blockAccessList` field per EIP-7928.
    ///
    /// See also <https://eips.ethereum.org/EIPS/eip-7928>
    async fn get_payload_bodies_by_hash_v2(
        &self,
        block_hashes: Vec<BlockHash>,
    ) -> TransportResult<ExecutionPayloadBodiesV2>;

    /// Returns the execution payload bodies by the range starting at `start`, containing `count`
    /// blocks.
    ///
    /// This is a V2 variant that includes the `blockAccessList` field per EIP-7928.
    ///
    /// WARNING: This method is associated with the BeaconBlocksByRange message in the consensus
    /// layer p2p specification, meaning the input should be treated as untrusted or potentially
    /// adversarial.
    ///
    /// Implementers should take care when acting on the input to this method, specifically
    /// ensuring that the range is limited properly, and that the range boundaries are computed
    /// correctly and without panics.
    ///
    /// See also <https://eips.ethereum.org/EIPS/eip-7928>
    async fn get_payload_bodies_by_range_v2(
        &self,
        start: u64,
        count: u64,
    ) -> TransportResult<ExecutionPayloadBodiesV2>;

    /// Returns the requested blobs and their associated proofs for the given versioned hashes.
    ///
    /// Returns `None` for any blob that is not available.
    ///
    /// See also <https://github.com/ethereum/execution-apis/pull/559>
    async fn get_blobs_v1(
        &self,
        versioned_hashes: Vec<B256>,
    ) -> TransportResult<Vec<Option<BlobAndProofV1>>>;

    /// Returns the requested blobs and their associated cell proofs for the given versioned
    /// hashes.
    ///
    /// Returns `None` for any blob that is not available.
    ///
    /// See also <https://github.com/ethereum/execution-apis/pull/630>
    async fn get_blobs_v2(
        &self,
        versioned_hashes: Vec<B256>,
    ) -> TransportResult<Vec<Option<BlobAndProofV2>>>;

    /// Returns the execution client version information.
    ///
    /// Note:
    /// > The `client_version` parameter identifies the consensus client.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/main/src/engine/identification.md#engine_getclientversionv1>
    async fn get_client_version_v1(
        &self,
        client_version: ClientVersionV1,
    ) -> TransportResult<Vec<ClientVersionV1>>;

    /// Returns the list of Engine API methods supported by the execution layer client software.
    ///
    /// See also <https://github.com/ethereum/execution-apis/blob/6452a6b194d7db269bf1dbd087a267251d3cc7f8/src/engine/common.md#capabilities>
    async fn exchange_capabilities(
        &self,
        capabilities: Vec<String>,
    ) -> TransportResult<Vec<String>>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> EngineApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    async fn new_payload_v7(
        &self,
        payload: ExecutionPayloadV5,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
        inclusion_lists: Vec<InclusionListV1>,
    ) -> TransportResult<PayloadStatusV2> {
        self.client()
            .request(
                "engine_newPayloadV7",
                (
                    payload,
                    versioned_hashes,
                    parent_beacon_block_root,
                    execution_requests,
                    inclusion_lists,
                ),
            )
            .await
    }

    async fn fork_choice_updated_v6(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributesV6>,
        custody_columns: Option<B128>,
    ) -> TransportResult<ForkchoiceUpdatedResponseV2> {
        self.client()
            .request(
                "engine_forkchoiceUpdatedV6",
                (fork_choice_state, payload_attributes, custody_columns),
            )
            .await
    }

    async fn get_payload_v7(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV7> {
        self.client().request("engine_getPayloadV7", (payload_id,)).await
    }

    async fn get_payload_bodies_by_hash_v3(
        &self,
        block_hashes: Vec<BlockHash>,
    ) -> TransportResult<ExecutionPayloadBodiesV3> {
        self.client().request("engine_getPayloadBodiesByHashV3", (block_hashes,)).await
    }

    async fn get_payload_bodies_by_range_v3(
        &self,
        start: u64,
        count: u64,
    ) -> TransportResult<ExecutionPayloadBodiesV3> {
        self.client()
            .request("engine_getPayloadBodiesByRangeV3", (U64::from(start), U64::from(count)))
            .await
    }

    async fn get_inclusion_list_v2(&self) -> TransportResult<InclusionListV1> {
        self.client().request_noparams("engine_getInclusionListV2").await
    }

    async fn new_payload_v1(&self, payload: ExecutionPayloadV1) -> TransportResult<PayloadStatus> {
        self.client().request("engine_newPayloadV1", (payload,)).await
    }

    async fn new_payload_v2(
        &self,
        payload: ExecutionPayloadInputV2,
    ) -> TransportResult<PayloadStatus> {
        self.client().request("engine_newPayloadV2", (payload,)).await
    }

    async fn new_payload_v3(
        &self,
        payload: ExecutionPayloadV3,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
    ) -> TransportResult<PayloadStatus> {
        self.client()
            .request("engine_newPayloadV3", (payload, versioned_hashes, parent_beacon_block_root))
            .await
    }

    async fn new_payload_v4(
        &self,
        payload: ExecutionPayloadV3,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: Vec<Bytes>,
    ) -> TransportResult<PayloadStatus> {
        self.client()
            .request(
                "engine_newPayloadV4",
                (payload, versioned_hashes, parent_beacon_block_root, execution_requests),
            )
            .await
    }

    async fn new_payload_v4_requests(
        &self,
        payload: ExecutionPayloadV3,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
    ) -> TransportResult<PayloadStatus> {
        self.client()
            .request(
                "engine_newPayloadV4",
                (payload, versioned_hashes, parent_beacon_block_root, execution_requests),
            )
            .await
    }

    async fn new_payload_v5(
        &self,
        payload: ExecutionPayloadV4,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
    ) -> TransportResult<PayloadStatus> {
        self.client()
            .request(
                "engine_newPayloadV5",
                (payload, versioned_hashes, parent_beacon_block_root, execution_requests),
            )
            .await
    }

    async fn new_payload_v6(
        &self,
        payload: ExecutionPayloadV4,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
        execution_requests: RequestsOrHash,
        inclusion_list_transactions: Vec<Bytes>,
    ) -> TransportResult<PayloadStatusV2> {
        self.client()
            .request(
                "engine_newPayloadV6",
                (
                    payload,
                    versioned_hashes,
                    parent_beacon_block_root,
                    execution_requests,
                    inclusion_list_transactions,
                ),
            )
            .await
    }

    async fn fork_choice_updated_v1(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated> {
        self.client()
            .request("engine_forkchoiceUpdatedV1", (fork_choice_state, payload_attributes))
            .await
    }

    async fn fork_choice_updated_v2(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated> {
        self.client()
            .request("engine_forkchoiceUpdatedV2", (fork_choice_state, payload_attributes))
            .await
    }

    async fn fork_choice_updated_v3(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated> {
        self.client()
            .request("engine_forkchoiceUpdatedV3", (fork_choice_state, payload_attributes))
            .await
    }

    async fn fork_choice_updated_v4(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
    ) -> TransportResult<ForkchoiceUpdated> {
        self.client()
            .request("engine_forkchoiceUpdatedV4", (fork_choice_state, payload_attributes))
            .await
    }

    async fn fork_choice_updated_v5(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<PayloadAttributes>,
        custody_columns: Option<B128>,
    ) -> TransportResult<ForkchoiceUpdatedResponseV2> {
        self.client()
            .request(
                "engine_forkchoiceUpdatedV5",
                (fork_choice_state, payload_attributes, custody_columns),
            )
            .await
    }

    async fn get_payload_v1(&self, payload_id: PayloadId) -> TransportResult<ExecutionPayloadV1> {
        self.client().request("engine_getPayloadV1", (payload_id,)).await
    }

    async fn get_payload_v2(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV2> {
        self.client().request("engine_getPayloadV2", (payload_id,)).await
    }

    async fn get_payload_v3(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV3> {
        self.client().request("engine_getPayloadV3", (payload_id,)).await
    }

    async fn get_payload_v4(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV4> {
        self.client().request("engine_getPayloadV4", (payload_id,)).await
    }

    async fn get_payload_v5(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV5> {
        self.client().request("engine_getPayloadV5", (payload_id,)).await
    }

    async fn get_payload_v6(
        &self,
        payload_id: PayloadId,
    ) -> TransportResult<ExecutionPayloadEnvelopeV6> {
        self.client().request("engine_getPayloadV6", (payload_id,)).await
    }

    async fn get_payload_bodies_by_hash_v1(
        &self,
        block_hashes: Vec<BlockHash>,
    ) -> TransportResult<ExecutionPayloadBodiesV1> {
        self.client().request("engine_getPayloadBodiesByHashV1", (block_hashes,)).await
    }

    async fn get_payload_bodies_by_range_v1(
        &self,
        start: u64,
        count: u64,
    ) -> TransportResult<ExecutionPayloadBodiesV1> {
        self.client()
            .request("engine_getPayloadBodiesByRangeV1", (U64::from(start), U64::from(count)))
            .await
    }

    async fn get_payload_bodies_by_hash_v2(
        &self,
        block_hashes: Vec<BlockHash>,
    ) -> TransportResult<ExecutionPayloadBodiesV2> {
        self.client().request("engine_getPayloadBodiesByHashV2", (block_hashes,)).await
    }

    async fn get_payload_bodies_by_range_v2(
        &self,
        start: u64,
        count: u64,
    ) -> TransportResult<ExecutionPayloadBodiesV2> {
        self.client()
            .request("engine_getPayloadBodiesByRangeV2", (U64::from(start), U64::from(count)))
            .await
    }

    async fn get_blobs_v1(
        &self,
        versioned_hashes: Vec<B256>,
    ) -> TransportResult<Vec<Option<BlobAndProofV1>>> {
        self.client().request("engine_getBlobsV1", (versioned_hashes,)).await
    }

    async fn get_blobs_v2(
        &self,
        versioned_hashes: Vec<B256>,
    ) -> TransportResult<Vec<Option<BlobAndProofV2>>> {
        self.client().request("engine_getBlobsV2", (versioned_hashes,)).await
    }

    async fn get_client_version_v1(
        &self,
        client_version: ClientVersionV1,
    ) -> TransportResult<Vec<ClientVersionV1>> {
        self.client().request("engine_getClientVersionV1", (client_version,)).await
    }

    async fn exchange_capabilities(
        &self,
        capabilities: Vec<String>,
    ) -> TransportResult<Vec<String>> {
        self.client().request("engine_exchangeCapabilities", (capabilities,)).await
    }
}

#[cfg(test)]
mod eip8288_tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_consensus::{Block, BlockBody, Header, TxEnvelope};
    use alloy_primitives::bytes;
    use alloy_rpc_types_engine::{BlobsBundleV2, RecursiveStarkV1};
    use alloy_transport::mock::Asserter;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn eip8288_methods_encode_wire_parameters() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        let block = Block::<TxEnvelope>::new(Header::default(), BlockBody::default());
        let payload = ExecutionPayloadV5 {
            payload_inner: ExecutionPayloadV4::from_block_unchecked(B256::ZERO, &block),
            dependency_proof: bytes!("1234"),
        };
        let list = InclusionListV1 {
            transactions: vec![bytes!("ff")],
            recursive_stark: RecursiveStarkV1::default(),
        };
        let status = json!({"status": "VALID", "latestValidHash": null,
            "validationError": null, "inclusionListSatisfied": true});
        asserter.push_success(&status);
        let result = provider
            .new_payload_v7(
                payload.clone(),
                vec![B256::ZERO],
                B256::ZERO,
                RequestsOrHash::default(),
                vec![list.clone()],
            )
            .await
            .unwrap();
        assert_eq!(result.inclusion_list_satisfied, Some(true));

        asserter.push_success(&json!({"payloadStatus": status, "payloadId": null}));
        let attrs =
            PayloadAttributesV6 { inclusion_lists: vec![list.clone()], ..Default::default() };
        provider
            .fork_choice_updated_v6(ForkchoiceState::default(), Some(attrs), Some(B128::ZERO))
            .await
            .unwrap();
        asserter.push_success(&json!({"payloadStatus": status, "payloadId": null}));
        provider.fork_choice_updated_v6(ForkchoiceState::default(), None, None).await.unwrap();

        let envelope = ExecutionPayloadEnvelopeV7 {
            execution_payload: payload,
            block_value: Default::default(),
            blobs_bundle: BlobsBundleV2::default(),
            should_override_builder: false,
            execution_requests: Default::default(),
        };
        asserter.push_success(&envelope);
        assert_eq!(provider.get_payload_v7(PayloadId::new([1; 8])).await.unwrap(), envelope);
        asserter.push_success(&json!([null, {"transactions": [], "withdrawals": null,
            "blockAccessList": null, "dependencyProof": "0x"}]));
        let bodies = provider.get_payload_bodies_by_hash_v3(vec![B256::ZERO]).await.unwrap();
        assert!(bodies[0].is_none());
        assert_eq!(bodies[1].as_ref().unwrap().dependency_proof, Some(Bytes::new()));
        asserter.push_success(&json!([]));
        provider.get_payload_bodies_by_range_v3(16, 2).await.unwrap();
        asserter.push_success(&list);
        assert_eq!(provider.get_inclusion_list_v2().await.unwrap(), list);

        let requests = asserter.take_requests();
        let methods: Vec<_> = requests.iter().map(|request| request.method()).collect();
        assert_eq!(
            methods,
            [
                "engine_newPayloadV7",
                "engine_forkchoiceUpdatedV6",
                "engine_forkchoiceUpdatedV6",
                "engine_getPayloadV7",
                "engine_getPayloadBodiesByHashV3",
                "engine_getPayloadBodiesByRangeV3",
                "engine_getInclusionListV2"
            ]
        );
        let params = |index: usize| -> Value {
            serde_json::from_str(requests[index].params().unwrap().get()).unwrap()
        };
        let new_payload = params(0);
        assert_eq!(new_payload.as_array().unwrap().len(), 5);
        assert_eq!(new_payload[0]["dependencyProof"], "0x1234");
        assert!(new_payload[3].is_array());
        assert_eq!(new_payload[4][0]["transactions"], json!(["0xff"]));
        assert_eq!(params(1)[1]["inclusionLists"][0]["recursiveStark"]["starkProof"], "0x");
        assert_eq!(params(1)[2], "0x00000000000000000000000000000000");
        assert_eq!(params(2)[1], Value::Null);
        assert_eq!(params(2)[2], Value::Null);
        assert_eq!(params(3), json!(["0x0101010101010101"]));
        assert_eq!(params(5), json!(["0x10", "0x2"]));
        assert!(requests[6].params().is_none());
    }
}

#[cfg(test)]
mod tests {
    use alloy_rpc_types_engine::CAPABILITIES;

    #[test]
    fn capabilities_list_every_engine_api_method() {
        // `engine_exchangeCapabilities` is how capabilities are exchanged, not one of them.
        let methods = include_str!("engine.rs").split('"').filter(|s| {
            s.len() > "engine_".len()
                && s.starts_with("engine_")
                && *s != "engine_exchangeCapabilities"
        });
        for method in methods {
            assert!(CAPABILITIES.contains(&method), "{method} is missing from `CAPABILITIES`");
        }
    }
}
