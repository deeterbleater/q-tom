# ADR-001: Start Q-TOM as a Single-Machine CPU/CUDA Prototype

**Status:** Proposed
**Date:** 2026-06-10

## Context

Q-TOM is intended to become part of a larger middleware ecosystem for intelligent multi-agent orchestration. The larger system is still open-ended because routing performance and routing quality will shape many downstream design choices.

Available hardware:

- MacBook Pro M4 with 24 GB unified memory.
- Windows 11 desktop with RTX 4090-class NVIDIA GPU, 16 GB VRAM available for this project, and 32 GB host RAM.

The original concept included GPU-resident routing, RDMA ingress, multi-GPU coordination, and possible inference colocation. Those are useful future directions, but they would obscure the first question: whether the vector routing algorithm is correct, useful, and faster than a CPU baseline under plausible workloads.

## Decision

Prototype Q-TOM first as a single-machine routing engine with:

- A Rust CPU reference implementation.
- A deterministic fixture generator and simulator.
- A CUDA backend for the RTX 4090 machine.
- A backend-agnostic router trait for future middleware integration.
- A top-k routing result rather than a single winner, with `k = 8` as the default prototype target.
- A fixed local `Qwen3-2507` model profile for Prototype 1, with agent variation coming from prompt, tool bundle, MCP library set, and memory set.
- LLM-graded benchmark vectors, with GPT-5.5 Medium via API as the intended evaluator and evaluator prompts/rubrics versioned as part of the test data.
- Available top-k output on the production fast path, with observed top-k output reserved for debug and telemetry.
- Queue depth defined initially as Q-TOM-assigned pending work normalized by an agent capacity window.
- Local-agent orchestration as the first integration target. Remote API inference is deferred until the local routing concept is validated.
- Geometric substitute quality based on proximity to the selected region of the capability gradient.
- An initial adaptive neighborhood radius that usually includes at least 3 observed agents, while still returning up to `k = 8` ranked candidates.

Exclude RDMA, NVLink, NCCL, multi-GPU sharding, and local inference colocation from Prototype 1.

## Options Considered

### Option A: Full Hardware-Ambitious Prototype

Include CUDA, RDMA, multi-GPU synchronization, and inference colocation immediately.

| Dimension   | Assessment                                                                               |
| ----------- | ---------------------------------------------------------------------------------------- |
| Complexity  | Very high                                                                                |
| Cost        | High hardware and setup cost                                                             |
| Scalability | Tests future architecture early                                                          |
| Risk        | Hard to tell whether failures come from routing, hardware setup, or orchestration design |

**Pros:** Closer to the long-term vision.

**Cons:** Too many variables before the routing algorithm is validated.

### Option B: CPU-Only Simulator

Build only the routing algorithm and simulation harness.

| Dimension   | Assessment                                      |
| ----------- | ----------------------------------------------- |
| Complexity  | Low                                             |
| Cost        | Low                                             |
| Scalability | Limited                                         |
| Risk        | Cannot answer whether GPU routing is worthwhile |

**Pros:** Fastest way to test routing semantics.

**Cons:** Avoids the main performance question.

### Option C: CPU Reference Plus Single-GPU CUDA Backend

Build the CPU truth source first, then a single-GPU CUDA implementation that must match it.

| Dimension   | Assessment                                                  |
| ----------- | ----------------------------------------------------------- |
| Complexity  | Medium                                                      |
| Cost        | Fits available hardware                                     |
| Scalability | Enough to test the key performance hypothesis               |
| Risk        | Keeps correctness, benchmarking, and optimization separable |

**Pros:** Best match for available hardware and current uncertainty.

**Cons:** Does not prove multi-GPU or RDMA behavior yet.

### Environment Decision

Start with WSL2 Ubuntu on the Windows desktop for CUDA development unless setup friction proves higher than expected. WSL2 keeps the Rust/CUDA workflow closer to Linux and the Mac-side development environment. Native Windows remains a fallback for profiling workflows, Nsight integration, or CUDA Toolkit/MSVC compatibility issues.

## Consequences

- The first milestone becomes concrete and testable.
- The Mac can contribute immediately through Rust core code, fixtures, and CPU benchmarks.
- The 4090 machine becomes the CUDA validation target.
- The larger middleware can depend on a stable `RouterBackend` trait instead of GPU details.
- Future RDMA and multi-GPU claims remain parked until the base routing path is proven.
- The first benchmark scale starts at 128 agents and expands by 8x steps: 128, 1024, 8192, and later 65536 if earlier results justify it.
- The local-model prototype can ignore model choice as a variable after pinning `Qwen3-2507` and focus on prompt, tool, MCP library, and memory-set variation.
- Natural batching is assumed, so single-task latency is measured as a boundary case rather than the main operating mode.
- Production callers receive available candidates. Telemetry preserves unavailable ideal candidates so route-quality loss can be measured without exposing unroutable agents to ordinary dispatch code.
- Queue depth starts as `pending_assigned_tasks / agent_capacity_window`; later versions can add separate execution pressure and failure cooldown signals.
- Substitute quality is measured first with distance-based metrics, not human preference or real task success. This keeps Prototype 1 focused on whether the routing geometry behaves as intended.
- The first radius policy targets at least 3 observed candidate agents and is tuned from benchmark results rather than fixed upfront.
- If local-agent routing does not produce useful behavior, the design can be narrowed or rewritten around remote-inference orchestration later.

## Action Items

1. Pin `Qwen3-2507` for local Prototype 1 tests.
2. Define the LLM-graded benchmark schema and evaluator rubric for agent capability vectors.
3. Build deterministic synthetic fixtures plus evaluator-produced fixture records.
4. Implement top-k CPU routing with `k = 1`, `k = 4`, and `k = 8`.
5. Include available top-k output, debug observed top-k output, and score explanation fields in CPU output.
6. Add substitute distance delta, top-k radius, and radius-for-3-candidates metrics.
7. Benchmark CPU routing on Mac and Windows.
8. Implement the simple CUDA router on the 4090 machine.
9. Compare CPU/GPU correctness and performance.
10. Decide whether optimized CUDA kernels are justified.
