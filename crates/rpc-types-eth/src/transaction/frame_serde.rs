//! JSON adapters for frame transaction requests.

use alloc::{borrow::Cow, string::String, vec::Vec};
use alloy_eips::eip8141::{FrameAddress, FrameSignature, SignatureMessage, SignatureScheme};
use alloy_primitives::{Bytes, U256};
use core::{fmt, marker::PhantomData};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Keep ordinary request fee fields and their derived representation unchanged. Frame fees
// that do not fit those fields are normalized to the canonical top-level JSON fields.
impl Serialize for super::TransactionRequest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Some(fees) = self.eip8141_fees.as_ref().filter(|_| serializer.is_human_readable())
        else {
            return Self::serialize(self, serializer);
        };

        FrameFeeRequest {
            fields: RequestFields(self),
            max_fee: self.max_fee.is_none().then_some(fees.max_fee),
            max_priority_fee_per_gas: self
                .max_priority_fee_per_gas
                .is_none()
                .then_some(fees.max_priority_fee_per_gas),
        }
        .serialize(serializer)
    }
}

struct RequestFields<'a>(&'a super::TransactionRequest);

impl Serialize for RequestFields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        super::TransactionRequest::serialize(self.0, serializer)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameFeeRequest<'a> {
    #[serde(flatten)]
    fields: RequestFields<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_fee: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_priority_fee_per_gas: Option<U256>,
}

impl<'de> Deserialize<'de> for super::TransactionRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if !deserializer.is_human_readable() {
            return Self::deserialize(deserializer);
        }

        deserializer.deserialize_map(RequestVisitor)
    }
}

// Intercept only the priority fee. All other values go straight to the derived request
// deserializer, retaining its duplicate checks, flattening, and borrowed input handling.
struct RequestVisitor;

impl<'de> serde::de::Visitor<'de> for RequestVisitor {
    type Value = super::TransactionRequest;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a transaction request object")
    }

    fn visit_map<M: serde::de::MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
        let mut wide_priority = None;
        let mut request =
            super::TransactionRequest::deserialize(serde::de::value::MapAccessDeserializer::new(
                RequestMap { map, wide_priority: &mut wide_priority, priority_field: false },
            ))?;
        if let Some(max_priority_fee_per_gas) = wide_priority {
            if request.max_fee.is_none()
                && request.frames.is_none()
                && request.signatures.is_none()
                && request.nonce_keys.is_none()
            {
                return Err(serde::de::Error::custom(
                    "maxPriorityFeePerGas exceeds u128 for an ordinary transaction",
                ));
            }
            let max_fee =
                request.max_fee.ok_or_else(|| serde::de::Error::missing_field("maxFee"))?;
            request.eip8141_fees =
                Some(alloy_eips::eip8141::TransactionFees { max_fee, max_priority_fee_per_gas });
        }
        Ok(request)
    }
}

struct RequestMap<'a, M> {
    map: M,
    wide_priority: &'a mut Option<U256>,
    priority_field: bool,
}

impl<'de, M: serde::de::MapAccess<'de>> serde::de::MapAccess<'de> for RequestMap<'_, M> {
    type Error = M::Error;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        let Some(RequestKey(key)) = self.map.next_key()? else { return Ok(None) };
        self.priority_field = key == "maxPriorityFeePerGas";
        match key {
            Cow::Borrowed(key) => {
                seed.deserialize(serde::de::value::BorrowedStrDeserializer::new(key)).map(Some)
            }
            Cow::Owned(key) => {
                seed.deserialize(serde::de::value::StringDeserializer::new(key)).map(Some)
            }
        }
    }

    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        if !self.priority_field {
            return self.map.next_value_seed(seed);
        }
        let priority = self.map.next_value::<Option<U256>>()?;
        let value = priority.and_then(|priority| {
            if priority <= U256::from(u128::MAX) {
                Some(priority.to::<u128>())
            } else {
                *self.wide_priority = Some(priority);
                None
            }
        });
        seed.deserialize(NarrowPriority { value, error: PhantomData })
    }

    fn size_hint(&self) -> Option<usize> {
        self.map.size_hint()
    }
}

struct RequestKey<'de>(Cow<'de, str>);

