//! A minimal git implementation in Rust.
//!
//! # Example
//!
//! ```sh
//! # Initialize repository
//! cargo run -- init
//!
//! # Hash an object
//! cargo run -- hash-object -w myfile.txt
//!
//! # List tree contents
//! cargo run -- ls-tree <hash>
//!
//! # Write directory tree
//! cargo run -- write-tree
//! ```

use std::env;
use codecrafters_git::git::GitCommand;
use codecrafters_git::error::GitError;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Err(err) = run(args) {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), GitError> {
    let command = GitCommand::from_args(&args)?;
    command.execute()
}

