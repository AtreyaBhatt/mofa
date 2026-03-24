# IMPLEMENTATION GUIDE — Cognitive Swarm Orchestrator for MoFA

This guide describes how to implement the Cognitive Swarm Orchestrator with HITL governance, Plugin Marketplace Core, and Semantic Agent Discovery. It lists architecture decisions, crate/module boundaries, planned files, and integration points. Follow `.opencode.md` standards (typed errors, `#[non_exhaustive]` enums, `pub(crate)` visibility, feature gating with `optional = true`, native async-in-traits, cached regex/timestamps).

## 1) Scope and Goals
- Deliver a new orchestration layer that decomposes tasks, forms teams, selects coordination patterns, runs with HITL approvals, and integrates MoFA gateway/smith/SDK/CLI.
- Add Plugin Marketplace Core (SemVer resolver, Ed25519 signature verification, trust scoring) and Capability Registry with Semantic Discovery (hybrid dense + BM25).
- Provide CLI/SDK/REST façades and examples; keep kernel/runtime boundaries intact.

## 2) New Crate: `crates/mofa-orchestrator`
- **Cargo.toml**: feature flags (`semantic`, `notifications-*`, `marketplace`, `rhai`, `otlp`), deps (serde, thiserror, semver, ed25519-dalek or equivalent, tokio, tracing, tower-http if REST exposed, optional search/embedding libs, bm25 adapter).
- **src/lib.rs**: re-exports, module wiring, feature gating, typed `OrchestratorError`.
- **src/error.rs**: `#[non_exhaustive]` typed error enum, no blanket `From<anyhow::Error>`.
- **Models (src/models/...)**:
  - `task.rs`: Task, Subtask (with risk, capability needs), DAG edges.
  - `plan.rs`: ExecutionPlan, nodes, statuses, timestamps.
  - `approval.rs`: ApprovalRequest/Decision, tokens, TTL, escalation flags.
  - `audit.rs`: AuditEntry (who/what/when), SLA markers.
  - `registry.rs`: Agent/Tool capability descriptors, metadata, embeddings, trust score.
  - `plugin_manifest.rs`: Plugin metadata, deps (SemVer ranges), signatures, publisher info.
- **Analyzer (src/analyzer/...)**:
  - `task_analyzer.rs`: LLM-assisted + rule-based fallback; outputs DAG + risk tags; injectable LLM client trait.
- **Composer (src/composer/...)**:
  - `capability_matcher.rs`: scoring (metadata + embeddings + availability); uses registry + optional embedding service.
  - `pattern_selector.rs`: heuristic/LLM-assisted selection among coordination patterns.
- **Patterns (src/patterns/...)**:
  - `pattern_trait.rs`: `PatternExecutor` trait (plan slice, bus handle, storage handle, observability).
  - Executors: `sequential.rs`, `parallel.rs`, `map_reduce.rs`, `routing.rs`, `supervision.rs`, `debate.rs`, `consensus.rs`. Each: typed config, retry/timeout policy, trace spans.
- **HITL (src/hitl/...)**:
  - `governor.rs`: Secretary 5-phase FSM (Receive/Clarify/Schedule/Monitor/Report); state persisted.
  - `approvals.rs`: queue, idempotent decision tokens, TTL + escalation routing, reminder scheduling.
- **Governance (src/governance/...)**:
  - `sla.rs`: deadline tracking, alerts.
  - `audit_log.rs`: append-only audit trail writer.
  - `notifications/{mod.rs, slack.rs, telegram.rs, email.rs, dingtalk.rs}`: feature-gated adapters; shared trait.
- **Marketplace (src/marketplace/...)**:
  - `resolver.rs`: SemVer dependency resolution with conflict detection; unit/property tests.
  - `signatures.rs`: Ed25519 verification for manifests/bundles.
  - `trust.rs`: score computation (ratings, downloads, audits) with weights.
- **Registry (src/registry/...)**:
  - `capability_registry.rs`: publish/update/list/search capability entries; storage-backed.
  - `semantic_search.rs`: hybrid dense + BM25; pluggable embedder trait; caching; BM25 adapter (feature `bm25`).
