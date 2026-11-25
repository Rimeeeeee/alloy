//! Bounded, immutable evidence shared by transactions from the same wrapper.

use super::{
    proofs::{validate_dependencies, verify_proof, ProofError},
    SignatureDependency, MAX_DEPENDENCY_PROOF_BYTES, MAX_LEANSIG_DEPS_PER_WRAPPER,
};
use alloy_primitives::Bytes;
use lean_vm::lean_sphincs;
use std::sync::OnceLock;

/// Immutable wrapper evidence. Validation is shared across all transactions in that wrapper.
#[derive(Debug)]
pub struct DependencyEvidence {
    pub(super) dependencies: Vec<SignatureDependency>,
    pub(super) witness: Witness,
    verified: OnceLock<Result<(), ProofError>>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Witness {
    Direct(Vec<Bytes>),
    Aggregate(Bytes),
}

impl PartialEq for DependencyEvidence {
    fn eq(&self, other: &Self) -> bool {
        self.dependencies == other.dependencies && self.witness == other.witness
    }
}

impl Eq for DependencyEvidence {}

impl DependencyEvidence {
    /// Constructs bounded direct evidence. Signature validity is checked by [`Self::verify`].
    pub fn direct(
        dependencies: Vec<SignatureDependency>,
        signatures: Vec<Bytes>,
    ) -> Result<Self, ProofError> {
        validate_dependencies(&dependencies)?;
        if dependencies.len() > MAX_LEANSIG_DEPS_PER_WRAPPER
            || dependencies.len() != signatures.len()
        {
            return Err(ProofError::WitnessCount);
        }
        if signatures.iter().any(|signature| signature.len() != lean_sphincs::SIG_SIZE) {
            return Err(ProofError::Signature);
        }
        Ok(Self { dependencies, witness: Witness::Direct(signatures), verified: OnceLock::new() })
    }

    /// Constructs bounded recursive evidence. Proof validity is checked by [`Self::verify`].
    pub fn aggregate(
        dependencies: Vec<SignatureDependency>,
        proof: Bytes,
    ) -> Result<Self, ProofError> {
        validate_dependencies(&dependencies)?;
        if proof.len() > MAX_DEPENDENCY_PROOF_BYTES || dependencies.is_empty() != proof.is_empty() {
            return Err(ProofError::Proof);
        }
        Ok(Self { dependencies, witness: Witness::Aggregate(proof), verified: OnceLock::new() })
    }

    /// Returns the canonical, resolved dependency list carried by this evidence.
    pub fn dependencies(&self) -> &[SignatureDependency] {
        &self.dependencies
    }

    /// Verifies immutable witnesses once, including when shared by several pool transactions.
    pub fn verify(&self) -> Result<(), ProofError> {
        *self.verified.get_or_init(|| match &self.witness {
            Witness::Direct(signatures) => {
                for (dependency, signature) in self.dependencies.iter().zip(signatures) {
                    let signature =
                        signature.as_ref().try_into().map_err(|_| ProofError::Signature)?;
                    if !lean_sphincs::verify(
                        &lean_sphincs::LeanSphincsPublicKey::from_bytes(
                            &dependency.verification_key_hash.0,
                        ),
                        &dependency.data_hash.0,
                        &lean_sphincs::LeanSphincsSignature::from_bytes(signature),
                    ) {
                        return Err(ProofError::Signature);
                    }
                }
                Ok(())
            }
            Witness::Aggregate(proof) => verify_proof(&self.dependencies, proof),
        })
    }

    /// Checks that this verified wrapper covers every dependency of one transaction.
    pub fn covers(&self, dependencies: &[SignatureDependency]) -> Result<(), ProofError> {
        if dependencies
            .iter()
            .any(|dependency| self.dependencies.binary_search(dependency).is_err())
        {
            return Err(ProofError::Coverage);
        }
        self.verify()
    }

    /// Returns the immutable evidence's retained byte size for conservative pool accounting.
    pub fn size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.dependencies.len() * std::mem::size_of::<SignatureDependency>()
            + match &self.witness {
                Witness::Direct(signatures) => {
                    signatures.iter().map(|signature| signature.len()).sum()
                }
                Witness::Aggregate(proof) => proof.len(),
            }
    }

    /// Returns direct witnesses or the single recursive proof for wrapper serialization.
    pub fn wire_witnesses(&self) -> (u8, &[Bytes]) {
        match &self.witness {
            Witness::Direct(signatures) => (0, signatures),
            Witness::Aggregate(proof) => (1, std::slice::from_ref(proof)),
        }
    }
}
