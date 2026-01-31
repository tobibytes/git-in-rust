//! Core git repository operations.

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{GitError, Result};
use crate::object::{GitObject, ObjectType, TreeEntry, CommitMetadata, DEFAULT_AUTHOR, DEFAULT_EMAIL, DEFAULT_TIMEZONE};

const GIT_DIR: &str = ".git";
const OBJECTS_DIR: &str = ".git/objects";
const REFS_DIR: &str = ".git/refs";
const HEAD_FILE: &str = ".git/HEAD";
const HEAD_DEFAULT: &str = "ref: refs/heads/main\n";
const FILE_MODE: u32 = 100644;
const DIR_MODE: u32 = 40000;
const GITIGNORE_FILE: &str = ".gitignore";

/// Git repository interface.
pub struct GitRepository;

impl GitRepository {
    /// Initialize a new git repository.
    pub fn init() -> Result<()> {
        // Clean up existing git directory if present
        if fs::exists(GIT_DIR).unwrap_or(false) {
            fs::remove_dir_all(GIT_DIR)?;
        }

        fs::create_dir(GIT_DIR)?;
        fs::create_dir(Path::new(OBJECTS_DIR))?;
        fs::create_dir(Path::new(REFS_DIR))?;
        fs::write(HEAD_FILE, HEAD_DEFAULT)?;

        println!("Initialized empty git repository in {}", GIT_DIR);
        Ok(())
    }

    /// Read a git object by hash.
    pub fn cat_file(hash: &str) -> Result<String> {
        let obj = Self::read_object(hash)?;
        Ok(String::from_utf8(obj.content)?)
    }

    /// Hash and optionally write a file object.
    pub fn hash_object(file_path: &str, write: bool) -> Result<String> {
        let content = fs::read_to_string(file_path)?;
        let obj = GitObject::new(ObjectType::Blob, content.into_bytes());
        let hash_hex = obj.hash_hex();

        if write {
            Self::write_object(&obj)?;
        }

        Ok(hash_hex)
    }

    /// List tree contents.
    pub fn ls_tree(hash: &str, name_only: bool) -> Result<String> {
        let obj = Self::read_object(hash)?;
        let entries = Self::parse_tree_entries(&obj.content)?;

        let mut output = String::new();
        for entry in entries {
            if name_only {
                output.push_str(&format!("{}\n", entry.name));
            } else {
                output.push_str(&format!("{} {}\0{}\n", entry.mode, entry.name, entry.hash_hex()));
            }
        }

        Ok(output)
    }

    /// Write the working directory tree structure.
    pub fn write_tree() -> Result<String> {
        let tree_bytes = Self::write_dir_recursive(Path::new("./"))?;
        let obj = GitObject::new(ObjectType::Tree, tree_bytes);
        Self::write_object(&obj)?;
        Ok(obj.hash_hex())
    }

    /// Create a commit object.
    pub fn commit_tree(
        tree_sha: &str,
        parent_sha: Option<&str>,
        message: &str,
    ) -> Result<String> {
        // Get current timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| GitError::Other("Failed to get system time".to_string()))?
            .as_secs();

        let author = format!(
            "{} <{}> {} {}",
            DEFAULT_AUTHOR, DEFAULT_EMAIL, timestamp, DEFAULT_TIMEZONE
        );

        let committer = format!(
            "{} <{}> {} {}",
            DEFAULT_AUTHOR, DEFAULT_EMAIL, timestamp, DEFAULT_TIMEZONE
        );

        let metadata = CommitMetadata::new(
            tree_sha.to_string(),
            parent_sha.map(|s| s.to_string()),
            author,
            committer,
            message.to_string(),
        );

        let content = metadata.to_bytes();
        let obj = GitObject::new(ObjectType::Commit, content);
        Self::write_object(&obj)?;
        Ok(obj.hash_hex())
    }

    // ===================== Private helpers =====================

    /// Read a git object from storage.
    fn read_object(hash: &str) -> Result<GitObject> {
        if hash.len() < 2 {
            return Err(GitError::InvalidHash(hash.to_string()));
        }

        let obj_folder = &hash[..2];
        let obj_hash = &hash[2..];
        let path = format!("{}/{}/{}", OBJECTS_DIR, obj_folder, obj_hash);

        let file = File::open(&path)
            .map_err(|_| GitError::NotFound(path))?;
        let decoder = ZlibDecoder::new(file);
        let mut reader = BufReader::new(decoder);

        let mut header = Vec::new();
        reader.read_until(0, &mut header)?;

        let header_str = String::from_utf8(header)?;
        let header_str = header_str.trim_end_matches('\0');
        let parts: Vec<&str> = header_str.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(GitError::Other("Invalid object header".to_string()));
        }

