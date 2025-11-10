use llvm_profparser::{merge_profiles, parse, parse_bytes};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::read_dir;
use std::io::BufRead as _;
use std::iter::FromIterator as _;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

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

// map of { llvm: rustc } versions
static SUPPORTED_LLVM_VERSIONS: LazyLock<HashMap<u8, &str>> = LazyLock::new(|| {
    LazyLock::force(&ASSERT_CMDS_EXIST);

    let map = HashMap::from_iter([
        #[cfg(feature = "__llvm_11")]
        (11, "1.51"),
        #[cfg(feature = "__llvm_12")]
        (12, "1.55"),
        #[cfg(feature = "__llvm_13")]
        (13, "1.57"),
        #[cfg(feature = "__llvm_14")]
        (14, "1.64"),
        #[cfg(feature = "__llvm_15")]
        (15, "1.69"),
        #[cfg(feature = "__llvm_16")]
        (16, "1.72"),
        #[cfg(feature = "__llvm_17")]
        (17, "1.77"),
        #[cfg(feature = "__llvm_18")]
        (18, "1.81"),
        #[cfg(feature = "__llvm_19")]
        (19, "1.86"),
        #[cfg(feature = "__llvm_20")]
        (20, "1.90"),
        // TODO: pin this to 1.91 once it releases
        #[cfg(feature = "__llvm_21")]
        (21, "nightly-2025-09-07"),
    ]);

    // Install all the versions we care about.
    // TODO: this is slow :/ but adding a stamp file is non-trivial because it needs to account for
    // which rustc versions are present in the map.
    println!("installing {} rustc versions", map.len());
    let status = Command::new("rustup")
        .args(&[
            "install",
            "--profile=minimal",
            "--component=llvm-tools-preview",
            "--no-self-update",
        ])
        .args(map.values())
        .status();
    assert!(
        status.ok().is_some_and(|s| s.success()),
        "failed to install rustc versions"
    );

    map
});

static LATEST_SUPPORTED_VERSION: u8 = 21;

#[test]
// this test is 'heavy', since it downloads a new toolchain each day.
// make it opt-in with `cargo test -- --ignored`.
#[ignore]
fn latest_llvm_supported() {
    let status = Command::new("rustup")
        .args(&["update", "nightly", "--no-self-update"])
        .status();
    assert!(
        status.ok().is_some_and(|s| s.success()),
        "failed to update nightly",
    );
    let rustc_vv = Command::new("rustc")
        .arg("+nightly")
        .arg("-vV")
        .output()
        .expect("failed to get rustc +nightly version")
        .stdout;
    let last_line = rustc_vv.lines().last().unwrap().unwrap();
    let llvm_v = last_line
        .split("LLVM version: ")
        .nth(1)
        .expect("no llvm version?");
    let llvm_major_v: u8 = llvm_v
        .split('.')
        .next()
        .expect("LLVM version format changed?")
        .parse()
        .expect("LLVM major version not a u8?");
    assert_eq!(llvm_major_v, LATEST_SUPPORTED_VERSION);
}

fn get_data_dir(llvm_version: u8) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("profdata")
        .join(format!("llvm-{llvm_version}"))
}

