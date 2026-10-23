//! JSON adapters for frame transaction requests.

use alloc::{string::String, vec::Vec};
use alloy_eips::eip8141::{FrameAddress, FrameSignature, SignatureMessage, SignatureScheme};
use alloy_primitives::{Bytes, U256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Preserve duplicate-field rejection while buffering JSON for full-width fee normalization.
// Every nested array and object must use this parser, rather than Value's last-key-wins parser.
struct RequestValue(serde_json::Value);

impl<'de> Deserialize<'de> for RequestValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = serde_json::Value;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a JSON value without duplicate object fields")
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| E::invalid_value(serde::de::Unexpected::Float(value), &self))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(serde_json::Value::Null)
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(serde_json::Value::Null)
            }

            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: S,
            ) -> Result<Self::Value, S::Error> {
                let mut values = Vec::new();
                while let Some(RequestValue(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(serde_json::Value::Array(values))
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut object = serde_json::Map::new();
                while let Some((key, RequestValue(value))) = map.next_entry::<String, _>()? {
                    if object.insert(key.clone(), value).is_some() {
                        return Err(serde::de::Error::custom(alloc::format!(
                            "duplicate field `{key}`"
                        )));
                    }
                }
                Ok(serde_json::Value::Object(object))
            }
        }

        deserializer.deserialize_any(ValueVisitor).map(Self)
    }
}

// Keep ordinary request fee fields and their derived representation unchanged. Frame fees
// that do not fit those fields are normalized to the canonical top-level JSON fields.
impl Serialize for super::TransactionRequest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Some(fees) = self.eip8141_fees.as_ref().filter(|_| serializer.is_human_readable())
        else {
            return Self::serialize(self, serializer);
        };

        struct Fields<'a>(&'a super::TransactionRequest);
        impl Serialize for Fields<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                super::TransactionRequest::serialize(self.0, serializer)
            }
        }

        let mut value = serde_json::to_value(Fields(self)).map_err(serde::ser::Error::custom)?;
        if self.max_fee.is_none() {
            value["maxFee"] =
                serde_json::to_value(fees.max_fee).map_err(serde::ser::Error::custom)?;
        }
        if self.max_priority_fee_per_gas.is_none() {
            value["maxPriorityFeePerGas"] = serde_json::to_value(fees.max_priority_fee_per_gas)
                .map_err(serde::ser::Error::custom)?;
        }
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for super::TransactionRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if !deserializer.is_human_readable() {
            return Self::deserialize(deserializer);
        }

        let mut value = RequestValue::deserialize(deserializer)?.0;
        let mut wide_priority = None;
        if let Some(object) = value.as_object_mut() {
            let is_frame = ["maxFee", "frames", "signatures", "nonceKeys"]
                .iter()
                .any(|field| object.contains_key(*field));
            if is_frame {
                if let Some(priority) = object.get("maxPriorityFeePerGas") {
                    let priority: Option<U256> = serde_json::from_value(priority.clone())
                        .map_err(serde::de::Error::custom)?;
                    if let Some(priority) = priority.filter(|fee| *fee > U256::from(u128::MAX)) {
                        object.remove("maxPriorityFeePerGas");
                        wide_priority = Some(priority);
                    }
                }
            }
        }

        let mut request = Self::deserialize(value).map_err(serde::de::Error::custom)?;
        if let Some(max_priority_fee_per_gas) = wide_priority {
            let max_fee =
                request.max_fee.ok_or_else(|| serde::de::Error::missing_field("maxFee"))?;
            request.eip8141_fees =
                Some(alloy_eips::eip8141::TransactionFees { max_fee, max_priority_fee_per_gas });
        }
        Ok(request)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureRequest {
    scheme: SignatureScheme,
    #[serde(default, skip_serializing_if = "is_sender")]
    signer: FrameAddress,
    #[serde(default)]
    msg: SignatureMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signature: Option<Bytes>,
}

const fn is_sender(address: &FrameAddress) -> bool {
    address.address().is_none()
}

