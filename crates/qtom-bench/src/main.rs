use qtom_core::{
    AgentProfile, AgentRouteTable, BackendParityTolerance, BatchMetrics, CpuRouter, Fixture,
    FixtureConfig, ProjectConfig, RouterBackend, RoutingRequest, ScoreCoefficients,
    assert_backend_parity, assert_backend_parity_with_tolerance, batch_metrics, generate_fixture,
    read_golden_fixture, routing_results_checksum,
    score::{dist_sq, dist_sq_blocked, score_components_for_vector},
    write_golden_fixture,
};
use qtom_cuda::{CudaRouteK1Executor, CudaRouter, CudaRuntime};
use std::env;
use std::path::Path;
use std::time::{Duration, Instant};

const AGENT_COUNTS: [usize; 3] = [128, 1024, 8192];
const STRESS_AGENT_COUNTS: [usize; 1] = [65536];
const PROFILE_AGENT_COUNTS: [usize; 4] = [8192, 65536, 131072, 262144];
const BATCH_PROFILE_AGENT_COUNTS: [usize; 3] = [8192, 65536, 262144];
const PROFILE_DIMENSIONS: [usize; 2] = [16, 32];
const TOP_K_VALUES: [usize; 3] = [1, 4, 8];
const PROD_TOP_K_VALUES: [usize; 2] = [1, 8];
const CUDA_SCALE_AGENT_COUNTS: [usize; 7] = [512, 1024, 2048, 4096, 8192, 16384, 32768];
const CUDA_SCALE_TASK_COUNT: usize = 2048;
const CUDA_SCALE_DIMENSIONS: usize = 16;
const PREFILTER_AGENT_COUNTS: [usize; 3] = [8192, 65536, 262144];
const PREFILTER_CANDIDATE_BUDGETS: [usize; 6] = [32, 64, 128, 256, 512, 1024];
const PREFILTER_GRID_BINS: usize = 32;
const PREFILTER_2D_A: [usize; 2] = [0, 1];
const PREFILTER_2D_B: [usize; 2] = [2, 3];
const PREFILTER_2D_C: [usize; 2] = [4, 5];
const PREFILTER_2D_D: [usize; 2] = [6, 7];
const PREFILTER_3D_A: [usize; 3] = [0, 1, 2];
const PREFILTER_3D_B: [usize; 3] = [3, 4, 5];
const PREFILTER_3D_C: [usize; 3] = [6, 7, 8];
const CUDA_PARITY_SCORE_ABS_EPSILON: f32 = 1.0e-5;
const CUDA_TIMING_ITERATIONS: usize = 5;

fn main() {
    let mode = BenchMode::from_args(env::args().skip(1));
    let project_config = ProjectConfig::from_env_and_dotenv(".env");
    println!(
        "config mode={} local_model={} evaluator_model={} api_key_present={} default_k={} default_agents={}",
        mode.as_str(),
        project_config.local_model,
        project_config.evaluator_model,
        project_config.openai_api_key_present,
        project_config.default_k,
        project_config.default_agent_count
    );

    match &mode {
        BenchMode::Invalid(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
        BenchMode::Smoke | BenchMode::Stress => run_route_matrix(&mode),
        BenchMode::Profile => run_profile_matrix(),
        BenchMode::BatchProfile => run_batch_profile_matrix(),
        BenchMode::ProdProfile => run_prod_profile_matrix(),
        BenchMode::LayoutProfile => run_layout_profile_matrix(),
        BenchMode::CandidatePrefilterProfile => run_candidate_prefilter_profile(),
        BenchMode::WriteGolden { path } => run_write_golden(path),
        BenchMode::WriteCudaGolden { path } => run_write_cuda_golden(path),
        BenchMode::GoldenParity { path } => run_golden_parity(path),
        BenchMode::CudaParity { path } => run_cuda_parity(path),
        BenchMode::CudaTiming { path } => run_cuda_timing(path),
        BenchMode::CudaScale => run_cuda_scale(),
        BenchMode::CudaPlan { path } => run_cuda_plan(path),
    }
}

fn run_route_matrix(mode: &BenchMode) {
    println!(
        "agents,tasks,dims,k,total_ms,routes_s,p50_us,p95_us,p99_us,max_us,ideal_unavailable,mean_delta,mean_radius"
    );

    for &agent_count in mode.agent_counts() {
        for k in TOP_K_VALUES {
            run_scenario(FixtureConfig {
                agent_count,
                task_count: task_count_for(agent_count),
                dimensions: 16,
                k,
                seed: scenario_seed(agent_count, k),
            });
        }
    }
}

fn run_write_golden(path: &Path) {
    let config = golden_fixture_config();
    let fixture = generate_fixture(config);
    write_golden_fixture(path, config, &fixture).expect("golden fixture should be writable");
    let loaded = read_golden_fixture(path).expect("written golden fixture should be readable");

    assert_eq!(loaded.config, config);
    assert_eq!(loaded.fixture, fixture);

    println!(
        "golden_fixture_written,path={},agents={},tasks={},dims={},k={},seed={:016x}",
        path.display(),
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        config.seed
    );
}

fn run_write_cuda_golden(path: &Path) {
    let config = cuda_golden_fixture_config();
    let fixture = generate_fixture(config);
    write_golden_fixture(path, config, &fixture).expect("CUDA golden fixture should be writable");
    let loaded = read_golden_fixture(path).expect("written CUDA golden fixture should be readable");

    assert_eq!(loaded.config, config);
    assert_eq!(loaded.fixture, fixture);

    println!(
        "cuda_golden_fixture_written,path={},agents={},tasks={},dims={},k={},seed={:016x}",
        path.display(),
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        config.seed
    );
}

fn run_golden_parity(path: &Path) {
    let golden = read_golden_fixture(path).expect("golden fixture should be readable");
    let config = golden.config;
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1)
        .min(config.task_count.max(1));

    let requests = golden.fixture.requests;
    let states = golden.fixture.states;
    let agents = golden.fixture.agents;
    let reference = CpuWorkerBackend::new("cpu-sequential", agents.clone(), 1);
    let candidate = CpuWorkerBackend::new("cpu-parallel", agents, workers);
    let report = assert_backend_parity(&reference, &candidate, &requests, &states)
        .expect("CPU backends should match over golden fixture");

    println!(
        "golden_parity_ok,path={},reference={},candidate={},agents={},tasks={},dims={},k={},workers={},routes={},ideal_unavailable={},checksum={:.6}",
        path.display(),
        report.reference_backend,
        report.candidate_backend,
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        workers,
        report.routes,
        report.ideal_unavailable_count,
        report.checksum
    );
}

fn run_cuda_parity(path: &Path) {
    let golden = read_golden_fixture(path).expect("golden fixture should be readable");
    let config = golden.config;
    assert_eq!(config.k, 1, "CUDA parity requires a k=1 golden fixture");

    let requests = golden.fixture.requests;
    let states = golden.fixture.states;
    let agents = golden.fixture.agents;
    let reference = CpuWorkerBackend::new("cpu-sequential", agents.clone(), 1);
    let candidate = CudaRouter::new(agents, ScoreCoefficients::default());
    let status = candidate.status();
    let runtime = status.runtime;
    let report = assert_backend_parity_with_tolerance(
        &reference,
        &candidate,
        &requests,
        &states,
        BackendParityTolerance::new(CUDA_PARITY_SCORE_ABS_EPSILON),
    )
    .expect("CUDA backend should match CPU over k=1 golden fixture");

    println!(
        "cuda_parity_ok,path={},reference={},candidate={},agents={},tasks={},dims={},k={},routes={},ideal_unavailable={},checksum={:.6},score_abs_epsilon={:.8},runtime_available={},runtime_devices={},runtime_reason=\"{}\"",
        path.display(),
        report.reference_backend,
        report.candidate_backend,
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        report.routes,
        report.ideal_unavailable_count,
        report.checksum,
        CUDA_PARITY_SCORE_ABS_EPSILON,
        runtime.available,
        runtime.device_count,
        runtime.reason
    );
}

