use qtom_core::{
    AgentRuntime, HydratedContext, MockConstructorRuntime, MockConstructorRuntimeConfig,
    TaskEnvelope,
};

#[test]
fn mock_constructor_runtime_executes_through_agent_runtime_interface() {
    let task = TaskEnvelope::child(11, 1, 10, 7, 42, 88, "build constructor artifact")
        .expect("child task should be valid");
    let runtime = MockConstructorRuntime::new(MockConstructorRuntimeConfig {
        agent_id: 10_000,
        next_artifact_id: 900,
        next_packet_id: 1_200,
        next_event_id: 2_000,
        occurred_at_ms: 10_000,
        correlation_id: 77,
    });
    let context = HydratedContext::new(
        "inline://prompt/7",
        vec!["inline://tool/default".to_string()],
        vec!["inline://memory/default".to_string()],
    )
    .expect("hydrated context should be valid");

    let output = runtime
        .execute(&task, &context)
        .expect("mock runtime should execute offline");

    assert_eq!(output.artifacts.len(), 1);
    assert_eq!(output.artifacts[0].agent_id, 10_000);
    assert_eq!(output.decommission_packet.packet_id, 1_200);
    assert_eq!(output.decommission_packet.agent_id, 10_000);
    assert_eq!(output.decommission_packet.task_id, 11);
    assert_eq!(
        output.event_log.validate_replay().unwrap().completion_count,
        1
    );
}

#[test]
fn hydrated_context_requires_prompt_ref() {
    let err = HydratedContext::new(
        " ",
        vec!["inline://tool/default".to_string()],
        vec!["inline://memory/default".to_string()],
    )
    .expect_err("prompt ref should be required");

    assert_eq!(err.to_string(), "`prompt_ref` must not be empty");
}
