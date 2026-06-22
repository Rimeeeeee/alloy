//! Roundtrip tests for the external Engine API REST-SSZ vector artifact.

#![cfg(feature = "ssz")]

#[path = "support/ssz_vector_values.rs"]
mod values;

use alloy_eips::eip4895::Withdrawal;
use alloy_rpc_types_engine::{
    ssz_engine_types::{
        BlobCellsAndProofs, BlobEntry, BlobsV1Request, BlobsV1Response, BlobsV2Request,
        BlobsV2Response, BlobsV3Response, BlobsV4Request, BlobsV4Response, BodiesByHashRequest,
        BodiesResponseAmsterdam, BodyEntry, BuiltPayloadAmsterdam, BuiltPayloadCancun,
        BuiltPayloadOsaka, BuiltPayloadParis, BuiltPayloadPrague, BuiltPayloadShanghai,
        ExecutionPayloadAmsterdam, ExecutionPayloadBodyAmsterdam, ExecutionPayloadBodyCancun,
        ExecutionPayloadBodyOsaka, ExecutionPayloadBodyParis, ExecutionPayloadBodyPrague,
        ExecutionPayloadBodyShanghai, ExecutionPayloadCancun, ExecutionPayloadEnvelopeAmsterdam,
        ExecutionPayloadEnvelopeCancun, ExecutionPayloadEnvelopeOsaka,
        ExecutionPayloadEnvelopeParis, ExecutionPayloadEnvelopePrague,
        ExecutionPayloadEnvelopeShanghai, ExecutionPayloadOsaka, ExecutionPayloadParis,
        ExecutionPayloadPrague, ExecutionPayloadShanghai, ForkchoiceUpdateAmsterdam,
        ForkchoiceUpdateCancun, ForkchoiceUpdateOsaka, ForkchoiceUpdateParis,
        ForkchoiceUpdatePrague, ForkchoiceUpdateResponse, ForkchoiceUpdateShanghai,
        PayloadAttributesAmsterdam, PayloadAttributesCancun, PayloadAttributesOsaka,
        PayloadAttributesParis, PayloadAttributesPrague, PayloadAttributesShanghai, PayloadStatus,
    },
    BlobAndProofV1, BlobAndProofV2, BlobsBundleV1, BlobsBundleV2, ForkchoiceState,
};
use ssz::{Decode, Encode};
use std::{
    collections::BTreeSet,
    env, fmt, fs, panic,
    path::{Path, PathBuf},
    thread,
};

const EXPECTED_TOTAL_CASES: usize = 915;
const EXPECTED_JSON_ONLY_CASES: usize = 45;
const EXPECTED_SSZ_CASES: usize = EXPECTED_TOTAL_CASES - EXPECTED_JSON_ONLY_CASES;
const EXPECTED_SEMANTIC_REJECTIONS: usize = 87;

