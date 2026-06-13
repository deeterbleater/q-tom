# Core Entities

**Status:** Draft entity model
**Date:** 2026-06-13
**Scope:** Durable objects shared by Q-TOM, the Agent Task Loom, memory curation, evaluation, governance, and observability.

## 1. Purpose

This document defines the first durable entity model for the local Task Loom MVP. It is intentionally schema-like without committing the project to a database engine yet.

Every entity below includes:

- **Owner:** The layer that owns lifecycle authority.
- **Lifecycle:** The expected state changes.
- **Storage:** The persistence expectation.
- **Lineage:** The IDs or references needed for replay, audit, and diagram projections.

The near-term implementation can start with JSONL or SQLite, but the entity contracts should survive that storage choice.

## 2. Entity Summary

```text
Prompt
  -> PlanNode
  -> TaskEnvelope
  -> RouteRequest
  -> RouteDecision
  -> AgentProfile
  -> ArtifactRef / SignalRef
  -> IntegrationGroup
  -> IntegrationReport
  -> AgentDecommissionPacket
  -> MemoryNode
  -> GradientSpace
  -> TopologyProposal
  -> TopologySnapshot
```

## 3. Prompt

The root input that starts a loom run.

**Owner:** Agent Task Loom.

**Lifecycle:** `received -> normalized -> decomposed -> integrated -> remembered`.

**Storage:** Durable event-log record plus optional content reference. Raw prompt content may be stored directly for local development, but production should allow a content-addressed reference.

**Lineage:** `prompt_id`, user/session reference if available, root task ID, creation timestamp, source channel, prompt content hash, prompt content reference.

Required fields:

```text
prompt_id
source_ref
content_hash
content_ref
root_task_id
created_at_ms
```

## 4. PlanNode

A planning artifact emitted by a Director Agent.

**Owner:** Agent Task Loom.

**Lifecycle:** `proposed -> expanded -> assigned -> integrated -> superseded`.

**Storage:** Durable event-log record. Plan revisions should create new versions rather than mutating older records.

**Lineage:** `plan_id`, prompt ID, root task ID, parent task ID, director agent ID, decomposition reason reference, child task IDs, dependency edges, integration group ID, acceptance criteria reference.

Required fields:

```text
plan_id
prompt_id
root_task_id
parent_task_id
director_agent_id
decomposition_reason_ref
child_task_ids
dependency_edges
integration_group_id
acceptance_criteria_ref
risk_flags
version
```

## 5. TaskEnvelope

The durable task record used by the Task Loom.

**Owner:** Agent Task Loom.

**Lifecycle:** `created -> routable -> assigned -> running -> blocked|completed|failed -> integrated -> remembered`.

**Storage:** Durable event-log record plus status projection. The event log is canonical; status tables are query accelerators.

**Lineage:** Prompt ID, root task ID, parent task ID, plan ID, creating Director Agent, assigned Constructor Agent, integration group, input artifacts, output artifacts, route decision, decommission packet, memory nodes.

Required fields:

```text
task_id
root_task_id
parent_task_id
prompt_id
plan_id
created_by_agent_id
assigned_agent_id
integration_group_id
task_kind
objective_summary
constraints_ref
input_refs
output_refs
route_decision_id
status
created_at_ms
updated_at_ms
```

## 6. AgentProfile

A routable execution profile.

**Owner:** Evaluation Layer for benchmark-derived vector changes; Governance Layer for committed profile versions; Agent Runtime for executable bindings.

**Lifecycle:** `proposed -> benchmarked -> approved -> active -> deprecated -> archived`.

**Storage:** Versioned registry record. Active route tables are packed projections derived from committed agent profiles.

**Lineage:** Agent profile ID, model profile, prompt profile, tool bundle profile, MCP library profile, memory profile, benchmark records, vector schema version, topology snapshot ID.

Required fields:

```text
agent_profile_id
model_profile
system_prompt_profile
tool_bundle_profile
mcp_library_profile
memory_set_profile
capability_vector
vector_schema_version
benchmark_record_refs
runtime_limits_ref
status
version
```

## 7. RouteRequest

The input envelope sent to Q-TOM for ranking candidates.

**Owner:** Q-TOM Router owns validation and scoring semantics; Agent Task Loom owns request creation.

**Lifecycle:** `created -> validated -> scored -> recorded`.

**Storage:** Durable when linked to a task assignment. Benchmark-only requests may live in fixture files.

