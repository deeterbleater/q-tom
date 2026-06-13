use qtom_core::{
    AgentDecommissionPacket, ArtifactRef, DependencyEdge, DependencyKind, GradientAxis,
    GradientSpace, IntegrationGroup, IntegrationReport, IntegrationStatus, JoinPolicy,
    LoomModelError, MemoryNode, MemoryNodeKind, MemoryPlacement, PlanNode, TaskEnvelope,
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

#[test]
fn artifact_ref_preserves_task_and_agent_lineage() {
    let artifact = ArtifactRef::new(900, 1, 11, 301, "report.markdown", "inline://artifact/900")
        .expect("artifact ref should be valid");

    assert_eq!(artifact.artifact_id, 900);
    assert_eq!(artifact.root_task_id, 1);
    assert_eq!(artifact.task_id, 11);
    assert_eq!(artifact.agent_id, 301);
    assert_eq!(artifact.artifact_kind, "report.markdown");
    assert_eq!(artifact.content_ref, "inline://artifact/900");
}

#[test]
fn artifact_ref_rejects_empty_content_ref() {
    let err = ArtifactRef::new(900, 1, 11, 301, "report.markdown", " ")
        .expect_err("empty content ref should fail");

    assert_eq!(err, LoomModelError::EmptyField("content_ref"));
}

#[test]
fn accepted_integration_report_tracks_completed_children_and_artifacts() {
    let report = IntegrationReport::accepted(
        88,
        vec![11, 12],
        vec![900, 901],
        "inline://integration/report/88",
    )
    .expect("accepted integration report should be valid");

    assert_eq!(report.integration_group_id, 88);
    assert_eq!(report.included_task_ids, vec![11, 12]);
    assert!(report.excluded_task_ids.is_empty());
    assert!(report.conflict_edges.is_empty());
    assert!(report.gap_edges.is_empty());
    assert!(report.repair_task_ids.is_empty());
    assert_eq!(report.final_artifact_refs, vec![900, 901]);
    assert_eq!(report.acceptance_status, IntegrationStatus::Accepted);
}

#[test]
fn accepted_integration_report_requires_included_tasks() {
    let err = IntegrationReport::accepted(88, vec![], vec![900], "inline://integration/report/88")
        .expect_err("accepted report should require included children");

    assert_eq!(err, LoomModelError::EmptyCollection("included_task_ids"));
}

#[test]
fn decommission_packet_preserves_task_agent_plan_and_artifact_lineage() {
    let packet = AgentDecommissionPacket::completed(
        1_200,
        301,
        1,
        11,
        7,
        42,
        vec![900],
        "inline://summary/1200",
    )
    .expect("decommission packet should be valid");

    assert_eq!(packet.packet_id, 1_200);
    assert_eq!(packet.agent_id, 301);
    assert_eq!(packet.root_task_id, 1);
    assert_eq!(packet.task_id, 11);
    assert_eq!(packet.prompt_id, 7);
    assert_eq!(packet.plan_id, 42);
    assert_eq!(packet.deliverable_refs, vec![900]);
    assert_eq!(packet.final_status, "completed");
}

#[test]
fn decommission_packet_requires_deliverable_refs() {
    let err = AgentDecommissionPacket::completed(
        1_200,
        301,
        1,
        11,
        7,
        42,
        vec![],
        "inline://summary/1200",
    )
    .expect_err("completed packet should require deliverables");

    assert_eq!(err, LoomModelError::EmptyCollection("deliverable_refs"));
}

#[test]
fn memory_node_preserves_packet_evidence() {
    let node = MemoryNode::from_packet(
        1_500,
        MemoryNodeKind::Episode,
        1,
        11,
        1_200,
        vec!["inline://summary/1200".to_string()],
        "completed constructor task",
    )
    .expect("memory node should be valid");

    assert_eq!(node.memory_node_id, 1_500);
    assert_eq!(node.kind, MemoryNodeKind::Episode);
    assert_eq!(node.root_task_id, 1);
    assert_eq!(node.task_id, 11);
    assert_eq!(node.packet_id, 1_200);
    assert_eq!(node.evidence_refs, vec!["inline://summary/1200"]);
    assert_eq!(node.summary, "completed constructor task");
}

#[test]
fn memory_node_requires_evidence_refs() {
    let err = MemoryNode::from_packet(
        1_500,
        MemoryNodeKind::Episode,
        1,
        11,
        1_200,
        vec![],
        "completed constructor task",
    )
    .expect_err("memory node should require evidence refs");

    assert_eq!(err, LoomModelError::EmptyCollection("evidence_refs"));
}

#[test]
fn gradient_space_preserves_versioned_axes() {
    let space = GradientSpace::new(
        44,
        "mock-memory",
        3,
        vec![
            GradientAxis::new(1, "domain", "general", "specialized", 0.9)
                .expect("axis should be valid"),
            GradientAxis::new(2, "tool-affinity", "low", "high", 0.8)
                .expect("axis should be valid"),
        ],
    )
    .expect("space should be valid");

    assert_eq!(space.gradient_space_id, 44);
    assert_eq!(space.name, "mock-memory");
    assert_eq!(space.version, 3);
    assert_eq!(space.axes.len(), 2);
}

#[test]
fn memory_node_can_be_placed_in_versioned_gradient_space() {
    let space = GradientSpace::new(
        44,
        "mock-memory",
        3,
        vec![
            GradientAxis::new(1, "domain", "general", "specialized", 0.9)
                .expect("axis should be valid"),
            GradientAxis::new(2, "tool-affinity", "low", "high", 0.8)
                .expect("axis should be valid"),
        ],
    )
    .expect("space should be valid");
    let node = MemoryNode::from_packet(
        1_500,
        MemoryNodeKind::Episode,
        1,
        11,
        1_200,
        vec!["inline://summary/1200".to_string()],
        "completed constructor task",
    )
    .expect("memory node should be valid");

    let placement = space
        .place_memory_node(
            9_000,
            &node,
            vec![0.25, 0.75],
            "inline://placement/evidence/9000",
        )
        .expect("placement should be valid");

    assert_eq!(placement.placement_id, 9_000);
    assert_eq!(placement.memory_node_id, 1_500);
    assert_eq!(placement.gradient_space_id, 44);
    assert_eq!(placement.gradient_space_version, 3);
    assert_eq!(placement.coordinates, vec![0.25, 0.75]);
    assert_eq!(
        placement.placement_evidence_ref,
        "inline://placement/evidence/9000"
    );
}

#[test]
fn memory_placement_rejects_coordinate_axis_mismatch() {
    let space = GradientSpace::new(
        44,
        "mock-memory",
        3,
        vec![
            GradientAxis::new(1, "domain", "general", "specialized", 0.9)
                .expect("axis should be valid"),
        ],
    )
    .expect("space should be valid");
    let node = MemoryNode::from_packet(
        1_500,
        MemoryNodeKind::Episode,
        1,
        11,
        1_200,
        vec!["inline://summary/1200".to_string()],
        "completed constructor task",
    )
    .expect("memory node should be valid");

    let err = space
        .place_memory_node(
            9_000,
            &node,
            vec![0.25, 0.75],
            "inline://placement/evidence/9000",
        )
        .expect_err("coordinate count must match axes");

    assert_eq!(
        err,
        LoomModelError::InvalidNumericField {
            field: "coordinates",
            reason: "must match gradient axis count",
        }
    );
}

#[test]
fn gradient_space_produces_radius_limited_memory_candidates() {
    let space = two_axis_space();
    let placements = vec![
        placement(9_000, 1_500, 44, 3, vec![0.20, 0.20]),
        placement(9_001, 1_501, 44, 3, vec![0.50, 0.50]),
        placement(9_002, 1_502, 44, 3, vec![0.90, 0.90]),
    ];

    let candidates = space
        .memory_candidates(&placements, vec![0.0, 0.0], 1.0, 2)
        .expect("candidate selection should succeed");

    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.memory_node_id)
            .collect::<Vec<_>>(),
        vec![1_500, 1_501]
    );
    assert!(candidates[0].distance_sq < candidates[1].distance_sq);
    assert_eq!(candidates[0].gradient_space_version, 3);
}

