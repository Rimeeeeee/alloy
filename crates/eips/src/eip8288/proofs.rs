//! Experimental leanVM proof profile; this does not finalize EIP-8288's protocol constants.
//!
//! The header digest uses Keccak-256 over canonical message/public-key pairs. Verification
//! reconstructs leanVM's exact native claims independently from the block dependencies.

use super::{
    evidence::Witness, DependencyEvidence, DependencyValidationError, RecursiveStark,
    SignatureDependency, LEANSPHINCS_SCHEME,
};
use alloy_primitives::{Bytes, Keccak256, B256};
use lean_vm::{lean_sphincs, AggregateClaims, AggregateProof, OutputClaims};
use std::{
    collections::BTreeMap,
    sync::{Arc, Once},
};

/// Maximum canonical dependencies in an experimental recursive wrapper.
pub const MAX_AGGREGATE_DEPENDENCIES: usize = 16_384;
/// Maximum serialized aggregate proof accepted by this devnet profile.
pub const MAX_PROOF_BYTES: usize = 1_048_576;
/// Maximum encoded wrapper size in the experimental gossip protocol.
pub const MAX_WRAPPER_BYTES: usize = 2_097_152;
/// Pinned program and proof-system profile, separate from the unfinished protocol constants.
pub const PROFILE: &str = "eip8288-leanvm-84780fb42667bc288c0e567c125b7e10479fb568-keccak-v1";

