#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
use std::fmt::format;
use std::io::stdout;
#[allow(unused_imports)]
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use anyhow::Context;
use anyhow::Ok;
use bytes::buf;
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
        let mut m = sha1_smol::Sha1::from(&content_to_encode).digest().to_string();
        let obj_folder = &m[..2];
        let obj_file = &m[2..];
        let obj_path = format!(".git/objects/{}/{}", obj_folder, obj_file);
        let o = fs::File::create(obj_path);
        print!("{}",m);
        

    } 
    else {
        println!("unknown command: {}", args[1])
    }
    Ok(())
}