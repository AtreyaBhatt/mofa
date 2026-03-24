# MoFA Orchestrator

This crate provides the Cognitive Swarm Orchestrator: task analysis, capability matching, coordination patterns, HITL governance, and marketplace/registry plumbing.

## Quickstart

```rust
use mofa_orchestrator::api::OrchestratorService;
use mofa_orchestrator::patterns::{TaskRunner, SubtaskOutcome};
use mofa_orchestrator::models::SubtaskStatus;

struct EchoRunner;
impl TaskRunner for EchoRunner {
    fn run(
        &self,
        subtask: &mofa_orchestrator::models::SubtaskSpec,
    ) -> mofa_orchestrator::error::OrchestratorResult<SubtaskOutcome> {
        Ok(SubtaskOutcome {
            status: SubtaskStatus::Completed,
            output: Some(subtask.description.clone()),
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = OrchestratorService::new();
    let plan = svc.execute("collect requirements. write draft.", &EchoRunner)?;
    println!("plan status: {:?}", plan.status);
    Ok(())
}
```

## Semantic Search Wiring

The registry and composer support hybrid scoring (keywords + embeddings) and optional BM25 (feature `bm25`).

```rust
use mofa_orchestrator::registry::{CapabilityRegistry, Embedder, SearchConfig};

struct StaticEmbedder;
impl Embedder for StaticEmbedder {
    fn embed(&self, text: &str) -> mofa_orchestrator::error::OrchestratorResult<Vec<f32>> {
        // Replace with a real embedding service call
        Ok(vec![text.len() as f32, 1.0])
    }
}

fn main() -> mofa_orchestrator::error::OrchestratorResult<()> {
    let registry = CapabilityRegistry::new();
    let config = SearchConfig {
        keyword_weight: 1.0,
        embedding_weight: 0.8,
        bm25_weight: 0.5,
        trust_weight: 0.4,
        availability_weight: 0.3,
    };
    let _ = registry.search("demo task", Some(&StaticEmbedder), &config)?;
    Ok(())
}
```

Enable BM25 via `--features bm25` to include term-frequency boost.

## Features

- `semantic`: enables embedding-aware matching.
- `bm25`: optional BM25 boost for search scoring.
- `marketplace`: resolver/signature/trust.
- `notifications-*`: channel adapters for HITL notifications.
- `rest`: REST façade (if added).

See `USAGE_GUIDE.md` and `IMPLEMENTATION_GUIDE.md` for the broader plan and API details.
