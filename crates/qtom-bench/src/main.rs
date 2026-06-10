use qtom_core::{
    BatchMetrics, CpuRouter, FixtureConfig, ProjectConfig, ScoreCoefficients, batch_metrics,
    generate_fixture,
};
use std::time::{Duration, Instant};

const AGENT_COUNTS: [usize; 3] = [128, 1024, 8192];
const TOP_K_VALUES: [usize; 3] = [1, 4, 8];

fn main() {
    let project_config = ProjectConfig::from_env_and_dotenv(".env");
    println!(
        "config local_model={} evaluator_model={} api_key_present={} default_k={} default_agents={}",
        project_config.local_model,
        project_config.evaluator_model,
        project_config.openai_api_key_present,
        project_config.default_k,
        project_config.default_agent_count
    );

    println!(
        "agents,tasks,dims,k,total_ms,routes_s,p50_us,p95_us,p99_us,max_us,ideal_unavailable,mean_delta,mean_radius"
    );

    for agent_count in AGENT_COUNTS {
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

fn task_count_for(agent_count: usize) -> usize {
    match agent_count {
        128 => 128,
        1024 => 512,
        _ => 2048,
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
}
