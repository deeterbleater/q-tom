# Q-TOM And Agent Task Loom Architecture

**Status:** Draft formal architecture
**Date:** 2026-06-13
**Scope:** Consolidated architecture for Q-TOM as a routing substrate and the Agent Task Loom as the local-first orchestration system.

## 1. Executive Summary

Q-TOM is the exact routing and scoring substrate for a larger async agent orchestration system called the Agent Task Loom. The system treats orchestration as a topology problem: tasks, agents, memories, tools, and artifacts are placed into routable spaces; hard constraints remain exact; soft relevance can be projected into deterministic candidate neighborhoods.

The architecture separates low-level route scoring from high-level orchestration:

```text
Q-TOM Router       exact candidate scoring and telemetry
Agent Task Loom    Split, Build, Join, Remember orchestration
Curator Layer      memory curation and gradient-space placement
Governance Layer   versioned topology proposals, commits, and rollback
```

The first implementation target is a local, observable, replayable Task Loom simulator. It should use mock SBJR role agents before introducing real local LLM workers.

Supporting docs:

- `docs/glossary.md`
- `docs/system-boundaries.md`
- `docs/core-entities.md`
- `docs/event-vocabulary.md`
- `docs/lifecycle-flows.md`
- `docs/topology-governance.md`
- `docs/mvp-roadmap.md`
- `docs/realization-plan.md`
- `docs/formal-architecture-process.md`
- `docs/local-agent-readiness.md`
- `docs/mvp-review-gate.md`

## 2. Problem Statement

Most agent orchestration systems treat concurrency as queues, chains, or chat histories. That works for small workflows, but it becomes brittle when many specialized agents, tools, memories, and tasks compete for attention.

The project hypothesis is:

```text
Large agent swarms need topology-aware routing and replayable async orchestration.
```

The system must answer:

- Which agent should receive this task?
- Which substitute is acceptable if the ideal agent is unavailable?
- Which memory nodes are relevant enough to hydrate?
- Which work can proceed asynchronously?
- Which outputs must be joined?
- Which traces become reusable memory?
- Which topology changes are safe to promote?

## 3. Goals And Non-Goals

### Goals

- Keep Q-TOM narrow and correctness-first.
- Build an async Task Loom around SBJR: Split, Build, Join, Remember.
- Preserve replayability through append-only events and versioned topology snapshots.
- Track route decisions, candidate quality, ideal-unavailable behavior, and substitute distance.
- Make memory curation a candidate-generation layer before exact scoring.
- Add governance before self-improving topology updates.
- Prove the system locally before adding real LLM swarms or cluster dispatch.

### Non-Goals

- No claim that GPU routing always beats CPU routing.
- No cluster orchestration in the MVP.
- No autonomous topology mutation.
- No hidden memory rewrite behavior.
- No direct LLM prompt parsing inside CUDA kernels.
- No real local LLM swarm until the mock loom proves replay and lineage semantics.

## 4. Design Principles

- **Hard constraints are exact.** Permissions, availability, budgets, required tools, and memory access are masks, not suggestions.
- **Soft relevance may be fuzzy.** Semantic relevance, queue pressure, latency pressure, and neighborhood density may be projected or quantized when deterministic and measurable.
- **Q-TOM stays narrow.** It scores candidates and records route facts. It does not own orchestration.
- **The loom owns lifecycle.** Prompt decomposition, task assignment, integration, decommission, and memory curation are Task Loom concerns.
- **Everything important is replayable.** Route decisions, topology snapshots, event order, and evidence references must be durable.
- **Topology changes are proposals before truth.** New axes, indexes, policies, and agent profiles pass through governance.
- **Mocks before real agents.** The architecture should be cheap to break before local LLM execution is introduced.

## 5. System Context

The current codebase already contains:

- CPU routing truth source.
- CUDA `k = 1`, `dims = 16` narrow path.
- Golden fixtures and parity checks.
- Benchmark ledger for CPU/CUDA routing.
- Candidate prefilter experiments.
- Architecture roadmap artifacts.

The architecture extends that foundation into a local Task Loom:

```text
Prompt
  -> Director Agent splits work
  -> Q-TOM routes constructor tasks
  -> Constructor Agents build artifacts
  -> Integration Agents join outputs
  -> Curator Agents remember evidence
  -> Governance versions topology updates
```

## 6. Layered Architecture

### Q-TOM Router

Owns exact route scoring, CPU/CUDA backend boundaries, available top-k, observed top-k telemetry, substitute quality, fallback status, and golden fixture parity.

### Agent Task Loom

Owns prompt-to-task graph creation, task assignment, SBJR lifecycle, join policies, task lineage, integration groups, and replay boundaries.

### Agent Runtime

Owns execution of local agent profiles, prompt/tool/MCP/memory binding, artifact emission, telemetry, and decommission packets.

### Memory And Curator Layer

