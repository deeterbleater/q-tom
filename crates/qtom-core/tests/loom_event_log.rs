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
        causation_id: None,
        correlation_id: 99,
    }
}

fn caused_by(mut event: LoomEvent, causation_id: u64) -> LoomEvent {
    event.causation_id = Some(causation_id);
    event
}

#[test]
fn event_log_appends_and_replays_in_order() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("first event should append");
    log.append(event(LoomEventType::RouteDecisionRecorded, 2, 10))
        .expect("second event should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
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
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("third event should append");

    let replayed: Vec<_> = log.replay(ReplayCursor::after(2)).collect();

    assert_eq!(
        replayed.iter().map(|event| event.event_id).collect::<Vec<_>>(),
        vec![3]
    );
}

#[test]
fn task_assignment_requires_route_decision_cause() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");

    let err = log
        .append(caused_by(event(LoomEventType::TaskAssigned, 2, 10), 1))
        .expect_err("task_assigned should require route decision causation");

    assert_eq!(
        err,
        LoomEventError::InvalidCausationType {
            event_type: LoomEventType::TaskAssigned,
            causation_id: 1,
            expected: LoomEventType::RouteDecisionRecorded,
            actual: LoomEventType::TaskCreated,
        }
    );
}

#[test]
fn task_assignment_accepts_route_decision_cause() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");
    log.append(event(LoomEventType::RouteDecisionRecorded, 2, 10))
        .expect("route_decision_recorded should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("task_assigned should accept route decision causation");
}

#[test]
fn artifact_ready_requires_artifact_declared_cause() {
    let mut log = InMemoryEventLog::new();

    let err = log
        .append(event(LoomEventType::ArtifactReady, 2, 10))
        .expect_err("artifact_ready should require causation");

    assert_eq!(
        err,
        LoomEventError::MissingRequiredCausation(LoomEventType::ArtifactReady)
    );

    log.append(event(LoomEventType::ArtifactDeclared, 1, 10))
        .expect("artifact_declared should append");
    log.append(caused_by(event(LoomEventType::ArtifactReady, 2, 10), 1))
        .expect("artifact_ready should accept artifact_declared causation");
}

#[test]
fn unknown_causation_id_is_rejected() {
    let mut log = InMemoryEventLog::new();

    let err = log
        .append(caused_by(event(LoomEventType::TaskAssigned, 2, 10), 999))
        .expect_err("unknown causation should fail");

    assert_eq!(err, LoomEventError::UnknownCausationId(999));
}

#[test]
fn task_scoped_events_require_task_id() {
    let mut invalid = event(LoomEventType::TaskCreated, 1, 10);
    invalid.task_id = None;

    let err = InMemoryEventLog::new()
        .append(invalid)
        .expect_err("task_created should require task id");

    assert_eq!(
        err,
        LoomEventError::MissingRequiredField {
            event_type: LoomEventType::TaskCreated,
            field: "task_id",
        }
    );
}

#[test]
fn agent_decommissioned_requires_agent_id() {
    let mut invalid = event(LoomEventType::AgentDecommissioned, 1, 10);
    invalid.agent_id = None;

    let err = InMemoryEventLog::new()
        .append(invalid)
        .expect_err("agent_decommissioned should require agent id");

    assert_eq!(
        err,
        LoomEventError::MissingRequiredField {
            event_type: LoomEventType::AgentDecommissioned,
            field: "agent_id",
        }
    );
}

#[test]
fn topology_committed_requires_snapshot_id() {
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TopologyProposed, 1, 0))
        .expect("topology_proposed should append");

    let mut invalid = caused_by(event(LoomEventType::TopologyCommitted, 2, 0), 1);
    invalid.topology_snapshot_id = None;

    let err = log
        .append(invalid)
        .expect_err("topology_committed should require topology snapshot id");

    assert_eq!(
        err,
        LoomEventError::MissingRequiredField {
            event_type: LoomEventType::TopologyCommitted,
            field: "topology_snapshot_id",
        }
    );
}
