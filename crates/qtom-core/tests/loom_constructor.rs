use qtom_core::{
    LoomEventType, MockConstructor, MockConstructorConfig, ReplayCursor, TaskEnvelope,
    validate_events,
};

fn child_task() -> TaskEnvelope {
    TaskEnvelope::child(11, 1, 10, 7, 42, 88, "build constructor artifact")
        .expect("child task should be valid")
}

#[test]
fn constructor_builds_artifact_and_required_lifecycle_events() {
    let constructor = MockConstructor::new(MockConstructorConfig {
        agent_id: 301,
        next_artifact_id: 900,
        next_event_id: 2_000,
        occurred_at_ms: 10_000,
        correlation_id: 77,
    });

    let output = constructor
        .build_child_task(&child_task())
        .expect("constructor should build child task");

    assert_eq!(output.artifact.artifact_id, 900);
    assert_eq!(output.artifact.root_task_id, 1);
    assert_eq!(output.artifact.task_id, 11);
    assert_eq!(output.artifact.agent_id, 301);

    let event_types: Vec<_> = output
        .event_log
        .replay(ReplayCursor::start())
        .map(|event| event.event_type)
        .collect();
    assert_eq!(
        event_types,
        vec![
            LoomEventType::ArtifactDeclared,
            LoomEventType::ArtifactReady,
            LoomEventType::TaskCompleted,
            LoomEventType::AgentDecommissioned,
        ]
    );

    let report = output
        .event_log
        .validate_replay()
        .expect("constructor event log should replay");
    assert_eq!(report.completion_count, 1);
    assert_eq!(report.decommission_count, 1);
}

#[test]
fn constructor_events_preserve_task_agent_and_artifact_refs() {
    let output = MockConstructor::default()
        .build_child_task(&child_task())
        .expect("constructor should build child task");

    let events: Vec<_> = output.event_log.replay(ReplayCursor::start()).collect();

    assert!(events.iter().all(|event| event.root_task_id == 1));
    assert!(events.iter().all(|event| event.task_id == Some(11)));
    assert!(events.iter().all(|event| event.agent_id == Some(301)));
    assert!(
        events
            .iter()
            .all(|event| event.payload_ref == "inline://artifact/900"
                || event.payload_ref == "inline://decommission/301/11")
    );
    assert_eq!(events[1].causation_id, Some(events[0].event_id));
    validate_events(&events.into_iter().cloned().collect::<Vec<_>>())
        .expect("event slice should validate");
}
