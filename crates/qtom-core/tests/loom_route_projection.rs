use qtom_core::{
    InMemoryEventLog, LoomEvent, LoomEventError, LoomEventType, MockTaskLoom,
    artifact_provenance_projection, integration_group_projection, loom_projection_bundle,
    loom_replay_report, memory_lineage_projection, route_trace_projection,
    task_dependency_projection, topology_governance_projection,
};

fn event(event_type: LoomEventType, event_id: u64, payload_ref: impl Into<String>) -> LoomEvent {
    LoomEvent {
        event_id,
        event_type,
        root_task_id: 1,
        task_id: Some(0),
        parent_task_id: None,
        prompt_id: Some(7),
        agent_id: None,
        agent_role: None,
        topology_snapshot_id: Some(9_000 + event_id),
        payload_schema: "test.payload.v1".to_string(),
        payload_ref: payload_ref.into(),
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
fn route_trace_projection_is_derived_from_loom_events() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let projection = route_trace_projection(&output.event_log);

    assert!(projection.starts_with("flowchart TD\n"));
    assert!(projection.contains("task_1000[\"Task 1000\"]"));
    assert!(projection.contains("route_100[\"RouteDecision 500\"]"));
    assert!(projection.contains("assignment_101[\"TaskAssigned 1000\"]"));
    assert!(projection.contains("agent_10000[\"Agent 10000\"]"));
    assert!(projection.contains("task_1000 --> route_100"));
    assert!(projection.contains("route_100 --> assignment_101"));
    assert!(projection.contains("assignment_101 --> agent_10000"));
    assert!(projection.contains("task_1001 --> route_102"));
    assert!(projection.contains("route_102 --> assignment_103"));
    assert!(projection.contains("assignment_103 --> agent_10001"));
}

#[test]
fn memory_lineage_projection_is_derived_from_decommission_and_memory_events() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let projection = memory_lineage_projection(&output.event_log);

    assert!(projection.starts_with("flowchart TD\n"));
    assert!(projection.contains("task_1000[\"Task 1000\"]"));
    assert!(projection.contains("decommission_2003[\"Decommission 10000\"]"));
    assert!(projection.contains("memory_4000[\"MemoryNode 1500\"]"));
    assert!(projection.contains("task_1000 --> decommission_2003"));
    assert!(projection.contains("decommission_2003 --> memory_4000"));
    assert!(projection.contains("task_1001 --> decommission_2013"));
    assert!(projection.contains("decommission_2013 --> memory_4001"));
}

#[test]
fn task_dependency_projection_is_derived_from_task_and_integration_events() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let projection = task_dependency_projection(&output.event_log);

    assert!(projection.starts_with("flowchart TD\n"));
    assert!(projection.contains("task_10[\"Task 10\"]"));
    assert!(projection.contains("task_1000[\"Task 1000\"]"));
    assert!(projection.contains("task_1001[\"Task 1001\"]"));
    assert!(projection.contains("integration_10[\"Integration 10\"]"));
    assert!(projection.contains("task_10 --> task_1000"));
    assert!(projection.contains("task_10 --> task_1001"));
    assert!(projection.contains("task_1000 --> integration_10"));
    assert!(projection.contains("task_1001 --> integration_10"));
}

#[test]
fn artifact_provenance_projection_is_derived_from_artifact_events() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let projection = artifact_provenance_projection(&output.event_log);

    assert!(projection.starts_with("flowchart TD\n"));
    assert!(projection.contains("task_1000[\"Task 1000\"]"));
    assert!(projection.contains("artifact_declared_2000[\"ArtifactDeclared 900\"]"));
    assert!(projection.contains("artifact_ready_2001[\"ArtifactReady 900\"]"));
    assert!(projection.contains("agent_10000[\"Agent 10000\"]"));
    assert!(projection.contains("task_1000 --> artifact_declared_2000"));
    assert!(projection.contains("artifact_declared_2000 --> artifact_ready_2001"));
    assert!(projection.contains("artifact_ready_2001 --> agent_10000"));
    assert!(projection.contains("task_1001 --> artifact_declared_2010"));
    assert!(projection.contains("artifact_declared_2010 --> artifact_ready_2011"));
    assert!(projection.contains("artifact_ready_2011 --> agent_10001"));
}

