use llvm_profparser::*;

fn main() {

    let files = vec![
        "./benches/data/cargo_testsuite.profdata",
        "./benches/data/tokio-rt.profraw",
        "./benches/data/cargo_testsuite.profraw",
    ];

    for _ in 0..10_000 {
         merge_profiles(&files);
    }
}
