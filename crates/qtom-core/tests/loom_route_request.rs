use qtom_core::{
    AgentRuntimeState, CpuRouter, InMemoryEventLog, LoomEvent, RouteDecision, ScoreCoefficients,
    TaskEnvelope, TaskRouteDecisionEventConfig, TaskRouteRequestConfig,
    build_route_request_from_task, route_decision_recorded_event, simulated_agents_for_requests,
};

fn child_task() -> TaskEnvelope {
    TaskEnvelope::child(11, 1, 10, 7, 42, 88, "build constructor artifact")
        .expect("child task should be valid")
}

#[test]
fn route_request_from_task_preserves_task_and_policy_knobs() {
    let request = build_route_request_from_task(
        &child_task(),
        TaskRouteRequestConfig {
            dimensions: 16,
            k: 3,
            fallback_generalist_id: 999,
            radius_max_threshold: 0.75,
        },
    )
    .expect("route request should build");

    assert_eq!(request.task_id, 11);
    assert_eq!(request.k, 3);
    assert_eq!(request.fallback_generalist_id, 999);
    assert_eq!(request.radius_max_threshold, 0.75);
    assert_eq!(request.vector.len(), 16);
}

#[test]
fn route_request_vector_is_deterministic_for_same_task() {
    let first = build_route_request_from_task(&child_task(), TaskRouteRequestConfig::default())
        .expect("first request should build");
    let second = build_route_request_from_task(&child_task(), TaskRouteRequestConfig::default())
        .expect("second request should build");

    assert_eq!(first.vector, second.vector);
}

#[test]
fn route_request_vector_changes_with_task_lineage() {
    let first = build_route_request_from_task(&child_task(), TaskRouteRequestConfig::default())
        .expect("first request should build");
    let other_task = TaskEnvelope::child(12, 1, 10, 7, 42, 88, "build constructor artifact")
        .expect("other child task should be valid");
    let second = build_route_request_from_task(&other_task, TaskRouteRequestConfig::default())
        .expect("second request should build");

    assert_ne!(first.vector, second.vector);
}

#[test]
fn route_request_rejects_zero_dimensions() {
    let err = build_route_request_from_task(
        &child_task(),
        TaskRouteRequestConfig {
            dimensions: 0,
            ..TaskRouteRequestConfig::default()
        },
    )
    .expect_err("zero dimensions should fail");

    assert_eq!(err.to_string(), "`dimensions` must be greater than zero");
}

#[test]
fn child_task_route_requests_can_be_routed_through_cpu_router() {
    let tasks = [
        child_task(),
        TaskEnvelope::child(12, 1, 10, 7, 42, 88, "validate constructor artifact")
            .expect("second child task should be valid"),
    ];
    let requests = tasks
        .iter()
        .map(|task| build_route_request_from_task(task, TaskRouteRequestConfig::default()))
        .collect::<Result<Vec<_>, _>>()
        .expect("requests should build");
    let agents = simulated_agents_for_requests(&requests, 10_000);
    let states = vec![AgentRuntimeState::available(); agents.len()];
    let router = CpuRouter::new(agents, ScoreCoefficients::default());

    let results = router
        .route_batch_with_workers(&requests, &states, 1)
        .expect("cpu router should route mock loom tasks");

    assert_eq!(
        results
            .iter()
            .map(|result| result.task_id)
            .collect::<Vec<_>>(),
        vec![11, 12]
    );
    assert_eq!(results[0].available_candidates[0].agent_id, 10_000);
    assert_eq!(results[1].available_candidates[0].agent_id, 10_001);
    assert!(results.iter().all(|result| !result.used_fallback));
}

