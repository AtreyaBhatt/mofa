use serde::{Deserialize, Serialize};

pub type PluginId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub id: PluginId,
    pub version_req: String, // SemVer range
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub public_key_id: String,
    pub signature_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: PluginId,
    pub version: String,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    pub content_hash: String,
    pub signature: Option<Signature>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