#[test]
fn integration_group_projection_is_derived_from_task_and_integration_events() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let projection = integration_group_projection(&output.event_log);

    assert!(projection.starts_with("flowchart TD\n"));
    assert!(projection.contains("task_10[\"Task 10\"]"));
    assert!(projection.contains("task_1000[\"Task 1000\"]"));
    assert!(projection.contains("task_1001[\"Task 1001\"]"));
    assert!(projection.contains("integration_group_10[\"IntegrationGroup 1\"]"));
    assert!(projection.contains("integration_report_3000[\"IntegrationReport 1\"]"));
    assert!(projection.contains("agent_700[\"Agent 700\"]"));
    assert!(projection.contains("task_10 --> integration_group_10"));
    assert!(projection.contains("task_1000 --> integration_group_10"));
    assert!(projection.contains("task_1001 --> integration_group_10"));
    assert!(projection.contains("integration_group_10 --> integration_report_3000"));
    assert!(projection.contains("integration_report_3000 --> agent_700"));
}

#[test]
fn topology_governance_projection_is_derived_from_topology_events() {
    let mut log = InMemoryEventLog::new();
    log.append(event(
        LoomEventType::TopologyProposed,
        1,
        "inline://topology/proposal/8000",
    ))
    .expect("proposal should append");
    log.append(caused_by(
        event(
            LoomEventType::TopologyShadowed,
            2,
            "inline://topology/shadow-report/8000",
        ),
        1,
    ))
    .expect("shadow should append");
    log.append(caused_by(
        event(
            LoomEventType::TopologyCanaried,
            3,
            "inline://topology/canary-report/8000",
        ),
        2,
    ))
    .expect("canary should append");
    log.append(caused_by(
        event(
            LoomEventType::TopologyCommitted,
            4,
            "inline://topology/snapshot/9000",
        ),
        1,
    ))
    .expect("commit should append");
    log.append(caused_by(
        event(
            LoomEventType::TopologyRolledBack,
            5,
            "inline://topology/rollback/10000",
        ),
        4,
    ))
    .expect("rollback should append");

    let projection = topology_governance_projection(&log);

    assert!(projection.starts_with("flowchart TD\n"));
    assert!(projection.contains("topology_proposed_1[\"TopologyProposed 8000\"]"));
    assert!(projection.contains("topology_shadowed_2[\"TopologyShadowed 8000\"]"));
    assert!(projection.contains("topology_canaried_3[\"TopologyCanaried 8000\"]"));
    assert!(projection.contains("topology_committed_4[\"TopologyCommitted 9004\"]"));
    assert!(projection.contains("topology_rolled_back_5[\"TopologyRolledBack 9005\"]"));
    assert!(projection.contains("topology_proposed_1 --> topology_shadowed_2"));
    assert!(projection.contains("topology_shadowed_2 --> topology_canaried_3"));
    assert!(projection.contains("topology_proposed_1 --> topology_committed_4"));
    assert!(projection.contains("topology_committed_4 --> topology_rolled_back_5"));
}

#[test]
fn projection_bundle_contains_all_current_replay_views() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let bundle = loom_projection_bundle(&output.event_log);

    assert!(bundle.task_dependency.contains("task_10 --> task_1000"));
    assert!(bundle.route_trace.contains("route_100 --> assignment_101"));
    assert!(
        bundle
            .artifact_provenance
            .contains("artifact_declared_2000 --> artifact_ready_2001")
    );
    assert!(
        bundle
            .integration_group
            .contains("integration_group_10 --> integration_report_3000")
    );
    assert!(
        bundle
            .memory_lineage
            .contains("decommission_2003 --> memory_4000")
    );
    assert!(bundle.topology_governance.starts_with("flowchart TD\n"));
}

#[test]
fn projection_bundle_is_stable_for_same_replayed_log() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let first = loom_projection_bundle(&output.event_log);
    let second = loom_projection_bundle(&output.event_log);

    assert_eq!(first, second);
}

#[test]
fn replay_report_validates_and_projects_full_mock_run() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let report = loom_replay_report(&output.event_log).expect("mock run should replay");

    assert_eq!(report.validation.route_decision_count, 2);
    assert_eq!(report.validation.assignment_count, 2);
    assert_eq!(report.validation.decommission_count, 2);
    assert_eq!(report.validation.memory_node_count, 2);
    assert!(
        report
            .projections
            .route_trace
            .contains("route_100 --> assignment_101")
    );
    assert!(
        report
            .projections
            .memory_lineage
            .contains("decommission_2003 --> memory_4000")
    );
}

#[test]
fn replay_report_rejects_invalid_log_before_projecting() {
    let log = InMemoryEventLog::new();

    let err = loom_replay_report(&log).expect_err("empty replay should fail missing routes");

    assert_eq!(err, LoomEventError::EmptyReplayLog);
}
