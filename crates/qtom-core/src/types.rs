pub const DEFAULT_DIM: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct AgentProfile {
    pub id: u32,
    pub vector: Vec<f32>,
    pub labels: AgentLabels,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentLabels {
    pub model_profile: u16,
    pub tool_profile: u16,
    pub mcp_profile: u16,
    pub memory_profile: u16,
    pub cost_class: u8,
    pub latency_class: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentRuntimeState {
    pub queue_depth_norm: f32,
    pub latency_norm: f32,
    pub cache_pressure_norm: f32,
    pub availability: u32,
}

impl AgentRuntimeState {
    pub fn available() -> Self {
        Self {
            queue_depth_norm: 0.0,
            latency_norm: 0.0,
            cache_pressure_norm: 0.0,
            availability: 1,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            availability: 0,
            ..Self::available()
        }
    }

    pub fn is_available(self) -> bool {
        self.availability != 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutingRequest {
    pub task_id: u64,
    pub vector: Vec<f32>,
    pub k: usize,
    pub fallback_generalist_id: u32,
    pub radius_max_threshold: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutingResult {
    pub task_id: u64,
    pub available_candidates: Vec<RouteCandidate>,
    pub used_fallback: bool,
    pub ideal_candidate_unavailable: bool,
    pub debug: Option<RouteDebugInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteDebugInfo {
    pub observed_candidates: Vec<RouteCandidate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteCandidate {
    pub agent_id: u32,
    pub effective_distance: f32,
    pub base_distance: f32,
    pub omega: f32,
    pub queue_penalty: f32,
    pub latency_penalty: f32,
    pub cache_penalty: f32,
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteError {
    EmptyAgents,
    BackendUnavailable {
        backend: &'static str,
        reason: &'static str,
    },
    DimensionMismatch {
        expected: usize,
        actual: usize,
        context: &'static str,
    },
    StateLengthMismatch {
        agents: usize,
        states: usize,
    },
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteError::EmptyAgents => write!(f, "router has no agents"),
            RouteError::BackendUnavailable { backend, reason } => {
                write!(f, "{backend} backend unavailable: {reason}")
            }
            RouteError::DimensionMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "{context} dimension mismatch: expected {expected}, got {actual}"
            ),
            RouteError::StateLengthMismatch { agents, states } => write!(
                f,
                "runtime state length mismatch: {agents} agents, {states} states"
            ),
        }
    }
}

impl std::error::Error for RouteError {}
