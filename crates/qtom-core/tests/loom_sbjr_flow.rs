use qtom_core::{LoomEventType, MockTaskLoom, ReplayCursor};

#[test]
fn mock_task_loom_runs_split_build_join_remember_flow() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    assert_eq!(output.root_task.task_id, 10);
    assert_eq!(output.director.children.len(), 2);
    assert_eq!(output.route_decisions.len(), 2);
    assert_eq!(output.constructor_outputs.len(), 2);
    assert_eq!(
        output
            .constructor_outputs
            .iter()
            .map(|output| output.artifact.agent_id)
            .collect::<Vec<_>>(),
        output
            .route_decisions
            .iter()
            .map(|decision| u64::from(decision.selected_agent_id))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        output.integration.report.included_task_ids,
        output
            .director
            .children
            .iter()
            .map(|child| child.task_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(output.curator_outputs.len(), 2);

    let report = output
        .event_log
        .validate_replay()
        .expect("combined SBJR event log should replay");
    assert_eq!(report.completion_count, 2);
    assert_eq!(report.decommission_count, 2);
    assert_eq!(report.memory_node_count, 2);
    assert_eq!(report.integration_request_count, 2);
    assert_eq!(report.route_decision_count, 2);
    assert_eq!(report.assignment_count, 2);
}

#[test]
fn mock_task_loom_event_sequence_matches_lifecycle_flow() {
    let output = MockTaskLoom::default()
        .run_prompt(7, 10, "prototype the routing boundary")
        .expect("mock SBJR flow should run");

    let event_types: Vec<_> = output
        .event_log
        .replay(ReplayCursor::start())
        .map(|event| event.event_type)
        .collect();

    assert_eq!(
        event_types,
        vec![
            LoomEventType::TaskCreated,
            LoomEventType::TaskCreated,
            LoomEventType::TaskCreated,
            LoomEventType::IntegrationRequested,
            LoomEventType::RouteDecisionRecorded,
            LoomEventType::TaskAssigned,
            LoomEventType::ArtifactDeclared,
            LoomEventType::ArtifactReady,
            LoomEventType::TaskCompleted,
            LoomEventType::AgentDecommissioned,
            LoomEventType::RouteDecisionRecorded,
            LoomEventType::TaskAssigned,
            LoomEventType::ArtifactDeclared,
            LoomEventType::ArtifactReady,
            LoomEventType::TaskCompleted,
            LoomEventType::AgentDecommissioned,
            LoomEventType::IntegrationRequested,
            LoomEventType::MemoryNodeCreated,
            LoomEventType::MemoryNodeCreated,
        ]
    );
}