fn run_cuda_timing(path: &Path) {
    let golden = read_golden_fixture(path).expect("golden fixture should be readable");
    let config = golden.config;
    assert_eq!(config.k, 1, "CUDA timing requires a k=1 golden fixture");

    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1)
        .min(config.task_count.max(1));
    let requests = golden.fixture.requests;
    let states = golden.fixture.states;
    let agents = golden.fixture.agents;
    let route_table =
        AgentRouteTable::from_agent_slice(&agents).expect("golden fixture should be valid");
    let cpu_sequential = CpuWorkerBackend::new("cpu-sequential", agents.clone(), 1);
    let cpu_parallel = CpuWorkerBackend::new("cpu-parallel", agents.clone(), workers);
    let cuda = CudaRouter::new(agents, ScoreCoefficients::default());
    let status = cuda.status();
    let runtime = status.runtime;

    assert_backend_parity_with_tolerance(
        &cpu_sequential,
        &cuda,
        &requests,
        &states,
        BackendParityTolerance::new(CUDA_PARITY_SCORE_ABS_EPSILON),
    )
    .expect("CUDA backend should match CPU before timing");

    println!(
        "cuda_timing_header,path={},agents={},tasks={},dims={},k={},iterations={},score_abs_epsilon={:.8},runtime_available={},runtime_devices={},runtime_reason=\"{}\"",
        path.display(),
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        CUDA_TIMING_ITERATIONS,
        CUDA_PARITY_SCORE_ABS_EPSILON,
        runtime.available,
        runtime.device_count,
        runtime.reason
    );
    println!(
        "cuda_timing_columns,backend,workers,avg_batch_ms,p50_batch_ms,p95_batch_ms,p99_batch_ms,min_batch_ms,max_batch_ms,routes_s,ideal_unavailable,checksum"
    );

    print_backend_timing(
        &cpu_sequential,
        &requests,
        &states,
        1,
        CUDA_TIMING_ITERATIONS,
    );
    print_backend_timing(
        &cpu_parallel,
        &requests,
        &states,
        workers,
        CUDA_TIMING_ITERATIONS,
    );
    print_backend_timing(&cuda, &requests, &states, 0, CUDA_TIMING_ITERATIONS);
    let reuse_runtime = CudaRuntime::initialize().expect("CUDA runtime should initialize");
    let reuse_executor = CudaRouteK1Executor::new(&reuse_runtime, ScoreCoefficients::default())
        .expect("CUDA route executor should initialize");
    let cuda_reuse = CudaReuseBackend {
        name: "cuda-reuse",
        executor: reuse_executor,
        route_table,
    };
    print_backend_timing(&cuda_reuse, &requests, &states, 0, CUDA_TIMING_ITERATIONS);
    print_cuda_timing_breakdown(&cuda, &requests, &states, CUDA_TIMING_ITERATIONS);
    print_cuda_reuse_timing_breakdown(&cuda_reuse, &requests, &states, CUDA_TIMING_ITERATIONS);
}

fn run_cuda_scale() {
    let runtime = CudaRuntime::initialize().expect("CUDA runtime should initialize");
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1)
        .min(CUDA_SCALE_TASK_COUNT);

    println!(
        "cuda_scale_header,tasks={},dims={},k=1,iterations={},workers={}",
        CUDA_SCALE_TASK_COUNT, CUDA_SCALE_DIMENSIONS, CUDA_TIMING_ITERATIONS, workers
    );
    println!(
        "cuda_scale_columns,agents,tasks,dims,candidates,cpu_parallel_avg_ms,cuda_reuse_avg_ms,cuda_reuse_device_ms,cuda_reuse_host_to_device_ms,cuda_reuse_decode_ms,cuda_speedup_vs_cpu_parallel"
    );

    for agent_count in CUDA_SCALE_AGENT_COUNTS {
        let config = FixtureConfig {
            agent_count,
            task_count: CUDA_SCALE_TASK_COUNT,
            dimensions: CUDA_SCALE_DIMENSIONS,
            k: 1,
            seed: scenario_seed(agent_count, 1),
        };
        let fixture = generate_fixture(config);
        let route_table = AgentRouteTable::from_agent_slice(&fixture.agents)
            .expect("generated fixture should have a valid route table");
        let cpu_parallel = CpuWorkerBackend::new("cpu-parallel", fixture.agents.clone(), workers);
        let executor = CudaRouteK1Executor::new(&runtime, ScoreCoefficients::default())
            .expect("CUDA route executor should initialize");
        let cuda_reuse = CudaReuseBackend {
            name: "cuda-reuse",
            executor,
            route_table,
        };

        assert_backend_parity_with_tolerance(
            &cpu_parallel,
            &cuda_reuse,
            &fixture.requests,
            &fixture.states,
            BackendParityTolerance::new(CUDA_PARITY_SCORE_ABS_EPSILON),
        )
        .expect("CUDA reuse backend should match CPU before scale timing");

        let cpu_report = time_backend_batch(
            &cpu_parallel,
            &fixture.requests,
            &fixture.states,
            CUDA_TIMING_ITERATIONS,
        );
        let cuda_report = time_backend_batch(
            &cuda_reuse,
            &fixture.requests,
            &fixture.states,
            CUDA_TIMING_ITERATIONS,
        );
        let cuda_breakdown = time_cuda_reuse_route_breakdown(
            &cuda_reuse,
            &fixture.requests,
            &fixture.states,
            CUDA_TIMING_ITERATIONS,
        );
        let speedup = cpu_report.avg_batch_ms / cuda_report.avg_batch_ms.max(f64::EPSILON);

        println!(
            "cuda_scale,agents={},tasks={},dims={},candidates={},cpu_parallel_avg_ms={:.3},cuda_reuse_avg_ms={:.3},cuda_reuse_device_ms={:.3},cuda_reuse_host_to_device_ms={:.3},cuda_reuse_decode_ms={:.3},cuda_speedup_vs_cpu_parallel={:.3}",
            agent_count,
            config.task_count,
            config.dimensions,
            agent_count * config.task_count,
            cpu_report.avg_batch_ms,
            cuda_report.avg_batch_ms,
            cuda_breakdown.avg_kernel_device_ms,
            cuda_breakdown.avg_host_to_device_ms,
            cuda_breakdown.avg_decode_ms,
            speedup
        );
    }
}

fn run_cuda_plan(path: &Path) {
    let golden = read_golden_fixture(path).expect("golden fixture should be readable");
    let config = golden.config;
    let router = CudaRouter::new(golden.fixture.agents, ScoreCoefficients::default());
    let status = router.status();
    let runtime = status.runtime;
    let plan = router
        .buffer_plan(config.task_count, config.k)
        .expect("golden fixture should produce a CUDA buffer plan");

    println!(
        "cuda_plan,path={},backend={},available={},reason=\"{}\",runtime_available={},runtime_devices={},runtime_reason=\"{}\",agents={},tasks={},dims={},k={},agent_vector_f32={},request_vector_f32={},agent_score_weight_f32={},output_slots={},total_f32={},total_u32={}",
        path.display(),
        router.name(),
        status.available,
        status.reason,
        runtime.available,
        runtime.device_count,
        runtime.reason,
        plan.agent_count,
        plan.request_count,
        plan.dimensions,
        plan.k,
        plan.agent_vector_f32_len,
        plan.request_vector_f32_len,
        plan.agent_score_weight_f32_len,
        plan.output_candidate_u32_len,
        plan.total_f32_len(),
        plan.total_u32_len()
    );
}

fn print_backend_timing<B>(
    backend: &B,
    requests: &[qtom_core::RoutingRequest],
    states: &[qtom_core::AgentRuntimeState],
    workers: usize,
    iterations: usize,
) where
    B: RouterBackend,
{
    let report = time_backend_batch(backend, requests, states, iterations);
    println!(
        "cuda_timing,backend={},workers={},avg_batch_ms={:.3},p50_batch_ms={:.3},p95_batch_ms={:.3},p99_batch_ms={:.3},min_batch_ms={:.3},max_batch_ms={:.3},routes_s={:.1},ideal_unavailable={},checksum={:.6}",
        backend.name(),
        workers,
        report.avg_batch_ms,
        report.p50_batch_ms,
        report.p95_batch_ms,
        report.p99_batch_ms,
        report.min_batch_ms,
        report.max_batch_ms,
        report.routes_per_second,
        report.ideal_unavailable_count,
        report.checksum
    );
}

