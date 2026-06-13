use std::collections::HashMap;
use std::fmt::Write;

use crate::{InMemoryEventLog, LoomEvent, LoomEventType, ReplayCursor};

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

fn route_decision_id(event: &LoomEvent) -> &str {
    event
        .payload_ref
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
}
