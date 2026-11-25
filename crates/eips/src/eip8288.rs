//! EIP-8288 dependency declarations. Cryptographic witnesses are carried separately.
//!
//! The `eip8288-proofs` feature enables the experimental pinned leanVM backend and requires
//! Rust 1.97.1. It uses a Keccak dependency digest and native leanVM claims, rather than fixing
//! the draft's still-unspecified protocol hash and verification key. Both wrapper modes retain the
//! draft's 16-signature limit; block aggregates allow up to 16,384 dependencies within bounded
//! proof sizes. Application leanSTARK declarations remain reserved and invalid.

use alloy_eip8141::{Frame, FrameMode};
use alloy_primitives::B256;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use core::slice::ChunksExact;

/// Dependency verification frame mode.
pub const DEP_VERIFY_FRAME_MODE: u8 = 4;
/// Encoded size of a dependency triple.
pub const DEPENDENCY_BYTES: usize = 96;
/// Maximum declarations in one dependency frame.
pub const MAX_DEPENDENCIES_PER_FRAME: usize = 256;
/// leanSPHINCS dependency scheme.
pub const LEANSPHINCS_SCHEME: u8 = 0x10;
/// Reserved leanSTARK dependency scheme; declarations using it are currently invalid.
pub const LEANSTARK_SCHEME: u8 = 0x11;
/// Execution gas charged per leanSPHINCS declaration.
pub const LEANSPHINCS_VERIFICATION_GAS: u64 = 3_000;
/// Intrinsic proof charge per dependency declaration.
pub const LEANSTARK_VERIFICATION_GAS: u64 = 30_000;
/// Mempool aggregation interval in milliseconds.
pub const AGGREGATION_INTERVAL: u64 = 1_000;
/// Public-pool limit on distinct signature dependencies per transaction.
pub const MAX_SIGS_PER_TX: usize = 16;
/// Public-pool limit on distinct STARK dependencies per transaction.
pub const MAX_STARKS_PER_TX: usize = 1;
/// Signature dependency limit per wrapper in the current draft.
pub const MAX_LEANSIG_DEPS_PER_WRAPPER: usize = 16;
/// STARK dependency limit per wrapper in the current draft.
pub const MAX_LEANSTARK_DEPS_PER_WRAPPER: usize = 1;

/// Maximum serialized dependency proof size.
pub const MAX_DEPENDENCY_PROOF_BYTES: usize = 1 << 20;

/// Computes the experimental Keccak digest over canonical message/public-key pairs.
/// The protocol's `DEPS_HASH` and verification key remain unspecified.
pub fn dependency_digest(dependencies: &[SignatureDependency]) -> B256 {
    let mut hash = alloy_primitives::Keccak256::new();
    for dependency in dependencies {
        hash.update(dependency.data_hash);
        hash.update(dependency.verification_key_hash);
    }
    hash.finalize()
}

/// A declaration, or a resolved dependency after replacing a zero message hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, RlpEncodable, RlpDecodable)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SignatureDependency {
    /// Verification scheme; currently only [`LEANSPHINCS_SCHEME`] is accepted.
    pub scheme: u8,
    /// Message digest, with zero denoting the containing transaction's signing hash.
    pub data_hash: B256,
    /// For leanSPHINCS, the public key `PK.seed || PK.root` itself.
    pub verification_key_hash: B256,
}

impl SignatureDependency {
    /// Encodes the padded scheme word followed by the two 32-byte fields.
    pub fn encode(self) -> [u8; DEPENDENCY_BYTES] {
        let mut encoded = [0; DEPENDENCY_BYTES];
        encoded[31] = self.scheme;
        encoded[32..64].copy_from_slice(self.data_hash.as_slice());
        encoded[64..].copy_from_slice(self.verification_key_hash.as_slice());
        encoded
    }

    /// Replaces the zero-message sentinel without changing the transaction's frame bytes.
    pub fn resolve(mut self, signature_hash: B256) -> Self {
        if self.data_hash.is_zero() {
            self.data_hash = signature_hash;
        }
        self
    }
}

