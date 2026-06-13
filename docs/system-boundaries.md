# System Boundaries

**Status:** Draft architecture boundary map
**Date:** 2026-06-13
**Scope:** Ownership lines between Q-TOM, the Agent Task Loom, runtime, memory, evaluation, governance, and observability.

## 1. Purpose

This document states which layer owns each major responsibility and where the interfaces should harden into traits, APIs, or event contracts. The goal is to keep Q-TOM narrow while allowing the larger Agent Task Loom to become a real orchestration system.

The practical rule is:

```text
If a feature changes how candidates are scored, it may belong in Q-TOM.
If a feature changes how work is created, executed, remembered, or governed, it belongs above Q-TOM.
```

## 2. Boundary Principles

- Keep routing, orchestration, execution, memory, evaluation, governance, and observability separable.
- Keep hard constraints outside approximate candidate generation as exact masks.
- Keep raw evidence append-only and derived topology versioned.
- Keep CUDA, CPU, and future backend details behind a router trait boundary.
- Keep cross-layer coordination explicit through event contracts.
- Prefer typed references over hidden shared state.
- Do not let any layer silently mutate topology while a route is in flight.

## 3. Q-TOM Router

### Owns

- Candidate scoring over task, agent, or memory vectors.
- CPU route correctness oracle.
- CUDA backend parity for supported shapes.
- Backend trait boundary for CPU, CUDA, and future backends.
- Available top-k output.
- Observed top-k debug telemetry.
- Runtime-state penalties used in route scoring.
- Substitute quality metrics.
- Fallback status and ideal-unavailable flags.
- Golden fixture parity and benchmark hooks.

### Does Not Own

- Prompt decomposition.
- Task graph lifecycle.
- Agent process execution.
- LLM API calls.
- Tool invocation.
- Memory curation policy.
- Benchmark rubric authorship.
- Topology approval.
- Event-log storage.
- User-facing orchestration policy.

### Primary Interfaces

Expected trait boundary:

```text
RouterBackend
  route_batch(requests, registry, runtime_state, route_policy) -> RoutingResult[]
```

Expected input contracts:

- `RoutingRequest`
- `AgentRouteTable`
- `AgentRuntimeState`
- `RoutePolicy`
- `TopologySnapshotRef`

Expected output contracts:

- `RoutingResult`
- `RouteCandidate`
- `RouteDebugInfo`
- `RouteDecision`

Expected event contract:

```text
route_decision_recorded
```

The router emits route facts. It does not dispatch work.

## 4. Agent Task Loom

### Owns

- Prompt-to-task graph creation.
- SBJR lifecycle enforcement.
- Task lineage.
- Join policies.
- Integration group lifecycle.
- Dispatch decisions after routing.
- Task blocking, resuming, and completion state.
- Decommission packet requirements.
- Replay boundaries for orchestration events.
- Diagram-friendly task, signal, artifact, and memory edges.

### Does Not Own

- Low-level route scoring.
- CUDA memory layout.
- LLM inference internals.
- Tool implementation details.
- Benchmark grading internals.
- Direct mutation of committed topology snapshots.

### Primary Interfaces

Expected internal API boundary:

```text
TaskLoom
  accept_prompt(prompt) -> root_task_id
  create_task(envelope) -> task_id
  request_route(task_id, route_context) -> route_decision_id
  assign_task(task_id, agent_id) -> assignment_id
  request_integration(integration_group_id) -> integration_request_id
```

Expected event contracts:

- `task_created`
- `task_assigned`
- `task_blocked`
- `task_resumed`
- `task_completed`
- `integration_requested`
- `signal_emitted`

The Task Loom asks Q-TOM for ranked candidates, then owns what happens next.

## 5. Agent Runtime

### Owns

- Running agent profiles.
- Local model invocation.
- Prompt profile binding.
- Tool bundle binding.
- MCP library binding.
- Memory set hydration.
- Agent execution telemetry.
- Deliverable emission.
- Decommission packet emission.

### Does Not Own

- Task decomposition policy.
- Route scoring.
- Topology placement.
- Memory node curation.
- Benchmark grading policy.
- Event-log retention policy.

### Primary Interfaces

Expected runtime API boundary:

```text
AgentRuntime
  execute(task_envelope, agent_profile, hydrated_context) -> AgentExecutionResult
```

Expected input contracts:

- `TaskEnvelope`
- `AgentProfile`
- `HydratedContext`
- `ToolBindingSet`
- `MemorySetRef`

Expected output contracts:

- `ArtifactRef`
- `SignalRef`
- `AgentExecutionTelemetry`
- `AgentDecommissionPacket`

Expected event contracts:

- `artifact_declared`
- `artifact_ready`
- `signal_emitted`
- `agent_decommissioned`

The runtime executes assigned work. It does not decide global swarm topology.

## 6. Memory And Curator Layer

### Owns

- Decommission packet ingestion.
- Raw evidence references.
- Memory node derivation.
- Memory node typing.
- Gradient space placement.
- Placement disagreement and confidence.
- Curated memory candidate sets.
- Memory index versions.
- Memory retrieval telemetry.

### Does Not Own

- Raw route scoring logic.
- Prompt decomposition.
- Agent execution.
- Hard permission enforcement.
- Direct topology commits without governance.

### Primary Interfaces

Expected curator API boundary:

```text
Curator
  curate(packet, artifacts, traces) -> MemoryNode[]
  place(memory_node, gradient_space_version) -> PlacementRecord
  propose_candidates(query_context) -> CandidateSet
```

Expected input contracts:

- `AgentDecommissionPacket`
- `ArtifactRef`
- `ToolTraceRef`
- `ConversationLogRef`
- `GradientSpace`

