# Windows / RTX 4060 Handoff

This note is the compact context handoff for reopening the Q-TOM work on the Windows 11 desktop with the RTX 4060.

## Current State

Q-TOM is a Rust prototype for topology-aware agent routing. The CPU implementation is the correctness oracle. The project now has:

- deterministic synthetic fixtures
- exact CPU top-k routing
- stack-backed top-k for `k <= 8`
- specialized `k = 1` path
- production profile mode
- size-gated production batch scanner
- golden fixture export/import
- `RouterBackend` trait and exact backend parity harness
- `qtom-cuda` scaffold that compiles without CUDA and keeps general CUDA routing conservative
- opt-in `cuda-runtime` feature for CUDA Driver API availability detection
- CUDA flat buffer-layout planner driven from the same golden fixture
- checked CUDA buffer-plan sizing
- typed RAII wrappers for the retained primary context, streams, and device buffers
- typed RAII wrappers for CUDA modules and function handles
- typed synchronous host/device copies for `f32` and `u32` buffers
- typed `qtom_route_agents_k1` parameter packing and synchronous launch
- conservative `k = 1` CUDA scoring kernel with a tiny CPU parity test
- internal `k = 1` CUDA execution helper that decodes `RoutingResult` values, including fallback/radius behavior
- deterministic generated-fixture parity for decoded `k = 1` CUDA results
- public `CudaRouter::route_batch` gate for valid `k = 1` CUDA routing under `cuda-runtime`
- public CUDA golden-fixture parity CLI for `k = 1`
- public CUDA whole-call timing CLI for `k = 1`
- checked-in `route_agents.ptx` module artifact

The public repo is:

```text
https://github.com/deeterbleater/q-tom
```

## First Commands On Windows

```sh
git clone https://github.com/deeterbleater/q-tom.git
cd q-tom
cp .env.example .env
cargo test --workspace
cargo test -p qtom-cuda --features cuda-runtime
cargo run -p qtom-bench --release -- --write-golden work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --golden-parity work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --cuda-plan work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --write-cuda-golden work/golden/8192x2048d16k1.fixture
cargo run -p qtom-bench --release --features qtom-cuda/cuda-runtime -- --cuda-parity work/golden/8192x2048d16k1.fixture
cargo run -p qtom-bench --release --features qtom-cuda/cuda-runtime -- --cuda-timing work/golden/8192x2048d16k1.fixture
```

Expected `--cuda-plan` shape for the default fixture:

```text
agents=8192
tasks=2048
dims=16
k=8
agent_vector_f32=131072
request_vector_f32=32768
output_slots=16384
total_f32=221184
total_u32=34816
available=false
runtime_available=false
runtime_devices=0
```

For the default `k=8` fixture, `available=false` is expected because general CUDA routing remains closed. Without `cuda-runtime`, `runtime_available=false` is also expected. With `cuda-runtime` on a working NVIDIA driver, runtime detection may report `runtime_available=true` and a positive device count; public `CudaRouter::route_batch` can route valid `k = 1` batches but still returns unavailable for `k > 1`.

## Important Files

```text
crates/qtom-core/src/backend.rs     shared RouterBackend trait and parity harness
crates/qtom-core/src/cpu_router.rs  CPU correctness oracle
crates/qtom-core/src/golden.rs      golden fixture reader/writer
crates/qtom-cuda/src/lib.rs         CUDA scaffold, buffer plan, copies, and launch boundary
crates/qtom-cuda/kernels/route_agents.cu  k=1 CUDA kernel source
crates/qtom-cuda/kernels/route_agents.ptx checked-in PTX artifact
crates/qtom-bench/src/main.rs       benchmark and fixture CLI
docs/cuda-safety.md                 CUDA memory-safety constraints
docs/cuda-toolchain.md              CUDA feature/toolchain notes
docs/implementation-spec.md         architecture and roadmap
AGENTS.md                           automated-agent directives
```

## Hardware Note

The Windows CUDA target is an RTX 4060 with 8 GB dedicated VRAM and 32 GB host RAM. Keep CUDA validation correctness-first, and treat large-batch or large-agent-count stress results as constrained by midrange GPU VRAM capacity and memory bandwidth.

## Current Benchmark Reference

See `docs/benchmark-ledger.md` for the current consolidated CPU p99, CUDA timing, CUDA stage-breakdown, and `--cuda-scale` results.

Recent CPU smoke result at `8192 agents / 2048 tasks / 16 dims`:

