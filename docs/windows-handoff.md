# Windows / RTX 4090 Handoff

This note is the compact context handoff for reopening the Q-TOM work on the Windows 11 desktop with the RTX 4090.

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
- `qtom-cuda` scaffold that compiles without CUDA and reports unavailable until host runtime/kernels exist
- CUDA flat buffer-layout planner driven from the same golden fixture

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
cargo run -p qtom-bench --release -- --write-golden work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --golden-parity work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --cuda-plan work/golden/8192x2048d16k8.fixture
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
```

`available=false` is expected. The CUDA backend scaffold intentionally reports unavailable until the CUDA runtime and kernels are wired.

## Important Files

```text
crates/qtom-core/src/backend.rs     shared RouterBackend trait and parity harness
crates/qtom-core/src/cpu_router.rs  CPU correctness oracle
crates/qtom-core/src/golden.rs      golden fixture reader/writer
crates/qtom-cuda/src/lib.rs         CUDA scaffold and buffer plan
crates/qtom-bench/src/main.rs       benchmark and fixture CLI
docs/implementation-spec.md         architecture and roadmap
```

## Current Benchmark Reference

Recent CPU smoke result at `8192 agents / 2048 tasks / 16 dims`:

```text
k=1 p99 ~= 55 us
k=4 p99 ~= 59 us
k=8 p99 ~= 63 us
```

Treat these as directional local Mac numbers, not final claims.

## Design Constraints

- CPU remains the correctness oracle.
- CUDA must match CPU on golden fixtures before optimization.
- The first CUDA kernel should be naive and boring.
- Start with `k = 1`, one CUDA thread routing one task against all agents.
- Do not optimize until CPU/GPU parity is proven.
- Lossy deterministic candidate generation is a later scale feature, not the next task.

## Next Task

Continue from:

```text
Build the host-side CUDA runtime boundary for qtom-cuda.
```

Recommended next implementation slice:

1. Add a Cargo feature such as `cuda-runtime`.
2. Add CUDA availability detection that is disabled by default on non-CUDA hosts.
3. Add `kernels/route_agents.cu` as a placeholder source file.
4. Add build integration or documentation for the Windows CUDA toolchain.
5. Keep the crate compiling without CUDA.
6. Keep `CudaRouter` returning `BackendUnavailable` until the runtime boundary is real.

After that, implement the first naive `k = 1` CUDA parity kernel.

## Prompt To Use In New Codex Thread

```text
Continue Q-TOM CUDA scaffold from docs/windows-handoff.md. Start with host-side CUDA runtime/build integration for the qtom-cuda crate. Keep non-CUDA hosts compiling. Do not optimize yet; preserve CPU/golden-fixture parity as the goal.
```
