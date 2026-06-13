# MVP Roadmap

**Status:** Draft build roadmap
**Date:** 2026-06-13
**Scope:** Implementation phases for the first local, observable, replayable Task Loom MVP.

## 1. Purpose

This roadmap turns the architecture artifacts into buildable steps. The first MVP should prove the loom semantics with cheap mocks before real local LLM agents are introduced.

The MVP goal is:

```text
Run a local Split, Build, Join, Remember loop where every task, route decision, artifact, decommission packet, memory node, and diagram projection is replayable from stored events.
```

The MVP should stay local, observable, and replayable.

## 2. Non-Goals

- No real LLM swarm in the first simulator.
- No remote API orchestration.
- No cluster dispatch.
- No autonomous topology approval.
- No live topology mutation.
- No CUDA changes unless needed for route-decision integration tests.
- No large artifact store.

## 3. Phase 0: Event Log Foundation

Build the smallest event-log layer that can support replay.

### Scope

- Add a local event-log module or crate.
- Define a `LoomEvent` shape.
- Support append-only writes.
- Support deterministic reads in write order.
- Support event type filtering.
- Support replay from an event slice.

### Candidate Storage

Start with JSONL unless there is a strong reason to use SQLite immediately. JSONL keeps diffs inspectable and simulator fixtures easy to create.

### Acceptance Criteria

- Appending events preserves order.
- Event IDs are unique in a run.
- Replay reads the same events in the same order.
- Invalid event payloads fail validation.
- Tests cover append, read, filter, and replay.

### Test Strategy

- Unit tests for event encoding and decoding.
- Fixture test for replay order.
- Property-style test for stable append/read round trips if lightweight enough.

### Commit Shape

One commit for the event type and in-memory log. A second commit for JSONL persistence if it is not tiny.

## 4. Phase 1: Mock SBJR Task Loom

Add mock role agents and the minimal task graph flow.

### Scope

- Mock Director Agent creates a `PlanNode` and child `TaskEnvelope` records.
- Mock Constructor Agent produces `ArtifactRef` records.
- Mock Integration Agent creates an `IntegrationReport`.
- Mock Curator Agent creates `MemoryNode` records.
- The loom records all transitions as events.

### Acceptance Criteria

- A root prompt creates a root task.
- The Director mock creates at least two child tasks.
- Every child task has prompt, root task, parent task, plan, and integration group lineage.
- The Constructor mock emits `artifact_declared`, `artifact_ready`, `task_completed`, and `agent_decommissioned`.
- The Integration mock emits an accepted report for completed children.
- The Curator mock emits memory nodes from decommission packets.

### Test Strategy

- Unit test each mock role in isolation.
- Integration test the full Split, Build, Join, Remember flow.
- Assert that the emitted event sequence matches `docs/lifecycle-flows.md`.

### Commit Shape

- One commit for task/plan/integration mock data.
- One commit for Director mock.
- One commit for Constructor and Integration mocks.
- One commit for Curator mock.

## 5. Phase 2: Route Mock Constructor Tasks Through Q-TOM

Connect the mock Task Loom to the existing Q-TOM router.

### Scope

- Build route requests from child task envelopes.
- Use deterministic simulated `AgentProfile` records.
- Route mock constructor tasks through Q-TOM.
- Record `route_decision_recorded`.
- Assign selected available candidates.
- Preserve observed-vs-available telemetry.

### Acceptance Criteria

- Every constructor task has a `RouteDecision`.
- Every assignment references a route decision.
- Route telemetry includes available candidate, observed ideal candidate reference, ideal-unavailable flag, substitute distance delta, and fallback status.
- Replay can reconstruct which agent was assigned and why.
- Existing CPU route tests remain the correctness oracle.

### Test Strategy

- Unit test route-request construction from a task envelope.
- Integration test mock loom dispatch through CPU router.
- Fixture test with one unavailable ideal candidate.
- Assert substitute distance delta is recorded.

### Commit Shape

- One commit for route request construction.
- One commit for route decision event recording.
- One commit for assignment flow through Q-TOM.

## 6. Phase 3: Decommission Packets And Memory Nodes

Make Remember real enough to support candidate curation.

### Scope

- Define minimal decommission packet structs for simulator use.
- Emit decommission packets for every mock agent completion.
- Curator mock derives typed memory nodes.
- Memory nodes preserve evidence refs.
- Memory nodes can be placed into a simple versioned `GradientSpace`.

### Acceptance Criteria

