# Q-TOM

Q-TOM is a prototype routing layer for local multi-agent orchestration. It tests whether a GPU-resident vector routing kernel can eventually make agent selection faster, more stable, or more scalable than a CPU routing loop.

The current implementation is the Phase 0/1 truth source:

- deterministic synthetic fixtures
- CPU top-k router
- observed-vs-available candidate output
- geometric substitute-quality metrics
- benchmark smoke runner with latency percentiles

CUDA is intentionally not implemented yet. The CPU route is the correctness oracle for the future RTX 4090 backend.

## Current Prototype Decisions

- Local-first orchestration target
- Fixed local model profile: `Qwen3-2507`
- LLM-graded benchmark plan with GPT-5.5 Medium via API as intended evaluator
- Default top-k: `8`
- Initial agent count: `128`, scaling by factors of `8`
- Queue pressure starts as `pending_assigned_tasks / agent_capacity_window`
- Production fast path returns available candidates; debug telemetry preserves observed candidates

## Setup

```sh
cp .env.example .env
cargo test --workspace
cargo run -p qtom-bench --release
cargo run -p qtom-bench --release -- --stress
cargo run -p qtom-bench --release -- --profile
```

Add real secrets only to `.env`. Do not commit `.env`.

The benchmark runner prints CSV-style rows for the current CPU router across:

- agent counts: `128`, `1024`, `8192`
- top-k values: `1`, `4`, `8`
- latency summaries: p50, p95, p99, max per routed task

Use `--stress` to run the opt-in `65536`-agent scenario.

Use `--profile` to compare raw nearest-distance scanning against the full CPU router. This helps isolate whether the current bottleneck is the vector scan itself or router bookkeeping.
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
docs/          # design/spec documents
```

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace
```

## Notes

This is an early prototype. The next major milestone is a CUDA backend that matches the CPU router exactly before attempting optimization.
