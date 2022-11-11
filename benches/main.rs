use core::num;
use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hamster::HAMT;

fn setup_big_map() -> (i32, HAMT<i32, i32>) {
    let num_keys = 10000;
    let mut map = HAMT::new();
    for k in 1..num_keys {
        map = map.insert(k, -k);
    }
    (num_keys, map)
}

fn big_remove() {
    let (n, mut map) = setup_big_map();
    for k in (1..n).step_by(2) {
        map = map.remove(k);
    }
    for k in (1..n).step_by(2) {
        assert!(!map.contains_key(k));
    }
    for k in (2..n).step_by(2) {
        assert!(map.contains_key(k));
    }
}

fn setup_big_map_std() -> (i32, HashMap<i32, i32>) {
    let num_keys = 10000;
    let mut map = HashMap::new();
    for k in 1..num_keys {
        map.insert(k, -k);
    }
    (num_keys, map)
}

fn big_remove_std() {
    let (n, mut map) = setup_big_map_std();
    for k in (1..n).step_by(2) {
        map.remove(&k);
    }
    for k in (1..n).step_by(2) {
        assert!(!map.contains_key(&k));
    }
    for k in (2..n).step_by(2) {
        assert!(map.contains_key(&k));
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("big remove", |b| b.iter(|| black_box(big_remove())));
    c.bench_function("big remove std", |b| b.iter(|| black_box(big_remove_std())));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