#[test]
fn route_decision_preserves_available_and_observed_telemetry() {
    let request = build_route_request_from_task(&child_task(), TaskRouteRequestConfig::default())
        .expect("request should build");
    let agents = simulated_agents_for_requests(&[request.clone()], 10_000);
    let states = vec![
        AgentRuntimeState::unavailable(),
        AgentRuntimeState::available(),
    ];
    let router = CpuRouter::new(
        vec![
            agents[0].clone(),
            qtom_core::AgentProfile {
                id: 10_001,
                vector: request
                    .vector
                    .iter()
                    .map(|value| value + 0.1)
                    .collect::<Vec<_>>(),
                labels: qtom_core::AgentLabels::default(),
            },
        ],
        ScoreCoefficients::default(),
    );
    let result = router
        .route_one(&request, &states)
        .expect("router should produce result");

    let decision = RouteDecision::from_result(501, 99, "cpu", "mock-routing-v1", &result)
        .expect("route decision should build");

    assert_eq!(decision.route_decision_id, 501);
    assert_eq!(decision.task_id, 11);
    assert_eq!(decision.route_policy_id, 99);
    assert_eq!(decision.route_backend, "cpu");
    assert_eq!(decision.selected_agent_id, 10_001);
    assert_eq!(decision.available_candidate_ids, vec![10_001]);
    assert_eq!(decision.observed_candidate_ids, vec![10_000, 10_001]);
    assert!(decision.ideal_candidate_unavailable);
    assert!(decision.substitute_distance_delta > 0.0);
    assert!(!decision.used_fallback);
}

#[test]
fn route_decision_event_records_replayable_route_metadata() {
    let decision = RouteDecision {
        route_decision_id: 501,
        task_id: 11,
        route_policy_id: 99,
        route_backend: "cpu".to_string(),
        policy_version: "mock-routing-v1".to_string(),
        selected_agent_id: 10_001,
        available_candidate_ids: vec![10_001],
        observed_candidate_ids: vec![10_000, 10_001],
        ideal_candidate_unavailable: true,
        substitute_distance_delta: 0.1,
        used_fallback: false,
    };

    let event = route_decision_recorded_event(
        &decision,
        TaskRouteDecisionEventConfig {
            event_id: 3_000,
            root_task_id: 1,
            prompt_id: 7,
            topology_snapshot_id: 44,
            occurred_at_ms: 12_345,
            correlation_id: 77,
        },
    );

    assert_eq!(event.event_id, 3_000);
    assert_eq!(
        event.event_type,
        qtom_core::LoomEventType::RouteDecisionRecorded
    );
    assert_eq!(event.root_task_id, 1);
    assert_eq!(event.task_id, Some(11));
    assert_eq!(event.prompt_id, Some(7));
    assert_eq!(event.topology_snapshot_id, Some(44));
    assert_eq!(event.payload_schema, "qtom.route_decision.v1");
    assert_eq!(event.payload_ref, "inline://route-decision/501");
}

#[test]
fn route_decision_event_supports_assignment_causation() {
    let decision = RouteDecision {
        route_decision_id: 501,
        task_id: 11,
        route_policy_id: 99,
        route_backend: "cpu".to_string(),
        policy_version: "mock-routing-v1".to_string(),
        selected_agent_id: 10_001,
        available_candidate_ids: vec![10_001],
        observed_candidate_ids: vec![10_000, 10_001],
        ideal_candidate_unavailable: true,
        substitute_distance_delta: 0.1,
        used_fallback: false,
    };
    let route_event = route_decision_recorded_event(
        &decision,
        TaskRouteDecisionEventConfig {
            event_id: 3_000,
            root_task_id: 1,
            prompt_id: 7,
            topology_snapshot_id: 44,
            occurred_at_ms: 12_345,
            correlation_id: 77,
        },
    );
    let assignment_event = LoomEvent {
        event_id: 3_001,
        event_type: qtom_core::LoomEventType::TaskAssigned,
        root_task_id: 1,
        task_id: Some(11),
        parent_task_id: Some(10),
        prompt_id: Some(7),
        agent_id: Some(10_001),
        agent_role: Some("constructor".to_string()),
        topology_snapshot_id: None,
        payload_schema: "qtom.task_assignment.v1".to_string(),
        payload_ref: "inline://task-assignment/11/10001".to_string(),
        occurred_at_ms: 12_346,
        causation_id: Some(route_event.event_id),
        correlation_id: 77,
    };
    let mut log = InMemoryEventLog::new();

    log.append(route_event)
        .expect("route decision event should append");
    log.append(assignment_event)
        .expect("assignment caused by route decision should append");

    let report = log
        .validate_replay()
        .expect("route assignment log should replay");
    assert_eq!(report.route_decision_count, 1);
    assert_eq!(report.assignment_count, 1);
}
