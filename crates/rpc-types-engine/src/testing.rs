//! Testing namespace types for building a block in a single call.
//!
//! This follows the `testing_buildBlockV1` specification.

use crate::PayloadAttributes;
use alloc::vec::Vec;
use alloy_primitives::{Bytes, B256};

/// Capability string for `testing_buildBlockV1`.
pub const TESTING_BUILD_BLOCK_V1: &str = "testing_buildBlockV1";

/// Request payload for `testing_buildBlockV1`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TestingBuildBlockRequestV1 {
    /// Parent block hash of the block to build.
    pub parent_block_hash: B256,
    /// Payload attributes.
    pub payload_attributes: PayloadAttributes,
    /// Raw signed transactions to force-include in order.
    pub transactions: Vec<Bytes>,
    /// Optional extra data for the block header.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub extra_data: Option<Bytes>,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TestingBuildBlockRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, SeqAccess, Visitor};

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Helper {
            parent_block_hash: B256,
            payload_attributes: PayloadAttributes,
            #[serde(default, deserialize_with = "alloy_serde::null_as_default")]
            transactions: Vec<Bytes>,
            #[serde(default)]
            extra_data: Option<Bytes>,
        }

        struct RequestVisitor;

        impl<'de> Visitor<'de> for RequestVisitor {
            type Value = TestingBuildBlockRequestV1;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(
                    "a testing_buildBlockV1 request object or params tuple \
                     [parentBlockHash, payloadAttributes, transactions, extraData]",
                )
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let helper = <Helper as serde::Deserialize>::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )?;
                Ok(TestingBuildBlockRequestV1 {
                    parent_block_hash: helper.parent_block_hash,
                    payload_attributes: helper.payload_attributes,
                    transactions: helper.transactions,
                    extra_data: helper.extra_data,
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let parent_block_hash =
                    seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let payload_attributes =
                    seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let transactions =
                    seq.next_element::<Option<Vec<Bytes>>>()?.flatten().unwrap_or_default();
                let extra_data = seq.next_element()?;

                Ok(TestingBuildBlockRequestV1 {
                    parent_block_hash,
                    payload_attributes,
                    transactions,
                    extra_data,
                })
            }
        }

        deserializer.deserialize_any(RequestVisitor)
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::TestingBuildBlockRequestV1;
    use alloy_primitives::{b256, bytes};

    fn expected_request() -> TestingBuildBlockRequestV1 {
        TestingBuildBlockRequestV1 {
            parent_block_hash: b256!(
                "e27a3e81bd7cfe2aec2cc9e832c73a17c93e7efcf659cf4b39883b96c48708c2"
            ),
            payload_attributes: serde_json::from_value(serde_json::json!({
                "parentBeaconBlockRoot": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884365149a42212e8822",
                "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "suggestedFeeRecipient": "0x0000000000000000000000000000000000000000",
                "timestamp": "0x1ce",
                "withdrawals": []
            }))
            .unwrap(),
            transactions: Vec::new(),
            extra_data: Some(bytes!("")),
        }
    }

    #[test]
    fn deserialize_testing_build_block_request_v1_from_positional_params_with_empty_transactions() {
        let request: TestingBuildBlockRequestV1 = serde_json::from_value(serde_json::json!([
            "0xe27a3e81bd7cfe2aec2cc9e832c73a17c93e7efcf659cf4b39883b96c48708c2",
            {
                "parentBeaconBlockRoot": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884365149a42212e8822",
                "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "suggestedFeeRecipient": "0x0000000000000000000000000000000000000000",
                "timestamp": "0x1ce",
                "withdrawals": []
            },
            [],
            "0x"
        ]))
        .unwrap();

        assert_eq!(request, expected_request());
    }

    #[test]
    fn deserialize_testing_build_block_request_v1_from_positional_params_with_null_transactions() {
        let request: TestingBuildBlockRequestV1 = serde_json::from_value(serde_json::json!([
            "0xe27a3e81bd7cfe2aec2cc9e832c73a17c93e7efcf659cf4b39883b96c48708c2",
            {
                "parentBeaconBlockRoot": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884365149a42212e8822",
                "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "suggestedFeeRecipient": "0x0000000000000000000000000000000000000000",
                "timestamp": "0x1ce",
                "withdrawals": []
            },
            null,
            "0x"
        ]))
        .unwrap();

        assert_eq!(request, expected_request());
    }

    #[test]
    fn deserialize_testing_build_block_request_v1_from_object_form() {
        let request: TestingBuildBlockRequestV1 = serde_json::from_value(serde_json::json!({
            "parentBlockHash": "0xe27a3e81bd7cfe2aec2cc9e832c73a17c93e7efcf659cf4b39883b96c48708c2",
            "payloadAttributes": {
                "parentBeaconBlockRoot": "0xcf8e0d4e9587369b2301d0790347320302cc0943d5a1884365149a42212e8822",
                "prevRandao": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "suggestedFeeRecipient": "0x0000000000000000000000000000000000000000",
                "timestamp": "0x1ce",
                "withdrawals": []
            },
            "transactions": null,
            "extraData": "0x"
        }))
        .unwrap();

        assert_eq!(request, expected_request());
    }
}
