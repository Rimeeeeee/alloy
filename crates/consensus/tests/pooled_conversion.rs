use alloy_consensus::{
    EthereumTxEnvelope, SignableTransaction, Transaction, TxEip1559, TxEip2930, TxEip4844,
    TxEip7702, TxEip8141, TxLegacy,
    transaction::{
        PooledTransactionWithSidecarVariant as Pooled, TxEip4844WithSidecar, TxEip8141WithSidecar,
        TxHashRef,
    },
};
use alloy_eips::{
    eip4844::{Blob, Bytes48},
    eip7594::{BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant, CELLS_PER_EXT_BLOB},
};
use alloy_primitives::{Signature, U256};

#[test]
fn pooled_conversion_preserves_canonical_transactions() {
    let signature = Signature::new(U256::ONE, U256::ONE, false);
    let sidecar = BlobTransactionSidecarEip7594::new(
        vec![Blob::default()],
        vec![Bytes48::default()],
        vec![Bytes48::default(); CELLS_PER_EXT_BLOB],
    );
    let mut transactions: Vec<Pooled> = vec![
        TxLegacy::default().into_signed(signature).into(),
        TxEip2930::default().into_signed(signature).into(),
        TxEip1559::default().into_signed(signature).into(),
        TxEip7702::default().into_signed(signature).into(),
        TxEip4844WithSidecar::from_tx_and_sidecar(
            TxEip4844 {
                blob_versioned_hashes: sidecar.versioned_hashes().collect(),
                ..Default::default()
            },
            BlobTransactionSidecarVariant::Eip7594(sidecar.clone()),
        )
        .into_signed(signature)
        .into(),
        Pooled::try_from(TxEip8141::default()).unwrap(),
    ];
    let frame = TxEip8141 {
        blob_versioned_hashes: sidecar.versioned_hashes().collect(),
        ..Default::default()
    };
    transactions.push(TxEip8141WithSidecar::new(frame.clone(), sidecar).into());
    assert!(
        Pooled::try_from(frame).is_err(),
        "consensus-to-pool must still reject missing sidecars"
    );
    for pooled in transactions {
        let hash = *pooled.tx_hash();
        let frame_fields = pooled.frame_transaction().cloned();
        let canonical: EthereumTxEnvelope<TxEip4844> = pooled.into();
        assert_eq!(*canonical.tx_hash(), hash);
        assert_eq!(canonical.frame_transaction(), frame_fields.as_ref());
    }
}
