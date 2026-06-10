use crate::score::{ScoreCoefficients, score_agent};
use crate::types::{
    AgentProfile, AgentRuntimeState, RouteCandidate, RouteDebugInfo, RouteError, RoutingRequest,
    RoutingResult,
};

pub trait RouterBackend {
    fn route_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError>;
}

#[derive(Clone, Debug)]
pub struct CpuRouter {
    agents: Vec<AgentProfile>,
    coefficients: ScoreCoefficients,
    debug_observed: bool,
}

impl CpuRouter {
    pub fn new(agents: Vec<AgentProfile>, coefficients: ScoreCoefficients) -> Self {
        Self {
            agents,
            coefficients,
            debug_observed: true,
        }
    }

    pub fn with_debug_observed(mut self, debug_observed: bool) -> Self {
        self.debug_observed = debug_observed;
        self
    }

    pub fn route_one(
        &self,
        request: &RoutingRequest,
        states: &[AgentRuntimeState],
    ) -> Result<RoutingResult, RouteError> {
        self.validate_request(request, states)?;

        let mut observed = Vec::with_capacity(self.agents.len());
        let mut available = Vec::with_capacity(self.agents.len());

        for (agent, state) in self.agents.iter().zip(states.iter().copied()) {
            let candidate = score_agent(&request.vector, agent, state, self.coefficients)?;
            observed.push(candidate.clone());
            if candidate.available {
                available.push(candidate);
            }
        }

        sort_candidates(&mut observed, CandidateSort::Observed);
        sort_candidates(&mut available, CandidateSort::Available);

        let observed_top_k = take_k(observed, request.k);
        let mut available_top_k = take_k(available, request.k);
        let ideal_candidate_unavailable = observed_top_k
            .first()
            .map(|candidate| !candidate.available)
            .unwrap_or(false);

        let used_fallback = available_top_k
            .first()
            .map(|candidate| candidate.base_distance > request.radius_max_threshold)
            .unwrap_or(true);

        if used_fallback {
            available_top_k.push(RouteCandidate {
                agent_id: request.fallback_generalist_id,
                effective_distance: f32::INFINITY,
                base_distance: f32::INFINITY,
                omega: 1.0,
                queue_penalty: 0.0,
                latency_penalty: 0.0,
                cache_penalty: 0.0,
                available: true,
            });
        }

        Ok(RoutingResult {
            task_id: request.task_id,
            available_candidates: available_top_k,
            used_fallback,
            ideal_candidate_unavailable,
            debug: self.debug_observed.then_some(RouteDebugInfo {
                observed_candidates: observed_top_k,
            }),
        })
    }

    fn validate_request(
        &self,
        request: &RoutingRequest,
        states: &[AgentRuntimeState],
    ) -> Result<(), RouteError> {
        if self.agents.is_empty() {
            return Err(RouteError::EmptyAgents);
        }
        if self.agents.len() != states.len() {
            return Err(RouteError::StateLengthMismatch {
                agents: self.agents.len(),
                states: states.len(),
            });
        }

        let expected = self.agents[0].vector.len();
        if request.vector.len() != expected {
            return Err(RouteError::DimensionMismatch {
                expected,
                actual: request.vector.len(),
                context: "routing request",
            });
        }

        for agent in &self.agents {
            if agent.vector.len() != expected {
                return Err(RouteError::DimensionMismatch {
                    expected,
                    actual: agent.vector.len(),
                    context: "agent vector",
                });
            }
        }

        Ok(())
    }
}

impl RouterBackend for CpuRouter {
    fn route_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError> {
        requests
            .iter()
            .map(|request| self.route_one(request, states))
            .collect()
    }
}

#[derive(Clone, Copy)]
enum CandidateSort {
    Observed,
    Available,
}

fn sort_candidates(candidates: &mut [RouteCandidate], sort: CandidateSort) {
    candidates.sort_by(|left, right| {
        let left_key = match sort {
            CandidateSort::Observed => left.base_distance,
            CandidateSort::Available => left.effective_distance,
        };
        let right_key = match sort {
            CandidateSort::Observed => right.base_distance,
            CandidateSort::Available => right.effective_distance,
        };

        left_key
            .total_cmp(&right_key)
            .then_with(|| left.agent_id.cmp(&right.agent_id))
    });
}

fn take_k(mut candidates: Vec<RouteCandidate>, k: usize) -> Vec<RouteCandidate> {
    candidates.truncate(k);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentLabels, AgentRuntimeState, RoutingRequest};

    #[test]
    fn route_returns_available_candidates_and_debug_observed_candidates() {
        let router = CpuRouter::new(
            vec![
                agent(1, &[0.0, 0.0]),
                agent(2, &[0.1, 0.0]),
                agent(3, &[1.0, 1.0]),
            ],
            ScoreCoefficients::default(),
        );
        let states = vec![
            AgentRuntimeState::unavailable(),
            AgentRuntimeState::available(),
            AgentRuntimeState::available(),
        ];
        let request = request(&[0.05, 0.0], 2, 999, 10.0);

        let result = router.route_one(&request, &states).unwrap();

        assert!(result.ideal_candidate_unavailable);
        assert_eq!(result.available_candidates[0].agent_id, 2);
        assert_eq!(
            result
                .debug
                .as_ref()
                .unwrap()
                .observed_candidates
                .iter()
                .map(|candidate| candidate.agent_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn runtime_queue_penalty_can_shift_available_winner() {
        let router = CpuRouter::new(
            vec![agent(1, &[0.0, 0.0]), agent(2, &[0.2, 0.0])],
            ScoreCoefficients {
                alpha_queue: 100.0,
                beta_latency: 0.0,
                gamma_cache: 0.0,
            },
        );
        let states = vec![
            AgentRuntimeState {
                queue_depth_norm: 1.0,
                ..AgentRuntimeState::available()
            },
            AgentRuntimeState::available(),
        ];
        let request = request(&[0.05, 0.0], 2, 999, 10.0);

        let result = router.route_one(&request, &states).unwrap();

        assert_eq!(result.available_candidates[0].agent_id, 2);
        assert_eq!(
            result.debug.unwrap().observed_candidates[0].agent_id,
            1,
            "observed candidates remain sorted by semantic/base distance"
        );
    }

    #[test]
    fn fallback_is_appended_when_best_available_exceeds_radius() {
        let router = CpuRouter::new(
            vec![agent(1, &[10.0, 10.0]), agent(2, &[11.0, 11.0])],
            ScoreCoefficients::default(),
        );
        let states = vec![
            AgentRuntimeState::available(),
            AgentRuntimeState::available(),
        ];
        let request = request(&[0.0, 0.0], 2, 42, 1.0);

        let result = router.route_one(&request, &states).unwrap();

        assert!(result.used_fallback);
        assert_eq!(result.available_candidates.last().unwrap().agent_id, 42);
    }

    fn agent(id: u32, vector: &[f32]) -> AgentProfile {
        AgentProfile {
            id,
            vector: vector.to_vec(),
            labels: AgentLabels::default(),
        }
    }

    fn request(
        vector: &[f32],
        k: usize,
        fallback_generalist_id: u32,
        radius: f32,
    ) -> RoutingRequest {
        RoutingRequest {
            task_id: 100,
            vector: vector.to_vec(),
            k,
            fallback_generalist_id,
            radius_max_threshold: radius,
        }
    }
}
