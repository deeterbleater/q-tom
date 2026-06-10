use crate::types::{AgentProfile, AgentRuntimeState, RouteCandidate, RouteError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreCoefficients {
    pub alpha_queue: f32,
    pub beta_latency: f32,
    pub gamma_cache: f32,
}

impl Default for ScoreCoefficients {
    fn default() -> Self {
        Self {
            alpha_queue: 0.35,
            beta_latency: 0.25,
            gamma_cache: 0.20,
        }
    }
}

pub fn score_agent(
    task_vector: &[f32],
    agent: &AgentProfile,
    state: AgentRuntimeState,
    coefficients: ScoreCoefficients,
) -> Result<RouteCandidate, RouteError> {
    if task_vector.len() != agent.vector.len() {
        return Err(RouteError::DimensionMismatch {
            expected: agent.vector.len(),
            actual: task_vector.len(),
            context: "task vector",
        });
    }

    let base_distance = dist_sq(task_vector, &agent.vector);
    let queue_penalty = coefficients.alpha_queue * state.queue_depth_norm;
    let latency_penalty = coefficients.beta_latency * state.latency_norm;
    let cache_penalty = coefficients.gamma_cache * state.cache_pressure_norm;
    let omega = 1.0 + queue_penalty + latency_penalty + cache_penalty;
    let available = state.is_available();
    let effective_distance = if available {
        base_distance * omega
    } else {
        f32::INFINITY
    };

    Ok(RouteCandidate {
        agent_id: agent.id,
        effective_distance,
        base_distance,
        omega,
        queue_penalty,
        latency_penalty,
        cache_penalty,
        available,
    })
}

pub fn dist_sq(lhs: &[f32], rhs: &[f32]) -> f32 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(left, right)| {
            let diff = left - right;
            diff * diff
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentLabels, AgentProfile, AgentRuntimeState};

    #[test]
    fn score_includes_runtime_penalties() {
        let agent = AgentProfile {
            id: 7,
            vector: vec![1.0, 1.0],
            labels: AgentLabels::default(),
        };
        let state = AgentRuntimeState {
            queue_depth_norm: 0.5,
            latency_norm: 0.25,
            cache_pressure_norm: 0.0,
            availability: 1,
        };
        let coefficients = ScoreCoefficients {
            alpha_queue: 2.0,
            beta_latency: 4.0,
            gamma_cache: 8.0,
        };

        let candidate = score_agent(&[0.0, 0.0], &agent, state, coefficients).unwrap();

        assert_eq!(candidate.base_distance, 2.0);
        assert_eq!(candidate.queue_penalty, 1.0);
        assert_eq!(candidate.latency_penalty, 1.0);
        assert_eq!(candidate.cache_penalty, 0.0);
        assert_eq!(candidate.omega, 3.0);
        assert_eq!(candidate.effective_distance, 6.0);
    }

    #[test]
    fn unavailable_agent_has_infinite_effective_distance() {
        let agent = AgentProfile {
            id: 1,
            vector: vec![0.0, 0.0],
            labels: AgentLabels::default(),
        };

        let candidate = score_agent(
            &[0.0, 0.0],
            &agent,
            AgentRuntimeState::unavailable(),
            ScoreCoefficients::default(),
        )
        .unwrap();

        assert!(!candidate.available);
        assert!(candidate.effective_distance.is_infinite());
        assert_eq!(candidate.base_distance, 0.0);
    }
}
