use qtom_core::{
    LoomModelError, RollbackRecord, TopologyProposal, TopologyProposalKind, TopologySnapshot,
    TopologySnapshotStatus, append_rollback_record_jsonl, append_topology_proposal_jsonl,
    append_topology_snapshot_jsonl, read_rollback_records_jsonl, read_topology_proposals_jsonl,
    read_topology_snapshots_jsonl, write_rollback_records_jsonl, write_topology_proposals_jsonl,
    write_topology_snapshots_jsonl,
};

fn proposal(topology_proposal_id: u64) -> TopologyProposal {
    TopologyProposal::draft(
        topology_proposal_id,
        TopologyProposalKind::MemoryIndexVersion,
        "curator://agent/800",
        format!("inline://topology/change-set/{topology_proposal_id}"),
        vec![format!(
            "inline://candidate-reduction/report/{topology_proposal_id}"
        )],
        50_000 + topology_proposal_id,
    )
    .expect("proposal should be valid")
}

fn temp_jsonl_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("qtom-topology-{name}-{}.jsonl", std::process::id()))
}

fn snapshot(topology_snapshot_id: u64) -> TopologySnapshot {
    TopologySnapshot {
        topology_snapshot_id,
        parent_snapshot_id: Some(topology_snapshot_id - 1),
        source_proposal_id: topology_snapshot_id - 1_000,
        agent_registry_version: format!("agent-registry-v{topology_snapshot_id}"),
        gradient_space_versions: vec![format!("gradient-space-44-v{topology_snapshot_id}")],
        memory_index_versions: vec![format!("memory-index-v{topology_snapshot_id}")],
        route_policy_versions: vec![format!("route-policy-v{topology_snapshot_id}")],
        hard_constraint_policy_version: "hard-constraints-v1".to_string(),
        status: TopologySnapshotStatus::Active,
        created_at_ms: 60_000 + topology_snapshot_id,
    }
}

fn rollback(rollback_id: u64) -> RollbackRecord {
    RollbackRecord::new(
        rollback_id,
        9_000 + rollback_id,
        8_000 + rollback_id,
        format!("rollback reason {rollback_id}"),
        format!("monitor://rollback/{rollback_id}"),
        vec![format!("inline://route-decision/{rollback_id}")],
        70_000 + rollback_id,
    )
    .expect("rollback record should be valid")
}

#[test]
fn topology_proposals_round_trip_through_jsonl() {
    let path = temp_jsonl_path("roundtrip");
    let _ = std::fs::remove_file(&path);
    let proposals = vec![proposal(8_000), proposal(8_001)];

    write_topology_proposals_jsonl(&path, &proposals).expect("proposals should write");
    let read = read_topology_proposals_jsonl(&path).expect("proposals should read");

    assert_eq!(read, proposals);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_topology_proposal_preserves_existing_proposals() {
    let path = temp_jsonl_path("append");
    let _ = std::fs::remove_file(&path);

    append_topology_proposal_jsonl(&path, &proposal(8_000)).expect("first append should work");
    append_topology_proposal_jsonl(&path, &proposal(8_001)).expect("second append should work");

    let read = read_topology_proposals_jsonl(&path).expect("proposals should read");

    assert_eq!(read, vec![proposal(8_000), proposal(8_001)]);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_topology_proposal_rejects_duplicate_proposal_id() {
    let path = temp_jsonl_path("duplicate");
    let _ = std::fs::remove_file(&path);

    append_topology_proposal_jsonl(&path, &proposal(8_000)).expect("first append should work");

    let err = append_topology_proposal_jsonl(&path, &proposal(8_000))
        .expect_err("duplicate proposal id should fail");

    assert_eq!(err, LoomModelError::DuplicateTopologyProposalId(8_000));
    assert_eq!(
        read_topology_proposals_jsonl(&path).expect("existing proposal should remain"),
        vec![proposal(8_000)]
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn topology_snapshots_round_trip_through_jsonl() {
    let path = temp_jsonl_path("snapshot-roundtrip");
    let _ = std::fs::remove_file(&path);
    let snapshots = vec![snapshot(9_000), snapshot(9_001)];

    write_topology_snapshots_jsonl(&path, &snapshots).expect("snapshots should write");
    let read = read_topology_snapshots_jsonl(&path).expect("snapshots should read");

    assert_eq!(read, snapshots);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_topology_snapshot_preserves_existing_snapshots() {
    let path = temp_jsonl_path("snapshot-append");
    let _ = std::fs::remove_file(&path);

    append_topology_snapshot_jsonl(&path, &snapshot(9_000)).expect("first append should work");
    append_topology_snapshot_jsonl(&path, &snapshot(9_001)).expect("second append should work");

    let read = read_topology_snapshots_jsonl(&path).expect("snapshots should read");

    assert_eq!(read, vec![snapshot(9_000), snapshot(9_001)]);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_topology_snapshot_rejects_duplicate_snapshot_id() {
    let path = temp_jsonl_path("snapshot-duplicate");
    let _ = std::fs::remove_file(&path);

    append_topology_snapshot_jsonl(&path, &snapshot(9_000)).expect("first append should work");

    let err = append_topology_snapshot_jsonl(&path, &snapshot(9_000))
        .expect_err("duplicate snapshot id should fail");

    assert_eq!(err, LoomModelError::DuplicateTopologySnapshotId(9_000));
    assert_eq!(
        read_topology_snapshots_jsonl(&path).expect("existing snapshot should remain"),
        vec![snapshot(9_000)]
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn rollback_records_round_trip_through_jsonl() {
    let path = temp_jsonl_path("rollback-roundtrip");
    let _ = std::fs::remove_file(&path);
    let records = vec![rollback(10_000), rollback(10_001)];

    write_rollback_records_jsonl(&path, &records).expect("rollback records should write");
    let read = read_rollback_records_jsonl(&path).expect("rollback records should read");

    assert_eq!(read, records);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_rollback_record_preserves_existing_records() {
    let path = temp_jsonl_path("rollback-append");
    let _ = std::fs::remove_file(&path);

    append_rollback_record_jsonl(&path, &rollback(10_000)).expect("first append should work");
    append_rollback_record_jsonl(&path, &rollback(10_001)).expect("second append should work");

    let read = read_rollback_records_jsonl(&path).expect("rollback records should read");

    assert_eq!(read, vec![rollback(10_000), rollback(10_001)]);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_rollback_record_rejects_duplicate_rollback_id() {
    let path = temp_jsonl_path("rollback-duplicate");
    let _ = std::fs::remove_file(&path);

    append_rollback_record_jsonl(&path, &rollback(10_000)).expect("first append should work");

    let err = append_rollback_record_jsonl(&path, &rollback(10_000))
        .expect_err("duplicate rollback id should fail");

    assert_eq!(err, LoomModelError::DuplicateRollbackId(10_000));
    assert_eq!(
        read_rollback_records_jsonl(&path).expect("existing rollback should remain"),
        vec![rollback(10_000)]
    );

    let _ = std::fs::remove_file(&path);
}
