#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskEnvelope {
    pub task_id: u64,
    pub root_task_id: u64,
    pub parent_task_id: Option<u64>,
    pub prompt_id: u64,
    pub plan_id: u64,
    pub integration_group_id: u64,
    pub summary: String,
}

impl TaskEnvelope {
    pub fn child(
        task_id: u64,
        root_task_id: u64,
        parent_task_id: u64,
        prompt_id: u64,
        plan_id: u64,
        integration_group_id: u64,
        summary: impl Into<String>,
    ) -> Result<Self, LoomModelError> {
        let summary = summary.into();
        ensure_not_empty("summary", &summary)?;

        Ok(Self {
            task_id,
            root_task_id,
            parent_task_id: Some(parent_task_id),
            prompt_id,
            plan_id,
            integration_group_id,
            summary,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanNode {
    pub plan_id: u64,
    pub root_task_id: u64,
    pub task_id: u64,
    pub director_agent_id: u64,
    pub decomposition_reason_ref: String,
    pub child_task_ids: Vec<u64>,
    pub dependency_edges: Vec<DependencyEdge>,
    pub integration_group_id: u64,
    pub acceptance_criteria_ref: String,
    pub risk_flags: Vec<String>,
}

impl PlanNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plan_id: u64,
        root_task_id: u64,
        task_id: u64,
        director_agent_id: u64,
        decomposition_reason_ref: impl Into<String>,
        child_task_ids: Vec<u64>,
        dependency_edges: Vec<DependencyEdge>,
        integration_group_id: u64,
        acceptance_criteria_ref: impl Into<String>,
        risk_flags: Vec<String>,
    ) -> Result<Self, LoomModelError> {
        ensure_not_empty_collection("child_task_ids", &child_task_ids)?;
        let decomposition_reason_ref = decomposition_reason_ref.into();
        let acceptance_criteria_ref = acceptance_criteria_ref.into();
        ensure_not_empty("decomposition_reason_ref", &decomposition_reason_ref)?;
        ensure_not_empty("acceptance_criteria_ref", &acceptance_criteria_ref)?;

        Ok(Self {
            plan_id,
            root_task_id,
            task_id,
            director_agent_id,
            decomposition_reason_ref,
            child_task_ids,
            dependency_edges,
            integration_group_id,
            acceptance_criteria_ref,
            risk_flags,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyEdge {
    pub from_task_id: u64,
    pub to_task_id: u64,
    pub kind: DependencyKind,
}

impl DependencyEdge {
    pub const fn new(from_task_id: u64, to_task_id: u64, kind: DependencyKind) -> Self {
        Self {
            from_task_id,
            to_task_id,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyKind {
    Blocks,
    ProvidesEvidence,
    RepairsGap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrationGroup {
    pub integration_group_id: u64,
    pub root_task_id: u64,
    pub parent_task_id: u64,
    pub plan_id: u64,
    pub expected_child_task_ids: Vec<u64>,
    pub join_policy: JoinPolicy,
    pub acceptance_criteria_ref: String,
    pub integration_agent_ids: Vec<u64>,
}

impl IntegrationGroup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        integration_group_id: u64,
        root_task_id: u64,
        parent_task_id: u64,
        plan_id: u64,
        expected_child_task_ids: Vec<u64>,
        join_policy: JoinPolicy,
        acceptance_criteria_ref: impl Into<String>,
        integration_agent_ids: Vec<u64>,
    ) -> Result<Self, LoomModelError> {
        ensure_not_empty_collection("expected_child_task_ids", &expected_child_task_ids)?;
        ensure_not_empty_collection("integration_agent_ids", &integration_agent_ids)?;
        let acceptance_criteria_ref = acceptance_criteria_ref.into();
        ensure_not_empty("acceptance_criteria_ref", &acceptance_criteria_ref)?;

        Ok(Self {
            integration_group_id,
            root_task_id,
            parent_task_id,
            plan_id,
            expected_child_task_ids,
            join_policy,
            acceptance_criteria_ref,
            integration_agent_ids,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinPolicy {
    WaitAll,
    WaitQuorum,
    WaitFirstViable,
    TimeoutThenIntegrate,
    StreamingIncremental,
    HumanGate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoomModelError {
    EmptyField(&'static str),
    EmptyCollection(&'static str),
}

impl std::fmt::Display for LoomModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "`{field}` must not be empty"),
            Self::EmptyCollection(field) => write!(f, "`{field}` must not be empty"),
        }
    }
}

impl std::error::Error for LoomModelError {}

fn ensure_not_empty(field: &'static str, value: &str) -> Result<(), LoomModelError> {
    if value.trim().is_empty() {
        Err(LoomModelError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn ensure_not_empty_collection<T>(field: &'static str, values: &[T]) -> Result<(), LoomModelError> {
    if values.is_empty() {
        Err(LoomModelError::EmptyCollection(field))
    } else {
        Ok(())
    }
}
