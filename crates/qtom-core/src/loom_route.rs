use crate::types::DEFAULT_DIM;
use crate::{
    AgentLabels, AgentProfile, LoomEvent, LoomEventType, LoomModelError, RoutingRequest,
    RoutingResult, TaskEnvelope,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaskRouteRequestConfig {
    pub dimensions: usize,
    pub k: usize,
    pub fallback_generalist_id: u32,
    pub radius_max_threshold: f32,
}

impl Default for TaskRouteRequestConfig {
    fn default() -> Self {
        Self {
            dimensions: DEFAULT_DIM,
            k: 3,
            fallback_generalist_id: 999,
            radius_max_threshold: 0.75,
        }
    }
}

pub fn build_route_request_from_task(
    task: &TaskEnvelope,
    config: TaskRouteRequestConfig,
) -> Result<RoutingRequest, LoomModelError> {
    if config.dimensions == 0 {
        return Err(LoomModelError::InvalidNumericField {
            field: "dimensions",
            reason: "must be greater than zero",
        });
    }

    let mut vector = vec![0.0; config.dimensions];
    let seeds = [
        task.task_id,
        task.root_task_id,
        task.parent_task_id.unwrap_or_default(),
        task.prompt_id,
        task.plan_id,
        task.integration_group_id,
        stable_text_hash(&task.summary),
    ];

    for (index, slot) in vector.iter_mut().enumerate() {
        let seed = seeds[index % seeds.len()].wrapping_add(index as u64 * 31);
        *slot = unit_interval(seed);
    }

    Ok(RoutingRequest {
        task_id: task.task_id,
        vector,
        k: config.k,
        fallback_generalist_id: config.fallback_generalist_id,
        radius_max_threshold: config.radius_max_threshold,
    })
}

pub fn simulated_agents_for_requests(
    requests: &[RoutingRequest],
    first_agent_id: u32,
) -> Vec<AgentProfile> {
    requests
        .iter()
        .enumerate()
        .map(|(index, request)| AgentProfile {
            id: first_agent_id + index as u32,
            vector: request.vector.clone(),
            labels: AgentLabels {
                memory_profile: (request.task_id % u16::MAX as u64) as u16,
                ..AgentLabels::default()
            },
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteDecision {
    pub route_decision_id: u64,
    pub task_id: u64,
    pub route_policy_id: u64,
    pub route_backend: String,
    pub policy_version: String,
    pub selected_agent_id: u32,
    pub available_candidate_ids: Vec<u32>,
    pub observed_candidate_ids: Vec<u32>,
    pub ideal_candidate_unavailable: bool,
    pub substitute_distance_delta: f32,
    pub used_fallback: bool,
}

impl RouteDecision {
    pub fn from_result(
        route_decision_id: u64,
        route_policy_id: u64,
        route_backend: impl Into<String>,
        policy_version: impl Into<String>,
        result: &RoutingResult,
    ) -> Result<Self, LoomModelError> {
        let selected = result
            .available_candidates
            .first()
            .ok_or(LoomModelError::MissingRouteCandidate(result.task_id))?;
        let observed_candidates = result
            .debug
            .as_ref()
            .map(|debug| debug.observed_candidates.as_slice())
            .unwrap_or(&[]);
        let observed_ideal = observed_candidates.first();
        let substitute_distance_delta = observed_ideal
            .map(|ideal| selected.base_distance - ideal.base_distance)
            .unwrap_or(0.0);

        Ok(Self {
            route_decision_id,
            task_id: result.task_id,
            route_policy_id,
            route_backend: route_backend.into(),
            policy_version: policy_version.into(),
            selected_agent_id: selected.agent_id,
            available_candidate_ids: result
                .available_candidates
                .iter()
                .map(|candidate| candidate.agent_id)
                .collect(),
            observed_candidate_ids: observed_candidates
                .iter()
                .map(|candidate| candidate.agent_id)
                .collect(),
            ideal_candidate_unavailable: result.ideal_candidate_unavailable,
            substitute_distance_delta,
            used_fallback: result.used_fallback,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskRouteDecisionEventConfig {
    pub event_id: u64,
    pub root_task_id: u64,
    pub prompt_id: u64,
    pub topology_snapshot_id: u64,
    pub occurred_at_ms: u64,
    pub correlation_id: u64,
}

pub fn route_decision_recorded_event(
    decision: &RouteDecision,
    config: TaskRouteDecisionEventConfig,
) -> LoomEvent {
    LoomEvent {
        event_id: config.event_id,
        event_type: LoomEventType::RouteDecisionRecorded,
        root_task_id: config.root_task_id,
        task_id: Some(decision.task_id),
        parent_task_id: None,
        prompt_id: Some(config.prompt_id),
        agent_id: None,
        agent_role: None,
        topology_snapshot_id: Some(config.topology_snapshot_id),
        payload_schema: "qtom.route_decision.v1".to_string(),
        payload_ref: format!("inline://route-decision/{}", decision.route_decision_id),
        occurred_at_ms: config.occurred_at_ms,
        causation_id: None,
        correlation_id: config.correlation_id,
    }
}

fn stable_text_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        hash.wrapping_mul(0x100_0000_01b3) ^ u64::from(byte)
    })
}

fn unit_interval(value: u64) -> f32 {
    let mixed = value ^ (value >> 33).wrapping_mul(0xff51_afd7_ed55_8ccd);
    (mixed % 10_000) as f32 / 10_000.0
}
