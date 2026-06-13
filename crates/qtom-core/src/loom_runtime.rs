use crate::loom_model::ensure_not_empty;
use crate::{
    AgentDecommissionPacket, ArtifactRef, InMemoryEventLog, LoomEventType, LoomModelError,
    MockConstructor, MockConstructorConfig, TaskEnvelope,
};

pub trait AgentRuntime {
    fn execute(
        &self,
        task: &TaskEnvelope,
        context: &HydratedContext,
    ) -> Result<AgentExecutionResult, LoomModelError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydratedContext {
    pub prompt_ref: String,
    pub tool_refs: Vec<String>,
    pub memory_refs: Vec<String>,
}

impl HydratedContext {
    pub fn new(
        prompt_ref: impl Into<String>,
        tool_refs: Vec<String>,
        memory_refs: Vec<String>,
    ) -> Result<Self, LoomModelError> {
        let prompt_ref = prompt_ref.into();
        ensure_not_empty("prompt_ref", &prompt_ref)?;

        Ok(Self {
            prompt_ref,
            tool_refs,
            memory_refs,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentExecutionResult {
    pub artifacts: Vec<ArtifactRef>,
    pub decommission_packet: AgentDecommissionPacket,
    pub event_log: InMemoryEventLog,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockConstructorRuntimeConfig {
    pub agent_id: u64,
    pub next_artifact_id: u64,
    pub next_packet_id: u64,
    pub next_event_id: u64,
    pub occurred_at_ms: u64,
    pub correlation_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockConstructorRuntime {
    config: MockConstructorRuntimeConfig,
}

impl MockConstructorRuntime {
    pub fn new(config: MockConstructorRuntimeConfig) -> Self {
        Self { config }
    }
}

impl AgentRuntime for MockConstructorRuntime {
    fn execute(
        &self,
        task: &TaskEnvelope,
        _context: &HydratedContext,
    ) -> Result<AgentExecutionResult, LoomModelError> {
        let constructor = MockConstructor::new(MockConstructorConfig {
            agent_id: self.config.agent_id,
            next_artifact_id: self.config.next_artifact_id,
            next_event_id: self.config.next_event_id,
            occurred_at_ms: self.config.occurred_at_ms,
            correlation_id: self.config.correlation_id,
        });
        let output = constructor.build_child_task(task)?;
        let decommission_event = output
            .event_log
            .events_by_type(LoomEventType::AgentDecommissioned)
            .into_iter()
            .next()
            .expect("mock constructor runtime should emit decommission event")
            .clone();
        let decommission_packet = AgentDecommissionPacket::completed(
            self.config.next_packet_id,
            output.artifact.agent_id,
            task.root_task_id,
            task.task_id,
            task.prompt_id,
            task.plan_id,
            vec![output.artifact.artifact_id],
            decommission_event.payload_ref,
        )?;

        Ok(AgentExecutionResult {
            artifacts: vec![output.artifact],
            decommission_packet,
            event_log: output.event_log,
        })
    }
}
