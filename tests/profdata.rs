use llvm_profparser::{merge_profiles, parse, parse_bytes};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::Command;

/*
Counters:
  simple_loops:
    Hash: 0x00046d109c4436d1
    Counters: 4
    Function count: 1
    Block counts: [100, 100, 75]

    Instrumentation level: Front-end
Functions shown: 12
Total functions: 12
Maximum function count: 1
Maximum internal block count: 100
 */

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct Output {
    #[serde(rename = "Counters", default)]
    counters: HashMap<String, Entry>,
    #[serde(rename = "Instrumentation level")]
    instrumentation_level: Option<String>,
    #[serde(rename = "Functions shown")]
    functions_shown: Option<usize>,
    #[serde(rename = "Total functions")]
    total_functions: Option<usize>,
    #[serde(rename = "Maximum function count")]
    maximum_function_count: Option<usize>,
    #[serde(rename = "Maximum internal block count")]
    maximum_internal_block_count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Entry {
    hash: Option<usize>,
    counters: Option<usize>,
    #[serde(rename = "Function count")]
    function_count: Option<usize>,
    #[serde(rename = "Block counts", default)]
    block_counts: Vec<usize>,
}

fn data_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/profdata")
}

fn get_data_dir() -> PathBuf {
    cfg_if::cfg_if! {
        if #[cfg(llvm_11)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-11")
        } else if #[cfg(llvm_12)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-12")
        } else if #[cfg(llvm_13)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-13")
        } else if #[cfg(llvm_14)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-14")
        } else if #[cfg(llvm_15)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-15")
        } else if #[cfg(llvm_16)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-16")
        } else if #[cfg(llvm_17)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-17")
        } else if #[cfg(llvm_18)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-18")
        } else if #[cfg(llvm_19)] {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("profdata").join("llvm-19")
        } else {
            data_root_dir()
        }
    }
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
        let llvm_records = llvm_merged.records().iter().collect::<HashSet<_>>();
        let rust_records = rust_merged.records().iter().collect::<HashSet<_>>();
        assert!(!llvm_records.is_empty());
        std::assert_eq!(llvm_records, rust_records);
    } else {
        println!("Unsupported LLVM version");
    }
}

fn check_command(ext: &OsStr) {
    // TODO we should consider doing different permutations of args. Some things which rely on
    // the ordering of elements in a priority_queue etc will display differently though...
    let data = get_data_dir();
    println!("Data directory: {}", data.display());
    let mut count = 0;
    for raw_file in read_dir(&data)
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.path().extension().unwrap_or_default() == ext)
    {
        println!("{:?}", raw_file.file_name());
        // llvm-profdata won't be able to work on all the files as it depends on what the host OS
        // llvm comes with by default. So first we check if it works and if so we test
        let llvm = Command::new("cargo")
            .current_dir(&data)
            .args(&["profdata", "--", "show", "--all-functions", "--counts"])
            .arg(raw_file.file_name())
            .output()
            .expect("cargo binutils or llvm-profdata is not installed");

        let llvm_struct: Output = serde_yaml::from_slice(&llvm.stdout).unwrap();

        if llvm.status.success() {
            println!("Checking {:?}", raw_file.file_name());
            count += 1;
            let rust = assert_cmd::Command::cargo_bin("profparser")
                .unwrap()
                .current_dir(&data)
                .args(&["show", "--all-functions", "--counts", "-i"])
                .arg(raw_file.file_name())
                .output()
                .expect("Failed to run profparser on file");
            println!("{}", String::from_utf8_lossy(&rust.stderr));

            let rust_struct: Output = serde_yaml::from_slice(&rust.stdout).unwrap();

            assert_eq!(rust_struct, llvm_struct);
        } else {
            println!(
                "LLVM tools failed:\n{}",
                String::from_utf8_lossy(&llvm.stderr)
            );
        }
    }
    if count == 0 {
        panic!("No tests for this LLVM version");
    }
}

