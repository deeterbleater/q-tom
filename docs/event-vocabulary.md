# Event Vocabulary

**Status:** Draft event catalog
**Date:** 2026-06-13
**Scope:** Canonical event names and payload expectations for the local Task Loom MVP.

## 1. Purpose

The Agent Task Loom is event-driven. This document defines the initial event vocabulary so the MVP can be replayable, inspectable, and testable without inventing hidden state.

Each event should have:

- **Producer:** The layer or role that emits the event.
- **Consumer:** The layer or role expected to react to it.
- **Payload:** The minimum durable fields.
- **Replay note:** What the event must preserve for deterministic replay or audit.

Events should be append-only. Current status can be kept in projections, but projections must be rebuildable from the event log plus committed topology snapshots.

## 2. Shared Event Envelope

Every event should fit inside a shared envelope.

```text
LoomEvent
- event_id
- event_type
- root_task_id
- task_id
- parent_task_id
- prompt_id
- agent_id
- agent_role
- topology_snapshot_id
- payload_schema
- payload_ref
- occurred_at_ms
- causation_id
- correlation_id
```

`causation_id` points to the event that directly caused this event. `correlation_id` groups events from the same root prompt or replay run.

## 3. task_created

Emitted when a Director Agent or the loom creates a new task envelope.

**Producer:** Agent Task Loom or Director Agent.

**Consumer:** Agent Task Loom, Q-TOM routing request builder, Observability Layer.

**Payload:**

```text
task_id
root_task_id
parent_task_id
prompt_id
plan_id
created_by_agent_id
integration_group_id
task_kind
objective_summary
constraints_ref
input_refs
```

**Replay note:** Must preserve enough task envelope data to rebuild task dependency projections.

## 4. task_assigned

Emitted when the loom assigns a task to an agent after a route decision.

**Producer:** Agent Task Loom.

**Consumer:** Agent Runtime, Observability Layer.

**Payload:**

```text
task_id
agent_profile_id
route_decision_id
assignment_id
assigned_at_ms
```

**Replay note:** Must reference the route decision instead of copying all scoring data.

## 5. artifact_declared

Emitted when an agent declares that an artifact will be produced.

**Producer:** Agent Runtime.

**Consumer:** Agent Task Loom, Integration Agents, Observability Layer.

**Payload:**

```text
artifact_id
artifact_kind
producer_task_id
producer_agent_id
schema_name
expected_storage_uri
retention_policy
```

**Replay note:** Allows consumers to subscribe to readiness before content exists.

## 6. artifact_ready

Emitted when a declared artifact is available for consumption.

**Producer:** Agent Runtime.

**Consumer:** Agent Task Loom, Integration Agents, Curator Agents, Observability Layer.

**Payload:**

```text
artifact_id
content_hash
storage_uri
size_class
created_at_ms
validation_refs
```

**Replay note:** Content should be addressed by hash or immutable URI so replay can detect drift.

## 7. signal_emitted

Emitted when an agent sends a typed signal to another agent or task.

**Producer:** Agent Runtime.

**Consumer:** Agent Task Loom, target agent, Observability Layer.

**Payload:**

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
```

**Replay note:** Must preserve signal causality for signal-flow diagrams.

## 8. task_blocked

Emitted when a task cannot continue.

**Producer:** Agent Task Loom, Agent Runtime, Integration Agent.

**Consumer:** Agent Task Loom, Director Agents, Observability Layer.

**Payload:**

```text
task_id
blocked_reason
blocked_by_ref
retry_policy_ref
repair_request_ref
```

**Replay note:** Must distinguish waiting, failed prerequisites, unavailable agents, policy stops, and missing evidence.

## 9. task_resumed

Emitted when a blocked task becomes runnable again.

**Producer:** Agent Task Loom.

**Consumer:** Agent Runtime, Observability Layer.

**Payload:**

```text
task_id
prior_blocked_event_id
resume_reason
resume_ref
```

**Replay note:** Must link to the blocked event it resolves.

## 10. task_completed

Emitted when a task reaches a terminal successful state.

**Producer:** Agent Runtime or Agent Task Loom.

**Consumer:** Integration Agents, Curator Agents, Observability Layer.

**Payload:**

```text
task_id
agent_profile_id
output_refs
duration_ms
completion_status
validation_refs
```

**Replay note:** Completion does not imply integration. Integration has its own event path.

## 11. agent_decommissioned

Emitted when an agent finishes, fails, or is retired from task execution.

**Producer:** Agent Runtime.

**Consumer:** Curator Agents, Agent Task Loom, Observability Layer.

**Payload:**

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
validation_refs
failure_refs
open_question_refs
```

**Replay note:** The decommission packet is canonical evidence for memory curation. Large logs should be referenced, not copied.

