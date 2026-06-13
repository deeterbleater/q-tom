use std::fs;
use std::path::PathBuf;

use qtom_core::{
    InMemoryEventLog, LoomEvent, LoomEventError, LoomEventType, MockTaskLoom, ReplayCursor,
    append_event_log_jsonl, loom_replay_report, read_event_log_jsonl, write_event_log_jsonl,
};

const GOLDEN_MOCK_LOOM_LOG: &str = "tests/fixtures/mock_loom_event_log.jsonl";

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "qtom-{name}-{}-{}.jsonl",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

fn repo_fixture_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path)
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

#[test]
fn jsonl_append_preserves_existing_events() {
    let path = temp_path("append");
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");

    write_event_log_jsonl(&path, log.replay(ReplayCursor::start()))
        .expect("jsonl write should succeed");
    append_event_log_jsonl(&path, &event(LoomEventType::RouteDecisionRecorded, 2, 10))
        .expect("jsonl append should succeed");
    let loaded = read_event_log_jsonl(&path).expect("jsonl read should succeed");
    fs::remove_file(&path).ok();

    let event_ids: Vec<_> = loaded
        .replay(ReplayCursor::start())
        .map(|event| event.event_id)
        .collect();

    assert_eq!(event_ids, vec![1, 2]);
}

#[test]
fn jsonl_append_rejects_duplicate_without_changing_file() {
    let path = temp_path("append-duplicate");
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");

    write_event_log_jsonl(&path, log.replay(ReplayCursor::start()))
        .expect("jsonl write should succeed");
    let before = fs::read_to_string(&path).expect("jsonl fixture should exist");

    let err = append_event_log_jsonl(&path, &event(LoomEventType::TaskCreated, 1, 11))
        .expect_err("duplicate append should fail");
    let after = fs::read_to_string(&path).expect("jsonl fixture should still exist");
    fs::remove_file(&path).ok();

    assert_eq!(err, LoomEventError::DuplicateEventId(1));
    assert_eq!(after, before);
}

#[test]
fn jsonl_round_trip_preserves_full_mock_replay_report() {
    let path = temp_path("mock-replay-report");
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");
    let expected_report =
        loom_replay_report(&output.event_log).expect("mock log should replay before persistence");

    write_event_log_jsonl(&path, output.event_log.replay(ReplayCursor::start()))
        .expect("jsonl write should succeed");
    let loaded = read_event_log_jsonl(&path).expect("jsonl read should succeed");
    fs::remove_file(&path).ok();

    let loaded_report =
        loom_replay_report(&loaded).expect("loaded mock log should replay after persistence");

    assert_eq!(loaded_report, expected_report);
}

#[test]
fn golden_mock_loom_event_log_matches_current_replay_report() {
    let path = repo_fixture_path(GOLDEN_MOCK_LOOM_LOG);
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");
    let temp = temp_path("golden-mock-log");

    write_event_log_jsonl(&temp, output.event_log.replay(ReplayCursor::start()))
        .expect("jsonl write should succeed");
    let generated = fs::read_to_string(&temp).expect("generated jsonl should exist");
    fs::remove_file(&temp).ok();

    let expected = fs::read_to_string(&path).expect("golden mock loom log should exist");
    assert_eq!(generated, expected);

    let loaded = read_event_log_jsonl(&path).expect("golden mock loom log should load");
    let report = loom_replay_report(&loaded).expect("golden mock loom log should replay");

    assert_eq!(report.validation.route_decision_count, 2);
    assert_eq!(report.validation.memory_node_count, 2);
    assert!(
        report
            .projections
            .integration_group
            .contains("integration_group_10 --> integration_report_3000")
    );
}

#[test]
fn corrupted_golden_mock_loom_log_reports_unknown_route_causation() {
    let golden_path = repo_fixture_path(GOLDEN_MOCK_LOOM_LOG);
    let corrupted_path = temp_path("corrupted-mock-log");
    let golden = fs::read_to_string(&golden_path).expect("golden mock loom log should exist");
    let corrupted = golden.replace(
        r#""event_id":101,"event_type":"TaskAssigned","root_task_id":10,"task_id":1000,"parent_task_id":10,"prompt_id":7,"agent_id":10000,"agent_role":"constructor","topology_snapshot_id":null,"payload_schema":"qtom.mock.task_assignment.v1","payload_ref":"inline://task-assignment/1000/10000","occurred_at_ms":2101,"causation_id":100,"#,
        r#""event_id":101,"event_type":"TaskAssigned","root_task_id":10,"task_id":1000,"parent_task_id":10,"prompt_id":7,"agent_id":10000,"agent_role":"constructor","topology_snapshot_id":null,"payload_schema":"qtom.mock.task_assignment.v1","payload_ref":"inline://task-assignment/1000/10000","occurred_at_ms":2101,"causation_id":999,"#,
    );
    assert_ne!(corrupted, golden);
    fs::write(&corrupted_path, corrupted).expect("write corrupted golden fixture");

    let err = read_event_log_jsonl(&corrupted_path)
        .expect_err("corrupted assignment route causation should fail load");
    fs::remove_file(&corrupted_path).ok();

    assert_eq!(err, LoomEventError::UnknownCausationId(999));
}
