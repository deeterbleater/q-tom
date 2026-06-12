# CUDA Safety Constraints

Q-TOM's CUDA integration must stay correctness-first and memory-safe at the Rust boundary. The first CUDA implementation should be simple enough that failures become normal errors, not memory corruption.

## Core Rule

CPU routing and golden-fixture parity are the safety oracle. GPU output is not trusted until it matches CPU output on deterministic fixtures. Optimization starts only after parity is proven.

## Constraints

1. Keep CUDA behind a feature gate.
   `qtom-cuda` must compile and test without CUDA by default. Runtime and toolchain-specific code should live behind an opt-in feature such as `cuda-runtime`.

2. Keep the public Rust API safe.
   Public callers should see `CudaRouter`, typed plans or buffers, and `Result` values. Raw CUDA pointers, streams, modules, handles, and lifetimes must not escape through safe public APIs.

3. Isolate every `unsafe` block.
   Each `unsafe` block must be small, local, and paired with a short comment explaining the invariant: pointer validity, byte length, alignment, lifetime, stream synchronization, or ABI expectations.

4. Use RAII wrappers for CUDA resources.
   Device allocations, streams, modules, kernels, and contexts should be owned by Rust types with `Drop`, so cleanup is automatic and double-free is structurally hard.

5. Validate all shapes before allocation or launch.
   Agent count, request count, dimensions, `k`, state length, and output slot counts must be checked on the host before any CUDA allocation, copy, or launch.

6. Use checked arithmetic for buffer sizes.
   Do not compute allocation sizes with unchecked multiplication or addition. Use checked arithmetic or prevalidated layout helpers before deriving element counts or byte lengths.

7. Keep buffer layout single-source.
   `CudaBufferPlan` and related typed layout helpers should remain the source of truth for flat buffer lengths. Kernel launch code should consume validated plans rather than recomputing sizes ad hoc.

8. Prefer typed buffers over byte buffers.
   Device memory should be represented as typed buffers such as `DeviceBuffer<f32>` and `DeviceBuffer<u32>`. Use raw `*mut c_void` only at the FFI boundary.

9. Copy only plain data.
   Host/device transfer types must be plain data: `f32`, `u32`, or explicitly stable `repr(C)` POD structs. Do not copy Rust structs containing `Vec`, references, padding-sensitive fields, or non-stable layouts.

10. Guard every kernel thread.
    CUDA kernels must check bounds such as `if task_idx >= task_count { return; }` before reading or writing per-task data.

11. Start synchronous.
    Initial integration should synchronize after launches and copies. Async streams can come later only after buffer ownership, lifetimes, and synchronization points are explicit.

12. Treat CUDA errors as data.
    Every CUDA allocation, copy, module load, kernel launch, and synchronization result must be checked and converted into `RouteError` or a CUDA-specific error type.

13. Preserve CPU parity as the release gate.
    CUDA routing must match CPU routing on golden fixtures before any benchmark result is treated as meaningful.

14. Cap stress workloads explicitly.
    The Windows CUDA target is an RTX 4060 with 8 GB dedicated VRAM. Large fixtures, high batch sizes, and future `65536+` agent stress tests should be opt-in and size-gated.

15. Keep fallback behavior safe.
    If CUDA availability, allocation, module loading, copying, launch, or synchronization fails, `CudaRouter` should return a clear backend error. It must never return partial or uninitialized route results.

16. Test unsafe boundaries directly.
    Add tests for invalid dimensions, state length mismatch, zero-sized or empty cases, oversized plans, unavailable CUDA runtime, and buffer-plan overflow.

17. Do not use undefined behavior as an optimization.
    Avoid unchecked indexing, aliasing tricks, lifetime extension, `transmute`-based layout assumptions, and kernel-side out-of-bounds risk. Performance work must preserve the same safety boundaries as the naive implementation.

## Implementation Bias

The first CUDA path should be naive and boring: one thread per task, `k = 1` first, synchronous execution, explicit error checks, and exact parity with the CPU route. Make correctness visible before making performance interesting.