**Lineage:** Task ID, route policy ID, topology snapshot ID, candidate registry reference, live-state snapshot reference, fallback generalist ID.

Required fields:

```text
route_request_id
task_id
task_vector
k
fallback_generalist_id
radius_max_threshold
route_policy_id
candidate_registry_ref
live_state_snapshot_ref
topology_snapshot_id
created_at_ms
```

## 8. RouteDecision

The durable record of a routing choice.

**Owner:** Q-TOM Router owns scoring facts; Agent Task Loom owns dispatch based on the decision; Observability owns projections.

**Lifecycle:** `recorded -> dispatched -> replayed|audited`.

**Storage:** Durable event-log record. Route decisions are required for replay and substitute-quality analysis.

**Lineage:** Route request ID, task ID, selected agent ID, available top-k, observed top-k debug reference, route backend, policy version, topology snapshot, live-state snapshot, fallback status.

Required fields:

```text
route_decision_id
route_request_id
task_id
selected_candidate_id
available_candidates
observed_candidates_ref
backend
route_policy_id
topology_snapshot_id
live_state_snapshot_ref
used_fallback
ideal_candidate_unavailable
substitute_distance_delta
score_explanation_ref
recorded_at_ms
```

## 9. ArtifactRef

A typed reference to produced or consumed content.

**Owner:** Agent Runtime creates artifacts; Agent Task Loom owns graph links; Observability owns projections.

**Lifecycle:** `declared -> ready -> consumed -> retained|expired`.

**Storage:** Durable reference by default. Heavy content lives behind `storage_uri` unless replay or audit requires inline content.

**Lineage:** Producer task ID, producer agent ID, schema name, content hash, storage URI, consuming task IDs, integration report IDs.

Required fields:

```text
artifact_id
artifact_kind
producer_task_id
producer_agent_id
schema_name
content_hash
storage_uri
size_class
retention_policy
created_at_ms
```

## 10. SignalRef

A typed cross-agent or cross-task signal record.

**Owner:** Agent Runtime emits signals; Agent Task Loom owns routing of signal consequences; Observability owns signal-flow projections.

**Lifecycle:** `emitted -> delivered -> handled|expired`.

**Storage:** Durable event-log record for signals that affect task graph state. Ephemeral runtime chatter can be summarized.

**Lineage:** Source agent, target agent, source task, target task, root task, signal type, artifact references, status, timestamp.

Required fields:

```text
signal_id
source_agent_id
target_agent_id
source_task_id
target_task_id
root_task_id
signal_type
payload_schema
artifact_refs
status
occurred_at_ms
```

## 11. IntegrationGroup

The join target for a decomposed set of tasks.

**Owner:** Agent Task Loom.

**Lifecycle:** `created -> waiting -> integrating -> accepted|repair_requested|failed`.

**Storage:** Durable event-log record plus status projection.

**Lineage:** Root task ID, parent task ID, plan ID, expected child tasks, join policy, acceptance criteria, integration agent IDs.

Required fields:

```text
integration_group_id
root_task_id
parent_task_id
plan_id
expected_child_task_ids
join_policy
acceptance_criteria_ref
integration_agent_ids
status
created_at_ms
updated_at_ms
```

## 12. IntegrationReport

The result of an Integration Agent joining task outputs.

**Owner:** Agent Task Loom.

**Lifecycle:** `drafted -> accepted|repair_requested|superseded`.

**Storage:** Durable artifact-like record with references to included outputs.

**Lineage:** Integration group ID, included tasks, excluded tasks, conflict edges, gap edges, repair tasks, final artifact references, acceptance status.

Required fields:

```text
integration_report_id
integration_group_id
included_task_ids
excluded_task_ids
conflict_edges
gap_edges
repair_task_ids
final_artifact_refs
acceptance_status
created_at_ms
```

## 13. AgentDecommissionPacket

The evidence bundle emitted when an agent completes, fails, or is retired from a task.

**Owner:** Agent Runtime emits it; Memory and Curator Layer ingests it; Agent Task Loom requires it.

**Lifecycle:** `emitted -> ingested -> curated -> retained|rolled_up`.

**Storage:** Append-only canonical evidence. Large logs are referenced rather than duplicated.

**Lineage:** Agent ID, task ID, root task ID, prompt ID, plan ID, deliverables, traces, telemetry, validation, failures, open questions.

Required fields:

