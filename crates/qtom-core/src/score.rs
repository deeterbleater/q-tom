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

    let score = score_components_for_vector(task_vector, &agent.vector, state, coefficients);

    Ok(RouteCandidate {
        agent_id: agent.id,
        effective_distance: score.effective_distance,
        base_distance: score.base_distance,
        omega: score.omega,
        queue_penalty: score.queue_penalty,
        latency_penalty: score.latency_penalty,
        cache_penalty: score.cache_penalty,
        available: score.available,
    })
}

pub fn score_components(
    task_vector: &[f32],
    agent: &AgentProfile,
    state: AgentRuntimeState,
    coefficients: ScoreCoefficients,
) -> Result<ScoreComponents, RouteError> {
    if task_vector.len() != agent.vector.len() {
        return Err(RouteError::DimensionMismatch {
            expected: agent.vector.len(),
            actual: task_vector.len(),
            context: "task vector",
        });
    }

    Ok(score_components_for_vector(
        task_vector,
        &agent.vector,
        state,
        coefficients,
    ))
}

#[inline(always)]
pub fn score_components_for_vector(
    task_vector: &[f32],
    agent_vector: &[f32],
    state: AgentRuntimeState,
    coefficients: ScoreCoefficients,
) -> ScoreComponents {
    debug_assert_eq!(task_vector.len(), agent_vector.len());

    let base_distance = dist_sq_blocked(task_vector, agent_vector);
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

    ScoreComponents {
        base_distance,
        effective_distance,
        omega,
        queue_penalty,
        latency_penalty,
        cache_penalty,
        available,
    }
}

#[inline(always)]
pub fn dist_sq(lhs: &[f32], rhs: &[f32]) -> f32 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(left, right)| {
            let diff = left - right;
            diff * diff
        })
        .sum()
}

#[inline(always)]
pub fn dist_sq_blocked(lhs: &[f32], rhs: &[f32]) -> f32 {
    debug_assert_eq!(lhs.len(), rhs.len());

    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut sum3 = 0.0f32;

    let mut lhs_chunks = lhs.chunks_exact(4);
    let mut rhs_chunks = rhs.chunks_exact(4);

    for (left, right) in lhs_chunks.by_ref().zip(rhs_chunks.by_ref()) {
        let diff0 = left[0] - right[0];
        let diff1 = left[1] - right[1];
        let diff2 = left[2] - right[2];
        let diff3 = left[3] - right[3];

        sum0 += diff0 * diff0;
        sum1 += diff1 * diff1;
        sum2 += diff2 * diff2;
        sum3 += diff3 * diff3;
    }

    let mut sum = (sum0 + sum1) + (sum2 + sum3);
    for (left, right) in lhs_chunks.remainder().iter().zip(rhs_chunks.remainder()) {
        let diff = left - right;
        sum += diff * diff;
    }

    sum
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreComponents {
    pub base_distance: f32,
    pub effective_distance: f32,
    pub omega: f32,
    pub queue_penalty: f32,
    pub latency_penalty: f32,
    pub cache_penalty: f32,
    pub available: bool,
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

    #[test]
    fn blocked_distance_matches_iterator_distance() {
        let left = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 0.25];
        let right = [1.0, 1.5, 1.0, 5.0, 4.5, 2.0, 0.75];

        assert!((dist_sq_blocked(&left, &right) - dist_sq(&left, &right)).abs() < f32::EPSILON);
    }
}
