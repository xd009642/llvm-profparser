use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llvm_profparser::*;
use std::fs;

// This benchmark is a bit faffy as first you need to generate the binaries and profraws and pop
// them in the appropriate folder structure. After that it will work as expected

pub fn coverage_mapping(c: &mut Criterion) {
    let files = fs::read_dir("./benches/data/mapping/profraws")
        .unwrap()
        .map(|x| x.unwrap().path())
        .collect::<Vec<_>>();

    let profile = merge_profiles(&files).unwrap();

    let binaries = fs::read_dir("./benches/data/mapping/binaries")
        .unwrap()
        .map(|x| x.unwrap().path())
        .collect::<Vec<_>>();

    c.bench_function("coverage mapping", |b| {
        b.iter(|| {
            let _a = CoverageMapping::new(black_box(&binaries), black_box(&profile), true).unwrap();
        })
    });
}

pub fn report_generation(c: &mut Criterion) {
    let files = fs::read_dir("./benches/data/mapping/profraws")
        .unwrap()
        .map(|x| x.unwrap().path())
        .collect::<Vec<_>>();

    let profile = merge_profiles(&files).unwrap();

    let binaries = fs::read_dir("./benches/data/mapping/binaries")
        .unwrap()
        .map(|x| x.unwrap().path())
        .collect::<Vec<_>>();

    let mapping = CoverageMapping::new(&binaries, &profile, true).unwrap();

    c.bench_function("report generation", |b| {
        b.iter(|| {
            let _a = mapping.generate_report();
        })
    });
}

criterion_group!(benches, coverage_mapping, report_generation);

criterion_main!(benches);
