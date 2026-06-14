use std::cmp::Ordering;

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
    pub fn root(
        task_id: u64,
        prompt_id: u64,
        summary: impl Into<String>,
    ) -> Result<Self, LoomModelError> {
        let summary = summary.into();
        ensure_not_empty("summary", &summary)?;

        Ok(Self {
            task_id,
            root_task_id: task_id,
            parent_task_id: None,
            prompt_id,
            plan_id: 0,
            integration_group_id: 0,
            summary,
        })
    }

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
pub struct ArtifactRef {
    pub artifact_id: u64,
    pub root_task_id: u64,
    pub task_id: u64,
    pub agent_id: u64,
    pub artifact_kind: String,
    pub content_ref: String,
}

impl ArtifactRef {
    pub fn new(
        artifact_id: u64,
        root_task_id: u64,
        task_id: u64,
        agent_id: u64,
        artifact_kind: impl Into<String>,
        content_ref: impl Into<String>,
    ) -> Result<Self, LoomModelError> {
        let artifact_kind = artifact_kind.into();
        let content_ref = content_ref.into();
        ensure_not_empty("artifact_kind", &artifact_kind)?;
        ensure_not_empty("content_ref", &content_ref)?;

        Ok(Self {
            artifact_id,
            root_task_id,
            task_id,
            agent_id,
            artifact_kind,
            content_ref,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrationReport {
    pub integration_group_id: u64,
    pub included_task_ids: Vec<u64>,
    pub excluded_task_ids: Vec<u64>,
    pub conflict_edges: Vec<DependencyEdge>,
    pub gap_edges: Vec<DependencyEdge>,
    pub repair_task_ids: Vec<u64>,
    pub final_artifact_refs: Vec<u64>,
    pub report_ref: String,
    pub acceptance_status: IntegrationStatus,
}

impl IntegrationReport {
    pub fn accepted(
        integration_group_id: u64,
        included_task_ids: Vec<u64>,
        final_artifact_refs: Vec<u64>,
        report_ref: impl Into<String>,
    ) -> Result<Self, LoomModelError> {
        ensure_not_empty_collection("included_task_ids", &included_task_ids)?;
        ensure_not_empty_collection("final_artifact_refs", &final_artifact_refs)?;
        let report_ref = report_ref.into();
        ensure_not_empty("report_ref", &report_ref)?;

        Ok(Self {
            integration_group_id,
            included_task_ids,
            excluded_task_ids: Vec::new(),
            conflict_edges: Vec::new(),
            gap_edges: Vec::new(),
            repair_task_ids: Vec::new(),
            final_artifact_refs,
            report_ref,
            acceptance_status: IntegrationStatus::Accepted,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationStatus {
    Accepted,
    NeedsRepair,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AgentDecommissionPacket {
    pub packet_id: u64,
    pub agent_id: u64,
    pub root_task_id: u64,
    pub task_id: u64,
    pub prompt_id: u64,
    pub plan_id: u64,
    pub final_status: String,
    pub deliverable_refs: Vec<u64>,
    pub self_summary_ref: String,
}

pub fn write_decommission_packets_jsonl<'a, I, P>(path: P, packets: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a AgentDecommissionPacket>,
    P: AsRef<Path>,
{
    let packets = packets.into_iter().collect::<Vec<_>>();
    validate_unique_packet_ids(packets.iter().copied())?;

    let file = File::create(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let mut writer = BufWriter::new(file);

    for packet in packets {
        serde_json::to_writer(&mut writer, packet).map_err(|source| LoomModelError::Json {
            line: None,
            source: source.to_string(),
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| LoomModelError::Io {
                path: path.as_ref().display().to_string(),
                source: source.to_string(),
            })?;
    }

    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

pub fn read_decommission_packets_jsonl<P>(
    path: P,
) -> Result<Vec<AgentDecommissionPacket>, LoomModelError>
where
    P: AsRef<Path>,
{
    let file = File::open(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let reader = BufReader::new(file);
    let mut packets = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let packet: AgentDecommissionPacket =
            serde_json::from_str(&line).map_err(|source| LoomModelError::Json {
                line: Some(line_number),
                source: source.to_string(),
            })?;
        packets.push(packet);
    }

    validate_unique_packet_ids(packets.iter())?;

    Ok(packets)
}

pub fn append_decommission_packet_jsonl<P>(
    path: P,
    packet: &AgentDecommissionPacket,
) -> Result<(), LoomModelError>
where
    P: AsRef<Path>,
{
    let mut packets = if path.as_ref().exists() {
        read_decommission_packets_jsonl(path.as_ref())?
    } else {
        Vec::new()
    };
    packets.push(packet.clone());
    validate_unique_packet_ids(packets.iter())?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, packet).map_err(|source| LoomModelError::Json {
        line: None,
        source: source.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

fn validate_unique_packet_ids<'a, I>(packets: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a AgentDecommissionPacket>,
{
    let mut packet_ids = HashSet::new();

    for packet in packets {
        if !packet_ids.insert(packet.packet_id) {
            return Err(LoomModelError::DuplicatePacketId(packet.packet_id));
        }
    }

    Ok(())
}

impl AgentDecommissionPacket {
    #[allow(clippy::too_many_arguments)]
    pub fn completed(
        packet_id: u64,
        agent_id: u64,
        root_task_id: u64,
        task_id: u64,
        prompt_id: u64,
        plan_id: u64,
        deliverable_refs: Vec<u64>,
        self_summary_ref: impl Into<String>,
    ) -> Result<Self, LoomModelError> {
        ensure_not_empty_collection("deliverable_refs", &deliverable_refs)?;
        let self_summary_ref = self_summary_ref.into();
        ensure_not_empty("self_summary_ref", &self_summary_ref)?;

        Ok(Self {
            packet_id,
            agent_id,
            root_task_id,
            task_id,
            prompt_id,
            plan_id,
            final_status: "completed".to_string(),
            deliverable_refs,
            self_summary_ref,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryNode {
    pub memory_node_id: u64,
    pub kind: MemoryNodeKind,
    pub root_task_id: u64,
    pub task_id: u64,
    pub packet_id: u64,
    pub evidence_refs: Vec<String>,
    pub summary: String,
}

impl MemoryNode {
    pub fn from_packet(
        memory_node_id: u64,
        kind: MemoryNodeKind,
        root_task_id: u64,
        task_id: u64,
        packet_id: u64,
        evidence_refs: Vec<String>,
        summary: impl Into<String>,
    ) -> Result<Self, LoomModelError> {
        ensure_not_empty_collection("evidence_refs", &evidence_refs)?;
        let summary = summary.into();
        ensure_not_empty("summary", &summary)?;

        Ok(Self {
            memory_node_id,
            kind,
            root_task_id,
            task_id,
            packet_id,
            evidence_refs,
            summary,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryNodeKind {
    Episode,
    Decision,
    Artifact,
    Heuristic,
    Failure,
    Preference,
    OpenLoop,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientAxis {
    pub axis_id: u64,
    pub name: String,
    pub low_anchor: String,
    pub high_anchor: String,
    pub confidence: f32,
}

impl GradientAxis {
    pub fn new(
        axis_id: u64,
        name: impl Into<String>,
        low_anchor: impl Into<String>,
        high_anchor: impl Into<String>,
        confidence: f32,
    ) -> Result<Self, LoomModelError> {
        let name = name.into();
        let low_anchor = low_anchor.into();
        let high_anchor = high_anchor.into();
        ensure_not_empty("name", &name)?;
        ensure_not_empty("low_anchor", &low_anchor)?;
        ensure_not_empty("high_anchor", &high_anchor)?;

        Ok(Self {
            axis_id,
            name,
            low_anchor,
            high_anchor,
            confidence,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientSpace {
    pub gradient_space_id: u64,
    pub name: String,
    pub version: u64,
    pub axes: Vec<GradientAxis>,
}

impl GradientSpace {
    pub fn new(
        gradient_space_id: u64,
        name: impl Into<String>,
        version: u64,
        axes: Vec<GradientAxis>,
    ) -> Result<Self, LoomModelError> {
        let name = name.into();
        ensure_not_empty("name", &name)?;
        ensure_not_empty_collection("axes", &axes)?;

        Ok(Self {
            gradient_space_id,
            name,
            version,
            axes,
        })
    }

    pub fn place_memory_node(
        &self,
        placement_id: u64,
        node: &MemoryNode,
        coordinates: Vec<f32>,
        placement_evidence_ref: impl Into<String>,
    ) -> Result<MemoryPlacement, LoomModelError> {
        if coordinates.len() != self.axes.len() {
            return Err(LoomModelError::InvalidNumericField {
                field: "coordinates",
                reason: "must match gradient axis count",
            });
        }

        let placement_evidence_ref = placement_evidence_ref.into();
        ensure_not_empty("placement_evidence_ref", &placement_evidence_ref)?;

        Ok(MemoryPlacement {
            placement_id,
            memory_node_id: node.memory_node_id,
            gradient_space_id: self.gradient_space_id,
            gradient_space_version: self.version,
            coordinates,
            placement_evidence_ref,
        })
    }

    pub fn memory_candidates(
        &self,
        placements: &[MemoryPlacement],
        query_coordinates: Vec<f32>,
        radius_sq: f32,
        budget: usize,
    ) -> Result<Vec<MemoryCandidate>, LoomModelError> {
        Ok(self
            .memory_candidate_report(placements, query_coordinates, radius_sq, budget, 0)?
            .candidates)
    }

    pub fn memory_candidate_report(
        &self,
        placements: &[MemoryPlacement],
        query_coordinates: Vec<f32>,
        radius_sq: f32,
        budget: usize,
        target_min_candidates: usize,
    ) -> Result<MemoryCandidateReport, LoomModelError> {
        if query_coordinates.len() != self.axes.len() {
            return Err(LoomModelError::InvalidNumericField {
                field: "query_coordinates",
                reason: "must match gradient axis count",
            });
        }

        if radius_sq < 0.0 {
            return Err(LoomModelError::InvalidNumericField {
                field: "radius_sq",
                reason: "must be non-negative",
            });
        }

        let mut hard_masked_placements = 0;
        let mut candidates = Vec::new();
        for placement in placements {
            if placement.gradient_space_id != self.gradient_space_id
                || placement.gradient_space_version != self.version
            {
                continue;
            }
            hard_masked_placements += 1;

            if placement.coordinates.len() != self.axes.len() {
                return Err(LoomModelError::InvalidNumericField {
                    field: "placement.coordinates",
                    reason: "must match gradient axis count",
                });
            }

            let distance_sq = placement
                .coordinates
                .iter()
                .zip(query_coordinates.iter())
                .map(|(placement_coordinate, query_coordinate)| {
                    let delta = placement_coordinate - query_coordinate;
                    delta * delta
                })
                .sum::<f32>();

            if distance_sq <= radius_sq {
                candidates.push(MemoryCandidate {
                    memory_node_id: placement.memory_node_id,
                    placement_id: placement.placement_id,
                    gradient_space_id: placement.gradient_space_id,
                    gradient_space_version: placement.gradient_space_version,
                    distance_sq,
                });
            }
        }

        candidates.sort_by(|left, right| {
            left.distance_sq
                .partial_cmp(&right.distance_sq)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.memory_node_id.cmp(&right.memory_node_id))
                .then_with(|| left.placement_id.cmp(&right.placement_id))
        });
        let radius_matched_candidates = candidates.len();
        candidates.truncate(budget);
        let returned_candidates = candidates.len();

        Ok(MemoryCandidateReport {
            total_placements: placements.len(),
            hard_masked_placements,
            radius_matched_candidates,
            returned_candidates,
            target_min_candidates,
            target_met: radius_matched_candidates >= target_min_candidates,
            candidates,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryPlacement {
    pub placement_id: u64,
    pub memory_node_id: u64,
    pub gradient_space_id: u64,
    pub gradient_space_version: u64,
    pub coordinates: Vec<f32>,
    pub placement_evidence_ref: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryCandidate {
    pub memory_node_id: u64,
    pub placement_id: u64,
    pub gradient_space_id: u64,
    pub gradient_space_version: u64,
    pub distance_sq: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryCandidateReport {
    pub total_placements: usize,
    pub hard_masked_placements: usize,
    pub radius_matched_candidates: usize,
    pub returned_candidates: usize,
    pub target_min_candidates: usize,
    pub target_met: bool,
    pub candidates: Vec<MemoryCandidate>,
}

impl MemoryCandidateReport {
    pub fn hard_mask_violation_rate(&self) -> f32 {
        0.0
    }

    pub fn scanned_candidate_reduction(&self) -> f32 {
        if self.total_placements == 0 {
            return 0.0;
        }

        1.0 - (self.returned_candidates as f32 / self.total_placements as f32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum TopologyProposalKind {
    Axis,
    RoutePolicy,
    AgentProfile,
    MemoryIndexVersion,
    BenchmarkRubric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum TopologyProposalStatus {
    Drafted,
    Tested,
    Shadowed,
    Canaried,
    Approved,
    Committed,
    Rejected,
    Superseded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum TopologySnapshotStatus {
    Active,
    Superseded,
    RolledBack,
}

impl TopologyProposalStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Drafted => "Drafted",
            Self::Tested => "Tested",
            Self::Shadowed => "Shadowed",
            Self::Canaried => "Canaried",
            Self::Approved => "Approved",
            Self::Committed => "Committed",
            Self::Rejected => "Rejected",
            Self::Superseded => "Superseded",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TopologyProposal {
    pub topology_proposal_id: u64,
    pub proposal_kind: TopologyProposalKind,
    pub proposer_ref: String,
    pub change_set_ref: String,
    pub evidence_refs: Vec<String>,
    pub benchmark_report_refs: Vec<String>,
    pub shadow_report_refs: Vec<String>,
    pub canary_report_refs: Vec<String>,
    pub approval_refs: Vec<String>,
    pub status: TopologyProposalStatus,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TopologySnapshot {
    pub topology_snapshot_id: u64,
    pub parent_snapshot_id: Option<u64>,
    pub source_proposal_id: u64,
    pub agent_registry_version: String,
    pub gradient_space_versions: Vec<String>,
    pub memory_index_versions: Vec<String>,
    pub route_policy_versions: Vec<String>,
    pub hard_constraint_policy_version: String,
    pub status: TopologySnapshotStatus,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct RollbackRecord {
    pub rollback_id: u64,
    pub from_topology_snapshot_id: u64,
    pub to_topology_snapshot_id: u64,
    pub reason: String,
    pub triggered_by_ref: String,
    pub affected_route_decision_refs: Vec<String>,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyGovernanceStore {
    pub proposals: Vec<TopologyProposal>,
    pub snapshots: Vec<TopologySnapshot>,
    pub rollback_records: Vec<RollbackRecord>,
}

impl RollbackRecord {
    pub fn new(
        rollback_id: u64,
        from_topology_snapshot_id: u64,
        to_topology_snapshot_id: u64,
        reason: impl Into<String>,
        triggered_by_ref: impl Into<String>,
        affected_route_decision_refs: Vec<String>,
        created_at_ms: u64,
    ) -> Result<Self, LoomModelError> {
        let reason = reason.into();
        let triggered_by_ref = triggered_by_ref.into();
        ensure_not_empty("reason", &reason)?;
        ensure_not_empty("triggered_by_ref", &triggered_by_ref)?;
        ensure_not_empty_collection(
            "affected_route_decision_refs",
            &affected_route_decision_refs,
        )?;

        if to_topology_snapshot_id == from_topology_snapshot_id {
            return Err(LoomModelError::InvalidNumericField {
                field: "to_topology_snapshot_id",
                reason: "must differ from from_topology_snapshot_id",
            });
        }

        Ok(Self {
            rollback_id,
            from_topology_snapshot_id,
            to_topology_snapshot_id,
            reason,
            triggered_by_ref,
            affected_route_decision_refs,
            created_at_ms,
        })
    }
}

impl TopologyGovernanceStore {
    pub fn new(
        proposals: Vec<TopologyProposal>,
        snapshots: Vec<TopologySnapshot>,
        rollback_records: Vec<RollbackRecord>,
    ) -> Result<Self, LoomModelError> {
        validate_topology_governance_records(&proposals, &snapshots, &rollback_records)?;

        Ok(Self {
            proposals,
            snapshots,
            rollback_records,
        })
    }

    pub fn active_topology_snapshot(&self) -> Result<&TopologySnapshot, LoomModelError> {
        self.snapshots
            .iter()
            .filter(|snapshot| snapshot.status == TopologySnapshotStatus::Active)
            .max_by_key(|snapshot| (snapshot.created_at_ms, snapshot.topology_snapshot_id))
            .ok_or(LoomModelError::NoActiveTopologySnapshot)
    }

    pub fn apply_rollback(
        mut self,
        rollback_record: RollbackRecord,
    ) -> Result<Self, LoomModelError> {
        let mut rollback_records = self.rollback_records.clone();
        rollback_records.push(rollback_record.clone());
        validate_topology_governance_records(&self.proposals, &self.snapshots, &rollback_records)?;

        for snapshot in &mut self.snapshots {
            if snapshot.topology_snapshot_id == rollback_record.to_topology_snapshot_id {
                snapshot.status = TopologySnapshotStatus::Active;
            } else if snapshot.topology_snapshot_id == rollback_record.from_topology_snapshot_id {
                snapshot.status = TopologySnapshotStatus::RolledBack;
            } else if snapshot.status == TopologySnapshotStatus::Active {
                snapshot.status = TopologySnapshotStatus::Superseded;
            }
        }

        self.rollback_records = rollback_records;
        validate_topology_governance_records(
            &self.proposals,
            &self.snapshots,
            &self.rollback_records,
        )?;

        Ok(self)
    }
}

impl TopologyProposal {
    pub fn draft(
        topology_proposal_id: u64,
        proposal_kind: TopologyProposalKind,
        proposer_ref: impl Into<String>,
        change_set_ref: impl Into<String>,
        evidence_refs: Vec<String>,
        created_at_ms: u64,
    ) -> Result<Self, LoomModelError> {
        let proposer_ref = proposer_ref.into();
        let change_set_ref = change_set_ref.into();
        ensure_not_empty("proposer_ref", &proposer_ref)?;
        ensure_not_empty("change_set_ref", &change_set_ref)?;
        ensure_not_empty_collection("evidence_refs", &evidence_refs)?;

        Ok(Self {
            topology_proposal_id,
            proposal_kind,
            proposer_ref,
            change_set_ref,
            evidence_refs,
            benchmark_report_refs: Vec::new(),
            shadow_report_refs: Vec::new(),
            canary_report_refs: Vec::new(),
            approval_refs: Vec::new(),
            status: TopologyProposalStatus::Drafted,
            created_at_ms,
            updated_at_ms: created_at_ms,
        })
    }

    pub fn mark_tested(
        mut self,
        benchmark_report_refs: Vec<String>,
        updated_at_ms: u64,
    ) -> Result<Self, LoomModelError> {
        ensure_not_empty_collection("benchmark_report_refs", &benchmark_report_refs)?;
        ensure_increasing_timestamp(self.updated_at_ms, updated_at_ms)?;

        self.benchmark_report_refs = benchmark_report_refs;
        self.status = TopologyProposalStatus::Tested;
        self.updated_at_ms = updated_at_ms;

        Ok(self)
    }

    pub fn mark_shadowed(
        mut self,
        shadow_report_refs: Vec<String>,
        updated_at_ms: u64,
    ) -> Result<Self, LoomModelError> {
        ensure_status(
            self.status,
            TopologyProposalStatus::Tested,
            TopologyProposalStatus::Shadowed,
        )?;
        ensure_not_empty_collection("shadow_report_refs", &shadow_report_refs)?;
        ensure_increasing_timestamp(self.updated_at_ms, updated_at_ms)?;

        self.shadow_report_refs = shadow_report_refs;
        self.status = TopologyProposalStatus::Shadowed;
        self.updated_at_ms = updated_at_ms;

        Ok(self)
    }

    pub fn mark_approved(
        mut self,
        approval_refs: Vec<String>,
        updated_at_ms: u64,
    ) -> Result<Self, LoomModelError> {
        ensure_status(
            self.status,
            TopologyProposalStatus::Shadowed,
            TopologyProposalStatus::Approved,
        )?;
        ensure_not_empty_collection("approval_refs", &approval_refs)?;
        ensure_increasing_timestamp(self.updated_at_ms, updated_at_ms)?;

        self.approval_refs = approval_refs;
        self.status = TopologyProposalStatus::Approved;
        self.updated_at_ms = updated_at_ms;

        Ok(self)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit(
        mut self,
        topology_snapshot_id: u64,
        parent_snapshot_id: Option<u64>,
        agent_registry_version: impl Into<String>,
        gradient_space_versions: Vec<String>,
        memory_index_versions: Vec<String>,
        route_policy_versions: Vec<String>,
        hard_constraint_policy_version: impl Into<String>,
        updated_at_ms: u64,
    ) -> Result<(Self, TopologySnapshot), LoomModelError> {
        ensure_status(
            self.status,
            TopologyProposalStatus::Approved,
            TopologyProposalStatus::Committed,
        )?;

        let agent_registry_version = agent_registry_version.into();
        let hard_constraint_policy_version = hard_constraint_policy_version.into();
        ensure_not_empty("agent_registry_version", &agent_registry_version)?;
        ensure_not_empty_collection("gradient_space_versions", &gradient_space_versions)?;
        ensure_not_empty_collection("memory_index_versions", &memory_index_versions)?;
        ensure_not_empty_collection("route_policy_versions", &route_policy_versions)?;
        ensure_not_empty(
            "hard_constraint_policy_version",
            &hard_constraint_policy_version,
        )?;
        ensure_increasing_timestamp(self.updated_at_ms, updated_at_ms)?;

        self.status = TopologyProposalStatus::Committed;
        self.updated_at_ms = updated_at_ms;

        Ok((
            self.clone(),
            TopologySnapshot {
                topology_snapshot_id,
                parent_snapshot_id,
                source_proposal_id: self.topology_proposal_id,
                agent_registry_version,
                gradient_space_versions,
                memory_index_versions,
                route_policy_versions,
                hard_constraint_policy_version,
                status: TopologySnapshotStatus::Active,
                created_at_ms: updated_at_ms,
            },
        ))
    }
}

pub fn write_topology_proposals_jsonl<'a, I, P>(path: P, proposals: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a TopologyProposal>,
    P: AsRef<Path>,
{
    let proposals = proposals.into_iter().collect::<Vec<_>>();
    validate_unique_topology_proposal_ids(proposals.iter().copied())?;

    let file = File::create(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let mut writer = BufWriter::new(file);

    for proposal in proposals {
        serde_json::to_writer(&mut writer, proposal).map_err(|source| LoomModelError::Json {
            line: None,
            source: source.to_string(),
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| LoomModelError::Io {
                path: path.as_ref().display().to_string(),
                source: source.to_string(),
            })?;
    }

    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

pub fn read_topology_proposals_jsonl<P>(path: P) -> Result<Vec<TopologyProposal>, LoomModelError>
where
    P: AsRef<Path>,
{
    let file = File::open(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let reader = BufReader::new(file);
    let mut proposals = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let proposal: TopologyProposal =
            serde_json::from_str(&line).map_err(|source| LoomModelError::Json {
                line: Some(line_number),
                source: source.to_string(),
            })?;
        proposals.push(proposal);
    }

    validate_unique_topology_proposal_ids(proposals.iter())?;

    Ok(proposals)
}

pub fn append_topology_proposal_jsonl<P>(
    path: P,
    proposal: &TopologyProposal,
) -> Result<(), LoomModelError>
where
    P: AsRef<Path>,
{
    let mut proposals = if path.as_ref().exists() {
        read_topology_proposals_jsonl(path.as_ref())?
    } else {
        Vec::new()
    };
    proposals.push(proposal.clone());
    validate_unique_topology_proposal_ids(proposals.iter())?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, proposal).map_err(|source| LoomModelError::Json {
        line: None,
        source: source.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

fn validate_unique_topology_proposal_ids<'a, I>(proposals: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a TopologyProposal>,
{
    let mut topology_proposal_ids = HashSet::new();

    for proposal in proposals {
        if !topology_proposal_ids.insert(proposal.topology_proposal_id) {
            return Err(LoomModelError::DuplicateTopologyProposalId(
                proposal.topology_proposal_id,
            ));
        }
    }

    Ok(())
}

pub fn write_topology_snapshots_jsonl<'a, I, P>(path: P, snapshots: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a TopologySnapshot>,
    P: AsRef<Path>,
{
    let snapshots = snapshots.into_iter().collect::<Vec<_>>();
    validate_unique_topology_snapshot_ids(snapshots.iter().copied())?;

    let file = File::create(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let mut writer = BufWriter::new(file);

    for snapshot in snapshots {
        serde_json::to_writer(&mut writer, snapshot).map_err(|source| LoomModelError::Json {
            line: None,
            source: source.to_string(),
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| LoomModelError::Io {
                path: path.as_ref().display().to_string(),
                source: source.to_string(),
            })?;
    }

    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

pub fn read_topology_snapshots_jsonl<P>(path: P) -> Result<Vec<TopologySnapshot>, LoomModelError>
where
    P: AsRef<Path>,
{
    let file = File::open(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let reader = BufReader::new(file);
    let mut snapshots = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let snapshot: TopologySnapshot =
            serde_json::from_str(&line).map_err(|source| LoomModelError::Json {
                line: Some(line_number),
                source: source.to_string(),
            })?;
        snapshots.push(snapshot);
    }

    validate_unique_topology_snapshot_ids(snapshots.iter())?;

    Ok(snapshots)
}

pub fn append_topology_snapshot_jsonl<P>(
    path: P,
    snapshot: &TopologySnapshot,
) -> Result<(), LoomModelError>
where
    P: AsRef<Path>,
{
    let mut snapshots = if path.as_ref().exists() {
        read_topology_snapshots_jsonl(path.as_ref())?
    } else {
        Vec::new()
    };
    snapshots.push(snapshot.clone());
    validate_unique_topology_snapshot_ids(snapshots.iter())?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, snapshot).map_err(|source| LoomModelError::Json {
        line: None,
        source: source.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

fn validate_unique_topology_snapshot_ids<'a, I>(snapshots: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a TopologySnapshot>,
{
    let mut topology_snapshot_ids = HashSet::new();

    for snapshot in snapshots {
        if !topology_snapshot_ids.insert(snapshot.topology_snapshot_id) {
            return Err(LoomModelError::DuplicateTopologySnapshotId(
                snapshot.topology_snapshot_id,
            ));
        }
    }

    Ok(())
}

pub fn write_rollback_records_jsonl<'a, I, P>(path: P, records: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a RollbackRecord>,
    P: AsRef<Path>,
{
    let records = records.into_iter().collect::<Vec<_>>();
    validate_unique_rollback_ids(records.iter().copied())?;

    let file = File::create(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let mut writer = BufWriter::new(file);

    for record in records {
        serde_json::to_writer(&mut writer, record).map_err(|source| LoomModelError::Json {
            line: None,
            source: source.to_string(),
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| LoomModelError::Io {
                path: path.as_ref().display().to_string(),
                source: source.to_string(),
            })?;
    }

    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

pub fn read_rollback_records_jsonl<P>(path: P) -> Result<Vec<RollbackRecord>, LoomModelError>
where
    P: AsRef<Path>,
{
    let file = File::open(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let record: RollbackRecord =
            serde_json::from_str(&line).map_err(|source| LoomModelError::Json {
                line: Some(line_number),
                source: source.to_string(),
            })?;
        records.push(record);
    }

    validate_unique_rollback_ids(records.iter())?;

    Ok(records)
}

pub fn append_rollback_record_jsonl<P>(
    path: P,
    record: &RollbackRecord,
) -> Result<(), LoomModelError>
where
    P: AsRef<Path>,
{
    let mut records = if path.as_ref().exists() {
        read_rollback_records_jsonl(path.as_ref())?
    } else {
        Vec::new()
    };
    records.push(record.clone());
    validate_unique_rollback_ids(records.iter())?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, record).map_err(|source| LoomModelError::Json {
        line: None,
        source: source.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

fn validate_unique_rollback_ids<'a, I>(records: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a RollbackRecord>,
{
    let mut rollback_ids = HashSet::new();

    for record in records {
        if !rollback_ids.insert(record.rollback_id) {
            return Err(LoomModelError::DuplicateRollbackId(record.rollback_id));
        }
    }

    Ok(())
}

pub fn read_topology_governance_store_jsonl<P, S, R>(
    proposals_path: P,
    snapshots_path: S,
    rollback_records_path: R,
) -> Result<TopologyGovernanceStore, LoomModelError>
where
    P: AsRef<Path>,
    S: AsRef<Path>,
    R: AsRef<Path>,
{
    TopologyGovernanceStore::new(
        read_topology_proposals_jsonl(proposals_path)?,
        read_topology_snapshots_jsonl(snapshots_path)?,
        read_rollback_records_jsonl(rollback_records_path)?,
    )
}

fn validate_topology_governance_records(
    proposals: &[TopologyProposal],
    snapshots: &[TopologySnapshot],
    rollback_records: &[RollbackRecord],
) -> Result<(), LoomModelError> {
    validate_unique_topology_proposal_ids(proposals.iter())?;
    validate_unique_topology_snapshot_ids(snapshots.iter())?;
    validate_unique_rollback_ids(rollback_records.iter())?;

    let proposal_ids = proposals
        .iter()
        .map(|proposal| proposal.topology_proposal_id)
        .collect::<HashSet<_>>();
    let snapshot_ids = snapshots
        .iter()
        .map(|snapshot| snapshot.topology_snapshot_id)
        .collect::<HashSet<_>>();

    for snapshot in snapshots {
        if !proposal_ids.contains(&snapshot.source_proposal_id) {
            return Err(LoomModelError::UnknownTopologyProposalId {
                field: "snapshot.source_proposal_id",
                topology_proposal_id: snapshot.source_proposal_id,
            });
        }
    }

    for record in rollback_records {
        if !snapshot_ids.contains(&record.from_topology_snapshot_id) {
            return Err(LoomModelError::UnknownTopologySnapshotId {
                field: "rollback.from_topology_snapshot_id",
                topology_snapshot_id: record.from_topology_snapshot_id,
            });
        }

        if !snapshot_ids.contains(&record.to_topology_snapshot_id) {
            return Err(LoomModelError::UnknownTopologySnapshotId {
                field: "rollback.to_topology_snapshot_id",
                topology_snapshot_id: record.to_topology_snapshot_id,
            });
        }
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EvaluatorConfig {
    pub model: String,
    pub rubric_version: String,
    pub prompt_version: String,
    pub scoring_schema_version: String,
    pub temperature: f32,
    pub seed: Option<u64>,
}

impl EvaluatorConfig {
    pub fn new(
        model: impl Into<String>,
        rubric_version: impl Into<String>,
        prompt_version: impl Into<String>,
        scoring_schema_version: impl Into<String>,
        temperature: f32,
        seed: Option<u64>,
    ) -> Result<Self, LoomModelError> {
        let model = model.into();
        let rubric_version = rubric_version.into();
        let prompt_version = prompt_version.into();
        let scoring_schema_version = scoring_schema_version.into();
        ensure_not_empty("model", &model)?;
        ensure_not_empty("rubric_version", &rubric_version)?;
        ensure_not_empty("prompt_version", &prompt_version)?;
        ensure_not_empty("scoring_schema_version", &scoring_schema_version)?;

        Ok(Self {
            model,
            rubric_version,
            prompt_version,
            scoring_schema_version,
            temperature,
            seed,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EvaluationFixture {
    pub evaluation_id: u64,
    pub evaluator: EvaluatorConfig,
    pub task_id: u64,
    pub artifact_refs: Vec<u64>,
    pub score: f32,
    pub rationale: String,
}

impl EvaluationFixture {
    pub fn new(
        evaluation_id: u64,
        evaluator: EvaluatorConfig,
        task_id: u64,
        artifact_refs: Vec<u64>,
        score: f32,
        rationale: impl Into<String>,
    ) -> Result<Self, LoomModelError> {
        ensure_not_empty_collection("artifact_refs", &artifact_refs)?;
        if !(0.0..=1.0).contains(&score) {
            return Err(LoomModelError::InvalidNumericField {
                field: "score",
                reason: "must be between 0 and 1",
            });
        }

        let rationale = rationale.into();
        ensure_not_empty("rationale", &rationale)?;

        Ok(Self {
            evaluation_id,
            evaluator,
            task_id,
            artifact_refs,
            score,
            rationale,
        })
    }
}

pub fn write_evaluation_fixtures_jsonl<'a, I, P>(path: P, fixtures: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a EvaluationFixture>,
    P: AsRef<Path>,
{
    let fixtures = fixtures.into_iter().collect::<Vec<_>>();
    validate_unique_evaluation_ids(fixtures.iter().copied())?;

    let file = File::create(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let mut writer = BufWriter::new(file);

    for fixture in fixtures {
        serde_json::to_writer(&mut writer, fixture).map_err(|source| LoomModelError::Json {
            line: None,
            source: source.to_string(),
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| LoomModelError::Io {
                path: path.as_ref().display().to_string(),
                source: source.to_string(),
            })?;
    }

    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

pub fn read_evaluation_fixtures_jsonl<P>(path: P) -> Result<Vec<EvaluationFixture>, LoomModelError>
where
    P: AsRef<Path>,
{
    let file = File::open(path.as_ref()).map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;
    let reader = BufReader::new(file);
    let mut fixtures = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let fixture: EvaluationFixture =
            serde_json::from_str(&line).map_err(|source| LoomModelError::Json {
                line: Some(line_number),
                source: source.to_string(),
            })?;
        fixtures.push(fixture);
    }

    validate_unique_evaluation_ids(fixtures.iter())?;

    Ok(fixtures)
}

pub fn append_evaluation_fixture_jsonl<P>(
    path: P,
    fixture: &EvaluationFixture,
) -> Result<(), LoomModelError>
where
    P: AsRef<Path>,
{
    let mut fixtures = if path.as_ref().exists() {
        read_evaluation_fixtures_jsonl(path.as_ref())?
    } else {
        Vec::new()
    };
    fixtures.push(fixture.clone());
    validate_unique_evaluation_ids(fixtures.iter())?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, fixture).map_err(|source| LoomModelError::Json {
        line: None,
        source: source.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| LoomModelError::Io {
            path: path.as_ref().display().to_string(),
            source: source.to_string(),
        })?;
    writer.flush().map_err(|source| LoomModelError::Io {
        path: path.as_ref().display().to_string(),
        source: source.to_string(),
    })?;

    Ok(())
}

fn validate_unique_evaluation_ids<'a, I>(fixtures: I) -> Result<(), LoomModelError>
where
    I: IntoIterator<Item = &'a EvaluationFixture>,
{
    let mut evaluation_ids = HashSet::new();

    for fixture in fixtures {
        if !evaluation_ids.insert(fixture.evaluation_id) {
            return Err(LoomModelError::DuplicateEvaluationId(fixture.evaluation_id));
        }
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoomModelError {
    EmptyField(&'static str),
    EmptyCollection(&'static str),
    MissingTaskArtifact(u64),
    MissingRouteCandidate(u64),
    DuplicatePacketId(u64),
    DuplicateEvaluationId(u64),
    DuplicateTopologyProposalId(u64),
    DuplicateTopologySnapshotId(u64),
    DuplicateRollbackId(u64),
    NoActiveTopologySnapshot,
    UnknownTopologyProposalId {
        field: &'static str,
        topology_proposal_id: u64,
    },
    UnknownTopologySnapshotId {
        field: &'static str,
        topology_snapshot_id: u64,
    },
    InvalidStateTransition {
        from: &'static str,
        to: &'static str,
    },
    InvalidNumericField {
        field: &'static str,
        reason: &'static str,
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

impl std::fmt::Display for LoomModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "`{field}` must not be empty"),
            Self::EmptyCollection(field) => write!(f, "`{field}` must not be empty"),
            Self::MissingTaskArtifact(task_id) => {
                write!(f, "`completed_child_artifacts` is missing task {task_id}")
            }
            Self::MissingRouteCandidate(task_id) => {
                write!(
                    f,
                    "route result for task {task_id} has no available candidate"
                )
            }
            Self::DuplicatePacketId(packet_id) => {
                write!(f, "duplicate decommission packet id {packet_id}")
            }
            Self::DuplicateEvaluationId(evaluation_id) => {
                write!(f, "duplicate evaluation fixture id {evaluation_id}")
            }
            Self::DuplicateTopologyProposalId(topology_proposal_id) => {
                write!(f, "duplicate topology proposal id {topology_proposal_id}")
            }
            Self::DuplicateTopologySnapshotId(topology_snapshot_id) => {
                write!(f, "duplicate topology snapshot id {topology_snapshot_id}")
            }
            Self::DuplicateRollbackId(rollback_id) => {
                write!(f, "duplicate rollback id {rollback_id}")
            }
            Self::NoActiveTopologySnapshot => {
                write!(f, "no active topology snapshot is available")
            }
            Self::UnknownTopologyProposalId {
                field,
                topology_proposal_id,
            } => {
                write!(
                    f,
                    "`{field}` references unknown topology proposal id {topology_proposal_id}"
                )
            }
            Self::UnknownTopologySnapshotId {
                field,
                topology_snapshot_id,
            } => {
                write!(
                    f,
                    "`{field}` references unknown topology snapshot id {topology_snapshot_id}"
                )
            }
            Self::InvalidStateTransition { from, to } => {
                write!(f, "cannot transition topology proposal from {from} to {to}")
            }
            Self::InvalidNumericField { field, reason } => {
                write!(f, "`{field}` {reason}")
            }
            Self::Io { path, source } => {
                write!(f, "decommission packet I/O failed at {path}: {source}")
            }
            Self::Json { line, source } => match line {
                Some(line) => write!(
                    f,
                    "decommission packet JSONL parse failed at line {line}: {source}"
                ),
                None => write!(f, "decommission packet JSON encode failed: {source}"),
            },
        }
    }
}

impl std::error::Error for LoomModelError {}

pub fn ensure_not_empty(field: &'static str, value: &str) -> Result<(), LoomModelError> {
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

fn ensure_increasing_timestamp(current: u64, next: u64) -> Result<(), LoomModelError> {
    if next <= current {
        Err(LoomModelError::InvalidNumericField {
            field: "updated_at_ms",
            reason: "must be greater than the current update time",
        })
    } else {
        Ok(())
    }
}

fn ensure_status(
    actual: TopologyProposalStatus,
    expected: TopologyProposalStatus,
    next: TopologyProposalStatus,
) -> Result<(), LoomModelError> {
    if actual != expected {
        Err(LoomModelError::InvalidStateTransition {
            from: actual.label(),
            to: next.label(),
        })
    } else {
        Ok(())
    }
}
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
