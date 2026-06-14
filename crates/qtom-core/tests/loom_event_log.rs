use qtom_core::{
    InMemoryEventLog, LoomEvent, LoomEventError, LoomEventType, ReplayCursor, validate_events,
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

fn route_decision_event(event_id: u64, task_id: u64) -> LoomEvent {
    let mut event = event(LoomEventType::RouteDecisionRecorded, event_id, task_id);
    event.payload_schema = "qtom.route_decision.v1".to_string();
    event.payload_ref = format!("inline://route-decision/{event_id}");
    event
}

fn caused_by(mut event: LoomEvent, causation_id: u64) -> LoomEvent {
    event.causation_id = Some(causation_id);
    event
}

fn with_root_task(mut event: LoomEvent, root_task_id: u64) -> LoomEvent {
    event.root_task_id = root_task_id;
    event
}

fn with_correlation(mut event: LoomEvent, correlation_id: u64) -> LoomEvent {
    event.correlation_id = correlation_id;
    event
}

fn with_payload_ref(mut event: LoomEvent, payload_ref: &str) -> LoomEvent {
    event.payload_ref = payload_ref.to_string();
    event
}

fn child_task(mut event: LoomEvent, parent_task_id: u64) -> LoomEvent {
    event.parent_task_id = Some(parent_task_id);
    event
}

#[test]
fn event_log_appends_and_replays_in_order() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("first event should append");
    log.append(route_decision_event(2, 10))
        .expect("second event should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("third event should append");

    let replayed: Vec<_> = log.replay(ReplayCursor::start()).collect();

    assert_eq!(
        replayed
            .iter()
            .map(|event| event.event_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(log.len(), 3);
}

#[test]
fn event_log_filters_by_type_without_reordering() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");
    log.append(route_decision_event(2, 10))
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
    log.append(route_decision_event(2, 10))
        .expect("second event should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("third event should append");

    let replayed: Vec<_> = log.replay(ReplayCursor::after(2)).collect();

    assert_eq!(
        replayed
            .iter()
            .map(|event| event.event_id)
            .collect::<Vec<_>>(),
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
    log.append(route_decision_event(2, 10))
        .expect("route_decision_recorded should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("task_assigned should accept route decision causation");
}

#[test]
fn task_assignment_rejects_route_decision_for_different_task() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");
    log.append(route_decision_event(2, 11))
        .expect("route_decision_recorded should append");

    let err = log
        .append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect_err("task_assigned should require same-task route causation");

    assert_eq!(
        err,
        LoomEventError::MismatchedTaskRouteDecision {
            task_id: 10,
            route_task_id: 11,
        }
    );
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

    log.append(with_payload_ref(
        event(LoomEventType::ArtifactDeclared, 1, 10),
        "inline://artifact/900",
    ))
    .expect("artifact_declared should append");
    log.append(caused_by(
        with_payload_ref(
            event(LoomEventType::ArtifactReady, 2, 10),
            "inline://artifact/900",
        ),
        1,
    ))
    .expect("artifact_ready should accept artifact_declared causation");
}

#[test]
fn artifact_ready_rejects_different_artifact_ref() {
    let mut log = InMemoryEventLog::new();

    log.append(with_payload_ref(
        event(LoomEventType::ArtifactDeclared, 1, 10),
        "inline://artifact/900",
    ))
    .expect("artifact_declared should append");

    let err = log
        .append(caused_by(
            with_payload_ref(
                event(LoomEventType::ArtifactReady, 2, 10),
                "inline://artifact/901",
            ),
            1,
        ))
        .expect_err("artifact_ready should match declared artifact ref");

    assert_eq!(
        err,
        LoomEventError::MismatchedArtifactReady {
            ready_event_id: 2,
            declared_event_id: 1,
            field: "payload_ref",
        }
    );
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

#[test]
fn topology_rolled_back_requires_snapshot_id() {
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TopologyProposed, 1, 0))
        .expect("topology_proposed should append");
    log.append(caused_by(event(LoomEventType::TopologyCommitted, 2, 0), 1))
        .expect("topology_committed should append");

    let mut invalid = caused_by(event(LoomEventType::TopologyRolledBack, 3, 0), 2);
    invalid.topology_snapshot_id = None;

    let err = log
        .append(invalid)
        .expect_err("topology_rolled_back should require topology snapshot id");

    assert_eq!(
        err,
        LoomEventError::MissingRequiredField {
            event_type: LoomEventType::TopologyRolledBack,
            field: "topology_snapshot_id",
        }
    );
}

#[test]
fn topology_rolled_back_requires_committed_causation() {
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TopologyProposed, 1, 0))
        .expect("topology_proposed should append");

    let err = log
        .append(caused_by(event(LoomEventType::TopologyRolledBack, 2, 0), 1))
        .expect_err("topology_rolled_back should be caused by a topology commit");

    assert_eq!(
        err,
        LoomEventError::InvalidCausationType {
            event_type: LoomEventType::TopologyRolledBack,
            causation_id: 1,
            expected: LoomEventType::TopologyCommitted,
            actual: LoomEventType::TopologyProposed,
        }
    );
}

#[test]
fn topology_shadowed_requires_proposed_causation() {
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");

    let err = log
        .append(caused_by(event(LoomEventType::TopologyShadowed, 2, 0), 1))
        .expect_err("topology_shadowed should be caused by a topology proposal");

    assert_eq!(
        err,
        LoomEventError::InvalidCausationType {
            event_type: LoomEventType::TopologyShadowed,
            causation_id: 1,
            expected: LoomEventType::TopologyProposed,
            actual: LoomEventType::TaskCreated,
        }
    );
}

#[test]
fn topology_canaried_requires_shadowed_causation() {
    let mut log = InMemoryEventLog::new();
    log.append(event(LoomEventType::TopologyProposed, 1, 0))
        .expect("topology_proposed should append");

    let err = log
        .append(caused_by(event(LoomEventType::TopologyCanaried, 2, 0), 1))
        .expect_err("topology_canaried should be caused by shadow evidence");

    assert_eq!(
        err,
        LoomEventError::InvalidCausationType {
            event_type: LoomEventType::TopologyCanaried,
            causation_id: 1,
            expected: LoomEventType::TopologyShadowed,
            actual: LoomEventType::TopologyProposed,
        }
    );
}

#[test]
fn replay_validation_reports_event_counts() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");
    log.append(route_decision_event(2, 10))
        .expect("route_decision_recorded should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("task_assigned should append");

    let report = log.validate_replay().expect("replay should validate");

    assert_eq!(report.event_count, 3);
    assert_eq!(report.root_task_count, 1);
    assert_eq!(report.task_event_count, 3);
}

#[test]
fn replay_validation_reports_lifecycle_counts() {
    let mut log = InMemoryEventLog::new();

    log.append(event(LoomEventType::TaskCreated, 1, 10))
        .expect("task_created should append");
    log.append(route_decision_event(2, 10))
        .expect("route_decision_recorded should append");
    log.append(caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2))
        .expect("task_assigned should append");
    log.append(event(LoomEventType::TaskCompleted, 4, 10))
        .expect("task_completed should append");

    let mut decommissioned = event(LoomEventType::AgentDecommissioned, 5, 10);
    decommissioned.agent_id = Some(42);
    log.append(decommissioned)
        .expect("agent_decommissioned should append");

    log.append(caused_by(event(LoomEventType::MemoryNodeCreated, 6, 10), 5))
        .expect("memory_node_created should append");
    log.append(event(LoomEventType::TopologyProposed, 7, 0))
        .expect("topology_proposed should append");
    log.append(caused_by(event(LoomEventType::TopologyShadowed, 8, 0), 7))
        .expect("topology_shadowed should append");
    log.append(caused_by(event(LoomEventType::TopologyCanaried, 9, 0), 8))
        .expect("topology_canaried should append");
    log.append(caused_by(event(LoomEventType::TopologyCommitted, 10, 0), 7))
        .expect("topology_committed should append");
    log.append(caused_by(event(LoomEventType::TopologyRolledBack, 11, 0), 10))
        .expect("topology_rolled_back should append");

    let report = log.validate_replay().expect("replay should validate");

    assert_eq!(report.route_decision_count, 1);
    assert_eq!(report.assignment_count, 1);
    assert_eq!(report.completion_count, 1);
    assert_eq!(report.decommission_count, 1);
    assert_eq!(report.memory_node_count, 1);
    assert_eq!(report.topology_shadow_count, 1);
    assert_eq!(report.topology_canary_count, 1);
    assert_eq!(report.topology_commit_count, 1);
    assert_eq!(report.topology_rollback_count, 1);
}