fn check_against_text(ext: &OsStr) {
    let data = get_data_dir();
    let mut count = 0;
    for raw_file in read_dir(&data)
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.path().extension().unwrap_or_default() == ext)
    {
        println!("{:?}", raw_file.file_name());
        let llvm = Command::new("cargo")
            .current_dir(&data)
            .args(&[
                "profdata",
                "--",
                "show",
                "--text",
                "--all-functions",
                "--counts",
            ])
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
            let text_records = text_prof.records().iter().collect::<HashSet<_>>();
            let parse_records = parsed_prof.records().iter().collect::<HashSet<_>>();
            assert_eq!(text_records, parse_records);
        } else {
            println!("{} failed", raw_file.path().display());
        }
    }
    if count == 0 {
        panic!("No tests for this LLVM version");
    }
}

#[test]
fn show_profraws() {
    let ext = OsStr::new("profraw");
    check_command(ext);
}

#[test]
fn show_proftexts() {
    let ext = OsStr::new("proftext");
    check_command(ext);
}

#[test]
fn show_profdatas() {
    let ext = OsStr::new("profdata");
    // Ordering of elements in printout make most of these tests troublesome
    check_against_text(ext);
}

#[test]
#[cfg_attr(any(llvm_15, llvm_16), ignore)]
fn merge() {
    let data = get_data_dir();
    let files = [
        data.join("foo3bar3-1.proftext"),
        data.join("foo3-1.proftext"),
        data.join("foo3-2.proftext"),
    ];

    check_merge_command(&files, "foo_results");
}

#[test]
fn multi_app_profraw_merging() {
    let premerge_1 = data_root_dir()
        .join("misc")
        .join("multibin_merge/bin_1.profraw");
    let premerge_2 = data_root_dir()
        .join("misc")
        .join("multibin_merge/bin_2.1.profraw");
    let premerge_3 = data_root_dir()
        .join("misc")
        .join("multibin_merge/bin_2.2.profraw");
    let premerge_4 = data_root_dir()
        .join("misc")
        .join("multibin_merge/bin_2.3.profraw");

    let merged = merge_profiles(&[
        premerge_1.clone(),
        premerge_2.clone(),
        premerge_3.clone(),
        premerge_4.clone(),
    ])
    .unwrap();

    let profraw = parse(&premerge_1).unwrap();
    for (hash, name) in profraw.symtab.iter() {
        assert_eq!(merged.symtab.get(*hash), Some(name));
    }

    let profraw = parse(&premerge_2).unwrap();
    for (hash, name) in profraw.symtab.iter() {
        assert_eq!(merged.symtab.get(*hash), Some(name));
    }

    let profraw = parse(&premerge_3).unwrap();
    for (hash, name) in profraw.symtab.iter() {
        assert_eq!(merged.symtab.get(*hash), Some(name));
    }

    let profraw = parse(&premerge_4).unwrap();
    for (hash, name) in profraw.symtab.iter() {
        assert_eq!(merged.symtab.get(*hash), Some(name));
    }
}

#[test]
fn profraw_merging() {
    let premerge_1 = data_root_dir().join("misc").join("premerge_1.profraw");
    let premerge_2 = data_root_dir().join("misc").join("premerge_2.profraw");
    let merged = data_root_dir().join("misc").join("merged.profdata");

    let expected_merged = merge_profiles(&[merged]).unwrap();
    let merged = merge_profiles(&[premerge_1, premerge_2]).unwrap();

    assert_eq!(merged.symtab, expected_merged.symtab);
    assert_eq!(merged.records(), expected_merged.records());
}

#[test]
fn check_raw_data_consistency() {
    let raw = data_root_dir().join("misc").join("stable.profraw");
    let data = data_root_dir().join("misc").join("stable.profdata");

    let raw = merge_profiles(&[raw]).unwrap();
    let data = merge_profiles(&[data]).unwrap();

    // Merged with sparse so need to filter out some records
    for (hash, name) in data.symtab.iter() {
        println!("Seeing if {}:{} in Raw", hash, name);
        std::assert_eq!(name, raw.symtab.get(*hash).unwrap());

        let data_record = data.get_record(name);
        let raw_record = raw.get_record(name);
        std::assert_eq!(data_record, raw_record);
    }
}
