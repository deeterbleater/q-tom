# Architecture Roadmap

**Status:** Draft planning document
**Date:** 2026-06-13
**Scope:** Steps from the current Q-TOM routing prototype to a formal architecture document and local Task Loom MVP.

## 1. Purpose

Q-TOM has cleared the first conceptual hurdle: the routing substrate is now specific enough to benchmark, and the larger orchestration model has a coherent shape. The next goal is not to write a final architecture document in one pass. The next goal is to produce the intermediate artifacts that make a formal architecture document precise, testable, and resistant to hand-waving.

This roadmap describes how to get from:

```text
Q-TOM routing prototype
```

to:

```text
formal architecture document
  -> local async Task Loom MVP
  -> evaluated local-agent orchestration system
  -> cluster-capable architecture decisions
```

The main architectural split is:

```text
Q-TOM stays narrow.
The Agent Task Loom becomes the system.
```

Q-TOM should remain the exact scoring and routing substrate. The Task Loom should own decomposition, execution, joining, decommission, memory curation, topology evolution, replay, and governance.

## 2. Working Thesis

The system treats agent orchestration as an async topology problem. Tasks, agents, tools, memories, and artifacts are placed into routable spaces; deterministic hard constraints preserve replayability; soft topological state handles ambiguity, substitution, and fuzziness.

The core claim should stay conservative:

```text
Topology-aware routing can make large agent swarms easier to schedule, inspect, replay, and adapt.
```

The project does not need to claim new mathematics. It needs to prove that this design pattern produces better operational behavior than flat queues, ad hoc agent selection, or monolithic prompt loops.

## 3. Layer Boundaries

The formal architecture should separate the system into clear layers.

### 3.1 Q-TOM Router

Responsibilities:

- Maintain CPU and CUDA route parity.
- Score task vectors against candidate agent or memory vectors.
- Return ranked available candidates.
- Preserve observed-vs-available telemetry.
- Measure substitute quality, ideal-unavailable rate, top-k radius, and latency.
- Expose backend boundaries without leaking CUDA details upward.

Non-responsibilities:

- Decomposing prompts.
- Executing LLM calls.
- Owning memory policy.
- Mutating topology live.
- Deciding whether a proposed new agent profile should exist.

### 3.2 Agent Task Loom

Responsibilities:

- Represent prompts as async task graphs.
- Enforce Split, Build, Join, Remember lifecycle semantics.
- Dispatch tasks through Q-TOM route decisions.
- Track lineage from prompt to plan, task, agent, artifact, decommission packet, and memory node.
- Model explicit join policies.
- Keep synchronous waits limited to named join points.

### 3.3 Agent Runtime

Responsibilities:

- Run local agent profiles.
- Bind prompts, tool bundles, MCP libraries, memory sets, and model profiles.
- Report execution telemetry.
- Emit deliverables and decommission packets.

Prototype 1 keeps the model profile fixed to local `Qwen3-2507`. Agent variation comes from system prompt profile, tool bundle, MCP library set, memory set, and higher-level worker behavior.

### 3.4 Memory And Curator Layer

Responsibilities:

- Treat raw execution logs and decommission packets as canonical append-only evidence.
- Convert raw packets into typed memory nodes.
- Place memory nodes into versioned shared `GradientSpace` indexes.
- Produce compact candidate sets for exact Q-TOM scoring.
- Preserve evidence references and placement disagreement.

The memory layer should use lossy deterministic projection only for soft relevance. Hard constraints such as permissions, access policy, budget ceilings, and required tools remain exact masks.

### 3.5 Evaluation Layer

Responsibilities:

- Generate benchmark-derived agent capability vectors.
- Keep evaluator prompts, rubrics, model IDs, and scoring schemas versioned.
- Compare route decisions against golden fixtures and exact CPU routing.
- Track orchestration-level outcomes: deadlocks, route churn, fallback rate, repair tasks, join failures, and final artifact quality.

The intended evaluator is configured as GPT-5.5 Medium via API, but the exact model identifier must remain configuration, not a hard-coded architecture assumption.

### 3.6 Governance Layer

Responsibilities:

