//! Git object types and operations.

use sha1_smol::Sha1;

/// Default author for commits.
pub const DEFAULT_AUTHOR: &str = "John Doe";
/// Default email for commits.
pub const DEFAULT_EMAIL: &str = "john@example.com";
/// Default timezone offset.
pub const DEFAULT_TIMEZONE: &str = "+0000";

/// Git object type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    /// Blob object type.
    Blob,
    /// Tree object type.
    Tree,
    /// Commit object type.
    Commit,
}

impl ObjectType {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
        }
    }
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a git object (blob, tree, commit).
#[derive(Debug, Clone)]
pub struct GitObject {
    /// Object type.
    pub obj_type: ObjectType,
    /// Object content.
    pub content: Vec<u8>,
}

impl GitObject {
    /// Create a new git object.
    pub fn new(obj_type: ObjectType, content: Vec<u8>) -> Self {
        Self { obj_type, content }
    }

    /// Calculate SHA1 hash of the object.
    pub fn hash(&self) -> [u8; 20] {
        let header = format!("{} {}\0", self.obj_type.as_str(), self.content.len());
        let mut bytes = Vec::from(header.as_bytes());
        bytes.extend_from_slice(&self.content);
        Sha1::from(&bytes).digest().bytes()
    }

    /// Get hex representation of the hash.
    pub fn hash_hex(&self) -> String {
        self.hash()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

/// Tree entry representing a file or directory in a git tree.
#[derive(Debug, Clone)]
pub struct TreeEntry {
    /// File mode (e.g., 100644 for file, 40000 for directory).
    pub mode: u32,
    /// File name.
    pub name: String,
    /// SHA1 hash of the object.
    pub hash: [u8; 20],
}

impl TreeEntry {
    /// Create a new tree entry.
    pub fn new(mode: u32, name: String, hash: [u8; 20]) -> Self {
        Self { mode, name, hash }
    }

    /// Serialize entry to bytes for tree storage.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(format!("{} {}\0", self.mode, self.name).as_bytes());
        bytes.extend_from_slice(&self.hash);
        bytes
    }

    /// Get hex representation of the hash.
    pub fn hash_hex(&self) -> String {
        self.hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

/// Commit metadata for a git commit object.
#[derive(Debug, Clone)]
pub struct CommitMetadata {
    /// Tree SHA-1 hash (hex format).
    pub tree: String,
    /// Optional parent commit SHA-1 hash (hex format).
    pub parent: Option<String>,
    /// Author name and email.
    pub author: String,
    /// Committer name and email.
    pub committer: String,
    /// Commit message.
    pub message: String,
}

impl CommitMetadata {
    /// Create a new commit metadata.
    pub fn new(
        tree: String,
        parent: Option<String>,
        author: String,
        committer: String,
        message: String,
    ) -> Self {
        Self {
            tree,
            parent,
            author,
            committer,
            message,
        }
    }

    /// Serialize commit metadata to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(format!("tree {}\n", self.tree).as_bytes());

        if let Some(parent) = &self.parent {
            bytes.extend_from_slice(format!("parent {}\n", parent).as_bytes());
        }

        bytes.extend_from_slice(format!("author {}\n", self.author).as_bytes());
        bytes.extend_from_slice(format!("committer {}\n", self.committer).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(self.message.as_bytes());
        bytes.extend_from_slice(b"\n");

        bytes
    }
}
