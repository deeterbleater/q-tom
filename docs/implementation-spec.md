# Q-TOM Implementation Specification

**Status:** Draft implementation spec
**Target phase:** Single-machine prototype
**Primary GPU target:** Windows 11 desktop with RTX 4060 NVIDIA GPU, 8 GB dedicated VRAM, 32 GB host RAM
**Secondary development target:** MacBook Pro M4 with 24 GB unified memory

## 1. Purpose

Q-TOM is a prototype routing layer for multi-agent orchestration. It selects an agent for an incoming task by comparing the task against a precomputed map of benchmarked agent capabilities, then modifying that choice with live runtime state such as queue depth, latency, cache pressure, and availability.

The prototype exists to answer one question first:

> Can a GPU-resident vector routing kernel make agent selection faster, more stable, or more scalable than a CPU routing loop under realistic swarm-orchestration loads?

The first implementation should be narrow. It should prove or disprove the routing mechanism before the larger middleware ecosystem is designed around it.

## 2. Current Assumptions

- An "agent" is a routable execution profile. In the full system it may include model provider, model family, prompt profile, tool bundle, MCP library set, memory set, and higher-level worker behavior. In Prototype 1, the model variable is fixed to a local `Qwen3-2507` profile, and the agent varies by prompt/tool/MCP/memory profile.
- Every agent has a static capability vector derived from benchmark results or manual capability labels.
- Every task has a task vector in the same vector space.
- Agent vectors and task vectors use a small fixed dimension, initially `d = 16` or `d = 32`.
- Live state modifies routing but does not define the agent's core capability.
- The RTX 4060 Windows desktop is the current CUDA test target.
- The MacBook Pro M4 is useful for CPU reference code, simulation, data generation, and possibly a future Metal backend, but not the first CUDA prototype.
- RDMA, NVLink, NCCL, and multi-GPU behavior are out of scope for the first prototype.
- Prototype 1 starts at 128 agents and scales by factors of 8: `128`, `1024`, `8192`, and later `65536` for stress testing if the earlier stages justify it.
- Routing returns a ranked top-k list, not only one winner. The default prototype value is `k = 8`.
- The middleware is expected to batch routing requests naturally. Single-task routing is still benchmarked, but batch routing is the default operating mode.
- Benchmark scoring is LLM-graded from the start, because production benchmark updates are expected to come from an agent-driven evaluation pipeline. The intended evaluator is GPT-5.5 Medium via API, represented in configuration as an evaluator model string so the implementation can adapt to the exact API identifier available at runtime.
- Prototype 1 targets local agents first. Remote API inference is deferred until the local routing concept is validated or shown to need a different design.
- Substitute quality is judged geometrically at first: a substitute is better when it remains close to the selected region of the capability gradient.
- The initial neighborhood radius should usually include at least 3 candidate agents. This is a tuning target, not a fixed correctness requirement.

## 3. Non-Goals for Prototype 1

- No GPUDirect RDMA.
- No multi-GPU sharding.
- No direct public API token stream into GPU memory.
- No full prompt or document parsing inside GPU kernels.
- No dynamic creation of agent vectors inside the hot routing path.
- No claim that GPU routing always beats CPU routing.

## 4. Routing Algorithm

### 4.1 Inputs

For each task:

```text
task_vector[t, d] : float32
```

For each agent:

```text
agent_vector[a, d] : float32
state[a] = {
    queue_depth_norm,
    latency_score,
    cache_pressure,
    availability
}
```

The first version should normalize all dynamic metrics before they reach the kernel. That keeps the kernel simple and makes CPU/GPU parity easier to test. In Prototype 1, queue depth means Q-TOM-assigned pending work:

```text
queue_depth_norm = pending_assigned_tasks / agent_capacity_window
```

Later versions may split this into separate pressure signals, such as assigned queue depth, execution pressure, and failure cooldown.

### 4.2 Score Function

Use squared Euclidean distance to avoid square roots:

