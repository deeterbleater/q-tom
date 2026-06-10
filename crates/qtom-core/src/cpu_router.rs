use crate::route_table::AgentRouteTable;
use crate::score::{ScoreCoefficients, ScoreComponents, score_components_for_vector};
use crate::types::{
    AgentProfile, AgentRuntimeState, RouteCandidate, RouteDebugInfo, RouteError, RoutingRequest,
    RoutingResult,
};

const STACK_TOP_K_CAP: usize = 8;

pub trait RouterBackend {
    fn route_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError>;
}

#[derive(Clone, Debug)]
pub struct CpuRouter {
    route_table: Result<AgentRouteTable, RouteError>,
    coefficients: ScoreCoefficients,
    debug_observed: bool,
}

impl CpuRouter {
    pub fn new(agents: Vec<AgentProfile>, coefficients: ScoreCoefficients) -> Self {
        Self {
            route_table: AgentRouteTable::from_agents(agents),
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
        let route_table = self.validate_request(request, states)?;
        self.route_one_validated(request, states, route_table)
    }

    fn route_one_validated(
        &self,
        request: &RoutingRequest,
        states: &[AgentRuntimeState],
        route_table: &AgentRouteTable,
    ) -> Result<RoutingResult, RouteError> {
        if request.k == 1 {
            return self.route_one_best_validated(request, states, route_table);
        }

        let mut available_top_k = BoundedTopK::new(request.k, CandidateSort::Available);
        let (debug, ideal_candidate_unavailable) = if self.debug_observed {
            let mut observed_top_k = BoundedTopK::new(request.k, CandidateSort::Observed);
            scan_candidates(
                &request.vector,
                self.coefficients,
                route_table,
                states,
                |candidate| {
                    observed_top_k.push(candidate);
                    if candidate.available {
                        available_top_k.push(candidate);
                    }
                },
            );

            let observed_candidates = observed_top_k
                .into_vec()
                .into_iter()
                .map(CompactCandidate::into_route_candidate)
                .collect::<Vec<_>>();
            let ideal_candidate_unavailable = observed_candidates
                .first()
                .map(|candidate| !candidate.available)
                .unwrap_or(false);

            (
                Some(RouteDebugInfo {
                    observed_candidates,
                }),
                ideal_candidate_unavailable,
            )
        } else {
            let mut observed_best = BestCandidate::new(CandidateSort::Observed, request.k > 0);
            scan_candidates(
                &request.vector,
                self.coefficients,
                route_table,
                states,
                |candidate| {
                    observed_best.push(candidate);
                    if candidate.available {
                        available_top_k.push(candidate);
                    }
                },
            );

            (None, observed_best.ideal_candidate_unavailable())
        };

        let mut available_top_k = available_top_k
            .into_vec()
            .into_iter()
            .map(CompactCandidate::into_route_candidate)
            .collect::<Vec<_>>();

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
            debug,
        })
    }

    fn route_one_best_validated(
        &self,
        request: &RoutingRequest,
        states: &[AgentRuntimeState],
        route_table: &AgentRouteTable,
    ) -> Result<RoutingResult, RouteError> {
        let mut observed_best = BestCandidate::new(CandidateSort::Observed, true);
        let mut available_best = BestCandidate::new(CandidateSort::Available, true);

        scan_candidates(
            &request.vector,
            self.coefficients,
            route_table,
            states,
            |candidate| {
                observed_best.push(candidate);
                if candidate.available {
                    available_best.push(candidate);
                }
            },
        );

        let observed_best = observed_best.into_candidate();
        let ideal_candidate_unavailable = observed_best
            .map(|candidate| !candidate.available)
            .unwrap_or(false);
        let debug = self.debug_observed.then(|| RouteDebugInfo {
            observed_candidates: observed_best
                .map(CompactCandidate::into_route_candidate)
                .into_iter()
                .collect(),
        });
        let mut available_candidates = available_best
            .into_candidate()
            .map(CompactCandidate::into_route_candidate)
            .into_iter()
            .collect::<Vec<_>>();

        let used_fallback = available_candidates
            .first()
            .map(|candidate| candidate.base_distance > request.radius_max_threshold)
            .unwrap_or(true);

        if used_fallback {
            available_candidates.push(RouteCandidate {
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
            available_candidates,
            used_fallback,
            ideal_candidate_unavailable,
            debug,
        })
    }

    pub fn route_batch_with_workers(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
        workers: usize,
    ) -> Result<Vec<RoutingResult>, RouteError> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }
        let route_table = self.validate_batch(requests, states)?;

        let worker_count = workers.max(1).min(requests.len());
        if worker_count == 1 {
            return self.route_batch_validated_sequential(requests, states, route_table);
        }

        let chunk_size = requests.len().div_ceil(worker_count);
        let partials = std::thread::scope(|scope| {
            let handles = requests
                .chunks(chunk_size)
                .map(|chunk| {
                    scope.spawn(move || {
                        let mut results = Vec::with_capacity(chunk.len());
                        for request in chunk {
                            results.push(self.route_one_validated(request, states, route_table)?);
                        }
                        Ok::<_, RouteError>(results)
                    })
                })
                .collect::<Vec<_>>();

            handles
                .into_iter()
                .map(|handle| handle.join().expect("route worker should not panic"))
                .collect::<Vec<_>>()
        });

        let mut results = Vec::with_capacity(requests.len());
        for partial in partials {
            let mut chunk_results = partial?;
            results.append(&mut chunk_results);
        }

        Ok(results)
    }

    fn validate_request(
        &self,
        request: &RoutingRequest,
        states: &[AgentRuntimeState],
    ) -> Result<&AgentRouteTable, RouteError> {
        self.validate_batch(std::slice::from_ref(request), states)
    }

    fn validate_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<&AgentRouteTable, RouteError> {
        let route_table = self.route_table.as_ref().map_err(Clone::clone)?;
        if route_table.is_empty() {
            return Err(RouteError::EmptyAgents);
        }
        if route_table.len() != states.len() {
            return Err(RouteError::StateLengthMismatch {
                agents: route_table.len(),
                states: states.len(),
            });
        }

        let expected = route_table.dimensions();
        for request in requests {
            if request.vector.len() != expected {
                return Err(RouteError::DimensionMismatch {
                    expected,
                    actual: request.vector.len(),
                    context: "routing request",
                });
            }
        }

        Ok(route_table)
    }

    fn route_batch_validated_sequential(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
        route_table: &AgentRouteTable,
    ) -> Result<Vec<RoutingResult>, RouteError> {
        requests
            .iter()
            .map(|request| self.route_one_validated(request, states, route_table))
            .collect()
    }
}

