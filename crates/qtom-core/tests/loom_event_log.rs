use qtom_core::{
    InMemoryEventLog, LoomEvent, LoomEventError, LoomEventType, ReplayCursor,
};

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
        causation_id: event_id.checked_sub(1),
        correlation_id: 99,
    }
}

#[test]
fn event_log_appends_and_replays_in_order() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("first event should append");
    log.append(event(LoomEventType::RouteDecisionRecorded, 2, 10))
        .expect("second event should append");
    log.append(event(LoomEventType::TaskAssigned, 3, 10))
        .expect("third event should append");

    let replayed: Vec<_> = log.replay(ReplayCursor::start()).collect();

    assert_eq!(
        replayed.iter().map(|event| event.event_id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(log.len(), 3);
}

#[test]
fn event_log_filters_by_type_without_reordering() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");
    log.append(event(LoomEventType::RouteDecisionRecorded, 2, 10))
        .expect("route_decision_recorded should append");
    log.append(event(LoomEventType::TaskCreated, 3, 11))
        .expect("second task_created should append");

    let task_events = log.events_by_type(LoomEventType::TaskCreated);

    assert_eq!(
        task_events
            .iter()
            .map(|event| event.event_id)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
}

#[test]
fn event_log_rejects_duplicate_event_ids() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("first event should append");

    let err = log
        .append(event(LoomEventType::TaskAssigned, 1, 10))
        .expect_err("duplicate event id should fail");

    assert_eq!(err, LoomEventError::DuplicateEventId(1));
}

#[test]
fn event_log_validates_payload_metadata() {
    let mut invalid = event(LoomEventType::TaskCreated, 1, 10);
    invalid.payload_schema.clear();

    let err = InMemoryEventLog::new()
        .append(invalid)
        .expect_err("empty payload schema should fail");

    assert_eq!(err, LoomEventError::MissingPayloadSchema);
}

#[test]
fn replay_cursor_starts_after_seen_events() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("first event should append");
    log.append(event(LoomEventType::RouteDecisionRecorded, 2, 10))
        .expect("second event should append");
    log.append(event(LoomEventType::TaskAssigned, 3, 10))
        .expect("third event should append");

    let replayed: Vec<_> = log.replay(ReplayCursor::after(2)).collect();

    assert_eq!(
        replayed.iter().map(|event| event.event_id).collect::<Vec<_>>(),
        vec![3]
    );
}