#[test]
fn memory_candidates_ignore_other_gradient_space_versions() {
    let space = two_axis_space();
    let placements = vec![
        placement(9_000, 1_500, 44, 2, vec![0.01, 0.01]),
        placement(9_001, 1_501, 44, 3, vec![0.50, 0.50]),
        placement(9_002, 1_502, 45, 3, vec![0.01, 0.01]),
    ];

    let candidates = space
        .memory_candidates(&placements, vec![0.0, 0.0], 1.0, 8)
        .expect("candidate selection should succeed");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].memory_node_id, 1_501);
}

#[test]
fn memory_candidates_reject_query_axis_mismatch() {
    let space = two_axis_space();
    let err = space
        .memory_candidates(&[], vec![0.25], 1.0, 8)
        .expect_err("query coordinates should match axis count");

    assert_eq!(
        err,
        LoomModelError::InvalidNumericField {
            field: "query_coordinates",
            reason: "must match gradient axis count",
        }
    );
}

#[test]
fn memory_candidates_reject_negative_radius() {
    let space = two_axis_space();
    let err = space
        .memory_candidates(&[], vec![0.25, 0.25], -1.0, 8)
        .expect_err("radius should be non-negative");

    assert_eq!(
        err,
        LoomModelError::InvalidNumericField {
            field: "radius_sq",
            reason: "must be non-negative",
        }
    );
}

#[test]
fn memory_candidates_allow_zero_budget() {
    let space = two_axis_space();
    let placements = vec![placement(9_000, 1_500, 44, 3, vec![0.01, 0.01])];

    let candidates = space
        .memory_candidates(&placements, vec![0.0, 0.0], 1.0, 0)
        .expect("zero budget should be a valid empty candidate request");

    assert!(candidates.is_empty());
}

fn two_axis_space() -> GradientSpace {
    GradientSpace::new(
        44,
        "mock-memory",
        3,
        vec![
            GradientAxis::new(1, "domain", "general", "specialized", 0.9)
                .expect("axis should be valid"),
            GradientAxis::new(2, "tool-affinity", "low", "high", 0.8)
                .expect("axis should be valid"),
        ],
    )
    .expect("space should be valid")
}

fn placement(
    placement_id: u64,
    memory_node_id: u64,
    gradient_space_id: u64,
    gradient_space_version: u64,
    coordinates: Vec<f32>,
) -> MemoryPlacement {
    MemoryPlacement {
        placement_id,
        memory_node_id,
        gradient_space_id,
        gradient_space_version,
        coordinates,
        placement_evidence_ref: format!("inline://placement/evidence/{placement_id}"),
    }
}
