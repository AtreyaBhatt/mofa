//! Placeholder analyzer module. Will implement LLM + offline DAG builder.

use crate::error::{OrchestratorError, OrchestratorResult};
use crate::models::SubtaskSpec;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Default)]
pub struct TaskAnalyzer;

static SENTENCE_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)[.!?]\s+").expect("split regex should compile"));

impl TaskAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_offline(&self, task: &str) -> OrchestratorResult<Vec<SubtaskSpec>> {
        if task.trim().is_empty() {
            return Err(OrchestratorError::InvalidInput("task is empty".into()));
        }
        let mut specs = Vec::new();
        for (idx, sentence) in SENTENCE_SPLIT
            .split(task)
            .filter(|s| !s.trim().is_empty())
            .enumerate()
        {
            specs.push(SubtaskSpec {
                id: format!("step-{}", idx + 1),
                description: sentence.trim().to_string(),
                capabilities: vec![],
                complexity: 0.5,
                deps: if idx == 0 {
                    vec![]
                } else {
                    vec![format!("step-{}", idx)]
                },
                risk_level: Some(if idx == 0 {
                    crate::models::RiskLevel::Critical
                } else {
                    crate::models::RiskLevel::Low
                }),
                estimated_duration_secs: None,
            });
        }
        if specs.is_empty() {
            return Err(OrchestratorError::InvalidInput(
                "no subtasks produced".into(),
            ));
        }
        Ok(specs)
    }
}

#[cfg(test)]
mod tests {
    use super::TaskAnalyzer;

    #[test]
    fn analyzer_rejects_empty() {
        let analyzer = TaskAnalyzer::new();
        let err = analyzer.analyze_offline("   ").unwrap_err();
        assert!(matches!(
            err,
            crate::error::OrchestratorError::InvalidInput(_)
        ));
    }

    #[test]
    fn analyzer_splits_sentences_and_sets_risk() {
        let analyzer = TaskAnalyzer::new();
        let specs = analyzer
            .analyze_offline("First sentence. Second sentence!")
            .unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].id, "step-1");
        assert!(specs[0].deps.is_empty());
        assert_eq!(specs[1].deps, vec!["step-1".to_string()]);
        assert!(matches!(
            specs[0].risk_level,
            Some(crate::models::RiskLevel::Critical)
        ));
        assert!(matches!(
            specs[1].risk_level,
            Some(crate::models::RiskLevel::Low)
        ));
    }
}
