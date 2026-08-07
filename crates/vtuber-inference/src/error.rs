//! Typed errors for the inference subsystem.

use thiserror::Error;

/// Errors that can occur during inference model loading or execution.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum InferenceError {
    /// Failure to load the model from disk.
    #[error("model load failed: {0}")]
    LoadFailed(String),
    /// Model hash mismatch.
    #[error("model hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        /// Expected SHA-256 digest.
        expected: String,
        /// Actual SHA-256 digest.
        actual: String,
    },
    /// Failure to optimize the model for the runtime.
    #[error("model optimization failed: {0}")]
    OptimizationFailed(String),
    /// Failure during tensor execution.
    #[error("inference execution failed: {0}")]
    ExecutionFailed(String),
    /// The provided video frame was incompatible with the model input.
    #[error("input frame incompatible: {0}")]
    InvalidInput(String),
    /// Runtime-specific internal error.
    #[error("internal runtime error: {0}")]
    Internal(String),
    /// Worker is already running and cannot be started again.
    #[error("inference worker already running")]
    AlreadyRunning,
}

/// Result type alias for inference operations.
pub type Result<T> = std::result::Result<T, InferenceError>;
