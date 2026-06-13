# Formal Architecture Process

**Status:** Draft process contract
**Date:** 2026-06-13
**Scope:** How Q-TOM and the Agent Task Loom move from promising prototype to formal architecture without outrunning evidence.

## 1. Purpose

The long-term idea is now coherent enough to deserve architecture discipline, but not so fixed that the project should freeze the design too early. This document defines the process for turning the current local-first prototype into a formal architecture that can guide implementation, testing, benchmarking, and future cluster work.

The guiding rule is:

```text
Do not promote an architectural claim until the repo can point to evidence for it.
```

The immediate goal is not to write a perfect final architecture. The immediate goal is to produce the artifacts, tests, fixtures, and measurements that make `docs/architecture.md` defensible.

## 2. Architecture Target

The architecture should describe a local-first, replayable, topology-aware orchestration system:

```text
Q-TOM Router
  -> exact route scoring, candidate ranking, backend parity, route telemetry

Agent Task Loom
  -> Split, Build, Join, Remember lifecycle over typed tasks and append-only events

Agent Runtime
  -> local agent execution with explicit prompt/tool/MCP/memory/model profiles

Memory And Curator Layer
  -> decommission evidence, typed memory nodes, gradient placement, candidate reduction

Governance Layer
  -> versioned topology proposals, shadow routing, canary promotion, rollback
```

The architecture should remain honest about scope:

- Q-TOM is the routing substrate, not the whole orchestration system.
- The Agent Task Loom is the system boundary for async swarm control.
- CUDA is an optional backend, not the architecture.
- Real local agents are introduced only after replay, lineage, and validation work with mocks.
- Cluster execution is a later scaling stage, not an MVP assumption.

## 3. Work Products

The formal architecture should be assembled from smaller, testable documents.

| Work product | Purpose | Promotion signal |
| --- | --- | --- |
| `docs/glossary.md` | Stabilize vocabulary. | Core terms stop drifting across docs and code. |
| `docs/system-boundaries.md` | Define ownership and interfaces. | New features have an obvious layer home. |
| `docs/core-entities.md` | Define durable entities and lineage. | Entities map to structs, fixtures, or events. |
| `docs/event-vocabulary.md` | Define replayable event contracts. | Simulator events need no hidden state. |
| `docs/lifecycle-flows.md` | Show Split, Build, Join, Remember flows. | A full run can be explained end to end. |
| `docs/topology-governance.md` | Control topology mutation. | Proposals, commits, and rollback are explicit. |
| `docs/mvp-roadmap.md` | Convert design into build phases. | Every phase has acceptance criteria and tests. |
| `docs/realization-plan.md` | Sequence practical implementation. | Near-term work has a clear evidence path. |
| `docs/local-agent-readiness.md` | Gate real local LLM execution. | Mock behavior can be replaced safely. |
| `docs/mvp-review-gate.md` | Decide when the MVP is real enough. | Go, no-go, reshape, or scrap decisions are evidence-based. |
| `docs/architecture.md` | Consolidate the working architecture. | All major claims cite implemented evidence or open gates. |

## 4. Evidence Gates

Each gate converts a speculative architecture claim into a working contract.

### 4.1 Routing Gate

Question:

```text
Can Q-TOM make explainable route decisions that remain correct across supported backends?
```

Required evidence:

- CPU golden fixtures remain the correctness oracle.
- CUDA and future backends fail closed for unsupported shapes.
- Route decisions preserve selected candidate, available top-k, observed ideal candidate, ideal-unavailable flag, substitute distance delta, backend, route policy, and live-state snapshot.
- Every assignment in the loom is caused by a recorded route decision.

### 4.2 Replay Gate

Question:

```text
Can a run be reconstructed from events and immutable references?
```

Required evidence:

- Event IDs are unique and causation is valid.
- A complete mock SBJR run validates from append-only events.
- Projections are generated from events only.
- Broken logs fail with specific validation errors.

