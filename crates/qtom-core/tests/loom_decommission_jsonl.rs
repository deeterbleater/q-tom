use qtom_core::{
    AgentDecommissionPacket, LoomModelError, append_decommission_packet_jsonl,
    read_decommission_packets_jsonl, write_decommission_packets_jsonl,
};

fn packet(packet_id: u64, task_id: u64) -> AgentDecommissionPacket {
    AgentDecommissionPacket::completed(
        packet_id,
        10_000 + task_id,
        1,
        task_id,
        7,
        42,
        vec![900 + task_id],
        format!("inline://decommission/{packet_id}"),
    )
    .expect("packet should be valid")
}

#[test]
fn decommission_packets_round_trip_through_jsonl() {
    let path = std::env::temp_dir().join(format!(
        "qtom-decommission-roundtrip-{}.jsonl",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let packets = vec![packet(1_200, 11), packet(1_201, 12)];

    write_decommission_packets_jsonl(&path, &packets).expect("packets should write");
    let read = read_decommission_packets_jsonl(&path).expect("packets should read");

    assert_eq!(read, packets);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_decommission_packet_preserves_existing_packets() {
    let path = std::env::temp_dir().join(format!(
        "qtom-decommission-append-{}.jsonl",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    append_decommission_packet_jsonl(&path, &packet(1_200, 11)).expect("first append should work");
    append_decommission_packet_jsonl(&path, &packet(1_201, 12)).expect("second append should work");

    let read = read_decommission_packets_jsonl(&path).expect("packets should read");

    assert_eq!(read, vec![packet(1_200, 11), packet(1_201, 12)]);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn append_decommission_packet_rejects_duplicate_packet_id() {
    let path = std::env::temp_dir().join(format!(
        "qtom-decommission-duplicate-{}.jsonl",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    append_decommission_packet_jsonl(&path, &packet(1_200, 11)).expect("first append should work");

    let err = append_decommission_packet_jsonl(&path, &packet(1_200, 12))
        .expect_err("duplicate packet id should fail");

    assert_eq!(err, LoomModelError::DuplicatePacketId(1_200));
    assert_eq!(
        read_decommission_packets_jsonl(&path).expect("existing packet should remain"),
        vec![packet(1_200, 11)]
    );

    let _ = std::fs::remove_file(&path);
}