Owns decommission packet ingestion, typed memory nodes, gradient-space placement, candidate memory sets, and memory index versions.

### Evaluation Layer

Owns benchmark definitions, evaluator configuration, capability vector generation, route-quality regression tests, and benchmark reports.

### Governance Layer

Owns topology proposals, shadow routing, canary routing, approval records, topology snapshot commits, rollback, and version pinning.

### Observability Layer

Owns route telemetry, task graph projections, signal flow, memory lineage, artifact provenance, latency summaries, and replay diagnostics.

See `docs/system-boundaries.md` for ownership details.

## 7. Core Entity Model

The durable model starts with:

```text
Prompt
PlanNode
TaskEnvelope
AgentProfile
RouteRequest
RouteDecision
ArtifactRef
SignalRef
IntegrationGroup
IntegrationReport
AgentDecommissionPacket
MemoryNode
GradientSpace
TopologyProposal
TopologySnapshot
```

Each entity needs an owner, lifecycle, storage expectation, and lineage requirements. The first implementation may use local structs and JSONL events, but those contracts should not depend on the storage engine.

See `docs/core-entities.md`.

## 8. Event And Storage Model

The Task Loom is event-driven. The event log is the canonical history. Status tables, diagrams, and metrics are projections.

Initial events include:

```text
task_created
task_assigned
artifact_declared
artifact_ready
signal_emitted
task_blocked
task_resumed
task_completed
agent_decommissioned
integration_requested
memory_node_created
index_updated
route_decision_recorded
topology_proposed
topology_committed
```

Storage guidance:

```text
append-only events     canonical history
status projections     query acceleration
artifact store         heavy content by reference
fixture files          benchmark and route parity data
topology snapshots     immutable committed architecture state
```

See `docs/event-vocabulary.md`.

## 9. Routing Lifecycle

Routing begins when a task enters a routable state.

```text
TaskEnvelope
  -> RouteRequest
  -> hard-constraint masks
  -> candidate scoring
  -> available top-k
  -> RouteDecision
  -> task assignment
```

Route decisions must record:

- route policy
- backend
- topology snapshot
- candidate set
- live-state snapshot
- selected candidate
- available top-k
- observed ideal candidate reference when available
- ideal-unavailable flag
- substitute distance delta
- fallback status

The CPU router remains the correctness oracle. CUDA and future backends must prove parity against CPU/golden fixtures for supported shapes.

## 10. Task Loom Lifecycle

The Task Loom lifecycle is SBJR:

```text
Split
Build
Join
Remember
```

Director Agents split tasks. Constructor Agents build artifacts. Integration Agents join outputs. Curator Agents remember evidence.

The MVP should start with mock role agents that emit the same events real agents will eventually emit. This proves task graph semantics without incurring LLM runtime complexity.

See `docs/lifecycle-flows.md`.

## 11. Memory Curation Lifecycle

Memory curation starts from canonical evidence:

```text
AgentDecommissionPacket
  -> evidence refs
  -> MemoryNode
  -> GradientSpace placement
  -> memory index version
  -> future candidate set
  -> exact Q-TOM scoring
```

Raw logs and decommission packets are append-only. Memory nodes are derived, versioned interpretations with evidence pointers.

The performance goal is not to make memory fuzzy and unaccountable. The goal is to let curators produce compact candidate sets, then use exact scoring inside those sets.

## 12. Topology Update Lifecycle

Topology changes follow governance:

```text
drafted
  -> proposed
  -> tested
  -> shadowed
  -> canaried
  -> approved
  -> committed
  -> active
  -> superseded or rolled back
```

Governed changes include new axes, split axes, deprecated axes, new agent profiles, benchmark schema changes, memory index versions, and route policy changes.

No proposal becomes routing truth until it is evaluated, versioned, committed, and rollbackable.

See `docs/topology-governance.md`.

## 13. Replay And Determinism

Replay should reconstruct the task graph, route decisions, artifacts, integration attempts, decommission packets, memory nodes, and topology snapshot references without re-running LLM inference.

Replay requires:

- append-only event order
- stable event IDs
- content hashes or immutable artifact refs
- topology snapshot IDs
- route policy versions
- live-state snapshot refs
- evidence refs for memory nodes

Determinism does not mean the system cannot be fuzzy. It means the fuzziness is explicit, deterministic, and inspectable.

## 14. Evidence Gate Status

The architecture is promoted claim by claim. Current gate status:

| Gate | Status | Current Evidence | Remaining Work |
| --- | --- | --- | --- |
| Route parity | Cleared for current CPU truth and narrow backend parity checks. | Golden fixture tests, CPU oracle tests, and backend parity reports. | Broaden parity evidence as more CUDA shapes and candidate-set paths are supported. |
| Route explainability | Cleared for mock loom assignments. | `route_decision_recorded` events cause `task_assigned` events, with replay validation checking task, root, and correlation context. | Keep the same contract when real runtime roles replace mocks. |
| Loom replay | Cleared for the mock SBJR run. | The checked-in mock loom event log round-trips through JSONL and produces a replay report from events only. | Add durable storage benchmarks before moving beyond JSONL. |
| Join correctness | Cleared for mock integration groups. | Integration-group projection and validation link parent tasks, child tasks, join policy, integration reports, and integration agents. | Add repair-loop cases. |
| Memory evidence | Cleared for decommission-to-memory lineage. | Decommission packets are persisted as canonical evidence, memory nodes preserve evidence refs, and memory lineage projection reads from replay events. | Add richer memory typing and placement disagreement checks. |
| Candidate reduction | Partial. | The candidate reduction report, `MemoryCandidateReport`, records total placements, hard-mask survivors, radius matches, returned candidates, target status, hard-mask violation rate, and scanned-candidate reduction. | Measure recall and p99 latency against exact full-scan routing before trusting candidate reduction without fallback. |
| Evaluator scaffolding | Cleared for stored fixtures. | The checked-in mock evaluator fixture preserves evaluator model, rubric version, prompt version, scoring schema version, seed, task ID, artifact refs, score, and rationale. | Add real evaluator API invocation behind configuration only after local-agent readiness gates pass. |
| Local agent usefulness | Open. | No Qwen-backed role has been promoted yet. | Prove prompt/tool/MCP/memory profile differences with versioned evaluator fixtures. |
| Governance safety | Partial. | `TopologyGovernanceStore` reads proposal, snapshot, and rollback JSONL ledgers together; validates unknown proposal and unknown snapshot references; resolves `active_topology_snapshot`; and applies rollback through `apply_rollback` by appending the rollback record, activating the target snapshot, rolling back the failed snapshot, and superseding other active snapshots. | Add shadow routing and canary evidence, then connect governance records to replay events and route-decision fixtures. |

## 15. Evaluation And Benchmarking

Evaluation has two jobs:

- Generate and update agent capability vectors.
- Verify that routing and orchestration changes improve measured behavior.

Benchmark records should include evaluator model, rubric version, prompt version, scoring schema version, and structured rationales.

Routing benchmarks should report:

- p50, p95, p99, max latency
- routes/sec
- top-k overlap
- candidate recall
- fallback rate
- ideal-unavailable rate
- substitute distance delta
- hard-constraint violation rate

The current routing benchmark evidence lives in `docs/benchmark-ledger.md`.

## 16. Observability

Observability should make the loom understandable from stored events.

Required projections:

- task dependency diagram
- route trace diagram
- agent handoff diagram
- artifact provenance diagram
- integration group diagram
- decommission lineage diagram
- memory lineage diagram
- topology governance diagram

Metrics should include deadlocks, blocked tasks, route churn, fallback rate, repair tasks, integration failures, and memory retrieval quality.

## 17. Security And Governance

The security posture starts with hard constraints:

- availability
- permissions
- required tool access
- memory access
- model class
- budget ceilings
- safety boundaries
- data access policy

Hard constraints are evaluated before approximate candidate generation. Any proposal that changes hard-constraint behavior requires explicit governance.

Governance records must preserve who or what proposed a change, what evidence supported it, how it was tested, who approved it, what snapshot committed it, and how it can be rolled back.

## 18. Scaling Strategy

Scaling should proceed in stages:

```text
Stage A  Single-machine router
Stage B  Local Task Loom simulator
Stage C  Local LLM Task Loom
Stage D  Curated memory routing
Stage E  Self-improving topology proposals
Stage F  Cluster-capable swarm
```

Do not jump to cluster behavior until local replay, lineage, memory curation, and topology governance work.

## 19. MVP Plan

The MVP plan is:

1. Build event log foundation.
2. Add mock SBJR Task Loom.
3. Route mock constructor tasks through Q-TOM.
4. Emit decommission packets and memory nodes.
5. Generate diagram projections.
6. Add replay and validation harness.
7. Define the local LLM readiness gate.

The MVP acceptance bar is a complete local mock run that can be replayed from events and topology snapshots.

See `docs/mvp-roadmap.md`.

The process for promoting design intent into working architecture is defined in `docs/formal-architecture-process.md`.

## 20. Risks And Open Questions

Key risks:

- The topology may not predict useful real-agent substitution.
- Memory curation may become expensive or subjective.
- The event model may become too heavy before real value appears.
- CUDA exact scoring may remain limited by candidate count unless curation works.
- Governance may slow iteration if it is too formal too early.
- LLM-graded benchmarks may be noisy without strong rubrics and versioning.

Open questions:

- Should the simulator live in `qtom-core`, `qtom-loom`, or `qtom-sim`?
- Should JSONL or SQLite back the first event log?
- Which `GradientSpace` axes should seed memory curation?
- Should route requests become first-class events?
- Which topology changes can skip canary in local development?
- What route-quality threshold justifies moving from mocks to local LLM workers?
