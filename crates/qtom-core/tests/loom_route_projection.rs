use qtom_core::{
    MockTaskLoom, artifact_provenance_projection, integration_group_projection,
    memory_lineage_projection, route_trace_projection, task_dependency_projection,
};

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