```text
k=1 p99 ~= 55 us
k=4 p99 ~= 59 us
k=8 p99 ~= 63 us
```

Treat these as directional local Mac numbers, not final claims.

Recent Windows RTX 4060 whole-batch timing at `8192 agents / 2048 tasks / 16 dims / k=1`:

```text
cpu-sequential avg_batch_ms ~= 81.9
cpu-parallel   avg_batch_ms ~= 9.6
cuda           avg_batch_ms ~= 68.0
cuda-reuse     avg_batch_ms ~= 5.0
```

Treat this as a correctness-first baseline. The `cuda` row measures the public router path with setup paid per call. The `cuda-reuse` row reuses runtime/module/buffer resources for the fixed golden shape.

Current CUDA timing breakdown:

```text
cuda-reuse total_ms              ~= 5.0
cuda-reuse host_prepare_ms       ~= 0.0
cuda-reuse device_allocate_ms    ~= 0.0
cuda-reuse host_to_device_ms     ~= 0.2
cuda-reuse module_stream_setup_ms ~= 0.0
cuda-reuse kernel_launch_sync_ms ~= 4.7-5.1
cuda-reuse kernel_device_ms      ~= 4.6-5.1
cuda-reuse kernel_host_overhead_ms ~= 0.0
cuda-reuse device_to_host_ms     ~= 0.1
cuda-reuse decode_ms             ~= 0.1
```

After runtime/module/buffer reuse, decode lookup cleanup, CUDA event timing, a `dimensions == 16` unrolled distance path, and precomputed per-agent score weights, the obvious cost center is still actual device work scanning every agent for every request. Launch/sync overhead, allocation, transfer, and decode are not the current bottlenecks for this fixture.

Recent `--cuda-scale` timing holds `tasks=2048`, `dims=16`, and `k=1` constant while varying candidate-set size:

```text
agents   cuda-reuse avg ms   device ms   speedup vs CPU parallel
512      ~= 0.49             ~= 0.29     ~= 2.6-3.0x
1024     ~= 0.78             ~= 0.57     ~= 2.3-2.6x
2048     ~= 1.4              ~= 1.14     ~= 1.9-2.2x
4096     ~= 2.6              ~= 2.35     ~= 1.7-1.8x
8192     ~= 5.0              ~= 4.6-5.0  ~= 1.4-1.6x
16384    ~= 9.5              ~= 8.6-9.3  ~= 1.5-1.7x
32768    ~= 20               ~= 17-19    ~= 1.4-1.6x
```

This supports the memory-curation architecture: exact CUDA scoring is useful, but the strongest lever is handing it compact curated candidate sets rather than asking every request to scan the whole archive.

## Design Constraints

- CPU remains the correctness oracle.
- CUDA must match CPU on golden fixtures before optimization.
- CUDA runtime and kernel work must follow `docs/cuda-safety.md`.
- CUDA kernel changes should stay conservative and parity-first.
- Start with `k = 1`, one CUDA thread routing one task against all agents.
- Do not optimize until CPU/GPU parity is proven.
- Lossy deterministic candidate generation and memory curation are scale features, but the `--cuda-scale` curve says they are central to the architecture rather than decorative.

## Next Task

Continue from:

```text
Model curated candidate-set routing before widening beyond k = 1.
```

Recommended next implementation slice:

1. Keep all CUDA execution behind `cuda-runtime`.
2. Preserve the CUDA event timing columns while changing kernel internals.
3. Keep CPU/CUDA parity checks before timing comparisons.
4. Preserve `BackendUnavailable` or shape errors for all unsupported cases.
5. Keep CPU parity tests for tiny and deterministic generated fixtures with debug telemetry disabled.
6. Keep default non-CUDA builds compiling.
7. Do not add `k > 1` until the optimized `k = 1` path remains stable.
8. Prefer a measured next step such as a candidate-set or memory-node prefilter benchmark, tiled/shared-memory request or agent reuse, and keep exact CPU parity as the gate.

## Prompt To Use In New Codex Thread

```text
Continue Q-TOM CUDA scaffold from docs/windows-handoff.md. The k = 1 CUDA path now has CUDA event timing, a dimensions == 16 unrolled distance path, precomputed per-agent score weights, and a --cuda-scale probe showing exact scoring is roughly linear in candidate count. Start the next measured improvement with memory-curated candidate sets in mind, likely a candidate-set/prefilter benchmark or shared-memory tile experiment, while preserving BackendUnavailable for unsupported cases, keeping non-CUDA hosts compiling, and preserving CPU/golden-fixture parity as the goal.
```
