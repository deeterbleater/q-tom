# Q-TOM Agent Directives

These directives apply to automated coding agents working in this repository.

## CUDA Safety

Before changing CUDA runtime, kernel, FFI, or device-buffer code, read `docs/cuda-safety.md`.

- Keep CUDA integration correctness-first. CPU and golden-fixture parity are the safety oracle.
- Keep CUDA runtime integration behind an opt-in Cargo feature so the workspace compiles without CUDA by default.
- Keep public Rust APIs safe. Do not expose raw CUDA pointers, handles, streams, modules, or lifetimes through safe public APIs.
- Isolate every `unsafe` block. Keep it small, local, and documented with the exact invariant being upheld.
- Use RAII wrappers for CUDA resources, including allocations, streams, modules, contexts, and loaded kernels.
- Validate request shape, state length, dimensions, `k`, and buffer plans before allocation, copies, or kernel launch.
- Use checked arithmetic for buffer lengths and byte sizes.
- Treat `CudaBufferPlan` and related typed layout helpers as the single source of truth for flat device-buffer sizes.
- Copy only plain data across the host/device boundary: `f32`, `u32`, or explicitly stable `repr(C)` POD structs.
- Guard every CUDA kernel thread against out-of-bounds reads and writes.
- Start with synchronized launches and copies. Add async behavior only after ownership and lifetimes are explicit.
- Convert every CUDA allocation, copy, module, launch, and synchronization failure into a typed Rust error.
- On CUDA failure, return a clear backend error rather than partial or uninitialized route results.
- Do not optimize through undefined behavior, unchecked indexing, aliasing tricks, lifetime extension, or layout assumptions.