#[test]
fn replay_validation_rejects_duplicate_ids_in_slice() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        event(LoomEventType::TaskCreated, 1, 11),
    ];

    let err = validate_events(&events).expect_err("duplicate ids should fail replay validation");

    assert_eq!(err, LoomEventError::DuplicateEventId(1));
}

#[test]
fn replay_validation_rejects_bad_causation_in_slice() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        caused_by(event(LoomEventType::TaskAssigned, 2, 10), 1),
    ];

    let err = validate_events(&events).expect_err("bad causation should fail replay validation");

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
fn replay_validation_rejects_completed_task_without_decommission() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        event(LoomEventType::TaskCompleted, 2, 10),
    ];

    let err = validate_events(&events).expect_err("completed task should require decommission");

    assert_eq!(err, LoomEventError::MissingTaskDecommission { task_id: 10 });
}

#[test]
fn replay_validation_accepts_completed_task_with_decommission() {
    let mut decommissioned = event(LoomEventType::AgentDecommissioned, 3, 10);
    decommissioned.agent_id = Some(42);

    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        event(LoomEventType::TaskCompleted, 2, 10),
        decommissioned,
    ];

    let report = validate_events(&events).expect("completed task has decommission");

    assert_eq!(report.completion_count, 1);
    assert_eq!(report.decommission_count, 1);
}

