//! [EIP-8253] zero-nonce account transition.
//!
//! [EIP-8253]: https://eips.ethereum.org/EIPS/eip-8253

use alloy_primitives::{address, Address};

/// Mainnet accounts whose nonce is set to one by EIP-8253.
///
/// Source: <https://github.com/ethereum/EIPs/blob/573ff960/assets/eip-8253/targeted-accounts.json>.
pub const MAINNET_ACCOUNTS: [Address; 28] = [
    address!("f468bcbc4a0bfdb06336e773382c5202e674db71"),
    address!("d8253352f6044cfe55bcc0748c3fa37b7df81f98"),
    address!("5983c6ac846dcf85fbbc4303f43eb91c379f79ae"),
    address!("de425ad4b8d2d9e0e12f65cbcd6d55f447b44083"),
    address!("50b1497068bae652df3562eb8ea7677ff84477fa"),
    address!("8398ff6c618e9515468c1c4b198d53666cbe8462"),
    address!("6f156dbf8ed30e53f7c9df73144e69f65cbb7e94"),
    address!("2c081ed1949d7dd9447f9d96e509befe576d4461"),
    address!("db7c577b93baeb56dab50af4d6f86f99a06b96a2"),
    address!("14725085d004f1b10ee07234a4ab28c5ad2a7b9e"),
    address!("ae3703584494ade958ad27ec2d289b7a67c19e90"),
    address!("7d6ae067de8d44ae1a08750e7d626d61a623c44a"),
    address!("4d149eb99bdeefc1f858f8fd22289c6beae99f2c"),
    address!("361d7a60b43587c7f6bba4f9fd9642747f65210a"),
    address!("b619f45637c39ca49a41ac64c11637a0a194455e"),
    address!("5071cb62aa170b7f66b26cae8004d90e6078bb1e"),
    address!("add92e0650457c5db0c4c08cbf7ca580175d33d2"),
    address!("3311c08066580cb906a7287b6786e504c2ebd09f"),
    address!("02820e4bee488c40f7455fdca53125565148708f"),
    address!("e62dc49c92fa799033644d2a9afd7e3babe5a80a"),
    address!("5cc182fabfb81a056b6080d4200bc5150673d06f"),
    address!("f4a835ec1364809003de3925685f24cd360bdffe"),
    address!("fc4465f84b29a1f8794dc753f41bef1f4b025ed2"),
    address!("40490c9c468622d5c89646d6f3097f8eaf80c411"),
    address!("a21b22389bfc1cd6bc7ba19a4fc96adc3d0fe074"),
    address!("59ec0410867828e3b8c23dd8a29d9796ef523b17"),
    address!("19272418753b90d9a3e3efc8430b1612c55fcb3a"),
    address!("fee7707fa4b8c0a923a0e40399db3e7ce26069c6"),
];
