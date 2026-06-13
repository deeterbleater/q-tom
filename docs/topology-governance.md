# Topology Governance

**Status:** Draft governance policy
**Date:** 2026-06-13
**Scope:** How route-relevant topology changes are proposed, tested, approved, committed, and rolled back.

## 1. Purpose

The long-term system should be able to improve its own routing topology, memory indexes, benchmark schemas, and agent profile map. That power needs a hard governance boundary. Curator Agents, Evaluation Agents, and future self-improvement loops may propose changes, but they must not silently rewrite live topology.

The governance rule is:

```text
No proposal becomes routing truth until it is evaluated, versioned, committed, and rollbackable.
```

## 2. Governed Change Classes

Topology governance covers any change that can alter routing, retrieval, memory placement, benchmark-derived capability vectors, or hard-constraint behavior.

### 2.1 New Axes

New axes add a dimension to a `GradientSpace`.

Required evidence:

- placement rubric
- low and high anchors
- midpoint examples
- calibration examples
- expected retrieval or routing effect
- migration plan for existing nodes

Approval concern:

New axes can improve candidate quality, but they can also create artificial precision. They require evidence that the axis changes retrieval or routing outcomes, not only that it sounds meaningful.

### 2.2 Split Axes

Split axes replace one overloaded axis with two or more clearer axes.

Required evidence:

- high placement disagreement on the original axis
- dense incompatible clusters
- retrieval errors linked to the overloaded axis
- backfill strategy
- comparison against the unsplit version

Approval concern:

Axis splits should reduce ambiguity without making placement too expensive or too subjective.

### 2.3 Deprecated Axes

Deprecated axes remove or retire a low-value axis.

Required evidence:

- low retrieval effect
- low route-quality impact
- replacement axis or migration strategy when needed
- rollback plan for old placements

Approval concern:

Deprecation must not orphan memory nodes or make old route decisions unreplayable.

### 2.4 New Agent Profiles

New agent profiles add routable execution profiles to the registry.

Required evidence:

- prompt/tool/MCP/memory profile definition
- benchmark records
- capability vector
- runtime limits
- permission envelope
- expected substitute neighborhood

Approval concern:

Agent profiles are not live just because they exist. They must pass benchmark and policy checks before entering an active topology snapshot.

### 2.5 Benchmark Schema Changes

Benchmark schema changes alter how agent capability vectors are generated or interpreted.

Required evidence:

- old schema version
- new schema version
- migration or re-evaluation plan
- evaluator config
- rubric version
- comparison report

Approval concern:

Changing the schema changes the map. Existing route decisions must remain tied to their original schema and topology snapshot.

### 2.6 Memory Index Versions

Memory index versions change how memory nodes are placed, retrieved, or shortlisted.

Required evidence:

- index version
- source memory node set
- candidate recall against oracle set
- retrieval latency
- hard-constraint violation rate
- storage and rebuild plan

Approval concern:

Memory index changes must preserve evidence lineage and never bypass exact hard constraints.

### 2.7 Route Policy Changes

Route policy changes alter scoring weights, fallback thresholds, candidate budgets, radius policy, or lossy prefilter behavior.

Required evidence:

- policy version
- benchmark fixture comparison
- top-k overlap
- ideal-unavailable agreement
- substitute distance delta
- p50/p95/p99/max latency
- fallback rate

Approval concern:

Route policy changes should improve measured behavior without hiding quality loss behind faster routing.

## 3. Proposal Lifecycle

Every governed change moves through the same lifecycle.

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

Small local MVP changes may skip `canaried`, but they should not skip `proposed`, `tested`, `approved`, `committed`, or rollback planning.

## 4. Proposal Record

A `TopologyProposal` should include:

```text
topology_proposal_id
proposal_kind
proposer_ref
change_set_ref
affected_entities
affected_hard_constraints
expected_effect
evidence_refs
benchmark_report_refs
shadow_report_refs
canary_report_refs
approval_refs
rollback_plan_ref
status
created_at_ms
updated_at_ms
```

The proposal must be immutable after approval. Corrections create a new proposal version.

## 5. Testing Requirements

Testing happens before approval.

Minimum tests:

- shape validation
- hard-constraint validation
- fixture replay
- route-quality comparison
- latency comparison
- observability comparison
- rollback dry run

For lossy candidate generation, testing must also report:

- candidate recall against exact full scan
- top-k overlap
- ideal-unavailable flag agreement
- substitute distance delta
- scanned-candidate reduction
- hard-constraint violation rate

The hard-constraint violation rate must be zero.

## 6. Shadow Routing

Shadow routing compares a proposed topology against the active topology without affecting dispatch.

Process:

1. Keep the active topology snapshot as routing truth.
2. Run the same route requests against the proposed topology.
3. Record selected candidate deltas.
4. Record top-k overlap.
5. Record fallback changes.
6. Record ideal-unavailable changes.
7. Record substitute distance changes.
8. Record latency and candidate-count changes.

