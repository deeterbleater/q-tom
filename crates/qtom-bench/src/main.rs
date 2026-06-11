use qtom_core::{
    AgentRouteTable, BatchMetrics, CpuRouter, FixtureConfig, ProjectConfig, RouterBackend,
    ScoreCoefficients, assert_backend_parity, batch_metrics, generate_fixture, read_golden_fixture,
    routing_results_checksum,
    score::{dist_sq, dist_sq_blocked},
    write_golden_fixture,
};
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
        BenchMode::WriteGolden { path } => run_write_golden(path),
        BenchMode::GoldenParity { path } => run_golden_parity(path),
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

struct CpuWorkerBackend {
    name: &'static str,
    router: CpuRouter,
    workers: usize,
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

fn checksum_results(results: &[qtom_core::RoutingResult]) -> f64 {
    routing_results_checksum(results)
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
    WriteGolden { path: std::path::PathBuf },
    GoldenParity { path: std::path::PathBuf },
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
                "--write-golden" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--write-golden requires a path".to_string());
                    };
                    mode = Self::WriteGolden { path: path.into() };
                }
                "--golden-parity" => {
                    let Some(path) = args.next() else {
                        return Self::Invalid("--golden-parity requires a path".to_string());
                    };
                    mode = Self::GoldenParity { path: path.into() };
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
            Self::WriteGolden { .. } | Self::GoldenParity { .. } | Self::Invalid(_) => &[],
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
            Self::WriteGolden { .. } => "write-golden",
            Self::GoldenParity { .. } => "golden-parity",
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
    fn golden_flags_require_paths() {
        assert_eq!(
            BenchMode::from_args(["--write-golden".to_string()]),
            BenchMode::Invalid("--write-golden requires a path".to_string())
        );
        assert_eq!(
            BenchMode::from_args(["--golden-parity".to_string()]),
            BenchMode::Invalid("--golden-parity requires a path".to_string())
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
