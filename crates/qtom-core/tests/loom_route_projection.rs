use qtom_core::{MockTaskLoom, route_trace_projection};

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