fn time_backend_batch<B>(
    backend: &B,
    requests: &[qtom_core::RoutingRequest],
    states: &[qtom_core::AgentRuntimeState],
    iterations: usize,
) -> BackendTimingReport
where
    B: RouterBackend,
{
    let warmup = backend
        .route_batch(requests, states)
        .expect("backend warmup should route fixture");
    let mut checksum = checksum_results(&warmup);
    let mut ideal_unavailable_count = warmup
        .iter()
        .filter(|result| result.ideal_candidate_unavailable)
        .count();
    let mut durations = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let results = backend
            .route_batch(requests, states)
            .expect("backend should route fixture");
        durations.push(start.elapsed());
        checksum = checksum_results(&results);
        ideal_unavailable_count = results
            .iter()
            .filter(|result| result.ideal_candidate_unavailable)
            .count();
    }

    BackendTimingReport::from_durations(
        durations,
        requests.len(),
        ideal_unavailable_count,
        checksum,
    )
}

fn print_cuda_timing_breakdown(
    cuda: &CudaRouter,
    requests: &[qtom_core::RoutingRequest],
    states: &[qtom_core::AgentRuntimeState],
    iterations: usize,
) {
    let report = time_cuda_route_breakdown(cuda, requests, states, iterations);
    println!(
        "cuda_timing_breakdown_columns,avg_total_ms,avg_runtime_init_ms,avg_runtime_teardown_ms,avg_host_prepare_ms,avg_device_allocate_ms,avg_host_to_device_ms,avg_module_stream_setup_ms,avg_kernel_launch_sync_ms,avg_kernel_device_ms,avg_kernel_host_overhead_ms,avg_device_to_host_ms,avg_decode_ms"
    );
    println!(
        "cuda_timing_breakdown,avg_total_ms={:.3},avg_runtime_init_ms={:.3},avg_runtime_teardown_ms={:.3},avg_host_prepare_ms={:.3},avg_device_allocate_ms={:.3},avg_host_to_device_ms={:.3},avg_module_stream_setup_ms={:.3},avg_kernel_launch_sync_ms={:.3},avg_kernel_device_ms={:.3},avg_kernel_host_overhead_ms={:.3},avg_device_to_host_ms={:.3},avg_decode_ms={:.3}",
        report.avg_total_ms,
        report.avg_runtime_init_ms,
        report.avg_runtime_teardown_ms,
        report.avg_host_prepare_ms,
        report.avg_device_allocate_ms,
        report.avg_host_to_device_ms,
        report.avg_module_stream_setup_ms,
        report.avg_kernel_launch_sync_ms,
        report.avg_kernel_device_ms,
        report.avg_kernel_host_overhead_ms,
        report.avg_device_to_host_ms,
        report.avg_decode_ms
    );
}

fn time_cuda_route_breakdown(
    cuda: &CudaRouter,
    requests: &[qtom_core::RoutingRequest],
    states: &[qtom_core::AgentRuntimeState],
    iterations: usize,
) -> CudaTimingBreakdownReport {
    cuda.route_batch_with_timing(requests, states)
        .expect("CUDA warmup should route fixture");
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let timed = cuda
            .route_batch_with_timing(requests, states)
            .expect("CUDA should route fixture");
        samples.push(timed.timing);
    }

    CudaTimingBreakdownReport::from_samples(&samples)
}

fn print_cuda_reuse_timing_breakdown(
    cuda_reuse: &CudaReuseBackend<'_>,
    requests: &[qtom_core::RoutingRequest],
    states: &[qtom_core::AgentRuntimeState],
    iterations: usize,
) {
    let report = time_cuda_reuse_route_breakdown(cuda_reuse, requests, states, iterations);
    println!(
        "cuda_reuse_timing_breakdown_columns,avg_total_ms,avg_runtime_init_ms,avg_runtime_teardown_ms,avg_host_prepare_ms,avg_device_allocate_ms,avg_host_to_device_ms,avg_module_stream_setup_ms,avg_kernel_launch_sync_ms,avg_kernel_device_ms,avg_kernel_host_overhead_ms,avg_device_to_host_ms,avg_decode_ms"
    );
    println!(
        "cuda_reuse_timing_breakdown,avg_total_ms={:.3},avg_runtime_init_ms={:.3},avg_runtime_teardown_ms={:.3},avg_host_prepare_ms={:.3},avg_device_allocate_ms={:.3},avg_host_to_device_ms={:.3},avg_module_stream_setup_ms={:.3},avg_kernel_launch_sync_ms={:.3},avg_kernel_device_ms={:.3},avg_kernel_host_overhead_ms={:.3},avg_device_to_host_ms={:.3},avg_decode_ms={:.3}",
        report.avg_total_ms,
        report.avg_runtime_init_ms,
        report.avg_runtime_teardown_ms,
        report.avg_host_prepare_ms,
        report.avg_device_allocate_ms,
        report.avg_host_to_device_ms,
        report.avg_module_stream_setup_ms,
        report.avg_kernel_launch_sync_ms,
        report.avg_kernel_device_ms,
        report.avg_kernel_host_overhead_ms,
        report.avg_device_to_host_ms,
        report.avg_decode_ms
    );
}

fn time_cuda_reuse_route_breakdown(
    cuda_reuse: &CudaReuseBackend<'_>,
    requests: &[qtom_core::RoutingRequest],
    states: &[qtom_core::AgentRuntimeState],
    iterations: usize,
) -> CudaTimingBreakdownReport {
    cuda_reuse
        .executor
        .route_batch_with_timing(&cuda_reuse.route_table, requests, states)
        .expect("CUDA reuse warmup should route fixture");
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let timed = cuda_reuse
            .executor
            .route_batch_with_timing(&cuda_reuse.route_table, requests, states)
            .expect("CUDA reuse should route fixture");
        samples.push(timed.timing);
    }

    CudaTimingBreakdownReport::from_samples(&samples)
}

struct CpuWorkerBackend {
    name: &'static str,
    router: CpuRouter,
    workers: usize,
}

struct CudaReuseBackend<'runtime> {
    name: &'static str,
    executor: CudaRouteK1Executor<'runtime>,
    route_table: AgentRouteTable,
}

impl RouterBackend for CudaReuseBackend<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn route_batch(
        &self,
        requests: &[qtom_core::RoutingRequest],
        states: &[qtom_core::AgentRuntimeState],
    ) -> Result<Vec<qtom_core::RoutingResult>, qtom_core::RouteError> {
        self.executor
            .route_batch_with_timing(&self.route_table, requests, states)
            .map(|timed| timed.results)
    }
}

impl CpuWorkerBackend {
    fn new(name: &'static str, agents: Vec<qtom_core::AgentProfile>, workers: usize) -> Self {
        Self {
            name,
            router: CpuRouter::new(agents, ScoreCoefficients::default()).with_debug_observed(false),
            workers,
        }
    }
}

impl RouterBackend for CpuWorkerBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn route_batch(
        &self,
        requests: &[qtom_core::RoutingRequest],
        states: &[qtom_core::AgentRuntimeState],
    ) -> Result<Vec<qtom_core::RoutingResult>, qtom_core::RouteError> {
        self.router
            .route_batch_with_workers(requests, states, self.workers)
    }
}

fn run_profile_matrix() {
    println!(
        "kernel,agents,tasks,dims,k,total_ms,routes_s,candidates_s,gdim_ops_s,p50_us,p95_us,p99_us,max_us,checksum"
    );

    for agent_count in PROFILE_AGENT_COUNTS {
        for dimensions in PROFILE_DIMENSIONS {
            let config = FixtureConfig {
                agent_count,
                task_count: profile_task_count_for(agent_count),
                dimensions,
                k: 8,
                seed: scenario_seed(agent_count, dimensions),
            };

            run_scan_profile(config);
            run_route_profile(config);
        }
    }
}