- Every completed constructor task has one decommission packet.
- Every decommission packet links to task, agent, prompt, plan, and artifacts.
- Every memory node links back to at least one packet or artifact.
- Memory placement records include gradient space version.
- Replay can reconstruct memory lineage.

### Test Strategy

- Unit test packet creation.
- Unit test memory node derivation.
- Integration test decommission-to-memory flow.
- Assert missing evidence refs fail validation.

### Commit Shape

- One commit for decommission packet model.
- One commit for memory node model.
- One commit for curator mock and placement.

## 7. Phase 4: Diagram Projections

Generate useful diagrams from events and entity references.

### Scope

- Task dependency projection.
- Route trace projection.
- Artifact provenance projection.
- Memory lineage projection.
- Text or Mermaid output.

### Acceptance Criteria

- Projection output is derived only from events and entity refs.
- No hidden simulator state is required.
- Task dependency graph includes prompt, parent, child, and integration group edges.
- Memory lineage graph includes packet, evidence, memory node, gradient space, and index version.
- Projection tests compare stable text output.

### Test Strategy

- Fixture event log for each projection.
- Snapshot-like tests using plain string comparison.
- Negative test for missing lineage edge.

### Commit Shape

- One commit per projection family.
- One commit for README or docs usage examples.

## 8. Phase 5: Replay And Validation Harness

Make replay a first-class acceptance gate.

### Scope

- Replay a complete event log.
- Validate event ordering rules.
- Validate required lineage edges.
- Validate integration path existence.
- Validate every completion has a decommission packet.
- Validate every memory node has evidence.

### Acceptance Criteria

- The full mock SBJR run replays deterministically.
- Replaying the same log produces the same projections.
- Broken logs fail with useful validation errors.
- The validation harness reports missing route decisions, missing integration groups, missing decommission packets, and missing memory evidence.

### Test Strategy

- Golden event-log fixture.
- Corrupted fixture tests.
- Full replay integration test.

### Commit Shape

- One commit for replay.
- One commit for validation rules.
- One commit for golden fixture coverage.

## 9. Phase 6: Local LLM Readiness Gate

Prepare to replace mocks with controlled local agents.

### Scope

- Define what a real local agent profile must implement.
- Define hydration boundaries for prompt, tools, MCP libraries, and memory sets.
- Define evaluator fixture expectations.
- Preserve mock mode as a test harness.

### Acceptance Criteria

- Mock and real agent runtimes share a narrow interface.
- The loom can run without network access.
- Real local agent execution can be disabled in CI.
- Benchmark/evaluator outputs are versioned before they can alter agent vectors.

### Test Strategy

- Trait or interface conformance tests.
- Mock runtime remains the default test runtime.
- Optional ignored tests for local model execution later.

### Commit Shape

- One commit for runtime interface.
- One commit for local-agent readiness docs.
- One commit for optional local execution harness when ready.

## 10. Phase 7: MVP Review Gate

Decide whether the simulator is good enough to justify real local agents.

### Acceptance Criteria

- Full mock run passes replay validation.
- Route decisions are recorded for constructor tasks.
- Decommission packets and memory nodes exist for completions.
- Task dependency and memory lineage projections are generated.
- No hard constraints are bypassed.
- The code remains small enough to reason about.

### Exit Decision

If the mock simulator feels wrong, scrap or reshape it before adding real LLMs. The simulator is supposed to make architecture flaws cheap to find.

## 11. Evidence Gates

The MVP should maintain these gates:

```text
event log gate       append/read/replay works
lineage gate         every entity links to required parents
routing gate         every assigned constructor task has RouteDecision
decommission gate    every completed task emits packet
memory gate          every MemoryNode has evidence
projection gate      diagrams derive from events only
replay gate          deterministic replay succeeds
```

## 12. First Build Sequence

The next engineering sequence should be:

1. Add event type definitions.
2. Add in-memory append-only event log.
3. Add JSONL persistence if the in-memory shape feels right.
4. Add minimal task envelope and plan structs for simulator use.
5. Add Director mock.
6. Add Constructor mock.
7. Add Integration mock.
8. Add Curator mock.
9. Route constructor tasks through Q-TOM.
10. Add replay validation.
11. Add diagram projections.

## 13. Open MVP Questions

- Should the simulator live in `qtom-core`, a new `qtom-loom` crate, or a `qtom-sim` crate?
- Should JSONL persistence come before or after the in-memory event model proves useful?
- Should event IDs be monotonic integers, UUID-like strings, or deterministic fixture IDs?
- Should projection output use Mermaid first or a plain adjacency-list format first?
- Which validation errors should be hard failures in MVP versus warnings?
- Should route requests become explicit events before implementation begins?
