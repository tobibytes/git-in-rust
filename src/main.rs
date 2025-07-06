#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
use std::io::stdout;
#[allow(unused_imports)]
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use anyhow::Context;
use anyhow::Ok;
use flate2::read::ZlibDecoder;
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
        let d = ZlibDecoder::new(file);
        let mut d = BufReader::new(d);
        let mut header_s = Vec::new();
        let mut full_string = String::new();
        d.read_until(0, &mut header_s)?;
        let header = CStr::from_bytes_with_nul(&header_s)
        .expect("know there is exactly one nul, and it's at the end");
        let header = header
        .to_str()
        .context(".git/objects file header isn't valid UTF-8")?;
        d.read_to_string(&mut full_string).context("reading from git objects")?;
        let header_len = header.len();
        // let body = &full_string[header_len..];
        print!("{}", full_string);
  
        
       }
    }
    else {
        println!("unknown command: {}", args[1])
    }
    Ok(())
}