fn run_layout_profile_matrix() {
    println!(
        "kernel,agents,tasks,dims,k,total_ms,routes_s,candidates_s,gdim_ops_s,p50_us,p95_us,p99_us,max_us,checksum"
    );

    for agent_count in PROFILE_AGENT_COUNTS {
        for dimensions in PROFILE_DIMENSIONS {
            let config = FixtureConfig {
                agent_count,
                task_count: profile_task_count_for(agent_count),
                dimensions,
                k: 8,
                seed: scenario_seed(agent_count, dimensions),
            };

            run_aos_scan_profile("scan-aos", config, dist_sq);
            run_aos_scan_profile("scan-aos-blocked", config, dist_sq_blocked);
            run_packed_scan_profile("scan-packed", config, dist_sq);
            run_packed_scan_profile("scan-packed-blocked", config, dist_sq_blocked);
        }
    }
}

fn run_batch_profile_matrix() {
    println!(
        "kernel,workers,debug_observed,agents,tasks,dims,k,total_ms,routes_s,candidates_s,gdim_ops_s,ideal_unavailable,mean_delta,mean_radius,checksum"
    );

    for agent_count in BATCH_PROFILE_AGENT_COUNTS {
        for dimensions in PROFILE_DIMENSIONS {
            let config = FixtureConfig {
                agent_count,
                task_count: profile_task_count_for(agent_count),
                dimensions,
                k: 8,
                seed: scenario_seed(agent_count, dimensions),
            };

            for debug_observed in [true, false] {
                for workers in batch_worker_counts(config.task_count) {
                    run_batch_route_profile(config, workers, debug_observed);
                }
            }
        }
    }
}

fn run_prod_profile_matrix() {
    println!(
        "kernel,workers,agents,tasks,dims,k,total_ms,routes_s,candidates_s,gdim_ops_s,ideal_unavailable,checksum"
    );

    for agent_count in BATCH_PROFILE_AGENT_COUNTS {
        for dimensions in PROFILE_DIMENSIONS {
            for k in PROD_TOP_K_VALUES {
                let config = FixtureConfig {
                    agent_count,
                    task_count: profile_task_count_for(agent_count),
                    dimensions,
                    k,
                    seed: scenario_seed(agent_count, dimensions),
                };

                for workers in batch_worker_counts(config.task_count) {
                    run_prod_route_profile(config, workers);
                }
            }
        }
    }
}

fn run_candidate_prefilter_profile() {
    println!(
        "candidate_prefilter_columns,strategy,layers,agents,tasks,dims,bins,budget,avg_candidates,scan_reduction,top1_recall,ideal_unavailable_match,prefilter_ms,exact_subset_ms,total_ms"
    );

    let strategies = [
        PrefilterStrategy {
            name: "2d-single",
            projections: &[&PREFILTER_2D_A],
        },
        PrefilterStrategy {
            name: "3d-single",
            projections: &[&PREFILTER_3D_A],
        },
        PrefilterStrategy {
            name: "2d-stacked",
            projections: &[
                &PREFILTER_2D_A,
                &PREFILTER_2D_B,
                &PREFILTER_2D_C,
                &PREFILTER_2D_D,
            ],
        },
        PrefilterStrategy {
            name: "3d-stacked",
            projections: &[&PREFILTER_3D_A, &PREFILTER_3D_B, &PREFILTER_3D_C],
        },
    ];

    for agent_count in PREFILTER_AGENT_COUNTS {
        let config = FixtureConfig {
            agent_count,
            task_count: profile_task_count_for(agent_count),
            dimensions: 16,
            k: 1,
            seed: scenario_seed(agent_count, 1),
        };
        let fixture = generate_fixture(config);
        let exact_router = CpuRouter::new(fixture.agents.clone(), ScoreCoefficients::default())
            .with_debug_observed(false);
        let exact_results = exact_router
            .route_batch_with_workers(
                &fixture.requests,
                &fixture.states,
                std::thread::available_parallelism()
                    .map(usize::from)
                    .unwrap_or(1),
            )
            .expect("exact CPU route should succeed");
        let expected_agent_ids = exact_results
            .iter()
            .map(first_candidate_id)
            .collect::<Vec<_>>();
        let expected_ideal_flags = exact_results
            .iter()
            .map(|result| result.ideal_candidate_unavailable)
            .collect::<Vec<_>>();

        for strategy in strategies {
            let index_start = Instant::now();
            let index = LayeredGridIndex::new(&fixture.agents, PREFILTER_GRID_BINS, strategy);
            let index_ms = index_start.elapsed().as_secs_f64() * 1000.0;

            for budget in PREFILTER_CANDIDATE_BUDGETS {
                let report = profile_prefilter_budget(
                    &fixture,
                    &index,
                    &expected_agent_ids,
                    &expected_ideal_flags,
                    budget,
                );
                println!(
                    "candidate_prefilter,strategy={},layers={},agents={},tasks={},dims={},bins={},budget={},avg_candidates={:.1},scan_reduction={:.3},top1_recall={:.4},ideal_unavailable_match={:.4},prefilter_ms={:.3},exact_subset_ms={:.3},total_ms={:.3}",
                    strategy.name,
                    strategy.projections.len(),
                    config.agent_count,
                    config.task_count,
                    config.dimensions,
                    PREFILTER_GRID_BINS,
                    budget,
                    report.avg_candidates,
                    report.scan_reduction,
                    report.top1_recall,
                    report.ideal_unavailable_match,
                    index_ms + report.prefilter_ms,
                    report.exact_subset_ms,
                    index_ms + report.prefilter_ms + report.exact_subset_ms
                );
            }
        }
    }
}

fn first_candidate_id(result: &qtom_core::RoutingResult) -> u32 {
    result
        .available_candidates
        .first()
        .map(|candidate| candidate.agent_id)
        .unwrap_or(u32::MAX)
}

#[derive(Clone, Copy, Debug)]
struct PrefilterStrategy {
    name: &'static str,
    projections: &'static [&'static [usize]],
}

#[derive(Clone, Debug)]
struct LayeredGridIndex {
    layers: Vec<ProjectionGridIndex>,
    agent_count: usize,
}

impl LayeredGridIndex {
    fn new(agents: &[AgentProfile], bins: usize, strategy: PrefilterStrategy) -> Self {
        let layers = strategy
            .projections
            .iter()
            .map(|projection| ProjectionGridIndex::new(agents, bins, projection))
            .collect::<Vec<_>>();

        Self {
            layers,
            agent_count: agents.len(),
        }
    }

    fn candidate_indices(&self, request: &RoutingRequest, budget: usize, out: &mut Vec<usize>) {
        out.clear();
        if budget == 0 || self.agent_count == 0 {
            return;
        }

        let mut seen = vec![false; self.agent_count];
        let mut layer_candidates = Vec::new();
        for layer in &self.layers {
            layer.candidate_indices(request, budget, &mut layer_candidates);
            for &agent_index in &layer_candidates {
                if !seen[agent_index] {
                    seen[agent_index] = true;
                    out.push(agent_index);
                }
            }
        }

        if out.len() > budget {
            let compare = |left: &usize, right: &usize| {
                let left_dist = self.projection_distance(request, *left);
                let right_dist = self.projection_distance(request, *right);
                left_dist
                    .total_cmp(&right_dist)
                    .then_with(|| left.cmp(right))
            };
            out.select_nth_unstable_by(budget, compare);
            out.truncate(budget);
            out.sort_by(compare);
        }
    }

    fn projection_distance(&self, request: &RoutingRequest, agent_index: usize) -> f32 {
        self.layers
            .iter()
            .map(|layer| layer.projection_distance(request, agent_index))
            .sum()
    }
}

#[derive(Clone, Debug)]
struct ProjectionGridIndex {
    bins: usize,
    dims: Vec<usize>,
    cells: Vec<Vec<usize>>,
    projected_vectors: Vec<Vec<f32>>,
}

