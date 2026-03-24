use serde::{Deserialize, Serialize};

pub type CapabilityId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub embeddings: Option<Vec<f32>>, // optional for semantic feature
    #[serde(default)]
    pub trust_score: Option<f32>,
    #[serde(default)]
    pub availability: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatch {
    pub id: CapabilityId,
    pub score: f32,
    pub descriptor: CapabilityDescriptor,
}

impl CapabilityMatch {
    pub fn new(id: CapabilityId, score: f32, descriptor: CapabilityDescriptor) -> Self {
        Self {
            id,
            score,
            descriptor,
        }
    }
}
