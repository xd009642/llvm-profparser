use pretty_assertions::assert_eq;
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

#[test]
fn show_profraws() {
    // TODO we should consider doing different permutations of args. Some things which rely on
    // the ordering of elements in a priority_queue etc will display differently though...
    let data = get_data_dir();
    println!("Data directory: {}", data.display());
    let ext = OsStr::new("profraw");
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
            count += 1;
            let rust = assert_cmd::Command::cargo_bin("llvm_profparser")
                .unwrap()
                .current_dir(&data)
                .args(&["show", "--all-functions", "-i"])
                .arg(raw_file.file_name())
                .output()
                .expect("Failed to run llvm_profparser on file");
            println!("{}", String::from_utf8_lossy(&rust.stderr));

            assert_eq!(get_printout(&llvm.stdout), get_printout(&rust.stdout));
            assert_eq!(get_printout(&llvm.stderr), get_printout(&rust.stderr));
        } else {
            println!("Skipping {:?}", raw_file.file_name());
            println!("{}", String::from_utf8_lossy(&llvm.stderr));
        }
    }
    assert!(count > 0);
}
