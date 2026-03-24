//! Placeholder capability registry and semantic search hooks.

use crate::error::{OrchestratorError, OrchestratorResult};
use crate::models::{CapabilityDescriptor, CapabilityId, CapabilityMatch};

/// Pluggable embedding client for semantic search.
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> OrchestratorResult<Vec<f32>>;
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub keyword_weight: f32,
    pub embedding_weight: f32,
    pub bm25_weight: f32,
    pub trust_weight: f32,
    pub availability_weight: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            keyword_weight: 1.0,
            embedding_weight: 0.75,
            bm25_weight: 0.5,
            trust_weight: 0.5,
            availability_weight: 0.5,
        }
    }
}

#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    entries: Vec<CapabilityDescriptor>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn publish(&mut self, desc: CapabilityDescriptor) -> OrchestratorResult<()> {
        if desc.id.is_empty() {
            return Err(OrchestratorError::InvalidInput(
                "capability id is empty".into(),
            ));
        }
        self.entries.push(desc);
        Ok(())
    }

    pub fn search(
        &self,
        query: &str,
        embedder: Option<&dyn Embedder>,
        config: &SearchConfig,
    ) -> OrchestratorResult<Vec<CapabilityMatch>> {
        let q = query.to_ascii_lowercase();
        let tokens: Vec<&str> = q.split_whitespace().filter(|t| !t.is_empty()).collect();
        let query_embedding = if let Some(e) = embedder {
            Some(e.embed(query)?)
        } else {
            None
        };

        let mut matches = Vec::new();
        for d in &self.entries {
            let mut score = 0.0;
            score += self.keyword_score(&q, d) * config.keyword_weight;
            if let (Some(qe), Some(de)) = (query_embedding.as_ref(), d.embeddings.as_ref()) {
                if let Some(sim) = cosine_similarity(qe, de) {
                    score += sim * config.embedding_weight;
                }
            }
            #[cfg(feature = "bm25")]
            {
                let bm25 = bm25_score(&tokens, d);
                score += bm25 * config.bm25_weight;
            }
            if let Some(avail) = d.availability {
                score += avail.clamp(0.0, 1.0) * config.availability_weight;
            }
            if let Some(trust) = d.trust_score {
                score += trust.clamp(0.0, 1.0) * config.trust_weight;
            }
            matches.push(CapabilityMatch::new(d.id.clone(), score, d.clone()));
        }
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(matches)
    }

    pub fn get(&self, id: &CapabilityId) -> Option<&CapabilityDescriptor> {
        self.entries.iter().find(|d| &d.id == id)
    }

    fn keyword_score(&self, q: &str, d: &CapabilityDescriptor) -> f32 {
        let mut score = 0.0;
        if d.name.to_ascii_lowercase().contains(q) {
            score += 1.0;
        }
        if d.description.to_ascii_lowercase().contains(q) {
            score += 0.5;
        }
        if d.tags.iter().any(|t| t.eq_ignore_ascii_case(q)) {
            score += 0.5;
        }
        score
    }
}

pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return None;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }
    Some(dot / (norm_a.sqrt() * norm_b.sqrt()))
}

#[cfg(feature = "bm25")]
fn bm25_score(tokens: &[&str], d: &CapabilityDescriptor) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }
    // Simple BM25-like scoring using term frequency over name/description/tags.
    let corpus = format!("{} {} {}", d.name, d.description, d.tags.join(" ")).to_ascii_lowercase();
    let doc_len = (corpus.split_whitespace().count() as f32).max(1.0);
    let avg_doc_len = crate::bm25::avg_doc_len(&[corpus.clone()]);
    let k1 = 1.2f32;
    let b = 0.75f32;
    let mut score = 0.0f32;
    for t in tokens {
        let tf = corpus.matches(t).count() as f32;
        if tf == 0.0 {
            continue;
        }
        let idf = 1.5f32; // heuristic constant since we lack corpus stats
        let numerator = tf * (k1 + 1.0);
        let denominator = tf + k1 * (1.0 - b + b * (doc_len / avg_doc_len));
        score += idf * (numerator / denominator);
    }
    score
}

#[cfg(test)]
mod tests {
    use super::{CapabilityRegistry, Embedder};
    use crate::models::CapabilityDescriptor;

    fn desc(id: &str, name: &str, desc: &str, tags: &[&str]) -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            embeddings: Some(vec![0.1, 0.2]),
            trust_score: Some(0.7),
            availability: Some(0.6),
        }
    }

    struct StaticEmbedder(Vec<f32>);

    impl Embedder for StaticEmbedder {
        fn embed(&self, _text: &str) -> crate::error::OrchestratorResult<Vec<f32>> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn publish_and_search_ranks_matches() {
        let mut reg = CapabilityRegistry::new();
        reg.publish(desc("a", "Writer", "Writes docs", &["write"]))
            .unwrap();
        reg.publish(desc("b", "Reader", "Reads docs", &["read"]))
            .unwrap();
        let matches = reg
            .search("write", None, &crate::registry::SearchConfig::default())
            .unwrap();
        assert_eq!(matches[0].id, "a");
        assert!(matches[0].score >= matches[1].score);
        assert!(reg.get(&"a".to_string()).is_some());
    }

    #[test]
    fn embedding_similarity_boosts_score() {
        let mut reg = CapabilityRegistry::new();
        reg.publish(desc("a", "Writer", "Writes docs", &["write"]))
            .unwrap();
        let embedder = StaticEmbedder(vec![0.1, 0.2]);
        let matches = reg
            .search(
                "write",
                Some(&embedder),
                &crate::registry::SearchConfig::default(),
            )
            .unwrap();
        assert!(!matches.is_empty());
        assert!(matches[0].score > 0.0);
    }

    #[test]
    fn bm25_can_be_enabled() {
        let mut reg = CapabilityRegistry::new();
        reg.publish(desc("a", "Writer", "Writes docs", &["write"]))
            .unwrap();
        let matches = reg
            .search(
                "writes docs",
                None,
                &crate::registry::SearchConfig::default(),
            )
            .unwrap();
        assert!(!matches.is_empty());
    }
}
