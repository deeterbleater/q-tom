# Architecture Realization Plan

**Status:** Draft execution plan
**Date:** 2026-06-13
**Scope:** Practical steps for turning the Q-TOM routing prototype and Agent Task Loom concept into a formal, buildable architecture.

## 1. Purpose

The project is now past the "is this coherent?" stage. The next risk is different: turning a promising topology-aware routing prototype into a system architecture without letting the vision outrun the evidence.

This document defines the path from the current local prototype to a formal architecture that can guide implementation, testing, benchmarking, and future cluster work.

The operating rule is:

```text
Architecture follows evidence, but evidence must be shaped by architecture.
```

That means Q-TOM should keep producing hard measurements, while the Agent Task Loom should define the contracts those measurements need to support.

## 2. Current Position

The repository now has three useful foundations:

- A CPU-first routing core with deterministic fixtures and route telemetry.
- A narrow CUDA path that is gated by CPU parity and benchmark evidence.
- A draft Agent Task Loom model using Split, Build, Join, Remember.

The strongest architectural thesis is:

```text
Agent orchestration should be modeled as topology-aware, replayable, async scheduling.
```

Q-TOM is the scoring substrate. The Task Loom is the system that decides what needs to be routed, when work can proceed, when work must join, and what gets remembered.

## 3. Definition Of "Real"

The system should not be considered real just because it can call agents. It should be considered real when it can prove these properties locally:

- A prompt becomes an explicit task graph.
- Every task assignment is caused by a recorded route decision.
- Every completed task produces artifact and decommission evidence.
- Every integration point has a named join policy.
- Every memory node points back to raw evidence.
- Every topology change is proposed before it is committed.
- A run can be replayed without hidden runtime state.
- Benchmarks can compare route quality, latency, fallback behavior, and final output quality across versions.

This gives the architecture a concrete target: not "swarm intelligence" in the abstract, but replayable local orchestration with measurable substitution behavior.

## 4. Work Tracks

The project should move on four tracks in parallel, but only one track should be allowed to define live behavior at a time.

### 4.1 Routing Track

Purpose:

- Keep Q-TOM exact, narrow, and measurable.

Immediate work:

- Finish assignment flow through route-decision events.
- Keep CPU routing as the correctness oracle.
- Continue CUDA only where parity and benchmark fixtures exist.
- Measure candidate-set reduction before adding more GPU complexity.

Architecture output:

- Backend boundary.
- Route decision schema.
- Candidate-set contract.
- Substitute-quality metric definitions.

### 4.2 Loom Track

Purpose:

- Prove that SBJR can run as a replayable event-driven lifecycle.

Immediate work:

- Keep using mock Director, Constructor, Integration, and Curator roles.
- Route constructor tasks through Q-TOM.
- Emit assignment, artifact, decommission, integration, and memory events from the same run.
- Add diagram projections from events.

Architecture output:

- Task graph contract.
- Join-policy contract.
- Replay validation rules.
- Runtime role boundaries.

### 4.3 Memory Track

Purpose:

- Turn Remember into useful retrieval, not just archival logging.

Immediate work:

- Treat decommission packets as canonical raw evidence.
- Derive typed memory nodes from packets.
- Add a minimal versioned `GradientSpace`.
- Use memory placement to produce compact candidate sets for exact Q-TOM scoring.

Architecture output:

- Memory-node schema.
- Evidence-ref requirements.
- Gradient-space versioning rules.
- Recall and placement-disagreement metrics.

### 4.4 Governance Track

Purpose:

- Prevent self-improving topology from silently mutating the system.

Immediate work:

- Keep topology changes as versioned proposals.
- Define proposal types for axes, route policies, agent profiles, memory indexes, and benchmark rubrics.
- Add shadow-routing and rollback expectations before any live topology mutation exists.

Architecture output:

- Topology proposal schema.
- Commit and rollback rules.
- Shadow-routing gate.
- Human approval or policy approval boundary.

## 5. Sequence To Formal Architecture

The formal architecture document should be assembled after these steps, not guessed up front.

