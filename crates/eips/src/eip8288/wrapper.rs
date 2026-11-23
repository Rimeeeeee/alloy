//! Bounded RLP transport for direct and recursively aggregated dependency witnesses.

use super::{
    DependencyEvidence, ProofError, SignatureDependency, MAX_AGGREGATE_DEPENDENCIES,
    MAX_LEANSIG_DEPS_PER_WRAPPER, MAX_WRAPPER_BYTES,
};
use alloy_primitives::Bytes;
use alloy_rlp::{Decodable, Encodable};
use std::sync::Arc;

/// A wire package of typed transaction bytes (or 32-byte hashes) and shared dependency evidence.
#[derive(Clone, Debug)]
pub struct DependencyWrapper {
    /// EIP-2718 transaction bytes, or a 32-byte reference to a previously announced transaction.
    pub transactions: Vec<Bytes>,
    /// Canonical dependency list and direct witnesses or aggregate proof.
    pub evidence: Arc<DependencyEvidence>,
}

impl DependencyWrapper {
    /// Decodes a bounded RLP `[transactions, mode, [dependencies, witnesses_or_proof]]` package.
    /// Transaction resolution and exact dependency coverage must be checked before admission.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProofError> {
        if bytes.len() > MAX_WRAPPER_BYTES {
            return Err(ProofError::Wrapper);
        }
        let mut bytes = bytes;
        let header = alloy_rlp::Header::decode(&mut bytes).map_err(|_| ProofError::Wrapper)?;
        if !header.list || header.payload_length != bytes.len() {
            return Err(ProofError::Wrapper);
        }
        let transactions = decode_list::<Bytes>(&mut bytes, MAX_AGGREGATE_DEPENDENCIES)?;
        if transactions.iter().any(|transaction| transaction.len() < 32) {
            return Err(ProofError::Wrapper);
        }
        let mode = u8::decode(&mut bytes).map_err(|_| ProofError::Wrapper)?;
        let content = alloy_rlp::Header::decode(&mut bytes).map_err(|_| ProofError::Wrapper)?;
        if !content.list || content.payload_length != bytes.len() {
            return Err(ProofError::Wrapper);
        }
        let dependencies =
            decode_list::<SignatureDependency>(&mut bytes, MAX_LEANSIG_DEPS_PER_WRAPPER)?;
        let evidence = match mode {
            0 => DependencyEvidence::direct(
                dependencies,
                decode_list(&mut bytes, MAX_LEANSIG_DEPS_PER_WRAPPER)?,
            )?,
            1 => DependencyEvidence::aggregate(
                dependencies,
                Bytes::decode(&mut bytes).map_err(|_| ProofError::Wrapper)?,
            )?,
            _ => return Err(ProofError::Wrapper),
        };
        if !bytes.is_empty() {
            return Err(ProofError::Wrapper);
        }
        Ok(Self { transactions, evidence: Arc::new(evidence) })
    }

    /// Encodes the package without changing any transaction's canonical bytes or hash.
    pub fn encode(&self) -> Result<Bytes, ProofError> {
        if self.transactions.len() > MAX_AGGREGATE_DEPENDENCIES
            || self.transactions.iter().any(|transaction| transaction.len() < 32)
            || self.evidence.dependencies.len() > MAX_LEANSIG_DEPS_PER_WRAPPER
        {
            return Err(ProofError::Wrapper);
        }
        let (mode, witnesses) = self.evidence.wire_witnesses();
        let direct_length: usize = witnesses.iter().map(Encodable::length).sum();
        let direct_header = alloy_rlp::Header { list: true, payload_length: direct_length };
        let witness_len =
            if mode == 0 { direct_header.length_with_payload() } else { witnesses[0].length() };
        let content = alloy_rlp::Header {
            list: true,
            payload_length: self.evidence.dependencies.length() + witness_len,
        };
        let outer = alloy_rlp::Header {
            list: true,
            payload_length: self.transactions.length()
                + mode.length()
                + content.length_with_payload(),
        };
        if outer.length_with_payload() > MAX_WRAPPER_BYTES {
            return Err(ProofError::Wrapper);
        }
        let mut bytes = Vec::with_capacity(outer.length_with_payload());
        outer.encode(&mut bytes);
        self.transactions.encode(&mut bytes);
        mode.encode(&mut bytes);
        content.encode(&mut bytes);
        self.evidence.dependencies.encode(&mut bytes);
        if mode == 0 {
            direct_header.encode(&mut bytes);
            for witness in witnesses {
                witness.encode(&mut bytes);
            }
        } else {
            witnesses[0].encode(&mut bytes);
        }
        Ok(bytes.into())
    }
}

