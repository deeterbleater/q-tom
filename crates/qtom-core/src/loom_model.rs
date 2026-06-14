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

pub fn write_evaluation_fixtures_jsonl<'a, I, P>(
    path: P,
    fixtures: I,
) -> Result<(), LoomModelError>
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

pub fn read_evaluation_fixtures_jsonl<P>(
    path: P,
) -> Result<Vec<EvaluationFixture>, LoomModelError>
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
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
