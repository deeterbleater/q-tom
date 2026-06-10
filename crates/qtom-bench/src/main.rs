use qtom_core::{
    BatchMetrics, CpuRouter, FixtureConfig, ProjectConfig, RouterBackend, ScoreCoefficients,
    batch_metrics, generate_fixture,
};
use std::time::Instant;

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

    let scenarios = [
        FixtureConfig {
            agent_count: 128,
            task_count: 128,
            dimensions: 16,
            k: 8,
            seed: 0x5154_4f4d,
        },
        FixtureConfig {
            agent_count: 1024,
            task_count: 512,
            dimensions: 16,
            k: 8,
            seed: 0x5154_4f4e,
        },
        FixtureConfig {
            agent_count: 8192,
            task_count: 2048,
            dimensions: 16,
            k: 8,
            seed: 0x5154_4f4f,
        },
    ];

    for config in scenarios {
        run_scenario(config);
    }
}

fn run_scenario(config: FixtureConfig) {
    let fixture = generate_fixture(config);
    let router = CpuRouter::new(fixture.agents, ScoreCoefficients::default());
    let start = Instant::now();
    let results = router
        .route_batch(&fixture.requests, &fixture.states)
        .expect("fixture should be valid");
    let elapsed = start.elapsed();
    let metrics = batch_metrics(&results);

    print_report(config, elapsed.as_secs_f64(), metrics);
}

fn print_report(config: FixtureConfig, elapsed_secs: f64, metrics: BatchMetrics) {
    let routes_per_second = metrics.routes as f64 / elapsed_secs.max(f64::EPSILON);
    println!(
        "agents={:<5} tasks={:<5} dims={:<2} k={} elapsed_ms={:>8.3} routes_s={:>12.1} ideal_unavailable={:<5} mean_delta={:.6} mean_radius={:.6}",
        config.agent_count,
        config.task_count,
        config.dimensions,
        config.k,
        elapsed_secs * 1000.0,
        routes_per_second,
        metrics.ideal_unavailable_count,
        metrics.mean_substitute_distance_delta,
        metrics.mean_top_k_radius
    );
}