#[test]
fn replay_validation_rejects_completed_task_with_decommission_from_different_root() {
    let mut decommissioned = with_root_task(event(LoomEventType::AgentDecommissioned, 3, 10), 2);
    decommissioned.agent_id = Some(42);

    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        event(LoomEventType::TaskCompleted, 2, 10),
        decommissioned,
    ];

    let err = validate_events(&events)
        .expect_err("completed task should require decommission from same root");

    assert_eq!(
        err,
        LoomEventError::MismatchedTaskDecommission {
            task_id: 10,
            decommission_event_id: 3,
            field: "root_task_id",
        }
    );
}

#[test]
fn replay_validation_rejects_completed_task_with_decommission_from_different_correlation() {
    let mut decommissioned =
        with_correlation(event(LoomEventType::AgentDecommissioned, 3, 10), 100);
    decommissioned.agent_id = Some(42);

    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        event(LoomEventType::TaskCompleted, 2, 10),
        decommissioned,
    ];

    let err = validate_events(&events)
        .expect_err("completed task should require decommission from same correlation");

    assert_eq!(
        err,
        LoomEventError::MismatchedTaskDecommission {
            task_id: 10,
            decommission_event_id: 3,
            field: "correlation_id",
        }
    );
}

#[test]
fn replay_validation_rejects_assignment_without_route_decision_in_slice() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        caused_by(event(LoomEventType::TaskAssigned, 2, 10), 99),
    ];

    let err = validate_events(&events).expect_err("assigned task should require route decision");

    assert_eq!(
        err,
        LoomEventError::MissingTaskRouteDecision { task_id: 10 }
    );
}

#[test]
fn replay_validation_accepts_assignment_with_route_decision() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        route_decision_event(2, 10),
        caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2),
    ];

    let report = validate_events(&events).expect("assigned task has route decision");

    assert_eq!(report.route_decision_count, 1);
    assert_eq!(report.assignment_count, 1);
}

#[test]
fn replay_validation_rejects_assignment_caused_by_different_task_route_decision() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        route_decision_event(2, 11),
        caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2),
    ];

    let err = validate_events(&events)
        .expect_err("assignment should require a route decision for the same task");

    assert_eq!(
        err,
        LoomEventError::MismatchedTaskRouteDecision {
            task_id: 10,
            route_task_id: 11,
        }
    );
}

#[test]
fn replay_validation_rejects_assignment_caused_by_different_root_route_decision() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        with_root_task(route_decision_event(2, 10), 2),
        caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2),
    ];

    let err = validate_events(&events)
        .expect_err("assignment should require route decision from the same root");

    assert_eq!(
        err,
        LoomEventError::MismatchedRouteDecisionContext {
            assignment_event_id: 3,
            route_decision_event_id: 2,
            field: "root_task_id",
        }
    );
}