impl RouterBackend for CpuRouter {
    fn route_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError> {
        let workers = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        self.route_batch_with_workers(requests, states, workers)
    }
}

#[inline(always)]
fn scan_candidates<F>(
    request_vector: &[f32],
    coefficients: ScoreCoefficients,
    route_table: &AgentRouteTable,
    states: &[AgentRuntimeState],
    mut push: F,
) where
    F: FnMut(CompactCandidate),
{
    if route_table.dimensions() == 0 {
        for (index, state) in states.iter().copied().enumerate() {
            push(score_compact_candidate(
                request_vector,
                coefficients,
                route_table.agent_id(index),
                route_table.vector(index),
                state,
            ));
        }
    } else {
        for ((agent_id, agent_vector), state) in route_table
            .agent_ids()
            .iter()
            .copied()
            .zip(
                route_table
                    .packed_vectors()
                    .chunks_exact(route_table.dimensions()),
            )
            .zip(states.iter().copied())
        {
            push(score_compact_candidate(
                request_vector,
                coefficients,
                agent_id,
                agent_vector,
                state,
            ));
        }
    }
}

#[inline(always)]
fn score_compact_candidate(
    request_vector: &[f32],
    coefficients: ScoreCoefficients,
    agent_id: u32,
    agent_vector: &[f32],
    state: AgentRuntimeState,
) -> CompactCandidate {
    let score = score_components_for_vector(request_vector, agent_vector, state, coefficients);
    CompactCandidate::from_score(agent_id, score)
}

struct BestCandidate {
    best: Option<CompactCandidate>,
    sort: CandidateSort,
    track_ideal: bool,
}

