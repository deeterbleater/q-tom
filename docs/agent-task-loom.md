# Agent Task Loom

Q-TOM's larger orchestration model is a massively agentic, async-first task loom. The routing benchmarks validate the low-level principle: ordered candidate spaces matter, and exact scoring becomes more useful when the system can hand it compact, curated neighborhoods. This document describes the next conceptual layer: how swarms create, execute, join, and remember granular work without collapsing into a monolithic context blob.

## SBJR

SBJR is CRUD for agentic swarms:

```text
Split
Build
Join
Remember
```

The roles are:

```text
Director Agents     Split intent into granular async tasks.
Constructor Agents  Build task deliverables.
Integration Agents  Join completed threads into coherent outputs.
Curator Agents      Remember telemetry, decisions, traces, and lessons.
```

The loop is:

```text
prompt
  -> Split into a task graph
  -> Build granular deliverables
  -> Join deliverables into integrated artifacts
  -> Remember the execution
  -> Warm the next related task
```

Core invariants:

- Every prompt must be splittable.
- Every task must be buildable.
- Every decomposition must be joinable.
- Every agent completion must be rememberable.
- Every edge must be traceable to the prompt, plan, task, agent, and artifact that caused it.

## Async-Only Operating Model

Async concurrency is not an optimization. It is the operating model. Any layer that assumes a single prompt, a single linear plan, a single agent, and a single final answer has not been decomposed far enough.

All cross-agent work should be event or message based:

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
```

Tasks are graph nodes, not stack frames. Outputs may be declared before they are ready. Consumers subscribe to readiness instead of blocking the whole system. Synchronous waits are allowed only at explicit join points with a named policy.

Join policies include:

```text
wait_all
wait_quorum
wait_first_viable
timeout_then_integrate
streaming_incremental
human_gate
```

## Director Agents

Director Agents split intent. They can be hierarchical, just like Constructor Agents. A top-level Director Agent classifies the incoming root task and hands it to the Director Agent that best fits the task family. Specialist Director Agents formalize plans, split work into subtasks, and either pass subtasks to more specialized Director Agents or publish granular tasks to orchestration.

Director Agents should produce small planning artifacts:

```text
PlanNode
- plan_id
- root_task_id
- task_id
- director_agent_id
- decomposition_reason_ref
- child_task_ids
- dependency_edges
- integration_group_id
- acceptance_criteria_ref
- risk_flags
```

Director Agents should not hide decomposition inside prose. Every subtask should be represented as a task envelope with lineage IDs and typed dependency edges.

## Constructor Agents

Constructor Agents build. They accept granular tasks, produce deliverable artifacts, emit task events, and decommission into memory curation. Massively agentic systems should push task granularity downward so work can be produced like a swarm-scale manufacturing line.

Constructor completion produces three streams:

```text
deliverable artifact -> Integration Agents
decommission packet  -> Curator Agents
status and edges     -> Loom graph
```

## Integration Agents

Integration Agents are async join operators over the task graph. They collect completed threads from the same root task, reconcile their outputs, and produce integrated artifacts. When they detect gaps, conflicts, or missing evidence, they emit new task requests back to the Director swarm.

Integration Agents are not only summarizers. They validate, reconcile, merge, and decide what remains unresolved.

```text
IntegrationGroup
- integration_group_id
- root_task_id
- parent_task_id
- plan_id
- expected_child_task_ids
- join_policy
- acceptance_criteria_ref
- integration_agent_ids
- status
```

```text
IntegrationReport
- integration_group_id
- included_task_ids
- excluded_task_ids
- conflict_edges
- gap_edges
- repair_task_ids
- final_artifact_refs
- acceptance_status
```

The key invariant is:

```text
Every decomposition must have an integration path.
```

If a Director Agent splits work, the loom must know who or what is responsible for joining it again.

## Curator Agents And Decommission

When an agent finishes, fails, or is decommissioned, it should leave behind a durable packet. The packet is eligible for curation; not every byte automatically becomes long-term memory.

```text
AgentDecommissionPacket
- packet_id
- agent_id
- root_task_id
- task_id
- prompt_id
- plan_id
- final_status
- deliverable_refs
- conversation_log_ref
- tool_trace_refs
- telemetry_refs
- files_touched_refs
- validation_refs
- failure_refs
- self_summary_ref
- open_question_refs
```

Curator Agents transform packets into memory nodes:

```text
EpisodeNode     What happened.
DecisionNode    Why a choice was made.
ArtifactNode    Which file, command, output, or benchmark mattered.
HeuristicNode   What should be reused.
FailureNode     What failed and how it was fixed.
PreferenceNode  What user or project preference was discovered.
OpenLoopNode    What should be continued later.
```

Raw packets remain canonical and append-only. Curated nodes are derived, versioned interpretations with evidence pointers back to raw sources.

## Shared Gradient Spaces

The memory system should not treat the archive as a flat pile. Curator Agents should place memory nodes into ordered gradient spaces so nearby nodes are actually likely to be useful substitutes or companions.

The design goal is:

```text
memory / task / agent data
  -> curators derive ordered features
  -> features form shared spatial manifolds
  -> nearby means meaningfully related
  -> exact Q-TOM scoring adjudicates the final neighborhood