### 4.3 Lineage Gate

Question:

```text
Can the system explain how a prompt became artifacts and memories?
```

Required evidence:

- Root prompt, plan node, child task, assignment, artifact, integration report, decommission packet, and memory node are linked.
- Every completed task has decommission evidence.
- Every memory node has evidence refs.
- Task dependency, route trace, and memory lineage projections are stable.

### 4.4 Memory Candidate Gate

Question:

```text
Can fuzzy memory placement reduce candidate count without breaking hard constraints?
```

Required evidence:

- Memory placement records include gradient space version.
- Candidate reduction preserves hard-mask violation rate of zero.
- Recall, p99 latency, ideal-unavailable agreement, and substitute-quality impact are measured against exact routing.
- Exact Q-TOM scoring remains the final selection step inside the candidate set.

### 4.5 Local Agent Gate

Question:

```text
Do real local agent profiles differ enough for topology-aware routing to matter?
```

Required evidence:

- Qwen-backed profiles are varied by prompt, tool bundle, MCP library set, memory set, or worker behavior.
- Model choice remains configuration, with `Qwen3-2507` as the local prototype target.
- Evaluator configuration is versioned.
- Route proximity predicts acceptable substitution better than random or flat priority selection.

### 4.6 Governance Gate

Question:

```text
Can the system evolve topology without silently rewriting routing truth?
```

Required evidence:

- Topology changes are proposed before commit.
- Axis, route policy, agent profile, memory index, and benchmark-rubric proposals are versioned.
- Shadow routing or canary routing compares proposals against current topology.
- Rollback is tested.

## 5. Architecture Promotion Rules

Use these rules when updating `docs/architecture.md`.

- Claims backed by code, tests, fixtures, or benchmark artifacts may be written as current architecture.
- Claims backed only by design intent must be written as planned architecture.
- Claims with unclear evidence must be written as open questions.
- Real local-agent behavior must not be assumed from mock-agent behavior.
- CUDA behavior must be described as a backend capability, not a system requirement.
- Fuzzy candidate generation must never weaken hard constraints.
- Any topology mutation path must include proposal, evidence, commit, and rollback semantics.

## 6. Build Sequence

The practical sequence is:

1. Keep the mock SBJR loom as the regression harness.
2. Strengthen replay validation until invalid causation, missing route decisions, missing decommission evidence, and missing memory evidence fail loudly.
3. Add the remaining projections: artifact provenance and integration group views.
4. Add a first memory candidate loop that produces compact candidate sets before exact Q-TOM scoring.
5. Add evaluator fixture scaffolding before invoking real evaluator APIs.
6. Replace one mock role with a controlled local runtime role.
7. Measure whether route proximity predicts acceptable substitution.
8. Promote only the cleared parts of `docs/architecture.md` from draft to working architecture.

This order keeps the project rigorous while still letting the idea breathe.

## 7. Decision Log Expectations

When the project makes a major architecture choice, record:

- decision date
- decision owner
- options considered
- selected option
- evidence available
- trade-offs accepted
- reversal trigger
- follow-up tests or benchmarks

Small choices can live in regular docs. Larger choices should become ADRs under `docs/adr-*`.

## 8. Open Questions

- Should the first durable event store remain JSONL through the local LLM stage, or should SQLite enter before real local agents?
- Which role should become real first: Director, Constructor, Integration, or Curator?
- What is the minimum useful seed axis set for shared `GradientSpace` memory placement?
- What evaluator rubric best measures substitution quality without overfitting to one model judge?
- Which topology proposal types can be auto-approved in local development, if any?
- What evidence threshold justifies cluster planning beyond local replay and candidate reduction?

## 9. Next Hurdle

The next architecture-relevant implementation hurdle is:

```text
route decision causation -> replay validation -> projection evidence -> MVP review gate
```

Once those contracts are boring and reliable, the project can safely start replacing mock roles with real local agents.
