pub mod backend;
pub mod config;
pub mod cpu_router;
pub mod fixtures;
pub mod golden;
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
pub use metrics::{BatchMetrics, RouteMetrics, batch_metrics, route_metrics};
pub use route_table::AgentRouteTable;
pub use score::{ScoreCoefficients, score_agent};
pub use types::{
    AgentLabels, AgentProfile, AgentRuntimeState, RouteCandidate, RouteDebugInfo, RouteError,
    RoutingRequest, RoutingResult,
};