```text
dist_sq(t, a) = sum_j((task_vector[t, j] - agent_vector[a, j])^2)
```

Apply a runtime state modifier:

```text
omega(a) = 1
         + alpha * queue_depth_norm[a]
         + beta  * latency_norm[a]
         + gamma * cache_pressure_norm[a]
```

Then compute:

```text
effective_distance(t, a) = dist_sq(t, a) * omega(a)
```

Agents that are unavailable should be skipped or assigned an infinite penalty:

```text
if availability[a] == 0:
    effective_distance = INF
```

The ranked candidate set is:

```text
top_k_agents(t) = k agents with the lowest effective_distance(t, a)
```

The first candidate is the preferred route. The remaining candidates are ordered fallback options. If all candidates exceed `radius_max_threshold`, include `fallback_generalist_id` as the final fallback target.

Top-k routing has two views:

- `available_top_k`: candidates that are currently routable.
- `observed_top_k`: the raw nearest candidates before availability filtering.

The fast path returns `available_top_k` and tracks only the nearest observed candidate needed to set `ideal_candidate_unavailable`. Debug and telemetry records include the full `observed_top_k`. This lets the middleware route only to available agents while still measuring how often the ideal or near-ideal pick was unavailable without paying full observed-top-k bookkeeping cost in production mode.

For Prototype 1, top-k can be understood as a circular or hyperspherical neighborhood around the selected point in the capability gradient. The initial radius target is "large enough to usually contain at least 3 agents." The router still returns up to `k = 8` ranked candidates, adjusted by runtime penalties. If the best semantic target is unavailable, substitute quality is measured by how far the selected available candidate moves away from that target region.

### 4.3 Why This Shape

This preserves the original "throw a dart at a capability map" idea while making it testable:

- The static vector map encodes capability similarity.
- Nearby agents should be acceptable substitutes when the ideal agent is unavailable.
- Runtime state pushes overloaded or degraded agents farther away.
- The fallback threshold catches tasks that do not fit any known agent cluster.
- Returning top-k candidates lets the middleware try the nearest good substitutes without rerunning the full route calculation.
- Keeping both available and observed top-k lists makes routing behavior debuggable when the best semantic match cannot be used.

### 4.4 Future: Lossy Deterministic Candidate Generation

Lossy determinism is not a replacement for top-k routing. It is a future candidate-generation layer that can sit in front of exact top-k scoring when the agent registry becomes too large for full scans.

The rule is:

- Preserve hard constraints exactly.
- Project soft relevance state deterministically and lossily.
- Run exact scoring and top-k selection only inside the selected candidate region.

Hard constraints include availability, permissions, required tool access, memory access, model class, and budget ceilings. These must remain exact masks. Soft relevance state includes task-vector location, queue pressure, latency pressure, cache pressure, and local neighborhood density. These may be quantized into deterministic cells or bands.

Candidate generation can then follow this shape:

```text
task vector
  -> deterministic quantized cell
  -> neighboring cells expanded in fixed order
  -> stop when enough candidates, radius, or scan budget is reached
  -> exact score function over candidate set
  -> exact available top-k output
```

This changes the performance target from scanning every agent to scanning a bounded candidate subset:

```text
full scan:      O(agent_count)
lossy prefilter: O(candidate_count), where candidate_count << agent_count
```

The expected tradeoff is scale-dependent. At `8192` agents, the current full CPU scan is likely faster than maintaining and querying a projection layer. At `65536+` agents, and especially for cluster-sized swarms, deterministic lossy candidate generation may become the main path to lower p99 latency.

The design must remain replayable. Tie handling, cell expansion order, hysteresis bands, and fallback behavior should be deterministic for the same fixture and live-state snapshot. The intended fuzziness is geometric and state-quantized, not random.

Validation requirements:

