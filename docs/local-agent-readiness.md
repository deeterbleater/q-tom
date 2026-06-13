# Local Agent Readiness

**Status:** Draft readiness gate
**Date:** 2026-06-13
**Scope:** Requirements before replacing mock Task Loom roles with controlled local LLM-backed agents.

## 1. Purpose

The Task Loom should not jump directly from mock agents to an uncontrolled local swarm. Real local execution must enter through a narrow, testable boundary so replay, routing, decommission, memory, and evaluation remain inspectable.

This document defines the gate for introducing local `Qwen3-2507` agents while preserving mock mode as the default test runtime.

## 2. Current Boundary

The code boundary is:

```text
AgentRuntime
  execute(task: TaskEnvelope, context: HydratedContext) -> AgentExecutionResult
```

`AgentRuntime` is the only interface a real local role should need to implement at first.

`HydratedContext` defines the inputs a runtime may receive:

```text
prompt_ref
tool_refs
memory_refs
```

The runtime returns:

```text
AgentExecutionResult
  artifacts
  decommission_packet
  event_log
```

The runtime does not own routing, memory curation, topology updates, evaluator policy, or event retention.

## 3. Mock Runtime Rule

The mock runtime remains the default test runtime.

Default CI must run without:

- network access
- OpenAI API access
- local model server access
- GPU availability
- Qwen runtime availability

The current `MockConstructorRuntime` proves the boundary offline by adapting the existing mock Constructor behavior into `AgentRuntime`.

## 4. Real Local Runtime Gate

A real local runtime may be added only after it satisfies the same interface and produces the same durable output classes as the mock runtime.

Required behavior:

- Accept a `TaskEnvelope`.
- Accept a `HydratedContext`.
- Produce at least one `ArtifactRef` or a typed blocked/failure result when that model exists.
- Produce an `AgentDecommissionPacket`.
- Emit replay-valid loom events.
- Avoid hidden writes outside the configured artifact and event stores.
- Keep model identity and prompt profile visible in configuration or evidence refs.

Initial model target:

```text
QTOM_LOCAL_MODEL=Qwen3-2507
```

The exact runtime provider is not part of this document. The implementation may use a local server, local process, or future adapter as long as it implements `AgentRuntime`.

## 5. CI And Optional Tests

Real local agent execution must be disabled in CI by default.

Any local-model execution test should be treated as ignored tests and should be:

- opt-in
- ignored by default
- clearly named as local execution
- safe to skip on machines without the model
- unable to affect committed fixtures unless explicitly requested

Suggested test shape:

```text
#[ignore]
local_qwen_constructor_runtime_executes_fixture
```

The normal workspace test suite should continue to pass with only mock runtimes.

## 6. Hydration Boundaries

Hydration should remain explicit. The runtime receives references, not hidden global state.

Initial hydration classes:

- prompt refs
- tool refs
- MCP library refs
- memory refs
- input artifact refs

The MVP `HydratedContext` currently covers prompt, tools, and memory. Input artifacts and MCP library refs can be added when the first real role needs them.

## 7. Evaluator Fixture Gate

Benchmark and evaluator outputs must be versioned before they can alter agent vectors.

Each evaluator fixture should preserve:

- evaluator model
- rubric version
- prompt version
- scoring schema version
- temperature
- seed when available
- task ID
- artifact refs
- numeric score
- structured rationale

Evaluator output should not directly mutate an `AgentProfile`. It should produce a versioned `EvaluationFixture`; governance or an explicit calibration step can later decide whether that fixture updates capability vectors.

## 8. Promotion Checklist

Before replacing any mock role with a real local role:

- The role implements `AgentRuntime`.
- Mock runtime tests still pass.
- Real runtime tests are ignored by default.
- The local model can be disabled through configuration.
- The runtime emits replay-valid events.
- The runtime produces decommission evidence.
- Evaluator fixtures are versioned.
- No API key or secret is printed.
- No route, memory, or topology mutation happens inside the runtime.

## 9. First Candidate Role

The first real role should be a Constructor Agent.

Reasoning:

- Constructor tasks already have clear inputs and outputs.
- The runtime boundary already returns artifacts and decommission packets.
- Director and Integration roles are more likely to mutate task structure.
- Curator roles affect memory placement and therefore topology pressure.

Start with one controlled fixture, one local model profile, and one measurable behavior change.