- Separate proposal from commit for topology changes.
- Review new axes, agent profiles, benchmark schemas, memory placement rules, and routing policies.
- Support shadow routing, canary routing, rollback, and version pinning.
- Enforce hard constraints before any approximate candidate generation.

Topology updates should be versioned artifacts, not invisible live mutation.

### 3.7 Observability Layer

Responsibilities:

- Record route decisions, substitute quality, ideal-unavailable flags, and fallback reasons.
- Preserve task graph lineage.
- Expose per-stage latency and queue pressure.
- Make diagram projections possible from typed events and edges.
- Support replay and benchmark comparisons.

## 4. System Invariants

These rules should appear near the top of the formal architecture document.

- Every prompt must be splittable into traceable tasks.
- Every task must be buildable or explicitly blocked with a reason.
- Every decomposition must have an integration path.
- Every agent completion must produce a decommission packet.
- Every memory node must point back to evidence.
- Every route decision must identify its route policy, backend, candidate set, and live-state snapshot.
- Every topology update must be versioned and rollbackable.
- Raw logs and decommission packets are append-only.
- Hard constraints are exact.
- Soft relevance state may be projected, quantized, or lossy only when the projection is deterministic and measurable.

## 5. Pre-Architecture Artifacts

The formal architecture document should be assembled from smaller documents. Each one should be useful on its own.

### 5.1 Glossary

Target file:

```text
docs/glossary.md
```

Purpose:

- Define Task Loom, SBJR, route decision, candidate set, topology, gradient space, memory node, decommission packet, integration group, and substitute quality.
- Prevent overloaded terms from drifting while implementation moves quickly.

Done when:

- A new contributor can read the glossary and understand the core vocabulary without reading every design note.

### 5.2 System Boundaries

Target file:

```text
docs/system-boundaries.md
```

Purpose:

- State what Q-TOM owns.
- State what the Task Loom owns.
- State what the runtime, memory, evaluation, governance, and observability layers own.
- Identify boundaries that should become traits, APIs, or event contracts.

Done when:

- It is clear where a new feature belongs before code is written.

### 5.3 Core Entities

Target file:

```text
docs/core-entities.md
```

Purpose:

- Define the durable entity model.
- Start with `Prompt`, `TaskEnvelope`, `PlanNode`, `AgentProfile`, `RouteRequest`, `RouteDecision`, `ArtifactRef`, `SignalRef`, `IntegrationGroup`, `IntegrationReport`, `AgentDecommissionPacket`, `MemoryNode`, `GradientSpace`, `TopologyProposal`, and `TopologySnapshot`.

Done when:

- Each entity has an owner, required IDs, lifecycle state, storage expectation, and lineage requirements.

### 5.4 Event Vocabulary

Target file:

```text
docs/event-vocabulary.md
```

Purpose:

- Define the events that make the loom replayable and inspectable.
- Start with `task_created`, `task_assigned`, `artifact_declared`, `artifact_ready`, `signal_emitted`, `task_blocked`, `task_resumed`, `task_completed`, `agent_decommissioned`, `integration_requested`, `memory_node_created`, `index_updated`, `route_decision_recorded`, `topology_proposed`, and `topology_committed`.

Done when:

- A local simulator can emit these events without inventing additional hidden state.

### 5.5 Lifecycle Flows

Target file:

```text
docs/lifecycle-flows.md
```

Purpose:

- Show the main system flows as diagrams or ordered traces.
- Include root prompt flow, task decomposition flow, constructor execution flow, integration flow, decommission flow, memory curation flow, route-decision flow, and topology-update flow.

Done when:

- The system can be explained from input prompt to remembered execution without skipping a layer.

### 5.6 Topology Governance

Target file:

```text
docs/topology-governance.md
```

Purpose:

- Define how topology changes are proposed, tested, approved, committed, and rolled back.
- Cover new axes, split axes, deprecated axes, new agent profiles, benchmark schema changes, memory index versions, and route policy changes.

Done when:

- A future self-improving or curator-driven topology update cannot silently rewrite the live system.

### 5.7 MVP Roadmap

Target file:

```text
docs/mvp-roadmap.md
```

Purpose:

