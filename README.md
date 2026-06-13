# Q-TOM

Q-TOM is a prototype routing layer for local multi-agent orchestration. It tests whether a GPU-resident vector routing kernel can eventually make agent selection faster, more stable, or more scalable than a CPU routing loop.

The current implementation is the Phase 0/1 truth source:

- deterministic synthetic fixtures
- CPU top-k router
- packed row-major agent route table
- blocked distance kernel for small fixed-width vectors
- stack-backed top-k for `k <= 8`
- specialized single-winner path for `k = 1`
- order-preserving batch router with configurable worker count
- explicit backend trait with exact parity harness
- CUDA backend scaffold and buffer-layout planner
- observed-vs-available candidate output
- geometric substitute-quality metrics
- benchmark runners for latency, scan overhead, memory layout, and batch throughput

CUDA is intentionally gated behind CPU parity. The CPU route is the correctness oracle for the RTX 4060 backend, and the public CUDA router currently opens only the narrow `k = 1` path when `cuda-runtime` is enabled and the NVIDIA driver initializes. CUDA runtime and kernel work must follow the safety constraints in `docs/cuda-safety.md`.

## Current Prototype Decisions

- Local-first orchestration target
- Fixed local model profile: `Qwen3-2507`
- LLM-graded benchmark plan with GPT-5.5 Medium via API as intended evaluator
- Default top-k: `8`
- Initial agent count: `128`, scaling by factors of `8`
- Queue pressure starts as `pending_assigned_tasks / agent_capacity_window`
- Production fast path returns available candidates and tracks only the nearest observed candidate needed for `ideal_candidate_unavailable`
- Debug telemetry preserves full observed top-k candidates

## Setup

```sh
cp .env.example .env
cargo test --workspace
cargo test -p qtom-cuda --features cuda-runtime
cargo run -p qtom-bench --release
cargo run -p qtom-bench --release -- --stress
cargo run -p qtom-bench --release -- --profile
cargo run -p qtom-bench --release -- --layout-profile
cargo run -p qtom-bench --release -- --batch-profile
cargo run -p qtom-bench --release -- --prod-profile
cargo run -p qtom-bench --release -- --candidate-prefilter-profile
cargo run -p qtom-bench --release -- --write-golden work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --golden-parity work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --cuda-plan work/golden/8192x2048d16k8.fixture
cargo run -p qtom-bench --release -- --write-cuda-golden work/golden/8192x2048d16k1.fixture
cargo run -p qtom-bench --release --features qtom-cuda/cuda-runtime -- --cuda-parity work/golden/8192x2048d16k1.fixture
cargo run -p qtom-bench --release --features qtom-cuda/cuda-runtime -- --cuda-timing work/golden/8192x2048d16k1.fixture
```

Add real secrets only to `.env`. Do not commit `.env`.

The benchmark runner prints CSV-style rows for the current CPU router across:

- agent counts: `128`, `1024`, `8192`
- top-k values: `1`, `4`, `8`
- latency summaries: p50, p95, p99, max per routed task

Use `--stress` to run the opt-in `65536`-agent scenario.

Use `--profile` to compare raw nearest-distance scanning against the full CPU router. This helps isolate whether the current bottleneck is the vector scan itself or router bookkeeping.

Use `--layout-profile` to compare legacy agent-struct scans, packed row-major scans, and blocked distance-kernel scans.

Use `--batch-profile` to compare order-preserving batch routing across worker counts with observed-candidate debug telemetry enabled and disabled. This helps isolate request-level parallelism from observability/materialization overhead.

Use `--prod-profile` to measure production routing with observed-candidate debug telemetry disabled across worker counts, vector dimensions, and `k=1` / `k=8`.

Use `--write-golden` to export the default `8192 agents / 2048 tasks / 16 dims / k=8` deterministic fixture with exact `f32` bit-pattern encoding. Use `--golden-parity` to read that fixture back and compare sequential CPU routing against parallel CPU routing. This is the first parity artifact for the future CUDA backend.

Use `--cuda-plan` to read a golden fixture through the CUDA scaffold and print the planned flat device-buffer sizes. The scaffold compiles on non-CUDA hosts. Backend status remains conservative for general routing, while `CudaRouter::route_batch` can route valid `k = 1` batches under `cuda-runtime` and returns unavailable for unsupported shapes or runtime failures.

Use `--write-cuda-golden` to export the default `8192 agents / 2048 tasks / 16 dims / k=1` deterministic CUDA parity fixture. Use `--cuda-parity` with `--features qtom-cuda/cuda-runtime` to compare CPU routing against public `CudaRouter::route_batch` over that fixture. CUDA parity requires identical route decisions and allows only a small absolute tolerance on floating score fields.

Use `--cuda-timing` with `--features qtom-cuda/cuda-runtime` to time whole public `route_batch` calls over the CUDA `k=1` golden fixture after first checking CPU/CUDA parity. This measures the current integration boundary and prints a CUDA stage breakdown for runtime init, host preparation, allocation, host/device copies, module/stream setup, host launch/sync wall time, CUDA event device time, inferred host overhead, and decode.

Use `--cuda-scale` with `--features qtom-cuda/cuda-runtime` to generate deterministic `k=1`, `dims=16` fixtures across multiple agent counts while holding task count fixed. This probes how exact CUDA scoring responds to smaller curated candidate sets before broader memory-retrieval or lossy prefilter work.

Use `--candidate-prefilter-profile` to measure deterministic CPU-only lossy prefilters against exact CPU routing. It compares single 2D, single 3D, stacked 2D, and stacked 3D projection strategies, then reports scan reduction, top-1 recall, ideal-unavailable flag agreement, prefilter time, and exact subset scoring time.

See `docs/benchmark-ledger.md` for the current CPU p99, CUDA timing, CUDA stage-breakdown, candidate-set scale, and prefilter-recall numbers.

Use `cargo test -p qtom-cuda --features cuda-runtime` to opt into CUDA Driver API availability detection and resource-wrapper smoke tests. It verifies that the NVIDIA driver runtime can be loaded, queried, used for tiny stream/device-buffer lifecycles, used to load the route-kernel module, used for typed host/device copies, and used for decoded `k = 1` CPU parity through both the internal helper and public `CudaRouter::route_batch`. See `docs/cuda-toolchain.md`.

Treat profile output as a coarse signal. Run it multiple times on a quiet machine before drawing hard conclusions.

## Environment

Supported configuration keys:

```sh
OPENAI_API_KEY=
QTOM_EVALUATOR_MODEL=gpt-5.5-medium
QTOM_LOCAL_MODEL=Qwen3-2507
QTOM_DEFAULT_K=8
QTOM_DEFAULT_AGENT_COUNT=128
```

The code only reports whether `OPENAI_API_KEY` is present. It does not print the key.

## Repository Layout

```text
crates/
  qtom-core/   # core types, CPU router, fixtures, metrics
  qtom-bench/  # CPU benchmark smoke runner
  qtom-cuda/   # CUDA backend scaffold and buffer layout
docs/          # design/spec documents
```

Automated coding agents should also follow the repository directives in `AGENTS.md`.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace
```

## Notes

This is an early prototype. CUDA event timing now shows launch/sync overhead is small for the reusable `k = 1` path; the next major milestone is reducing repeated full-agent-scan work in the kernel while preserving exact CPU/golden-fixture route parity.
