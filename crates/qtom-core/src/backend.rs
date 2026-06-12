use crate::types::{AgentRuntimeState, RouteCandidate, RouteError, RoutingRequest, RoutingResult};

pub trait RouterBackend {
    fn name(&self) -> &str;

    fn route_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackendParityReport {
    pub reference_backend: String,
    pub candidate_backend: String,
    pub routes: usize,
    pub ideal_unavailable_count: usize,
    pub checksum: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendMismatch {
    pub first_mismatch_index: usize,
    pub reference_len: usize,
    pub candidate_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackendParityTolerance {
    pub score_abs_epsilon: f32,
}

impl BackendParityTolerance {
    pub const fn new(score_abs_epsilon: f32) -> Self {
        Self { score_abs_epsilon }
    }
}

#[derive(Debug)]
pub enum BackendParityError {
    ReferenceRoute {
        backend: String,
        source: RouteError,
    },
    CandidateRoute {
        backend: String,
        source: RouteError,
    },
    Mismatch {
        reference_backend: String,
        candidate_backend: String,
        mismatch: BackendMismatch,
    },
}

impl std::fmt::Display for BackendParityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReferenceRoute { backend, source } => {
                write!(f, "reference backend {backend} failed: {source}")
            }
            Self::CandidateRoute { backend, source } => {
                write!(f, "candidate backend {backend} failed: {source}")
            }
            Self::Mismatch {
                reference_backend,
                candidate_backend,
                mismatch,
            } => write!(
                f,
                "backend parity mismatch {reference_backend} vs {candidate_backend}: first_mismatch={} reference_len={} candidate_len={}",
                mismatch.first_mismatch_index, mismatch.reference_len, mismatch.candidate_len
            ),
        }
    }
}

impl std::error::Error for BackendParityError {}

pub fn assert_backend_parity<R, C>(
    reference: &R,
    candidate: &C,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
) -> Result<BackendParityReport, BackendParityError>
where
    R: RouterBackend,
    C: RouterBackend,
{
    let reference_results = reference.route_batch(requests, states).map_err(|source| {
        BackendParityError::ReferenceRoute {
            backend: reference.name().to_string(),
            source,
        }
    })?;
    let candidate_results = candidate.route_batch(requests, states).map_err(|source| {
        BackendParityError::CandidateRoute {
            backend: candidate.name().to_string(),
            source,
        }
    })?;

    if reference_results != candidate_results {
        return Err(BackendParityError::Mismatch {
            reference_backend: reference.name().to_string(),
            candidate_backend: candidate.name().to_string(),
            mismatch: first_mismatch(&reference_results, &candidate_results),
        });
    }

    Ok(BackendParityReport {
        reference_backend: reference.name().to_string(),
        candidate_backend: candidate.name().to_string(),
        routes: candidate_results.len(),
        ideal_unavailable_count: candidate_results
            .iter()
            .filter(|result| result.ideal_candidate_unavailable)
            .count(),
        checksum: routing_results_checksum(&candidate_results),
    })
}

pub fn assert_backend_parity_with_tolerance<R, C>(
    reference: &R,
    candidate: &C,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
    tolerance: BackendParityTolerance,
) -> Result<BackendParityReport, BackendParityError>
where
    R: RouterBackend,
    C: RouterBackend,
{
    let reference_results = reference.route_batch(requests, states).map_err(|source| {
        BackendParityError::ReferenceRoute {
            backend: reference.name().to_string(),
            source,
        }
    })?;
    let candidate_results = candidate.route_batch(requests, states).map_err(|source| {
        BackendParityError::CandidateRoute {
            backend: candidate.name().to_string(),
            source,
        }
    })?;

    if !routing_results_match_with_tolerance(&reference_results, &candidate_results, tolerance) {
        return Err(BackendParityError::Mismatch {
            reference_backend: reference.name().to_string(),
            candidate_backend: candidate.name().to_string(),
            mismatch: first_tolerant_mismatch(&reference_results, &candidate_results, tolerance),
        });
    }

    Ok(BackendParityReport {
        reference_backend: reference.name().to_string(),
        candidate_backend: candidate.name().to_string(),
        routes: candidate_results.len(),
        ideal_unavailable_count: candidate_results
            .iter()
            .filter(|result| result.ideal_candidate_unavailable)
            .count(),
        checksum: routing_results_checksum(&candidate_results),
    })
}

