use qtom_core::{JoinPolicy, MockDirector, MockDirectorConfig};

#[test]
fn director_splits_root_task_into_plan_children_and_integration_group() {
    let director = MockDirector::new(MockDirectorConfig {
        director_agent_id: 500,
        integration_agent_id: 700,
        next_plan_id: 42,
        next_integration_group_id: 88,
        next_child_task_id: 1_000,
    });

    let output = director
        .split_root_task(7, 10, 1, "prototype the routing boundary")
        .expect("root task should split");

    assert_eq!(output.plan.plan_id, 42);
    assert_eq!(output.plan.task_id, 10);
    assert_eq!(output.plan.root_task_id, 1);
    assert_eq!(output.plan.director_agent_id, 500);
    assert_eq!(output.children.len(), 2);
    assert_eq!(output.integration_group.integration_group_id, 88);
    assert_eq!(output.integration_group.join_policy, JoinPolicy::WaitAll);

    let child_ids: Vec<_> = output.children.iter().map(|child| child.task_id).collect();
    assert_eq!(output.plan.child_task_ids, child_ids);
    assert_eq!(output.integration_group.expected_child_task_ids, child_ids);

    for child in output.children {
        assert_eq!(child.root_task_id, 1);
        assert_eq!(child.parent_task_id, Some(10));
        assert_eq!(child.prompt_id, 7);
        assert_eq!(child.plan_id, 42);
        assert_eq!(child.integration_group_id, 88);
    }
}

#[test]
fn director_rejects_empty_root_summary() {
    let director = MockDirector::default();

    let err = director
        .split_root_task(7, 10, 1, " ")
        .expect_err("empty root summary should fail");

    assert_eq!(err.to_string(), "`summary` must not be empty");
}
