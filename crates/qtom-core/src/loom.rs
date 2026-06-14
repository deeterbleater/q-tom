use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct LoomEvent {
    pub event_id: u64,
    pub event_type: LoomEventType,
    pub root_task_id: u64,
    pub task_id: Option<u64>,
    pub parent_task_id: Option<u64>,
    pub prompt_id: Option<u64>,
    pub agent_id: Option<u64>,
    pub agent_role: Option<String>,
    pub topology_snapshot_id: Option<u64>,
    pub payload_schema: String,
    pub payload_ref: String,
    pub occurred_at_ms: u64,
    pub causation_id: Option<u64>,
    pub correlation_id: u64,
}

impl LoomEvent {
    fn validate(&self) -> Result<(), LoomEventError> {
        if self.payload_schema.trim().is_empty() {
            return Err(LoomEventError::MissingPayloadSchema);
        }

        if self.payload_ref.trim().is_empty() {
            return Err(LoomEventError::MissingPayloadRef);
        }

        self.validate_required_fields()?;

        Ok(())
    }

    fn validate_required_fields(&self) -> Result<(), LoomEventError> {
        if requires_task_id(self.event_type) && self.task_id.is_none() {
            return Err(LoomEventError::MissingRequiredField {
                event_type: self.event_type,
                field: "task_id",
            });
        }

        if requires_agent_id(self.event_type) && self.agent_id.is_none() {
            return Err(LoomEventError::MissingRequiredField {
                event_type: self.event_type,
                field: "agent_id",
            });
        }

        if requires_topology_snapshot_id(self.event_type) && self.topology_snapshot_id.is_none() {
            return Err(LoomEventError::MissingRequiredField {
                event_type: self.event_type,
                field: "topology_snapshot_id",
            });
        }

        Ok(())
    }
}

fn requires_task_id(event_type: LoomEventType) -> bool {
    matches!(
        event_type,
        LoomEventType::TaskCreated
            | LoomEventType::TaskAssigned
            | LoomEventType::TaskBlocked
            | LoomEventType::TaskResumed
            | LoomEventType::TaskCompleted
            | LoomEventType::RouteDecisionRecorded
    )
}

fn requires_agent_id(event_type: LoomEventType) -> bool {
    matches!(event_type, LoomEventType::AgentDecommissioned)
}