impl BestCandidate {
    fn new(sort: CandidateSort, enabled: bool) -> Self {
        Self {
            best: None,
            sort,
            track_ideal: enabled,
        }
    }

    fn push(&mut self, candidate: CompactCandidate) {
        if self.track_ideal {
            match self.best {
                Some(best) if compare_candidate(&candidate, &best, self.sort).is_lt() => {
                    self.best = Some(candidate);
                }
                None => self.best = Some(candidate),
                _ => {}
            }
        }
    }

    fn ideal_candidate_unavailable(self) -> bool {
        self.best
            .map(|candidate| !candidate.available)
            .unwrap_or(false)
    }

    fn into_candidate(self) -> Option<CompactCandidate> {
        self.best
    }
}

#[derive(Clone, Copy)]
enum CandidateSort {
    Observed,
    Available,
}

fn sort_candidates(candidates: &mut [CompactCandidate], sort: CandidateSort) {
    candidates.sort_by(|left, right| compare_candidate(left, right, sort));
}

struct BoundedTopK {
    k: usize,
    sort: CandidateSort,
    storage: TopKStorage,
    worst_idx: Option<usize>,
}

impl BoundedTopK {
    fn new(k: usize, sort: CandidateSort) -> Self {
        Self {
            k,
            sort,
            storage: TopKStorage::new(k),
            worst_idx: None,
        }
    }

    fn push(&mut self, candidate: CompactCandidate) {
        if self.k == 0 {
            return;
        }

        if self.len() < self.k {
            self.push_candidate(candidate);
            self.update_worst_after_push();
            return;
        }

        let Some(worst_idx) = self.worst_idx else {
            return;
        };

        if compare_candidate(&candidate, &self.candidate(worst_idx), self.sort).is_lt() {
            self.replace_candidate(worst_idx, candidate);
            self.recompute_worst();
        }
    }

    fn into_vec(self) -> Vec<CompactCandidate> {
        let mut candidates = self.storage.into_vec();
        sort_candidates(&mut candidates, self.sort);
        candidates
    }

    fn update_worst_after_push(&mut self) {
        let new_idx = self.len() - 1;
        match self.worst_idx {
            Some(worst_idx)
                if compare_candidate(
                    &self.candidate(new_idx),
                    &self.candidate(worst_idx),
                    self.sort,
                )
                .is_gt() =>
            {
                self.worst_idx = Some(new_idx);
            }
            None => self.worst_idx = Some(new_idx),
            _ => {}
        }
    }

    fn recompute_worst(&mut self) {
        self.worst_idx = (0..self.len()).max_by(|left, right| {
            compare_candidate(&self.candidate(*left), &self.candidate(*right), self.sort)
        });
    }

    fn len(&self) -> usize {
        self.storage.len()
    }

    fn candidate(&self, idx: usize) -> CompactCandidate {
        self.storage.candidate(idx)
    }

    fn push_candidate(&mut self, candidate: CompactCandidate) {
        self.storage.push(candidate);
    }

    fn replace_candidate(&mut self, idx: usize, candidate: CompactCandidate) {
        self.storage.replace(idx, candidate);
    }
}

enum TopKStorage {
    Stack {
        candidates: [Option<CompactCandidate>; STACK_TOP_K_CAP],
        len: usize,
    },
    Heap(Vec<CompactCandidate>),
}

impl TopKStorage {
    fn new(k: usize) -> Self {
        if k <= STACK_TOP_K_CAP {
            Self::Stack {
                candidates: [None; STACK_TOP_K_CAP],
                len: 0,
            }
        } else {
            Self::Heap(Vec::with_capacity(k))
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Stack { len, .. } => *len,
            Self::Heap(candidates) => candidates.len(),
        }
    }

    fn candidate(&self, idx: usize) -> CompactCandidate {
        match self {
            Self::Stack { candidates, len } => {
                debug_assert!(idx < *len);
                candidates[idx].expect("stack top-k slot should be initialized")
            }
            Self::Heap(candidates) => candidates[idx],
        }
    }

