use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write;

use crate::{
    InMemoryEventLog, LoomEvent, LoomEventError, LoomEventType, ReplayCursor,
    ReplayValidationReport,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoomProjectionBundle {
    pub task_dependency: String,
    pub route_trace: String,
    pub artifact_provenance: String,
    pub integration_group: String,
    pub memory_lineage: String,
    pub topology_governance: String,
}

pub fn loom_projection_bundle(log: &InMemoryEventLog) -> LoomProjectionBundle {
    LoomProjectionBundle {
        task_dependency: task_dependency_projection(log),
        route_trace: route_trace_projection(log),
        artifact_provenance: artifact_provenance_projection(log),
        integration_group: integration_group_projection(log),
        memory_lineage: memory_lineage_projection(log),
        topology_governance: topology_governance_projection(log),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoomReplayReport {
    pub validation: ReplayValidationReport,
    pub projections: LoomProjectionBundle,
}

pub fn loom_replay_report(log: &InMemoryEventLog) -> Result<LoomReplayReport, LoomEventError> {
    if log.is_empty() {
        return Err(LoomEventError::EmptyReplayLog);
    }

    let validation = log.validate_replay()?;
    let projections = loom_projection_bundle(log);

    Ok(LoomReplayReport {
        validation,
        projections,
    })
}

pub fn route_trace_projection(log: &InMemoryEventLog) -> String {
    let events = log.replay(ReplayCursor::start()).collect::<Vec<_>>();
    let route_events = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::RouteDecisionRecorded)
        .collect::<Vec<_>>();
    let assignments_by_cause = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::TaskAssigned)
        .filter_map(|event| event.causation_id.map(|cause| (cause, event)))
        .collect::<HashMap<_, _>>();

    let mut output = String::from("flowchart TD\n");

    for route_event in route_events {
        let Some(task_id) = route_event.task_id else {
            continue;
        };
        let route_node = format!("route_{}", route_event.event_id);
        let task_node = format!("task_{task_id}");
        let decision_id = route_decision_id(route_event);

        writeln!(output, "  {task_node}[\"Task {task_id}\"]")
            .expect("writing to String should not fail");
        writeln!(output, "  {route_node}[\"RouteDecision {decision_id}\"]")
            .expect("writing to String should not fail");
        writeln!(output, "  {task_node} --> {route_node}")
            .expect("writing to String should not fail");

        if let Some(assignment) = assignments_by_cause.get(&route_event.event_id) {
            let assignment_node = format!("assignment_{}", assignment.event_id);
            writeln!(output, "  {assignment_node}[\"TaskAssigned {task_id}\"]")
                .expect("writing to String should not fail");
            writeln!(output, "  {route_node} --> {assignment_node}")
                .expect("writing to String should not fail");

            if let Some(agent_id) = assignment.agent_id {
                let agent_node = format!("agent_{agent_id}");
                writeln!(output, "  {agent_node}[\"Agent {agent_id}\"]")
                    .expect("writing to String should not fail");
                writeln!(output, "  {assignment_node} --> {agent_node}")
                    .expect("writing to String should not fail");
            }
        }
    }

    output
}

pub fn task_dependency_projection(log: &InMemoryEventLog) -> String {
    let events = log.replay(ReplayCursor::start()).collect::<Vec<_>>();
    let task_events = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::TaskCreated)
        .collect::<Vec<_>>();
    let integration_parent_ids = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::IntegrationRequested)
        .filter_map(|event| event.task_id)
        .collect::<HashSet<_>>();
    let child_task_ids_by_parent = task_events
        .iter()
        .filter_map(|event| Some((event.task_id?, event.parent_task_id?)))
        .fold(
            HashMap::<u64, Vec<u64>>::new(),
            |mut map, (task, parent)| {
                map.entry(parent).or_default().push(task);
                map
            },
        );

    let mut output = String::from("flowchart TD\n");

    for task_event in task_events {
        let Some(task_id) = task_event.task_id else {
            continue;
        };
        let task_node = format!("task_{task_id}");
        writeln!(output, "  {task_node}[\"Task {task_id}\"]")
            .expect("writing to String should not fail");

        if let Some(parent_task_id) = task_event.parent_task_id {
            writeln!(output, "  task_{parent_task_id} --> {task_node}")
                .expect("writing to String should not fail");
        }
    }

    for parent_task_id in integration_parent_ids {
        let integration_node = format!("integration_{parent_task_id}");
        writeln!(
            output,
            "  {integration_node}[\"Integration {parent_task_id}\"]"
        )
        .expect("writing to String should not fail");

        if let Some(child_task_ids) = child_task_ids_by_parent.get(&parent_task_id) {
            for child_task_id in child_task_ids {
                writeln!(output, "  task_{child_task_id} --> {integration_node}")
                    .expect("writing to String should not fail");
            }
        }
    }

    output
}

