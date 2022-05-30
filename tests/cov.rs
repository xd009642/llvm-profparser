use llvm_profparser::{merge_profiles, parse, parse_bytes, CoverageMapping};
use pretty_assertions::assert_eq;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::Command;

#[cfg(llvm_11)]
fn get_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/cov/llvm-11")
}

#[cfg(llvm_12)]
fn get_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/cov/llvm-12")
}

#[cfg(llvm_13)]
fn get_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/cov/llvm-13")
}

#[cfg(not(any(llvm_11, llvm_12, llvm_13)))]
fn get_data_dir() -> PathBuf {
    // Nothing to do so lets get a directory with nothing in
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/profdata")
}

fn get_printout(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .lines()
        .map(|x| x.to_string())
        .collect()
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
        // llvm-profdata won't be able to work on all the files as it depends on what the host OS
        // llvm comes with by default. So first we check if it works and if so we test
        let llvm = Command::new("cargo")
            .current_dir(&data)
            .args(&["profdata", "--", "show", "--all-functions", "--counts"])
            .arg(raw_file.file_name())
            .output()
            .expect("cargo binutils or llvm-profdata is not installed");

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

            assert_eq!(get_printout(&llvm.stdout), get_printout(&rust.stdout));
            assert_eq!(get_printout(&llvm.stderr), get_printout(&rust.stderr));
        }
    }
    if count == 0 {
        panic!("No tests for this LLVM version");
    }
}

#[test]
fn check_mapping_consistency() {
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/cov");
    let obj = example.join("simple_project");
    let prof = example.join("simple_project.profraw");

    let instr = parse(prof).unwrap();

    let mapping = CoverageMapping::new(&[obj], &instr).unwrap();
    let info = &mapping.mapping_info[0];
    for record in &instr.records {
        let fun = info
            .cov_fun
            .iter()
            .find(|x| record.hash == Some(x.header.fn_hash))
            .unwrap();
        assert!(info.cov_map.contains_key(&fun.header.filenames_ref));
        let sym_name = instr.symtab.get(fun.header.name_hash);
        assert_eq!(sym_name, record.name.as_ref());
        // record.name record.hash record.counts() + more

        // mapping.mapping_info
        //      mapping_info.cov_map
        //      mapping_info.cov_fun
        //      mapping_info.prof_names
        //      mapping_info.prof_counts
        //      mapping_info.prof_data
    }

    let expected_len = info
        .prof_data
        .iter()
        .map(|x| x.counters_len as usize)
        .sum::<usize>();
    assert_eq!(expected_len, info.prof_counts.len());
}
