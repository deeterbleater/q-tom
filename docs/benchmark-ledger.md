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
cargo run -p qtom-bench --release -- --candidate-prefilter-profile
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

## Candidate Prefilter Probe

`--candidate-prefilter-profile` is a CPU-only benchmark for lossy candidate generation. It compares deterministic coarse-grid projections, trims to a final candidate budget by projected distance, then runs the normal exact score function only over that subset. The exact full CPU router remains the recall oracle.

Current strategies:

```text
2d-single   dims [0,1]
3d-single   dims [0,1,2]
2d-stacked  union of dims [0,1], [2,3], [4,5], [6,7]
3d-stacked  union of dims [0,1,2], [3,4,5], [6,7,8]
```

Representative run:

```text
strategy    agents  tasks  budget  scan_reduction  top1_recall  ideal_flag_match  total_ms
2d-single   8192    512    128     0.984           0.3535       0.9668            5.109
3d-single   8192    512    128     0.984           0.5469       0.9746            8.776
2d-stacked  8192    512    128     0.984           0.8418       0.9941            219.175
3d-stacked  8192    512    128     0.984           0.9336       1.0000            168.246
2d-single   65536   256    1024    0.984           0.4961       0.9805            29.265
3d-single   65536   256    1024    0.984           0.6836       0.9844            51.541
2d-stacked  65536   256    1024    0.984           0.9336       1.0000            1307.716
3d-stacked  65536   256    1024    0.984           1.0000       1.0000            1014.820
2d-single   262144  64     1024    0.996           0.1250       0.9844            30.919
3d-single   262144  64     1024    0.996           0.2500       0.9531            28.656
2d-stacked  262144  64     1024    0.996           0.6250       1.0000            591.388
3d-stacked  262144  64     1024    0.996           0.6719       0.9844            379.647
```

The quality result is positive: adding a third dimension improves recall, and stacking independent projections improves recall substantially. At `65536` agents and a `1024` candidate budget, `2d-single` recalls `49.61%`, `3d-single` recalls `68.36%`, `2d-stacked` recalls `93.36%`, and `3d-stacked` recalls `100%`.

The performance result is negative for this naive implementation: stacked projection expansion and union trimming cost too much as written. This is still useful because it separates candidate quality from prefilter mechanics. Memory-node curation should keep the stacked/multi-view idea, but use a cheaper retrieval structure than naive grid expansion, such as precomputed per-layer neighbor lists, inverted semantic keys, approximate-nearest structures, or curator-maintained shortlist tables.

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
-> lossy candidate generation quality
```

The next useful benchmark should measure shared-memory tiling or another exact-kernel improvement while keeping the candidate generator result in view: compact candidate sets are valuable, but they must come from a higher-recall memory curation layer than the toy 2D grid.