## 12. integration_requested

Emitted when an integration group is ready or required to attempt a join.

**Producer:** Agent Task Loom.

**Consumer:** Integration Agents, Observability Layer.

**Payload:**

```text
integration_group_id
root_task_id
parent_task_id
plan_id
expected_child_task_ids
join_policy
acceptance_criteria_ref
trigger_reason
```

**Replay note:** Must preserve the named join policy used for the integration attempt.

## 13. memory_node_created

Emitted when a Curator Agent creates a derived memory node.

**Producer:** Curator Agent.

**Consumer:** Memory indexer, Observability Layer, Governance Layer for topology-relevant changes.

**Payload:**

```text
memory_node_id
memory_kind
summary_ref
evidence_refs
source_packet_ids
source_artifact_ids
confidence
durability
version
```

**Replay note:** Memory nodes must point back to evidence so curation can be audited or rebuilt.

## 14. index_updated

Emitted when a memory index or gradient space placement index changes.

**Producer:** Memory and Curator Layer.

**Consumer:** Q-TOM candidate providers, Observability Layer, Governance Layer.

**Payload:**

```text
index_id
index_kind
previous_version
new_version
changed_node_ids
gradient_space_id
topology_snapshot_id
```

**Replay note:** Route decisions should identify which index version was used.

## 15. route_decision_recorded

Emitted when Q-TOM records a route decision.

**Producer:** Q-TOM Router.

**Consumer:** Agent Task Loom, Observability Layer, Evaluation Layer.

**Payload:**

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
```

**Replay note:** Must reference route policy, topology snapshot, and live-state snapshot so deterministic replay can explain the choice.

## 16. topology_proposed

Emitted when a layer proposes a topology change.

**Producer:** Governance Layer, Curator Agent, Evaluation Layer, or operator.

**Consumer:** Governance Layer, Evaluation Layer, Observability Layer.

**Payload:**

```text
topology_proposal_id
proposal_kind
proposer_ref
change_set_ref
evidence_refs
benchmark_report_refs
status
```

**Replay note:** A proposal is not a commit. It must not affect live routing until promoted into a topology snapshot.

## 17. topology_committed

Emitted when governance commits a topology proposal into an immutable snapshot.

**Producer:** Governance Layer.

**Consumer:** Q-TOM Router, Agent Task Loom, Memory and Curator Layer, Observability Layer.

**Payload:**

```text
topology_snapshot_id
source_proposal_id
parent_snapshot_id
agent_registry_version
gradient_space_versions
memory_index_versions
route_policy_versions
hard_constraint_policy_version
committed_by_ref
```

**Replay note:** Route decisions after this event may reference the new topology snapshot. Older route decisions remain tied to their original snapshot.

## 18. topology_rolled_back

Emitted when governance rolls future routing back from a failed topology snapshot to a previous known-good snapshot.

**Producer:** Governance Layer.

**Consumer:** Q-TOM Router, Agent Task Loom, Memory and Curator Layer, Observability Layer.

**Payload:**

```text
rollback_id
from_topology_snapshot_id
to_topology_snapshot_id
reason
triggered_by_ref
affected_route_decision_refs
```

**Replay note:** Must reference the restored `topology_snapshot_id` and be caused by a prior `topology_committed` event. Rollback changes future routing truth but does not delete route decisions made under the rolled-back snapshot.

## 19. Event Ordering Rules

- `task_assigned` requires a prior `route_decision_recorded`.
- `artifact_ready` requires a prior `artifact_declared`.
- `task_resumed` requires a prior `task_blocked`.
- `integration_requested` requires an `IntegrationGroup`.
- `memory_node_created` requires at least one evidence reference.
- `index_updated` requires a version change.
- `topology_committed` requires a prior `topology_proposed`.
- `topology_rolled_back` requires a prior `topology_committed`.

These rules should become simulator assertions before real agents are introduced.

## 20. Replay Requirements

A replay engine should be able to reconstruct:

- task dependency graph
- route decisions and selected agents
- observed ideal-unavailable cases
- artifact provenance
- signal flow
- integration attempts
- decommission packet lineage
- memory node lineage
- topology snapshot usage

Replay does not need to re-run LLM inference by default. It must reconstruct the event graph and detect when referenced content, topology, route policy, or live-state snapshots are missing.

## 21. Open Event Questions

- Should `task_failed` be separate from `task_completed` with failed status?
- Should topology shadow and canary events exist in the first MVP or wait for governance implementation?
- Should memory candidate retrieval have a first-class `memory_candidates_proposed` event?
- Should route requests be first-class events or only embedded in route decision records?
- Which events need monotonic sequence numbers in addition to timestamps?
