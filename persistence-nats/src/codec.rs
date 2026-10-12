//! Versioned instance encoding. JSON records remain readable in every build.
use engine_core::{
    ProcessInstance,
    error::{EngineError, EngineResult},
};
#[cfg(any(feature = "compact-instances", feature = "separate-audit-log", test))]
use serde::Serialize;
use serde::{Deserialize, de::DeserializeOwned};

// Not a valid JSON prefix. The byte after the magic is the envelope version.
const MAGIC: &[u8] = b"\x89BPMNI";
const VERSION: u8 = 1;

fn failure(error: impl std::fmt::Display) -> EngineError {
    EngineError::PersistenceError(format!("Instance codec: {error}"))
}

#[cfg(any(not(feature = "separate-audit-log"), test))]
pub(crate) fn encode_instance(instance: &ProcessInstance) -> EngineResult<Vec<u8>> {
    #[cfg(feature = "compact-instances")]
    {
        encode_compact(instance)
    }
    #[cfg(not(feature = "compact-instances"))]
    {
        serde_json::to_vec(instance).map_err(failure)
    }
}

#[cfg(any(
    all(feature = "compact-instances", not(feature = "separate-audit-log")),
    test
))]
fn encode_compact<T: Serialize>(value: &T) -> EngineResult<Vec<u8>> {
    encode_version(value, VERSION)
}

#[cfg(any(feature = "compact-instances", feature = "separate-audit-log", test))]
fn encode_version<T: Serialize>(value: &T, version: u8) -> EngineResult<Vec<u8>> {
    let mut bytes = CompactBuffer(MAGIC.to_vec());
    bytes.0.push(version);
    // Named fields tolerate additions with serde defaults; readable scalar
    // representations preserve UUID strings in the monitoring JSON view.
    value
        .serialize(
            &mut rmp_serde::Serializer::new(&mut bytes)
                .with_struct_map()
                .with_human_readable(),
        )
        .map_err(failure)?;
    Ok(bytes.0)
}

#[cfg(any(feature = "compact-instances", feature = "separate-audit-log", test))]
struct CompactBuffer(Vec<u8>);

