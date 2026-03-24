	 # USAGE GUIDE — MoFA Cognitive Swarm Orchestrator

 This guide shows how to build, configure, and exercise the new `mofa-orchestrator` crate (Cognitive Swarm Orchestrator with HITL governance, marketplace, and semantic discovery). It reflects the current scaffold and the intended feature-flagged surfaces.

 ## Prerequisites
 - Rust toolchain (stable) and workspace dependencies.
 - From repository root: all commands assume you are in `C:\Users\Atreya\Desktop\mofa`.
 - Feature flags are off by default; enable only what you need (semantic, marketplace, notifications-* , rest).

 ## Build & Lint
 - Build minimal surface: `cargo build -p mofa-orchestrator`
 - Build with features (example): `cargo build -p mofa-orchestrator --features semantic,marketplace`
 - Lints: `cargo clippy -p mofa-orchestrator --all-targets --all-features -D warnings`
 - Docs: `cargo doc -p mofa-orchestrator --no-deps`

 ## Tests
 - Smoke tests (current scaffold): `cargo test -p mofa-orchestrator --tests`
 - When patterns/composer/HITL grow, prefer targeted runs:
   - Patterns: `cargo test -p mofa-orchestrator pattern`
   - Marketplace: `cargo test -p mofa-orchestrator resolver` (resolver/signature/trust suites)
   - Semantic search: `cargo test -p mofa-orchestrator semantic`
   - HITL/governance: `cargo test -p mofa-orchestrator hitl`

  ## Feature Flags (crate)
  - `semantic`: enable embedding client + hybrid search paths.
  - `bm25`: enable BM25 boost inside hybrid search (used by registry/composer search scoring; combined with `semantic` when embeddings are available).
  - `marketplace`: enable manifest resolver/signature/trust modules.
  - `notifications-slack` / `notifications-telegram` / `notifications-email` / `notifications-dingtalk`: channel adapters for HITL/governance notifications.
  - `rest`: expose REST façade (if/when added).
  - Default: none enabled to keep the surface lean.

## Current API (implemented subset)
- Entry: `mofa_orchestrator::api::OrchestratorService`
  - `analyze_task(&str) -> Result<Vec<SubtaskSpec>>`: offline sentence splitter → ordered subtasks with dependencies; errors on empty input.
  - `plan(&str) -> Result<ExecutionPlan>`: builds plan with pending nodes from analyzer output.
  - `pick_pattern(&[SubtaskSpec]) -> Result<CoordinationPattern>`: heuristic (deps → Sequential, >1 step → Parallel, else Sequential).
- `execute(&str, runner: &dyn TaskRunner) -> Result<ExecutionPlan>`: analyze → pattern → execute via provided `TaskRunner`; updates node statuses and stores plan in-memory. HITL-required subtasks create/reuse an approval request, mark the subtask and plan as `WaitingApproval`, and return an Approval error so callers can pause. After applying approval via `apply_approval`, call `resume(plan, specs, runner)` to continue; executors proceed when approval status is `Approved`.

 - Patterns (all wired): Sequential, Parallel, MapReduce, Routing, Supervision, Debate, Consensus. Today, Parallel/MapReduce run concurrently via `join_all`; others are sequential placeholders. All call HITL/Governance hooks per subtask (audit + metrics counters).
 - Marketplace: resolver validates SemVer strings; signature verifier (Ed25519 over `content_hash`); trust scorer combines downloads/audits/ratings. Tests cover resolver/signature-missing/trust monotonicity.
 - Storage: In-memory plan/approval/audit stores with status updates; approval store used by HITL governor.
 - Models: `models/*` for subtasks, plans, approvals (with optional expiry/escalation), audit, registry capabilities, plugin manifests.
 - Semantic search: `registry::search` supports keyword + embeddings; BM25 boost available when `bm25` is enabled; weights configurable via `SearchConfig`.

## Using the service with a runner
Implement `TaskRunner` to plug in execution:

```rust
use mofa_orchestrator::api::OrchestratorService;
use mofa_orchestrator::patterns::{TaskRunner, SubtaskOutcome};
use mofa_orchestrator::models::SubtaskStatus;
use mofa_orchestrator::registry::{Embedder, SearchConfig};

struct EchoRunner;
impl TaskRunner for EchoRunner {
    fn run(&self, subtask: &mofa_orchestrator::models::SubtaskSpec) -> mofa_orchestrator::error::OrchestratorResult<SubtaskOutcome> {
        Ok(SubtaskOutcome { status: SubtaskStatus::Completed, output: Some(subtask.description.clone()) })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = OrchestratorService::new();
    let plan = svc.execute("collect requirements. write draft.", &EchoRunner)?;
    println!("plan status: {:?}", plan.status);

    // Optional: semantic search configuration example
    let config = SearchConfig { keyword_weight: 1.0, embedding_weight: 0.8, bm25_weight: 0.5, trust_weight: 0.4, availability_weight: 0.3 };
    struct StaticEmbedder;
    impl Embedder for StaticEmbedder {
        fn embed(&self, text: &str) -> mofa_orchestrator::error::OrchestratorResult<Vec<f32>> {
            // Replace with a real embedding client; this is a stub for wiring/demo.
            Ok(vec![text.len() as f32, 1.0])
        }
    }
    let registry = mofa_orchestrator::registry::CapabilityRegistry::new();
    let _ = registry.search("demo", Some(&StaticEmbedder), &config);
    Ok(())
}
```