fn decode_list<T: Decodable>(bytes: &mut &[u8], limit: usize) -> Result<Vec<T>, ProofError> {
    let header = alloy_rlp::Header::decode(bytes).map_err(|_| ProofError::Wrapper)?;
    if !header.list || header.payload_length > bytes.len() {
        return Err(ProofError::Wrapper);
    }
    let (mut payload, rest) = bytes.split_at(header.payload_length);
    *bytes = rest;
    let mut values = Vec::new();
    while !payload.is_empty() {
        if values.len() == limit {
            return Err(ProofError::Wrapper);
        }
        values.push(T::decode(&mut payload).map_err(|_| ProofError::Wrapper)?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eip8288::LEANSPHINCS_SCHEME;
    use alloy_primitives::B256;

    // Independent wire construction exercises decoding even when encode rejects the wrapper.
    #[derive(alloy_rlp::RlpEncodable)]
    struct WireContent<T: Encodable> {
        dependencies: Vec<SignatureDependency>,
        witnesses: T,
    }

    #[derive(alloy_rlp::RlpEncodable)]
    struct WireWrapper<T: Encodable> {
        transactions: Vec<Bytes>,
        mode: u8,
        content: WireContent<T>,
    }

    #[test]
    fn wrapper_dependency_limit_applies_to_both_modes() {
        for count in [
            MAX_LEANSIG_DEPS_PER_WRAPPER - 1,
            MAX_LEANSIG_DEPS_PER_WRAPPER,
            MAX_LEANSIG_DEPS_PER_WRAPPER + 1,
        ] {
            let dependencies = (0..count)
                .map(|index| SignatureDependency {
                    scheme: LEANSPHINCS_SCHEME,
                    data_hash: B256::repeat_byte(index as u8),
                    verification_key_hash: B256::repeat_byte(1),
                })
                .collect::<Vec<_>>();
            let transactions = vec![Bytes::from(vec![1; 32])];
            let proof = Bytes::from_static(&[1]);
            let direct = alloy_rlp::encode(WireWrapper {
                transactions: transactions.clone(),
                mode: 0,
                content: WireContent {
                    dependencies: dependencies.clone(),
                    witnesses: vec![Bytes::from(vec![0; lean_vm::lean_sphincs::SIG_SIZE]); count],
                },
            });
            let aggregate = alloy_rlp::encode(WireWrapper {
                transactions: transactions.clone(),
                mode: 1,
                content: WireContent {
                    dependencies: dependencies.clone(),
                    witnesses: proof.clone(),
                },
            });
            for encoded in [direct, aggregate] {
                if count <= MAX_LEANSIG_DEPS_PER_WRAPPER {
                    let decoded = DependencyWrapper::decode(&encoded).unwrap();
                    assert_eq!(decoded.evidence.dependencies(), dependencies);
                    assert_eq!(decoded.encode().unwrap(), encoded);
                } else {
                    assert_eq!(
                        DependencyWrapper::decode(&encoded).unwrap_err(),
                        ProofError::Wrapper
                    );
                }
            }
            // Block evidence may exceed the wrapper limit, but must not be sent as a wrapper.
            let wrapper = DependencyWrapper {
                transactions,
                evidence: Arc::new(DependencyEvidence::aggregate(dependencies, proof).unwrap()),
            };
            assert_eq!(wrapper.encode().is_ok(), count <= MAX_LEANSIG_DEPS_PER_WRAPPER);
        }
    }

    #[test]
    fn wrapper_encoding_is_bounded_and_canonical() {
        let wrapper = DependencyWrapper {
            transactions: vec![Bytes::from(vec![1; 32])],
            evidence: Arc::new(DependencyEvidence::aggregate(vec![], Bytes::new()).unwrap()),
        };
        let encoded = wrapper.encode().unwrap();
        let decoded = DependencyWrapper::decode(&encoded).unwrap();
        assert_eq!(decoded.transactions, wrapper.transactions);
        assert_eq!(decoded.evidence, wrapper.evidence);
        assert_eq!(decoded.encode().unwrap(), encoded);
        let mut trailing = encoded.to_vec();
        trailing.push(0);
        assert_eq!(DependencyWrapper::decode(&trailing).unwrap_err(), ProofError::Wrapper);
        assert_eq!(
            DependencyWrapper::decode(&vec![0; MAX_WRAPPER_BYTES + 1]).unwrap_err(),
            ProofError::Wrapper
        );
    }
}