impl ProjectionGridIndex {
    fn new(agents: &[AgentProfile], bins: usize, dims: &[usize]) -> Self {
        assert!(bins > 0, "coarse grid must have at least one bin");
        assert!(
            !dims.is_empty(),
            "projection must include at least one dimension"
        );
        let cell_count = (0..dims.len()).fold(1usize, |acc, _| {
            acc.checked_mul(bins)
                .expect("projection grid cell count should fit usize")
        });
        let mut cells = vec![Vec::new(); cell_count];
        let mut projected_vectors = Vec::with_capacity(agents.len());
        for (agent_index, agent) in agents.iter().enumerate() {
            let projection = Self::project_vector(&agent.vector, dims);
            let cell = Self::cell_for_projection(&projection, bins);
            projected_vectors.push(projection);
            cells[cell].push(agent_index);
        }

        Self {
            bins,
            dims: dims.to_vec(),
            cells,
            projected_vectors,
        }
    }

    fn candidate_indices(&self, request: &RoutingRequest, budget: usize, out: &mut Vec<usize>) {
        out.clear();
        let request_projection = Self::project_vector(&request.vector, &self.dims);
        let center = request_projection
            .iter()
            .map(|value| Self::bin_for_value(*value, self.bins))
            .collect::<Vec<_>>();
        let mut seen_cells = vec![false; self.cells.len()];

        for radius in 0..self.bins {
            self.collect_radius_cells(&center, radius, &mut seen_cells, out);

            if out.len() >= budget {
                break;
            }
        }

        if out.len() > budget {
            let compare = |left: &usize, right: &usize| {
                let left_dist =
                    dist_sq_dynamic(&request_projection, &self.projected_vectors[*left]);
                let right_dist =
                    dist_sq_dynamic(&request_projection, &self.projected_vectors[*right]);
                left_dist
                    .total_cmp(&right_dist)
                    .then_with(|| left.cmp(right))
            };
            out.select_nth_unstable_by(budget, compare);
            out.truncate(budget);
            out.sort_by(compare);
        }
    }

    fn projection_distance(&self, request: &RoutingRequest, agent_index: usize) -> f32 {
        let request_projection = Self::project_vector(&request.vector, &self.dims);
        dist_sq_dynamic(&request_projection, &self.projected_vectors[agent_index])
    }

    fn collect_radius_cells(
        &self,
        center: &[usize],
        radius: usize,
        seen_cells: &mut [bool],
        out: &mut Vec<usize>,
    ) {
        let mut mins = Vec::with_capacity(center.len());
        let mut maxes = Vec::with_capacity(center.len());
        for &value in center {
            mins.push(value.saturating_sub(radius));
            maxes.push(value.saturating_add(radius).min(self.bins - 1));
        }

        let mut coords = mins.clone();
        loop {
            let on_border = radius == 0
                || coords
                    .iter()
                    .enumerate()
                    .any(|(dim, coord)| *coord == mins[dim] || *coord == maxes[dim]);
            if on_border {
                let cell_index = self.cell_index(&coords);
                if !seen_cells[cell_index] {
                    seen_cells[cell_index] = true;
                    out.extend_from_slice(&self.cells[cell_index]);
                }
            }

            if !increment_coords(&mut coords, &mins, &maxes) {
                break;
            }
        }
    }

    fn project_vector(vector: &[f32], dims: &[usize]) -> Vec<f32> {
        dims.iter()
            .map(|dim| vector.get(*dim).copied().unwrap_or(0.0))
            .collect()
    }

    fn cell_for_projection(projection: &[f32], bins: usize) -> usize {
        let mut index = 0usize;
        let mut stride = 1usize;
        for value in projection {
            index += Self::bin_for_value(*value, bins) * stride;
            stride *= bins;
        }
        index
    }

    fn cell_index(&self, coords: &[usize]) -> usize {
        let mut index = 0usize;
        let mut stride = 1usize;
        for coord in coords {
            index += *coord * stride;
            stride *= self.bins;
        }
        index
    }

    fn bin_for_value(value: f32, bins: usize) -> usize {
        let scaled = (value.clamp(0.0, 1.0) * bins as f32).floor() as usize;
        scaled.min(bins - 1)
    }
}

fn increment_coords(coords: &mut [usize], mins: &[usize], maxes: &[usize]) -> bool {
    for dim in 0..coords.len() {
        if coords[dim] < maxes[dim] {
            coords[dim] += 1;
            coords[..dim].copy_from_slice(&mins[..dim]);
            return true;
        }
    }
    false
}

fn dist_sq_dynamic(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            let diff = left - right;
            diff * diff
        })
        .sum()
}

#[derive(Clone, Copy, Debug)]
struct PrefilterReport {
    avg_candidates: f64,
    scan_reduction: f64,
    top1_recall: f64,
    ideal_unavailable_match: f64,
    prefilter_ms: f64,
    exact_subset_ms: f64,
}

fn profile_prefilter_budget(
    fixture: &Fixture,
    index: &LayeredGridIndex,
    expected_agent_ids: &[u32],
    expected_ideal_flags: &[bool],
    budget: usize,
) -> PrefilterReport {
    let coefficients = ScoreCoefficients::default();
    let mut candidates = Vec::new();
    let mut total_candidates = 0usize;
    let mut top1_matches = 0usize;
    let mut ideal_matches = 0usize;
    let mut prefilter_elapsed = Duration::ZERO;
    let mut exact_subset_elapsed = Duration::ZERO;

    for (task_index, request) in fixture.requests.iter().enumerate() {
        let prefilter_start = Instant::now();
        index.candidate_indices(request, budget, &mut candidates);
        prefilter_elapsed += prefilter_start.elapsed();
        total_candidates += candidates.len();

        let exact_start = Instant::now();
        let mut best_available: Option<PrefilterCandidate> = None;
        let mut observed_best: Option<PrefilterCandidate> = None;
        for &agent_index in &candidates {
            let agent = &fixture.agents[agent_index];
            let score = score_components_for_vector(
                &request.vector,
                &agent.vector,
                fixture.states[agent_index],
                coefficients,
            );
            let candidate = PrefilterCandidate {
                agent_id: agent.id,
                effective_distance: score.effective_distance,
                base_distance: score.base_distance,
                available: score.available,
            };

            if candidate.available
                && best_available
                    .as_ref()
                    .is_none_or(|best| candidate.cmp_available(best).is_lt())
            {
                best_available = Some(candidate);
            }
            if observed_best
                .as_ref()
                .is_none_or(|best| candidate.cmp_observed(best).is_lt())
            {
                observed_best = Some(candidate);
            }
        }
        exact_subset_elapsed += exact_start.elapsed();

        if best_available
            .map(|candidate| candidate.agent_id)
            .is_some_and(|agent_id| agent_id == expected_agent_ids[task_index])
        {
            top1_matches += 1;
        }
        if observed_best
            .map(|candidate| !candidate.available)
            .is_some_and(|ideal_unavailable| ideal_unavailable == expected_ideal_flags[task_index])
        {
            ideal_matches += 1;
        }
    }

    let route_count = fixture.requests.len().max(1);
    let avg_candidates = total_candidates as f64 / route_count as f64;
    PrefilterReport {
        avg_candidates,
        scan_reduction: 1.0 - (avg_candidates / fixture.agents.len().max(1) as f64),
        top1_recall: top1_matches as f64 / route_count as f64,
        ideal_unavailable_match: ideal_matches as f64 / route_count as f64,
        prefilter_ms: prefilter_elapsed.as_secs_f64() * 1000.0,
        exact_subset_ms: exact_subset_elapsed.as_secs_f64() * 1000.0,
    }
}

#[derive(Clone, Copy, Debug)]
struct PrefilterCandidate {
    agent_id: u32,
    effective_distance: f32,
    base_distance: f32,
    available: bool,
}

impl PrefilterCandidate {
    fn cmp_available(&self, other: &Self) -> std::cmp::Ordering {
        self.effective_distance
            .total_cmp(&other.effective_distance)
            .then_with(|| self.agent_id.cmp(&other.agent_id))
    }

    fn cmp_observed(&self, other: &Self) -> std::cmp::Ordering {
        self.base_distance
            .total_cmp(&other.base_distance)
            .then_with(|| self.agent_id.cmp(&other.agent_id))
    }
}

