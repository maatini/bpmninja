//! Immutable full checkpoints and byte-splice deltas. Decoding is always available.
use crate::NatsPersistence;
use engine_core::error::{EngineError, EngineResult};
use uuid::Uuid;

pub(crate) const MAX_DEPTH: u8 = 8;
const HEAD_MAGIC: &[u8] = b"\x89BPMNI\x03";
const FULL: &[u8] = b"BC1\0";
const DELTA: &[u8] = b"BD1\0";
const MAX_BYTES: usize = engine_core::runtime::MAX_INSTANCE_PAYLOAD_BYTES;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Head {
    pub id: Uuid,
    pub key: String,
    pub depth: u8,
}

pub(crate) fn failure(error: impl std::fmt::Display) -> EngineError {
    EngineError::PersistenceError(format!("Instance journal: {error}"))
}
pub(crate) fn is_head(bytes: &[u8]) -> bool {
    bytes.starts_with(HEAD_MAGIC)
}
pub(crate) fn key(id: Uuid, bytes: &[u8]) -> String {
    crate::codec::audit_key(id, bytes)
}
fn valid_key(id: Uuid, key: &str) -> bool {
    key.strip_prefix(&format!("{id}."))
        .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub(crate) fn decode_head(bytes: &[u8]) -> EngineResult<Head> {
    let body = bytes
        .strip_prefix(HEAD_MAGIC)
        .ok_or_else(|| failure("invalid head envelope"))?;
    let head: Head = serde_json::from_slice(body).map_err(failure)?;
    if head.depth > MAX_DEPTH || !valid_key(head.id, &head.key) {
        return Err(failure("invalid journal head"));
    }
    Ok(head)
}
#[cfg(any(feature = "delta-instances", test))]
pub(crate) fn encode_head(head: &Head) -> EngineResult<Vec<u8>> {
    let mut bytes = HEAD_MAGIC.to_vec();
    bytes.extend_from_slice(&serde_json::to_vec(head).map_err(failure)?);
    Ok(bytes)
}

pub(crate) enum Node {
    Full(Vec<u8>),
    Delta {
        parent: String,
        prefix: usize,
        suffix: usize,
        middle: Vec<u8>,
    },
}
impl Node {
    pub(crate) fn decode(bytes: &[u8], id: Uuid) -> EngineResult<Self> {
        if bytes.len() > MAX_BYTES + FULL.len() {
            return Err(failure("oversized journal node"));
        }
        if let Some(full) = bytes.strip_prefix(FULL) {
            return Ok(Self::Full(full.to_vec()));
        }
        let delta = bytes
            .strip_prefix(DELTA)
            .ok_or_else(|| failure("unknown journal node"))?;
        if delta.len() < 10 {
            return Err(failure("truncated delta header"));
        }
        let prefix = u32::from_be_bytes(delta[..4].try_into().map_err(failure)?) as usize;
        let suffix = u32::from_be_bytes(delta[4..8].try_into().map_err(failure)?) as usize;
        let length = u16::from_be_bytes(delta[8..10].try_into().map_err(failure)?) as usize;
        let parent = delta
            .get(10..10 + length)
            .ok_or_else(|| failure("truncated parent key"))?;
        let parent = std::str::from_utf8(parent).map_err(failure)?.to_string();
        if !valid_key(id, &parent) {
            return Err(failure("invalid delta parent key"));
        }
        Ok(Self::Delta {
            parent,
            prefix,
            suffix,
            middle: delta[10 + length..].to_vec(),
        })
    }
    pub(crate) fn apply(self, base: &[u8]) -> EngineResult<Vec<u8>> {
        let Self::Delta {
            prefix,
            suffix,
            middle,
            ..
        } = self
        else {
            return Err(failure("expected delta"));
        };
        let kept = prefix
            .checked_add(suffix)
            .ok_or_else(|| failure("delta bounds overflow"))?;
        let total = kept
            .checked_add(middle.len())
            .ok_or_else(|| failure("delta length overflow"))?;
        if kept > base.len() || total > MAX_BYTES {
            return Err(failure("delta outside snapshot bounds"));
        }
        let mut output = Vec::with_capacity(total);
        output.extend_from_slice(&base[..prefix]);
        output.extend_from_slice(&middle);
        output.extend_from_slice(&base[base.len() - suffix..]);
        Ok(output)
    }
}

#[cfg(any(feature = "delta-instances", test))]
pub(crate) fn choose_node(
    snapshot: &[u8],
    previous: Option<(&Head, &[u8])>,
) -> EngineResult<(Vec<u8>, u8)> {
    if snapshot.len() > MAX_BYTES {
        return Err(failure("snapshot exceeds size limit"));
    }
    let full_size = FULL.len() + snapshot.len();
    if let Some((head, base)) = previous
        && head.depth < MAX_DEPTH
    {
        let prefix = snapshot
            .iter()
            .zip(base)
            .take_while(|(a, b)| a == b)
            .count();
        let remaining = snapshot.len().min(base.len()) - prefix;
        let suffix = snapshot
            .iter()
            .rev()
            .zip(base.iter().rev())
            .take(remaining)
            .take_while(|(a, b)| a == b)
            .count();
        let mut delta = DELTA.to_vec();
        delta.extend_from_slice(&(prefix as u32).to_be_bytes());
        delta.extend_from_slice(&(suffix as u32).to_be_bytes());
        delta.extend_from_slice(&(head.key.len() as u16).to_be_bytes());
        delta.extend_from_slice(head.key.as_bytes());
        delta.extend_from_slice(&snapshot[prefix..snapshot.len() - suffix]);
        if delta.len() < full_size {
            return Ok((delta, head.depth + 1));
        }
    }
    let mut full = FULL.to_vec();
    full.extend_from_slice(snapshot);
    Ok((full, 0))
}

pub(crate) async fn lock_snapshot(id: Uuid) -> tokio::sync::MutexGuard<'static, ()> {
    static LOCKS: std::sync::OnceLock<Vec<tokio::sync::Mutex<()>>> = std::sync::OnceLock::new();
    let locks = LOCKS.get_or_init(|| (0..256).map(|_| tokio::sync::Mutex::new(())).collect());
    locks[(id.as_u128() % 256) as usize].lock().await
}

