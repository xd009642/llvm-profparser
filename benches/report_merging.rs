use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llvm_profparser::*;

pub fn merge_bench_profiles(c: &mut Criterion) {
    let files = vec![
        "./benches/data/cargo_testsuite.profdata",
        "./benches/data/tokio-rt.profraw",
        "./benches/data/cargo_testsuite.profraw",
    ];

    c.bench_function("merge", |b| {
        b.iter(|| {
            let _ = merge_profiles(black_box(&files));
        })
    });
}
criterion_group!(benches, merge_bench_profiles);

criterion_main!(benches);