#[test]
fn replay_validation_rejects_assignment_caused_by_different_correlation_route_decision() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        with_correlation(route_decision_event(2, 10), 100),
        caused_by(event(LoomEventType::TaskAssigned, 3, 10), 2),
    ];

    let err = validate_events(&events)
        .expect_err("assignment should require route decision from the same correlation");

    assert_eq!(
        err,
        LoomEventError::MismatchedRouteDecisionContext {
            assignment_event_id: 3,
            route_decision_event_id: 2,
            field: "correlation_id",
        }
    );
}

#[test]
fn replay_validation_rejects_route_decision_with_wrong_schema() {
    let mut route_decision = event(LoomEventType::RouteDecisionRecorded, 2, 10);
    route_decision.payload_schema = "test.payload.v1".to_string();
    route_decision.payload_ref = "inline://route-decision/501".to_string();
    let events = vec![event(LoomEventType::TaskCreated, 1, 10), route_decision];

    let err = validate_events(&events).expect_err("route decision schema should be exact");

    assert_eq!(
        err,
        LoomEventError::InvalidRouteDecisionTelemetry {
            event_id: 2,
            field: "payload_schema",
        }
    );
}

#[test]
fn replay_validation_rejects_route_decision_with_wrong_payload_ref() {
    let mut route_decision = event(LoomEventType::RouteDecisionRecorded, 2, 10);
    route_decision.payload_schema = "qtom.route_decision.v1".to_string();
    route_decision.payload_ref = "inline://event/2".to_string();
    let events = vec![event(LoomEventType::TaskCreated, 1, 10), route_decision];

    let err = validate_events(&events).expect_err("route decision payload ref should be exact");

    assert_eq!(
        err,
        LoomEventError::InvalidRouteDecisionTelemetry {
            event_id: 2,
            field: "payload_ref",
        }
    );
}

#[test]
fn replay_validation_rejects_artifact_ready_caused_by_different_task_declaration() {
    let events = vec![
        with_payload_ref(
            event(LoomEventType::ArtifactDeclared, 1, 11),
            "inline://artifact/900",
        ),
        caused_by(
            with_payload_ref(
                event(LoomEventType::ArtifactReady, 2, 10),
                "inline://artifact/900",
            ),
            1,
        ),
    ];

    let err =
        validate_events(&events).expect_err("artifact_ready should match declaration task context");

    assert_eq!(
        err,
        LoomEventError::MismatchedArtifactReady {
            ready_event_id: 2,
            declared_event_id: 1,
            field: "task_id",
        }
    );
}

#[test]
fn replay_validation_rejects_artifact_ready_caused_by_different_root_declaration() {
    let events = vec![
        with_root_task(
            with_payload_ref(
                event(LoomEventType::ArtifactDeclared, 1, 10),
                "inline://artifact/900",
            ),
            2,
        ),
        caused_by(
            with_payload_ref(
                event(LoomEventType::ArtifactReady, 2, 10),
                "inline://artifact/900",
            ),
            1,
        ),
    ];

    let err =
        validate_events(&events).expect_err("artifact_ready should match declaration root context");

    assert_eq!(
        err,
        LoomEventError::MismatchedArtifactReady {
            ready_event_id: 2,
            declared_event_id: 1,
            field: "root_task_id",
        }
    );
}

#[test]
fn replay_validation_rejects_artifact_ready_caused_by_different_correlation_declaration() {
    let events = vec![
        with_correlation(
            with_payload_ref(
                event(LoomEventType::ArtifactDeclared, 1, 10),
                "inline://artifact/900",
            ),
            100,
        ),
        caused_by(
            with_payload_ref(
                event(LoomEventType::ArtifactReady, 2, 10),
                "inline://artifact/900",
            ),
            1,
        ),
    ];

    let err = validate_events(&events)
        .expect_err("artifact_ready should match declaration correlation context");

    assert_eq!(
        err,
        LoomEventError::MismatchedArtifactReady {
            ready_event_id: 2,
            declared_event_id: 1,
            field: "correlation_id",
        }
    );
}

#[test]
fn replay_validation_rejects_memory_node_without_evidence() {
    let events = vec![event(LoomEventType::MemoryNodeCreated, 1, 10)];

    let err = validate_events(&events).expect_err("memory node should require evidence");

    assert_eq!(err, LoomEventError::MissingMemoryEvidence { event_id: 1 });
}