    fn push(&mut self, candidate: CompactCandidate) {
        match self {
            Self::Stack { candidates, len } => {
                debug_assert!(*len < STACK_TOP_K_CAP);
                candidates[*len] = Some(candidate);
                *len += 1;
            }
            Self::Heap(candidates) => candidates.push(candidate),
        }
    }

    fn replace(&mut self, idx: usize, candidate: CompactCandidate) {
        match self {
            Self::Stack { candidates, len } => {
                debug_assert!(idx < *len);
                candidates[idx] = Some(candidate);
            }
            Self::Heap(candidates) => candidates[idx] = candidate,
        }
    }

    fn into_vec(self) -> Vec<CompactCandidate> {
        match self {
            Self::Stack { candidates, len } => {
                let mut out = Vec::with_capacity(len);
                for candidate in candidates.iter().take(len) {
                    out.push(candidate.expect("stack top-k slot should be initialized"));
                }
                out
            }
            Self::Heap(candidates) => candidates,
        }
    }
}

fn compare_candidate(
    left: &CompactCandidate,
    right: &CompactCandidate,
    sort: CandidateSort,
) -> std::cmp::Ordering {
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CompactCandidate {
    agent_id: u32,
    effective_distance: f32,
    base_distance: f32,
    omega: f32,
    queue_penalty: f32,
    latency_penalty: f32,
    cache_penalty: f32,
    available: bool,
}

impl CompactCandidate {
    fn from_score(agent_id: u32, score: ScoreComponents) -> Self {
        Self {
            agent_id,
            effective_distance: score.effective_distance,
            base_distance: score.base_distance,
            omega: score.omega,
            queue_penalty: score.queue_penalty,
            latency_penalty: score.latency_penalty,
            cache_penalty: score.cache_penalty,
            available: score.available,
        }
    }

    fn into_route_candidate(self) -> RouteCandidate {
        RouteCandidate {
            agent_id: self.agent_id,
            effective_distance: self.effective_distance,
            base_distance: self.base_distance,
            omega: self.omega,
            queue_penalty: self.queue_penalty,
            latency_penalty: self.latency_penalty,
            cache_penalty: self.cache_penalty,
            available: self.available,
        }
    }
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

    #[test]
    fn debug_disabled_skips_observed_candidates_but_keeps_ideal_flag() {
        let router = CpuRouter::new(
            vec![agent(1, &[0.0, 0.0]), agent(2, &[0.1, 0.0])],
            ScoreCoefficients::default(),
        )
        .with_debug_observed(false);
        let states = vec![
            AgentRuntimeState::unavailable(),
            AgentRuntimeState::available(),
        ];
        let request = request(&[0.0, 0.0], 2, 999, 10.0);

        let result = router.route_one(&request, &states).unwrap();

        assert!(result.ideal_candidate_unavailable);
        assert!(result.debug.is_none());
        assert_eq!(result.available_candidates[0].agent_id, 2);
    }

    #[test]
    fn k_one_route_uses_single_winner_path_with_debug_candidate() {
        let router = CpuRouter::new(
            vec![
                agent(1, &[0.0, 0.0]),
                agent(2, &[0.1, 0.0]),
                agent(3, &[0.2, 0.0]),
            ],
            ScoreCoefficients::default(),
        );
        let states = vec![
            AgentRuntimeState::unavailable(),
            AgentRuntimeState::available(),
            AgentRuntimeState::available(),
        ];
        let request = request(&[0.0, 0.0], 1, 999, 10.0);

        let result = router.route_one(&request, &states).unwrap();

        assert!(result.ideal_candidate_unavailable);
        assert_eq!(result.available_candidates.len(), 1);
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
            vec![1]
        );
    }

    #[test]
    fn bounded_top_k_matches_full_sort_order() {
        let candidates = vec![
            candidate(5, 0.5, 0.9, true),
            candidate(1, 0.1, 0.7, true),
            candidate(4, 0.4, 0.4, true),
            candidate(2, 0.2, 0.3, true),
            candidate(3, 0.3, 0.2, true),
        ];
        let mut bounded = BoundedTopK::new(3, CandidateSort::Observed);
        for candidate in candidates.clone() {
            bounded.push(candidate);
        }

        let mut sorted = candidates;
        sort_candidates(&mut sorted, CandidateSort::Observed);
        sorted.truncate(3);

        assert_eq!(
            bounded
                .into_vec()
                .iter()
                .map(|candidate| candidate.agent_id)
                .collect::<Vec<_>>(),
            sorted
                .iter()
                .map(|candidate| candidate.agent_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn bounded_top_k_heap_fallback_matches_full_sort_order() {
        let candidates = (0..12)
            .map(|idx| candidate(idx, (12 - idx) as f32, idx as f32, true))
            .collect::<Vec<_>>();
        let mut bounded = BoundedTopK::new(9, CandidateSort::Available);
        for candidate in candidates.clone() {
            bounded.push(candidate);
        }

        let mut sorted = candidates;
        sort_candidates(&mut sorted, CandidateSort::Available);
        sorted.truncate(9);

        assert_eq!(
            bounded
                .into_vec()
                .iter()
                .map(|candidate| candidate.agent_id)
                .collect::<Vec<_>>(),
            sorted
                .iter()
                .map(|candidate| candidate.agent_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn parallel_batch_matches_single_worker_batch_order() {
        let router = CpuRouter::new(
            vec![
                agent(1, &[0.0, 0.0]),
                agent(2, &[0.1, 0.0]),
                agent(3, &[1.0, 1.0]),
                agent(4, &[0.2, 0.2]),
            ],
            ScoreCoefficients::default(),
        );
        let states = vec![
            AgentRuntimeState::unavailable(),
            AgentRuntimeState::available(),
            AgentRuntimeState::available(),
            AgentRuntimeState::available(),
        ];
        let requests = vec![
            request(&[0.05, 0.0], 2, 999, 10.0),
            request(&[0.15, 0.0], 2, 999, 10.0),
            request(&[0.95, 1.0], 2, 999, 10.0),
            request(&[0.25, 0.2], 2, 999, 10.0),
        ];

        let sequential = router
            .route_batch_with_workers(&requests, &states, 1)
            .unwrap();
        let parallel = router
            .route_batch_with_workers(&requests, &states, 8)
            .unwrap();

        assert_eq!(parallel, sequential);
        assert_eq!(
            parallel
                .iter()
                .map(|result| result.task_id)
                .collect::<Vec<_>>(),
            requests
                .iter()
                .map(|request| request.task_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn empty_batch_returns_empty_results() {
        let router = CpuRouter::new(vec![agent(1, &[0.0, 0.0])], ScoreCoefficients::default());
        let results = router
            .route_batch_with_workers(&[], &[AgentRuntimeState::available()], 8)
            .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn batch_validates_request_dimensions_once_before_routing() {
        let router = CpuRouter::new(vec![agent(1, &[0.0, 0.0])], ScoreCoefficients::default());
        let requests = vec![request(&[0.0, 0.0, 0.0], 1, 999, 10.0)];

        let error = router
            .route_batch_with_workers(&requests, &[AgentRuntimeState::available()], 4)
            .unwrap_err();

        assert_eq!(
            error,
            RouteError::DimensionMismatch {
                expected: 2,
                actual: 3,
                context: "routing request"
            }
        );
    }

    #[test]
    fn invalid_agent_vector_dimensions_are_reported_on_route() {
        let router = CpuRouter::new(
            vec![agent(1, &[0.0]), agent(2, &[0.0, 1.0])],
            ScoreCoefficients::default(),
        );

        let error = router
            .route_one(
                &request(&[0.0], 1, 999, 10.0),
                &[
                    AgentRuntimeState::available(),
                    AgentRuntimeState::available(),
                ],
            )
            .unwrap_err();

        assert_eq!(
            error,
            RouteError::DimensionMismatch {
                expected: 1,
                actual: 2,
                context: "agent vector"
            }
        );
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

    fn candidate(
        agent_id: u32,
        base_distance: f32,
        effective_distance: f32,
        available: bool,
    ) -> CompactCandidate {
        CompactCandidate {
            agent_id,
            effective_distance,
            base_distance,
            omega: 1.0,
            queue_penalty: 0.0,
            latency_penalty: 0.0,
            cache_penalty: 0.0,
            available,
        }
    }
}