fn run_scenario(config: FixtureConfig) {
    let fixture = generate_fixture(config);
    let router = CpuRouter::new(fixture.agents, ScoreCoefficients::default());
    let mut latencies = Vec::with_capacity(fixture.requests.len());
    let start = Instant::now();
    let results = fixture
        .requests
        .iter()
        .map(|request| {
            let route_start = Instant::now();
            let result = router
                .route_one(request, &fixture.states)
                .expect("fixture should be valid");
            latencies.push(route_start.elapsed());
            result
        })
        .collect::<Vec<_>>();
    let total_elapsed = start.elapsed();
    let metrics = batch_metrics(&results);
    let latency = LatencySummary::from_durations(&latencies);

    print_report(config, total_elapsed, latency, metrics);
}

fn run_scan_profile(config: FixtureConfig) {
    run_aos_scan_profile("scan", config, dist_sq);
}

fn run_aos_scan_profile<D>(kernel: &str, config: FixtureConfig, distance: D)
where
    D: Fn(&[f32], &[f32]) -> f32 + Copy,
{
    let fixture = generate_fixture(config);
    let mut latencies = Vec::with_capacity(fixture.requests.len());
    let start = Instant::now();
    let mut checksum = 0.0f64;

    for request in &fixture.requests {
        let route_start = Instant::now();
        let mut best_agent = 0u32;
        let mut best_distance = f32::INFINITY;

        for agent in &fixture.agents {
            let distance = distance(&request.vector, &agent.vector);
            if distance < best_distance {
                best_distance = distance;
                best_agent = agent.id;
            }
        }

        checksum += best_distance as f64 + best_agent as f64;
        latencies.push(route_start.elapsed());
    }

    print_profile_report(kernel, config, start.elapsed(), latencies, checksum);
}

fn run_packed_scan_profile<D>(kernel: &str, config: FixtureConfig, distance: D)
where
    D: Fn(&[f32], &[f32]) -> f32 + Copy,
{
    let fixture = generate_fixture(config);
    let route_table =
        AgentRouteTable::from_agent_slice(&fixture.agents).expect("fixture should be valid");
    let mut latencies = Vec::with_capacity(fixture.requests.len());
    let start = Instant::now();
    let mut checksum = 0.0f64;

    for request in &fixture.requests {
        let route_start = Instant::now();
        let mut best_agent = 0u32;
        let mut best_distance = f32::INFINITY;

        if route_table.dimensions() == 0 {
            for index in 0..route_table.len() {
                let distance = distance(&request.vector, route_table.vector(index));
                if distance < best_distance {
                    best_distance = distance;
                    best_agent = route_table.agent_id(index);
                }
            }
        } else {
            for (agent_id, agent_vector) in route_table.agent_ids().iter().copied().zip(
                route_table
                    .packed_vectors()
                    .chunks_exact(route_table.dimensions()),
            ) {
                let distance = distance(&request.vector, agent_vector);
                if distance < best_distance {
                    best_distance = distance;
                    best_agent = agent_id;
                }
            }
        }

        checksum += best_distance as f64 + best_agent as f64;
        latencies.push(route_start.elapsed());
    }

    print_profile_report(kernel, config, start.elapsed(), latencies, checksum);
}

fn run_route_profile(config: FixtureConfig) {
    let fixture = generate_fixture(config);
    let router = CpuRouter::new(fixture.agents, ScoreCoefficients::default());
    let mut latencies = Vec::with_capacity(fixture.requests.len());
    let start = Instant::now();
    let mut checksum = 0.0f64;

    for request in &fixture.requests {
        let route_start = Instant::now();
        let result = router
            .route_one(request, &fixture.states)
            .expect("fixture should be valid");
        let selected = result
            .available_candidates
            .first()
            .expect("fixture should always have available candidates");
        checksum += selected.base_distance as f64 + selected.agent_id as f64;
        latencies.push(route_start.elapsed());
    }

    print_profile_report("route", config, start.elapsed(), latencies, checksum);
}

fn run_batch_route_profile(config: FixtureConfig, workers: usize, debug_observed: bool) {
    let fixture = generate_fixture(config);
    let router = CpuRouter::new(fixture.agents, ScoreCoefficients::default())
        .with_debug_observed(debug_observed);
    let actual_workers = workers.max(1).min(config.task_count.max(1));

    let start = Instant::now();
    let results = router
        .route_batch_with_workers(&fixture.requests, &fixture.states, actual_workers)
        .expect("fixture should be valid");
    let total_elapsed = start.elapsed();

    let metrics = batch_metrics(&results);
    let checksum = checksum_results(&results);
    print_batch_profile_report(
        "route-batch",
        config,
        actual_workers,
        debug_observed,
        total_elapsed,
        metrics,
        checksum,
    );
}

fn print_report(
    config: FixtureConfig,
    total_elapsed: Duration,
    latency: LatencySummary,
    metrics: BatchMetrics,
) {
    let elapsed_secs = total_elapsed.as_secs_f64();
    let routes_per_second = metrics.routes as f64 / elapsed_secs.max(f64::EPSILON);
    println!(
        "{},{},{},{},{:.3},{:.1},{:.3},{:.3},{:.3},{:.3},{},{:.6},{:.6}",
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        elapsed_secs * 1000.0,
        routes_per_second,
        latency.p50_us,
        latency.p95_us,
        latency.p99_us,
        latency.max_us,
        metrics.ideal_unavailable_count,
        metrics.mean_substitute_distance_delta,
        metrics.mean_top_k_radius
    );
}

fn print_profile_report(
    kernel: &str,
    config: FixtureConfig,
    total_elapsed: Duration,
    latencies: Vec<Duration>,
    checksum: f64,
) {
    let elapsed_secs = total_elapsed.as_secs_f64();
    let routes_per_second = config.task_count as f64 / elapsed_secs.max(f64::EPSILON);
    let candidates_per_second =
        (config.task_count * config.agent_count) as f64 / elapsed_secs.max(f64::EPSILON);
    let gdim_ops_per_second = candidates_per_second * config.dimensions as f64 / 1_000_000_000.0;
    let latency = LatencySummary::from_durations(&latencies);

    println!(
        "{},{},{},{},{},{:.3},{:.1},{:.1},{:.3},{:.3},{:.3},{:.3},{:.3},{:.6}",
        kernel,
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        elapsed_secs * 1000.0,
        routes_per_second,
        candidates_per_second,
        gdim_ops_per_second,
        latency.p50_us,
        latency.p95_us,
        latency.p99_us,
        latency.max_us,
        checksum
    );
}

fn run_prod_route_profile(config: FixtureConfig, workers: usize) {
    let fixture = generate_fixture(config);
    let router =
        CpuRouter::new(fixture.agents, ScoreCoefficients::default()).with_debug_observed(false);
    let actual_workers = workers.max(1).min(config.task_count.max(1));

    let start = Instant::now();
    let results = router
        .route_batch_with_workers(&fixture.requests, &fixture.states, actual_workers)
        .expect("fixture should be valid");
    let total_elapsed = start.elapsed();

    let metrics = batch_metrics(&results);
    let checksum = checksum_results(&results);
    print_prod_profile_report(
        "route-prod",
        config,
        actual_workers,
        total_elapsed,
        metrics,
        checksum,
    );
}

fn print_batch_profile_report(
    kernel: &str,
    config: FixtureConfig,
    workers: usize,
    debug_observed: bool,
    total_elapsed: Duration,
    metrics: BatchMetrics,
    checksum: f64,
) {
    let elapsed_secs = total_elapsed.as_secs_f64();
    let routes_per_second = metrics.routes as f64 / elapsed_secs.max(f64::EPSILON);
    let candidates_per_second =
        (config.task_count * config.agent_count) as f64 / elapsed_secs.max(f64::EPSILON);
    let gdim_ops_per_second = candidates_per_second * config.dimensions as f64 / 1_000_000_000.0;

    println!(
        "{},{},{},{},{},{},{},{:.3},{:.1},{:.1},{:.3},{},{:.6},{:.6},{:.6}",
        kernel,
        workers,
        debug_observed,
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        elapsed_secs * 1000.0,
        routes_per_second,
        candidates_per_second,
        gdim_ops_per_second,
        metrics.ideal_unavailable_count,
        metrics.mean_substitute_distance_delta,
        metrics.mean_top_k_radius,
        checksum
    );
}