fn check_merge_command(files: &[PathBuf], id: &str, rustc_version: &str) {
    let llvm_output = PathBuf::from(format!("llvm_{}.profdata", id));
    let names = files
        .iter()
        .map(|x| x.display().to_string())
        .collect::<Vec<String>>();
    let llvm = Command::new("cargo")
        .args(&[&format!("+{rustc_version}"), "profdata", "--", "merge"])
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

// Check that we have the exes we need to run these test at all.
// Otherwise, we give very poor error messages running the same commands in a loop over and over.
static ASSERT_CMDS_EXIST: LazyLock<()> = LazyLock::new(|| {
    assert_cmd::Command::new("cargo")
        .args(&["profdata", "--version"])
        .assert()
        .append_context(
            "help",
            "run 'cargo install cargo-binutils && rustup component add llvm-tools-preview'",
        )
        .success();
    // this is the version of llvm-profdata itself
    assert_cmd::Command::new("cargo")
        .args(&["profdata", "--", "--version"])
        .assert()
        .append_context("help", "run 'rustup component add llvm-tools-preview'")
        .success();
});

static KNOWN_FAILING_TESTS: &[(Option<u8>, &str)] = &[
    (None, "flatten_instr.proftext"),
    (None, "instr-remap.proftext"),
    (None, "overlap_1.proftext"),
    (None, "overlap_1_cs.proftext"),
    (None, "overlap_1_vp.proftext"),
    (None, "overlap_2.proftext"),
    (None, "overlap_2_cs.proftext"),
    (None, "overlap_2_vp.proftext"),
    (None, "ir-basic.proftext"),
    (None, "cs.proftext"),
    (None, "mix_instr.proftext"),
    (None, "mix_instr_small.proftext"),
    (None, "FUnique.proftext"),
    (None, "NoFUnique.proftext"),
    (None, "CSIR_profile.proftext"),
    (None, "IR_profile.proftext"),
    (None, "same-name-1.proftext"),
    (None, "same-name-2.proftext"),
    (None, "multiple-profdata-merge.proftext"),
    (None, "header-directives-1.proftext"),
    (None, "cutoff.proftext"),
    (None, "vtable-value-prof.proftext"),
    (None, "pseudo-count-warm.proftext"),
    (None, "pseudo-count-hot.proftext"),
    (None, "noncs.proftext"),
    (None, "header-directives-2.proftext"),
    (None, "header-directives-3.proftext"),
    (None, "overflow-instr.proftext"),
];

fn check_command(ext: &OsStr, llvm_version: u8) {
    // TODO we should consider doing different permutations of args. Some things which rely on
    // the ordering of elements in a priority_queue etc will display differently though...
    let data = get_data_dir(llvm_version);
    let rustc_version = SUPPORTED_LLVM_VERSIONS
        .get(&llvm_version)
        .expect("unsupported llvm version?");
    println!("Data directory: {}", data.display());
    let mut count = 0;
    'tests: for raw_file in read_dir(&data)
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.path().extension().unwrap_or_default() == ext)
    {
        for &(version, filename) in KNOWN_FAILING_TESTS {
            if (version.is_none() || Some(llvm_version) == version)
                && raw_file.file_name() == filename
            {
                continue 'tests;
            }
        }
        println!("{:?}", raw_file.file_name());
        // llvm-profdata won't be able to work on all the files as it depends on what the host OS
        // llvm comes with by default. So first we check if it works and if so we test
        let llvm = Command::new("cargo")
            .current_dir(&data)
            .args(&[
                &format!("+{rustc_version}"),
                "profdata",
                "--",
                "show",
                "--all-functions",
                "--counts",
            ])
            .arg(raw_file.file_name())
            .output()
            .expect("cargo not installed???");

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

            let mut rust_struct: Output = serde_yaml::from_slice(&rust.stdout).unwrap();
            if llvm_version == 11
                && rust_struct.instrumentation_level == Some("IR  entry_first = 0".into())
            {
                rust_struct.instrumentation_level = Some("IR".into());
            }

            assert_eq!(rust_struct, llvm_struct);
        } else {
            println!(
                "LLVM tools failed:\n{}",
                String::from_utf8_lossy(&llvm.stderr)
            );
        }
    }
    if count == 0 {
        panic!("No tests for LLVM version {}", llvm_version);
    }
}

fn check_against_text(ext: &OsStr, llvm_version: u8) {
    let data = get_data_dir(llvm_version);
    let rustc_version = SUPPORTED_LLVM_VERSIONS
        .get(&llvm_version)
        .expect("unsupported llvm version?");

    let mut count = 0;
    for raw_file in read_dir(&data)
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| x.path().extension().unwrap_or_default() == ext)
    {
        if llvm_version == 11 && raw_file.file_name() == "compat.v1.profdata" {
            // Known failing test.
            continue;
        }
        println!("{:?}", raw_file.file_name());
        let llvm = Command::new("cargo")
            .current_dir(&data)
            .args(&[
                &format!("+{rustc_version}"),
                "profdata",
                "--",
                "show",
                "--text",
                "--all-functions",
                "--counts",
            ])
            .arg(raw_file.file_name())
            .output()
            .expect("failed to spawn cargo?");

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
            println!(
                "{} failed: {}",
                raw_file.path().display(),
                String::from_utf8_lossy(&llvm.stderr),
            );
        }
    }
    if count == 0 {
        panic!("No tests for this LLVM version");
    }
}

#[test]
fn show_profraws() {
    let ext = OsStr::new("profraw");
    for &llvm_version in SUPPORTED_LLVM_VERSIONS.keys() {
        println!("testing profraws for llvm version {llvm_version}");
        check_command(ext, llvm_version);
    }
}

#[test]
fn show_proftexts() {
    let ext = OsStr::new("proftext");
    for &llvm_version in SUPPORTED_LLVM_VERSIONS.keys() {
        println!("testing proftext for llvm version {llvm_version}");
        check_command(ext, llvm_version);
    }
}

#[test]
fn show_profdatas() {
    let ext = OsStr::new("profdata");
    // Ordering of elements in printout make most of these tests troublesome
    for &llvm_version in SUPPORTED_LLVM_VERSIONS.keys() {
        check_against_text(ext, llvm_version);
    }
}

#[test]
fn merge() {
    for (llvm_version, rustc_version) in &*SUPPORTED_LLVM_VERSIONS {
        let data = get_data_dir(*llvm_version);
        let files = [
            data.join("foo3bar3-1.proftext"),
            data.join("foo3-1.proftext"),
            data.join("foo3-2.proftext"),
        ];
        check_merge_command(&files, "foo_results", rustc_version);
    }
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

// https://github.com/xd009642/llvm-profparser/issues/65 prevent regression on hash table parsing
// changing
#[test]
fn hash_table_regression_check() {
    let ferrocene = data_root_dir()
        .join("misc")
        .join("ferrocene-library-aarch64-apple-darwin.profdata");

    // We've not locked this to version, but it's enough to parse it and making sure it parses
    // correctly to prevent a regression.
    parse(&ferrocene).unwrap();
}