/// A dependency or proof cannot establish the requested claim set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProofError {
    /// A frame declaration is malformed.
    #[error(transparent)]
    Declaration(#[from] DependencyValidationError),
    /// A list is unsorted, duplicated, unsupported, or too large.
    #[error("invalid EIP-8288 dependency list")]
    Dependencies,
    /// Witness count does not match the direct dependency count.
    #[error("EIP-8288 witness count does not match dependencies")]
    WitnessCount,
    /// The witness does not verify its claimed key and message.
    #[error("invalid leanSPHINCS dependency signature")]
    Signature,
    /// The serialized proof is malformed or does not establish the expected claims.
    #[error("invalid EIP-8288 aggregate proof")]
    Proof,
    /// A declared hash differs from the dependencies reconstructed from transactions.
    #[error("EIP-8288 dependency coverage mismatch")]
    Coverage,
    /// A required dependency has no accompanying evidence.
    #[error("missing EIP-8288 dependency evidence")]
    MissingEvidence,
    /// Wrapper bytes are malformed, oversized, or contain an unsupported mode.
    #[error("invalid EIP-8288 wrapper encoding")]
    Wrapper,
    /// The prover could not produce the requested aggregate.
    #[error("EIP-8288 aggregation failed: {0}")]
    Aggregation(#[from] lean_vm::AggregateError),
}

/// Computes the devnet header digest without allocating a concatenation buffer.
pub fn dependency_digest(dependencies: &[SignatureDependency]) -> B256 {
    let mut hash = Keccak256::new();
    for dependency in dependencies {
        hash.update(dependency.data_hash);
        hash.update(dependency.verification_key_hash);
    }
    hash.finalize()
}

/// Checks the header digest and proof against the exact dependencies of the block.
pub fn verify_header(
    dependencies: &[SignatureDependency],
    entry: &RecursiveStark,
) -> Result<(), ProofError> {
    if entry.block_deps_hash != dependency_digest(dependencies) {
        return Err(ProofError::Coverage);
    }
    verify_proof(dependencies, &entry.stark_proof)
}

/// Recursively combines verified evidence, retaining exactly the dependencies selected for output.
/// Call from a blocking proving worker, not an asynchronous executor thread.
pub fn prove(
    dependencies: &[SignatureDependency],
    inputs: &[Arc<DependencyEvidence>],
) -> Result<RecursiveStark, ProofError> {
    validate_dependencies(dependencies)?;
    let block_deps_hash = dependency_digest(dependencies);
    if dependencies.is_empty() {
        return Ok(RecursiveStark { stark_proof: Bytes::new(), block_deps_hash });
    }
    for input in inputs {
        match &input.witness {
            Witness::Aggregate(proof) if input.dependencies == dependencies => {
                input.verify()?;
                return Ok(RecursiveStark { stark_proof: proof.clone(), block_deps_hash });
            }
            _ => {}
        }
    }
    setup();
    let mut raw = BTreeMap::new();
    let mut children: Vec<(AggregateClaims, AggregateProof)> = Vec::new();
    let retained = claims(dependencies);
    for input in inputs {
        input.verify()?;
        match &input.witness {
            Witness::Direct(signatures) => {
                for (dependency, signature) in input.dependencies.iter().zip(signatures) {
                    if dependencies.binary_search(dependency).is_err() {
                        continue;
                    }
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        raw.entry(*dependency)
                    {
                        let signature =
                            signature.as_ref().try_into().map_err(|_| ProofError::Signature)?;
                        entry.insert((
                            lean_sphincs::LeanSphincsPublicKey::from_bytes(
                                &dependency.verification_key_hash.0,
                            ),
                            dependency.data_hash.0,
                            lean_sphincs::LeanSphincsSignature::from_bytes(signature),
                        ));
                    }
                }
            }
            Witness::Aggregate(proof) if !input.dependencies.is_empty() => {
                if children.len() == lean_vm::MAX_RECURSIONS {
                    // Prune before recursively consolidating so unrelated pool claims do not
                    // accumulate across batches of child proofs.
                    let intermediate = AggregateClaims {
                        lean_sphincs: retained
                            .lean_sphincs
                            .iter()
                            .filter(|claim| {
                                children.iter().any(|(claims, _)| {
                                    claims.lean_sphincs.binary_search(claim).is_ok()
                                })
                            })
                            .copied()
                            .collect(),
                        ..Default::default()
                    };
                    let aggregate = lean_vm::aggregate(
                        &children,
                        vec![],
                        vec![],
                        None,
                        OutputClaims::Only(&intermediate),
                        1,
                    )?;
                    children.clear();
                    children.push(aggregate);
                }
                children.push((
                    claims(&input.dependencies),
                    AggregateProof::from_bytes(proof).ok_or(ProofError::Proof)?,
                ));
            }
            Witness::Aggregate(_) => {}
        }
    }
    // Claims already established by children do not need another raw signature verification.
    raw.retain(|dependency, _| {
        !inputs.iter().any(|input| {
            matches!(input.witness, Witness::Aggregate(_))
                && input.dependencies.binary_search(dependency).is_ok()
        })
    });
    let (_, proof) = lean_vm::aggregate(
        &children,
        vec![],
        raw.into_values().collect(),
        None,
        OutputClaims::Only(&retained),
        1,
    )?;
    let stark_proof = Bytes::from(proof.to_bytes());
    if stark_proof.len() > MAX_PROOF_BYTES {
        return Err(ProofError::Proof);
    }
    Ok(RecursiveStark { stark_proof, block_deps_hash })
}

fn setup() {
    static SETUP: Once = Once::new();
    SETUP.call_once(|| lean_vm::setup(0));
}

pub(super) fn validate_dependencies(
    dependencies: &[SignatureDependency],
) -> Result<(), ProofError> {
    if dependencies.len() > MAX_AGGREGATE_DEPENDENCIES
        || dependencies.iter().any(|dependency| dependency.scheme != LEANSPHINCS_SCHEME)
        || dependencies.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ProofError::Dependencies);
    }
    Ok(())
}

fn claims(dependencies: &[SignatureDependency]) -> AggregateClaims {
    let mut lean_sphincs = dependencies
        .iter()
        .map(|dependency| {
            (
                lean_sphincs::LeanSphincsPublicKey::from_bytes(&dependency.verification_key_hash.0),
                dependency.data_hash.0,
            )
        })
        .collect::<Vec<_>>();
    lean_sphincs.sort_unstable();
    AggregateClaims { lean_sphincs, ..Default::default() }
}