```text
packet_id
agent_id
root_task_id
task_id
prompt_id
plan_id
final_status
deliverable_refs
conversation_log_ref
tool_trace_refs
telemetry_refs
files_touched_refs
validation_refs
failure_refs
self_summary_ref
open_question_refs
created_at_ms
```

## 14. MemoryNode

A curated memory unit derived from raw evidence.

**Owner:** Memory and Curator Layer.

**Lifecycle:** `proposed -> placed -> retrieved -> reinforced|decayed|deprecated`.

**Storage:** Versioned derived record with evidence references. Memory nodes should be safe to rebuild from canonical packets and artifacts.

**Lineage:** Source decommission packet IDs, source artifact IDs, evidence spans, memory type, placement records, confidence, durability, gradient space version.

Required fields:

```text
memory_node_id
memory_kind
summary
evidence_refs
source_packet_ids
source_artifact_ids
placement_refs
confidence
durability
status
version
created_at_ms
```

## 15. GradientSpace

A versioned topology for placing tasks, agents, artifacts, or memory nodes along named axes.

**Owner:** Memory and Curator Layer proposes; Governance Layer commits.

**Lifecycle:** `drafted -> calibrated -> active -> revised|deprecated`.

**Storage:** Versioned topology artifact. Active route and memory indexes reference a specific version.

**Lineage:** Axis definitions, placement rubric, calibration examples, neighborhood policy, drift policy, prior version, proposal ID, topology snapshot ID.

Required fields:

```text
gradient_space_id
name
axes
placement_rubric_ref
calibration_examples_ref
neighborhood_policy
drift_policy
status
version
created_at_ms
```

## 16. TopologyProposal

A proposed change to route-relevant topology.

**Owner:** Governance Layer.

**Lifecycle:** `drafted -> evaluated -> shadowed -> canaried -> committed|rejected|superseded`.

**Storage:** Durable governance record with evidence references and decision history.

**Lineage:** Proposed change set, proposer, evidence references, benchmark reports, shadow routing reports, canary reports, approval records, resulting topology snapshot.

Required fields:

```text
topology_proposal_id
proposal_kind
proposer_ref
change_set_ref
evidence_refs
benchmark_report_refs
shadow_report_refs
canary_report_refs
approval_refs
status
created_at_ms
updated_at_ms
```

## 17. TopologySnapshot

An immutable committed view of route-relevant topology.

**Owner:** Governance Layer.

**Lifecycle:** `created -> active -> pinned|superseded|rolled_back`.

**Storage:** Durable immutable topology artifact. Route decisions reference the snapshot they used.

**Lineage:** Included gradient spaces, agent registry version, memory index versions, route policies, hard-constraint policy version, source proposal ID, parent snapshot ID.

Required fields:

```text
topology_snapshot_id
parent_snapshot_id
source_proposal_id
agent_registry_version
gradient_space_versions
memory_index_versions
route_policy_versions
hard_constraint_policy_version
status
created_at_ms
```

## 18. Lineage Requirements

The minimal graph should support these traversals:

```text
Prompt -> PlanNode -> TaskEnvelope
TaskEnvelope -> RouteRequest -> RouteDecision -> AgentProfile
TaskEnvelope -> ArtifactRef -> IntegrationGroup -> IntegrationReport
TaskEnvelope -> AgentDecommissionPacket -> MemoryNode -> GradientSpace
TopologyProposal -> TopologySnapshot -> RouteDecision
```

Every MVP event should carry enough IDs to reconstruct the task dependency graph, route trace, artifact provenance graph, and memory lineage graph.

## 19. Storage Guidance

The first local MVP can use either JSONL or SQLite. The recommended split is:

```text
append-only events     canonical history
status projections     query acceleration
artifact store         heavy content by reference
fixture files          benchmark and route parity data
topology snapshots     immutable committed architecture state
```

Do not let query projections become the source of truth. They are rebuildable from the event log plus committed topology snapshots.

## 20. Open Schema Questions

- Should `RouteDecision` store full observed top-k inline or by reference only?
- Should `Prompt` content be inline during local development or always content-addressed?
- Which entity should own budget constraints: `TaskEnvelope`, `RoutePolicy`, or a separate policy record?
- Should `MemoryNode` placement disagreement be part of the node or a separate `PlacementRecord` entity?
- Which lifecycle states should be enums in the first implementation?
- What is the smallest `TopologySnapshot` that still supports route replay?
