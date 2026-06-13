use std::fs;

use qtom_core::{
    InMemoryEventLog, LoomEvent, LoomEventError, LoomEventType, ReplayCursor, read_event_log_jsonl,
    write_event_log_jsonl,
};

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "qtom-{name}-{}-{}.jsonl",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

fn event(event_type: LoomEventType, event_id: u64, task_id: u64) -> LoomEvent {
    LoomEvent {
        event_id,
        event_type,
        root_task_id: 1,
        task_id: Some(task_id),
        parent_task_id: None,
        prompt_id: Some(7),
        agent_id: None,
        agent_role: None,
        topology_snapshot_id: Some(3),
        payload_schema: "test.payload.v1".to_string(),
        payload_ref: format!("inline://event/{event_id}"),
        occurred_at_ms: 1_000 + event_id,
        causation_id: None,
        correlation_id: 99,
    }
}

fn caused_by(mut event: LoomEvent, causation_id: u64) -> LoomEvent {
    event.causation_id = Some(causation_id);
    event
}

#[test]
fn jsonl_round_trip_preserves_replay_order() {
    let path = temp_path("round-trip");
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");
    log.append(event(LoomEventType::RouteDecisionRecorded, 2, 10))
        .expect("route_decision_recorded should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("task_assigned should append");

    write_event_log_jsonl(&path, log.replay(ReplayCursor::start()))
        .expect("jsonl write should succeed");
    let loaded = read_event_log_jsonl(&path).expect("jsonl read should succeed");
    fs::remove_file(&path).ok();

    let replayed: Vec<_> = loaded.replay(ReplayCursor::start()).collect();

    assert_eq!(
        replayed
            .iter()
            .map(|event| event.event_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn jsonl_read_validates_loaded_events() {
    let path = temp_path("invalid");
    let bad_lines = [
        r#"{"event_id":1,"event_type":"TaskCreated","root_task_id":1,"task_id":10,"parent_task_id":null,"prompt_id":7,"agent_id":null,"agent_role":null,"topology_snapshot_id":3,"payload_schema":"test.payload.v1","payload_ref":"inline://event/1","occurred_at_ms":1001,"causation_id":null,"correlation_id":99}"#,
        r#"{"event_id":1,"event_type":"TaskCreated","root_task_id":1,"task_id":11,"parent_task_id":null,"prompt_id":7,"agent_id":null,"agent_role":null,"topology_snapshot_id":3,"payload_schema":"test.payload.v1","payload_ref":"inline://event/1","occurred_at_ms":1002,"causation_id":null,"correlation_id":99}"#,
    ]
    .join("\n");
    fs::write(&path, bad_lines).expect("write invalid jsonl fixture");

    let err = read_event_log_jsonl(&path).expect_err("duplicate event id should fail load");
    fs::remove_file(&path).ok();

    assert_eq!(err, LoomEventError::DuplicateEventId(1));
}
