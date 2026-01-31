//! Custom error types for git operations.

use std::fmt;
use std::io;

/// Result type alias for git operations.
pub type Result<T> = std::result::Result<T, GitError>;

/// Error type for git operations.
#[derive(Debug)]
pub enum GitError {
    /// IO operation failed.
    IoError(io::Error),
    /// File not found.
    NotFound(String),
    /// Invalid hash format.
    InvalidHash(String),
    /// UTF-8 encoding error.
    EncodingError(String),
    /// Compression/decompression error.
    CompressionError(String),
    /// Generic error message.
    Other(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::IoError(e) => write!(f, "IO error: {}", e),
            GitError::NotFound(path) => write!(f, "Not found: {}", path),
            GitError::InvalidHash(hash) => write!(f, "Invalid hash: {}", hash),
            GitError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            GitError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            GitError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for GitError {}

impl From<io::Error> for GitError {
    fn from(err: io::Error) -> Self {
        GitError::IoError(err)
    }
}

impl From<std::string::FromUtf8Error> for GitError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        GitError::EncodingError(err.to_string())
    }
}