- **API (src/api/mod.rs)**: service façade consumed by SDK/CLI/REST; methods for plan submission, status, approvals, registry ops.
- **Storage (src/storage.rs)**: traits for plans, approvals, audit logs, registry, plugin manifests; in-memory impl for tests; adapters to foundation storage.
- **Tests (tests/...)**: pattern unit tests (mock bus), resolver/signature/trust tests, semantic search sanity, HITL FSM tests, integration of Sequential/Parallel/MapReduce + approval resume.
- **Docs**: `README.md` inside crate; `examples/swarm_demo.rs` or YAML demo driver.

## 3) Existing Crate Changes
- **Workspace root**: add `crates/mofa-orchestrator` to `Cargo.toml` members; run `cargo metadata` to refresh lockfile.
- **mofa-sdk**: re-export orchestrator client/types; optionally add `OrchestratorClient` thin wrapper.
- **mofa-cli**: new `swarm` commands (`swarm run`, `swarm status`, `swarm approvals`, `swarm deploy`). Wire to SDK client; add flags for pattern override, HITL prompt, registry search.
- **mofa-runtime**: minimal integration hooks only (bus handle + task submission API). Avoid pulling runtime logic into orchestrator; keep dependency direction: orchestrator → runtime APIs.
- **mofa-foundation**: optional storage adapters for plans/approvals/audit/registry (SQL-backed). Feature-gated migrations if needed.
- **mofa-plugins**: optional manifest schema alignment if marketplace loads plugin metadata; no behavior change otherwise.
- **mofa-monitoring**: ensure tracing fields cover plan_id/task_id/approval_id; optional metrics helpers (counters for approvals, pattern runs, resolver conflicts).
- **mofa-gateway**: provide sample capability call used in demo (e.g., device action) with clear boundary.
- **mofa-smith**: accept orchestrator spans; optional evaluation hook for pattern success metrics.
- **tests/ workspace**: add integration tests exercising orchestrator + runtime bus; approval resume; resolver conflicts; signature verification; registry search ranking.
- **docs/**: add orchestration guide, feature-flag matrix, REST/CLI usage, marketplace/semantic search notes.
- **examples/**: add `examples/swarm_demo.yaml` and/or rust demo wiring two patterns and an approval step.

## 4) Interfaces and Data Flow
- **Task submission**: CLI/SDK → `OrchestratorService::submit(plan_request)` → Analyzer (DAG) → Composer (match agents, pick pattern) → PatternExecutor dispatch via runtime bus → results aggregated → stored + emitted.
- **HITL**: Governor creates ApprovalRequest → notifications sent (feature-gated) → human decision API/CLI → decision applied idempotently → execution resumes (current scaffold pauses execution and marks plan/node WaitingApproval; resume achieved by re-running execution once approval is Applied/Approved via `apply_approval` + `resume`; notifier hook and auto-escalation loop exist, channel adapters still pending). Governance records audit + metrics counters for plans/subtasks/approvals.
- **Marketplace**: ingest manifest → verify signature → resolve dependencies → compute trust score → store; resolver exposed via SDK/CLI.
- **Semantic discovery**: registry search endpoint; embed query (if enabled) + BM25 → hybrid scoring → top-K capabilities returned to Composer.

## 5) Storage Model (traits)
Provide in-memory impl for tests; SQL-backed impl can live in foundation with feature flag.

## 6) Observability
- Tracing spans: plan creation, analyzer, composer, each pattern step, approval request/decision, resolver, signature verification, notifications. Include IDs in span fields.
- Metrics: counters (plans_started/completed/failed, approvals_created/approved/rejected/expired, resolver_conflicts, signature_fail), histograms (latency per pattern, approval latency).

## 7) Feature Flags
- `semantic`: enable embedding client + hybrid search; otherwise BM25-only; BM25 boost also requires `bm25`.
- `marketplace`: enable resolver/signature/trust modules.
- `notifications-*`: Slack/Telegram/Email/DingTalk; default off.
- `rhai`: allow runtime hooks for analyzer/composer extensions.
- `otlp`: enable OpenTelemetry exporters.

## 8) CLI Additions (mofa-cli)
- `mofa swarm run <plan.yaml>`: submit task, optional `--pattern`, `--capability-filter`, `--hitl-required`.
- `mofa swarm status <plan_id>`: show DAG state, running/completed nodes, approvals waiting.
- `mofa swarm approvals [--apply <approval_id>:approve|reject]`.
- `mofa swarm registry search <query>`: show capability matches with trust scores.
- `mofa swarm plugins verify <manifest>` / `resolve <manifest>`.

## 9) REST (optional in orchestrator or thin service)
- Endpoints: `/plans` (POST), `/plans/{id}` (GET), `/approvals/{id}` (GET/PATCH), `/registry/search`, `/plugins/resolve`, `/plugins/verify`.
- Use tower/axum if REST exposed; keep behind feature flag.

## 10) Testing Plan
- Unit: analyzer DAG shapes; pattern executors with mock bus; resolver conflict matrix; signature verification vectors; trust score weight calc; semantic hybrid scoring sanity; HITL FSM transitions.
- Integration: Sequential/Parallel/MapReduce end-to-end with runtime bus; approval pause/resume; resolver + manifest load; registry search used by composer.
- Property tests: resolver correctness, signature validation invariants.
- Bench (optional): search latency, pattern scheduling overhead.

## 11) Migration/Backward Compatibility

## 12) Risks and Mitigations
- LLM dependency latency: provide deterministic fallback analyzer; cache embeddings.
- Scope of patterns: deliver top 5 first (Sequential, Parallel, MapReduce, Routing, Supervision), ship Debate/Consensus next; flag advanced ones.
- Notification/channel sprawl: gate by features; provide stubs for tests.
- Storage coupling: keep storage traits in orchestrator; adapters in foundation with features.

## 13) Delivery Milestones (brief)
- M1: Crate scaffold, errors, models, Analyzer + Sequential/Parallel + in-memory stores.
- M2: Composer matching + PatternSelector + MapReduce/Routing/Supervision; integration with runtime bus.
- M3: HITL Governor + approvals + notifications (flagged) + audit/SLA.
- M4: Marketplace resolver + signatures + trust scores; CLI/SDK commands for resolve/verify.
- M5: Registry + Semantic search; Composer hooked to search results; hybrid retrieval.
- M6: Docs, examples, integration tests, metrics/tracing polish; optional REST; Docker Compose demo.

## 14) Files to Create/Touch (summary)
- **New**: `crates/mofa-orchestrator/**` (files listed in §2); crate README; example demo.
- **Root**: `Cargo.toml` (workspace members); `Cargo.lock` regenerated.
- **SDK**: `crates/mofa-sdk/src/lib.rs` (+ minimal client surface) and `Cargo.toml` deps.
- **CLI**: `crates/mofa-cli/src/main.rs` + new command modules; `Cargo.toml` deps.
- **Runtime**: light hooks if needed for bus handle exposure (no logic move).
- **Foundation**: optional storage adapters/migrations (feature-gated).
- **Monitoring/Smith/Gateway**: small extensions for tracing fields or sample call.
- **Tests**: new integration files in `tests/tests/` and helpers in `tests/src/` for orchestrator flows.
- **Docs/examples**: orchestration guide, swarm demo YAML or rust example.

Adhere to `.opencode.md`: typed errors, `#[non_exhaustive]` enums, avoid public `anyhow`, cache regex/time, guard numeric casts, `pub(crate)` visibility, feature-gated optional deps, integration tests for interactions, English-only identifiers/docs.

---

## 15) Detailed Task List (with testing per task)
1) Inventory & gap confirmation
   - Read existing swarm module (foundation) and document gaps vs orchestrator scope.
   - Tests: none (doc task).

2) Architecture & API design
   - Define orchestrator crate module tree, traits, storage interfaces, feature flags, SDK/CLI/REST façades.
   - Produce a short ADR (in `docs/` or crate README) for API surface and dependency direction.
   - Tests: none (design doc); add lint check that forbids `anyhow` in public API (optional clippy config).

3) Patterns extension (core execution)
   - Implement MapReduce, Routing, Supervision, Debate, Consensus executors in `src/patterns/`.
   - Update `CoordinationPattern` enum; wire `into_scheduler`/factory.
   - Unit tests: executor behavior with mock bus and deterministic tasks; failure policies; timeout/retry where applicable.
   - Integration tests: end-to-end DAG with mixed dependencies using new patterns (in `tests/tests/`).

4) Swarm Composer (capability matching + pattern selection)
   - Implement matcher scoring (metadata + availability + optional embeddings) and pattern recommendation heuristics/LLM hook.
   - Integration tests: mock registry with agents/tools; verify chosen pattern and selected executors; deterministic scoring when LLM disabled.

5) HITL Governor
   - Implement approvals queue, idempotent decision tokens, TTL/escalation, reminder scheduling, Secretary 5-phase FSM.
   - Add notification adapters (feature-gated) with stubs for tests.
   - Unit tests: FSM transitions, idempotent approvals, escalation timers (with mocked clock), notification adapter invocation.
   - Integration tests: pause/resume flow where a subtask waits on human approval, resumes after decision.

6) Governance layer (SLA, audit, metrics)
   - SLA timers tied to plan/critical path, audit trail writer, metrics counters/histograms.
   - Unit tests: SLA breach detection with mocked clock; audit append/order; metrics labels.
   - Integration tests: plan with deadline → breach recorded; audit entries emitted on approvals and failures.

7) Plugin Marketplace Core
   - SemVer resolver, conflict detection; Ed25519 signature verification; trust scoring.
   - Unit/property tests: resolver (random graphs), signature vectors, trust weight math, malformed manifest handling.
   - Integration tests: manifest with deps → resolution result; signature fail blocks install.

8) Capability Registry + Semantic Discovery
   - Registry CRUD/search; embedding client trait; hybrid dense + BM25 scorer; caching.
   - Unit tests: search ranking deterministic with fixed embeddings/BM25 scores; cache hit behavior.
   - Integration tests: composer uses registry to pick agents; verify top-K includes expected capabilities.

9) Storage adapters
   - In-memory implementations in orchestrator crate for plans/approvals/audit/registry/manifest.
   - Optional SQL-backed adapters in foundation (feature-gated) with minimal migrations.
   - Unit tests: in-memory store semantics; SQL adapter round-trips if added.

10) SDK/CLI/REST surfaces
   - SDK: add orchestrator client/types; ensure `#[non_exhaustive]` on public enums.
   - CLI: `swarm run/status/approvals/registry/plugins` commands; flag validation.
   - REST (feature-gated): axum endpoints for plans, approvals, registry, resolver/verify.
   - Integration tests: CLI happy-path with mock bus/approvals; REST endpoints via supertest-style harness.

11) Observability
   - Add tracing spans/fields, metrics counters/histograms; ensure smith/monitoring integration.
   - Tests: unit assertions on span fields (via test subscriber) where feasible; metrics label coverage.

12) Examples and demos
   - `examples/swarm_demo.yaml` or rust example; includes HITL pause, registry search, and MapReduce step.
   - Manual smoke instructions (see §17) and optional automated example test.

13) Documentation
   - Crate README, orchestration guide in `docs/`, feature-flag table, CLI/REST reference, marketplace/discovery notes.
   - Update AGENTS/README pointers; mention new crate and commands.
   - No tests (doc task); ensure `cargo doc` passes.

---

## 16) Manual Implementation Guide (step-by-step)
1. Scaffold crate
   - Add `crates/mofa-orchestrator` to workspace; create modules/files from §2; define `OrchestratorError` with `thiserror`.
   - Add feature flags and minimal deps.

2. Port existing DAG pieces
   - Reuse or wrap foundation swarm DAG structs where safe; otherwise define orchestrator-local DAG types with `#[non_exhaustive]` enums and typed statuses.
   - Ensure risk/HITL fields are present.

3. Add patterns
   - Implement MapReduce, Routing, Supervision, Debate, Consensus executors and factory wiring.
   - Provide `PatternExecutor` trait and context (bus handle, storage, clocks, metrics).

4. Add analyzer + composer
   - LLM-assisted analyzer (with offline fallback) outputs DAG + risk; composer scores capabilities and picks pattern; expose knobs to bypass LLM.

5. Add HITL governor and governance
   - Approval queue, tokens, TTL/escalation, notifications; SLA timers and audit logging; metrics hooks.

6. Add marketplace and registry
   - SemVer resolver + signatures + trust scoring; capability registry + semantic search (hybrid); embedder trait; BM25 adapter.

7. Storage + integration
   - In-memory stores first; optional SQL adapters (foundation). Keep orchestrator depending on runtime, not vice versa.

8. Surfaces
   - SDK re-export; CLI commands; optional REST routes. Keep defaults minimal and feature-gated.

9. Observability
   - Add spans/metrics; thread IDs (plan_id, task_id, approval_id) in fields.

10. Tests
   - Implement unit/property/integration tests per §15.

11. Examples + docs
   - Add demo YAML and guide; update docs/README/AGENTS references.

---

## 17) Usage Guide (once implemented)
Prereqs: Rust toolchain, workspace buildable; enable features as needed.

Build & format
- `cargo fmt`
- `cargo build -p mofa-orchestrator` (add `--features semantic,marketplace,notifications-slack` as needed)
- `cargo clippy -p mofa-orchestrator --all-targets --all-features -D warnings`

Tests
- Unit: `cargo test -p mofa-orchestrator pattern` (targets pattern executors)
- Integration (orchestrator): `cargo test -p mofa-orchestrator --tests`
- Workspace targeted: `cargo test -p mofa-runtime --test integration_suite -- some_case` (if wired)
- Marketplace props: `cargo test -p mofa-orchestrator resolver`
- Semantic search: `cargo test -p mofa-orchestrator semantic`

CLI (after wiring)
- Run a plan: `cargo run -p mofa-cli -- mofa swarm run examples/swarm_demo.yaml`
- Check status: `cargo run -p mofa-cli -- mofa swarm status <plan_id>`
- Handle approval: `cargo run -p mofa-cli -- mofa swarm approvals --apply <approval_id>:approve`
- Registry search: `cargo run -p mofa-cli -- mofa swarm registry search "pdf extract"`
- Plugin verify: `cargo run -p mofa-cli -- mofa swarm plugins verify path/to/manifest.toml`

REST (if enabled)
- Start REST: (example) `cargo run -p mofa-orchestrator --features rest` (or via CLI flag if routed there)
- Submit plan: `POST /plans`
- Get status: `GET /plans/{id}`
- Approve: `PATCH /approvals/{id}`
- Search registry: `GET /registry/search?q=...`
- Resolve plugin: `POST /plugins/resolve`

Config & feature flags
- Enable `semantic` for embeddings; provide embedder endpoint/config via env or CLI flags.
- Enable `marketplace` for resolver/signatures; supply public keys for verification.
- Enable notification features per channel; provide webhook/credentials via env.
- Default features minimal; document required flags per scenario.

Example flow (demo)
1) Run CLI demo: `cargo run -p mofa-cli -- mofa swarm run examples/swarm_demo.yaml`
2) Observe status: `... swarm status <plan_id>` shows DAG nodes and waiting approvals.
3) Apply approval: `... swarm approvals --apply <approval_id>:approve`
4) Verify completion and outputs; check audit log/metrics (Prometheus/OTLP) if enabled.

Docs
- See `docs/orchestration.md` (to add) for architecture, patterns, HITL, marketplace, registry, feature flags.
- Crate README for quickstart code sample (SDK) and feature matrix.
