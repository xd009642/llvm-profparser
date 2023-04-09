use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llvm_profparser::*;
use std::fs;

pub fn cargo_profdata(c: &mut Criterion) {
    let data = fs::read("./benches/data/cargo_testsuite.profdata").unwrap();

    c.bench_function("profdata_parse_cargo", |b| {
        b.iter(|| parse_bytes(black_box(&data)))
    });
}

criterion_group!(benches, cargo_profdata);

criterion_main!(benches);
