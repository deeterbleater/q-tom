pub mod backend;
pub mod config;
pub mod cpu_router;
pub mod fixtures;
pub mod golden;
pub mod loom;
pub mod loom_mock;
pub mod loom_model;
pub mod loom_projection;
pub mod loom_route;
pub mod loom_runtime;
pub mod metrics;
pub mod route_table;
pub mod score;
pub mod types;

pub use backend::{
    BackendMismatch, BackendParityError, BackendParityReport, BackendParityTolerance,
    RouterBackend, assert_backend_parity, assert_backend_parity_with_tolerance,
    routing_results_checksum,
};
pub use config::ProjectConfig;
pub use cpu_router::CpuRouter;
pub use fixtures::{Fixture, FixtureConfig, generate_fixture};
pub use golden::{GoldenFixture, GoldenFixtureError, read_golden_fixture, write_golden_fixture};
pub use loom::{
    InMemoryEventLog, LoomEvent, LoomEventError, LoomEventType, ReplayCursor,
    ReplayValidationReport, append_event_log_jsonl, read_event_log_jsonl, validate_events,
    write_event_log_jsonl,
};
pub use loom_mock::{
    ConstructorOutput, CuratorOutput, DirectorOutput, IntegrationOutput, MockConstructor,
    MockConstructorConfig, MockCurator, MockCuratorConfig, MockDirector, MockDirectorConfig,
    MockIntegration, MockIntegrationConfig, MockTaskLoom, MockTaskLoomOutput,
};
pub use loom_model::{
    AgentDecommissionPacket, ArtifactRef, DependencyEdge, DependencyKind, EvaluationFixture,
    EvaluatorConfig, GradientAxis, GradientSpace, IntegrationGroup, IntegrationReport,
    IntegrationStatus, JoinPolicy, LoomModelError, MemoryCandidate, MemoryCandidateReport,
    MemoryNode, MemoryNodeKind, MemoryPlacement, PlanNode, TaskEnvelope,
    append_decommission_packet_jsonl, append_evaluation_fixture_jsonl,
    read_decommission_packets_jsonl, write_decommission_packets_jsonl,
    read_evaluation_fixtures_jsonl, write_evaluation_fixtures_jsonl,
};
pub use loom_projection::{
    LoomProjectionBundle, LoomReplayReport, artifact_provenance_projection,
    integration_group_projection, loom_projection_bundle, loom_replay_report,
    memory_lineage_projection, route_trace_projection, task_dependency_projection,
};
pub use loom_route::{
    RouteDecision, TaskRouteDecisionEventConfig, TaskRouteRequestConfig,
    build_route_request_from_task, route_decision_recorded_event, simulated_agents_for_requests,
};
pub use loom_runtime::{
    AgentExecutionResult, AgentRuntime, HydratedContext, MockConstructorRuntime,
    MockConstructorRuntimeConfig,
};
pub use metrics::{BatchMetrics, RouteMetrics, batch_metrics, route_metrics};
pub use route_table::AgentRouteTable;
pub use score::{ScoreCoefficients, score_agent};
pub use types::{
    AgentLabels, AgentProfile, AgentRuntimeState, RouteCandidate, RouteDebugInfo, RouteError,
    RoutingRequest, RoutingResult,
};
