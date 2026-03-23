# CODEBASE OVERVIEW

## 1) High-Level Overview
- **Purpose**: MoFA (Modular Framework for Agents) is a production-grade, microkernel-based agent framework with dual-layer plugin architecture (compile-time Rust/WASM plugins + runtime Rhai scripts).
- **Problem it solves**: Provides fast, extensible, language-neutral agent runtime with hot-reloadable business logic, standardized LLM/tool integrations, and distributed-friendly orchestration.
- **Likely users**: Agent framework builders, backend/infra engineers, researchers prototyping multi-agent workflows, SDK consumers (Rust + generated UniFFI bindings), and plugin authors.

## 2) Architecture Summary
- **Microkernel core** (`mofa-kernel`): traits, core data types, lifecycle contracts.
- **Foundation** (`mofa-foundation`): concrete implementations (registries, storage, tool adapters), business-facing helpers.
- **Runtime** (`mofa-runtime`): orchestrates agent lifecycles, message bus, scheduling, plugin manager, health checks.
- **Plugins** (`mofa-plugins`): concrete plugin impls (LLM/tool/storage/adapters); depends on kernel/foundation.
- **SDK** (`mofa-sdk`): user-facing API, curated re-exports of kernel/foundation types.
- **CLI** (`mofa-cli`): thin binary for running/inspecting agents; delegates logic to runtime/SDK.
- **Extra** (`mofa-extra`): Rhai scripting, rule engine, dynamic tools (feature-gated).
- **FFI** (`mofa-ffi`): UniFFI bindings for polyglot use.
- **Monitoring** (`mofa-monitoring`): observability utilities (Prometheus/OpenTelemetry integration).
- **Macros** (`mofa-macros`): proc-macro helpers.
- **Gateway/Integrations/Local LLM/Smith**: adapters to external systems, LLM backends, and orchestration helpers.
- **Tests workspace** (`tests/`): integration and adversarial test suites, helpers, fixtures.
- **Design patterns**: microkernel layering, plugin pattern (compile-time + runtime), builder APIs for agents, actor-style concurrency (ractor), message bus abstraction, trait-based capability injection, feature-gated optional deps.

## 3) Entry Points
- **CLI**: `crates/mofa-cli/src/main.rs` (build with `cargo run -p mofa-cli -- ...`), dispatches to CLI commands invoking runtime/SDK.
- **Library API**: `mofa-sdk` crate exports public API; downstream crates depend on it.
- **Runtime startup**: `mofa-runtime` crate (agent registry/event loop/message bus) used by CLI and examples.
- **Examples**: `examples/` (e.g., `examples/cloud_voice_demo/src/main.rs`) runnable binaries showing coordination patterns.
- **Tests**: `tests/tests/*.rs` and `tests/src/*` host integration/adversarial entry points.

## 4) Data Flow (typical agent request)
- **Input**: CLI or SDK call constructs an agent request (e.g., via `mofa-runtime` API).
- **Routing**: Runtime uses the message bus to deliver the request to a registered agent (actor-style via ractor/flume/crossbeam).
- **Processing**: Agent implementation (from foundation/plugins) executes tools/LLM plugins and business logic; may invoke Rhai runtime plugins for dynamic rules.
- **Coordination**: Runtime orchestrates steps (sequential/parallel/debate/etc.) and aggregates intermediate results.
- **Output**: Response sent back through the bus to caller; observability emitted via tracing/metrics; optional persistence through configured storage plugin.

## 5) Key Directories & Files

## 6) Technology Stack
- **Languages**: Rust 2024 (workspace rust-version 1.85); Rhai scripting; some Python in CLI tooling/tests.
- **Concurrency**: ractor, tokio, flume/crossbeam channels.
- **Observability**: tracing, tracing-subscriber, OpenTelemetry, Prometheus.
- **Web/Server**: axum 0.8+ (route syntax `{param}`), tower/tower-http.
- **Serialization**: serde, serde_json/yaml, bincode; config crate for multi-format configs.
- **FFI**: UniFFI for multi-language bindings.
- **LLM/Plugins**: plugin abstraction; local LLM crate; OpenAI and others likely via plugins.
- **Build/Test**: cargo fmt/clippy/test; feature-gated components; GitHub Actions workflows.