fn print_prod_profile_report(
    kernel: &str,
    config: FixtureConfig,
    workers: usize,
    total_elapsed: Duration,
    metrics: BatchMetrics,
    checksum: f64,
) {
    let elapsed_secs = total_elapsed.as_secs_f64();
    let routes_per_second = metrics.routes as f64 / elapsed_secs.max(f64::EPSILON);
    let candidates_per_second =
        (config.task_count * config.agent_count) as f64 / elapsed_secs.max(f64::EPSILON);
    let gdim_ops_per_second = candidates_per_second * config.dimensions as f64 / 1_000_000_000.0;

    println!(
        "{},{},{},{},{},{},{:.3},{:.1},{:.1},{:.3},{},{:.6}",
        kernel,
        workers,
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        elapsed_secs * 1000.0,
        routes_per_second,
        candidates_per_second,
        gdim_ops_per_second,
        metrics.ideal_unavailable_count,
        checksum
    );
}

fn task_count_for(agent_count: usize) -> usize {
    match agent_count {
        128 => 128,
        1024 => 512,
        65536 => 512,
        _ => 2048,
    }
}

fn profile_task_count_for(agent_count: usize) -> usize {
    match agent_count {
        8192 => 512,
        65536 => 256,
        131072 => 128,
        _ => 64,
    }
}

fn batch_worker_counts(task_count: usize) -> Vec<usize> {
    let available = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let mut counts = vec![1, 2, available / 2, available];

    counts
        .iter_mut()
        .for_each(|count| *count = (*count).max(1).min(task_count.max(1)));
    counts.sort_unstable();
    counts.dedup();
    counts
}

fn scenario_seed(agent_count: usize, k: usize) -> u64 {
    0x5154_4f4d ^ ((agent_count as u64) << 8) ^ k as u64
}

fn golden_fixture_config() -> FixtureConfig {
    FixtureConfig {
        agent_count: 8192,
        task_count: 2048,
        dimensions: 16,
        k: 8,
        seed: scenario_seed(8192, 8),
    }
}

fn cuda_golden_fixture_config() -> FixtureConfig {
    FixtureConfig {
        agent_count: 8192,
        task_count: 2048,
        dimensions: 16,
        k: 1,
        seed: scenario_seed(8192, 1),
    }
}

fn checksum_results(results: &[qtom_core::RoutingResult]) -> f64 {
    routing_results_checksum(results)
}

#[derive(Clone, Copy, Debug, Default)]
struct CudaTimingBreakdownReport {
    avg_total_ms: f64,
    avg_runtime_init_ms: f64,
    avg_runtime_teardown_ms: f64,
    avg_host_prepare_ms: f64,
    avg_device_allocate_ms: f64,
    avg_host_to_device_ms: f64,
    avg_module_stream_setup_ms: f64,
    avg_kernel_launch_sync_ms: f64,
    avg_kernel_device_ms: f64,
    avg_kernel_host_overhead_ms: f64,
    avg_device_to_host_ms: f64,
    avg_decode_ms: f64,
}

impl CudaTimingBreakdownReport {
    fn from_samples(samples: &[qtom_cuda::CudaRouteTimingBreakdown]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let count = samples.len() as f64;
        Self {
            avg_total_ms: avg_duration_ms(samples.iter().map(|sample| sample.total), count),
            avg_runtime_init_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.runtime_init),
                count,
            ),
            avg_runtime_teardown_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.runtime_teardown),
                count,
            ),
            avg_host_prepare_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.host_prepare),
                count,
            ),
            avg_device_allocate_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.device_allocate),
                count,
            ),
            avg_host_to_device_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.host_to_device),
                count,
            ),
            avg_module_stream_setup_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.module_stream_setup),
                count,
            ),
            avg_kernel_launch_sync_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.kernel_launch_sync),
                count,
            ),
            avg_kernel_device_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.kernel_device),
                count,
            ),
            avg_kernel_host_overhead_ms: samples
                .iter()
                .map(|sample| {
                    (sample.kernel_launch_sync.as_secs_f64() - sample.kernel_device.as_secs_f64())
                        .max(0.0)
                        * 1000.0
                })
                .sum::<f64>()
                / count,
            avg_device_to_host_ms: avg_duration_ms(
                samples.iter().map(|sample| sample.device_to_host),
                count,
            ),
            avg_decode_ms: avg_duration_ms(samples.iter().map(|sample| sample.decode), count),
        }
    }
}

fn avg_duration_ms(durations: impl Iterator<Item = Duration>, count: f64) -> f64 {
    durations
        .map(|duration| duration.as_secs_f64() * 1000.0)
        .sum::<f64>()
        / count
}

#[derive(Clone, Copy, Debug, Default)]
struct BackendTimingReport {
    avg_batch_ms: f64,
    p50_batch_ms: f64,
    p95_batch_ms: f64,
    p99_batch_ms: f64,
    min_batch_ms: f64,
    max_batch_ms: f64,
    routes_per_second: f64,
    ideal_unavailable_count: usize,
    checksum: f64,
}

