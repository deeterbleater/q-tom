use crate::loom_model::ensure_not_empty;
use crate::{
    DependencyEdge, DependencyKind, IntegrationGroup, JoinPolicy, LoomModelError, PlanNode,
    TaskEnvelope,
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