Shadow routing output:

```text
ShadowRoutingReport
- active_topology_snapshot_id
- proposed_topology_snapshot_id
- request_set_ref
- selected_candidate_delta_rate
- top_k_overlap
- fallback_delta_rate
- ideal_unavailable_delta_rate
- substitute_distance_delta_summary
- latency_summary
- hard_constraint_violations
```

Shadow routing is required before canary or commit for any route-affecting proposal.

## 7. Canary Routing

Canary routing sends a bounded amount of real or simulator traffic through a proposed topology.

Canary constraints:

- bounded request percentage or fixed run count
- automatic rollback trigger
- observability enabled
- active topology remains available
- no hard-constraint bypass
- clear stop condition

Canary output:

```text
CanaryReport
- proposed_topology_snapshot_id
- traffic_scope
- duration
- route_quality_summary
- fallback_rate
- blocked_task_rate
- repair_task_rate
- latency_summary
- rollback_triggered
- operator_notes_ref
```

The first local MVP can use simulated canary runs before live traffic exists.

## 8. Approval

Approval records explain why a proposal may become active.

An `ApprovalRecord` should include:

```text
approval_id
topology_proposal_id
approver_ref
approval_basis
required_reports
known_risks
rollback_plan_ref
approved_at_ms
```

Early versions can use human approval only. Later versions may allow policy approval for low-risk proposals, but policy approvals must still write records.

## 9. Commit

Commit converts an approved proposal into an immutable `TopologySnapshot`.

Commit rules:

- A committed snapshot receives a new `topology_snapshot_id`.
- The snapshot records its parent snapshot.
- The snapshot records source proposal ID.
- The snapshot records agent registry version, gradient space versions, memory index versions, route policy versions, and hard-constraint policy version.
- Future route decisions may reference the new snapshot.
- Old route decisions remain tied to their original snapshots.

Commit event:

```text
topology_committed
```

Commit output:

```text
TopologySnapshot
- topology_snapshot_id
- parent_snapshot_id
- source_proposal_id
- agent_registry_version
- gradient_space_versions
- memory_index_versions
- route_policy_versions
- hard_constraint_policy_version
- status
- created_at_ms
```

## 10. Rollback

Rollback moves routing back to an earlier topology snapshot.

Rollback must be possible when:

- route quality regresses
- fallback rate spikes
- hard-constraint behavior is suspect
- latency exceeds threshold
- memory retrieval recall falls below threshold
- integration repair tasks increase unexpectedly

Rollback process:

1. Select previous known-good topology snapshot.
2. Emit rollback record.
3. Mark failed snapshot as rolled back or superseded.
4. Route future requests against the previous snapshot.
5. Preserve all route decisions made under the rolled-back snapshot for audit.

Rollback output:

```text
RollbackRecord
- rollback_id
- from_topology_snapshot_id
- to_topology_snapshot_id
- reason
- triggered_by_ref
- affected_route_decision_refs
- created_at_ms
```

Rollback does not delete history. It changes which snapshot future route decisions reference.

## 11. Versioning Rules

Version everything that can change route behavior:

- agent registry
- capability vector schema
- benchmark rubric
- evaluator config
- route policy
- hard-constraint policy
- gradient space
- memory index
- topology snapshot

Route decisions must record enough version references to explain why a candidate was selected at that time.

## 12. Hard-Constraint Policy

Hard constraints include:

- availability
- permissions
- required tool access
- memory access
- model class
- budget ceilings
- safety boundaries
- data access policy

Hard constraints must be evaluated before approximate candidate generation can remove or rank candidates. Any proposal that changes hard-constraint behavior requires explicit approval and focused testing.

## 13. Governance Events

Initial governance events:

```text
topology_proposed
topology_shadowed
topology_canaried
topology_committed
topology_rolled_back
```

Only `topology_proposed` and `topology_committed` are required by the first event vocabulary. The others should be added when the simulator begins modeling governance workflows.

## 14. MVP Governance

The local MVP should implement the smallest useful governance loop:

1. Create topology proposal record.
2. Run fixture comparison or simulated shadow routing.
3. Approve manually in a record.
4. Commit a new topology snapshot.
5. Route future simulated tasks against that snapshot.
6. Roll back to the parent snapshot.

The MVP does not need autonomous approval. It does need immutable records and replayable snapshot references.

## 15. Open Governance Questions

- What route-quality delta requires human approval rather than policy approval?
- Which proposal kinds can skip canary in local development?
- Should memory-only index updates commit through the same topology snapshot path as route policy updates?
- How much shadow-routing traffic is enough before cluster-scale deployment?
- Should rollback triggers be static thresholds or evaluator-proposed policies?
- How should rejected proposals be retained and searched for future learning?
