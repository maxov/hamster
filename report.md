---
author: Max Ovsiankin
title: Hamster Project Report
subtitle: TTIC Programming Project

colorlinks: true
geometry:
  - margin=1in
---
# Introduction
**HAM**s**T**er is a simple Rust implementation of
[Hash Array Mapped Tries (HAMTs)](https://en.wikipedia.org/wiki/Hash_array_mapped_trie).
Hash Array Mapped Tries are an immutable persistent datastructure that implement an associative array.
HAMTs were first formally introduced in Phil Bagwell's 2001 paper [B2001], and have since been
implemented as core datastructures or libraries in languages like Scala, Clojure, or JavaScript.
The implementation follows [B2001] pretty closely,
although it differs in a couple ways primarily for implementation simplicity.

This project consists of the following components:

- Core implementation of HAMT in `hamster::hamt` (`src/hamt.rs`)
- Fairly comprehensive test suite, largely in `hamster` (`src/lib.rs`)
- Benchmarks comparing performance with Rust's native `HashMap`

# Compiling and running
Make sure [Cargo is installed](https://doc.rust-lang.org/cargo/getting-started/installation.html#install-rust-and-cargo).
Then you can compile the project by simply running `cargo build`.
Tests can be run with `cargo test`.

Benchmarks can be run - how?

# What program does
# How program works
# How to compile

# How to run/use

# Potential future improvements

# References
[B2001] Phil Bagwell. *Ideal hash trees.* 2001. <http://lampwww.epfl.ch/papers/idealhashtrees.pdf>