pub fn artifact_provenance_projection(log: &InMemoryEventLog) -> String {
    let events = log.replay(ReplayCursor::start()).collect::<Vec<_>>();
    let declared_events = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::ArtifactDeclared)
        .collect::<Vec<_>>();
    let ready_events_by_cause = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::ArtifactReady)
        .filter_map(|event| event.causation_id.map(|cause| (cause, event)))
        .collect::<HashMap<_, _>>();

    let mut output = String::from("flowchart TD\n");

    for declared_event in declared_events {
        let Some(task_id) = declared_event.task_id else {
            continue;
        };
        let declared_node = format!("artifact_declared_{}", declared_event.event_id);
        let task_node = format!("task_{task_id}");
        let artifact_id = ref_tail(declared_event.payload_ref.as_str());

        writeln!(output, "  {task_node}[\"Task {task_id}\"]")
            .expect("writing to String should not fail");
        writeln!(
            output,
            "  {declared_node}[\"ArtifactDeclared {artifact_id}\"]"
        )
        .expect("writing to String should not fail");
        writeln!(output, "  {task_node} --> {declared_node}")
            .expect("writing to String should not fail");

        if let Some(ready_event) = ready_events_by_cause.get(&declared_event.event_id) {
            let ready_node = format!("artifact_ready_{}", ready_event.event_id);
            let ready_artifact_id = ref_tail(ready_event.payload_ref.as_str());

            writeln!(
                output,
                "  {ready_node}[\"ArtifactReady {ready_artifact_id}\"]"
            )
            .expect("writing to String should not fail");
            writeln!(output, "  {declared_node} --> {ready_node}")
                .expect("writing to String should not fail");

            if let Some(agent_id) = ready_event.agent_id {
                let agent_node = format!("agent_{agent_id}");
                writeln!(output, "  {agent_node}[\"Agent {agent_id}\"]")
                    .expect("writing to String should not fail");
                writeln!(output, "  {ready_node} --> {agent_node}")
                    .expect("writing to String should not fail");
            }
        }
    }

    output
}

pub fn integration_group_projection(log: &InMemoryEventLog) -> String {
    let events = log.replay(ReplayCursor::start()).collect::<Vec<_>>();
    let task_events = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::TaskCreated)
        .collect::<Vec<_>>();
    let child_task_ids_by_parent = task_events
        .iter()
        .filter_map(|event| Some((event.task_id?, event.parent_task_id?)))
        .fold(
            HashMap::<u64, Vec<u64>>::new(),
            |mut map, (task, parent)| {
                map.entry(parent).or_default().push(task);
                map
            },
        );
    let group_events = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::IntegrationRequested)
        .filter(|event| is_integration_group_ref(event.payload_ref.as_str()))
        .collect::<Vec<_>>();
    let report_events_by_group_id = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::IntegrationRequested)
        .filter(|event| is_integration_report_ref(event.payload_ref.as_str()))
        .map(|event| (ref_tail(event.payload_ref.as_str()).to_string(), event))
        .collect::<HashMap<_, _>>();

    let mut output = String::from("flowchart TD\n");

    for group_event in group_events {
        let Some(parent_task_id) = group_event.task_id else {
            continue;
        };
        let group_id = ref_tail(group_event.payload_ref.as_str());
        let parent_node = format!("task_{parent_task_id}");
        let group_node = format!("integration_group_{}", group_event.event_id);

        writeln!(output, "  {parent_node}[\"Task {parent_task_id}\"]")
            .expect("writing to String should not fail");
        writeln!(output, "  {group_node}[\"IntegrationGroup {group_id}\"]")
            .expect("writing to String should not fail");
        writeln!(output, "  {parent_node} --> {group_node}")
            .expect("writing to String should not fail");

        if let Some(child_task_ids) = child_task_ids_by_parent.get(&parent_task_id) {
            for child_task_id in child_task_ids {
                let child_node = format!("task_{child_task_id}");
                writeln!(output, "  {child_node}[\"Task {child_task_id}\"]")
                    .expect("writing to String should not fail");
                writeln!(output, "  {child_node} --> {group_node}")
                    .expect("writing to String should not fail");
            }
        }

        if let Some(report_event) = report_events_by_group_id.get(group_id) {
            let report_node = format!("integration_report_{}", report_event.event_id);
            writeln!(output, "  {report_node}[\"IntegrationReport {group_id}\"]")
                .expect("writing to String should not fail");
            writeln!(output, "  {group_node} --> {report_node}")
                .expect("writing to String should not fail");

            if let Some(agent_id) = report_event.agent_id {
                let agent_node = format!("agent_{agent_id}");
                writeln!(output, "  {agent_node}[\"Agent {agent_id}\"]")
                    .expect("writing to String should not fail");
                writeln!(output, "  {report_node} --> {agent_node}")
                    .expect("writing to String should not fail");
            }
        }
    }

    output
}

