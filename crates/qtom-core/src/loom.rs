use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
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
        LoomEventType::RouteDecisionRecorded | LoomEventType::TopologyCommitted
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    TopologyCommitted,
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

        Ok(())
    }
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
    let mut topology_commit_count = 0;
    let mut completed_task_ids = HashSet::new();
    let mut decommissioned_task_ids = HashSet::new();

    for event in events {
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
                    completed_task_ids.insert(task_id);
                }
            }
            LoomEventType::AgentDecommissioned => {
                decommission_count += 1;
                if let Some(task_id) = event.task_id {
                    decommissioned_task_ids.insert(task_id);
                }
            }
            LoomEventType::MemoryNodeCreated => memory_node_count += 1,
            LoomEventType::TopologyCommitted => topology_commit_count += 1,
            _ => {}
        }
    }

    if let Some(task_id) = completed_task_ids
        .difference(&decommissioned_task_ids)
        .min()
        .copied()
    {
        return Err(LoomEventError::MissingTaskDecommission { task_id });
    }

    Ok(ReplayValidationReport {
        event_count: events.len(),
        root_task_count: root_task_ids.len(),
        task_event_count,
        route_decision_count,
        assignment_count,
        completion_count,
        decommission_count,
        memory_node_count,
        topology_commit_count,
    })
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
    pub topology_commit_count: usize,
}

fn required_cause(event_type: LoomEventType) -> Option<LoomEventType> {
    match event_type {
        LoomEventType::TaskAssigned => Some(LoomEventType::RouteDecisionRecorded),
        LoomEventType::ArtifactReady => Some(LoomEventType::ArtifactDeclared),
        LoomEventType::TaskResumed => Some(LoomEventType::TaskBlocked),
        LoomEventType::TopologyCommitted => Some(LoomEventType::TopologyProposed),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoomEventError {
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
}

impl std::fmt::Display for LoomEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
        }
    }
}

impl std::error::Error for LoomEventError {}
