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
