# Glossary

**Status:** Draft architecture glossary
**Date:** 2026-06-13
**Scope:** Shared vocabulary for Q-TOM, the Agent Task Loom, and the architecture roadmap.

This glossary keeps project terms stable while the architecture moves from routing prototype to local Task Loom MVP. Prefer these terms in docs, code comments, event names, and future architecture records unless a replacement is explicitly proposed.

## Agent

A routable execution profile that can receive a task. In Prototype 1, the local model profile is fixed to `Qwen3-2507`, so agent variation comes from system prompt profile, tool bundle, MCP library set, memory set, benchmarked behavior, and runtime state.

An agent is not only an LLM model. It is the operational package the loom can assign work to.

## Agent Decommission Packet

A durable packet emitted when an agent finishes, fails, or is retired from a task. The packet captures enough evidence for curation without making every byte permanent memory.

Expected contents include:

- agent ID
- root task ID
- task ID
- prompt ID
- plan ID
- final status
- deliverable references
- conversation log reference
- tool trace references
- telemetry references
- validation references
- failure references
- open question references

Raw decommission packets are canonical append-only evidence. Curated memory nodes are derived interpretations that point back to these packets.

## Agent Task Loom

The larger async orchestration system around Q-TOM. The Task Loom owns decomposition, execution, joining, decommission, memory curation, topology evolution, replay, and governance.

The Task Loom treats tasks as graph nodes rather than stack frames. Work is event or message based; synchronous waits are limited to explicit join points with named join policies.

## Artifact Ref

A typed reference to a produced or consumed artifact. An artifact ref should identify the artifact kind, producer task, producer agent, schema name, content hash, storage URI, size class, retention policy, and creation time.

The loom should store artifact references by default and store heavy content only when replay, audit, integration, or curation requires it.

## Candidate Set

The bounded group of agents or memory nodes considered for exact scoring. A candidate set may come from a full scan, a prefilter, a curated memory shortlist, or a topological neighborhood.

Candidate generation may be approximate for soft relevance, but final routing still runs exact Q-TOM scoring over the selected candidates. Hard constraints are exact masks before candidate scoring.

## Constructor Agent

An agent role that builds task deliverables. Constructor Agents accept granular tasks, produce artifacts, emit status events, and decommission into curator-readable evidence.

Constructor completion produces three streams:

- deliverable artifacts for Integration Agents
- decommission packets for Curator Agents
- status and lineage events for the Task Loom graph

## Curator Agent

An agent role that converts decommission packets, traces, artifacts, and telemetry into typed memory nodes. Curator Agents decide what should become reusable memory and where it should live in a shared gradient space.

Curators do not rewrite raw evidence. They create derived, versioned interpretations with evidence pointers.

## Director Agent

An agent role that splits intent into traceable work. Director Agents create planning artifacts, task envelopes, dependency edges, and integration groups.

Director Agents should not hide decomposition inside prose. Every subtask should have IDs, lineage, constraints, expected outputs, and an integration path.

## Gradient Space

A versioned, human-readable and agent-readable topology for placing tasks, agents, artifacts, or memory nodes along named axes.

Unlike a private embedding dimension, a gradient space axis is a contract. It has a name, anchors, examples, confidence, calibration evidence, and a version.

Gradient spaces help the system find meaningful neighborhoods before exact Q-TOM scoring.

## Hard Constraint

A rule that must be preserved exactly. Hard constraints include availability, permissions, required tool access, memory access, model class, budget ceilings, safety boundaries, and data access policy.

Hard constraints are never made fuzzy. They are applied as exact masks before approximate candidate generation or final route scoring.

## Integration Agent

An agent role that joins completed task threads into coherent outputs. Integration Agents validate, reconcile, merge, detect gaps, and request repair tasks when needed.

Integration Agents are not only summarizers. They are async join operators over the Task Loom graph.

## Integration Group

A typed join target for a decomposed set of tasks. An integration group records the root task, parent task, plan, expected child tasks, join policy, acceptance criteria, integration agents, and status.

Every decomposition must have an integration group or another explicit integration path.

## Join Policy

The named rule an Integration Agent uses to decide when and how to join task outputs.

Initial join policies include:

- `wait_all`
- `wait_quorum`
- `wait_first_viable`
- `timeout_then_integrate`
- `streaming_incremental`
- `human_gate`