## 7) Configuration & Setup
- **Workspace build**: `cargo build` (or `cargo build -p <crate>`). Format with `cargo fmt`; lint with `cargo clippy --all-targets --all-features -D warnings`.
- **Tests**: `cargo test`; single test `cargo test -p <crate> -- <filter>`; doctests `cargo test --doc`; examples run via `cargo run -p <crate>`.
- **Configs**: Uses `config` crate; supports toml/json/yaml/ini/ron/json5. SQL migrations in `scripts/sql/migrations/*` for postgres/mysql/sqlite backends.
- **Features**: Optional deps declared with `optional = true`; check crate `Cargo.toml` for feature flags (e.g., dora, rhai, tracing exporters).
- **Env/CLI**: CLI example `cargo run -p mofa-cli -- mofa --help`; some examples may require `MOFA_BIN` or database env vars (see README/examples).

## 8) Current Limitations / Improvement Areas (surface-level)
- Documentation depth varies by crate; many advanced flows rely on README but crate-level docs may be sparse.
- Integration between plugins and runtime not exhaustively documented; tracing setup likely needs clearer examples.
- Tests are present (including adversarial), but coverage across all crates unknown; some crates may lack unit tests.
- Error handling rules strict, but enforcement depends on per-crate review; check for any lingering `anyhow` exposure in public APIs.
- Feature-flag matrix (dora, rhai, tracing exporters) could use explicit docs on combinations and defaults.
- Examples are helpful but not CI-gated uniformly; smoke example noted as manual in README.

## 9) Easy Contribution Opportunities
- Add crate-level READMEs for less-documented crates (gateway/integrations/local-llm/smith) summarizing responsibilities and feature flags.
- Improve tracing examples: add a short guide showing how to enable OpenTelemetry + Prometheus in a sample runtime run.
- Add targeted unit tests for builder validation and numeric cast safety in kernel/foundation types.
- Add doctests or examples for SDK public APIs to verify ergonomics.
- Expand CI to run example smoke tests (or add a lighter sanity check) if feasible.
- Add configuration templates/env examples for database-backed storage (postgres/mysql/sqlite) in `examples/`.
- Add a minimal “hello agent” example using Rhai runtime plugin hot-loading.
- Document feature flags per crate in a single table (md) and link from README.
- Add lint to forbid `anyhow::Result` in public APIs (clippy config) if missing.
- Provide CONTRIBUTING snippet on how to run single test cases and targeted clippy per crate (restate in AGENTS/README).

## 10) Reading Guide for New Contributors (2–3 hours)
- Start: `README.md` (overview, architecture, quick start, roadmap).
- Then: `AGENTS.md` (commands/style expectations) + `INSTRUCTIONS.md`/`.cursorrules` for coding standards.
- Contracts: skim `crates/mofa-kernel/src/lib.rs` + key traits/data types.
- Implementations: skim `crates/mofa-foundation/src/lib.rs` and one representative module (registry/storage/tool integration).
- Runtime wiring: skim `crates/mofa-runtime/src/lib.rs` and message bus/event loop entry points.
- API surface: skim `crates/mofa-sdk/src/lib.rs` to see what’s exposed.
- CLI: glance at `crates/mofa-cli/src/main.rs` to see how commands hook into runtime.
- Tests: open `tests/tests/integration.rs` or `tests/tests/adversarial_suite_tests.rs` to see end-to-end flows and helpers in `tests/src`.
- Optional: run `cargo test -p mofa-runtime -- --nocapture` on a single test to observe runtime behavior.