pub fn routing_results_checksum(results: &[RoutingResult]) -> f64 {
    results
        .iter()
        .filter_map(|result| result.available_candidates.first())
        .map(|candidate| candidate.base_distance as f64 + candidate.agent_id as f64)
        .sum()
}

fn first_tolerant_mismatch(
    reference: &[RoutingResult],
    candidate: &[RoutingResult],
    tolerance: BackendParityTolerance,
) -> BackendMismatch {
    let first_mismatch_index = reference
        .iter()
        .zip(candidate.iter())
        .position(|(left, right)| !routing_result_matches_with_tolerance(left, right, tolerance))
        .unwrap_or(reference.len().min(candidate.len()));

    BackendMismatch {
        first_mismatch_index,
        reference_len: reference.len(),
        candidate_len: candidate.len(),
    }
}

fn first_mismatch(reference: &[RoutingResult], candidate: &[RoutingResult]) -> BackendMismatch {
    let first_mismatch_index = reference
        .iter()
        .zip(candidate.iter())
        .position(|(left, right)| left != right)
        .unwrap_or(reference.len().min(candidate.len()));

    BackendMismatch {
        first_mismatch_index,
        reference_len: reference.len(),
        candidate_len: candidate.len(),
    }
}

fn routing_results_match_with_tolerance(
    reference: &[RoutingResult],
    candidate: &[RoutingResult],
    tolerance: BackendParityTolerance,
) -> bool {
    reference.len() == candidate.len()
        && reference
            .iter()
            .zip(candidate)
            .all(|(left, right)| routing_result_matches_with_tolerance(left, right, tolerance))
}

fn routing_result_matches_with_tolerance(
    reference: &RoutingResult,
    candidate: &RoutingResult,
    tolerance: BackendParityTolerance,
) -> bool {
    reference.task_id == candidate.task_id
        && reference.used_fallback == candidate.used_fallback
        && reference.ideal_candidate_unavailable == candidate.ideal_candidate_unavailable
        && candidate_lists_match_with_tolerance(
            &reference.available_candidates,
            &candidate.available_candidates,
            tolerance,
        )
        && debug_matches_with_tolerance(&reference.debug, &candidate.debug, tolerance)
}

fn debug_matches_with_tolerance(
    reference: &Option<crate::types::RouteDebugInfo>,
    candidate: &Option<crate::types::RouteDebugInfo>,
    tolerance: BackendParityTolerance,
) -> bool {
    match (reference, candidate) {
        (Some(reference), Some(candidate)) => candidate_lists_match_with_tolerance(
            &reference.observed_candidates,
            &candidate.observed_candidates,
            tolerance,
        ),
        (None, None) => true,
        _ => false,
    }
}

fn candidate_lists_match_with_tolerance(
    reference: &[RouteCandidate],
    candidate: &[RouteCandidate],
    tolerance: BackendParityTolerance,
) -> bool {
    reference.len() == candidate.len()
        && reference
            .iter()
            .zip(candidate)
            .all(|(left, right)| route_candidate_matches_with_tolerance(left, right, tolerance))
}

fn route_candidate_matches_with_tolerance(
    reference: &RouteCandidate,
    candidate: &RouteCandidate,
    tolerance: BackendParityTolerance,
) -> bool {
    reference.agent_id == candidate.agent_id
        && reference.available == candidate.available
        && f32_close(
            reference.effective_distance,
            candidate.effective_distance,
            tolerance.score_abs_epsilon,
        )
        && f32_close(
            reference.base_distance,
            candidate.base_distance,
            tolerance.score_abs_epsilon,
        )
        && f32_close(
            reference.omega,
            candidate.omega,
            tolerance.score_abs_epsilon,
        )
        && f32_close(
            reference.queue_penalty,
            candidate.queue_penalty,
            tolerance.score_abs_epsilon,
        )
        && f32_close(
            reference.latency_penalty,
            candidate.latency_penalty,
            tolerance.score_abs_epsilon,
        )
        && f32_close(
            reference.cache_penalty,
            candidate.cache_penalty,
            tolerance.score_abs_epsilon,
        )
}

