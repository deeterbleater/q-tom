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