HITL behavior: if a `SubtaskSpec` has `risk_level` of High/Critical, `execute` will create an approval request and return an `OrchestratorError::Approval` indicating the subtask is awaiting approval. (Queue/escalation/notifications are not yet implemented.)

  ## Patterns behavior (current)
- Sequential: runs in order.
- Parallel: runs all subtasks concurrently (blocking join); updates each node status.
- MapReduce: runs all but the last concurrently as “map”, then runs the final “reduce” step.
- Routing: tries subtasks in order, stops on first success; later tasks are skipped.
- Supervision: runs workers sequentially; on first failure, a supervisor task (if provided as last node) retries once.
- Debate: runs debaters in parallel, then runs a judge subtask (last node) to conclude.
- Consensus: runs voters in parallel; plan marked failed if any voter fails.

  ## Marketplace usage (current subset)
- Resolver: `marketplace::Resolver::resolve(&manifest)` validates SemVer strings, rejects duplicate deps, and returns deps.
- Signature verifier: `SignatureVerifier::verify(manifest, &VerifyingKey)` checks Ed25519 signature over `content_hash` (base64 sig, 64 bytes). Fails if missing/invalid.
- Trust scorer: `TrustScorer::score(downloads, audits_passed, rating)` combines simple weighted factors.

 - ## Known limitations (until next iterations)
 - HITL lacks channel-specific notifications; a notifier hook exists for tests/integration. Auto-escalation loop polls expiry. Waiting is supported by `HitlGovernor::wait_for_approval`; executors short-circuit on pending approvals and proceed when status is Approved on a subsequent run (caller must re-invoke execute/resume).
- Pattern executors still lack retries/timeouts and richer supervision/debate/consensus logic beyond current simple behaviors.
- Composer scoring lacks real semantic/BM25 search.
- Marketplace resolver lacks full conflict detection and key management; positive signature path now covered by tests.
- API surface is minimal (no submit/status/approve/search/resolve endpoints/CLI/REST/SDK yet).

 ## Quickstart (Rust)
 ```rust
 use mofa_orchestrator::api::OrchestratorService;

 fn main() -> Result<(), Box<dyn std::error::Error>> {
     let svc = OrchestratorService::new();
     let specs = svc.analyze_task("demo task")?;
     let plan = svc.plan("demo task")?;
     let pattern = svc.pick_pattern()?;

     println!("specs={:?} plan_nodes={} pattern={:?}", specs, plan.nodes.len(), pattern);
     Ok(())
 }
 ```

 Run it with: `cargo run -p mofa-orchestrator --example quickstart` (once an example is added), or embed in your own binary.

 ## CLI Flow (future, via mofa-cli)
 The CLI wiring will live in `mofa-cli` once orchestrator surfaces are exposed. Planned commands:
 - `mofa swarm run <plan.yaml>`: submit a plan; accepts `--pattern`, `--capability-filter`, `--hitl-required`.
 - `mofa swarm status <plan_id>`: show DAG status and approvals waiting.
 - `mofa swarm approvals --apply <approval_id>:approve|reject`: apply human decisions.
 - `mofa swarm registry search "pdf extract"`: semantic/hybrid capability lookup.
 - `mofa swarm plugins verify <manifest>` / `resolve <manifest>`: marketplace operations.

 ## REST Flow (future, gated by `rest`)
 - `POST /plans`: submit a plan request.
 - `GET /plans/{id}`: fetch status.
 - `PATCH /approvals/{id}`: approve/reject.
 - `GET /registry/search?q=...`: capability search.
 - `POST /plugins/resolve`: dependency resolution.

 ## Configuration Notes
 - Embeddings (when `semantic` on): configure embedder endpoint/key via env/CLI; provide timeouts; cache embeddings.
 - Marketplace: supply public keys for signature verification; keep manifests under version control.
 - Notifications: provide channel-specific credentials via env; keep off by default for tests.
 - Clocks: prefer injectable clocks for HITL/SLA timers to keep tests deterministic.

 ## Observability (to implement)
 - Tracing spans around analyzer/composer/pattern execution/approvals/resolver.
 - Metrics: counters (plans started/completed/failed, approvals created/approved/rejected/expired), histograms (latency per pattern, approval latency).
 - Ensure IDs (plan_id, task_id, approval_id) are attached to spans/logs.

 ## Example End-to-End (planned demo)
 1) Run demo: `cargo run -p mofa-cli -- mofa swarm run examples/swarm_demo.yaml`
 2) Check status: `cargo run -p mofa-cli -- mofa swarm status <plan_id>`
 3) Apply approval if required: `cargo run -p mofa-cli -- mofa swarm approvals --apply <approval_id>:approve`
 4) Verify completion; inspect audit log and metrics.

 ## Conventions & Standards
 - Follow `.opencode.md`/`AGENTS.md`: typed errors (`thiserror`), `#[non_exhaustive]` on public enums, avoid public `anyhow`, prefer `pub(crate)` internally, cache regex/timestamps, guard numeric casts, English docs.
 - Keep dependency direction: orchestrator may depend on runtime/foundation, not vice versa.
 - Feature-gate optional deps; avoid pulling heavy features into default.

 ## Status
 - Current state: crate scaffolded with placeholder analyzer/composer/pattern picker; in-memory stores; HITL now creates approvals and sets plan/node Waiting status; smoke + HITL/marketplace/registry tests passing.
 - See `IMPLEMENTATION_GUIDE.md` for the detailed plan and remaining work items.