pub(super) fn verify_proof(
    dependencies: &[SignatureDependency],
    proof: &[u8],
) -> Result<(), ProofError> {
    validate_dependencies(dependencies)?;
    if proof.len() > MAX_PROOF_BYTES {
        return Err(ProofError::Proof);
    }
    if dependencies.is_empty() {
        return if proof.is_empty() { Ok(()) } else { Err(ProofError::Proof) };
    }
    setup();
    if !AggregateProof::from_bytes(proof).is_some_and(|proof| proof.verify(&claims(dependencies))) {
        return Err(ProofError::Proof);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_malformed_proofs_fail_closed() {
        let empty = prove(&[], &[]).unwrap();
        assert_eq!(verify_header(&[], &empty), Ok(()));
        assert_eq!(dependency_digest(&[]), alloy_primitives::keccak256([]));
        let dependency = SignatureDependency {
            scheme: LEANSPHINCS_SCHEME,
            data_hash: B256::repeat_byte(1),
            verification_key_hash: B256::repeat_byte(2),
        };
        assert_eq!(verify_header(&[dependency], &empty), Err(ProofError::Coverage));
        assert_eq!(
            DependencyEvidence::direct(vec![dependency], vec![]).unwrap_err(),
            ProofError::WitnessCount
        );
        assert_eq!(
            DependencyEvidence::direct(vec![dependency, dependency], vec![]).unwrap_err(),
            ProofError::Dependencies
        );
        assert_eq!(
            DependencyEvidence::aggregate(vec![dependency], Bytes::new()).unwrap_err(),
            ProofError::Proof
        );
    }

    #[test]
    fn real_signatures_recursion_and_discard_preserve_exact_coverage() {
        let seed = [7; lean_sphincs::SECRET_KEY_LEN];
        let key = lean_sphincs::key_gen(&seed);
        let mut dependencies = Vec::new();
        let mut signatures = Vec::new();
        for value in [1, 2] {
            let message = [value; 32];
            dependencies.push(SignatureDependency {
                scheme: LEANSPHINCS_SCHEME,
                data_hash: B256::from(message),
                verification_key_hash: B256::from(key.to_bytes()),
            });
            signatures
                .push(Bytes::copy_from_slice(&lean_sphincs::sign(&seed, &message).to_bytes()));
        }
        let direct =
            Arc::new(DependencyEvidence::direct(dependencies.clone(), signatures.clone()).unwrap());
        assert_eq!(direct.verify(), Ok(()));
        let leaf = prove(&dependencies, &[Arc::clone(&direct)]).unwrap();
        assert_eq!(verify_header(&dependencies, &leaf), Ok(()));
        let child = Arc::new(
            DependencyEvidence::aggregate(dependencies.clone(), leaf.stark_proof).unwrap(),
        );
        let reused = prove(&dependencies, &[Arc::clone(&child)]).unwrap();
        assert_eq!(reused.stark_proof.as_ref(), child.wire_witnesses().1[0].as_ref());
        let retained = &dependencies[..1];
        let root = prove(retained, &[child, direct]).unwrap();
        assert_eq!(verify_header(retained, &root), Ok(()));
        assert_eq!(verify_header(&dependencies, &root), Err(ProofError::Coverage));
        let wrong = RecursiveStark {
            block_deps_hash: dependency_digest(&dependencies),
            stark_proof: root.stark_proof,
        };
        assert_eq!(verify_header(&dependencies, &wrong), Err(ProofError::Proof));
        signatures[0] = Bytes::from(vec![0; lean_sphincs::SIG_SIZE]);
        let forged = DependencyEvidence::direct(dependencies, signatures).unwrap();
        assert_eq!(forged.verify(), Err(ProofError::Signature));
    }
}
