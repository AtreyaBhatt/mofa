#![deny(missing_debug_implementations)]
#![deny(rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

pub mod error;
#[cfg(feature = "bm25")]
mod bm25;

// Stub modules: flesh out in subsequent steps
pub mod analyzer;
pub mod api;
pub mod composer;
pub mod governance;
pub mod hitl;
pub mod marketplace;
pub mod models;
pub mod patterns;
pub mod registry;
pub mod storage;

pub use error::OrchestratorError;
