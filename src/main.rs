#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use anyhow::Context;
use anyhow::Ok;
use flate2;
use std::io::BufReader;
fn ignore_item(items_to_ignore:  &Vec<&'static str>, item: &str) -> bool {
    for &i in items_to_ignore {
        if i.contains(item) {
            return true
        }
    }
    return false
}
fn init() {
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
    println!("Initialized git directory");
}
fn cat_file(full_hash: &String) -> Result<String, anyhow::Error> {
    let obj_folder = &full_hash[..2];
    let obj_hash = &full_hash[2..];
    let path = format!(".git/objects/{}/{}", obj_folder, obj_hash);
    let file = File::open(&path).expect("could not open file");
    
    let d = flate2::read::ZlibDecoder::new(file);
    let mut d = BufReader::new(d);
    let mut header = Vec::new();
    let mut full_string = String::new();
    d.read_until(0, &mut header).context("could not read header byte")?;
    d.read_to_string(&mut full_string).context("reading from git objects")?;
    print!("{}", full_string);
    Ok(full_string) 
}
fn hash_object(file_name: &String) -> Result<String, anyhow::Error> {
    let path = file_name;
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
    Ok(m)
}

fn hash_file(path: &Path) -> String{
    let code = 100644;
    let file_name = path.file_name().unwrap().to_str().unwrap();
    //code filename hash
    let obj_hash = hash_object(&String::from(path.to_str().unwrap())).unwrap();
    let obj_line = format!("{} {}\\0{}\n", code, file_name, obj_hash);
    obj_line
}

fn ls_tree(tree_hash: &str, name_only: bool) -> Result<String, anyhow::Error> {
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
        // print!("{} \0\n", header_s);
        let mut full_string = String::from(header_s);
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
                // print!("{}\n", file_name);
                full_string.push_str(&format!("{}\n", file_name));
            }
            else {
                // print!("{} {}\0{}\n", mode, file_name, sha_hex);
                full_string.push_str(&format!("{} {}\\0{}\n", mode, file_name, sha_hex));
            }
        }
        print!("{}", full_string);
         Ok(full_string)
}
fn write_dir_tree(full_string:  &String, folder_name: &str, code: i32) -> String {
    let m = sha1_smol::Sha1::from(&full_string).digest().to_string(); 
    let obj_folder = &m[..2];
    let obj_file = &m[2..];
    let obj_path = format!(".git/objects/{}/{}", obj_folder, obj_file);
    let op = std::path::Path::new(&obj_path);
    let pop = op.parent().unwrap();
    fs::create_dir_all(pop).unwrap();
    let of = fs::File::create_new(op).unwrap();
    let mut e = flate2::write::ZlibEncoder::new(of, flate2::Compression::default());
    e.write_all(full_string.as_bytes()).unwrap();
    let _ = e.finish();
    format!("{} {}\\0{}", code, folder_name, m)
}
fn write_tree(entry: &Path)-> String {
    let items_to_ignore = Vec::from([".git", "target"]);
    let mut full_string = String::from("");
    let code = 40000;
    print!("{:?}", entry.file_name());
    let folder_name = entry.file_name().unwrap().to_str().unwrap();
    let entries = entry.read_dir().unwrap();
    for en in entries {
       let f_path = en.unwrap().path();
        if ignore_item(&items_to_ignore, f_path.file_name().unwrap().to_str().unwrap()) {
            continue;
        }
        if !f_path.is_dir() {
            let line = hash_file(&f_path);
            full_string.push_str(&line);
        }
        else {
            let f_string = write_tree(&f_path);
            let folder_string = write_dir_tree(&f_string, folder_name, code);
            full_string.push_str(&folder_string);
        }
    }
    return full_string;

}
fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        init();
    }
    else if args[1] == "cat-file" {
       let obj_type = &args[2];
       let obj = &args[3];
       if obj_type == "-p" {
        cat_file(obj)?;
       }
    }
    else if args[1] == "hash-object" {
       if args[2] == "-w" {
        hash_object(&args[3])?;
       }
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
        ls_tree(tree_hash, name_only)?;
    }
    else if args[1] == "write-tree" {
        let outer_hash = write_tree(&Path::new("./"));
        print!("{}", outer_hash);
    }
    else {
        println!("unknown command: {}", args[1])
    }
    Ok(())
}