- Hard-constraint violation rate is always zero.
- Candidate recall is measured against the exact full-scan CPU router.
- Top-k overlap with the exact router is reported.
- Substitute distance delta is reported.
- Scanned-candidate reduction is reported.
- p50, p95, p99, and max latency are compared with the exact router.
- Route churn, repeat loops, and deadlocks are measured under orchestration stress tests.
- If recall or substitute quality falls below threshold, the router expands more cells or falls back to the exact scan.

## 5. Data Model

### 5.1 Agent Registry

```rust
pub struct AgentProfile {
    pub id: u32,
    pub vector: [f32; D],
    pub labels: AgentLabels,
}

pub struct AgentLabels {
    pub model_profile: u16,
    pub tool_profile: u16,
    pub mcp_profile: u16,
    pub memory_profile: u16,
    pub cost_class: u8,
    pub latency_class: u8,
}
```

For Prototype 1, `model_profile` can be constant. For API-based inference later, it becomes a real routing dimension.

### 5.1.1 Packed Route Table

`AgentProfile` is the host-side registry format. The hot routing path should use a packed route table:

```rust
pub struct AgentRouteTable {
    agent_ids: Vec<u32>,
    vectors: Vec<f32>, // row-major [agent_count, dimensions]
    dimensions: usize,
}
```

The CPU router uses this packed table as its internal scan source. Distance scoring uses blocked accumulation over four-float chunks so LLVM can schedule the small fixed-width vector math more effectively. This keeps the correctness oracle closer to the future CUDA buffer layout while preserving ergonomic registry structs at the API boundary.

Production CPU batch routing keeps the per-request path for small registries and switches to a blocked scanner for larger registries. The blocked scanner processes a small request block against agent blocks, reusing each agent vector and runtime penalty factors across multiple tasks while preserving the same ordered `RoutingResult` output.

### 5.2 Live State

```rust
pub struct AgentRuntimeState {
    pub queue_depth_norm: f32,
    pub latency_norm: f32,
    pub cache_pressure_norm: f32,
    pub availability: u32,
}
```

### 5.3 Routing Request

```rust
pub struct RoutingRequest {
    pub task_id: u64,
    pub vector: [f32; D],
    pub k: u32,
    pub fallback_generalist_id: u32,
    pub radius_max_threshold: f32,
}
```

### 5.4 Routing Result

```rust
pub struct RoutingResult {
    pub task_id: u64,
    pub available_candidates: Vec<RouteCandidate>,
    pub used_fallback: bool,
    pub ideal_candidate_unavailable: bool,
    pub debug: Option<RouteDebugInfo>,
}

pub struct RouteDebugInfo {
    pub observed_candidates: Vec<RouteCandidate>,
}

pub struct RouteCandidate {
    pub agent_id: u32,
    pub effective_distance: f32,
    pub base_distance: f32,
    pub omega: f32,
    pub queue_penalty: f32,
    pub latency_penalty: f32,
    pub cache_penalty: f32,
    pub available: bool,
}
```

For GPU memory, use structure-of-arrays or flat arrays rather than Rust structs. The Rust structs are host-side API types. GPU output should use fixed-size flat buffers shaped like `[batch_size, k]` for candidate IDs, scores, availability flags, and score components.

Production callers should rely on `available_candidates`. `debug.observed_candidates` is for observability, offline analysis, and diagnosing why the semantic best match was not selected.

## 6. Agent Vector and Benchmark Schema

The full system can eventually benchmark many combinations of model provider, model, prompt, tool bundle, MCP library set, and memory set. Prototype 1 should freeze the model choice and use a benchmark schema for the remaining agent variables.

### Prototype Agent Definition

```text
agent = {
    model_profile: fixed local Qwen3-2507 profile,
    system_prompt_profile,
    tool_bundle_profile,
    mcp_library_profile,
    memory_set_profile
}
```

### Benchmark Families

Initial benchmark families should be small, reproducible, LLM-graded, and designed to produce useful gradients:

- Tool selection accuracy.
- Multi-step planning reliability.
- Code or command synthesis reliability.
- Retrieval and memory use.
- Long-context instruction retention.
- Structured output validity.
- Error recovery behavior.
- Latency and cost proxies.