/// A malformed or currently unsupported dependency declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DependencyValidationError {
    /// The containing frame list exceeds EIP-8141's bound.
    #[error("too many EIP-8141 frames")]
    FrameCount,
    /// Payload must contain one to 256 complete triples.
    #[error("invalid EIP-8288 dependency payload length: {0}")]
    Length(usize),
    /// The scheme word's leading 31 bytes must be zero.
    #[error("nonzero EIP-8288 scheme padding")]
    Padding,
    /// The scheme is unknown or reserved.
    #[error("unsupported EIP-8288 scheme: {0}")]
    Scheme(u8),
    /// SignatureDependency frames require an absent target and zero value, flags and state gas.
    #[error("invalid EIP-8288 dependency frame fields")]
    FrameFields,
    /// A dependency frame must not be an atomic batch member or terminator.
    #[error("EIP-8288 dependency frame is in an atomic batch")]
    AtomicBatch,
    /// The execution limit must equal the declaration verification charge.
    #[error("invalid EIP-8288 dependency execution gas limit")]
    ExecutionGas,
}

/// Allocation-free parsing of bounded dependency-frame data.
#[derive(Clone, Debug)]
pub struct SignatureDependencies<'a>(ChunksExact<'a, u8>);

impl<'a> SignatureDependencies<'a> {
    /// Checks payload bounds before reading declarations; each item validates its scheme word.
    pub fn new(data: &'a [u8]) -> Result<Self, DependencyValidationError> {
        if data.is_empty()
            || data.len() > DEPENDENCY_BYTES * MAX_DEPENDENCIES_PER_FRAME
            || !data.len().is_multiple_of(DEPENDENCY_BYTES)
        {
            return Err(DependencyValidationError::Length(data.len()));
        }
        Ok(Self(data.chunks_exact(DEPENDENCY_BYTES)))
    }

    /// Checks frame fields and returns its declarations. Consume every item to validate them.
    pub fn from_frame(
        frame: &'a Frame,
        previous_atomic: bool,
    ) -> Result<Self, DependencyValidationError> {
        if frame.mode != FrameMode::DependencyVerify
            || !frame.target.is_empty()
            || !frame.value.is_zero()
            || frame.flags != 0
            || frame.limits.state != 0
        {
            return Err(DependencyValidationError::FrameFields);
        }
        if previous_atomic {
            return Err(DependencyValidationError::AtomicBatch);
        }
        let dependencies = Self::new(&frame.data)?;
        if frame.limits.execution != dependencies.len() as u64 * LEANSPHINCS_VERIFICATION_GAS {
            return Err(DependencyValidationError::ExecutionGas);
        }
        Ok(dependencies)
    }

    /// Validates every remaining declaration, returning the first malformed scheme or padding.
    pub fn validate(mut self) -> Result<(), DependencyValidationError> {
        self.try_for_each(|dependency| dependency.map(|_| ()))
    }
}

