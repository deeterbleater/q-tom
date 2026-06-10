use crate::types::{AgentProfile, RouteError};

#[derive(Clone, Debug, PartialEq)]
pub struct AgentRouteTable {
    agent_ids: Vec<u32>,
    vectors: Vec<f32>,
    dimensions: usize,
}

impl AgentRouteTable {
    pub fn from_agents(agents: Vec<AgentProfile>) -> Result<Self, RouteError> {
        if agents.is_empty() {
            return Ok(Self {
                agent_ids: Vec::new(),
                vectors: Vec::new(),
                dimensions: 0,
            });
        }

        let dimensions = agents[0].vector.len();
        let mut agent_ids = Vec::with_capacity(agents.len());
        let mut vectors = Vec::with_capacity(agents.len() * dimensions);

        for agent in agents {
            if agent.vector.len() != dimensions {
                return Err(RouteError::DimensionMismatch {
                    expected: dimensions,
                    actual: agent.vector.len(),
                    context: "agent vector",
                });
            }

            agent_ids.push(agent.id);
            vectors.extend(agent.vector);
        }

        Ok(Self {
            agent_ids,
            vectors,
            dimensions,
        })
    }

    pub fn from_agent_slice(agents: &[AgentProfile]) -> Result<Self, RouteError> {
        if agents.is_empty() {
            return Ok(Self {
                agent_ids: Vec::new(),
                vectors: Vec::new(),
                dimensions: 0,
            });
        }

        let dimensions = agents[0].vector.len();
        let mut agent_ids = Vec::with_capacity(agents.len());
        let mut vectors = Vec::with_capacity(agents.len() * dimensions);

        for agent in agents {
            if agent.vector.len() != dimensions {
                return Err(RouteError::DimensionMismatch {
                    expected: dimensions,
                    actual: agent.vector.len(),
                    context: "agent vector",
                });
            }

            agent_ids.push(agent.id);
            vectors.extend_from_slice(&agent.vector);
        }

        Ok(Self {
            agent_ids,
            vectors,
            dimensions,
        })
    }

    pub fn len(&self) -> usize {
        self.agent_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agent_ids.is_empty()
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn agent_id(&self, index: usize) -> u32 {
        self.agent_ids[index]
    }

    pub fn agent_ids(&self) -> &[u32] {
        &self.agent_ids
    }

    pub fn vector(&self, index: usize) -> &[f32] {
        let start = index * self.dimensions;
        &self.vectors[start..start + self.dimensions]
    }

    pub fn packed_vectors(&self) -> &[f32] {
        &self.vectors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentLabels;

    #[test]
    fn packs_agent_vectors_row_major() {
        let table =
            AgentRouteTable::from_agents(vec![agent(7, &[0.0, 1.0]), agent(9, &[2.0, 3.0])])
                .unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table.dimensions(), 2);
        assert_eq!(table.agent_id(0), 7);
        assert_eq!(table.agent_id(1), 9);
        assert_eq!(table.vector(0), &[0.0, 1.0]);
        assert_eq!(table.vector(1), &[2.0, 3.0]);
        assert_eq!(table.packed_vectors(), &[0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn rejects_mismatched_agent_vector_dimensions() {
        let error = AgentRouteTable::from_agents(vec![agent(1, &[0.0]), agent(2, &[0.0, 1.0])])
            .unwrap_err();

        assert_eq!(
            error,
            RouteError::DimensionMismatch {
                expected: 1,
                actual: 2,
                context: "agent vector"
            }
        );
    }

    fn agent(id: u32, vector: &[f32]) -> AgentProfile {
        AgentProfile {
            id,
            vector: vector.to_vec(),
            labels: AgentLabels::default(),
        }
    }
}