Expected output contracts:

- `MemoryNode`
- `PlacementRecord`
- `CandidateSet`
- `MemoryRetrievalTrace`

Expected event contracts:

- `memory_node_created`
- `index_updated`
- `memory_candidates_proposed`

Curators can propose compact candidate sets. Q-TOM still performs exact final scoring inside those sets.

## 7. Evaluation Layer

### Owns

- Benchmark definitions.
- LLM-graded rubric versions.
- Evaluator configuration.
- Agent capability vector generation.
- Route-quality regression tests.
- Orchestration outcome metrics.
- Comparison against golden fixtures and exact CPU routing.
- Benchmark ledger updates.

### Does Not Own

- Production route dispatch.
- Topology commit authority.
- Agent runtime implementation.
- CUDA kernel implementation.

### Primary Interfaces

Expected evaluation API boundary:

```text
Evaluator
  grade(run_artifact, rubric_version, evaluator_config) -> EvaluationRecord
  update_agent_vector(agent_profile, evaluation_records) -> AgentVectorProposal
```

Expected input contracts:

- `BenchmarkSpec`
- `RubricVersion`
- `EvaluatorConfig`
- `ArtifactRef`
- `RouteDecision`
- `ExecutionTraceRef`

Expected output contracts:

- `EvaluationRecord`
- `AgentVectorProposal`
- `BenchmarkReport`

Expected event contracts:

- `benchmark_run_started`
- `benchmark_run_completed`
- `agent_vector_proposed`

Evaluation can propose new vectors or topology changes. Governance decides whether they become committed topology.

## 8. Governance Layer

### Owns

- Topology proposal lifecycle.
- Human or policy approval.
- Shadow routing comparisons.
- Canary routing windows.
- Topology snapshot commit.
- Rollback.
- Version pinning.
- Hard-constraint policy.
- Promotion criteria for route policies, memory indexes, gradient axes, and agent profiles.

### Does Not Own

- Low-level scoring math.
- Agent execution.
- Memory curation evidence.
- Benchmark execution internals.

### Primary Interfaces

Expected governance API boundary:

```text
TopologyGovernance
  propose(change_set, evidence_refs) -> topology_proposal_id
  evaluate(proposal, shadow_results) -> governance_decision
  commit(proposal) -> topology_snapshot_id
  rollback(snapshot_id) -> topology_snapshot_id
```

Expected input contracts:

- `TopologyProposal`
- `EvidenceRef`
- `ShadowRoutingReport`
- `CanaryReport`
- `ApprovalRecord`

Expected output contracts:

- `TopologySnapshot`
- `GovernanceDecision`
- `RollbackRecord`

Expected event contracts:

- `topology_proposed`
- `topology_shadowed`
- `topology_canaried`
- `topology_committed`
- `topology_rolled_back`

Governance is the commit boundary for self-improving or curator-driven topology changes.

## 9. Observability Layer

### Owns

- Route telemetry.
- Task graph projections.
- Signal flow projections.
- Memory lineage projections.
- Artifact provenance projections.
- Latency and throughput summaries.
- Ideal-unavailable and fallback dashboards.
- Deadlock, route churn, and repair-task metrics.
- Replay diagnostics.

### Does Not Own

- Dispatch decisions.
- Route scoring.
- Agent execution.
- Memory curation.
- Topology approval.

### Primary Interfaces

Expected observability API boundary:

```text
ObservabilitySink
  record(event) -> event_id
  project(query) -> projection
  summarize(metric_query) -> metric_report
```

Expected input contracts:

- `LoomEvent`
- `RouteDecision`
- `AgentExecutionTelemetry`
- `IntegrationReport`
- `MemoryRetrievalTrace`
- `TopologySnapshotRef`

Expected output contracts:

- `TaskDependencyProjection`
- `SignalFlowProjection`
- `MemoryLineageProjection`
- `ArtifactProvenanceProjection`
- `MetricReport`

Expected event contract:

```text
loom_event_recorded
```

Observability reads the system. It does not mutate the system.

## 10. Cross-Layer Flow

The nominal local MVP flow crosses the boundaries like this:

```text
Prompt
  -> Agent Task Loom creates TaskEnvelope records
  -> Q-TOM Router records RouteDecision records
  -> Agent Task Loom assigns tasks
  -> Agent Runtime emits ArtifactRef and AgentDecommissionPacket records
  -> Integration Agents produce IntegrationReport records
  -> Curator Agents derive MemoryNode records
  -> Governance commits any topology changes
  -> Observability projects lineage, metrics, and replay views
```

## 11. Boundary Smells

These are signs the architecture is drifting:

- Q-TOM starts parsing prompts.
- CUDA backend code knows about Director, Constructor, Integration, or Curator roles.
- Agent Runtime mutates memory indexes directly.
- Curator Agents can commit topology changes without proposal records.
- Evaluation updates agent vectors without versioned evidence.
- Observability changes dispatch behavior.
- Task Loom stores large artifact contents by default instead of references.
- Approximate candidate generation filters out candidates before hard constraints are applied.

## 12. Near-Term Implementation Guidance

The next implementation work should preserve these boundaries by starting with:

1. A small append-only event log.
2. Typed Task Loom events.
3. Mock SBJR role agents.
4. Route decisions stored as first-class records.
5. Decommission packets emitted for every mock agent completion.
6. Curated memory nodes derived from packets.
7. Diagram projections built from events instead of special-case state.

The first Task Loom MVP should be boring around the edges. The novelty belongs in the topology and routing behavior, not in hidden coupling between layers.