```

Gradient spaces should be agent-readable and human-readable. A shared axis is not a private embedding dimension; it is a named contract.

```text
GradientSpace
- name
- axes
- placement_rubric_ref
- neighborhood_policy
- drift_policy
- version
```

```text
GradientAxis
- axis_id
- name
- low_anchor
- high_anchor
- midpoint_examples
- calibration_examples_ref
- confidence
- version
```

Possible memory axes:

```text
semantic domain
task concreteness
temporal relevance
project or entity affinity
tool or action affinity
user preference salience
emotional or relational salience
confidence and durability
```

Placement should be organic but inspectable:

```text
for each new memory component:
  multiple curator agents vote on shared axes
  aggregate placement by weighted mean
  store disagreement as uncertainty
  preserve evidence spans
  use retrieval outcomes to adjust future confidence
```

Axes evolve through pressure:

```text
high retrieval error + high placement disagreement -> propose new axis
dense incompatible clusters on one axis -> split or add companion axis
low retrieval effect -> deprecate or merge axis
meaning change -> publish a new GradientSpace version
```

## Task Lineage

Every granular task should trace back to the prompt that generated it and forward to the deliverables, integration group, decommission packet, and memory nodes that resulted from it.

```text
TaskEnvelope
- task_id
- root_task_id
- parent_task_id
- prompt_id
- plan_id
- created_by_agent_id
- integration_group_id
- task_kind
- objective_summary
- constraints_ref
- input_refs
- output_refs
- status
- created_at
```

Important IDs:

```text
prompt_id
root_task_id
task_id
parent_task_id
plan_id
integration_group_id
director_agent_id
constructor_agent_id
integration_agent_id
decommission_packet_id
memory_node_ids
```

The graph invariant is:

```text
task -> parent task -> root task -> source prompt
task -> creating Director Agent -> plan artifact
task -> executing Constructor Agent -> deliverable artifact
task -> Integration Agent -> integrated artifact
task -> decommission packet -> curated memory nodes
```

## Minimal I/O For Diagrams

The loom must capture enough metadata to construct class diagrams, signal diagrams, task dependency diagrams, agent handoff diagrams, memory lineage diagrams, and artifact provenance diagrams on the fly. It should not store heavy content unless that content is required for replay, audit, or curation.

Every I/O artifact and signal must be representable as a typed graph edge.

Artifact refs should include:

```text
ArtifactRef
- artifact_id
- artifact_kind
- producer_task_id
- producer_agent_id
- schema_name
- content_hash
- storage_uri
- size_class
- retention_policy
- created_at
```

Signal refs should include:

```text
SignalRef
- signal_id
- source_agent_id
- target_agent_id
- source_task_id
- target_task_id
- root_task_id
- signal_type
- payload_schema
- artifact_refs
- status
- occurred_at
```

Typed edges should be enough to draw:

```text
Task A --produces--> Artifact X
Artifact X --consumed_by--> Task B
Director A --decomposes--> Task C
Task C --assigned_to--> Constructor D
Constructor D --emits--> Signal E
Signal E --triggers--> Task F
Agent G --decommissions_into--> Packet H
Packet H --curated_into--> MemoryNode I
Deliverable J --joined_by--> IntegrationGroup K
IntegrationGroup K --requests_repair--> Task L
```

Minimal telemetry should be graph-grade:

```text
LoomEvent
- event_id
- root_task_id
- task_id
- parent_task_id
- prompt_id
- agent_id
- agent_role
- event_type
- status
- artifact_refs
- occurred_at_ms
- duration_ms
- error_code
```

The default retention rule is:

```text
Store lineage, status, timing, schemas, hashes, and artifact references by default.
Store content only when required for replay, audit, deliverable integration, or memory curation.
Curated memory nodes point back to evidence instead of duplicating the whole evidence.
```

## Diagram Projections

Because the loom is stored as typed nodes and edges, diagrams are query projections rather than separate artifacts.

Class-style diagrams can be generated from:

```text
schema_name
artifact_kind
agent_role
task_kind
edge_type
```

Signal diagrams can be generated from:

```text
SignalRef
LoomEvent
source_agent_id
target_agent_id
event_type
artifact_refs
occurred_at
```

Task dependency diagrams can be generated from:

```text
TaskEnvelope.parent_task_id
PlanNode.child_task_ids
PlanNode.dependency_edges
IntegrationGroup.expected_child_task_ids
IntegrationReport.repair_task_ids
```

Memory lineage diagrams can be generated from:

```text
AgentDecommissionPacket
Curator Agent events
MemoryNode evidence refs
GradientSpace placement versions
retrieval events
```

## Open Design Questions

- What minimum axis set should seed the first `GradientSpace`?
- What recall threshold is acceptable before a curated memory candidate set is trusted without exact full-scan fallback?
- Which loom events need long retention, and which can be rolled up after integration and curation?
- How should Integration Agents arbitrate conflicting deliverables from equally trusted Constructor Agents?
- Which diagram projections are needed first for developer visibility?
- What is the smallest useful decommission packet for memory curation without overcollecting data?
