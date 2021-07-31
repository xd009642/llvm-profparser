use llvm_profparser::{merge_profiles, parse, parse_bytes};
use pretty_assertions::assert_eq;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::Command;

fn get_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

fn get_printout(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .lines()
        .map(|x| x.to_string())
        .collect()
}

fn check_merge_command(files: &[PathBuf], id: &str) {
    let llvm_output = PathBuf::from(format!("llvm_{}.profdata", id));
    let names = files
        .iter()
        .map(|x| x.display().to_string())
        .collect::<Vec<String>>();
    let llvm = Command::new("cargo")
        .args(&["profdata", "--", "merge"])
        .args(&names)
        .arg("-o")
        .arg(&llvm_output)
        .output()
        .unwrap();

    if llvm.status.success() {
        let llvm_merged = parse(&llvm_output).unwrap();
        let rust_merged = merge_profiles(&names).unwrap();

        // Okay so we don't care about versioning. We don't care about symtab as there might be
        // hash collisions. And we don't care about the record ordering.
        assert_eq!(
            llvm_merged.is_ir_level_profile(),
            rust_merged.is_ir_level_profile()
        );
        assert_eq!(
            llvm_merged.has_csir_level_profile(),
            rust_merged.has_csir_level_profile()
        );
        let llvm_records = llvm_merged.records.iter().collect::<HashSet<_>>();
        let rust_records = rust_merged.records.iter().collect::<HashSet<_>>();
        assert!(!llvm_records.is_empty());
        assert_eq!(llvm_records, rust_records);
    } else {
        panic!("LLVM failed to merge: {:?}", files);
    }
}

fn check_command(ext: &OsStr) {
    // TODO we should consider doing different permutations of args. Some things which rely on
    // the ordering of elements in a priority_queue etc will display differently though...
    let data = get_data_dir();
    let mut count = 0;
    for raw_file in read_dir(&data)
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.path().extension().unwrap_or_default() == ext)
    {
        // llvm-profdata won't be able to work on all the files as it depends on what the host OS
        // llvm comes with by default. So first we check if it works and if so we test
        let llvm = Command::new("cargo")
            .current_dir(&data)
            .args(&["profdata", "--", "show", "--all-functions"])
            .arg(raw_file.file_name())
            .output()
            .expect("cargo binutils or llvm-profdata is not installed");

        if llvm.status.success() {
            println!("Checking {:?}", raw_file.file_name());
            count += 1;
            let rust = assert_cmd::Command::cargo_bin("profparser")
                .unwrap()
                .current_dir(&data)
                .args(&["show", "--all-functions", "-i"])
                .arg(raw_file.file_name())
                .output()
                .expect("Failed to run profparser on file");
            println!("{}", String::from_utf8_lossy(&rust.stderr));

            assert_eq!(get_printout(&llvm.stdout), get_printout(&rust.stdout));
            assert_eq!(get_printout(&llvm.stderr), get_printout(&rust.stderr));
        }
    }
    assert!(count > 0);
}

fn check_against_text(ext: &OsStr) {
    let data = get_data_dir();
    let mut count = 0;
    for raw_file in read_dir(&data)
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.path().extension().unwrap_or_default() == ext)
    {
        let llvm = Command::new("cargo")
            .current_dir(&data)
            .args(&["profdata", "--", "show", "--text", "--all-functions"])
            .arg(raw_file.file_name())
            .output()
            .expect("cargo binutils or llvm-profdata is not installed");

        if llvm.status.success() {
            count += 1;
            println!(
                "Parsing file: {}",
                data.join(raw_file.file_name()).display()
            );
            println!("{}", String::from_utf8_lossy(&llvm.stdout));
            let text_prof = parse_bytes(&llvm.stdout).unwrap();
            let parsed_prof = parse(data.join(raw_file.file_name())).unwrap();

            // Okay so we don't care about versioning. We don't care about symtab as there might be
            // hash collisions. And we don't care about the record ordering.

            assert_eq!(
                text_prof.is_ir_level_profile(),
                parsed_prof.is_ir_level_profile()
            );
            assert_eq!(
                text_prof.has_csir_level_profile(),
                parsed_prof.has_csir_level_profile()
            );
            let text_records = text_prof.records.iter().collect::<HashSet<_>>();
            let parse_records = parsed_prof.records.iter().collect::<HashSet<_>>();
            assert_eq!(text_records, parse_records);
        } else {
            println!("{} failed", raw_file.path().display());
        }
    }
    assert!(count > 0);
}

#[test]
fn show_profraws() {
    let ext = OsStr::new("profraw");
    check_command(&ext);
}

#[test]
fn show_proftexts() {
    let ext = OsStr::new("proftext");
    check_command(&ext);
}

#[test]
fn show_profdatas() {
    let ext = OsStr::new("profdata");
    // Ordering of elements in printout make most of these tests troublesome
    check_against_text(&ext);
}

#[test]
fn merge() {
    let data = get_data_dir();
    let files = [
        data.join("foo3bar3-1.proftext"),
        data.join("foo3-1.proftext"),
        data.join("foo3-2.proftext"),
    ];

    check_merge_command(&files, "foo_results");
}
