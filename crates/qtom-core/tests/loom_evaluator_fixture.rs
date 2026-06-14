use std::fs;

use qtom_core::{
    EvaluationFixture, EvaluatorConfig, LoomModelError, append_evaluation_fixture_jsonl,
    read_evaluation_fixtures_jsonl, write_evaluation_fixtures_jsonl,
};

fn temp_jsonl_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "qtom-evaluator-{name}-{}-{}.jsonl",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

fn evaluator_config() -> EvaluatorConfig {
    EvaluatorConfig::new(
        "gpt-5.5-medium",
        "route-quality-rubric-v1",
        "constructor-output-prompt-v1",
        "score-schema-v1",
        0.0,
        Some(42),
    )
    .expect("config should be valid")
}

fn fixture(evaluation_id: u64, task_id: u64, score: f32) -> EvaluationFixture {
    EvaluationFixture::new(
        evaluation_id,
        evaluator_config(),
        task_id,
        vec![900 + task_id],
        score,
        format!("Fixture {evaluation_id} preserves route evidence."),
    )
    .expect("fixture should be valid")
}

#[test]
fn evaluation_fixture_preserves_versioned_evaluator_metadata() {
    let fixture = EvaluationFixture::new(
        7_000,
        evaluator_config(),
        11,
        vec![900],
        0.82,
        "The artifact satisfies the task and preserves route evidence.",
    )
    .expect("fixture should be valid");

    assert_eq!(fixture.evaluation_id, 7_000);
    assert_eq!(fixture.evaluator.model, "gpt-5.5-medium");
    assert_eq!(fixture.evaluator.rubric_version, "route-quality-rubric-v1");
    assert_eq!(
        fixture.evaluator.prompt_version,
        "constructor-output-prompt-v1"
    );
    assert_eq!(fixture.evaluator.scoring_schema_version, "score-schema-v1");
    assert_eq!(fixture.evaluator.temperature, 0.0);
    assert_eq!(fixture.evaluator.seed, Some(42));
    assert_eq!(fixture.task_id, 11);
    assert_eq!(fixture.artifact_refs, vec![900]);
    assert_eq!(fixture.score, 0.82);
    assert!(fixture.rationale.contains("route evidence"));
}

#[test]
fn evaluation_fixture_rejects_score_outside_unit_interval() {
    let err = EvaluationFixture::new(7_000, evaluator_config(), 11, vec![900], 1.2, "too high")
        .expect_err("score outside unit interval should fail");

    assert_eq!(
        err,
        LoomModelError::InvalidNumericField {
            field: "score",
            reason: "must be between 0 and 1",
        }
    );
}

#[test]
fn evaluation_fixture_requires_structured_rationale() {
    let err = EvaluationFixture::new(7_000, evaluator_config(), 11, vec![900], 0.5, " ")
        .expect_err("blank rationale should fail");

    assert_eq!(err, LoomModelError::EmptyField("rationale"));
}

#[test]
fn evaluation_fixtures_round_trip_through_jsonl() {
    let path = temp_jsonl_path("roundtrip");
    let fixtures = vec![fixture(7_000, 11, 0.82), fixture(7_001, 12, 0.71)];

    write_evaluation_fixtures_jsonl(&path, &fixtures).expect("fixtures should write");
    let read = read_evaluation_fixtures_jsonl(&path).expect("fixtures should read");

    assert_eq!(read, fixtures);

    fs::remove_file(path).ok();
}

#[test]
fn append_evaluation_fixture_preserves_existing_fixtures() {
    let path = temp_jsonl_path("append");

    append_evaluation_fixture_jsonl(&path, &fixture(7_000, 11, 0.82))
        .expect("first append should work");
    append_evaluation_fixture_jsonl(&path, &fixture(7_001, 12, 0.71))
        .expect("second append should work");

    let read = read_evaluation_fixtures_jsonl(&path).expect("fixtures should read");
    assert_eq!(
        read.iter()
            .map(|fixture| fixture.evaluation_id)
            .collect::<Vec<_>>(),
        vec![7_000, 7_001]
    );

    fs::remove_file(path).ok();
}

#[test]
fn append_evaluation_fixture_rejects_duplicate_evaluation_id() {
    let path = temp_jsonl_path("duplicate");

    append_evaluation_fixture_jsonl(&path, &fixture(7_000, 11, 0.82))
        .expect("first append should work");

    let err = append_evaluation_fixture_jsonl(&path, &fixture(7_000, 12, 0.71))
        .expect_err("duplicate evaluation id should fail");

    assert_eq!(err, LoomModelError::DuplicateEvaluationId(7_000));
    assert_eq!(
        read_evaluation_fixtures_jsonl(&path).expect("existing fixture should remain"),
        vec![fixture(7_000, 11, 0.82)]
    );

    fs::remove_file(path).ok();
}
