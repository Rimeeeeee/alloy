//! [EIP-8272] constants.
//!
//! [EIP-8272]: https://eips.ethereum.org/EIPS/eip-8272
//!
//! # Provisional runtime
//!
//! EIP-8272 currently leaves `RECENT_ROOT_CODE` as `TBD`. [`RECENT_ROOT_CODE`] and
//! [`RECENT_ROOT_CODE_HASH`] therefore pin a provisional 320-byte two-operation runtime,
//! not a normative EIP bytecode. Its write path follows the pending
//! [`ethereum/sys-asm#53`] candidate; the complete validator runtime remains provisional.
//!
//! [`ethereum/sys-asm#53`]: https://github.com/ethereum/sys-asm/pull/53

use alloy_primitives::{address, b256, bytes, Address, Bytes, B256};

/// Address of the ordinary EIP-8272 recent-root contract.
pub const RECENT_ROOT_ADDRESS: Address = address!("0x8272d9679689ea2f307140cdf9002d27dc00ffff");

/// Number of slots retained by each recent-root source's ring buffer.
pub const RECENT_ROOT_LENGTH: u64 = 8192;

/// Largest permitted age, in slots, of a recent-root reference.
pub const RECENT_ROOT_USABLE_WINDOW: u64 = RECENT_ROOT_LENGTH - 1;

/// Maximum number of recent-root tuples in one verifier frame.
pub const MAX_RECENT_ROOT_REFERENCES: usize = 16;

/// Size, in bytes, of one `(source_id, slot, root)` tuple.
pub const RECENT_ROOT_TUPLE_BYTES: usize = 72;

/// Size, in bytes, of the `salt || root` write calldata.
pub const RECENT_ROOT_WRITE_CALLDATA_BYTES: usize = 64;

/// Domain used when deriving a committed recent-root entry hash.
pub const RECENT_ROOT_ENTRY_DOMAIN: B256 =
    b256!("8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc");

/// Domain used when deriving a recent-root storage key.
pub const RECENT_ROOT_STORAGE_DOMAIN: B256 =
    b256!("bdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa");

/// Provisional EIP-8272 recent-root runtime.
///
/// It dispatches by calldata length: 64-byte `salt || root` calls write a root, and one to
/// sixteen packed 72-byte tuples validate roots. See this module's documentation for why this
/// is not yet a normative EIP constant.
pub static RECENT_ROOT_CODE: Bytes = bytes!(
    "346100ba57366040146100c05736604836066100ba5780156100ba5761048081116100ba574b60005b602081013560c01c828110156100ba5780830361200011156100ba577f8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc60005260488260203760686000207fbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa60005290611fff1660c01b60405260486000205414156100ba5760480182811061002857005b60006000fd5b33600052602060006020376034600c20807f8f42481679c8e6fefa040974b3c905e0ce3f2e464ba93acdb074a41181617efc6040524b606852606052602060206088376068604020817fbdc897da2177d260ff5f4be5d4b2aad43f89c3347a305b584fa5a2546d053daa60a852611fff4b1660d05260c852604860a8205500"
);

/// Keccak-256 hash of the provisional [`RECENT_ROOT_CODE`].
pub const RECENT_ROOT_CODE_HASH: B256 =
    b256!("da160390a838ee04013b2ff3abf4decc9aa3cc6c2f59dd90ca176c2b850be4e3");

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::keccak256;

    #[test]
    fn provisional_runtime_is_pinned() {
        assert_eq!(RECENT_ROOT_CODE.len(), 320);
        assert_eq!(keccak256(RECENT_ROOT_CODE.as_ref()), RECENT_ROOT_CODE_HASH);
    }

    #[test]
    fn domains_match_their_preimages() {
        assert_eq!(keccak256(b"RECENT_ROOT_ENTRY"), RECENT_ROOT_ENTRY_DOMAIN);
        assert_eq!(keccak256(b"RECENT_ROOT_STORAGE"), RECENT_ROOT_STORAGE_DOMAIN);
    }
}