impl Iterator for SignatureDependencies<'_> {
    type Item = Result<SignatureDependency, DependencyValidationError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|data| {
            if data[..31].iter().any(|byte| *byte != 0) {
                return Err(DependencyValidationError::Padding);
            }
            if data[31] != LEANSPHINCS_SCHEME {
                return Err(DependencyValidationError::Scheme(data[31]));
            }
            Ok(SignatureDependency {
                scheme: data[31],
                data_hash: B256::from_slice(&data[32..64]),
                verification_key_hash: B256::from_slice(&data[64..]),
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for SignatureDependencies<'_> {}

#[cfg(feature = "eip8288-proofs")]
mod evidence;
#[cfg(feature = "eip8288-proofs")]
mod proofs;
#[cfg(feature = "eip8288-proofs")]
mod wrapper;

#[cfg(feature = "eip8288-proofs")]
pub use evidence::DependencyEvidence;
#[cfg(feature = "eip8288-proofs")]
pub use proofs::{
    prove, verify_proof, ProofError, MAX_AGGREGATE_DEPENDENCIES, MAX_WRAPPER_BYTES, PROFILE,
};
#[cfg(feature = "eip8288-proofs")]
pub use wrapper::DependencyWrapper;

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_eip8141::FrameLimits;
    use alloy_primitives::{Address, Bytes};

    fn declaration() -> SignatureDependency {
        SignatureDependency {
            scheme: LEANSPHINCS_SCHEME,
            data_hash: B256::ZERO,
            verification_key_hash: B256::repeat_byte(7),
        }
    }

    fn frame() -> Frame {
        Frame {
            mode: FrameMode::DependencyVerify,
            limits: FrameLimits { execution: LEANSPHINCS_VERIFICATION_GAS, state: 0 },
            data: Bytes::copy_from_slice(&declaration().encode()),
            ..Default::default()
        }
    }

    #[test]
    fn encoding_and_resolution() {
        let bytes = declaration().encode();
        assert_eq!(SignatureDependencies::new(&bytes).unwrap().next(), Some(Ok(declaration())));
        let hash = B256::repeat_byte(1);
        assert_eq!(declaration().resolve(hash).data_hash, hash);
        assert_eq!(declaration().resolve(hash).resolve(B256::repeat_byte(2)).data_hash, hash);
        assert_eq!(declaration().data_hash, B256::ZERO);
    }

    #[test]
    fn rejects_malformed_and_reserved_declarations() {
        for length in [0, 95, 97, DEPENDENCY_BYTES * (MAX_DEPENDENCIES_PER_FRAME + 1)] {
            assert_eq!(
                SignatureDependencies::new(&alloc::vec![0; length]).unwrap_err(),
                DependencyValidationError::Length(length)
            );
        }
        let mut bytes = declaration().encode();
        bytes[0] = 1;
        assert_eq!(
            SignatureDependencies::new(&bytes).unwrap().next(),
            Some(Err(DependencyValidationError::Padding))
        );
        bytes[0] = 0;
        for scheme in [0, LEANSTARK_SCHEME, 255] {
            bytes[31] = scheme;
            assert_eq!(
                SignatureDependencies::new(&bytes).unwrap().next(),
                Some(Err(DependencyValidationError::Scheme(scheme)))
            );
        }
    }

    #[test]
    fn enforces_frame_fields_and_batch_boundary() {
        let valid = frame();
        assert_eq!(
            SignatureDependencies::from_frame(&valid, false)
                .and_then(SignatureDependencies::validate),
            Ok(())
        );
        assert_eq!(
            SignatureDependencies::from_frame(&valid, true)
                .and_then(SignatureDependencies::validate),
            Err(DependencyValidationError::AtomicBatch)
        );
        let mut invalid = valid.clone();
        invalid.target = Address::ZERO.into();
        assert_eq!(
            SignatureDependencies::from_frame(&invalid, false)
                .and_then(SignatureDependencies::validate),
            Err(DependencyValidationError::FrameFields)
        );
        for mutate in [
            |frame: &mut Frame| frame.flags = 1,
            |frame: &mut Frame| frame.value = alloy_primitives::U256::from(1),
            |frame: &mut Frame| frame.limits.state = 1,
            |frame: &mut Frame| frame.mode = FrameMode::Default,
        ] {
            let mut invalid = valid.clone();
            mutate(&mut invalid);
            assert_eq!(
                SignatureDependencies::from_frame(&invalid, false)
                    .and_then(SignatureDependencies::validate),
                Err(DependencyValidationError::FrameFields)
            );
        }
        let mut invalid = valid;
        invalid.limits.execution += 1;
        assert_eq!(
            SignatureDependencies::from_frame(&invalid, false)
                .and_then(SignatureDependencies::validate),
            Err(DependencyValidationError::ExecutionGas)
        );
    }

    #[test]
    fn validation_rejects_malformed_trailing_declarations() {
        let mut bytes = declaration().encode().repeat(2);
        bytes[DEPENDENCY_BYTES + 31] = LEANSTARK_SCHEME;
        assert_eq!(
            SignatureDependencies::new(&bytes).unwrap().validate(),
            Err(DependencyValidationError::Scheme(LEANSTARK_SCHEME))
        );
        bytes[DEPENDENCY_BYTES + 31] = LEANSPHINCS_SCHEME;
        bytes[DEPENDENCY_BYTES] = 1;
        assert_eq!(
            SignatureDependencies::new(&bytes).unwrap().validate(),
            Err(DependencyValidationError::Padding)
        );
    }

    #[test]
    fn accepts_maximum_frame_and_charges_duplicates() {
        let mut frame = frame();
        frame.data = declaration().encode().repeat(MAX_DEPENDENCIES_PER_FRAME).into();
        frame.limits.execution = MAX_DEPENDENCIES_PER_FRAME as u64 * LEANSPHINCS_VERIFICATION_GAS;
        assert_eq!(
            SignatureDependencies::from_frame(&frame, false)
                .and_then(SignatureDependencies::validate),
            Ok(())
        );
        assert_eq!(
            SignatureDependencies::new(&frame.data).unwrap().len(),
            MAX_DEPENDENCIES_PER_FRAME
        );
    }
}
