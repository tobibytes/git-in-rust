#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use anyhow::Context;
use anyhow::Ok;
use flate2;
use std::io::BufReader;


fn main() -> Result<(), anyhow::Error> {
    eprintln!("Logs from your program will appear here!");
    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory");
    }

    else if args[1] == "cat-file" {
       let obj_type = &args[2];
       let obj = &args[3];
       if obj_type == "-p" {
        let obj_folder = &obj[..2];
        let obj_hash = &obj[2..];
        let path = format!(".git/objects/{}/{}", obj_folder, obj_hash);
        let file = File::open(&path).expect("could not open file");

        let d = flate2::read::ZlibDecoder::new(file);
        let mut d = BufReader::new(d);
        let mut header = Vec::new();
        let mut full_string = String::new();
        d.read_until(0, &mut header)?;
        d.read_to_string(&mut full_string).context("reading from git objects")?;
        print!("{}", full_string);
       }
    }
    else if args[1] == "hash-object" {
        let path = &args[3];
        let mut f = fs::File::open(path).expect("could not open file");
        let mut content =String::new();
        f.read_to_string(&mut content)?;
        let content_len =content.len();
        let content_to_encode = format!("blob {}\0{}", content_len, content);
        let m = sha1_smol::Sha1::from(&content_to_encode).digest().to_string();
        let obj_folder = &m[..2];
        let obj_file = &m[2..];
        let obj_path = format!(".git/objects/{}/{}", obj_folder, obj_file);
        let op = std::path::Path::new(&obj_path);
        let pop = op.parent().unwrap();
        fs::create_dir_all(pop)?;
        let of = fs::File::create_new(op)?;
        let mut encoder = flate2::write::ZlibEncoder::new(of, flate2::Compression::default());
        encoder.write_all(content_to_encode.as_bytes())?;
        encoder.finish()?;
        print!("{}",m);

    } 
    else if args[1] == "ls-tree" {
        let tree_hash: &str;
        let mut name_only = false;
        if args[2] == "--name-only" {
            tree_hash = &args[3];
            name_only = true;
        }
        else {
            tree_hash = &args[2];
        }
        let tree_folder = &tree_hash[..2];
        let tree_h = &tree_hash[2..];
        let path = format!(".git/objects/{}/{}", tree_folder, tree_h);
        let file = File::open(&path).expect("could not open file");
        let d = flate2::read::ZlibDecoder::new(file);
        let mut d = BufReader::new(d);
        let mut header= Vec::new();
        d.read_until(0, &mut header)?;
        let header_s = CStr::from_bytes_with_nul(&header)?;
        let header_s = header_s.to_str()?;
        print!("{}\n", header_s);
        loop {
            let mut entry = Vec::new();
            let n = d.read_until(0, &mut entry)?;
            if n == 0 {
                break;
            }
            if entry.is_empty() {
                break;
            }
            entry.pop(); // remove trailing null byte
            let entry_str = String::from_utf8_lossy(&entry);
            let entry_vec: Vec<&str> = entry_str.split(" ").collect();
            let mode = &entry_vec[0];
            let file_name = &entry_vec[1];
            let mut sha = [0u8; 20];
            if let Err(_) = d.read_exact(&mut sha) {
                break;
            }
            let sha_hex = sha.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            if name_only {
                print!("{}\n", file_name);
            }
            else {
                print!("{} {} {}\n", mode, file_name, sha_hex);
            }
        }


    }
    else {
        println!("unknown command: {}", args[1])
    }
    Ok(())
}