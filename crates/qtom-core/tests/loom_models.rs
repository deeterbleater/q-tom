use qtom_core::{
    DependencyEdge, DependencyKind, IntegrationGroup, JoinPolicy, LoomModelError, PlanNode,
    TaskEnvelope,
};

#[test]
fn child_task_envelope_preserves_lineage() {
    let task = TaskEnvelope::child(11, 1, 10, 7, 42, 88, "write implementation notes")
        .expect("child task should be valid");

    assert_eq!(task.task_id, 11);
    assert_eq!(task.root_task_id, 1);
    assert_eq!(task.parent_task_id, Some(10));
    assert_eq!(task.prompt_id, 7);
    assert_eq!(task.plan_id, 42);
    assert_eq!(task.integration_group_id, 88);
    assert_eq!(task.summary, "write implementation notes");
}

#[test]
fn child_task_envelope_rejects_empty_summary() {
    let err =
        TaskEnvelope::child(11, 1, 10, 7, 42, 88, " ").expect_err("empty task summary should fail");

    assert_eq!(err, LoomModelError::EmptyField("summary"));
}

#[test]
fn plan_node_requires_child_tasks_and_integration_group() {
    let plan = PlanNode::new(
        42,
        1,
        10,
        500,
        "ref://decomposition/42",
        vec![11, 12],
        vec![DependencyEdge::new(11, 12, DependencyKind::Blocks)],
        88,
        "ref://acceptance/42",
        vec!["needs_join_validation".to_string()],
    )
    .expect("plan should be valid");

    assert_eq!(plan.child_task_ids, vec![11, 12]);
    assert_eq!(plan.dependency_edges[0].from_task_id, 11);
    assert_eq!(plan.integration_group_id, 88);
}

#[test]
fn plan_node_rejects_empty_child_set() {
    let err = PlanNode::new(
        42,
        1,
        10,
        500,
        "ref://decomposition/42",
        vec![],
        vec![],
        88,
        "ref://acceptance/42",
        vec![],
    )
    .expect_err("plan without child tasks should fail");

    assert_eq!(err, LoomModelError::EmptyCollection("child_task_ids"));
}

#[test]
fn integration_group_tracks_join_policy_and_expected_children() {
    let group = IntegrationGroup::new(
        88,
        1,
        10,
        42,
        vec![11, 12],
        JoinPolicy::WaitAll,
        "ref://acceptance/42",
        vec![700],
    )
    .expect("integration group should be valid");

    assert_eq!(group.expected_child_task_ids, vec![11, 12]);
    assert_eq!(group.join_policy, JoinPolicy::WaitAll);
    assert_eq!(group.integration_agent_ids, vec![700]);
}

#[test]
fn integration_group_rejects_missing_expected_children() {
    let err = IntegrationGroup::new(
        88,
        1,
        10,
        42,
        vec![],
        JoinPolicy::WaitAll,
        "ref://acceptance/42",
        vec![700],
    )
    .expect_err("integration group without expected children should fail");

    assert_eq!(
        err,
        LoomModelError::EmptyCollection("expected_child_task_ids")
    );
}