fn requires_topology_snapshot_id(event_type: LoomEventType) -> bool {
    matches!(
        event_type,
        LoomEventType::RouteDecisionRecorded
            | LoomEventType::TopologyCommitted
            | LoomEventType::TopologyRolledBack
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum LoomEventType {
    TaskCreated,
    TaskAssigned,
    ArtifactDeclared,
    ArtifactReady,
    SignalEmitted,
    TaskBlocked,
    TaskResumed,
    TaskCompleted,
    AgentDecommissioned,
    IntegrationRequested,
    MemoryNodeCreated,
    IndexUpdated,
    RouteDecisionRecorded,
    TopologyProposed,
    TopologyShadowed,
    TopologyCommitted,
    TopologyRolledBack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayCursor {
    after_event_id: Option<u64>,
}

impl ReplayCursor {
    pub const fn start() -> Self {
        Self {
            after_event_id: None,
        }
    }

    pub const fn after(event_id: u64) -> Self {
        Self {
            after_event_id: Some(event_id),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InMemoryEventLog {
    events: Vec<LoomEvent>,
    event_ids: HashSet<u64>,
}

impl InMemoryEventLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, event: LoomEvent) -> Result<(), LoomEventError> {
        event.validate()?;

        if self.event_ids.contains(&event.event_id) {
            return Err(LoomEventError::DuplicateEventId(event.event_id));
        }

        self.validate_causation(&event)?;

        self.event_ids.insert(event.event_id);
        self.events.push(event);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn replay(&self, cursor: ReplayCursor) -> impl Iterator<Item = &LoomEvent> {
        self.events.iter().filter(move |event| {
            cursor
                .after_event_id
                .map(|after_event_id| event.event_id > after_event_id)
                .unwrap_or(true)
        })
    }

    pub fn events_by_type(&self, event_type: LoomEventType) -> Vec<&LoomEvent> {
        self.events
            .iter()
            .filter(|event| event.event_type == event_type)
            .collect()
    }

    pub fn validate_replay(&self) -> Result<ReplayValidationReport, LoomEventError> {
        validate_events(&self.events)
    }

    fn validate_causation(&self, event: &LoomEvent) -> Result<(), LoomEventError> {
        let Some(expected_cause) = required_cause(event.event_type) else {
            return Ok(());
        };

        let Some(causation_id) = event.causation_id else {
            return Err(LoomEventError::MissingRequiredCausation(event.event_type));
        };

        let causation_event = self
            .events
            .iter()
            .find(|candidate| candidate.event_id == causation_id)
            .ok_or(LoomEventError::UnknownCausationId(causation_id))?;

        if causation_event.event_type != expected_cause {
            return Err(LoomEventError::InvalidCausationType {
                event_type: event.event_type,
                causation_id,
                expected: expected_cause,
                actual: causation_event.event_type,
            });
        }

        if matches!(event.event_type, LoomEventType::TaskAssigned) {
            validate_route_decision_assignment_context(event, causation_event)?;
        }

        if matches!(event.event_type, LoomEventType::ArtifactReady) {
            validate_artifact_ready_context(event, causation_event)?;
        }

        Ok(())
    }
}

pub fn write_event_log_jsonl<'a, I, P>(path: P, events: I) -> Result<(), LoomEventError>
where
    I: IntoIterator<Item = &'a LoomEvent>,
    P: AsRef<Path>,
{
    let file = File::create(path.as_ref()).map_err(|source| LoomEventError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let mut writer = BufWriter::new(file);

    for event in events {
        serde_json::to_writer(&mut writer, event).map_err(|source| LoomEventError::Json {
            line: None,
            source: source.to_string(),
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| LoomEventError::Io {
                path: path.as_ref().display().to_string(),
                source: source.to_string(),
            })?;
    }

    writer.flush().map_err(|source| LoomEventError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

pub fn read_event_log_jsonl<P>(path: P) -> Result<InMemoryEventLog, LoomEventError>
where
    P: AsRef<Path>,
{
    let file = File::open(path.as_ref()).map_err(|source| LoomEventError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let reader = BufReader::new(file);
    let mut log = InMemoryEventLog::new();

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| LoomEventError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let event: LoomEvent =
            serde_json::from_str(&line).map_err(|source| LoomEventError::Json {
                line: Some(line_number),
                source: source.to_string(),
            })?;
        log.append(event)?;
    }

    Ok(log)
}

pub fn append_event_log_jsonl<P>(path: P, event: &LoomEvent) -> Result<(), LoomEventError>
where
    P: AsRef<Path>,
{
    let mut log = if path.as_ref().exists() {
        read_event_log_jsonl(path.as_ref())?
    } else {
        InMemoryEventLog::new()
    };
    log.append(event.clone())?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(|source| LoomEventError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, event).map_err(|source| LoomEventError::Json {
        line: None,
        source: source.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| LoomEventError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    writer.flush().map_err(|source| LoomEventError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

pub fn validate_events(events: &[LoomEvent]) -> Result<ReplayValidationReport, LoomEventError> {
    let mut replay_log = InMemoryEventLog::new();
    let mut root_task_ids = HashSet::new();
    let mut task_event_count = 0;
    let mut route_decision_count = 0;
    let mut assignment_count = 0;
    let mut completion_count = 0;
    let mut decommission_count = 0;
    let mut memory_node_count = 0;
    let mut topology_shadow_count = 0;
    let mut topology_commit_count = 0;
    let mut topology_rollback_count = 0;
    let mut integration_request_count = 0;
    let mut completed_task_events_by_task_id = HashMap::new();
    let mut decommission_events_by_task_id = HashMap::new();
    let mut child_task_events = Vec::new();
    let mut integration_events_by_parent_task_id = HashMap::new();

    for event in events {
        if matches!(event.event_type, LoomEventType::TaskAssigned) {
            validate_assignment_route_decision(event, &replay_log)?;
        }

        if matches!(event.event_type, LoomEventType::MemoryNodeCreated) {
            validate_memory_evidence(event, &replay_log)?;
        }

        if matches!(event.event_type, LoomEventType::RouteDecisionRecorded) {
            validate_route_decision_telemetry(event)?;
        }

        if matches!(event.event_type, LoomEventType::ArtifactReady) {
            validate_artifact_ready(event, &replay_log)?;
        }

        replay_log.append(event.clone())?;
        root_task_ids.insert(event.root_task_id);

        if event.task_id.is_some() {
            task_event_count += 1;
        }

        match event.event_type {
            LoomEventType::RouteDecisionRecorded => route_decision_count += 1,
            LoomEventType::TaskAssigned => assignment_count += 1,
            LoomEventType::TaskCompleted => {
                completion_count += 1;
                if let Some(task_id) = event.task_id {
                    completed_task_events_by_task_id.insert(task_id, event.clone());
                }
            }
            LoomEventType::AgentDecommissioned => {
                decommission_count += 1;
                if let Some(task_id) = event.task_id {
                    decommission_events_by_task_id.insert(task_id, event.clone());
                }
            }
            LoomEventType::MemoryNodeCreated => memory_node_count += 1,
            LoomEventType::TopologyShadowed => topology_shadow_count += 1,
            LoomEventType::TopologyCommitted => topology_commit_count += 1,
            LoomEventType::TopologyRolledBack => topology_rollback_count += 1,
            LoomEventType::IntegrationRequested => {
                integration_request_count += 1;
                if let Some(task_id) = event.task_id {
                    integration_events_by_parent_task_id.insert(task_id, event.clone());
                }
            }
            LoomEventType::TaskCreated => {
                if event.parent_task_id.is_some() {
                    child_task_events.push(event.clone());
                }
            }
            _ => {}
        }
    }

    if let Some(child_task) = child_task_events
        .iter()
        .filter(|event| {
            let Some(parent_task_id) = event.parent_task_id else {
                return false;
            };
            !integration_events_by_parent_task_id.contains_key(&parent_task_id)
        })
        .min_by_key(|event| event.task_id.unwrap_or_default())
    {
        return Err(LoomEventError::MissingTaskIntegration {
            task_id: child_task.task_id.unwrap_or_default(),
        });
    }

    validate_child_task_integration_context(
        &child_task_events,
        &integration_events_by_parent_task_id,
    )?;

    if let Some(task_id) = completed_task_events_by_task_id
        .keys()
        .filter(|task_id| !decommission_events_by_task_id.contains_key(task_id))
        .min()
        .copied()
    {
        return Err(LoomEventError::MissingTaskDecommission { task_id });
    }

    validate_task_decommission_context(
        &completed_task_events_by_task_id,
        &decommission_events_by_task_id,
    )?;

    Ok(ReplayValidationReport {
        event_count: events.len(),
        root_task_count: root_task_ids.len(),
        task_event_count,
        route_decision_count,
        assignment_count,
        completion_count,
        decommission_count,
        memory_node_count,
        topology_shadow_count,
        topology_commit_count,
        topology_rollback_count,
        integration_request_count,
    })
}

fn validate_task_decommission_context(
    completed_task_events_by_task_id: &HashMap<u64, LoomEvent>,
    decommission_events_by_task_id: &HashMap<u64, LoomEvent>,
) -> Result<(), LoomEventError> {
    let mut task_ids = completed_task_events_by_task_id
        .keys()
        .copied()
        .collect::<Vec<_>>();
    task_ids.sort_unstable();

    for task_id in task_ids {
        let Some(completed_event) = completed_task_events_by_task_id.get(&task_id) else {
            continue;
        };
        let Some(decommission_event) = decommission_events_by_task_id.get(&task_id) else {
            continue;
        };

        if completed_event.root_task_id != decommission_event.root_task_id {
            return Err(LoomEventError::MismatchedTaskDecommission {
                task_id,
                decommission_event_id: decommission_event.event_id,
                field: "root_task_id",
            });
        }

        if completed_event.correlation_id != decommission_event.correlation_id {
            return Err(LoomEventError::MismatchedTaskDecommission {
                task_id,
                decommission_event_id: decommission_event.event_id,
                field: "correlation_id",
            });
        }
    }

    Ok(())
}

fn validate_child_task_integration_context(
    child_task_events: &[LoomEvent],
    integration_events_by_parent_task_id: &HashMap<u64, LoomEvent>,
) -> Result<(), LoomEventError> {
    for child_task in child_task_events {
        let Some(parent_task_id) = child_task.parent_task_id else {
            continue;
        };
        let Some(integration_event) = integration_events_by_parent_task_id.get(&parent_task_id)
        else {
            continue;
        };

        let task_id = child_task.task_id.unwrap_or_default();

        if child_task.root_task_id != integration_event.root_task_id {
            return Err(LoomEventError::MismatchedTaskIntegration {
                task_id,
                integration_event_id: integration_event.event_id,
                field: "root_task_id",
            });
        }

        if child_task.correlation_id != integration_event.correlation_id {
            return Err(LoomEventError::MismatchedTaskIntegration {
                task_id,
                integration_event_id: integration_event.event_id,
                field: "correlation_id",
            });
        }
    }

    Ok(())
}

fn validate_memory_evidence(
    event: &LoomEvent,
    replay_log: &InMemoryEventLog,
) -> Result<(), LoomEventError> {
    let Some(causation_id) = event.causation_id else {
        return Err(LoomEventError::MissingMemoryEvidence {
            event_id: event.event_id,
        });
    };

    let Some(decommission_event) = replay_log.events.iter().find(|candidate| {
        candidate.event_id == causation_id
            && candidate.event_type == LoomEventType::AgentDecommissioned
    }) else {
        return Err(LoomEventError::MissingMemoryEvidence {
            event_id: event.event_id,
        });
    };

    validate_memory_evidence_context(event, decommission_event)?;

    Ok(())
}

fn validate_memory_evidence_context(
    memory_event: &LoomEvent,
    decommission_event: &LoomEvent,
) -> Result<(), LoomEventError> {
    if memory_event.task_id != decommission_event.task_id {
        return Err(LoomEventError::MismatchedMemoryEvidence {
            memory_event_id: memory_event.event_id,
            decommission_event_id: decommission_event.event_id,
            field: "task_id",
        });
    }

    if memory_event.root_task_id != decommission_event.root_task_id {
        return Err(LoomEventError::MismatchedMemoryEvidence {
            memory_event_id: memory_event.event_id,
            decommission_event_id: decommission_event.event_id,
            field: "root_task_id",
        });
    }

    if memory_event.correlation_id != decommission_event.correlation_id {
        return Err(LoomEventError::MismatchedMemoryEvidence {
            memory_event_id: memory_event.event_id,
            decommission_event_id: decommission_event.event_id,
            field: "correlation_id",
        });
    }

    Ok(())
}

fn validate_route_decision_telemetry(event: &LoomEvent) -> Result<(), LoomEventError> {
    if event.payload_schema != "qtom.route_decision.v1" {
        return Err(LoomEventError::InvalidRouteDecisionTelemetry {
            event_id: event.event_id,
            field: "payload_schema",
        });
    }

    if !event.payload_ref.starts_with("inline://route-decision/") {
        return Err(LoomEventError::InvalidRouteDecisionTelemetry {
            event_id: event.event_id,
            field: "payload_ref",
        });
    }

    Ok(())
}

fn validate_artifact_ready(
    event: &LoomEvent,
    replay_log: &InMemoryEventLog,
) -> Result<(), LoomEventError> {
    let Some(causation_id) = event.causation_id else {
        return Ok(());
    };

    let Some(declared_event) = replay_log
        .events
        .iter()
        .find(|candidate| candidate.event_id == causation_id)
    else {
        return Ok(());
    };

    if declared_event.event_type != LoomEventType::ArtifactDeclared {
        return Ok(());
    }

    validate_artifact_ready_context(event, declared_event)
}

fn validate_artifact_ready_context(
    ready_event: &LoomEvent,
    declared_event: &LoomEvent,
) -> Result<(), LoomEventError> {
    if ready_event.task_id != declared_event.task_id {
        return Err(LoomEventError::MismatchedArtifactReady {
            ready_event_id: ready_event.event_id,
            declared_event_id: declared_event.event_id,
            field: "task_id",
        });
    }

    if ready_event.root_task_id != declared_event.root_task_id {
        return Err(LoomEventError::MismatchedArtifactReady {
            ready_event_id: ready_event.event_id,
            declared_event_id: declared_event.event_id,
            field: "root_task_id",
        });
    }

    if ready_event.correlation_id != declared_event.correlation_id {
        return Err(LoomEventError::MismatchedArtifactReady {
            ready_event_id: ready_event.event_id,
            declared_event_id: declared_event.event_id,
            field: "correlation_id",
        });
    }

    if ready_event.payload_ref != declared_event.payload_ref {
        return Err(LoomEventError::MismatchedArtifactReady {
            ready_event_id: ready_event.event_id,
            declared_event_id: declared_event.event_id,
            field: "payload_ref",
        });
    }

    Ok(())
}

fn validate_assignment_route_decision(
    event: &LoomEvent,
    replay_log: &InMemoryEventLog,
) -> Result<(), LoomEventError> {
    let Some(causation_id) = event.causation_id else {
        return Ok(());
    };

    let Some(route_decision) = replay_log
        .events
        .iter()
        .find(|candidate| candidate.event_id == causation_id)
    else {
        return Err(LoomEventError::MissingTaskRouteDecision {
            task_id: event.task_id.unwrap_or_default(),
        });
    };

    validate_route_decision_assignment_context(event, route_decision)?;

    Ok(())
}

fn validate_route_decision_assignment_context(
    assignment: &LoomEvent,
    route_decision: &LoomEvent,
) -> Result<(), LoomEventError> {
    let task_id = assignment.task_id.unwrap_or_default();
    let route_task_id = route_decision.task_id.unwrap_or_default();

    if task_id != route_task_id {
        return Err(LoomEventError::MismatchedTaskRouteDecision {
            task_id,
            route_task_id,
        });
    }

    if assignment.root_task_id != route_decision.root_task_id {
        return Err(LoomEventError::MismatchedRouteDecisionContext {
            assignment_event_id: assignment.event_id,
            route_decision_event_id: route_decision.event_id,
            field: "root_task_id",
        });
    }

    if assignment.correlation_id != route_decision.correlation_id {
        return Err(LoomEventError::MismatchedRouteDecisionContext {
            assignment_event_id: assignment.event_id,
            route_decision_event_id: route_decision.event_id,
            field: "correlation_id",
        });
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayValidationReport {
    pub event_count: usize,
    pub root_task_count: usize,
    pub task_event_count: usize,
    pub route_decision_count: usize,
    pub assignment_count: usize,
    pub completion_count: usize,
    pub decommission_count: usize,
    pub memory_node_count: usize,
    pub topology_shadow_count: usize,
    pub topology_commit_count: usize,
    pub topology_rollback_count: usize,
    pub integration_request_count: usize,
}

fn required_cause(event_type: LoomEventType) -> Option<LoomEventType> {
    match event_type {
        LoomEventType::TaskAssigned => Some(LoomEventType::RouteDecisionRecorded),
        LoomEventType::ArtifactReady => Some(LoomEventType::ArtifactDeclared),
        LoomEventType::TaskResumed => Some(LoomEventType::TaskBlocked),
        LoomEventType::TopologyShadowed => Some(LoomEventType::TopologyProposed),
        LoomEventType::TopologyCommitted => Some(LoomEventType::TopologyProposed),
        LoomEventType::TopologyRolledBack => Some(LoomEventType::TopologyCommitted),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoomEventError {
    EmptyReplayLog,
    DuplicateEventId(u64),
    MissingPayloadSchema,
    MissingPayloadRef,
    MissingRequiredField {
        event_type: LoomEventType,
        field: &'static str,
    },
    MissingRequiredCausation(LoomEventType),
    UnknownCausationId(u64),
    InvalidCausationType {
        event_type: LoomEventType,
        causation_id: u64,
        expected: LoomEventType,
        actual: LoomEventType,
    },
    MissingTaskDecommission {
        task_id: u64,
    },
    MismatchedTaskDecommission {
        task_id: u64,
        decommission_event_id: u64,
        field: &'static str,
    },
    MissingTaskRouteDecision {
        task_id: u64,
    },
    MismatchedTaskRouteDecision {
        task_id: u64,
        route_task_id: u64,
    },
    MismatchedRouteDecisionContext {
        assignment_event_id: u64,
        route_decision_event_id: u64,
        field: &'static str,
    },
    MismatchedArtifactReady {
        ready_event_id: u64,
        declared_event_id: u64,
        field: &'static str,
    },
    InvalidRouteDecisionTelemetry {
        event_id: u64,
        field: &'static str,
    },
    MissingMemoryEvidence {
        event_id: u64,
    },
    MismatchedMemoryEvidence {
        memory_event_id: u64,
        decommission_event_id: u64,
        field: &'static str,
    },
    MissingTaskIntegration {
        task_id: u64,
    },
    MismatchedTaskIntegration {
        task_id: u64,
        integration_event_id: u64,
        field: &'static str,
    },
    Io {
        path: String,
        source: String,
    },
    Json {
        line: Option<usize>,
        source: String,
    },
}

impl std::fmt::Display for LoomEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyReplayLog => write!(f, "loom replay log is empty"),
            Self::DuplicateEventId(event_id) => {
                write!(f, "duplicate loom event id {event_id}")
            }
            Self::MissingPayloadSchema => write!(f, "loom event payload schema is required"),
            Self::MissingPayloadRef => write!(f, "loom event payload ref is required"),
            Self::MissingRequiredField { event_type, field } => {
                write!(f, "loom event {event_type:?} requires {field}")
            }
            Self::MissingRequiredCausation(event_type) => {
                write!(f, "loom event {event_type:?} requires causation")
            }
            Self::UnknownCausationId(event_id) => {
                write!(f, "loom event causation id {event_id} does not exist")
            }
            Self::InvalidCausationType {
                event_type,
                causation_id,
                expected,
                actual,
            } => write!(
                f,
                "loom event {event_type:?} causation id {causation_id} should reference {expected:?}, got {actual:?}"
            ),
            Self::MissingTaskDecommission { task_id } => {
                write!(f, "completed task {task_id} is missing decommission event")
            }
            Self::MismatchedTaskDecommission {
                task_id,
                decommission_event_id,
                field,
            } => write!(
                f,
                "completed task {task_id} references decommission event {decommission_event_id} with mismatched {field}"
            ),
            Self::MissingTaskRouteDecision { task_id } => {
                write!(f, "assigned task {task_id} is missing route decision event")
            }
            Self::MismatchedTaskRouteDecision {
                task_id,
                route_task_id,
            } => write!(
                f,
                "assigned task {task_id} references route decision for task {route_task_id}"
            ),
            Self::MismatchedRouteDecisionContext {
                assignment_event_id,
                route_decision_event_id,
                field,
            } => write!(
                f,
                "assignment event {assignment_event_id} references route decision event {route_decision_event_id} with mismatched {field}"
            ),
            Self::MismatchedArtifactReady {
                ready_event_id,
                declared_event_id,
                field,
            } => write!(
                f,
                "artifact ready event {ready_event_id} references declaration event {declared_event_id} with mismatched {field}"
            ),
            Self::InvalidRouteDecisionTelemetry { event_id, field } => {
                write!(f, "route decision event {event_id} has invalid {field}")
            }
            Self::MissingMemoryEvidence { event_id } => {
                write!(f, "memory node event {event_id} is missing evidence")
            }
            Self::MismatchedMemoryEvidence {
                memory_event_id,
                decommission_event_id,
                field,
            } => write!(
                f,
                "memory node event {memory_event_id} references decommission event {decommission_event_id} with mismatched {field}"
            ),
            Self::MissingTaskIntegration { task_id } => {
                write!(f, "child task {task_id} is missing integration path")
            }
            Self::MismatchedTaskIntegration {
                task_id,
                integration_event_id,
                field,
            } => write!(
                f,
                "child task {task_id} references integration event {integration_event_id} with mismatched {field}"
            ),
            Self::Io { path, source } => write!(f, "loom event log I/O failed at {path}: {source}"),
            Self::Json { line, source } => match line {
                Some(line) => write!(f, "loom event JSONL parse failed at line {line}: {source}"),
                None => write!(f, "loom event JSON encode failed: {source}"),
            },
        }
    }
}

impl std::error::Error for LoomEventError {}
