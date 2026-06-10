use crate::types::{RouteCandidate, RoutingResult};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RouteMetrics {
    pub substitute_distance_delta: f32,
    pub top_k_radius: f32,
    pub radius_3: Option<f32>,
    pub ideal_candidate_unavailable: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BatchMetrics {
    pub routes: usize,
    pub ideal_unavailable_count: usize,
    pub mean_substitute_distance_delta: f32,
    pub mean_top_k_radius: f32,
}

pub fn route_metrics(result: &RoutingResult) -> Option<RouteMetrics> {
    let debug = result.debug.as_ref()?;
    let ideal = debug.observed_candidates.first()?;
    let selected = result.available_candidates.first()?;

    Some(RouteMetrics {
        substitute_distance_delta: selected.base_distance - ideal.base_distance,
        top_k_radius: max_base_distance(&debug.observed_candidates),
        radius_3: radius_for_n(&debug.observed_candidates, 3),
        ideal_candidate_unavailable: result.ideal_candidate_unavailable,
    })
}

pub fn batch_metrics(results: &[RoutingResult]) -> BatchMetrics {
    let mut count = 0usize;
    let mut unavailable = 0usize;
    let mut substitute_sum = 0.0f32;
    let mut radius_sum = 0.0f32;

    for result in results {
        if let Some(metrics) = route_metrics(result) {
            count += 1;
            unavailable += usize::from(metrics.ideal_candidate_unavailable);
            substitute_sum += metrics.substitute_distance_delta;
            radius_sum += metrics.top_k_radius;
        }
    }

    if count == 0 {
        return BatchMetrics::default();
    }

    BatchMetrics {
        routes: count,
        ideal_unavailable_count: unavailable,
        mean_substitute_distance_delta: substitute_sum / count as f32,
        mean_top_k_radius: radius_sum / count as f32,
    }
}

pub fn radius_for_n(candidates: &[RouteCandidate], n: usize) -> Option<f32> {
    candidates
        .get(n.saturating_sub(1))
        .map(|candidate| candidate.base_distance)
}

fn max_base_distance(candidates: &[RouteCandidate]) -> f32 {
    candidates
        .iter()
        .map(|candidate| candidate.base_distance)
        .fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RouteDebugInfo, RoutingResult};

    #[test]
    fn radius_for_three_uses_third_observed_candidate() {
        let result = RoutingResult {
            task_id: 1,
            available_candidates: vec![candidate(2, 0.2, true)],
            used_fallback: false,
            ideal_candidate_unavailable: true,
            debug: Some(RouteDebugInfo {
                observed_candidates: vec![
                    candidate(1, 0.1, false),
                    candidate(2, 0.2, true),
                    candidate(3, 0.4, true),
                    candidate(4, 0.8, true),
                ],
            }),
        };

        let metrics = route_metrics(&result).unwrap();

        assert_eq!(metrics.substitute_distance_delta, 0.1);
        assert_eq!(metrics.radius_3, Some(0.4));
        assert_eq!(metrics.top_k_radius, 0.8);
        assert!(metrics.ideal_candidate_unavailable);
    }

    fn candidate(agent_id: u32, base_distance: f32, available: bool) -> RouteCandidate {
        RouteCandidate {
            agent_id,
            effective_distance: if available {
                base_distance
            } else {
                f32::INFINITY
            },
            base_distance,
            omega: 1.0,
            queue_penalty: 0.0,
            latency_penalty: 0.0,
            cache_penalty: 0.0,
            available,
        }
    }
}
