use crate::loom_model::ensure_not_empty;
use crate::{
    AgentDecommissionPacket, ArtifactRef, DependencyEdge, DependencyKind, InMemoryEventLog,
    IntegrationGroup, IntegrationReport, JoinPolicy, LoomEvent, LoomEventType, LoomModelError,
    MemoryNode, MemoryNodeKind, PlanNode, TaskEnvelope,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockDirectorConfig {
    pub director_agent_id: u64,
    pub integration_agent_id: u64,
    pub next_plan_id: u64,
    pub next_integration_group_id: u64,
    pub next_child_task_id: u64,
}

impl Default for MockDirectorConfig {
    fn default() -> Self {
        Self {
            director_agent_id: 500,
            integration_agent_id: 700,
            next_plan_id: 1,
            next_integration_group_id: 1,
            next_child_task_id: 1_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockDirector {
    config: MockDirectorConfig,
}

impl MockDirector {
    pub fn new(config: MockDirectorConfig) -> Self {
        Self { config }
    }

    pub fn split_root_task(
        &self,
        prompt_id: u64,
        root_task_id: u64,
        root_task_graph_id: u64,
        summary: impl Into<String>,
    ) -> Result<DirectorOutput, LoomModelError> {
        let summary = summary.into();
        ensure_not_empty("summary", &summary)?;

        let child_a = TaskEnvelope::child(
            self.config.next_child_task_id,
            root_task_graph_id,
            root_task_id,
            prompt_id,
            self.config.next_plan_id,
            self.config.next_integration_group_id,
            format!("Explore: {summary}"),
        )?;
        let child_b = TaskEnvelope::child(
            self.config.next_child_task_id + 1,
            root_task_graph_id,
            root_task_id,
            prompt_id,
            self.config.next_plan_id,
            self.config.next_integration_group_id,
            format!("Validate: {summary}"),
        )?;
        let children = vec![child_a, child_b];
        let child_task_ids = children
            .iter()
            .map(|child| child.task_id)
            .collect::<Vec<_>>();

        let plan = PlanNode::new(
            self.config.next_plan_id,
            root_task_graph_id,
            root_task_id,
            self.config.director_agent_id,
            format!("inline://decomposition/{}", self.config.next_plan_id),
            child_task_ids.clone(),
            vec![DependencyEdge::new(
                child_task_ids[0],
                child_task_ids[1],
                DependencyKind::ProvidesEvidence,
            )],
            self.config.next_integration_group_id,
            format!("inline://acceptance/{}", self.config.next_plan_id),
            vec!["mock_director_split".to_string()],
        )?;
        let integration_group = IntegrationGroup::new(
            self.config.next_integration_group_id,
            root_task_graph_id,
            root_task_id,
            self.config.next_plan_id,
            child_task_ids,
            JoinPolicy::WaitAll,
            plan.acceptance_criteria_ref.clone(),
            vec![self.config.integration_agent_id],
        )?;

        Ok(DirectorOutput {
            plan,
            children,
            integration_group,
        })
    }
}

impl Default for MockDirector {
    fn default() -> Self {
        Self::new(MockDirectorConfig::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectorOutput {
    pub plan: PlanNode,
    pub children: Vec<TaskEnvelope>,
    pub integration_group: IntegrationGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockConstructorConfig {
    pub agent_id: u64,
    pub next_artifact_id: u64,
    pub next_event_id: u64,
    pub occurred_at_ms: u64,
    pub correlation_id: u64,
}

impl Default for MockConstructorConfig {
    fn default() -> Self {
        Self {
            agent_id: 301,
            next_artifact_id: 900,
            next_event_id: 2_000,
            occurred_at_ms: 10_000,
            correlation_id: 77,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockConstructor {
    config: MockConstructorConfig,
}

impl MockConstructor {
    pub fn new(config: MockConstructorConfig) -> Self {
        Self { config }
    }

    pub fn build_child_task(
        &self,
        task: &TaskEnvelope,
    ) -> Result<ConstructorOutput, LoomModelError> {
        let artifact = ArtifactRef::new(
            self.config.next_artifact_id,
            task.root_task_id,
            task.task_id,
            self.config.agent_id,
            "mock.markdown",
            format!("inline://artifact/{}", self.config.next_artifact_id),
        )?;
        let mut event_log = InMemoryEventLog::new();
        let declared = self.event(
            self.config.next_event_id,
            LoomEventType::ArtifactDeclared,
            task,
            &artifact.content_ref,
            None,
        );
        let ready = self.event(
            self.config.next_event_id + 1,
            LoomEventType::ArtifactReady,
            task,
            &artifact.content_ref,
            Some(declared.event_id),
        );
        let completed = self.event(
            self.config.next_event_id + 2,
            LoomEventType::TaskCompleted,
            task,
            &artifact.content_ref,
            None,
        );
        let decommissioned = self.event(
            self.config.next_event_id + 3,
            LoomEventType::AgentDecommissioned,
            task,
            &format!(
                "inline://decommission/{}/{}",
                self.config.agent_id, task.task_id
            ),
            None,
        );

        event_log
            .append(declared)
            .expect("mock constructor should create valid artifact_declared event");
        event_log
            .append(ready)
            .expect("mock constructor should create valid artifact_ready event");
        event_log
            .append(completed)
            .expect("mock constructor should create valid task_completed event");
        event_log
            .append(decommissioned)
            .expect("mock constructor should create valid agent_decommissioned event");

        Ok(ConstructorOutput {
            artifact,
            event_log,
        })
    }

    fn event(
        &self,
        event_id: u64,
        event_type: LoomEventType,
        task: &TaskEnvelope,
        payload_ref: &str,
        causation_id: Option<u64>,
    ) -> LoomEvent {
        LoomEvent {
            event_id,
            event_type,
            root_task_id: task.root_task_id,
            task_id: Some(task.task_id),
            parent_task_id: task.parent_task_id,
            prompt_id: Some(task.prompt_id),
            agent_id: Some(self.config.agent_id),
            agent_role: Some("constructor".to_string()),
            topology_snapshot_id: None,
            payload_schema: "qtom.mock.constructor.v1".to_string(),
            payload_ref: payload_ref.to_string(),
            occurred_at_ms: self.config.occurred_at_ms + (event_id - self.config.next_event_id),
            causation_id,
            correlation_id: self.config.correlation_id,
        }
    }
}

impl Default for MockConstructor {
    fn default() -> Self {
        Self::new(MockConstructorConfig::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructorOutput {
    pub artifact: ArtifactRef,
    pub event_log: InMemoryEventLog,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockIntegrationConfig {
    pub integration_agent_id: u64,
    pub next_event_id: u64,
    pub occurred_at_ms: u64,
    pub correlation_id: u64,
}

impl Default for MockIntegrationConfig {
    fn default() -> Self {
        Self {
            integration_agent_id: 700,
            next_event_id: 3_000,
            occurred_at_ms: 20_000,
            correlation_id: 77,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockIntegration {
    config: MockIntegrationConfig,
}

impl MockIntegration {
    pub fn new(config: MockIntegrationConfig) -> Self {
        Self { config }
    }

    pub fn integrate_completed_children(
        &self,
        group: &IntegrationGroup,
        completed_child_artifacts: &[ArtifactRef],
    ) -> Result<IntegrationOutput, LoomModelError> {
        for expected_task_id in &group.expected_child_task_ids {
            if !completed_child_artifacts
                .iter()
                .any(|artifact| artifact.task_id == *expected_task_id)
            {
                return Err(LoomModelError::MissingTaskArtifact(*expected_task_id));
            }
        }

        let included_task_ids = group.expected_child_task_ids.clone();
        let final_artifact_refs = completed_child_artifacts
            .iter()
            .filter(|artifact| included_task_ids.contains(&artifact.task_id))
            .map(|artifact| artifact.artifact_id)
            .collect::<Vec<_>>();
        let report = IntegrationReport::accepted(
            group.integration_group_id,
            included_task_ids,
            final_artifact_refs,
            format!("inline://integration/report/{}", group.integration_group_id),
        )?;
        let mut event_log = InMemoryEventLog::new();
        event_log
            .append(LoomEvent {
                event_id: self.config.next_event_id,
                event_type: LoomEventType::IntegrationRequested,
                root_task_id: group.root_task_id,
                task_id: Some(group.parent_task_id),
                parent_task_id: None,
                prompt_id: None,
                agent_id: Some(self.config.integration_agent_id),
                agent_role: Some("integration".to_string()),
                topology_snapshot_id: None,
                payload_schema: "qtom.mock.integration.v1".to_string(),
                payload_ref: report.report_ref.clone(),
                occurred_at_ms: self.config.occurred_at_ms,
                causation_id: None,
                correlation_id: self.config.correlation_id,
            })
            .expect("mock integration should create valid integration_requested event");

        Ok(IntegrationOutput { report, event_log })
    }
}

impl Default for MockIntegration {
    fn default() -> Self {
        Self::new(MockIntegrationConfig::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrationOutput {
    pub report: IntegrationReport,
    pub event_log: InMemoryEventLog,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockCuratorConfig {
    pub curator_agent_id: u64,
    pub next_memory_node_id: u64,
    pub next_event_id: u64,
    pub occurred_at_ms: u64,
    pub correlation_id: u64,
}

impl Default for MockCuratorConfig {
    fn default() -> Self {
        Self {
            curator_agent_id: 800,
            next_memory_node_id: 1_500,
            next_event_id: 4_000,
            occurred_at_ms: 30_000,
            correlation_id: 77,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockCurator {
    config: MockCuratorConfig,
}

impl MockCurator {
    pub fn new(config: MockCuratorConfig) -> Self {
        Self { config }
    }

    pub fn curate_decommission_packet(
        &self,
        packet: &AgentDecommissionPacket,
        decommission_event: &LoomEvent,
    ) -> Result<CuratorOutput, LoomModelError> {
        let memory_node = MemoryNode::from_packet(
            self.config.next_memory_node_id,
            MemoryNodeKind::Episode,
            packet.root_task_id,
            packet.task_id,
            packet.packet_id,
            vec![packet.self_summary_ref.clone()],
            format!(
                "{} agent {} task {}",
                packet.final_status, packet.agent_id, packet.task_id
            ),
        )?;
        let mut event_log = InMemoryEventLog::new();
        event_log
            .append(decommission_event.clone())
            .expect("mock curator should receive valid agent_decommissioned evidence");
        event_log
            .append(LoomEvent {
                event_id: self.config.next_event_id,
                event_type: LoomEventType::MemoryNodeCreated,
                root_task_id: packet.root_task_id,
                task_id: Some(packet.task_id),
                parent_task_id: None,
                prompt_id: Some(packet.prompt_id),
                agent_id: Some(self.config.curator_agent_id),
                agent_role: Some("curator".to_string()),
                topology_snapshot_id: None,
                payload_schema: "qtom.mock.curator.v1".to_string(),
                payload_ref: format!("inline://memory/{}", memory_node.memory_node_id),
                occurred_at_ms: self.config.occurred_at_ms,
                causation_id: Some(decommission_event.event_id),
                correlation_id: self.config.correlation_id,
            })
            .expect("mock curator should create valid memory_node_created event");

        Ok(CuratorOutput {
            memory_node,
            event_log,
        })
    }
}

impl Default for MockCurator {
    fn default() -> Self {
        Self::new(MockCuratorConfig::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CuratorOutput {
    pub memory_node: MemoryNode,
    pub event_log: InMemoryEventLog,
}
