//! Git packfile format parsing.

use std::io::{Read, Cursor};
use crate::error::{GitError, Result};
use crate::object::{GitObject, ObjectType};
use sha1_smol::Sha1;

/// Represents a Git packfile.
#[derive(Debug)]
pub struct PackFile {
    /// Objects in the packfile.
    pub objects: Vec<GitObject>,
}

impl PackFile {
    /// Parse a packfile from raw bytes.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 28 {
            return Err(GitError::Other("Packfile too small".to_string()));
        }

        // Check magic number "PACK"
        if &data[0..4] != b"PACK" {
            return Err(GitError::Other("Invalid packfile magic".to_string()));
        }

        // Read version
        let version = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if version != 2 && version != 3 {
            return Err(GitError::Other(format!("Unsupported packfile version: {}", version)));
        }

        // Read number of objects
        let num_objects = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;

        // Parse objects
        let mut objects = Vec::new();
        let mut pos = 12;

        for _ in 0..num_objects {
            let (obj, new_pos) = parse_object(&data, pos)?;
            objects.push(obj);
            pos = new_pos;
        }

        Ok(PackFile { objects })
    }
}

/// Parse a single object from packfile.
fn parse_object(data: &[u8], mut pos: usize) -> Result<(GitObject, usize)> {
    if pos >= data.len() {
        return Err(GitError::Other("Unexpected end of packfile".to_string()));
    }

    let byte = data[pos];
    pos += 1;

    let obj_type_num = (byte >> 4) & 0x7;
    let mut size = (byte & 0x0f) as usize;

    let mut shift = 4;
    while byte & 0x80 != 0 && pos < data.len() {
        let byte = data[pos];
        pos += 1;
        size |= ((byte & 0x7f) as usize) << shift;
        shift += 7;
    }

    let obj_type = match obj_type_num {
        1 => ObjectType::Commit,
        2 => ObjectType::Tree,
        3 => ObjectType::Blob,
        _ => return Err(GitError::Other(format!("Unknown object type: {}", obj_type_num))),
    };

    // Decompress object data
    let mut compressed = Vec::new();
    let mut decompressor = flate2::read::ZlibDecoder::new(&data[pos..]);
    decompressor.read_to_end(&mut compressed)
        .map_err(|e| GitError::Other(format!("Failed to decompress: {}", e)))?;

    // Calculate new position (this is approximate, we should track actual bytes read)
    pos = data.len() - decompressor.get_ref().len();

    let obj = GitObject::new(obj_type, compressed);
    Ok((obj, pos))
}