impl BackendTimingReport {
    fn from_durations(
        durations: Vec<Duration>,
        route_count: usize,
        ideal_unavailable_count: usize,
        checksum: f64,
    ) -> Self {
        if durations.is_empty() {
            return Self {
                ideal_unavailable_count,
                checksum,
                ..Self::default()
            };
        }

        let mut batch_ms = durations
            .iter()
            .map(|duration| duration.as_secs_f64() * 1000.0)
            .collect::<Vec<_>>();
        batch_ms.sort_by(f64::total_cmp);
        let total_ms = batch_ms.iter().sum::<f64>();
        let avg_batch_ms = total_ms / batch_ms.len() as f64;
        let avg_seconds = (avg_batch_ms / 1000.0).max(f64::EPSILON);

        Self {
            avg_batch_ms,
            p50_batch_ms: percentile(&batch_ms, 0.50),
            p95_batch_ms: percentile(&batch_ms, 0.95),
            p99_batch_ms: percentile(&batch_ms, 0.99),
            min_batch_ms: *batch_ms.first().unwrap(),
            max_batch_ms: *batch_ms.last().unwrap(),
            routes_per_second: route_count as f64 / avg_seconds,
            ideal_unavailable_count,
            checksum,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct LatencySummary {
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
    max_us: f64,
}

impl LatencySummary {
    fn from_durations(durations: &[Duration]) -> Self {
        if durations.is_empty() {
            return Self::default();
        }

        let mut micros = durations
            .iter()
            .map(|duration| duration.as_secs_f64() * 1_000_000.0)
            .collect::<Vec<_>>();
        micros.sort_by(f64::total_cmp);

        Self {
            p50_us: percentile(&micros, 0.50),
            p95_us: percentile(&micros, 0.95),
            p99_us: percentile(&micros, 0.99),
            max_us: *micros.last().unwrap(),
        }
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    debug_assert!(!sorted.is_empty());

    if sorted.len() == 1 {
        return sorted[0];
    }

    let rank = percentile.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;

    if lower == upper {
        sorted[lower]
    } else {
        let weight = rank - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BenchMode {
    Smoke,
    Stress,
    Profile,
    BatchProfile,
    ProdProfile,
    LayoutProfile,
    CandidatePrefilterProfile,
    WriteGolden { path: std::path::PathBuf },
    WriteCudaGolden { path: std::path::PathBuf },
    GoldenParity { path: std::path::PathBuf },
    CudaParity { path: std::path::PathBuf },
    CudaTiming { path: std::path::PathBuf },
    CudaScale,
    CudaPlan { path: std::path::PathBuf },
    Invalid(String),
}

impl BenchMode {
    fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut mode = Self::Smoke;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--stress" => mode = Self::Stress,
                "--profile" => mode = Self::Profile,
                "--batch-profile" => mode = Self::BatchProfile,
                "--prod-profile" => mode = Self::ProdProfile,
                "--layout-profile" => mode = Self::LayoutProfile,
                "--candidate-prefilter-profile" => mode = Self::CandidatePrefilterProfile,
                "--write-golden" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--write-golden requires a path".to_string());
                    };
                    mode = Self::WriteGolden { path: path.into() };
                }
                "--write-cuda-golden" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--write-cuda-golden requires a path".to_string());
                    };
                    mode = Self::WriteCudaGolden { path: path.into() };
                }
                "--golden-parity" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--golden-parity requires a path".to_string());
                    };
                    mode = Self::GoldenParity { path: path.into() };
                }
                "--cuda-parity" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--cuda-parity requires a path".to_string());
                    };
                    mode = Self::CudaParity { path: path.into() };
                }
                "--cuda-timing" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--cuda-timing requires a path".to_string());
                    };
                    mode = Self::CudaTiming { path: path.into() };
                }
                "--cuda-scale" => mode = Self::CudaScale,
                "--cuda-plan" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--cuda-plan requires a path".to_string());
                    };
                    mode = Self::CudaPlan { path: path.into() };
                }
                _ => {}
            }
        }
        mode
    }

    fn agent_counts(&self) -> &'static [usize] {
        match self {
            Self::Smoke => &AGENT_COUNTS,
            Self::Stress => &STRESS_AGENT_COUNTS,
            Self::Profile => &PROFILE_AGENT_COUNTS,
            Self::BatchProfile => &BATCH_PROFILE_AGENT_COUNTS,
            Self::ProdProfile => &BATCH_PROFILE_AGENT_COUNTS,
            Self::LayoutProfile => &PROFILE_AGENT_COUNTS,
            Self::CandidatePrefilterProfile => &[],
            Self::WriteGolden { .. }
            | Self::WriteCudaGolden { .. }
            | Self::GoldenParity { .. }
            | Self::CudaParity { .. }
            | Self::CudaTiming { .. }
            | Self::CudaScale
            | Self::CudaPlan { .. }
            | Self::Invalid(_) => &[],
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Stress => "stress",
            Self::Profile => "profile",
            Self::BatchProfile => "batch-profile",
            Self::ProdProfile => "prod-profile",
            Self::LayoutProfile => "layout-profile",
            Self::CandidatePrefilterProfile => "candidate-prefilter-profile",
            Self::WriteGolden { .. } => "write-golden",
            Self::WriteCudaGolden { .. } => "write-cuda-golden",
            Self::GoldenParity { .. } => "golden-parity",
            Self::CudaParity { .. } => "cuda-parity",
            Self::CudaTiming { .. } => "cuda-timing",
            Self::CudaScale => "cuda-scale",
            Self::CudaPlan { .. } => "cuda-plan",
            Self::Invalid(_) => "invalid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_interpolates_between_samples() {
        let samples = [1.0, 2.0, 3.0, 4.0];

        assert_eq!(percentile(&samples, 0.0), 1.0);
        assert_eq!(percentile(&samples, 1.0), 4.0);
        assert_eq!(percentile(&samples, 0.5), 2.5);
    }

    #[test]
    fn stress_flag_selects_stress_mode() {
        assert_eq!(
            BenchMode::from_args(["--stress".to_string()]),
            BenchMode::Stress
        );
        assert_eq!(BenchMode::from_args(Vec::<String>::new()), BenchMode::Smoke);
    }

    #[test]
    fn profile_flag_selects_profile_mode() {
        assert_eq!(
            BenchMode::from_args(["--profile".to_string()]),
            BenchMode::Profile
        );
    }

    #[test]
    fn batch_profile_flag_selects_batch_profile_mode() {
        assert_eq!(
            BenchMode::from_args(["--batch-profile".to_string()]),
            BenchMode::BatchProfile
        );
    }

    #[test]
    fn prod_profile_flag_selects_prod_profile_mode() {
        assert_eq!(
            BenchMode::from_args(["--prod-profile".to_string()]),
            BenchMode::ProdProfile
        );
    }

    #[test]
    fn layout_profile_flag_selects_layout_profile_mode() {
        assert_eq!(
            BenchMode::from_args(["--layout-profile".to_string()]),
            BenchMode::LayoutProfile
        );
    }

    #[test]
    fn candidate_prefilter_profile_flag_selects_mode() {
        assert_eq!(
            BenchMode::from_args(["--candidate-prefilter-profile".to_string()]),
            BenchMode::CandidatePrefilterProfile
        );
    }

    #[test]
    fn write_golden_flag_selects_path_mode() {
        assert_eq!(
            BenchMode::from_args([
                "--write-golden".to_string(),
                "work/golden/default.fixture".to_string()
            ]),
            BenchMode::WriteGolden {
                path: "work/golden/default.fixture".into()
            }
        );
    }

    #[test]
    fn write_cuda_golden_flag_selects_path_mode() {
        assert_eq!(
            BenchMode::from_args([
                "--write-cuda-golden".to_string(),
                "work/golden/cuda-k1.fixture".to_string()
            ]),
            BenchMode::WriteCudaGolden {
                path: "work/golden/cuda-k1.fixture".into()
            }
        );
    }

    #[test]
    fn golden_parity_flag_selects_path_mode() {
        assert_eq!(
            BenchMode::from_args([
                "--golden-parity".to_string(),
                "work/golden/default.fixture".to_string()
            ]),
            BenchMode::GoldenParity {
                path: "work/golden/default.fixture".into()
            }
        );
    }

    #[test]
    fn cuda_parity_flag_selects_path_mode() {
        assert_eq!(
            BenchMode::from_args([
                "--cuda-parity".to_string(),
                "work/golden/cuda-k1.fixture".to_string()
            ]),
            BenchMode::CudaParity {
                path: "work/golden/cuda-k1.fixture".into()
            }
        );
    }

    #[test]
    fn cuda_timing_flag_selects_path_mode() {
        assert_eq!(
            BenchMode::from_args([
                "--cuda-timing".to_string(),
                "work/golden/cuda-k1.fixture".to_string()
            ]),
            BenchMode::CudaTiming {
                path: "work/golden/cuda-k1.fixture".into()
            }
        );
    }

    #[test]
    fn cuda_scale_flag_selects_mode() {
        assert_eq!(
            BenchMode::from_args(["--cuda-scale".to_string()]),
            BenchMode::CudaScale
        );
    }

    #[test]
    fn golden_flags_require_paths() {
        assert_eq!(
            BenchMode::from_args(["--write-golden".to_string()]),
            BenchMode::Invalid("--write-golden requires a path".to_string())
        );
        assert_eq!(
            BenchMode::from_args(["--golden-parity".to_string()]),
            BenchMode::Invalid("--golden-parity requires a path".to_string())
        );
        assert_eq!(
            BenchMode::from_args(["--write-cuda-golden".to_string()]),
            BenchMode::Invalid("--write-cuda-golden requires a path".to_string())
        );
        assert_eq!(
            BenchMode::from_args(["--cuda-parity".to_string()]),
            BenchMode::Invalid("--cuda-parity requires a path".to_string())
        );
        assert_eq!(
            BenchMode::from_args(["--cuda-timing".to_string()]),
            BenchMode::Invalid("--cuda-timing requires a path".to_string())
        );
    }

    #[test]
    fn cuda_plan_flag_selects_path_mode() {
        assert_eq!(
            BenchMode::from_args([
                "--cuda-plan".to_string(),
                "work/golden/default.fixture".to_string()
            ]),
            BenchMode::CudaPlan {
                path: "work/golden/default.fixture".into()
            }
        );
        assert_eq!(
            BenchMode::from_args(["--cuda-plan".to_string()]),
            BenchMode::Invalid("--cuda-plan requires a path".to_string())
        );
    }

    #[test]
    fn batch_worker_counts_are_unique_and_bounded() {
        let counts = batch_worker_counts(3);

        assert_eq!(counts[0], 1);
        assert!(counts.windows(2).all(|window| window[0] < window[1]));
        assert!(counts.iter().all(|count| *count <= 3));
    }
}
