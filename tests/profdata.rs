use pretty_assertions::assert_eq;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug)]
struct MergedFiles {
    llvm_output: PathBuf,
    rust_output: PathBuf,
}

fn get_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

fn get_printout(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .lines()
        .map(|x| x.to_string())
        .collect()
}

fn merge_command(files: &[PathBuf], id: &str) -> Option<MergedFiles> {
    let llvm_output = PathBuf::from(format!("llvm_{}.profdata", id));
    let rust_output = PathBuf::from(format!("rust_{}.profdata", id));
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

    let rust = assert_cmd::Command::cargo_bin("llvm_profparser")
        .unwrap()
        .args(&["merge", "-i"])
        .args(&names)
        .arg("-o")
        .arg(&rust_output)
        .output()
        .unwrap();

    if llvm.status.success() && rust.status.success() {
        Some(MergedFiles {
            llvm_output,
            rust_output,
        })
    } else {
        let _ = std::fs::remove_file(llvm_output);
        let _ = std::fs::remove_file(rust_output);
        // If llvm succeeds I should succeed!
        assert!(!llvm.status.success());
        println!("{}", String::from_utf8_lossy(&llvm.stderr));
        None
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
fn merge() {
    let data = get_data_dir();
    let files = [
        data.join("foo3bar3-1.proftext"),
        data.join("foo3-1.proftext"),
        data.join("foo3-2.proftext"),
    ];

    let output = merge_command(&files, "foo_results").expect("Pick better test files");
    let llvm_merge = assert_cmd::Command::cargo_bin("llvm_profparser")
        .unwrap()
        .args(&["show", "--all-functions", "-i"])
        .arg(&output.llvm_output)
        .output()
        .expect("Failed to run llvm_profparser on file");

    let rust_merge = assert_cmd::Command::cargo_bin("llvm_profparser")
        .unwrap()
        .args(&["show", "--all-functions", "-i"])
        .arg(&output.rust_output)
        .output()
        .expect("Failed to run llvm_profparser on file");

    assert_eq!(
        get_printout(&llvm_merge.stdout),
        get_printout(&rust_merge.stdout)
    );
    assert_eq!(
        get_printout(&llvm_merge.stderr),
        get_printout(&rust_merge.stderr)
    );
}
