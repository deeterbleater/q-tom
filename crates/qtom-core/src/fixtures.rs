use crate::types::{AgentLabels, AgentProfile, AgentRuntimeState, DEFAULT_DIM, RoutingRequest};

#[derive(Clone, Copy, Debug)]
pub struct FixtureConfig {
    pub agent_count: usize,
    pub task_count: usize,
    pub dimensions: usize,
    pub k: usize,
    pub seed: u64,
}

impl Default for FixtureConfig {
    fn default() -> Self {
        Self {
            agent_count: 128,
            task_count: 32,
            dimensions: DEFAULT_DIM,
            k: 8,
            seed: 0x5154_4f4d,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Fixture {
    pub agents: Vec<AgentProfile>,
    pub states: Vec<AgentRuntimeState>,
    pub requests: Vec<RoutingRequest>,
}

pub fn generate_fixture(config: FixtureConfig) -> Fixture {
    let mut rng = Lcg::new(config.seed);
    let cluster_count = 8usize.min(config.agent_count.max(1));
    let centers = generate_centers(&mut rng, cluster_count, config.dimensions);
    let mut agents = Vec::with_capacity(config.agent_count);
    let mut states = Vec::with_capacity(config.agent_count);

    for idx in 0..config.agent_count {
        let cluster = idx % cluster_count;
        let vector = jittered(&mut rng, &centers[cluster], 0.035);
        agents.push(AgentProfile {
            id: idx as u32,
            vector,
            labels: AgentLabels {
                model_profile: 1,
                tool_profile: cluster as u16,
                mcp_profile: (idx % 4) as u16,
                memory_profile: (idx % 16) as u16,
                cost_class: (idx % 3) as u8,
                latency_class: (idx % 5) as u8,
            },
        });

        states.push(AgentRuntimeState {
            queue_depth_norm: if idx % 17 == 0 { 0.45 } else { 0.0 },
            latency_norm: if idx % 19 == 0 { 0.35 } else { 0.0 },
            cache_pressure_norm: if idx % 23 == 0 { 0.25 } else { 0.0 },
            availability: if idx % 29 == 0 { 0 } else { 1 },
        });
    }

    let mut requests = Vec::with_capacity(config.task_count);
    for task_idx in 0..config.task_count {
        let cluster = task_idx % cluster_count;
        requests.push(RoutingRequest {
            task_id: task_idx as u64,
            vector: jittered(&mut rng, &centers[cluster], 0.020),
            k: config.k,
            fallback_generalist_id: 0,
            radius_max_threshold: 10.0,
        });
    }

    Fixture {
        agents,
        states,
        requests,
    }
}

fn generate_centers(rng: &mut Lcg, count: usize, dimensions: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|_| (0..dimensions).map(|_| rng.next_f32()).collect())
        .collect()
}

fn jittered(rng: &mut Lcg, center: &[f32], scale: f32) -> Vec<f32> {
    center
        .iter()
        .map(|value| (value + rng.next_signed_f32() * scale).clamp(0.0, 1.0))
        .collect()
}

#[derive(Clone, Debug)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn next_signed_f32(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_are_deterministic_for_seed() {
        let config = FixtureConfig {
            agent_count: 16,
            task_count: 4,
            dimensions: 8,
            k: 3,
            seed: 1234,
        };

        let left = generate_fixture(config);
        let right = generate_fixture(config);

        assert_eq!(left.agents, right.agents);
        assert_eq!(left.states, right.states);
        assert_eq!(left.requests, right.requests);
    }

    #[test]
    fn default_fixture_starts_at_128_agents() {
        let fixture = generate_fixture(FixtureConfig::default());

        assert_eq!(fixture.agents.len(), 128);
        assert_eq!(fixture.requests[0].k, 8);
    }
}