#[test]
#[ignore = "requires the external execution-apis SSZ vector artifact"]
fn execution_api_ssz_vectors_roundtrip() {
    thread::Builder::new()
        .name("engine-ssz-vectors".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(run_all_vectors)
        .expect("failed to spawn SSZ vector test thread")
        .join()
        .unwrap_or_else(|payload| panic::resume_unwind(payload));
}

fn run_all_vectors() {
    let root = vector_root();
    let mut cases = Vec::new();
    discover_cases(&root, &mut cases).unwrap_or_else(|err| panic!("{err}"));
    cases.sort();

    assert_eq!(
        cases.len(),
        EXPECTED_TOTAL_CASES,
        "unexpected vector count under {}",
        root.display()
    );

    let mut failures = Vec::new();
    let mut expected_failures = expected_failures();
    let mut skipped = 0;
    let mut tested = 0;
    let mut rejected = 0;
    let mut yaml_encoded = 0;

    for serialized_path in cases {
        let relative = serialized_path.strip_prefix(&root).expect("case is below vector root");
        let relative_key = path_key(relative);
        let key = vector_key(relative);

        if is_json_only(&key) {
            skipped += 1;
            continue;
        }

        let case_dir = serialized_path.parent().expect("serialized.ssz has a parent");
        for companion in ["value.yaml", "roots.yaml"] {
            if !case_dir.join(companion).is_file() {
                failures
                    .push(format!("{}: missing companion file {companion}", relative.display()));
            }
        }

        let bytes = match fs::read(&serialized_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                failures.push(format!("{}: read failed: {err}", relative.display()));
                continue;
            }
        };

        let value_path = case_dir.join("value.yaml");
        match fs::read_to_string(&value_path) {
            Ok(yaml) => match values::encode_yaml(&key, &yaml) {
                Ok(encoded) if encoded == bytes => yaml_encoded += 1,
                Ok(encoded) => failures.push(format!(
                    "{} [{key}]: YAML encoding {}",
                    relative.display(),
                    byte_mismatch(&bytes, &encoded)
                )),
                Err(err) => failures.push(format!(
                    "{} [{key}]: failed to encode value.yaml: {err}",
                    relative.display()
                )),
            },
            Err(err) => failures
                .push(format!("{} [{key}]: failed to read value.yaml: {err}", relative.display())),
        }

        tested += 1;
        let expected_failure = expected_failures.remove(relative_key.as_str());
        match (roundtrip_vector(&key, &bytes), expected_failure) {
            (Ok(()), true) => failures.push(format!(
                "{} [{key}]: expected semantic rejection, but decoding succeeded",
                relative.display()
            )),
            (Err(VectorError::Decode(_)), true) => rejected += 1,
            (Err(err), _) => failures.push(format!("{} [{key}]: {err}", relative.display())),
            (Ok(()), false) => {}
        }
    }

    assert_eq!(skipped, EXPECTED_JSON_ONLY_CASES, "unexpected JSON-only vector count");
    assert_eq!(tested, EXPECTED_SSZ_CASES, "unexpected tested SSZ vector count");
    assert_eq!(yaml_encoded, EXPECTED_SSZ_CASES, "unexpected YAML encoding match count");
    assert_eq!(rejected, EXPECTED_SEMANTIC_REJECTIONS, "unexpected semantic rejection count");

    if !expected_failures.is_empty() {
        failures.push(format!(
            "expected-failure entries did not match a vector:\n{}",
            expected_failures.into_iter().collect::<Vec<_>>().join("\n")
        ));
    }

    if !failures.is_empty() {
        panic!("{} of {tested} SSZ vector cases failed:\n{}", failures.len(), failures.join("\n"));
    }

    println!(
        "matched {yaml_encoded} YAML encodings; passed {} SSZ roundtrips; confirmed {rejected} semantic rejections; skipped {skipped} JSON-only identity/capability vectors",
        tested - rejected
    );
}

fn expected_failures() -> BTreeSet<&'static str> {
    let failures = include_str!("ssz_expected_failures.yaml")
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        failures.len(),
        EXPECTED_SEMANTIC_REJECTIONS,
        "unexpected expected-failure manifest size"
    );
    failures
}

fn path_key(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn vector_root() -> PathBuf {
    if let Some(path) = env::var_os("SSZ_VECTOR_DIR") {
        let path = PathBuf::from(path);
        return if path.is_absolute() { path } else { workspace_root().join(path) };
    }

    workspace_root().join("ssz_vectors").join("ssz_vectors")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn discover_cases(dir: &Path, cases: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read {} entry: {err}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to inspect {}: {err}", path.display()))?;

        if file_type.is_dir() {
            discover_cases(&path, cases)?;
        } else if entry.file_name() == "serialized.ssz" {
            cases.push(path);
        }
    }

    Ok(())
}

fn vector_key(relative: &Path) -> String {
    let mut components = relative.components();
    let family = components.next().expect("vector path has a family").as_os_str().to_string_lossy();

    if matches!(
        family.as_ref(),
        "BuiltPayload"
            | "ExecutionPayload"
            | "ExecutionPayloadBody"
            | "ExecutionPayloadEnvelope"
            | "ForkchoiceUpdate"
            | "PayloadAttributes"
    ) {
        let fork = components
            .next()
            .expect("fork-specific vector path has a fork")
            .as_os_str()
            .to_string_lossy();
        format!("{family}/{fork}")
    } else {
        family.into_owned()
    }
}

fn is_json_only(key: &str) -> bool {
    matches!(key, "CapabilitiesResponse" | "ClientVersion" | "IdentityResponse")
}

enum VectorError {
    Decode(String),
    Reencode(String),
    Harness(String),
}

impl fmt::Display for VectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(err) => write!(f, "decode failed: {err}"),
            Self::Reencode(err) => write!(f, "re-encode failed: {err}"),
            Self::Harness(err) => err.fmt(f),
        }
    }
}

