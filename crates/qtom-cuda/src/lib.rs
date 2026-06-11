use qtom_core::{
    AgentProfile, AgentRouteTable, AgentRuntimeState, RouteError, RouterBackend, RoutingRequest,
    RoutingResult, ScoreCoefficients,
};

pub const CUDA_BACKEND_NAME: &str = "cuda";

const SCAFFOLD_REASON: &str = "CUDA host runtime and kernels are not implemented yet";

#[derive(Clone, Debug)]
pub struct CudaRouter {
    name: String,
    route_table: Result<AgentRouteTable, RouteError>,
    coefficients: ScoreCoefficients,
    status: CudaBackendStatus,
}

impl CudaRouter {
    pub fn new(agents: Vec<AgentProfile>, coefficients: ScoreCoefficients) -> Self {
        Self {
            name: CUDA_BACKEND_NAME.to_string(),
            route_table: AgentRouteTable::from_agents(agents),
            coefficients,
            status: CudaBackendStatus::unavailable(SCAFFOLD_REASON),
        }
    }

    pub fn status(&self) -> CudaBackendStatus {
        self.status
    }

    pub fn coefficients(&self) -> ScoreCoefficients {
        self.coefficients
    }

    pub fn buffer_plan(
        &self,
        request_count: usize,
        k: usize,
    ) -> Result<CudaBufferPlan, RouteError> {
        let route_table = self.route_table.as_ref().map_err(Clone::clone)?;
        Ok(CudaBufferPlan::new(
            route_table.len(),
            request_count,
            route_table.dimensions(),
            k,
        ))
    }
}

impl RouterBackend for CudaRouter {
    fn name(&self) -> &str {
        &self.name
    }

    fn route_batch(
        &self,
        _requests: &[RoutingRequest],
        _states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError> {
        self.route_table.as_ref().map_err(Clone::clone)?;
        Err(RouteError::BackendUnavailable {
            backend: CUDA_BACKEND_NAME,
            reason: SCAFFOLD_REASON,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaBackendStatus {
    pub available: bool,
    pub reason: &'static str,
}

impl CudaBackendStatus {
    pub fn unavailable(reason: &'static str) -> Self {
        Self {
            available: false,
            reason,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaBufferPlan {
    pub agent_count: usize,
    pub request_count: usize,
    pub dimensions: usize,
    pub k: usize,
    pub agent_id_u32_len: usize,
    pub agent_vector_f32_len: usize,
    pub request_vector_f32_len: usize,
    pub queue_f32_len: usize,
    pub latency_f32_len: usize,
    pub cache_f32_len: usize,
    pub availability_u32_len: usize,
    pub output_candidate_u32_len: usize,
    pub output_effective_f32_len: usize,
    pub output_base_f32_len: usize,
    pub output_flag_u32_len: usize,
}

impl CudaBufferPlan {
    pub fn new(agent_count: usize, request_count: usize, dimensions: usize, k: usize) -> Self {
        let candidate_slots = request_count.saturating_mul(k);
        Self {
            agent_count,
            request_count,
            dimensions,
            k,
            agent_id_u32_len: agent_count,
            agent_vector_f32_len: agent_count.saturating_mul(dimensions),
            request_vector_f32_len: request_count.saturating_mul(dimensions),
            queue_f32_len: agent_count,
            latency_f32_len: agent_count,
            cache_f32_len: agent_count,
            availability_u32_len: agent_count,
            output_candidate_u32_len: candidate_slots,
            output_effective_f32_len: candidate_slots,
            output_base_f32_len: candidate_slots,
            output_flag_u32_len: request_count,
        }
    }

    pub fn total_f32_len(self) -> usize {
        self.agent_vector_f32_len
            + self.request_vector_f32_len
            + self.queue_f32_len
            + self.latency_f32_len
            + self.cache_f32_len
            + self.output_effective_f32_len
            + self.output_base_f32_len
    }

    pub fn total_u32_len(self) -> usize {
        self.agent_id_u32_len
            + self.availability_u32_len
            + self.output_candidate_u32_len
            + self.output_flag_u32_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtom_core::AgentLabels;

    #[test]
    fn buffer_plan_matches_flat_cuda_layout() {
        let plan = CudaBufferPlan::new(8, 4, 16, 3);

        assert_eq!(plan.agent_id_u32_len, 8);
        assert_eq!(plan.agent_vector_f32_len, 128);
        assert_eq!(plan.request_vector_f32_len, 64);
        assert_eq!(plan.output_candidate_u32_len, 12);
        assert_eq!(plan.output_effective_f32_len, 12);
        assert_eq!(plan.output_base_f32_len, 12);
        assert_eq!(plan.output_flag_u32_len, 4);
    }

    #[test]
    fn router_reports_unavailable_until_kernel_exists() {
        let router = CudaRouter::new(vec![agent(1, &[0.0, 0.0])], ScoreCoefficients::default());

        let error = router
            .route_batch(&[], &[AgentRuntimeState::available()])
            .unwrap_err();

        assert_eq!(
            error,
            RouteError::BackendUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: SCAFFOLD_REASON
            }
        );
    }

    #[test]
    fn router_rejects_invalid_agent_layout_before_unavailable_status() {
        let router = CudaRouter::new(
            vec![agent(1, &[0.0]), agent(2, &[0.0, 1.0])],
            ScoreCoefficients::default(),
        );

        let error = router
            .buffer_plan(1, 1)
            .expect_err("invalid route table should be preserved");

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
}