1. Stabilize the vocabulary.
   Keep `docs/glossary.md` current as code introduces real terms.

2. Prove replayable routing inside the loom.
   A `task_assigned` event must reference a `route_decision_recorded` event. This prevents routing from becoming invisible scheduler state.

3. Prove complete SBJR lineage.
   A single run should show prompt, plan, child tasks, assignments, artifacts, integration, decommission packets, and memory nodes.

4. Add projections.
   Generate at least task dependency, route trace, artifact provenance, and memory lineage views from events only.

5. Add validation gates.
   Broken logs should fail with specific errors: missing route decision, missing integration path, missing decommission packet, missing memory evidence, or impossible event ordering.

6. Add the first memory candidate loop.
   Memory nodes should produce a compact candidate set that is then scored exactly by Q-TOM.

7. Add evaluator scaffolding.
   Keep GPT-5.5 Medium as configuration for LLM grading, and version evaluator prompts, rubrics, model IDs, and scoring schemas.

8. Promote `docs/architecture.md` from draft to working architecture.
   The promotion should cite the evidence gates that have passed and list the gates that remain unproven.

## 6. Evidence Gates

Each gate should have tests or benchmark artifacts before it becomes an architectural assumption.

| Gate | Question | Evidence |
| --- | --- | --- |
| Route parity | Does the backend return the same decisions as CPU truth? | Golden fixture tests and parity reports |
| Route explainability | Can an assignment explain why that agent was selected? | `route_decision_recorded` plus assignment causation |
| Loom replay | Can the run be reconstructed from events? | Replay validation over complete mock SBJR logs |
| Join correctness | Does every split have an integration path? | Integration-group validation |
| Memory evidence | Does every memory node point to raw evidence? | Decommission-to-memory validation |
| Candidate reduction | Can fuzzy projection reduce exact scan size without losing too much recall? | Recall, p99 latency, and hard-mask violation metrics |
| Local agent usefulness | Do local agent profiles differ enough to route meaningfully? | Qwen-backed benchmark scores across prompt/tool/MCP/memory profiles |
| Governance safety | Can topology changes be tested before promotion? | Proposal, shadow-routing, canary, and rollback fixtures |

## 7. Near-Term Build Order

The next concrete work should be:

1. Finish Phase 2 assignment flow through Q-TOM.
2. Add a complete route trace projection.
3. Add event-log validation for assignment causation and route telemetry.
4. Add decommission packet persistence as canonical evidence.
5. Add minimal memory placement into a versioned `GradientSpace`.
6. Add memory-lineage projection.
7. Add an evaluator fixture format before invoking any real evaluator API.
8. Only then replace one mock role with a controlled local Qwen-backed role.

The first real local LLM step should be small: one role, one fixture, one measurable behavior change. The system should keep the mock path as the regression harness.

## 8. Architecture Promotion Checklist

Before calling `docs/architecture.md` the working architecture, confirm:

- Layer boundaries match implemented module boundaries.
- Core entities have tests or fixtures.
- Events are sufficient to replay a full mock run.
- Q-TOM route decisions are visible to loom validation.
- Memory curation has evidence references and versioned placement.
- Evaluation assumptions are configuration, not hard-coded behavior.
- CUDA behavior is described as an optional backend, not as the architecture itself.
- Open questions are explicit and testable.

## 9. Open Questions

- What is the smallest useful seed axis set for `GradientSpace`?
- Should the first durable event store remain JSONL, or should SQLite enter before local LLM agents?
- What is the minimum evaluator rubric needed to show route proximity predicts acceptable substitution?
- Which role should become real first: Director, Constructor, Integration, or Curator?
- What level of topology proposal can be automatically approved, if any?

## 10. Bias For The Next Decision

Prefer the path that preserves observability. The project becomes more convincing when every fuzzy choice leaves a crisp trace.

That makes the next architectural priority:

```text
route decisions -> assignment causation -> replay projection -> validation gate
```

Once that is boring and reliable, the system can safely become more agentic.
