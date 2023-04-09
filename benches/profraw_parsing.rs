use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llvm_profparser::*;
use std::fs;

pub fn tokio_rt_profraw(c: &mut Criterion) {
    let data = fs::read("./benches/data/tokio-rt.profraw").unwrap();

    c.bench_function("profraw_parse_tokio", |b| {
        b.iter(|| parse_bytes(black_box(&data)))
    });
}

pub fn cargo_profraw(c: &mut Criterion) {
    let data = fs::read("./benches/data/cargo_testsuite.profraw").unwrap();

    c.bench_function("profraw_parse_cargo", |b| {
        b.iter(|| parse_bytes(black_box(&data)))
    });
}

pub fn cargo_profdata(c: &mut Criterion) {
    let data = fs::read("./benches/data/cargo_testsuite.profdata").unwrap();

    c.bench_function("profdata_parse_cargo", |b| {
        b.iter(|| parse_bytes(black_box(&data)))
    });
}

criterion_group!(benches, tokio_rt_profraw, cargo_profraw, cargo_profdata);

criterion_main!(benches);