Each benchmark family should produce normalized features in `[0, 1]`. The first vector schema should be manually defined before any learned projection is introduced.

The evaluator should produce both a numeric score and a short structured rationale. The numeric score feeds the vector. The rationale feeds audit logs and helps diagnose why an agent profile moved in the capability space. The evaluator configuration should store the model identifier, rubric version, prompt version, temperature, seed if supported, and scoring schema version.

Example `d = 16` vector:

```text
[
  tool_accuracy,
  planning_score,
  coding_score,
  retrieval_score,
  memory_score,
  long_context_score,
  structured_output_score,
  error_recovery_score,
  latency_inverse,
  cost_inverse,
  prompt_stability,
  tool_call_precision,
  tool_call_recall,
  safety_boundary_score,
  deterministic_format_score,
  generalist_score
]
```

This schema is deliberately plain. If it does not produce useful clusters, replace it with a better benchmark-derived projection later.

### Substitute Quality

Prototype 1 evaluates substitute quality by proximity in the routing space.

For a task vector `t`, let `ideal_agent` be the lowest-distance candidate before availability filtering. Let `selected_agent` be the first available candidate returned by the router.

```text
substitute_distance_delta =
    dist_sq(t, selected_agent) - dist_sq(t, ideal_agent)
```

Lower values are better. A value near zero means the substitute is close to the ideal region. A large value means the router was forced away from the intended capability neighborhood.

Top-k radius can be reported as:

```text
top_k_radius = max(dist_sq(t, candidate_i)) for candidate_i in observed_top_k
```

This acts like the "circle" around the selected gradient area. In dimensions greater than two, it is a hypersphere, but the measurement is the same: how wide the candidate neighborhood had to become to produce `k` options.

Prototype 1 should also report:

```text
radius_3 = distance threshold needed to include at least 3 observed candidates
```

This gives the benchmark a practical starting radius. If `radius_3` is too wide or too narrow for useful substitutes, tune the vector schema or radius policy from measured results.

## 7. Repository Layout

```text
q-tom/
  crates/
    qtom-core/
      src/
        types.rs
        score.rs
        cpu_router.rs
        fixtures.rs
    qtom-cuda/
      kernels/
        route_agents.cu
      src/
        device.rs
        gpu_router.rs
    qtom-bench/
      src/
        main.rs
        scenarios.rs
        metrics.rs
  data/
    agents.example.json
    tasks.example.json
  docs/
    implementation-spec.md
    benchmark-plan.md
```

## 8. Implementation Phases

### Phase 0: Test Dataset and Simulator

Create synthetic and LLM-graded agent/task vectors before writing CUDA code.

Requirements:

- Generate clustered agent vectors.
- Generate task vectors near known clusters.
- Mark the ideal agent or ideal cluster for each task.
- Generate top-k expected candidate sets.
- Preserve unavailable ideal candidates in observability output.
- Simulate unavailable agents.
- Simulate rising queue depth and latency.
- Export deterministic fixtures from a fixed random seed.
- Export golden fixture files with exact `f32` bit-pattern encoding for cross-machine CPU/GPU parity tests.

Success criteria:

- Repeated runs generate identical fixtures.
- Golden fixtures round-trip exactly through the reader and writer.
- The nearest-agent route is known for basic cases.
- Substitute-agent behavior can be evaluated when the ideal agent is unavailable.
- Top-k ordering is deterministic for non-tie cases.
- `ideal_candidate_unavailable` can be measured per scenario.

### Phase 1: CPU Reference Router

Build the canonical routing implementation in Rust.

Requirements:

- Stores agent IDs and vectors in a packed row-major route table.
- Uses a blocked distance kernel for `d = 16` and `d = 32` style vectors.
- Uses stack-backed top-k storage for `k <= 8`.
- Uses a specialized single-winner path for `k = 1`.
- Implements the exact score function.
- Handles unavailable agents.
- Applies fallback threshold.
- Supports batch routing.
- Supports top-k candidate output.
- Returns score explanation fields for every candidate.
- Returns observed unavailable candidates only through debug/telemetry output.
- Produces deterministic results.
- Runs on both Mac M4 and Windows.

