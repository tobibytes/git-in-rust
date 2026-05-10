# git-in-rust

A from-scratch implementation of Git's plumbing commands in Rust. Reads, writes, and inspects real `.git` object stores using SHA-1 hashing, zlib compression, and the on-disk Git object format — fully compatible with the actual `git` CLI.

Built as a solution to the [CodeCrafters "Build Your Own Git"](https://codecrafters.io/challenges/git) challenge.

## What it does

| Command | Behavior |
| --- | --- |
| `init` | Initializes a new `.git` directory (`objects/`, `refs/`, `HEAD`) |
| `cat-file -p <sha>` | Pretty-prints the contents of a Git object by hash |
| `hash-object -w <file>` | SHA-1 hashes a file, writes a zlib-compressed blob into `.git/objects/`, prints the hash |
| `ls-tree [--name-only] <sha>` | Lists the entries of a tree object |
| `write-tree` | Builds a tree object from the current working directory and writes it to the object store |

## How it works

- **Object storage** — every object is `<type> <len>\0<content>`, zlib-deflated, then stored at `.git/objects/<sha[0..2]>/<sha[2..]>`.
- **Hashing** — `sha1_smol` over the uncompressed object body produces the 40-char hex content-address.
- **Compression** — `flate2` (zlib-rs backend) handles deflate/inflate for read and write paths.
- **Trees** — built recursively by walking the working tree, hashing each file as a blob, then writing the parent tree.

## Run it

```sh
# Build
cargo build --release

# Use the wrapper script (operates on .git in the current directory)
./your_program.sh init
./your_program.sh hash-object -w hello.txt
./your_program.sh cat-file -p <hash>
./your_program.sh ls-tree --name-only <tree-sha>
./your_program.sh write-tree
```

> Test it inside a scratch directory, not this repo — the implementation writes into `.git/` of wherever it's run.

```sh
mkdir -p /tmp/git-test && cd /tmp/git-test
/path/to/git-in-rust/your_program.sh init
```

## Stack

Rust 2021 · `flate2` · `sha1_smol` · `anyhow` · `bytes`