## Lossy Determinism

A design pattern for deterministic orchestration systems: preserve hard invariants exactly, but apply explicit deterministic projection to soft relevance state so the system can commit under ambiguity without relying on randomness.

In Q-TOM, lossy determinism is a candidate-generation idea, not a replacement for exact top-k scoring.

## Memory Node

A curated, typed memory unit derived from raw evidence. Memory nodes can represent episodes, decisions, artifacts, heuristics, failures, preferences, or open loops.

Memory nodes should include evidence references, type, confidence, durability, placement information, and version metadata. They are candidates for future retrieval and exact scoring.

## Observed Top-K

The nearest candidates before availability filtering. Observed top-k telemetry shows what the router would have preferred semantically if all candidates were routable.

Observed top-k is useful for debugging substitute quality and ideal-unavailable behavior.

## Available Top-K

The ranked candidates that remain after availability and other hard constraints are applied. Production dispatch should use available top-k rather than observed top-k.

## Q-TOM Router

The routing and scoring substrate. Q-TOM scores task vectors against agent or memory candidate vectors, applies runtime state penalties, preserves CPU/CUDA parity, and returns ranked candidates with telemetry.

Q-TOM should stay narrow. It does not decompose prompts, execute LLM calls, own memory policy, or mutate topology.

## Route Decision

The durable record of a routing choice. A route decision should identify the request, backend, route policy, topology or fixture version, live-state snapshot, candidate set, selected candidate, fallback status, ideal-unavailable flag, and score explanation fields.

Route decisions must be replayable against the same fixture and live-state snapshot except for documented floating-point tie cases.

## Route Request

The input envelope sent to Q-TOM for candidate ranking. A route request includes a task ID, task vector, requested `k`, fallback generalist ID, radius threshold, and references to the relevant candidate registry or topology snapshot.

## SBJR

The Task Loom lifecycle:

```text
Split
Build
Join
Remember
```

Director Agents split work. Constructor Agents build deliverables. Integration Agents join completed threads. Curator Agents remember telemetry, decisions, traces, and lessons.

## Signal Ref

A typed record of a cross-agent or cross-task signal. Signal refs should capture source and target agents, source and target tasks, root task, signal type, payload schema, artifact references, status, and timestamp.

Signal refs let the loom project message flow diagrams from stored events.

## Soft Relevance State

Routing context that may be projected, quantized, or approximated as long as the process is deterministic and measurable. Examples include task-vector location, queue pressure, latency pressure, cache pressure, local neighborhood density, semantic affinity, and recency.

Soft relevance can be fuzzy. Hard constraints cannot.

## Substitute Quality

The quality of the selected available candidate when the ideal semantic candidate is unavailable or penalized. Prototype 1 measures substitute quality geometrically:

```text
substitute_distance_delta =
    dist_sq(task, selected_agent) - dist_sq(task, ideal_agent)
```

Lower values are better. A value near zero means the substitute stayed near the intended capability region.

## Task Envelope

The durable task record used by the Task Loom. A task envelope should include task ID, root task ID, parent task ID, prompt ID, plan ID, creating agent, integration group, task kind, objective summary, constraints reference, input references, output references, status, and creation time.

Tasks are graph nodes, not stack frames.

## Top-K

The ordered list of the `k` best candidates under a route policy. Q-TOM distinguishes available top-k from observed top-k so dispatch can stay safe while telemetry can still explain unavailable ideal candidates.

The default prototype value is `k = 8`.

## Topology

The ordered structure that makes proximity meaningful for routing, retrieval, and substitution. In this project, topology includes agent capability spaces, memory gradient spaces, candidate neighborhoods, and versioned route policy context.

Topology should be inspectable and versioned. Topology changes should be proposals before they become committed snapshots.

## Topology Snapshot

A versioned, immutable view of the route-relevant topology. A route decision should be able to identify which topology snapshot it used.

Snapshots make replay, benchmark comparison, shadow routing, canary routing, and rollback possible.

## Topology Proposal

A proposed change to the topology, such as adding an axis, splitting an axis, deprecating an axis, adding an agent profile, changing a benchmark schema, updating memory placement rules, or changing route policy.

Topology proposals should be tested through evaluation, shadow routing, or canary routing before commit.
