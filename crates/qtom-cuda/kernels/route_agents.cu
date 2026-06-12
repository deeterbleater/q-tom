extern "C" __global__ void qtom_route_agents_k1(
    const float* agent_vectors,
    const unsigned int* agent_ids,
    const float* request_vectors,
    const float* queue_depth_norm,
    const float* latency_norm,
    const float* cache_pressure_norm,
    const unsigned int* availability,
    unsigned int* output_agent_ids,
    float* output_effective_distances,
    float* output_base_distances,
    unsigned int* output_flags,
    unsigned int agent_count,
    unsigned int request_count,
    unsigned int dimensions,
    float alpha_queue,
    float beta_latency,
    float gamma_cache
) {
    const unsigned int task_idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (task_idx >= request_count) {
        return;
    }

    const float infinity = __int_as_float(0x7f800000);
    unsigned int best_agent_id = 0;
    float best_effective_distance = infinity;
    float best_base_distance = infinity;
    bool best_available_seen = false;
    unsigned int observed_agent_id = 0;
    float observed_base_distance = infinity;
    bool observed_available = true;

    for (unsigned int agent_idx = 0; agent_idx < agent_count; ++agent_idx) {
        float base_distance = 0.0f;
        const unsigned int agent_offset = agent_idx * dimensions;
        const unsigned int request_offset = task_idx * dimensions;
        for (unsigned int dim_idx = 0; dim_idx < dimensions; ++dim_idx) {
            const float diff = request_vectors[request_offset + dim_idx] -
                agent_vectors[agent_offset + dim_idx];
            base_distance += diff * diff;
        }

        const unsigned int candidate_agent_id = agent_ids[agent_idx];
        const bool candidate_available = availability[agent_idx] != 0;
        const float omega = 1.0f +
            alpha_queue * queue_depth_norm[agent_idx] +
            beta_latency * latency_norm[agent_idx] +
            gamma_cache * cache_pressure_norm[agent_idx];
        const float effective_distance = candidate_available
            ? base_distance * omega
            : infinity;

        const bool observed_tie = base_distance == observed_base_distance &&
            candidate_agent_id < observed_agent_id;
        if (base_distance < observed_base_distance || observed_tie) {
            observed_agent_id = candidate_agent_id;
            observed_base_distance = base_distance;
            observed_available = candidate_available;
        }

        const bool available_tie = effective_distance == best_effective_distance &&
            candidate_agent_id < best_agent_id;
        if (candidate_available &&
            (!best_available_seen ||
                effective_distance < best_effective_distance ||
                available_tie)) {
            best_agent_id = candidate_agent_id;
            best_effective_distance = effective_distance;
            best_base_distance = base_distance;
            best_available_seen = true;
        }
    }

    output_agent_ids[task_idx] = best_available_seen ? best_agent_id : 0;
    output_effective_distances[task_idx] = best_effective_distance;
    output_base_distances[task_idx] = best_base_distance;
    output_flags[task_idx] = observed_available ? 0u : 1u;
}
