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

        Ok(())
    }
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoomEventError {
    DuplicateEventId(u64),
    MissingPayloadSchema,
    MissingPayloadRef,
}

impl std::fmt::Display for LoomEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateEventId(event_id) => {
                write!(f, "duplicate loom event id {event_id}")
            }
            Self::MissingPayloadSchema => write!(f, "loom event payload schema is required"),
            Self::MissingPayloadRef => write!(f, "loom event payload ref is required"),
        }
    }
}

impl std::error::Error for LoomEventError {}