pub(super) mod signatures {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(
        value: &Option<Vec<FrameSignature>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .as_ref()
            .map(|signatures| {
                signatures
                    .iter()
                    .map(|signature| SignatureRequest {
                        scheme: signature.scheme,
                        signer: signature.signer,
                        msg: signature.msg,
                        signature: if signature.signature.is_empty()
                            && u8::from(signature.scheme) != 0
                        {
                            None
                        } else {
                            Some(signature.signature.clone())
                        },
                    })
                    .collect::<Vec<_>>()
            })
            .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<FrameSignature>>, D::Error> {
        Option::<Vec<SignatureRequest>>::deserialize(deserializer).map(|signatures| {
            signatures.map(|signatures| {
                signatures
                    .into_iter()
                    .map(|signature| FrameSignature {
                        scheme: signature.scheme,
                        signer: signature.signer,
                        msg: signature.msg,
                        signature: signature.signature.unwrap_or_default(),
                    })
                    .collect()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Transaction, TransactionRequest};
    use alloy_consensus::{transaction::Recovered, TxEip8141, TxEnvelope};
    use alloy_eips::eip8141::{Frame, TransactionFees};
    use alloy_primitives::{Address, U256};
    use serde_json::json;

    #[test]
    fn request_accepts_flat_frames_and_scheme_only_placeholders() {
        let request: TransactionRequest = serde_json::from_value(json!({
            "type": "0x6", "from": Address::repeat_byte(0x11),
            "frames": [{"mode":"0x1", "executionGas":"0x123", "stateGas":"0x45"}],
            "signatures": [{"scheme":"0x1"}]
        }))
        .unwrap();
        let frame = &request.frames.as_ref().unwrap()[0];
        assert_eq!(frame.execution_gas, Some(0x123));
        assert_eq!(frame.state_gas, Some(0x45));
        assert!(frame.target.address().is_none());
        let signature = &request.signatures.as_ref().unwrap()[0];
        assert!(signature.signature.is_empty());
        assert!(signature.signer.address().is_none());
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["frames"][0]["executionGas"], "0x123");
        assert!(value["frames"][0].get("limits").is_none());
        assert!(value["signatures"][0].get("signature").is_none());
        assert!(value["signatures"][0].get("signer").is_none());
    }

    #[test]
    fn request_round_trips_canonical_full_width_fee_fields() {
        let tx = TxEip8141 {
            sender: Address::repeat_byte(1),
            nonce_keys: Some(vec![]),
            frames: vec![Frame::default()],
            fees: TransactionFees { max_fee: U256::MAX, max_priority_fee_per_gas: U256::MAX },
            ..Default::default()
        };
        let request: TransactionRequest = tx.clone().into();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["maxFee"], serde_json::to_value(U256::MAX).unwrap());
        assert_eq!(json["maxPriorityFeePerGas"], serde_json::to_value(U256::MAX).unwrap());
        for field in ["fees", "maxFeePerGas", "maxFeePerBlobGas"] {
            assert!(json.get(field).is_none(), "unexpected {field}");
        }
        let decoded: TransactionRequest = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(decoded, request);
        let rebuilt = decoded.build_8141().unwrap();
        assert_eq!(rebuilt, tx);
        let rebuilt_request: TransactionRequest = rebuilt.into();
        assert_eq!(serde_json::to_value(rebuilt_request).unwrap(), json);

        let mut stored_fees = request.clone();
        stored_fees.max_fee = None;
        assert_eq!(serde_json::to_value(&stored_fees).unwrap(), json);
        assert_eq!(stored_fees.build_8141().unwrap(), tx);

        let mut edited = request;
        edited.max_fee = Some(U256::ONE);
        edited.max_priority_fee_per_gas = Some(1);
        let edited_json = serde_json::to_value(&edited).unwrap();
        assert_eq!(edited_json["maxFee"], "0x1");
        assert_eq!(edited_json["maxPriorityFeePerGas"], "0x1");
        assert!(edited_json.get("fees").is_none());
        assert_eq!(edited.build_8141().unwrap().fees.max_fee, U256::ONE);
        let mut missing_budget = json;
        missing_budget.as_object_mut().unwrap().remove("maxFee");
        assert!(serde_json::from_value::<TransactionRequest>(missing_budget).is_err());
    }

    #[test]
    fn frame_priority_fee_json_preserves_integer_boundaries_and_selects_frames() {
        for priority in [U256::from(u128::MAX), U256::from(u128::MAX) + U256::ONE, U256::MAX] {
            let tx = TxEip8141 {
                frames: vec![Frame::default()],
                fees: TransactionFees { max_fee: U256::MAX, max_priority_fee_per_gas: priority },
                ..Default::default()
            };
            let request: TransactionRequest = tx.clone().into();
            let json = serde_json::to_value(&request).unwrap();
            assert_eq!(json["maxPriorityFeePerGas"], serde_json::to_value(priority).unwrap());
            assert!(json.get("fees").is_none());
            let decoded: TransactionRequest = serde_json::from_value(json).unwrap();
            assert_eq!(decoded.build_8141().unwrap(), tx);
        }

        let request: TransactionRequest = serde_json::from_value(json!({
            "maxFee": serde_json::to_value(U256::MAX).unwrap(),
            "maxPriorityFeePerGas": serde_json::to_value(U256::MAX).unwrap(),
        }))
        .unwrap();
        assert_eq!(request.minimal_tx_type(), alloy_consensus::TxType::Eip8141);
        assert_eq!(request.preferred_type(), alloy_consensus::TxType::Eip8141);
        let budget_only = TransactionRequest { max_fee: Some(U256::ONE), ..Default::default() };
        assert_eq!(budget_only.minimal_tx_type(), alloy_consensus::TxType::Eip8141);
        assert_eq!(budget_only.preferred_type(), alloy_consensus::TxType::Eip8141);
    }

    #[test]
    fn ordinary_priority_fee_width_and_duplicate_field_validation_are_unchanged() {
        let request: TransactionRequest = serde_json::from_value(json!({
            "maxPriorityFeePerGas": "0xffffffffffffffffffffffffffffffff",
            "maxFeePerGas": "0xffffffffffffffffffffffffffffffff",
        }))
        .unwrap();
        assert_eq!(request.max_priority_fee_per_gas, Some(u128::MAX));
        assert!(request.eip8141_fees.is_none());
        assert_eq!(
            serde_json::to_value(&request).unwrap()["maxPriorityFeePerGas"],
            "0xffffffffffffffffffffffffffffffff"
        );
        assert!(serde_json::from_value::<TransactionRequest>(json!({
            "maxPriorityFeePerGas": serde_json::to_value(U256::MAX).unwrap(),
        }))
        .is_err());
        assert!(serde_json::from_str::<TransactionRequest>(
            r#"{"maxPriorityFeePerGas":"0x1","maxPriorityFeePerGas":"0x2"}"#
        )
        .is_err());
    }

    #[test]
    fn request_rejects_duplicate_fields_in_nested_objects() {
        for (duplicate, valid) in [
            (r#"{"frames":[{"mode":"0x0","mode":"0x1"}]}"#, r#"{"frames":[{"mode":"0x1"}]}"#),
            (
                r#"{"signatures":[{"scheme":"0x0","scheme":"0x1","signature":"0x"}]}"#,
                r#"{"signatures":[{"scheme":"0x1","signature":"0x"}]}"#,
            ),
            (
                concat!(
                    r#"{"accessList":[{"address":"0x1111111111111111111111111111111111111111","#,
                    r#""address":"0x2222222222222222222222222222222222222222","storageKeys":[]}]}"#,
                ),
                r#"{"accessList":[{"address":"0x2222222222222222222222222222222222222222","storageKeys":[]}]}"#,
            ),
        ] {
            let error = serde_json::from_str::<TransactionRequest>(duplicate).unwrap_err();
            assert!(error.to_string().contains("duplicate field"), "{error}");
            assert!(serde_json::from_str::<TransactionRequest>(valid).is_ok());
        }
    }

    #[test]
    fn unsigned_frame_fill_preserves_placeholders_without_weakening_signed_validation() {
        let request: TransactionRequest = serde_json::from_value(json!({
            "type": "0x6", "from": Address::repeat_byte(0x11), "chainId": "0x1", "nonce": "0x0",
            "nonceKeys": [],
            "frames": [{"mode":"0x1", "executionGas":"0x123", "stateGas":"0x45"}],
            "signatures": [{"scheme":"0x1"}],
            "maxFee": "0x1", "maxPriorityFeePerGas": "0x0",
            "blobVersionedHashes": []
        }))
        .unwrap();
        assert!(request.clone().build_8141().is_err());
        let mut incomplete = request.clone();
        incomplete.frames.as_mut().unwrap()[0].execution_gas = None;
        assert_eq!(incomplete.complete_8141(), Err(vec!["missing frame executionGas"]));
        let transaction = request.build_typed_simulate_transaction().unwrap();
        let value = serde_json::to_value(&transaction).unwrap();
        assert_eq!(value["signatures"][0]["scheme"], "0x1");
        assert!(value["signatures"][0].get("signature").is_none());
        assert!(value["signatures"][0].get("signer").is_none());
        let decoded: alloy_consensus::EthereumTxEnvelope<alloy_consensus::TxEip4844> =
            serde_json::from_value(value).unwrap();
        assert_eq!(decoded, transaction);
    }

    #[test]
    fn frame_transaction_response_round_trips_with_one_from_field() {
        let sender = Address::repeat_byte(0x11);
        let tx = TxEip8141 { sender, frames: vec![Frame::default()], ..Default::default() };
        let rpc = Transaction {
            inner: Recovered::new_unchecked(
                TxEnvelope::Eip8141(alloy_primitives::Sealed::new_unchecked(
                    tx,
                    Default::default(),
                )),
                sender,
            ),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            effective_gas_price: None,
            block_timestamp: None,
        };
        let encoded = serde_json::to_string(&rpc).unwrap();
        assert_eq!(encoded.matches("\"from\":").count(), 1);
        assert!(!encoded.contains("\"sender\":"));
        let decoded: Transaction = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, rpc);
    }
}