        let obj_type = match parts[0] {
            "blob" => ObjectType::Blob,
            "tree" => ObjectType::Tree,
            "commit" => ObjectType::Commit,
            _ => return Err(GitError::Other(format!("Unknown object type: {}", parts[0]))),
        };

        let mut content = Vec::new();
        reader.read_to_end(&mut content)?;

        Ok(GitObject::new(obj_type, content))
    }

    /// Write a git object to storage.
    fn write_object(obj: &GitObject) -> Result<()> {
        let hash_hex = obj.hash_hex();
        let obj_folder = &hash_hex[..2];
        let obj_hash = &hash_hex[2..];
        let obj_path = format!("{}/{}/{}", OBJECTS_DIR, obj_folder, obj_hash);

        let path = Path::new(&obj_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create_new(path)?;
        let mut encoder = ZlibEncoder::new(file, Compression::default());

        let header = format!("{} {}\0", obj.obj_type.as_str(), obj.content.len());
        encoder.write_all(header.as_bytes())?;
        encoder.write_all(&obj.content)?;
        encoder.finish()?;

        Ok(())
    }

    /// Parse tree entries from raw bytes.
    fn parse_tree_entries(data: &[u8]) -> Result<Vec<TreeEntry>> {
        let mut entries = Vec::new();
        let mut cursor = 0;

        while cursor < data.len() {
            // Find the null terminator after mode and name
            let null_pos = data[cursor..]
                .iter()
                .position(|&b| b == 0)
                .ok_or(GitError::Other("Malformed tree entry".to_string()))?;

            let header = String::from_utf8(data[cursor..cursor + null_pos].to_vec())?;
            let parts: Vec<&str> = header.split_whitespace().collect();

            if parts.len() != 2 {
                return Err(GitError::Other("Invalid tree entry header".to_string()));
            }

            let mode = parts[0]
                .parse::<u32>()
                .map_err(|_| GitError::Other("Invalid mode".to_string()))?;
            let name = parts[1].to_string();

            cursor += null_pos + 1;

            // Read 20-byte SHA1 hash
            if cursor + 20 > data.len() {
                return Err(GitError::Other("Truncated SHA1 hash".to_string()));
            }

            let mut hash = [0u8; 20];
            hash.copy_from_slice(&data[cursor..cursor + 20]);
            cursor += 20;

            entries.push(TreeEntry::new(mode, name, hash));
        }

        Ok(entries)
    }

    /// Recursively write directory tree structure.
    fn write_dir_recursive(dir_path: &Path) -> Result<Vec<u8>> {
        let mut entries = Vec::new();

        let mut dir_entries: Vec<_> = fs::read_dir(dir_path)?
            .filter_map(|e| e.ok())
            .collect();
        
        dir_entries.sort_by(|a, b| {
            a.file_name()
                .to_str()
                .unwrap_or("")
                .cmp(b.file_name().to_str().unwrap_or(""))
        });

        for entry in dir_entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip .git directory and gitignore
            if name == ".git" || name == GITIGNORE_FILE {
                continue;
            }

            if path.is_file() {
                let content = fs::read(&path)?;
                let obj = GitObject::new(ObjectType::Blob, content);
                Self::write_object(&obj)?;

                let tree_entry = TreeEntry::new(FILE_MODE, name, obj.hash());
                entries.extend_from_slice(&tree_entry.to_bytes());
            } else if path.is_dir() {
                let tree_bytes = Self::write_dir_recursive(&path)?;
                let obj = GitObject::new(ObjectType::Tree, tree_bytes);
                Self::write_object(&obj)?;

                let tree_entry = TreeEntry::new(DIR_MODE, name, obj.hash());
                entries.extend_from_slice(&tree_entry.to_bytes());
            }
        }

        Ok(entries)
    }
}

/// Command-line interface for git commands.
pub enum GitCommand {
    /// Initialize repository.
    Init,
    /// Read object content.
    CatFile(String),
    /// Hash object.
    HashObject(String, bool),
    /// List tree.
    LsTree(String, bool),
    /// Write tree.
    WriteTree,
    /// Commit tree.
    CommitTree {
        /// Tree SHA.
        tree: String,
        /// Optional parent commit SHA.
        parent: Option<String>,
        /// Commit message.
        message: String,
    },
    /// Clone repository.
    Clone {
        /// Repository URL.
        url: String,
        /// Target directory.
        target_dir: String,
    },
}

