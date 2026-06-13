# MVP Review Gate

**Status:** Draft review gate
**Date:** 2026-06-13
**Scope:** Exit criteria for deciding whether the local mock Task Loom simulator is good enough to justify real local agents.

## 1. Purpose

The mock simulator exists to make architecture flaws cheap to find. It should not be treated as a success merely because tests pass. The review gate decides whether the current loom shape is strong enough to move toward real local agents, or whether the simulator should be reshaped or scrapped first.

The review question is:

```text
Does the mock loom prove enough replay, lineage, routing, memory, and observability behavior to justify controlled local Qwen-backed execution?
```

## 2. Required Evidence

The MVP must show evidence in the repository, not only in notes.

### Replay Validation

Required evidence:

- Full mock run passes replay validation.
- Broken logs fail with specific validation errors.
- Replay validation counts task events, route decisions, assignments, completions, decommission events, memory nodes, topology commits, and integration requests.

Go signal:

- The full mock SBJR flow validates deterministically from stored events.

No-go signal:

- Important state exists only in runtime structs and cannot be reconstructed from events.

### Routing Evidence

Required evidence:

- Route decisions are recorded for constructor tasks.
- Each task assignment references a route decision.
- Route telemetry records selected candidate, available candidates, observed candidates, ideal-unavailable status, substitute distance delta, fallback status, backend, policy, and route telemetry payload shape.
- Route trace projection is derived from events.

Go signal:

- A reviewer can explain why each constructor task was assigned without reading hidden scheduler state.

No-go signal:

- Assignments can happen without a route decision or route telemetry is too thin to audit.

### Decommission And Memory Evidence

Required evidence:

- Decommission packets exist for completed constructor tasks.
- Decommission packets preserve task, agent, prompt, plan, and artifact lineage.
- Memory nodes preserve evidence refs back to packets or artifacts.
- Memory nodes can be placed into a versioned `GradientSpace`.
- Memory lineage projection links task, decommission evidence, and memory nodes.

Go signal:

- Curator output is evidence-backed and versioned enough to become candidate memory later.

No-go signal:

- Memory nodes are summaries without durable evidence, or placement lacks gradient space versioning.

### Projection Evidence

Required evidence:

- Route trace projection is generated from events only.
- Task dependency projection exists or has a named gap before real local agents.
- Memory lineage projection is generated from events only.
- Projection tests compare stable text output.

Go signal:

- A run can be inspected through diagrams without depending on hidden in-memory simulator state.

No-go signal:

- Diagrams require special-case simulator state or manual reconstruction.

### Hard Constraints

Required evidence:

- Hard constraints are exact masks or validation rules.
- Hard constraints are not bypassed by fuzzy projection, mock runtime behavior, or memory placement.
- Local agent execution remains disabled in CI by default.
- Mock runtime remains the default test runtime.

Go signal:

- The system can stay deterministic and offline while adding optional real local execution later.

No-go signal:

- Any path requires network access, a local model, a GPU, or secrets to pass normal tests.

## 3. Go Decision

A go decision means:

```text
Proceed to one controlled real local Constructor Agent fixture.
```

The first real local agent should be narrow:

- one Constructor role
- one local `Qwen3-2507` model profile
- one task fixture
- one evaluator fixture
- one measurable behavior difference
- ignored tests by default

Go does not mean:

- enabling local LLM execution in CI
- replacing all mocks
- adding remote inference
- allowing topology mutation
- trusting evaluator output to mutate agent vectors directly

## 4. No-Go Decision

A no-go decision means:

```text
Do not add real local agents yet.
```

Use no-go when:

- replay validation is incomplete
- route decisions are not auditable
- decommission packets are not canonical evidence
- memory nodes lack evidence refs
- projections cannot be generated from events
- hard constraints are fuzzy or bypassable
- the simulator is already too hard to reason about

## 5. Reshape Or Scrap Criteria

Reshape the simulator when:

- the event vocabulary is close but missing specific lineage fields
- projections reveal unclear edges
- validation errors are too vague
- mock runtime boundaries need a narrower interface
- memory placement works but needs richer evidence

Scrap the simulator path when:

- replay cannot reconstruct the main flow
- the loom requires hidden mutable state to function
- mocks and real-runtime boundaries diverge
- routing is bolted on after assignment instead of causing assignment
- memory curation cannot be tied to decommission evidence

Scrapping here is not failure. It means the simulator did its job before real local agents made the mistake expensive.

## 6. Review Checklist

Before a go decision, check:

- `cargo test --workspace --quiet` passes.
- Full mock SBJR run validates replay.
- Route decisions exist for constructor tasks.
- Assignments reference route decisions.
- Decommission packets exist for completions.
- Memory nodes exist and preserve evidence refs.
- Memory placement records include gradient space version.
- Route trace projection is event-derived.
- Memory lineage projection is event-derived.
- Task dependency projection exists or is explicitly scheduled before real local execution.
- Evaluator fixture output is versioned.
- Real local execution remains opt-in and ignored by default.

## 7. Current Bias

The current bias should remain conservative:

```text
One more observability or validation improvement is usually cheaper than one premature local-agent integration.
```

Real local agents are justified only when the simulator is boring to replay, inspect, and explain.