#[test]
fn replay_validation_accepts_memory_node_with_decommission_evidence() {
    let mut decommissioned = event(LoomEventType::AgentDecommissioned, 1, 10);
    decommissioned.agent_id = Some(42);

    let events = vec![
        decommissioned,
        caused_by(event(LoomEventType::MemoryNodeCreated, 2, 10), 1),
    ];

    let report = validate_events(&events).expect("memory node has evidence");

    assert_eq!(report.decommission_count, 1);
    assert_eq!(report.memory_node_count, 1);
}

#[test]
fn replay_validation_rejects_memory_node_with_different_task_evidence() {
    let mut decommissioned = event(LoomEventType::AgentDecommissioned, 1, 11);
    decommissioned.agent_id = Some(42);

    let events = vec![
        decommissioned,
        caused_by(event(LoomEventType::MemoryNodeCreated, 2, 10), 1),
    ];

    let err =
        validate_events(&events).expect_err("memory node should match decommission task context");

    assert_eq!(
        err,
        LoomEventError::MismatchedMemoryEvidence {
            memory_event_id: 2,
            decommission_event_id: 1,
            field: "task_id",
        }
    );
}

#[test]
fn replay_validation_rejects_memory_node_with_different_root_evidence() {
    let mut decommissioned = with_root_task(event(LoomEventType::AgentDecommissioned, 1, 10), 2);
    decommissioned.agent_id = Some(42);

    let events = vec![
        decommissioned,
        caused_by(event(LoomEventType::MemoryNodeCreated, 2, 10), 1),
    ];

    let err =
        validate_events(&events).expect_err("memory node should match decommission root context");

    assert_eq!(
        err,
        LoomEventError::MismatchedMemoryEvidence {
            memory_event_id: 2,
            decommission_event_id: 1,
            field: "root_task_id",
        }
    );
}

#[test]
fn replay_validation_rejects_memory_node_with_different_correlation_evidence() {
    let mut decommissioned =
        with_correlation(event(LoomEventType::AgentDecommissioned, 1, 10), 100);
    decommissioned.agent_id = Some(42);

    let events = vec![
        decommissioned,
        caused_by(event(LoomEventType::MemoryNodeCreated, 2, 10), 1),
    ];

    let err = validate_events(&events)
        .expect_err("memory node should match decommission correlation context");

    assert_eq!(
        err,
        LoomEventError::MismatchedMemoryEvidence {
            memory_event_id: 2,
            decommission_event_id: 1,
            field: "correlation_id",
        }
    );
}

#[test]
fn replay_validation_rejects_child_task_without_integration_path() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        child_task(event(LoomEventType::TaskCreated, 2, 11), 10),
    ];

    let err = validate_events(&events).expect_err("child task should require integration path");

    assert_eq!(err, LoomEventError::MissingTaskIntegration { task_id: 11 });
}

#[test]
fn replay_validation_rejects_child_task_with_integration_path_from_different_root() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        child_task(event(LoomEventType::TaskCreated, 2, 11), 10),
        with_root_task(event(LoomEventType::IntegrationRequested, 3, 10), 2),
    ];

    let err = validate_events(&events)
        .expect_err("integration path should match child task root context");

    assert_eq!(
        err,
        LoomEventError::MismatchedTaskIntegration {
            task_id: 11,
            integration_event_id: 3,
            field: "root_task_id",
        }
    );
}

#[test]
fn replay_validation_rejects_child_task_with_integration_path_from_different_correlation() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        child_task(event(LoomEventType::TaskCreated, 2, 11), 10),
        with_correlation(event(LoomEventType::IntegrationRequested, 3, 10), 100),
    ];

    let err = validate_events(&events)
        .expect_err("integration path should match child task correlation context");

    assert_eq!(
        err,
        LoomEventError::MismatchedTaskIntegration {
            task_id: 11,
            integration_event_id: 3,
            field: "correlation_id",
        }
    );
}

#[test]
fn replay_validation_accepts_child_task_with_integration_path() {
    let events = vec![
        event(LoomEventType::TaskCreated, 1, 10),
        child_task(event(LoomEventType::TaskCreated, 2, 11), 10),
        event(LoomEventType::IntegrationRequested, 3, 10),
    ];

    let report = validate_events(&events).expect("child task has integration path");

    assert_eq!(report.integration_request_count, 1);
}
