use qtom_core::{
    AgentRuntimeState, CpuRouter, RouteDecision, ScoreCoefficients, TaskEnvelope,
    TaskRouteRequestConfig, build_route_request_from_task, simulated_agents_for_requests,
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