impl NatsPersistence {
    pub(crate) async fn resolve_snapshot(&self, bytes: &[u8]) -> EngineResult<Vec<u8>> {
        if !is_head(bytes) {
            return Ok(bytes.to_vec());
        }
        let head = decode_head(bytes)?;
        let mut next = head.key.clone();
        let mut patches = Vec::new();
        let store = self.kv("instance_states").await?;
        let mut remaining = head.depth;
        let mut full = loop {
            let data = store
                .get(&next)
                .await
                .map_err(failure)?
                .ok_or_else(|| failure(format!("missing journal node {next}")))?;
            if key(head.id, &data) != next {
                return Err(failure("journal node digest mismatch"));
            }
            match Node::decode(&data, head.id)? {
                Node::Full(bytes) => {
                    if remaining != 0 {
                        return Err(failure("journal depth mismatch"));
                    }
                    break bytes;
                }
                delta @ Node::Delta { .. } => {
                    if remaining == 0 {
                        return Err(failure("journal chain exceeds declared depth"));
                    }
                    if let Node::Delta { parent, .. } = &delta {
                        next = parent.clone();
                    }
                    patches.push(delta);
                    remaining -= 1;
                }
            }
        };
        for delta in patches.into_iter().rev() {
            full = delta.apply(&full)?;
        }
        if crate::codec::decode_instance_record(&full)?.0.id != head.id {
            return Err(failure("journal instance ID mismatch"));
        }
        Ok(full)
    }
}

#[cfg(feature = "delta-instances")]
#[derive(Clone)]
pub(crate) struct CachedSnapshot {
    revision: u64,
    head: Head,
    bytes: Vec<u8>,
}
#[cfg(feature = "delta-instances")]
pub(crate) type SnapshotCache = std::collections::HashMap<(String, Uuid), CachedSnapshot>;

