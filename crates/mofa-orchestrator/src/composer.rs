//! Placeholder composer module. Will implement capability matching and pattern selection.

use crate::error::OrchestratorResult;
use crate::models::{CapabilityDescriptor, CapabilityMatch};
use crate::registry::{Embedder, SearchConfig};

#[derive(Debug, Default)]
pub struct SwarmComposer;

impl SwarmComposer {
    pub fn new() -> Self {
        Self
    }

    pub fn match_capabilities(
        &self,
        needs: &[String],
        pool: &[CapabilityDescriptor],
        embedder: Option<&dyn Embedder>,
        config: &SearchConfig,
    ) -> OrchestratorResult<Vec<CapabilityMatch>> {
        if needs.is_empty() {
            return Ok(Vec::new());
        }
        let needs_lower: Vec<String> = needs.iter().map(|n| n.to_ascii_lowercase()).collect();
        let mut matches = Vec::new();
        let query_text = needs_lower.join(" ");
        let query_embedding = match embedder {
            Some(e) => e.embed(&query_text).ok(),
            None => None,
        };
        for cap in pool {
            let mut score = 0.0;
            for need in &needs_lower {
                // re-use registry keyword scoring heuristics
                if cap.tags.iter().any(|t| t.eq_ignore_ascii_case(need))
                    || cap.description.to_ascii_lowercase().contains(need)
                {
                    score += config.keyword_weight;
                }
            }
            if let (Some(qe), Some(emb)) = (query_embedding.as_ref(), cap.embeddings.as_ref()) {
                if let Some(sim) = crate::registry::cosine_similarity(qe, emb) {
                    score += sim * config.embedding_weight;
                }
            }
            if let Some(avail) = cap.availability {
                score += avail.clamp(0.0, 1.0) * config.availability_weight;
            }
            if let Some(trust) = cap.trust_score {
                score += trust.clamp(0.0, 1.0) * config.trust_weight;
            }
            matches.push(CapabilityMatch::new(cap.id.clone(), score, cap.clone()));
        }
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(matches)
    }

    pub fn pick_pattern(
        &self,
        subtasks: &[crate::models::SubtaskSpec],
    ) -> OrchestratorResult<crate::patterns::CoordinationPattern> {
        // Placeholder heuristic: if any deps, use Sequential; else Parallel; default Sequential.
        if subtasks.iter().any(|s| !s.deps.is_empty()) {
            return Ok(crate::patterns::CoordinationPattern::Sequential);
        }
        if subtasks.len() > 1 {
            return Ok(crate::patterns::CoordinationPattern::Parallel);
        }
        Ok(crate::patterns::CoordinationPattern::Sequential)
    }
}

#[cfg(test)]
mod tests {
    use super::SwarmComposer;
    use crate::models::CapabilityDescriptor;
    use crate::registry::SearchConfig;

    fn cap(id: &str, tags: &[&str]) -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: id.to_string(),
            name: id.to_string(),
            description: id.to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            embeddings: None,
            trust_score: Some(0.8),
            availability: Some(0.5),
        }
    }

    #[test]
    fn match_capabilities_scores_tags_and_trust() {
        let composer = SwarmComposer::new();
        let pool = vec![cap("a", &["write"]), cap("b", &["read"])];
        let matches = composer
            .match_capabilities(
                &["write".to_string()],
                &pool,
                None,
                &SearchConfig::default(),
            )
            .unwrap();
        assert_eq!(matches.len(), 2);
        // Writing capability should rank >= read due to tag match
        assert!(matches[0].score >= matches[1].score);
    }

    #[test]
    fn embedding_similarity_applies_when_available() {
        let composer = SwarmComposer::new();
        let mut a = cap("a", &["write"]);
        a.embeddings = Some(vec![0.1, 0.2]);
        let pool = vec![a];
        struct StaticEmbedder(Vec<f32>);
        impl crate::registry::Embedder for StaticEmbedder {
            fn embed(&self, _text: &str) -> crate::error::OrchestratorResult<Vec<f32>> {
                Ok(self.0.clone())
            }
        }
        let matches = composer
            .match_capabilities(
                &["write".to_string()],
                &pool,
                Some(&StaticEmbedder(vec![0.1, 0.2])),
                &SearchConfig::default(),
            )
            .unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].score > 0.0);
    }

    #[test]
    fn pick_pattern_prefers_parallel_without_deps() {
        let composer = SwarmComposer::new();
        let one = vec![crate::models::SubtaskSpec {
            id: "s1".into(),
            description: "d".into(),
            capabilities: vec![],
            complexity: 0.5,
            deps: vec![],
            risk_level: None,
            estimated_duration_secs: None,
        }];
        let two = vec![
            one[0].clone(),
            crate::models::SubtaskSpec {
                deps: vec![],
                ..one[0].clone()
            },
        ];
        assert!(matches!(
            composer.pick_pattern(&one).unwrap(),
            crate::patterns::CoordinationPattern::Sequential
        ));
        assert!(matches!(
            composer.pick_pattern(&two).unwrap(),
            crate::patterns::CoordinationPattern::Parallel
        ));
    }
}