fn roundtrip<T: Decode + Encode>(bytes: &[u8]) -> Result<(), VectorError> {
    let decoded =
        T::from_ssz_bytes(bytes).map_err(|err| VectorError::Decode(format!("{err:?}")))?;
    let encoded = decoded.as_ssz_bytes();

    if encoded == bytes {
        return Ok(());
    }

    Err(VectorError::Reencode(byte_mismatch(bytes, &encoded)))
}

fn byte_mismatch(expected: &[u8], actual: &[u8]) -> String {
    let first_difference = expected
        .iter()
        .zip(actual.iter())
        .position(|(expected, actual)| expected != actual)
        .unwrap_or(expected.len().min(actual.len()));
    let start = first_difference.saturating_sub(8);
    let expected_end = (first_difference + 9).min(expected.len());
    let actual_end = (first_difference + 9).min(actual.len());

    format!(
        "re-encode mismatch at byte {first_difference}; expected len {}, actual len {}; \
         expected {:02x?}, actual {:02x?}",
        expected.len(),
        actual.len(),
        &expected[start..expected_end],
        &actual[start..actual_end]
    )
}

macro_rules! dispatch {
    ($bytes:expr, $ty:ty) => {
        roundtrip::<$ty>($bytes)
    };
}

fn roundtrip_vector(key: &str, bytes: &[u8]) -> Result<(), VectorError> {
    match key {
        "BlobAndProofV1" => dispatch!(bytes, BlobAndProofV1),
        "BlobAndProofV2" => dispatch!(bytes, BlobAndProofV2),
        "BlobCellsAndProofs" => dispatch!(bytes, BlobCellsAndProofs),
        "BlobsBundleV1" => dispatch!(bytes, BlobsBundleV1),
        "BlobsBundleV2" => dispatch!(bytes, BlobsBundleV2),
        "BlobsV1Request" => dispatch!(bytes, BlobsV1Request),
        "BlobsV1Response" => dispatch!(bytes, BlobsV1Response),
        "BlobsV2Request" => dispatch!(bytes, BlobsV2Request),
        "BlobsV2Response" => dispatch!(bytes, BlobsV2Response),
        "BlobsV3Response" => dispatch!(bytes, BlobsV3Response),
        "BlobsV4Request" => dispatch!(bytes, BlobsV4Request),
        "BlobsV4Response" => dispatch!(bytes, BlobsV4Response),
        "BlobV1Entry" => dispatch!(bytes, BlobEntry<BlobAndProofV1>),
        "BlobV2Entry" => dispatch!(bytes, BlobEntry<BlobAndProofV2>),
        "BlobV4Entry" => dispatch!(bytes, BlobEntry<BlobCellsAndProofs>),
        "BodiesByHashRequest" => dispatch!(bytes, BodiesByHashRequest),
        "BodiesResponse" => dispatch!(bytes, BodiesResponseAmsterdam),
        "BodyEntry" => dispatch!(bytes, BodyEntry<ExecutionPayloadBodyAmsterdam>),
        "BuiltPayload/Amsterdam" => dispatch!(bytes, BuiltPayloadAmsterdam),
        "BuiltPayload/Cancun" => dispatch!(bytes, BuiltPayloadCancun),
        "BuiltPayload/Osaka" => dispatch!(bytes, BuiltPayloadOsaka),
        "BuiltPayload/Paris" => dispatch!(bytes, BuiltPayloadParis),
        "BuiltPayload/Prague" => dispatch!(bytes, BuiltPayloadPrague),
        "BuiltPayload/Shanghai" => dispatch!(bytes, BuiltPayloadShanghai),
        "ExecutionPayload/Amsterdam" => dispatch!(bytes, ExecutionPayloadAmsterdam),
        "ExecutionPayload/Cancun" => dispatch!(bytes, ExecutionPayloadCancun),
        "ExecutionPayload/Osaka" => dispatch!(bytes, ExecutionPayloadOsaka),
        "ExecutionPayload/Paris" => dispatch!(bytes, ExecutionPayloadParis),
        "ExecutionPayload/Prague" => dispatch!(bytes, ExecutionPayloadPrague),
        "ExecutionPayload/Shanghai" => dispatch!(bytes, ExecutionPayloadShanghai),
        "ExecutionPayloadBody/Amsterdam" => dispatch!(bytes, ExecutionPayloadBodyAmsterdam),
        "ExecutionPayloadBody/Cancun" => dispatch!(bytes, ExecutionPayloadBodyCancun),
        "ExecutionPayloadBody/Osaka" => dispatch!(bytes, ExecutionPayloadBodyOsaka),
        "ExecutionPayloadBody/Paris" => dispatch!(bytes, ExecutionPayloadBodyParis),
        "ExecutionPayloadBody/Prague" => dispatch!(bytes, ExecutionPayloadBodyPrague),
        "ExecutionPayloadBody/Shanghai" => dispatch!(bytes, ExecutionPayloadBodyShanghai),
        "ExecutionPayloadEnvelope/Amsterdam" => {
            dispatch!(bytes, ExecutionPayloadEnvelopeAmsterdam)
        }
        "ExecutionPayloadEnvelope/Cancun" => dispatch!(bytes, ExecutionPayloadEnvelopeCancun),
        "ExecutionPayloadEnvelope/Osaka" => dispatch!(bytes, ExecutionPayloadEnvelopeOsaka),
        "ExecutionPayloadEnvelope/Paris" => dispatch!(bytes, ExecutionPayloadEnvelopeParis),
        "ExecutionPayloadEnvelope/Prague" => dispatch!(bytes, ExecutionPayloadEnvelopePrague),
        "ExecutionPayloadEnvelope/Shanghai" => {
            dispatch!(bytes, ExecutionPayloadEnvelopeShanghai)
        }
        "ForkchoiceState" => dispatch!(bytes, ForkchoiceState),
        "ForkchoiceUpdate/Amsterdam" => dispatch!(bytes, ForkchoiceUpdateAmsterdam),
        "ForkchoiceUpdate/Cancun" => dispatch!(bytes, ForkchoiceUpdateCancun),
        "ForkchoiceUpdate/Osaka" => dispatch!(bytes, ForkchoiceUpdateOsaka),
        "ForkchoiceUpdate/Paris" => dispatch!(bytes, ForkchoiceUpdateParis),
        "ForkchoiceUpdate/Prague" => dispatch!(bytes, ForkchoiceUpdatePrague),
        "ForkchoiceUpdate/Shanghai" => dispatch!(bytes, ForkchoiceUpdateShanghai),
        "ForkchoiceUpdateResponse" => dispatch!(bytes, ForkchoiceUpdateResponse),
        "PayloadAttributes/Amsterdam" => dispatch!(bytes, PayloadAttributesAmsterdam),
        "PayloadAttributes/Cancun" => dispatch!(bytes, PayloadAttributesCancun),
        "PayloadAttributes/Osaka" => dispatch!(bytes, PayloadAttributesOsaka),
        "PayloadAttributes/Paris" => dispatch!(bytes, PayloadAttributesParis),
        "PayloadAttributes/Prague" => dispatch!(bytes, PayloadAttributesPrague),
        "PayloadAttributes/Shanghai" => dispatch!(bytes, PayloadAttributesShanghai),
        "PayloadStatus" => dispatch!(bytes, PayloadStatus),
        "Withdrawal" => dispatch!(bytes, Withdrawal),
        _ => Err(VectorError::Harness(format!("unhandled vector family {key}"))),
    }
}
