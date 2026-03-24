# FEATURES ADDED — MoFA Orchestrator

This file tracks implemented features and the files where they live. Update it whenever functionality changes.

## Core Orchestrator
- **TaskAnalyzer offline splitter** (`crates/mofa-orchestrator/src/analyzer.rs`): Splits input text into ordered subtasks with dependencies; errors on empty input.
- **SwarmComposer heuristics** (`crates/mofa-orchestrator/src/composer.rs`): Tag/description + embedding bonus scoring; pattern pick heuristic (deps → Sequential, multi-step → Parallel).
- **Semantic search plumbing** (`crates/mofa-orchestrator/src/registry.rs`, `src/composer.rs`): Embedder trait + configurable search weights; cosine similarity scoring; optional BM25 boost behind `bm25` feature; composer reuses search weights and embeddings when provided.
- **Coordination patterns** (`crates/mofa-orchestrator/src/patterns/*`): Sequential, Parallel, MapReduce, Routing, Supervision, Debate, Consensus executors wired to `TaskRunner`, `HitlGovernor`, and `Governance`. Parallel/MapReduce run concurrently via `join_all`; Routing stops on first success; Supervision retries a failing worker once via a supervisor task; Debate runs debaters in parallel then judge; Consensus runs voters in parallel and fails plan if any voter fails; others remain placeholders.
- **Pattern factory and runner contract** (`crates/mofa-orchestrator/src/patterns/mod.rs`): `make_executor`, `PatternExecutor`, `TaskRunner`, `SubtaskOutcome` types.
- **Execution plan updates** (`crates/mofa-orchestrator/src/models/plan.rs`): `ExecutionPlan::set_status` updates node status.

## HITL & Governance
- **HITL governor with approvals store** (`crates/mofa-orchestrator/src/hitl.rs`): Creates/reuses approval requests for HITL-required subtasks; applies decisions; uses shared `ApprovalStore` (Arc-backed in-memory impl); supports waiting for approval with timeout; exposes getter for external resume flows; optional notifier hook and auto-escalation loop with clock.
- **HITL pending/escalation** (`crates/mofa-orchestrator/src/hitl.rs`): list pending approvals; escalate expired approvals to `Escalated` status.
- **Approval models** (`crates/mofa-orchestrator/src/models/approval.rs`): Approval request/decision with optional expiry/escalation and level; statuses include Escalated.
- **Governance scaffold** (`crates/mofa-orchestrator/src/governance.rs`): Audit buffer, SLA deadline placeholder, injectable clock helper.

## Storage
- **In-memory stores** (`crates/mofa-orchestrator/src/storage.rs`): Plan store (insert/get/update_status), approval store (insert/get/update_status/list_pending), audit store.

## Marketplace
- **Resolver/verification/trust scorer** (`crates/mofa-orchestrator/src/marketplace/mod.rs`): SemVer validation, duplicate-dependency detection, Ed25519 signature verification over `content_hash`, simple trust scoring.
- **Marketplace tests** (`crates/mofa-orchestrator/tests/marketplace_tests.rs`): Resolver happy path, duplicate-dep rejection, missing-signature rejection, positive signature verification, trust score monotonicity.

## API Surface
- **OrchestratorService** (`crates/mofa-orchestrator/src/api.rs`): analyze → plan → pick_pattern; `execute` runs chosen pattern via injected `TaskRunner`, updates plan, persists in-memory; on HITL-required subtasks marks plan/node as `WaitingApproval` and returns approval error. Exposes plan store getter, approval store getter, `apply_approval`, and `resume` to continue after approvals; records plan metrics/audit events.

## Models
- **Task/subtask/risk/deps** (`crates/mofa-orchestrator/src/models/task.rs`)
- **Capabilities/registry** (`crates/mofa-orchestrator/src/models/registry.rs`): includes optional embeddings, trust_score, availability for composer scoring; registry search with basic scoring over name/description/tags/embeddings.
- **Plugin manifest** (`crates/mofa-orchestrator/src/models/plugin_manifest.rs`)
- **Error type** (`crates/mofa-orchestrator/src/error.rs`): typed `OrchestratorError`/`Result`.

## Documentation
- **Usage Guide** (`USAGE_GUIDE.md`): Describes current APIs, patterns behavior, HITL/marketplace/storage usage, runnable `TaskRunner` example, and known limitations.

## Tests
- **Smoke tests** (`crates/mofa-orchestrator/tests/smoke.rs`): Analyzer output, default pattern pick, and execute → WaitingApproval when HITL is required.
- **Marketplace tests** (`crates/mofa-orchestrator/tests/marketplace_tests.rs`): Resolver/signature/trust.
- **HITL tests** (`crates/mofa-orchestrator/tests/hitl_tests.rs`): Pending creation, approval apply, escalation, wait_for_approval.
- **Registry search test** (`crates/mofa-orchestrator/tests/registry_tests.rs`): Search ranks matching capability first.
- **Semantic search unit tests** (`crates/mofa-orchestrator/src/registry.rs`, `src/composer.rs`): Embedding similarity scoring, BM25 on when feature enabled.
