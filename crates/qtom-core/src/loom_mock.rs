use crate::loom_model::ensure_not_empty;
use crate::{
    ArtifactRef, DependencyEdge, DependencyKind, InMemoryEventLog, IntegrationGroup, JoinPolicy,
    LoomEvent, LoomEventType, LoomModelError, PlanNode, TaskEnvelope,
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
