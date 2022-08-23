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
HAMTs were first formally introduced in Phil Bagwell's 2001 paper [Bag01], and have since been
implemented as core datastructures or libraries in languages like Scala, Clojure, or Haskeel.
The implementation follows [Bag01] pretty closely,
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

# HAMT Overview
# Implementation details

# Potential future improvements
This implementation of HAMTs is usable, but by no means complete.
The following are some potential improvements to the datastructure:

- The current implementation uses Rust's `Rc` for reference counting and is thus not thread-safe. Perhaps the implementation can allow for `Rc` or `Arc`
as the 'reference counted type'. Other libraries for immutable datastructures in Rust simply use `Arc` everywhere, though this comes with a cost
in performance as all operations `Arc` does are implemented atomically.
- Reference-counted types allow access to `count()` and `weak_count()` methods, meaning that it is possible to detect when a subtree is not shared
between two map objects.
In this case, mutation on the subtree can be performed in-place without copies as a performance optimization.
- Deeper integration with [`RandomState`](https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html) and `Hashers`, like the
Rust std `HashMap`.
The current implementation instantiates a new `Hasher` each time it needs to hash a key, not letting the user configure.
- The `HAMTNode` and `HAMTNodeEntry::Chained` types both use `Vec` to store collections (collections of entries and chained pairs, respectively).
When they are modified, they copy the entire vector.
Certainly for `HAMTNodeEntry::Chained`, and perhaps for `HAMTNode`, it would be more performant to use an immutable List.
This avoids copying the entire vector, saving time and memory.
- Methods like `HAMT::from` require `K` and `V` to both implement `Clone`, due to using things like `insert` which require it.
Because `from` is guaranteed ownership of the values it is given, and all intermediary maps created during `inserts` are
only in the scope of `from`, `Clone` should not be necessary here.

# References
[Bag01] Phil Bagwell. *Ideal hash trees.* 2001. <http://lampwww.epfl.ch/papers/idealhashtrees.pdf>