Success criteria:

- Unit tests cover score calculation, availability, fallback, top-k ordering, and substitute selection.
- CPU benchmark reports p50, p95, p99, max latency, and routes/sec.
- CPU benchmark reports how often the observed best candidate is unavailable.
- CPU self-parity compares sequential and parallel routing over a golden fixture.

### Phase 2: CUDA Kernel on RTX 4060

Build the first GPU router.

CUDA runtime and kernel work must follow the memory-safety constraints in `docs/cuda-safety.md`. The CUDA backend should fail closed with typed backend errors rather than returning partial or uninitialized route results.

Initial scaffold:

- `qtom-cuda` compiles on non-CUDA hosts.
- CUDA Driver API availability detection is behind the opt-in `cuda-runtime` feature.
- `CudaRouter` implements `RouterBackend`. General CUDA routing remains closed, while valid `k = 1` batches can route through CUDA behind the opt-in `cuda-runtime` feature.
- The CUDA crate exposes the flat device-buffer plan for agents, requests, runtime state, candidate outputs, score outputs, and route flags.
- The benchmark CLI can read a golden fixture and print the CUDA buffer plan without requiring CUDA.
- CUDA buffer-plan sizing uses checked arithmetic and reports overflow before any allocation path exists.
- CUDA resource ownership starts with typed RAII wrappers for the retained primary context, streams, and device buffers.
- CUDA module ownership starts with typed RAII wrappers for checked-in PTX and function handles.
- CUDA copy ownership starts with exact-length typed `f32` and `u32` host/device copies.
- CUDA launch ownership starts with q-tom-specific typed parameter packing for `qtom_route_agents_k1`.
- The first naive `k = 1` CUDA scoring kernel matches CPU output on a tiny feature-gated parity fixture.
- The internal `k = 1` CUDA execution helper decodes output arrays into CPU-shaped `RoutingResult` values, including fallback/radius behavior.
- Decoded `k = 1` CUDA routing matches CPU output on a deterministic generated fixture.
- Public `CudaRouter::route_batch` uses the decoded `k = 1` path only when `cuda-runtime` is enabled, the driver initializes, and all request/state shapes validate; unsupported `k` values return `BackendUnavailable`.
- The benchmark CLI can write a deterministic `k = 1` CUDA golden fixture and compare CPU routing against public `CudaRouter::route_batch` through the shared tolerant backend parity harness. Route decisions remain exact; only floating score fields have tolerance.
- The benchmark CLI can time whole public `k = 1` CUDA route batches against CPU backends after parity is checked, including a CUDA stage breakdown for setup, transfer, kernel launch/sync, and decode.

Kernel shape:

- One CUDA thread routes one task against all agents.
- Agent vectors are stored as flat contiguous `float32`.
- State metrics are stored as separate flat `float32` arrays or a compact state matrix.
- Output arrays contain available top-k candidate IDs, observed top-k candidate IDs, score components, and fallback flags.

This is not the final fastest design. It is the simplest correctness-first GPU implementation.

Success criteria:

- GPU results match CPU results for the same golden fixtures.
- Mismatch rate is zero for deterministic fixtures, except for documented floating-point tie cases.
- Benchmarks cover batch sizes `1, 8, 32, 128, 512, 2048`.
- Benchmarks cover agent counts `128`, `1024`, `8192`, with `65536` reserved for later stress testing.

### Phase 3: Kernel Optimization

Optimize only after Phase 2 establishes correctness.

Candidate optimizations:

- Shared-memory tiling of agent vectors.
- One warp per task instead of one thread per task.
- Warp-level reduction for best agent.
- Top-k selection using small fixed-size per-thread or per-warp candidate buffers.
- Vectorized loads for `d = 16` or `d = 32`.
- Constant memory for coefficients.
- Half precision storage for vectors if quality remains acceptable.