impl<'de> Deserialize<'de> for RequestKey<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;
        impl<'de> serde::de::Visitor<'de> for KeyVisitor {
            type Value = RequestKey<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a transaction request field")
            }

            fn visit_borrowed_str<E: serde::de::Error>(
                self,
                key: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(RequestKey(Cow::Borrowed(key)))
            }

            fn visit_str<E: serde::de::Error>(self, key: &str) -> Result<Self::Value, E> {
                Ok(RequestKey(Cow::Owned(key.into())))
            }

            fn visit_string<E: serde::de::Error>(self, key: String) -> Result<Self::Value, E> {
                Ok(RequestKey(Cow::Owned(key)))
            }
        }

        deserializer.deserialize_identifier(KeyVisitor)
    }
}

struct NarrowPriority<E> {
    value: Option<u128>,
    error: PhantomData<E>,
}

impl<'de, E: serde::de::Error> Deserializer<'de> for NarrowPriority<E> {
    type Error = E;

    fn deserialize_any<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Some(value) => visitor.visit_u128(value),
            None => visitor.visit_unit(),
        }
    }

    fn deserialize_option<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.value {
            Some(value) => visitor.visit_some(serde::de::value::U128Deserializer::new(value)),
            None => visitor.visit_none(),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
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
    fn streaming_priority_fees_preserve_field_order_readers_and_extensions() {
        let priority = alloc::format!("{:#x}", U256::MAX);
        for raw in [
            alloc::format!(
                r#"{{"maxPriorityFeePerGas":"{priority}","maxFee":"{priority}","frames":[],"sourceHash":"0xbeef"}}"#
            ),
            alloc::format!(
                r#"{{"frames":[],"sourceHash":"0xbeef","maxFee":"{priority}","maxPriorityFeePerGas":"{priority}"}}"#
            ),
            alloc::format!(
                r#"{{"maxPriorityFeePer\u0047as":"{priority}","maxFee":"{priority}","frames":[],"sourceHash":"0xbeef"}}"#
            ),
        ] {
            let request: TransactionRequest = serde_json::from_str(&raw).unwrap();
            assert_eq!(request.max_priority_fee_per_gas, None);
            assert_eq!(request.eip8141_fees.as_ref().unwrap().max_priority_fee_per_gas, U256::MAX);
            assert_eq!(
                serde_json::from_slice::<TransactionRequest>(raw.as_bytes()).unwrap(),
                request
            );
            assert_eq!(
                serde_json::from_reader::<_, TransactionRequest>(raw.as_bytes()).unwrap(),
                request
            );
            let wrapped: alloy_serde::WithOtherFields<TransactionRequest> =
                serde_json::from_str(&raw).unwrap();
            assert_eq!(wrapped.inner, request);
            assert_eq!(wrapped.other.get("sourceHash"), Some(&json!("0xbeef")));
        }
    }

    #[test]
    fn streaming_priority_fees_preserve_null_numeric_and_duplicate_validation() {
        for (raw, priority) in [("null", None), ("1", Some(1)), (r#""0x1""#, Some(1))] {
            let raw = alloc::format!(r#"{{"maxPriorityFeePerGas":{raw}}}"#);
            let request: TransactionRequest = serde_json::from_str(&raw).unwrap();
            assert_eq!(request.max_priority_fee_per_gas, priority);
            assert!(request.eip8141_fees.is_none());
        }
        for raw in [
            r#"{"maxFee":"0x1","maxPriorityFeePerGas":"0x100000000000000000000000000000000","maxPriorityFeePerGas":null}"#,
            r#"{"maxFee":"0x1","maxPriorityFeePerGas":null,"maxPriorityFeePerGas":"0x100000000000000000000000000000000"}"#,
            r#"{"maxFee":"0x1","maxPriorityFeePerGas":"0x100000000000000000000000000000000","maxPriorityFeePer\u0047as":"0x1"}"#,
        ] {
            let error = serde_json::from_str::<TransactionRequest>(raw).unwrap_err();
            assert!(error.to_string().contains("duplicate field"), "{error}");
        }
    }

    #[test]
    fn streaming_requests_preserve_large_ordinary_and_frame_calldata() {
        let data = alloy_primitives::Bytes::from(alloc::vec![0xab; 65_536]);
        let ordinary = TransactionRequest::default()
            .input(crate::TransactionInput::new(data.clone()))
            .max_priority_fee_per_gas(1);
        let framed: TransactionRequest = TxEip8141 {
            frames: alloc::vec![Frame { data, ..Default::default() }],
            fees: TransactionFees { max_fee: U256::MAX, max_priority_fee_per_gas: U256::MAX },
            ..Default::default()
        }
        .into();
        for request in [ordinary, framed] {
            let json = serde_json::to_string(&request).unwrap();
            assert_eq!(serde_json::from_str::<TransactionRequest>(&json).unwrap(), request);
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