- Convert the architecture into implementation phases with acceptance criteria.
- Keep the first MVP local, observable, and replayable.

Done when:

- The next engineering steps are small enough to build and test without needing the whole swarm vision at once.

### 5.8 Formal Architecture

Target file:

```text
docs/architecture.md
```

Purpose:

- Consolidate the prior artifacts into the durable system architecture.
- Explain the design to future contributors, reviewers, and operators.

Done when:

- The architecture can guide implementation, benchmark planning, and code review.

## 6. Formal Architecture Outline

The final architecture document should use this shape.

```text
1. Executive Summary
2. Problem Statement
3. Goals And Non-Goals
4. Design Principles
5. System Context
6. Layered Architecture
7. Core Entity Model
8. Event And Storage Model
9. Routing Lifecycle
10. Task Loom Lifecycle
11. Memory Curation Lifecycle
12. Topology Update Lifecycle
13. Replay And Determinism
14. Evaluation And Benchmarking
15. Observability
16. Security And Governance
17. Scaling Strategy
18. MVP Plan
19. Risks And Open Questions
```

## 7. Local Task Loom MVP

The first implementation beyond Q-TOM should be a local Task Loom simulator. It should prove architecture shape before the project commits to real multi-agent runtime complexity.

### 7.1 MVP Constraints

- Local-first.
- Single process at first.
- File-backed or SQLite-backed event log.
- Mock Director, Constructor, Integration, and Curator agents before real LLM workers.
- Q-TOM routes simulated `AgentProfile` records.
- All route decisions and task events are replayable.
- No live topology mutation. Only versioned topology snapshots.

### 7.2 MVP Flow

```text
root prompt
  -> Director mock emits PlanNode and TaskEnvelope records
  -> Q-TOM routes tasks to Constructor profiles
  -> Constructor mocks emit ArtifactRef records
  -> Integration mock joins artifacts by policy
  -> Constructor completion emits AgentDecommissionPacket
  -> Curator mock derives MemoryNode records
  -> MemoryNode records are placed into a versioned GradientSpace
  -> next route can include memory-node candidate retrieval
```

### 7.3 MVP Acceptance Criteria

- A run can be replayed deterministically from event log plus topology snapshot.
- Every task has prompt, plan, route, agent, artifact, integration, and decommission lineage.
- Route telemetry records available candidate, observed ideal candidate, ideal-unavailable flag, substitute distance delta, and fallback status.
- Integration can use at least `wait_all`, `wait_quorum`, and `timeout_then_integrate`.
- Curator output can reduce a memory candidate set before exact Q-TOM scoring.
- The simulator reports deadlocks, blocked tasks, route churn, fallback rate, and join repair requests.
- The system can generate at least task dependency and memory lineage projections from stored events.

## 8. Evidence Gates

Architecture should advance only when the system clears measurable gates.

### Gate 1: Routing Correctness

Required evidence:

- CPU golden fixtures round-trip exactly.
- CUDA `k = 1` route decisions match CPU decisions on deterministic fixtures.
- Unsupported CUDA shapes fail closed.
- Route telemetry explains substitute choices.

Current state:

- Mostly cleared for the narrow `k = 1`, `dims = 16` CUDA path.

### Gate 2: Routing Performance

Required evidence:

- CPU and CUDA p50, p95, p99, max, and routes/sec are recorded for fixed fixtures.
- Reusable CUDA routing is compared against CPU parallel whole-batch routing.
- Candidate count scaling is measured.

Current state:

- The benchmark ledger shows reusable CUDA faster than CPU parallel for the current `8192 agents / 2048 tasks / dims=16 / k=1` fixture.
- Device-side full candidate scanning is now the main bottleneck.

### Gate 3: Candidate-Set Reduction

Required evidence:

- Candidate prefilter recall is measured against exact full-scan CPU routing.
- Hard-constraint violation rate remains zero.
- Candidate reduction improves p99 without unacceptable top-k recall loss.

Current state:

- Naive stacked projection has promising recall but poor retrieval cost.
- Architecture should preserve the multi-view idea while replacing naive grid expansion with curated indexes or shortlist tables.

