use qtom_core::{
    ArtifactRef, IntegrationGroup, IntegrationStatus, JoinPolicy, LoomEventType, MockIntegration,
    MockIntegrationConfig, ReplayCursor,
};

fn integration_group() -> IntegrationGroup {
    IntegrationGroup::new(
        88,
        1,
        10,
        42,
        vec![11, 12],
        JoinPolicy::WaitAll,
        "inline://acceptance/42",
        vec![700],
    )
    .expect("integration group should be valid")
}

fn artifact(artifact_id: u64, task_id: u64) -> ArtifactRef {
    ArtifactRef::new(
        artifact_id,
        1,
        task_id,
        301,
        "mock.markdown",
        format!("inline://artifact/{artifact_id}"),
    )
    .expect("artifact should be valid")
}

#[test]
fn integration_mock_accepts_completed_child_artifacts() {
    let integration = MockIntegration::new(MockIntegrationConfig {
        integration_agent_id: 700,
        next_event_id: 3_000,
        occurred_at_ms: 20_000,
        correlation_id: 77,
    });

    let output = integration
        .integrate_completed_children(
            &integration_group(),
            &[artifact(900, 11), artifact(901, 12)],
        )
        .expect("integration should accept completed children");

    assert_eq!(output.report.integration_group_id, 88);
    assert_eq!(output.report.included_task_ids, vec![11, 12]);
    assert_eq!(output.report.final_artifact_refs, vec![900, 901]);
    assert_eq!(output.report.acceptance_status, IntegrationStatus::Accepted);
}

#[test]
fn integration_mock_emits_join_event_with_group_lineage() {
    let output = MockIntegration::default()
        .integrate_completed_children(
            &integration_group(),
            &[artifact(900, 11), artifact(901, 12)],
        )
        .expect("integration should accept completed children");

    let events: Vec<_> = output.event_log.replay(ReplayCursor::start()).collect();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, LoomEventType::IntegrationRequested);
    assert_eq!(events[0].root_task_id, 1);
    assert_eq!(events[0].task_id, Some(10));
    assert_eq!(events[0].agent_id, Some(700));
    assert_eq!(events[0].payload_ref, "inline://integration/report/88");
    output
        .event_log
        .validate_replay()
        .expect("integration event log should replay");
}

#[test]
fn integration_mock_rejects_missing_child_artifact() {
    let err = MockIntegration::default()
        .integrate_completed_children(&integration_group(), &[artifact(900, 11)])
        .expect_err("missing child artifact should fail");

    assert_eq!(
        err.to_string(),
        "`completed_child_artifacts` is missing task 12"
    );
}
