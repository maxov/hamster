---
author: Max Ovsiankin
title: Hamster Project Report
subtitle: TTIC Programming Project

toc: true
colorlinks: true
geometry:
  - margin=1in
---
# Introduction
**HAM**s**T**er is a simple Rust implementation of
[Hash Array Mapped Tries (HAMTs)](https://en.wikipedia.org/wiki/Hash_array_mapped_trie).
Hash Array Mapped Tries are an immutable persistent datastructure that implement an associative array.
HAMTs were first formally introduced in Phil Bagwell's 2001 paper [Bag01], and have since been
implemented as core datastructures or libraries in languages like Scala, Clojure, or Haskell.
The implementation follows [Bag01] pretty closely,
although it differs in a couple ways primarily for implementation simplicity.

This project consists of the following components:

- Core implementation of HAMT and fairly comprehensive test suite in `hamster` (`src/lib.rs`)
- Benchmarks comparing performance with Rust's native `HashMap` (`benches/main.rs`)

# Compiling and running
Make sure [Cargo is installed](https://doc.rust-lang.org/cargo/getting-started/installation.html#install-rust-and-cargo).
Then you can compile the project by running `cargo build`.
Tests can be run with `cargo test`.

Benchmarks can be run with `cargo bench`. You can view the report generated in `target/criterion/report/index.html`.
These also show how the HAMT datastructure can be used as a library.

# HAMT Overview
`HAMT` can be thought of as a hash map that is implemented as an immutable data structure.
Methods like `get` and `contains_key` have the same API as for regular hash maps,
the difference is in "mutator" methods like `insert` and `remove`.
For a regular hash map these methods mutate the existing data structure in place,
while for a HAMT they return a new copy of the data structure while leaving the existing data structure
unmodified.
The "big idea" that makes these persistent data structures possible is that by using the
right layout, we can "share" a lot of the data between versions of the data structure.
For HAMT, this is because the data structure is implemented as a trie, with access at each level
determined by the bits within a key's hash.

This implementation contains a mix of ideas from Hash Array Mapped Tries and standard hash maps.
In particular, standard hash maps are implemented as resizeable array indexed by the key hashes
modulo the size of the array. Conflicts are handled by creating "chains", 
which are vectors of `(key, value)` pairs that take linear time to scan.
This linear time is fine so long as the chains are small, so resizing is performed
when enough entries in the array have multiple keys stored within them.
This implementation also uses chains, although they are only a tool to handle hash conflicts
(when two distinct keys hash to the same value).

## Core data structure

As mentioned before, `HAMT` is a trie where each node is of the same 'type' (there are not different kinds of nodes that behave differently,
aside from a special node that points to the root).
The keys in the HAMT are hashed to 64-bit unsigned values.
The first 5 most significant bits of the hash are used to index in the first level,
the second 5 are used to index in the second level, and so on.
In total there are 12 levels that use 5 bits each, and a 13th level that uses the last four bits.
Thus the `HAMT` functions very much like a trie, where the 'alphabet' has 32 symbols and each element is stored with
a key of about 13 symbols from this alphabet (not exactly 13 symbols, as of course the last level only allows for one of a subset of 16 different symbols from the alphabet).
In the code, these 5-bit subsequences of the hash are called "fragments".
Each internal node can be thought of as an array with exactly 32 spots, and thus the 5 bits from the hash corresponding
to that level are used to index directly into this array.

Each 'spot' in this array is an entry with one of three types: `Value`, `Node`, and `Chained`.
`Value` holds a single `(key, value)` pair, `Node` is a reference to another internal node, and `Chained`
is a reference to a vector of `(key, value)` pairs as mentioned above.
Every internal node aside from the root is created when needed by a key (this happens when two hashed keys share the same prefix for all the previous levels, so a 'branch' is needed at the current level to distinguish between them), thus it stores at least one item.
This recursion stops after the 13th level, where there are no more internal nodes, and all entries refer to either a `Value` or `Chained`
entry. 
Therefore, the only reason why we need a `Chained` entry is when the hashes of two distinct keys are identical.

## Retrieval and modification algorithms

The algorithms for `get` and `contains_key` are pretty similar: recurse through the tree, using the 5 bits at each level
to index into the current internal node.
Recursion stops once the entry indexed at the level does not exist, or is a `Value` (in which case the value is returned if the keys are equal), or is a `Chain` (in which case another method performs the linear-time scan through the chain).

The algorithm for `insert` is similar in order to find the location in the tree where to insert.
We recursively descend through the tree, until we are either at the "bottom" 13th level, or the internal node's entry for
the hash's current 5 bits are empty or are a `Value`.
If the current entry is not empty or a `Value`, then we insert the new key and value at this location.
If there is a distinct old key in this entry that conflicts with the new key,
we need to "split" this entry, handled by the private `create_split_entry` function.
In order to resolve the conflict, it will create a new internal node that stores the two conflicting keys if we are at the 12th level
or lower, and if we are at the 13th level it will create a chain.
If we are at the bottom level, then we will put a `Value` or `Chain` in the entry, depending on if there are conflicts with another key.

The `remove` method uses a similar method to find the location of the current key.
Again, recursion stops if the entry referred to by the 5 bits of the current level in the key's hash
is empty, or is a `Value` or `Chain`.
The first two cases are most straightforward: just remove the entry if it exists.
For the `Chain`, a linear scan is performed to find the key if it is present, and
the vector is copied with the key removed.
If the `Chain` becomes empty, then its entry in its parent internal node becomes empty.
If the lowest internal node that stores the key becomes empty after the removal of the key,
then we can remove it, and make the corresponding entry of the parent node empty.
This process may repeat multiple times if there are multiple internal nodes along the path that only have one entry.

## Runtime

All of these methods have an expected runtime of $\mathcal{O}(\log_{32} n)$ where $n \leq 2^{64}$ is the number of items stored in the hash map. Like any hash map implementation, the use of chaining means that asymptotically the worst case time for any method is $\mathcal{O}(n)$ (when the number of elements is much larger than $2^{64}$, the hash map reduces to an extremely fancy list of key-value pairs that requires linear scans).
However, for practical values of $n$ it is often said by e.g. [Bag01] that hash tries have effectively constant time, due to $\log_{32} n$ being very small using $32$ as a base.
Empirically this implementation is around 20x slower than Rust's standard HashMap according to benchmarks, so there is likely room for improvement.

# Implementation details
The core of the implementation lives in three types: `HAMT` (which keeps a pointer to the root node),
`HAMTNode` (the type representing the internal node mentioned earlier), and
`HAMTNodeEntry` (a Rust `Enum` representing the three different kinds of entry previously discussed).
All three of these types are generic types parameterized by `K`, the key type, and `V`, the value type.

Rust provides a `Hasher` type, which produces 64-bit hashes of the given keys `K`.

## Presence maps
One key optimization we implement from [Bag01] is what we call a "presence" map.
This fixes an issue in the implementation as we have described so far: internal nodes with a small number of entries
will be mostly empty. If the entries are stored in an array of constant length 32, then this is a waste of memory.
Internal nodes with a small number, or even only one, entry are common in HAMTs, so this optimization
has the potential to save a lot of memory.

In addition to the list of entries, each internal node stores a 32-bit integer, where each bit in the integer
flags whether the respective entry in the node is empty or not.
We will call this the presence map.
With this presence map, the array storing the entry can be a variable-length vector rather than a constant-length array.
Any time 5 bits from the hash are used to access an entry,
the presence map is checked to calculate what position that entry will have in the vector.
This is implemented in the `get_entries_index` method by using bitwise operations to keep only the part of the
presence map that stores the presence of the entries whose index is smaller than the desired index,
then counting the number of ones.
To implement `count_ones`, Rust uses LLVM's [`ctpop` intrinsic](https://releases.llvm.org/3.2/docs/LangRef.html#int_ctpop), which
decodes to a single assembly instruction on supported platforms. [Bag01] mentions this as one reason why HAMTs can be especially performant compared to other immutable maps.

The writable methods `insert` and `remove` maintain this presence map in the natural way:
if an empty entry in the current internal node becomes inhabited, the presence for that entry is updated to 1;
if the entry becomes empty, its presence is updated to 0.
This also makes checking for if the internal node is empty for cleanup very fast: just check if the presence map equals 0.

## Constraints on key and value types and use of Rust's trait system
`HAMT` implements three groups of methods, due to the constraint each places on the key and value types (using Rust's trait system).

- The minimal implementation places no constraint on the key and value types, and contains the `new` method which creates a new HAMT
and `height` which measures the height of the HAMT
- If `K` provides `Eq` and `Hash` (meaning the key type can be hashed and equality checks between members of the type can be performed),
then we have the read-only `get` and `contains_key` methods.
- If `K` and `V` further provide `Clone` (meaning that they give the ability to be duplicated),
then we have the writeable `from` (which creates the HAMT from a given array of `(key, value)` pairs), `insert`, and `remove` methods.

For all practical purposes, any types that use the HAMT should satisfy `K: Eq + Hash + Clone` and `V: Clone`.
The Rust standard library `Rc` type can be used as a wrapper on types that do not provide `Clone`, so that they can be cheaply duplicated
(where `Rc` reference-counts and cleans up as needed).

## Memory model
Rust is not garbage-collected, unlike most languages for which HAMTs are built.
This can pose a problem, as HAMTs by nature perform a lot of structural sharing:
when should a particular subtree be freed?
Rust provides a reference-counting smart pointer that only allows immutable references called `Rc`,
which is perfect for Hamster's use case.
Thus Hamster uses the `Rc` type in every place a node references another node, and when the `HAMT`
object references the root node.

However, just using this type is not sufficient: on updates, nodes that store actual keys and values are also sometimes copied,
so the keys and values _themselves_ need to be copied.
This is why most methods put a `Clone` trait constraint on the key and value types.
Primitives types like numbers implement `Copy`, which extends clone, so they can be used directly.

If users are using the HAMT to store non-cloneable types, as metnioned before the easiest way to use it would be to wrap those types in `Rc`.
`Rc` implements clone by creating a new tracked reference, so the value would be freed when there is no more HAMT referencing that value.

# Potential future improvements
This implementation of HAMTs is usable, but by no means complete.
The following are some potential improvements to the datastructure:

- Broadly, benchmarks show the implementation is about 20x slower than Rust's `HashMap` type.
It would be interesting to investigate where this overhead comes from (`Rc`, how much is inherent to immutability/structural sharing).
- The current implementation uses Rust's `Rc` for reference counting and is thus not thread-safe.
Perhaps the implementation can allow for a choice
of `Rc` or `Arc` as the 'reference counted type'.
Other libraries for immutable datastructures in Rust simply use `Arc` everywhere, though this comes with a cost
in performance as all operations `Arc` does are implemented atomically.
- Reference-counted types allow access to `count()` and `weak_count()` methods, meaning that it is possible to detect when a subtree is not shared
between two map objects.
In this case, mutation on the subtree can be performed in-place without copies as a performance optimization.
- Deeper integration with [`RandomState`](https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html) and `Hashers`, like the
Rust std `HashMap`.
The current implementation instantiates a new `Hasher` each time it needs to hash a key, not letting the user configure hashing behavior.
- The `HAMTNode` and `HAMTNodeEntry::Chained` types both use `Vec` to store collections (collections of entries and chained pairs, respectively).
When they are modified, they copy the entire vector.
Certainly for `HAMTNodeEntry::Chained`, and perhaps for `HAMTNode`, it would be more performant to use an immutable List.
This avoids copying the entire vector, saving time and memory.
- Methods like `HAMT::from` require `K` and `V` to both implement `Clone`, due to using things like `insert` which require it.
Because `from` is guaranteed ownership of the values it is given, and all intermediary maps created during `inserts` are
only in the scope of `from`, `Clone` should not be necessary here.

# References
[Bag01] Phil Bagwell. *Ideal hash trees.* 2001. <http://lampwww.epfl.ch/papers/idealhashtrees.pdf>