fn f32_close(left: f32, right: f32, epsilon: f32) -> bool {
    if left.is_infinite() || right.is_infinite() {
        left == right
    } else {
        (left - right).abs() <= epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentRuntimeState;

    #[derive(Clone)]
    struct FixedBackend {
        name: &'static str,
        result: Result<Vec<RoutingResult>, RouteError>,
    }

    impl RouterBackend for FixedBackend {
        fn name(&self) -> &str {
            self.name
        }

        fn route_batch(
            &self,
            _requests: &[RoutingRequest],
            _states: &[AgentRuntimeState],
        ) -> Result<Vec<RoutingResult>, RouteError> {
            self.result.clone()
        }
    }

    #[test]
    fn backend_parity_passes_for_matching_results() {
        let result = RoutingResult {
            task_id: 1,
            available_candidates: Vec::new(),
            used_fallback: false,
            ideal_candidate_unavailable: false,
            debug: None,
        };
        let reference = FixedBackend {
            name: "reference",
            result: Ok(vec![result.clone()]),
        };
        let candidate = FixedBackend {
            name: "candidate",
            result: Ok(vec![result]),
        };

        let report = assert_backend_parity(&reference, &candidate, &[], &[]).unwrap();

        assert_eq!(report.reference_backend, "reference");
        assert_eq!(report.candidate_backend, "candidate");
        assert_eq!(report.routes, 1);
        assert_eq!(report.ideal_unavailable_count, 0);
    }

    #[test]
    fn backend_parity_reports_first_mismatch() {
        let reference = FixedBackend {
            name: "reference",
            result: Ok(vec![result(1), result(2)]),
        };
        let candidate = FixedBackend {
            name: "candidate",
            result: Ok(vec![result(1), result(3)]),
        };

        let error = assert_backend_parity(&reference, &candidate, &[], &[]).unwrap_err();

        match error {
            BackendParityError::Mismatch { mismatch, .. } => {
                assert_eq!(mismatch.first_mismatch_index, 1);
                assert_eq!(mismatch.reference_len, 2);
                assert_eq!(mismatch.candidate_len, 2);
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn backend_parity_with_tolerance_allows_small_score_drift() {
        let reference = FixedBackend {
            name: "reference",
            result: Ok(vec![result_with_candidate(1, 7, 1.0)]),
        };
        let candidate = FixedBackend {
            name: "candidate",
            result: Ok(vec![result_with_candidate(1, 7, 1.000001)]),
        };

        let report = assert_backend_parity_with_tolerance(
            &reference,
            &candidate,
            &[],
            &[],
            BackendParityTolerance::new(0.00001),
        )
        .unwrap();

        assert_eq!(report.routes, 1);
    }

    #[test]
    fn backend_parity_with_tolerance_still_rejects_decision_drift() {
        let reference = FixedBackend {
            name: "reference",
            result: Ok(vec![result_with_candidate(1, 7, 1.0)]),
        };
        let candidate = FixedBackend {
            name: "candidate",
            result: Ok(vec![result_with_candidate(1, 8, 1.0)]),
        };

        let error = assert_backend_parity_with_tolerance(
            &reference,
            &candidate,
            &[],
            &[],
            BackendParityTolerance::new(0.00001),
        )
        .unwrap_err();

        match error {
            BackendParityError::Mismatch { mismatch, .. } => {
                assert_eq!(mismatch.first_mismatch_index, 0);
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    fn result(task_id: u64) -> RoutingResult {
        RoutingResult {
            task_id,
            available_candidates: Vec::new(),
            used_fallback: false,
            ideal_candidate_unavailable: false,
            debug: None,
        }
    }

    fn result_with_candidate(task_id: u64, agent_id: u32, distance: f32) -> RoutingResult {
        RoutingResult {
            task_id,
            available_candidates: vec![RouteCandidate {
                agent_id,
                effective_distance: distance,
                base_distance: distance,
                omega: 1.0,
                queue_penalty: 0.0,
                latency_penalty: 0.0,
                cache_penalty: 0.0,
                available: true,
            }],
            used_fallback: false,
            ideal_candidate_unavailable: false,
            debug: None,
        }
    }
}