#[cfg(any(feature = "compact-instances", feature = "separate-audit-log", test))]
impl std::io::Write for CompactBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len()
            > engine_core::runtime::MAX_INSTANCE_PAYLOAD_BYTES.saturating_sub(self.0.len())
        {
            return Err(std::io::Error::other(
                "Compact instance exceeds payload limit",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> EngineResult<T> {
    if let Some(envelope) = bytes.strip_prefix(MAGIC) {
        let Some((&version, payload)) = envelope.split_first() else {
            return Err(failure("missing compact envelope version"));
        };
        if version != VERSION {
            return Err(failure(format!(
                "unsupported compact envelope version {version}"
            )));
        }
        let mut cursor = std::io::Cursor::new(payload);
        let value =
            T::deserialize(&mut rmp_serde::Deserializer::new(&mut cursor).with_human_readable())
                .map_err(failure)?;
        if cursor.position() != payload.len() as u64 {
            return Err(failure("trailing bytes in compact envelope"));
        }
        Ok(value)
    } else {
        serde_json::from_slice(bytes).map_err(failure)
    }
}

/// Version 2 records reference an immutable audit snapshot in instance_audit.
#[derive(Debug, Deserialize)]
#[cfg_attr(any(feature = "separate-audit-log", test), derive(serde::Serialize))]
pub(crate) struct DetachedInstance {
    pub instance: ProcessInstance,
    pub audit_key: String,
}

pub(crate) fn detached(bytes: &[u8]) -> bool {
    bytes
        .strip_prefix(MAGIC)
        .is_some_and(|rest| rest.first() == Some(&2))
}

pub(crate) fn decode_instance_record(
    bytes: &[u8],
) -> EngineResult<(ProcessInstance, Option<String>)> {
    if detached(bytes) {
        let mut cursor = std::io::Cursor::new(&bytes[MAGIC.len() + 1..]);
        let record = DetachedInstance::deserialize(
            &mut rmp_serde::Deserializer::new(&mut cursor).with_human_readable(),
        )
        .map_err(failure)?;
        if cursor.position() != (bytes.len() - MAGIC.len() - 1) as u64
            || !record.instance.audit_log.is_empty()
        {
            return Err(failure("invalid detached audit record"));
        }
        // Audit keys are generated from this instance ID and SHA-256 digest.
        let expected_prefix = format!("{}.", record.instance.id);
        let Some(digest) = record.audit_key.strip_prefix(&expected_prefix) else {
            return Err(failure("audit key belongs to a different instance"));
        };
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(failure("invalid audit snapshot digest"));
        }
        Ok((record.instance, Some(record.audit_key)))
    } else {
        Ok((decode(bytes)?, None))
    }
}

pub(crate) fn audit_key(id: uuid::Uuid, bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{id}.{:x}", Sha256::digest(bytes))
}

// Internal prepared payload, never written verbatim to the instance bucket.
const PREPARED: &[u8] = b"BPMNP\0";

#[cfg(any(feature = "separate-audit-log", test))]
pub(crate) fn prepare_detached(instance: &ProcessInstance) -> EngineResult<Vec<u8>> {
    let mut audit = CompactBuffer(Vec::new());
    serde_json::to_writer(&mut audit, &instance.audit_log).map_err(failure)?;
    let audit = audit.0;
    let mut state = instance.clone();
    state.audit_log.clear();
    let record = encode_version(
        &DetachedInstance {
            instance: state,
            audit_key: audit_key(instance.id, &audit),
        },
        2,
    )?;
    let mut payload = PREPARED.to_vec();
    payload.extend_from_slice(&(record.len() as u32).to_be_bytes());
    payload.extend_from_slice(&record);
    payload.extend_from_slice(&audit);
    if payload.len() > engine_core::runtime::MAX_INSTANCE_PAYLOAD_BYTES {
        return Err(failure("Detached instance exceeds payload limit"));
    }
    Ok(payload)
}

pub(crate) fn split_prepared(bytes: &[u8]) -> EngineResult<(&[u8], &[u8])> {
    let Some(rest) = bytes.strip_prefix(PREPARED) else {
        return Err(failure("invalid prepared audit payload"));
    };
    let Some(length) = rest.get(..4) else {
        return Err(failure("truncated prepared audit payload"));
    };
    let length = u32::from_be_bytes(length.try_into().map_err(failure)?) as usize;
    let body = &rest[4..];
    if length > body.len() {
        return Err(failure("truncated detached instance"));
    }
    Ok(body.split_at(length))
}

pub(crate) fn is_prepared(bytes: &[u8]) -> bool {
    bytes.starts_with(PREPARED)
}

pub(crate) fn is_compact(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

pub(crate) fn monitoring_json(bytes: &[u8]) -> EngineResult<String> {
    if is_compact(bytes) {
        serde_json::to_string(&decode::<serde_json::Value>(bytes)?).map_err(failure)
    } else {
        String::from_utf8(bytes.to_vec()).map_err(failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn instance() -> ProcessInstance {
        serde_json::from_value(json!({
            "id": Uuid::new_v4(), "definition_key": Uuid::new_v4(),
            "business_key": "compact", "parent_instance_id": null,
            "state": {"Suspended": {"previous_state": {"CompletedWithError": {"error_code": "E1"}}}},
            "current_node": "task", "audit_log": ["started", "مرحبا"],
            "variables": {"nested": {"array": [null, true, -10, 1.5, "世界"]}, "large": u64::MAX, "numbers": (0..1000).collect::<Vec<_>>()},
            "tokens": {}, "active_tokens": [], "join_barriers": {},
            "multi_instance_state": {}, "compensation_log": [], "outstanding_calls": {},
            "started_at": "2026-10-12T00:00:00Z", "completed_at": null
        })).unwrap()
    }

    #[test]
    fn mixed_formats_preserve_instance_and_monitoring_values() {
        let instance = instance();
        let expected = serde_json::to_value(&instance).unwrap();
        let json = serde_json::to_vec(&instance).unwrap();
        let compact = encode_compact(&instance).unwrap();
        assert!(compact.len() < json.len());
        for bytes in [json, compact] {
            let restored: ProcessInstance = decode(&bytes).unwrap();
            assert_eq!(serde_json::to_value(restored).unwrap(), expected);
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&monitoring_json(&bytes).unwrap())
                    .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn unsupported_and_corrupt_envelopes_are_rejected() {
        let mut invalid = MAGIC.to_vec();
        assert!(decode::<ProcessInstance>(&invalid).is_err());
        invalid.push(99);
        assert!(
            decode::<ProcessInstance>(&invalid)
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
        invalid[MAGIC.len()] = VERSION;
        assert!(decode::<ProcessInstance>(&invalid).is_err());
        let mut truncated = encode_compact(&instance()).unwrap();
        let mut trailing = truncated.clone();
        trailing.push(0);
        assert!(decode::<ProcessInstance>(&trailing).is_err());
        truncated.pop();
        assert!(decode::<ProcessInstance>(&truncated).is_err());
    }

    #[test]
    fn compact_payload_limit_includes_envelope() {
        let oversized = vec![1.25_f64; engine_core::runtime::MAX_INSTANCE_PAYLOAD_BYTES / 8];
        assert!(encode_compact(&oversized).is_err());
    }

    #[test]
    fn detached_records_keep_audit_bytes_out_of_the_document() {
        let instance = instance();
        let prepared = prepare_detached(&instance).unwrap();
        let (record, audit) = split_prepared(&prepared).unwrap();
        let (mut state, key) = decode_instance_record(record).unwrap();
        assert!(state.audit_log.is_empty());
        assert_eq!(key.unwrap(), audit_key(instance.id, audit));
        state.audit_log = serde_json::from_slice(audit).unwrap();
        assert_eq!(
            serde_json::to_value(state).unwrap(),
            serde_json::to_value(instance).unwrap()
        );
        assert!(decode::<ProcessInstance>(record).is_err());
        let mut invalid = record.to_vec();
        invalid.push(0);
        assert!(decode_instance_record(&invalid).is_err());
        assert!(split_prepared(b"BPMNP\0\0\0\0\xff").is_err());
    }

    #[test]
    fn writes_follow_the_feature_setting() {
        let bytes = encode_instance(&instance()).unwrap();
        assert_eq!(
            bytes.starts_with(MAGIC),
            cfg!(feature = "compact-instances")
        );
        assert!(decode::<ProcessInstance>(&bytes).is_ok());
    }
}