Each optimization must have a before/after benchmark.

### Phase 4: Orchestration Stress Tests

Simulate local-agent swarm control behavior.

Scenarios:

- All agents available.
- Ideal agent unavailable.
- Top-k nearest agents unavailable.
- Queue depth spike on a high-quality agent.
- Latency degradation on a remote-provider class.
- Cache pressure on context-heavy agents.
- Mixed task stream with burst arrivals.
- Naturally batched arrivals at varying batch sizes.

Metrics:

- Routing latency.
- Routes/sec.
- Substitute quality loss.
- Substitute distance delta.
- Top-k radius.
- Queue balance over time.
- Fallback rate.
- Ideal-unavailable rate.
- Score component distributions.
- CPU utilization.
- GPU utilization.

### Phase 4.5: Lossy Candidate Generation

Add deterministic projection for large agent registries only after the exact CPU and CUDA routes are established.

Requirements:

- Quantize task vectors into deterministic routing cells.
- Preserve hard constraints as exact masks.
- Expand neighboring cells in a fixed, replayable order.
- Stop expansion by minimum candidate count, radius, or scan budget.
- Run the exact score function and top-k selection inside the candidate set.
- Fall back to the exact scan when candidate recall or substitute quality is below threshold.

Success criteria:

- Results are deterministic for the same fixture and state snapshot.
- Hard-constraint violation rate is zero.
- Candidate recall, top-k overlap, substitute distance delta, and scanned-candidate reduction are reported.
- p99 latency improves over full exact scan for at least one `65536+` agent workload.
- The exact router remains available as the correctness oracle and low-agent-count path.

### Phase 5: Integration Boundary

Define how the larger middleware will call Q-TOM.

Prototype API:

```rust
pub trait RouterBackend {
    fn name(&self) -> &str;

    fn route_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError>;
}
```

Backends:

- `CpuRouter`: always available. It provides the correctness oracle, a single-request path, and an order-preserving batch path. The current implementation can route a batch with an explicit worker count for request-level CPU parallelism experiments.
- `CudaRouter`: used on the RTX 4060 Windows machine. It stays portable without CUDA and currently exposes only the gated `k = 1` CUDA route under `cuda-runtime`; broader top-k routing remains unavailable until parity coverage expands.
- `SimRouter`: deterministic test harness.

The middleware should depend on this trait, not on CUDA directly.

Backend implementations must be testable through the shared exact parity harness. The initial harness compares two `RouterBackend` implementations over the same golden fixture and reports the first mismatch, route count, ideal-unavailable count, and checksum. Future GPU tolerance rules may be added only for documented floating-point tie cases.

## 9. Hardware-Specific Plan

### MacBook Pro M4

Use the Mac for:

- Rust core implementation.
- CPU reference benchmarks.
- Fixture generation.
- Algorithm experiments.
- Documentation.

Do not use it for the first GPU implementation unless a Metal backend becomes a separate goal.

### Windows 11 RTX 4060 Machine

Use the RTX 4060 Windows machine for:

- CUDA kernel development.
- GPU/CPU parity tests.
- Nsight Systems and Nsight Compute profiling.
- Stress testing high batch sizes and large agent registries.

Recommended environment options:

1. WSL2 Ubuntu with NVIDIA CUDA support.
2. Native Windows with CUDA Toolkit and MSVC.

Recommendation: start with WSL2 Ubuntu. NVIDIA supports CUDA development on WSL2, and it should give better cross-platform parity with the Mac-side Rust work. Do not install a Linux NVIDIA driver inside WSL; use the Windows NVIDIA driver and the WSL-compatible CUDA toolkit. Keep native Windows as a fallback if profiling, Nsight workflows, or driver/tooling constraints become a bottleneck.

## 10. Benchmark Plan

### Baselines

Compare:

