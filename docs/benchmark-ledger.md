# Benchmark Ledger

Last updated: 2026-06-13 on the Windows RTX 4060 target.

This document keeps the current benchmark numbers in one place. Treat these as directional local measurements, not universal claims. Re-run the listed commands on a quiet machine before using the numbers for a hard performance claim.

## Hardware And Scope

- GPU target: NVIDIA RTX 4060, 8 GB dedicated VRAM.
- CPU target: same Windows desktop, `available_parallelism = 28` in the current run.
- CUDA public route remains intentionally narrow: `k = 1`, `dims = 16` fast path, CPU parity required.
- CUDA timing is most meaningful in the reusable path, because the public `cuda` row still pays runtime/module/buffer setup per call.

## Commands

```sh
cargo run -p qtom-bench --release
cargo run -p qtom-bench --release -- --prod-profile
cargo run -p qtom-bench --release -- --batch-profile
cargo run -p qtom-bench --release --features qtom-cuda/cuda-runtime -- --cuda-timing work/golden/8192x2048d16k1.fixture
cargo run -p qtom-bench --release --features qtom-cuda/cuda-runtime -- --cuda-scale
```

## CPU Smoke Latency

The default smoke benchmark reports per-route latency distributions for sequential CPU routing.

```text
agents  tasks  dims  k  total_ms  routes_s   p50_us  p95_us  p99_us
128     128    16    1  0.102     1261084    0.700   0.865   2.246
128     128    16    4  0.170     750733     1.200   1.630   3.600
128     128    16    8  0.251     509960     1.700   2.900   4.073
1024    512    16    1  2.618     195569     5.000   5.100   6.956
1024    512    16    4  4.048     126492     7.200   9.090   19.001
1024    512    16    8  4.223     121241     8.100   8.400   13.012
8192    2048   16    1  80.759    25360      39.200  41.300  43.653
8192    2048   16    4  114.681   17858      54.700  60.200  70.100
8192    2048   16    8  114.824   17836      55.200  58.600  68.759
```

The important `k = 1` baseline for CUDA comparison is:

```text
CPU sequential exact full scan, 8192 agents / 2048 tasks / 16 dims / k=1:
total_ms ~= 80.8
p99 route latency ~= 43.7 us
```

## CPU Batch Throughput

The CUDA timing command also times a CPU parallel whole-batch row over the same `8192x2048d16k1` fixture. In the latest run:

```text
backend         workers  avg_batch_ms  p99_batch_ms  routes_s
cpu-sequential  1        81.351        83.450        25175
cpu-parallel    28       9.299         10.910        220230
```

This is a whole-batch distribution, not a per-route p99 distribution. For intuition only:

```text
cpu-parallel avg amortized time ~= 9.299 ms / 2048 routes ~= 4.54 us/route
cpu-parallel p99 batch amortized ~= 10.910 ms / 2048 routes ~= 5.33 us/route
```

## CUDA Timing

Latest `--cuda-timing` over `work/golden/8192x2048d16k1.fixture`:

```text
backend       avg_batch_ms  p99_batch_ms  routes_s
cuda          64.648        68.556        31680
cuda-reuse    5.088         5.356         402492
```

The public `cuda` row proves the safe public path works, but still pays runtime/module/buffer setup per call. The reusable row is the performance signal for a long-lived router.

Amortized intuition for the reusable row:

```text
cuda-reuse avg amortized time ~= 5.088 ms / 2048 routes ~= 2.48 us/route
cuda-reuse p99 batch amortized ~= 5.356 ms / 2048 routes ~= 2.62 us/route
```

That is faster than the CPU parallel whole-batch row for this workload:

```text
cpu-parallel avg_batch_ms / cuda-reuse avg_batch_ms ~= 1.83x
cpu-parallel p99_batch_ms / cuda-reuse p99_batch_ms ~= 2.04x
```

It is much faster than the sequential CPU exact scan as a whole batch:

```text
cpu-sequential avg_batch_ms / cuda-reuse avg_batch_ms ~= 16.0x
```

## CUDA Stage Breakdown

Latest reusable CUDA breakdown:

```text
stage                         avg_ms
total                         5.101
runtime_init                  0.000
runtime_teardown              0.000
host_prepare                  0.036
device_allocate               0.000
host_to_device                0.130
module_stream_setup           0.004
kernel_launch_sync            4.746
kernel_device                 4.720
kernel_host_overhead          0.026
device_to_host                0.049
decode                        0.133
```

Current conclusion:

```text
launch overhead is tiny
allocation is gone in reuse mode
copies and decode are small
device-side full-agent scanning is the bottleneck
```

## CUDA Scale Probe

`--cuda-scale` holds `tasks = 2048`, `dims = 16`, and `k = 1` constant while varying agent count. This models the effect of memory curation or candidate prefiltering: fewer agents means a smaller exact scoring candidate set.

Representative run:

```text
agents  candidates  cpu_parallel_avg_ms  cuda_reuse_avg_ms  cuda_device_ms  speedup
512     1048576     1.433                0.485              0.288           2.95x
1024    2097152     1.862                0.794              0.571           2.35x
2048    4194304     3.046                1.369              1.137           2.23x
4096    8388608     4.408                2.606              2.346           1.69x
8192    16777216    7.502                5.276              4.967           1.42x
16384   33554432    15.559               9.266              8.622           1.68x
32768   67108864    32.570               20.748             19.178          1.57x
```

This curve says exact CUDA scoring is useful, but still roughly linear in candidate count. The strongest architecture lever is not only making the full scan more clever; it is handing CUDA a compact, curated candidate set and then using CUDA for exact parity-preserving scoring.

## Memory-Curation Interpretation

For conversational memory, treat raw logs as canonical and memory nodes as indexed candidates:

```text
raw conversational logs -> curator agents -> compact memory-node candidate set -> exact Q-TOM/CUDA score -> hydrated context slice
```

The CUDA scale probe gives numbers for why this matters. If memory curators can reduce a candidate set from archive scale to a few hundred or a few thousand relevant memory nodes, exact scoring drops into sub-millisecond to low-millisecond territory while preserving deterministic route semantics.

## Current Bottleneck Map

The bottleneck has moved in this order:

```text
runtime/module setup
-> device allocation
-> decode lookup
-> launch-vs-device ambiguity
-> device-side kernel body
-> repeated full-agent/full-memory candidate scan
```

The next useful benchmark should measure either:

- candidate-set quality and recall for a memory-node prefilter, or
- shared-memory tiling in the exact `k = 1` CUDA scorer.

Either path should keep exact CPU/CUDA parity as the final correctness gate.
