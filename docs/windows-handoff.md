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
- naive `k = 1` CUDA scoring kernel with a tiny CPU parity test
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
crates/qtom-cuda/kernels/route_agents.cu  naive k=1 CUDA kernel source
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
cpu-parallel   avg_batch_ms ~= 8.5
cuda           avg_batch_ms ~= 71.9
```

Treat this as a correctness-first baseline for the current public CUDA path. It includes runtime setup, allocation, host/device copies, kernel launch/sync, copy-back, and decode.

Current CUDA timing breakdown:

```text
runtime_init_ms       ~= 42.2
host_prepare_ms       ~= 0.1
device_allocate_ms    ~= 0.1
host_to_device_ms     ~= 0.4
module_stream_setup_ms ~= 9.9
kernel_launch_sync_ms ~= 5.6
device_to_host_ms     ~= 0.1
decode_ms             ~= 1.7
```

The first obvious cost center is setup, not transfer. Reuse runtime/module resources before kernel-level optimization.

## Design Constraints

- CPU remains the correctness oracle.
- CUDA must match CPU on golden fixtures before optimization.
- CUDA runtime and kernel work must follow `docs/cuda-safety.md`.
- The first CUDA kernel should be naive and boring.
- Start with `k = 1`, one CUDA thread routing one task against all agents.
- Do not optimize until CPU/GPU parity is proven.
- Lossy deterministic candidate generation is a later scale feature, not the next task.

## Next Task

Continue from:

```text
Reuse CUDA runtime/module resources across public k = 1 batches.
```

Recommended next implementation slice:

1. Keep all CUDA execution behind `cuda-runtime`.
2. Add a reusable `CudaRouteK1Executor`-style internal object that owns the runtime, module, and route kernel lookup.
3. Keep per-call device buffers temporary at first; do not add buffer pooling yet.
4. Preserve `BackendUnavailable` or shape errors for all unsupported cases.
5. Keep CPU parity tests for tiny and deterministic generated fixtures with debug telemetry disabled.
6. Keep default non-CUDA builds compiling.
7. Do not optimize or add `k > 1` until this narrow path is stable.

## Prompt To Use In New Codex Thread

```text
Continue Q-TOM CUDA scaffold from docs/windows-handoff.md. Start by reusing CUDA runtime/module resources across public CudaRouter::route_batch calls on the narrow k = 1 CUDA golden fixture while preserving BackendUnavailable for unsupported cases. Keep per-call buffers temporary for now. Keep non-CUDA hosts compiling. Do not optimize the kernel yet; preserve CPU/golden-fixture parity as the goal.
```
