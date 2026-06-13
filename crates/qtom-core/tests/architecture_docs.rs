use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("qtom-core should live under crates/qtom-core")
        .to_path_buf()
}

#[test]
fn glossary_covers_roadmap_terms() {
    let glossary_path = repo_root().join("docs/glossary.md");
    let glossary = fs::read_to_string(&glossary_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", glossary_path.display()));
    let glossary = glossary.to_lowercase();

    let required_terms = [
        "task loom",
        "sbjr",
        "route decision",
        "candidate set",
        "topology",
        "gradient space",
        "memory node",
        "decommission packet",
        "integration group",
        "substitute quality",
    ];

    for term in required_terms {
        assert!(
            glossary.contains(term),
            "glossary should define or mention required roadmap term `{term}`"
        );
    }
}

#[test]
fn system_boundaries_cover_architecture_layers() {
    let boundaries_path = repo_root().join("docs/system-boundaries.md");
    let boundaries = fs::read_to_string(&boundaries_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", boundaries_path.display()));
    let boundaries = boundaries.to_lowercase();

    let required_layers = [
        "q-tom router",
        "agent task loom",
        "agent runtime",
        "memory and curator layer",
        "evaluation layer",
        "governance layer",
        "observability layer",
    ];

    for layer in required_layers {
        assert!(
            boundaries.contains(layer),
            "system boundaries should describe ownership for `{layer}`"
        );
    }

    let required_boundary_terms = [
        "owns",
        "does not own",
        "interface",
        "event contract",
        "trait boundary",
    ];

    for term in required_boundary_terms {
        assert!(
            boundaries.contains(term),
            "system boundaries should include boundary term `{term}`"
        );
    }
}

#[test]
fn core_entities_cover_required_model() {
    let entities_path = repo_root().join("docs/core-entities.md");
    let entities = fs::read_to_string(&entities_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", entities_path.display()));
    let entities = entities.to_lowercase();

    let required_entities = [
        "prompt",
        "taskenvelope",
        "plannode",
        "agentprofile",
        "routerequest",
        "routedecision",
        "artifactref",
        "signalref",
        "integrationgroup",
        "integrationreport",
        "agentdecommissionpacket",
        "memorynode",
        "gradientspace",
        "topologyproposal",
        "topologysnapshot",
    ];

    for entity in required_entities {
        assert!(
            entities.contains(entity),
            "core entities should define `{entity}`"
        );
    }

    let required_model_terms = ["owner", "lifecycle", "storage", "lineage"];

    for term in required_model_terms {
        assert!(
            entities.contains(term),
            "core entities should document `{term}` expectations"
        );
    }
}

#[test]
fn event_vocabulary_covers_replay_events() {
    let events_path = repo_root().join("docs/event-vocabulary.md");
    let events = fs::read_to_string(&events_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", events_path.display()));
    let events = events.to_lowercase();

    let required_events = [
        "task_created",
        "task_assigned",
        "artifact_declared",
        "artifact_ready",
        "signal_emitted",
        "task_blocked",
        "task_resumed",
        "task_completed",
        "agent_decommissioned",
        "integration_requested",
        "memory_node_created",
        "index_updated",
        "route_decision_recorded",
        "topology_proposed",
        "topology_committed",
    ];

    for event_name in required_events {
        assert!(
            events.contains(event_name),
            "event vocabulary should define `{event_name}`"
        );
    }

    let required_event_terms = ["payload", "producer", "consumer", "replay"];

    for term in required_event_terms {
        assert!(
            events.contains(term),
            "event vocabulary should document `{term}` expectations"
        );
    }
}

#[test]
fn lifecycle_flows_cover_main_system_paths() {
    let flows_path = repo_root().join("docs/lifecycle-flows.md");
    let flows = fs::read_to_string(&flows_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", flows_path.display()));
    let flows = flows.to_lowercase();

    let required_flows = [
        "root prompt flow",
        "task decomposition flow",
        "constructor execution flow",
        "integration flow",
        "decommission flow",
        "memory curation flow",
        "route-decision flow",
        "topology-update flow",
    ];

    for flow in required_flows {
        assert!(flows.contains(flow), "lifecycle flows should cover `{flow}`");
    }

    let required_flow_terms = ["ordered trace", "emitted events", "diagram", "replay"];

    for term in required_flow_terms {
        assert!(
            flows.contains(term),
            "lifecycle flows should include `{term}` guidance"
        );
    }
}

#[test]
fn topology_governance_covers_change_controls() {
    let governance_path = repo_root().join("docs/topology-governance.md");
    let governance = fs::read_to_string(&governance_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", governance_path.display()));
    let governance = governance.to_lowercase();

    let required_changes = [
        "new axes",
        "split axes",
        "deprecated axes",
        "new agent profiles",
        "benchmark schema changes",
        "memory index versions",
        "route policy changes",
    ];

    for change in required_changes {
        assert!(
            governance.contains(change),
            "topology governance should cover `{change}`"
        );
    }

    let required_controls = [
        "proposed",
        "tested",
        "approved",
        "committed",
        "rolled back",
        "shadow routing",
        "canary",
        "version",
    ];

    for control in required_controls {
        assert!(
            governance.contains(control),
            "topology governance should document `{control}` control"
        );
    }
}

#[test]
fn mvp_roadmap_covers_build_phases() {
    let roadmap_path = repo_root().join("docs/mvp-roadmap.md");
    let roadmap = fs::read_to_string(&roadmap_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", roadmap_path.display()));
    let roadmap = roadmap.to_lowercase();

    let required_phases = [
        "phase 0",
        "phase 1",
        "phase 2",
        "phase 3",
        "event log",
        "mock sbjr",
        "route mock constructor tasks",
        "decommission packets",
        "memory nodes",
        "diagram projections",
    ];

    for phase in required_phases {
        assert!(roadmap.contains(phase), "mvp roadmap should cover `{phase}`");
    }

    let required_constraints = [
        "local",
        "observable",
        "replayable",
        "acceptance criteria",
        "test strategy",
    ];

    for constraint in required_constraints {
        assert!(
            roadmap.contains(constraint),
            "mvp roadmap should include `{constraint}`"
        );
    }
}

#[test]
fn realization_plan_covers_execution_path() {
    let plan_path = repo_root().join("docs/realization-plan.md");
    let plan = fs::read_to_string(&plan_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", plan_path.display()));
    let plan = plan.to_lowercase();

    let required_tracks = [
        "routing track",
        "loom track",
        "memory track",
        "governance track",
    ];

    for track in required_tracks {
        assert!(
            plan.contains(track),
            "realization plan should cover `{track}`"
        );
    }

    let required_gates = [
        "route parity",
        "route explainability",
        "loom replay",
        "join correctness",
        "memory evidence",
        "candidate reduction",
        "local agent usefulness",
        "governance safety",
    ];

    for gate in required_gates {
        assert!(
            plan.contains(gate),
            "realization plan should define evidence gate `{gate}`"
        );
    }

    let required_terms = [
        "formal architecture",
        "architecture promotion checklist",
        "near-term build order",
        "route decisions -> assignment causation -> replay projection -> validation gate",
    ];

    for term in required_terms {
        assert!(
            plan.contains(term),
            "realization plan should include `{term}`"
        );
    }
}

#[test]
fn local_agent_readiness_covers_runtime_gate() {
    let readiness_path = repo_root().join("docs/local-agent-readiness.md");
    let readiness = fs::read_to_string(&readiness_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", readiness_path.display()));
    let readiness = readiness.to_lowercase();

    let required_terms = [
        "agentruntime",
        "hydratedcontext",
        "mock runtime",
        "default test runtime",
        "network access",
        "ci",
        "disabled",
        "ignored tests",
        "evaluator fixture",
        "versioned",
        "qwen3-2507",
    ];

    for term in required_terms {
        assert!(
            readiness.contains(term),
            "local-agent readiness doc should include `{term}`"
        );
    }
}

#[test]
fn mvp_review_gate_covers_exit_decision() {
    let review_path = repo_root().join("docs/mvp-review-gate.md");
    let review = fs::read_to_string(&review_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", review_path.display()));
    let review = review.to_lowercase();

    let required_terms = [
        "replay validation",
        "route decisions",
        "constructor tasks",
        "decommission packets",
        "memory nodes",
        "task dependency",
        "memory lineage",
        "hard constraints",
        "go decision",
        "no-go decision",
        "reshape",
        "scrap",
        "real local agents",
    ];

    for term in required_terms {
        assert!(
            review.contains(term),
            "mvp review gate doc should include `{term}`"
        );
    }
}

#[test]
fn architecture_doc_consolidates_outline() {
    let architecture_path = repo_root().join("docs/architecture.md");
    let architecture = fs::read_to_string(&architecture_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", architecture_path.display()));
    let architecture = architecture.to_lowercase();

    let required_sections = [
        "executive summary",
        "problem statement",
        "goals and non-goals",
        "design principles",
        "system context",
        "layered architecture",
        "core entity model",
        "event and storage model",
        "routing lifecycle",
        "task loom lifecycle",
        "memory curation lifecycle",
        "topology update lifecycle",
        "replay and determinism",
        "evaluation and benchmarking",
        "observability",
        "security and governance",
        "scaling strategy",
        "mvp plan",
        "risks and open questions",
    ];

    for section in required_sections {
        assert!(
            architecture.contains(section),
            "architecture doc should include section `{section}`"
        );
    }

    let supporting_docs = [
        "docs/glossary.md",
        "docs/system-boundaries.md",
        "docs/core-entities.md",
        "docs/event-vocabulary.md",
        "docs/lifecycle-flows.md",
        "docs/topology-governance.md",
        "docs/mvp-roadmap.md",
        "docs/realization-plan.md",
        "docs/local-agent-readiness.md",
        "docs/mvp-review-gate.md",
    ];

    for doc in supporting_docs {
        assert!(
            architecture.contains(doc),
            "architecture doc should link supporting doc `{doc}`"
        );
    }
}