### Gate 4: Loom Replayability

Required evidence:

- A Task Loom run can be replayed from stored events.
- Every task edge and artifact edge is reconstructable.
- Integration paths are explicit.
- Decommission packets and memory nodes preserve evidence pointers.

Current state:

- Defined conceptually in `docs/agent-task-loom.md`; not implemented yet.

### Gate 5: Real Local Agent Substitution

Required evidence:

- Local Qwen-backed agents with different prompt/tool/MCP/memory profiles show measurable behavioral differences.
- Route proximity predicts acceptable substitution better than random or flat priority routing.
- Ideal-unavailable events degrade gracefully.

Current state:

- Not started.

### Gate 6: Topology Governance

Required evidence:

- New axes, memory indexes, and agent profiles are proposed as versioned artifacts.
- Shadow routing or canary routing compares a proposal against the current topology.
- Rollback is tested.

Current state:

- Design needed before implementation.

## 9. Scaling Path

The architecture should move through these stages.

### Stage A: Single-Machine Router

Already underway.

- CPU truth source.
- CUDA narrow fast path.
- Golden fixtures.
- Benchmark ledger.

### Stage B: Local Task Loom Simulator

Next major milestone.

- Event log.
- Mock role agents.
- Route decisions.
- Decommission packets.
- Memory nodes.
- Diagram projections.

### Stage C: Local LLM Task Loom

Replace mocks with controlled local agents.

- Fixed local `Qwen3-2507` model profile.
- Multiple prompt/tool/MCP/memory profiles.
- LLM-graded benchmark generation.
- Local replay and evaluation loops.

### Stage D: Curated Memory Routing

Make memory curation performance-relevant.

- Curator-maintained candidate sets.
- Versioned `GradientSpace` indexes.
- Exact Q-TOM scoring over compact memory neighborhoods.
- Recall and substitution metrics.

### Stage E: Self-Improving Topology Proposals

Allow the system to propose changes without granting it direct mutation rights.

- Axis proposals.
- Agent-profile proposals.
- Benchmark-rubric proposals.
- Shadow routing.
- Canary promotion.
- Rollback.

### Stage F: Cluster-Capable Swarm

Only after local semantics are proven.

- Multi-worker dispatch.
- Sharded event storage.
- Distributed candidate indexes.
- Backend-specific routing workers.
- Cross-machine observability.
- Cluster-level queue pressure.

## 10. Immediate Work Plan

The next practical sequence is:

1. Write `docs/glossary.md`.
2. Write `docs/system-boundaries.md`.
3. Write `docs/core-entities.md`.
4. Write `docs/event-vocabulary.md`.
5. Implement a minimal event-log crate or module for the Task Loom simulator.
6. Add mock SBJR role agents.
7. Route mock constructor tasks through Q-TOM.
8. Emit decommission packets and memory nodes.
9. Generate task dependency and memory lineage projections.
10. Consolidate the artifacts into `docs/architecture.md`.

The first architecture implementation should not attempt to run a real swarm. It should prove that the data model, events, lineage, route decisions, and replay semantics are correct while the agents are still cheap mocks.

## 11. Open Questions

- What minimum `GradientSpace` axis set should seed memory curation?
- Should the first event log be SQLite, JSONL, or an append-only binary fixture format?
- Which diagram projection should be implemented first: task dependency, signal flow, or memory lineage?
- What route-quality threshold is acceptable before a curated candidate set can skip full-scan fallback?
- How should Integration Agents arbitrate conflicts between equally plausible constructor outputs?
- Which topology changes require human approval in early versions?
- How should budget, permissions, and memory access constraints be represented as exact masks?
- What is the smallest useful real local-agent benchmark that shows prompt/tool/MCP/memory profile differences?

## 12. Design Bias

Prefer boring, inspectable primitives around the novel idea:

- Append-only event logs.
- Versioned topology snapshots.
- Explicit route decisions.
- Deterministic replay.
- Exact masks for hard constraints.
- Lossy projection only for soft relevance.
- Mocks before real agents.
- Golden fixtures before optimization claims.

The project can be wild in what it enables. The architecture should be calm, narrow, and auditable.
