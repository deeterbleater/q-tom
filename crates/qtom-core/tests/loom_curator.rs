use qtom_core::{
    AgentDecommissionPacket, LoomEvent, LoomEventType, MemoryNodeKind, MockCurator,
    MockCuratorConfig, ReplayCursor,
};

fn packet() -> AgentDecommissionPacket {
    AgentDecommissionPacket::completed(1_200, 301, 1, 11, 7, 42, vec![900], "inline://summary/1200")
        .expect("decommission packet should be valid")
}

fn decommission_event() -> LoomEvent {
    LoomEvent {
        event_id: 2_003,
        event_type: LoomEventType::AgentDecommissioned,
        root_task_id: 1,
        task_id: Some(11),
        parent_task_id: Some(10),
        prompt_id: Some(7),
        agent_id: Some(301),
        agent_role: Some("constructor".to_string()),
        topology_snapshot_id: None,
        payload_schema: "qtom.mock.constructor.v1".to_string(),
        payload_ref: "inline://decommission/301/11".to_string(),
        occurred_at_ms: 10_003,
        causation_id: None,
        correlation_id: 77,
    }
}

#[test]
fn curator_derives_memory_node_from_decommission_packet() {
    let curator = MockCurator::new(MockCuratorConfig {
        curator_agent_id: 800,
        next_memory_node_id: 1_500,
        next_event_id: 4_000,
        occurred_at_ms: 30_000,
        correlation_id: 77,
    });

    let output = curator
        .curate_decommission_packet(&packet(), &decommission_event())
        .expect("curator should derive memory node");

    assert_eq!(output.memory_node.memory_node_id, 1_500);
    assert_eq!(output.memory_node.kind, MemoryNodeKind::Episode);
    assert_eq!(output.memory_node.root_task_id, 1);
    assert_eq!(output.memory_node.task_id, 11);
    assert_eq!(output.memory_node.packet_id, 1_200);
    assert_eq!(
        output.memory_node.evidence_refs,
        vec!["inline://summary/1200"]
    );
}

#[test]
fn curator_emits_replay_valid_memory_event_with_decommission_evidence() {
    let output = MockCurator::default()
        .curate_decommission_packet(&packet(), &decommission_event())
        .expect("curator should derive memory node");

    let events: Vec<_> = output.event_log.replay(ReplayCursor::start()).collect();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, LoomEventType::AgentDecommissioned);
    assert_eq!(events[1].event_type, LoomEventType::MemoryNodeCreated);
    assert_eq!(events[1].causation_id, Some(events[0].event_id));
    assert_eq!(events[1].agent_id, Some(800));
    assert_eq!(events[1].payload_ref, "inline://memory/1500");
    output
        .event_log
        .validate_replay()
        .expect("curator event log should replay");
}