pub fn memory_lineage_projection(log: &InMemoryEventLog) -> String {
    let events = log.replay(ReplayCursor::start()).collect::<Vec<_>>();
    let decommission_events = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::AgentDecommissioned)
        .collect::<Vec<_>>();
    let memory_events_by_cause = events
        .iter()
        .copied()
        .filter(|event| event.event_type == LoomEventType::MemoryNodeCreated)
        .filter_map(|event| event.causation_id.map(|cause| (cause, event)))
        .collect::<HashMap<_, _>>();

    let mut output = String::from("flowchart TD\n");

    for decommission_event in decommission_events {
        let Some(task_id) = decommission_event.task_id else {
            continue;
        };
        let decommission_node = format!("decommission_{}", decommission_event.event_id);
        let task_node = format!("task_{task_id}");
        let agent_label = decommission_event
            .agent_id
            .map(|agent_id| agent_id.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        writeln!(output, "  {task_node}[\"Task {task_id}\"]")
            .expect("writing to String should not fail");
        writeln!(
            output,
            "  {decommission_node}[\"Decommission {agent_label}\"]"
        )
        .expect("writing to String should not fail");
        writeln!(output, "  {task_node} --> {decommission_node}")
            .expect("writing to String should not fail");

        if let Some(memory_event) = memory_events_by_cause.get(&decommission_event.event_id) {
            let memory_node = format!("memory_{}", memory_event.event_id);
            let memory_id = ref_tail(memory_event.payload_ref.as_str());
            writeln!(output, "  {memory_node}[\"MemoryNode {memory_id}\"]")
                .expect("writing to String should not fail");
            writeln!(output, "  {decommission_node} --> {memory_node}")
                .expect("writing to String should not fail");
        }
    }

    output
}

pub fn topology_governance_projection(log: &InMemoryEventLog) -> String {
    let events = log.replay(ReplayCursor::start()).collect::<Vec<_>>();
    let topology_events = events
        .iter()
        .copied()
        .filter(|event| {
            matches!(
                event.event_type,
                LoomEventType::TopologyProposed
                    | LoomEventType::TopologyCommitted
                    | LoomEventType::TopologyRolledBack
            )
        })
        .collect::<Vec<_>>();
    let topology_events_by_id = topology_events
        .iter()
        .copied()
        .map(|event| (event.event_id, event))
        .collect::<HashMap<_, _>>();

    let mut output = String::from("flowchart TD\n");

    for event in topology_events {
        let node = topology_node_id(event);
        let label = topology_label(event);
        writeln!(output, "  {node}[\"{label}\"]").expect("writing to String should not fail");

        if let Some(causation_id) = event.causation_id {
            if let Some(cause) = topology_events_by_id.get(&causation_id) {
                let cause_node = topology_node_id(cause);
                writeln!(output, "  {cause_node} --> {node}")
                    .expect("writing to String should not fail");
            }
        }
    }

    output
}

fn topology_node_id(event: &LoomEvent) -> String {
    match event.event_type {
        LoomEventType::TopologyProposed => format!("topology_proposed_{}", event.event_id),
        LoomEventType::TopologyCommitted => format!("topology_committed_{}", event.event_id),
        LoomEventType::TopologyRolledBack => format!("topology_rolled_back_{}", event.event_id),
        _ => format!("topology_event_{}", event.event_id),
    }
}

fn topology_label(event: &LoomEvent) -> String {
    match event.event_type {
        LoomEventType::TopologyProposed => {
            format!("TopologyProposed {}", ref_tail(event.payload_ref.as_str()))
        }
        LoomEventType::TopologyCommitted => {
            let snapshot_id = event
                .topology_snapshot_id
                .map(|snapshot_id| snapshot_id.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("TopologyCommitted {snapshot_id}")
        }
        LoomEventType::TopologyRolledBack => {
            let snapshot_id = event
                .topology_snapshot_id
                .map(|snapshot_id| snapshot_id.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("TopologyRolledBack {snapshot_id}")
        }
        _ => format!("TopologyEvent {}", event.event_id),
    }
}

fn route_decision_id(event: &LoomEvent) -> &str {
    ref_tail(event.payload_ref.as_str())
}

fn ref_tail(payload_ref: &str) -> &str {
    payload_ref
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
}

fn is_integration_group_ref(payload_ref: &str) -> bool {
    payload_ref.contains("/integration/group/")
}

fn is_integration_report_ref(payload_ref: &str) -> bool {
    payload_ref.contains("/integration/report/")
}