#[cfg(feature = "delta-instances")]
impl NatsPersistence {
    pub(crate) async fn put_journal_snapshot(
        &self,
        bucket: &str,
        id: Uuid,
        snapshot: &[u8],
    ) -> EngineResult<()> {
        if crate::codec::decode_instance_record(snapshot)?.0.id != id {
            return Err(failure("snapshot ID mismatch"));
        }
        let cache_key = (bucket.to_string(), id);
        let store = self.kv(bucket).await?;
        let cached = self.snapshot_cache.lock().await.get(&cache_key).cloned();
        let (revision, previous) = if let Some(cached) = cached {
            (cached.revision, Some(cached))
        } else {
            let entry = store.entry(id.to_string()).await.map_err(failure)?;
            let revision = entry.as_ref().map_or(0, |entry| entry.revision);
            let previous = if let Some(entry) = entry.filter(|entry| {
                entry.operation == async_nats::jetstream::kv::Operation::Put
                    && is_head(&entry.value)
            }) {
                let head = decode_head(&entry.value)?;
                if head.id != id {
                    return Err(failure("stored head ID mismatch"));
                }
                Some(CachedSnapshot {
                    revision,
                    head,
                    bytes: self.resolve_snapshot(&entry.value).await?,
                })
            } else {
                None
            };
            (revision, previous)
        };
        let head = if let Some(previous) = previous
            .as_ref()
            .filter(|previous| previous.bytes == snapshot)
        {
            previous.head.clone()
        } else {
            let (node, depth) = choose_node(
                snapshot,
                previous
                    .as_ref()
                    .map(|cached| (&cached.head, cached.bytes.as_slice())),
            )?;
            let key = key(id, &node);
            let nodes = self.kv("instance_states").await?;
            if let Err(error) = nodes.create(&key, node.clone().into()).await
                && nodes.get(&key).await.map_err(failure)?.as_deref() != Some(node.as_slice())
            {
                return Err(failure(error));
            }
            Head { id, key, depth }
        };
        // The immutable node is acknowledged before publishing its head.
        let updated = store
            .update(id.to_string(), encode_head(&head)?.into(), revision)
            .await;
        let revision = match updated {
            Ok(revision) => revision,
            Err(error) => {
                self.snapshot_cache.lock().await.remove(&cache_key);
                return Err(failure(error));
            }
        };
        let mut cache = self.snapshot_cache.lock().await;
        if cache.len() >= 32 {
            cache.clear();
        }
        cache.insert(
            cache_key,
            CachedSnapshot {
                revision,
                head,
                bytes: snapshot.to_vec(),
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splice_roundtrips_and_checkpoints_when_needed() {
        let id = Uuid::new_v4();
        let base = vec![b'a'; 1000];
        let (checkpoint, depth) = choose_node(&base, None).unwrap();
        assert_eq!(depth, 0);
        let head = Head {
            id,
            key: key(id, &checkpoint),
            depth,
        };
        for target in [
            base.clone(),
            [base.clone(), "世界".as_bytes().to_vec()].concat(),
            {
                let mut target = base.clone();
                target[500] = b'b';
                target
            },
            base[..500].to_vec(),
        ] {
            let (bytes, depth) = choose_node(&target, Some((&head, &base))).unwrap();
            match Node::decode(&bytes, id).unwrap() {
                Node::Full(bytes) => assert_eq!(bytes, target),
                delta => assert_eq!(delta.apply(&base).unwrap(), target),
            }
            assert!(depth <= MAX_DEPTH);
        }
        let exhausted = Head {
            depth: MAX_DEPTH,
            ..head
        };
        assert_eq!(choose_node(&base, Some((&exhausted, &base))).unwrap().1, 0);
        assert_eq!(
            choose_node(b"short", Some((&exhausted, &base))).unwrap().1,
            0
        );
    }
    #[test]
    fn malformed_and_cross_instance_deltas_are_rejected() {
        let id = Uuid::new_v4();
        let other = Uuid::new_v4();
        let base = vec![b'a'; 1000];
        let head = Head {
            id,
            key: key(id, &base),
            depth: 0,
        };
        let mut target = base.clone();
        target[500] = b'b';
        let (node, _) = choose_node(&target, Some((&head, &base))).unwrap();
        assert!(Node::decode(&node, other).is_err());
        assert!(Node::decode(b"BD1\0short", id).is_err());
        assert!(Node::decode(&node, id).unwrap().apply(b"tiny").is_err());
        let bad = Head {
            depth: MAX_DEPTH + 1,
            ..head
        };
        assert!(decode_head(&encode_head(&bad).unwrap()).is_err());
    }
}
