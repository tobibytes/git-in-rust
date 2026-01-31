//! A minimal git implementation in Rust.
//!
//! This library provides core git operations including object storage,
//! tree management, and file hashing.

pub mod error;
pub mod git;
pub mod object;

pub use error::{GitError, Result};
pub use git::{GitCommand, GitRepository};
pub use object::{GitObject, ObjectType};
