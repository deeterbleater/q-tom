use crate::types::DEFAULT_DIM;
use crate::{AgentLabels, AgentProfile, LoomModelError, RoutingRequest, TaskEnvelope};

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

fn stable_text_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        hash.wrapping_mul(0x100_0000_01b3) ^ u64::from(byte)
    })
}

fn unit_interval(value: u64) -> f32 {
    let mixed = value ^ (value >> 33).wrapping_mul(0xff51_afd7_ed55_8ccd);
    (mixed % 10_000) as f32 / 10_000.0
}
