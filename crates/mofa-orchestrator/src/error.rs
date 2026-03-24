use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OrchestratorError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Dependency error: {0}")]
    Dependency(String),
    #[error("Capability resolution failed: {0}")]
    Capability(String),
    #[error("Pattern execution failed: {0}")]
    Pattern(String),
    #[error("Approval error: {0}")]
    Approval(String),
    #[error("SLA violation: {0}")]
    Sla(String),
    #[error("Marketplace error: {0}")]
    Marketplace(String),
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type OrchestratorResult<T> = Result<T, OrchestratorError>;
