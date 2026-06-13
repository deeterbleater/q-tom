use qtom_core::{EvaluationFixture, EvaluatorConfig, LoomModelError};

#[test]
fn evaluation_fixture_preserves_versioned_evaluator_metadata() {
    let config = EvaluatorConfig::new(
        "gpt-5.5-medium",
        "route-quality-rubric-v1",
        "constructor-output-prompt-v1",
        "score-schema-v1",
        0.0,
        Some(42),
    )
    .expect("config should be valid");

    let fixture = EvaluationFixture::new(
        7_000,
        config,
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
    let config = EvaluatorConfig::new(
        "gpt-5.5-medium",
        "route-quality-rubric-v1",
        "constructor-output-prompt-v1",
        "score-schema-v1",
        0.0,
        Some(42),
    )
    .expect("config should be valid");

    let err = EvaluationFixture::new(7_000, config, 11, vec![900], 1.2, "too high")
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
    let config = EvaluatorConfig::new(
        "gpt-5.5-medium",
        "route-quality-rubric-v1",
        "constructor-output-prompt-v1",
        "score-schema-v1",
        0.0,
        Some(42),
    )
    .expect("config should be valid");

    let err = EvaluationFixture::new(7_000, config, 11, vec![900], 0.5, " ")
        .expect_err("blank rationale should fail");

    assert_eq!(err, LoomModelError::EmptyField("rationale"));
}