- Naive CPU loop.
- Optimized CPU loop with Rayon or SIMD where practical.
- CUDA simple kernel.
- CUDA optimized kernels from Phase 3.
- Production batch route profile with observed-candidate debug telemetry disabled.
- Lossy deterministic candidate generation against the exact CPU/GPU router.

### Workload Matrix

| Dimension | Values |
| --- | --- |
| Vector dimension | 16, 32 |
| Agent count | 128, 1024, 8192, later 65536 |
| Top-k | 1, 4, 8 |
| Batch size | 1, 8, 32, 128, 512, 2048 |
| Availability | 100%, 90%, 75%, top-k nearest unavailable |
| Neighborhood radius | adaptive radius for at least 3 observed candidates |
| Candidate generation | exact full scan, lossy deterministic projection |
| Queue pressure | none, mild, severe |
| Latency pressure | none, one cluster degraded, random degraded |

The production profile must include at least `k=1` and `k=8` so the hot single-winner path can be tracked separately from normal top-k routing.

### Metrics

Report:

- p50, p95, p99, and max routing latency.
- Routes/sec.
- CPU utilization.
- GPU utilization.
- Host-to-device and device-to-host transfer time.
- Kernel execution time.
- CPU/GPU result mismatch rate.
- Candidate recall against exact full scan.
- Scanned-candidate reduction.
- Top-k overlap with CPU reference.
- Observed-best unavailable rate.
- Fallback rate.
- Substitute quality score.
- Substitute distance delta.
- Top-k radius.
- Radius needed to include at least 3 observed candidates.
- Score component distributions.

### Success Criteria

The prototype is promising if:

- GPU routing beats CPU routing for at least one realistic high-throughput region.
- CPU routing remains better for low batch sizes and that crossover point is measured.
- Substitute selection remains close to the intended cluster when the ideal agent is unavailable.
- The adaptive neighborhood usually contains at least 3 observed agents.
- Substitute distance delta remains within the chosen neighborhood threshold for most routed tasks.
- Top-k output improves orchestration resilience compared with single-winner output.
- Runtime penalties reduce overload without causing unstable route oscillation.
- Explanation fields are sufficient to diagnose why a candidate was selected or skipped.
- The middleware-facing API remains backend-agnostic.

## 11. Known Risks

- GPU launch overhead may dominate small batches.
- CPU routing may be faster for small agent counts.
- PCIe copy overhead may erase GPU gains unless batches are large or buffers persist.
- The vector map may not correlate with real task success.
- Runtime penalties may need careful normalization to avoid drowning out capability distance.
- Prototype 1 queue depth only captures Q-TOM-assigned pending work. It will not fully represent remote provider backpressure or local worker saturation until later pressure signals are added.
- The RTX 4060 target has 8 GB dedicated VRAM available for this project and midrange memory bandwidth, so very large agent sets and high batch sizes should be introduced only after 128, 1024, and 8192-agent tests are stable.
- If the GPU is also running local inference, routing kernels may reduce tokens/sec or increase decode jitter.
- Top-k selection is more expensive than single-winner selection, so the benchmark must measure `k = 1`, `k = 4`, and `k = 8` separately.
- LLM-graded benchmarks can drift if the evaluator prompt, model, or rubric changes. Evaluator versioning is required.

## 12. Clarifying Questions

These are the remaining non-obvious points that affect implementation:

1. What threshold should count as an acceptable substitute distance delta for each benchmark family?
2. Should the top-k neighborhood radius be fixed per task type, adaptive by local density, or simply reported rather than enforced in Prototype 1?

## 13. Immediate Next Step

Build the CPU reference and simulator first. That gives the project a stable truth source before CUDA enters the picture.

The first concrete milestone should be:

> Given a deterministic fixture containing clustered agents, task vectors, live state, and unavailable agents, the CPU router returns the expected best substitute and records benchmark metrics.

After that, the CUDA kernel has a clean target: match the CPU router exactly, then beat it where the workload is large enough.