impl GitCommand {
    /// Parse command from arguments.
    pub fn from_args(args: &[String]) -> Result<Self> {
        if args.is_empty() {
            return Err(GitError::Other("No command specified".to_string()));
        }

        match args[0].as_str() {
            "init" => Ok(GitCommand::Init),
            "cat-file" => {
                if args.len() < 3 || args[1] != "-p" {
                    return Err(GitError::Other(
                        "Usage: git cat-file -p <hash>".to_string(),
                    ));
                }
                Ok(GitCommand::CatFile(args[2].clone()))
            }
            "hash-object" => {
                if args.len() < 2 {
                    return Err(GitError::Other(
                        "Usage: git hash-object [-w] <file>".to_string(),
                    ));
                }
                let (write, file) = if args[1] == "-w" && args.len() >= 3 {
                    (true, &args[2])
                } else {
                    (false, &args[1])
                };
                Ok(GitCommand::HashObject(file.clone(), write))
            }
            "ls-tree" => {
                if args.len() < 2 {
                    return Err(GitError::Other(
                        "Usage: git ls-tree [--name-only] <hash>".to_string(),
                    ));
                }
                let (name_only, hash) = if args[1] == "--name-only" && args.len() >= 3 {
                    (true, &args[2])
                } else {
                    (false, &args[1])
                };
                Ok(GitCommand::LsTree(hash.clone(), name_only))
            }
            "write-tree" => Ok(GitCommand::WriteTree),
            "commit-tree" => {
                if args.len() < 3 {
                    return Err(GitError::Other(
                        "Usage: git commit-tree <tree> [-p <parent>] -m <message>".to_string(),
                    ));
                }

                let tree = args[1].clone();
                let mut parent = None;
                let mut message = None;

                let mut i = 2;
                while i < args.len() {
                    match args[i].as_str() {
                        "-p" => {
                            if i + 1 >= args.len() {
                                return Err(GitError::Other("Missing parent SHA".to_string()));
                            }
                            parent = Some(args[i + 1].clone());
                            i += 2;
                        }
                        "-m" => {
                            if i + 1 >= args.len() {
                                return Err(GitError::Other("Missing message".to_string()));
                            }
                            message = Some(args[i + 1].clone());
                            i += 2;
                        }
                        _ => {
                            return Err(GitError::Other(format!(
                                "Unknown flag: {}",
                                args[i]
                            )));
                        }
                    }
                }

                let message = message.ok_or(GitError::Other(
                    "Message is required (-m flag)".to_string(),
                ))?;

                Ok(GitCommand::CommitTree {
                    tree,
                    parent,
                    message,
                })
            }
            "clone" => {
                if args.len() < 3 {
                    return Err(GitError::Other(
                        "Usage: git clone <url> <directory>".to_string(),
                    ));
                }
                Ok(GitCommand::Clone {
                    url: args[1].clone(),
                    target_dir: args[2].clone(),
                })
            }
            cmd => Err(GitError::Other(format!("Unknown command: {}", cmd))),
        }
    }

    /// Execute the command.
    pub fn execute(&self) -> Result<()> {
        match self {
            GitCommand::Init => GitRepository::init(),
            GitCommand::CatFile(hash) => {
                print!("{}", GitRepository::cat_file(hash)?);
                Ok(())
            }
            GitCommand::HashObject(file, write) => {
                let hash = GitRepository::hash_object(file, *write)?;
                println!("{}", hash);
                Ok(())
            }
            GitCommand::LsTree(hash, name_only) => {
                print!("{}", GitRepository::ls_tree(hash, *name_only)?);
                Ok(())
            }
            GitCommand::WriteTree => {
                let hash = GitRepository::write_tree()?;
                print!("{}", hash);
                Ok(())
            }
            GitCommand::CommitTree {
                tree,
                parent,
                message,
            } => {
                let hash = GitRepository::commit_tree(
                    tree,
                    parent.as_deref(),
                    message,
                )?;
                println!("{}", hash);
                Ok(())
            }
            GitCommand::Clone { url, target_dir } => {
                // Use tokio runtime for async clone
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| GitError::Other(format!("Failed to create runtime: {}", e)))?;
                rt.block_on(crate::clone::clone(url, target_dir))
            }
        }
    }
}
