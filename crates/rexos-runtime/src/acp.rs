use anyhow::Context;
use rexos_memory::MemoryStore;

use crate::records::{AcpDeliveryCheckpointRecord, AcpEventRecord};
use crate::{ACP_CHECKPOINTS_KEY_PREFIX, ACP_EVENTS_KEY, ACP_EVENTS_MAX_RECORDS};

fn acp_checkpoints_key(session_id: &str) -> String {
    format!("{ACP_CHECKPOINTS_KEY_PREFIX}{session_id}")
}

pub(crate) fn acp_events_get(memory: &MemoryStore) -> anyhow::Result<Vec<AcpEventRecord>> {
    let raw = memory
        .kv_get(ACP_EVENTS_KEY)
        .context("kv_get acp events")?
        .unwrap_or_else(|| "[]".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn acp_events_set(memory: &MemoryStore, events: &[AcpEventRecord]) -> anyhow::Result<()> {
    let raw = serde_json::to_string(events).context("serialize acp events")?;
    memory
        .kv_set(ACP_EVENTS_KEY, &raw)
        .context("kv_set acp events")?;
    Ok(())
}

pub(crate) fn append_acp_event(memory: &MemoryStore, record: AcpEventRecord) -> anyhow::Result<()> {
    let mut events = acp_events_get(memory)?;
    events.push(record);
    if events.len() > ACP_EVENTS_MAX_RECORDS {
        events.drain(0..(events.len() - ACP_EVENTS_MAX_RECORDS));
    }
    acp_events_set(memory, &events)
}

pub(crate) fn acp_delivery_checkpoints_get(
    memory: &MemoryStore,
    session_id: &str,
) -> anyhow::Result<Vec<AcpDeliveryCheckpointRecord>> {
    let raw = memory
        .kv_get(&acp_checkpoints_key(session_id))
        .context("kv_get acp delivery checkpoints")?
        .unwrap_or_else(|| "[]".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

pub(crate) fn acp_delivery_checkpoints_set(
    memory: &MemoryStore,
    session_id: &str,
    checkpoints: &[AcpDeliveryCheckpointRecord],
) -> anyhow::Result<()> {
    let raw = serde_json::to_string(checkpoints).context("serialize acp delivery checkpoints")?;
    memory
        .kv_set(&acp_checkpoints_key(session_id), &raw)
        .context("kv_set acp delivery checkpoints")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        acp_delivery_checkpoints_get, acp_delivery_checkpoints_set, acp_events_get,
        append_acp_event,
    };
    use crate::records::{AcpDeliveryCheckpointRecord, AcpEventRecord};
    use rexos_kernel::paths::RexosPaths;
    use rexos_memory::MemoryStore;

    fn test_paths() -> RexosPaths {
        let base =
            std::env::temp_dir().join(format!("rexos-runtime-test-{}", uuid::Uuid::new_v4()));
        RexosPaths { base_dir: base }
    }

    #[test]
    fn append_event_keeps_only_recent_records_when_over_cap() {
        let paths = test_paths();
        paths.ensure_dirs().unwrap();
        let memory = MemoryStore::open_or_create(&paths).unwrap();

        for idx in 0..5_005 {
            append_acp_event(
                &memory,
                AcpEventRecord {
                    id: format!("e-{idx}"),
                    session_id: Some("s-1".to_string()),
                    event_type: "demo".to_string(),
                    payload: serde_json::json!({"idx": idx}),
                    created_at: idx,
                },
            )
            .unwrap();
        }

        let events = acp_events_get(&memory).unwrap();
        assert_eq!(events.len(), 5_000);
        assert_eq!(events.first().map(|e| e.id.as_str()), Some("e-5"));
        assert_eq!(events.last().map(|e| e.id.as_str()), Some("e-5004"));
    }

    #[test]
    fn checkpoints_round_trip_for_session() {
        let paths = test_paths();
        paths.ensure_dirs().unwrap();
        let memory = MemoryStore::open_or_create(&paths).unwrap();
        let checkpoints = vec![AcpDeliveryCheckpointRecord {
            channel: "console".to_string(),
            cursor: "c-1".to_string(),
            updated_at: 42,
        }];

        acp_delivery_checkpoints_set(&memory, "session-1", &checkpoints).unwrap();
        let loaded = acp_delivery_checkpoints_get(&memory, "session-1").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].cursor, "c-1");
    }
}
