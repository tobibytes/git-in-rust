//! Git clone implementation using HTTP protocol.

use std::fs;
use std::path::Path;
use std::io::Write;
use futures::stream::StreamExt;

use crate::error::{GitError, Result};
use crate::git::GitRepository;
use crate::object::GitObject;
use crate::pack::PackFile;

/// Clone a git repository from a remote URL.
pub async fn clone(url: &str, target_dir: &str) -> Result<()> {
    // Create target directory
    fs::create_dir_all(target_dir)?;

    // Initialize git repository
    let git_dir = format!("{}/.git", target_dir);
    fs::create_dir(&git_dir)?;
    fs::create_dir(format!("{}/.git/objects", target_dir))?;
    fs::create_dir(format!("{}/.git/refs", target_dir))?;
    fs::write(format!("{}/.git/HEAD", target_dir), "ref: refs/heads/main\n")?;

    // Fetch repository info and objects
    let (pack_data, refs) = fetch_pack(url).await?;

    // Write objects from packfile
    write_pack_objects(target_dir, &pack_data)?;

    // Write refs
    for (ref_name, sha) in &refs {
        let ref_path = format!("{}/.git/{}", target_dir, ref_name);
        if let Some(parent) = Path::new(&ref_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&ref_path, format!("{}\n", sha))?;
    }

    // Checkout working tree
    if let Some(head_sha) = refs.get("refs/heads/main") {
        checkout_tree(target_dir, head_sha)?;
    }

    Ok(())
}

/// Fetch pack data from remote repository.
async fn fetch_pack(url: &str) -> Result<(Vec<u8>, std::collections::HashMap<String, String>)> {
    let client = reqwest::Client::new();
    
    // Normalize URL
    let url = url.trim_end_matches(".git");
    let info_url = format!("{}/info/refs?service=git-upload-pack", url);

    // Get initial refs
    let response = client
        .get(&info_url)
        .send()
        .await
        .map_err(|e| GitError::Other(format!("Failed to fetch refs: {}", e)))?;

    let body = response
        .bytes()
        .await
        .map_err(|e| GitError::Other(format!("Failed to read response: {}", e)))?;

    let (refs, _) = parse_refs_response(&body)?;

    // Send upload-pack request
    let upload_url = format!("{}/git-upload-pack", url);
    let request_data = build_upload_request(&refs)?;

    let response = client
        .post(&upload_url)
        .header("Content-Type", "application/x-git-upload-pack-request")
        .body(request_data)
        .send()
        .await
        .map_err(|e| GitError::Other(format!("Failed to upload-pack: {}", e)))?;

    let mut pack_data = Vec::new();
    let mut stream = response
        .bytes_stream();
    
    use futures::stream::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| GitError::Other(format!("Stream error: {}", e)))?;
        pack_data.extend_from_slice(&chunk);
    }

    Ok((pack_data, refs))
}

/// Parse Git protocol refs response.
fn parse_refs_response(data: &[u8]) -> Result<(std::collections::HashMap<String, String>, usize)> {
    let mut refs = std::collections::HashMap::new();
    let mut pos = 0;

    // Skip the service line (e.g., "# service=git-upload-pack\n")
    while pos < data.len() && data[pos] != b'\n' {
        pos += 1;
    }
    if pos < data.len() {
        pos += 1; // skip newline
    }

    // Skip the flush packet
    if pos + 4 <= data.len() && &data[pos..pos + 4] == b"0000" {
        pos += 4;
    }

    // Parse ref lines
    while pos < data.len() {
        // Read pkt-line length (4 hex digits)
        if pos + 4 > data.len() {
            break;
        }

        let len_str = std::str::from_utf8(&data[pos..pos + 4])
            .map_err(|_| GitError::Other("Invalid pkt-line length".to_string()))?;
        let len = u16::from_str_radix(len_str, 16)
            .map_err(|_| GitError::Other("Invalid pkt-line length".to_string()))? as usize;

        if len == 0 {
            break; // flush packet
        }

        pos += 4;

        if pos + len - 4 > data.len() {
            break;
        }

        let line_data = &data[pos..pos + len - 4];
        pos += len - 4;

        let line = String::from_utf8_lossy(line_data);
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let sha = parts[0];
            let ref_name = parts[1];
            refs.insert(ref_name.to_string(), sha.to_string());
        }
    }

    Ok((refs, pos))
}

/// Build upload-pack request.
fn build_upload_request(refs: &std::collections::HashMap<String, String>) -> Result<Vec<u8>> {
    let mut request = Vec::new();

    // Send want for HEAD
    if let Some(head_sha) = refs.get("HEAD") {
        let want_line = format!("want {}\n", head_sha);
        add_pkt_line(&mut request, want_line.as_bytes())?;
    }

    // Send flush
    request.extend_from_slice(b"0000");

    // Send done
    let done_line = b"done\n";
    add_pkt_line(&mut request, done_line)?;

    Ok(request)
}

/// Add a pkt-line to the request.
fn add_pkt_line(data: &mut Vec<u8>, line: &[u8]) -> Result<()> {
    let len = line.len() + 4;
    let len_str = format!("{:04x}", len);
    data.extend_from_slice(len_str.as_bytes());
    data.extend_from_slice(line);
    Ok(())
}

/// Write objects from packfile.
fn write_pack_objects(git_dir: &str, pack_data: &[u8]) -> Result<()> {
    let pack = PackFile::parse(pack_data)?;
    
    for obj in pack.objects {
        let sha = obj.hash();
        let sha_hex = sha
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let obj_folder = &sha_hex[..2];
        let obj_hash = &sha_hex[2..];
        let obj_path = format!("{}/.git/objects/{}/{}", git_dir, obj_folder, obj_hash);

        let path = Path::new(&obj_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = fs::File::create_new(path)?;
        let mut encoder = flate2::write::ZlibEncoder::new(file, flate2::Compression::default());

        let header = format!("{} {}\0", obj.obj_type.as_str(), obj.content.len());
        encoder.write_all(header.as_bytes())?;
        encoder.write_all(&obj.content)?;
        encoder.finish()?;
    }

    Ok(())
}

/// Checkout working tree.
fn checkout_tree(target_dir: &str, tree_sha: &str) -> Result<()> {
    // Read the tree object
    let tree_path = format!("{}/.git/objects/{}/{}", target_dir, &tree_sha[..2], &tree_sha[2..]);
    let file = fs::File::open(&tree_path)?;
    let decoder = flate2::read::ZlibDecoder::new(file);
    let mut reader = std::io::BufReader::new(decoder);

    let mut header = Vec::new();
    use std::io::BufRead;
    reader.read_until(0, &mut header)?;

    // Parse tree and check out files
    checkout_tree_entries(target_dir, &mut reader)?;

    Ok(())
}

/// Recursively checkout tree entries.
fn checkout_tree_entries(target_dir: &str, reader: &mut std::io::BufReader<flate2::read::ZlibDecoder<fs::File>>) -> Result<()> {
    use std::io::Read;
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    // TODO: Parse and checkout entries
    Ok(())
}
