use qtom_core::{
    BatchMetrics, CpuRouter, FixtureConfig, ProjectConfig, ScoreCoefficients, batch_metrics,
    generate_fixture, score::dist_sq,
};
use std::env;
use std::time::{Duration, Instant};

const AGENT_COUNTS: [usize; 3] = [128, 1024, 8192];
const STRESS_AGENT_COUNTS: [usize; 1] = [65536];
const PROFILE_AGENT_COUNTS: [usize; 4] = [8192, 65536, 131072, 262144];
const PROFILE_DIMENSIONS: [usize; 2] = [16, 32];
const TOP_K_VALUES: [usize; 3] = [1, 4, 8];

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

    match mode {
        BenchMode::Smoke | BenchMode::Stress => run_route_matrix(mode),
        BenchMode::Profile => run_profile_matrix(),
    }
}

fn run_route_matrix(mode: BenchMode) {
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
    let fixture = generate_fixture(config);
    let mut latencies = Vec::with_capacity(fixture.requests.len());
    let start = Instant::now();
    let mut checksum = 0.0f64;

    for request in &fixture.requests {
        let route_start = Instant::now();
        let mut best_agent = 0u32;
        let mut best_distance = f32::INFINITY;

        for agent in &fixture.agents {
            let distance = dist_sq(&request.vector, &agent.vector);
            if distance < best_distance {
                best_distance = distance;
                best_agent = agent.id;
            }
        }

        checksum += best_distance as f64 + best_agent as f64;
        latencies.push(route_start.elapsed());
    }

    print_profile_report("scan", config, start.elapsed(), latencies, checksum);
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

fn scenario_seed(agent_count: usize, k: usize) -> u64 {
    0x5154_4f4d ^ ((agent_count as u64) << 8) ^ k as u64
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BenchMode {
    Smoke,
    Stress,
    Profile,
}

impl BenchMode {
    fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut mode = Self::Smoke;
        for arg in args {
            match arg.as_str() {
                "--stress" => mode = Self::Stress,
                "--profile" => mode = Self::Profile,
                _ => {}
            }
        }
        mode
    }

    fn agent_counts(self) -> &'static [usize] {
        match self {
            Self::Smoke => &AGENT_COUNTS,
            Self::Stress => &STRESS_AGENT_COUNTS,
            Self::Profile => &PROFILE_AGENT_COUNTS,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Stress => "stress",
            Self::Profile => "profile",
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
}
