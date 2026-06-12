# CUDA Toolchain Notes

Q-TOM's CUDA backend is still a conservative scaffold. The default workspace build does not require CUDA, NVIDIA headers, or CUDA import libraries.

## Cargo Features

`qtom-cuda` exposes one opt-in feature:

```sh
cargo test -p qtom-cuda --features cuda-runtime
```

The `cuda-runtime` feature enables CUDA Driver API availability detection. It dynamically loads the installed NVIDIA driver library at runtime:

- Windows: `nvcuda.dll`
- WSL2/Linux: `libcuda.so.1`, then `libcuda.so`

The detector calls `cuInit(0)` and `cuDeviceGetCount`. `CudaRuntime::initialize` additionally retains the device 0 primary context so resource wrappers can allocate safely. Module loading uses a checked-in PTX artifact and does not require CUDA Toolkit headers. The feature-gated test path performs synchronous host/device copies, launches `qtom_route_agents_k1` through `cuLaunchKernel`, synchronizes the stream immediately, and compares a tiny `k = 1` fixture against the CPU router.

## Expected Status

Without `cuda-runtime`, runtime detection reports:

```text
runtime_available=false
runtime_reason="CUDA runtime feature is disabled"
```

With `cuda-runtime`, a machine with a working NVIDIA driver should report a positive device count. The feature test also creates and destroys a stream, allocates and frees a tiny `f32` device buffer, verifies zero-length buffers, loads the checked-in PTX module, looks up `qtom_route_agents_k1`, packs typed launch parameters from `CudaBufferPlan`, copies plain `f32`/`u32` buffers, launches a naive one-thread-per-request `k = 1` scoring kernel, copies outputs back, decodes `RoutingResult` values including fallback behavior, and checks them against CPU results over tiny and deterministic generated fixtures. `CudaRouter::route_batch` now uses that path for valid `k = 1` batches when the runtime initializes; unsupported `k` values and runtime failures still return `BackendUnavailable`.

## Kernel Artifact

The first kernel artifact is checked in as PTX:

```text
crates/qtom-cuda/kernels/route_agents.ptx
```

The matching CUDA source is:

```text
crates/qtom-cuda/kernels/route_agents.cu
```

Until a build step is added, keep these in sync manually. The PTX currently implements the first naive `k = 1` scoring path: one CUDA thread per request, a full scan over all agents, CPU-equivalent score coefficients, best-available output, base/effective distance output, and an ideal-unavailable flag.

## Windows Target

The primary CUDA validation target is the Windows 11 desktop with an RTX 4060, 8 GB dedicated VRAM, and 32 GB host RAM.

For native Windows development:

1. Install a recent NVIDIA driver.
2. Install the CUDA Toolkit only when kernel compilation is needed.
3. Keep `cargo test --workspace` passing without CUDA features.
4. Use `cargo test -p qtom-cuda --features cuda-runtime` to test driver detection.

## WSL2 Target

For WSL2 CUDA development:

1. Use the Windows NVIDIA driver with WSL CUDA support.
2. Do not install a Linux NVIDIA kernel driver inside WSL2.
3. Install the WSL-compatible CUDA Toolkit only when kernel compilation is needed.
4. Confirm the driver library is visible through `ldconfig` or the standard library path.

## Current Integration Boundary

The runtime boundary now has RAII wrappers for the retained primary context, streams, typed device buffers, modules, function handles, and a q-tom-specific route-kernel launch wrapper. `RouteAgentsKernelArgs` consumes a validated `CudaBufferPlan`, checks buffer lengths, keeps output buffers mutably borrowed, packs the exact `qtom_route_agents_k1` parameters including score coefficients, launches synchronously, and reports typed CUDA errors. The `k = 1` execution helper allocates, copies, launches, synchronizes, copies back, and decodes CUDA output arrays into CPU-shaped `RoutingResult` values.

The public boundary is now intentionally narrow: `CudaRouter::route_batch` may return CUDA results only when `cuda-runtime` is enabled, the driver initializes, request/state shapes validate, and every request uses `k = 1`. The benchmark CLI exposes this as `--write-cuda-golden <path>` for the default `k = 1` fixture, `--cuda-parity <path>` for CPU-vs-public-CUDA parity, and `--cuda-timing <path>` for whole-call timing after parity is checked. CUDA parity keeps task IDs, candidate IDs, flags, and candidate counts exact, while allowing a small absolute tolerance on floating score fields. CUDA timing now reports runtime init, host preparation, allocation, host/device copies, module/stream setup, kernel launch/sync, and decode. The next boundary is reusing CUDA runtime/module resources across `k = 1` batches so the timing path stops paying setup costs every call. Do not add `k > 1` or kernel-level optimization work until the `k = 1` public path remains boringly deterministic. That work must follow `docs/cuda-safety.md`: checked sizes, explicit ownership, small documented `unsafe` blocks, and typed errors for every CUDA failure.
