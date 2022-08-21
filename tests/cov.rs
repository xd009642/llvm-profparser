use cargo_metadata::{diagnostic::DiagnosticLevel, CargoOpt, Message, Metadata, MetadataCommand};
use llvm_profparser::{merge_profiles, parse, parse_bytes, CoverageMapping};
use pretty_assertions::assert_eq;
use regex::Regex;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, read_dir};
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
enum Channel {
    Stable,
    Beta,
    Nightly,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
struct CargoVersionInfo {
    major: usize,
    minor: usize,
    channel: Channel,
    year: usize,
    month: usize,
    day: usize,
}

impl CargoVersionInfo {
    fn new() -> io::Result<Self> {
        let version_info = Regex::new(
            r"cargo (\d)\.(\d+)\.\d+([\-betanightly]*)(\.[[:alnum:]]+)? \([[:alnum:]]+ (\d{4})-(\d{2})-(\d{2})\)",
        )
        .unwrap();
        Command::new("cargo")
            .arg("--version")
            .output()
            .map(|x| {
                let s = String::from_utf8_lossy(&x.stdout);
                if let Some(cap) = version_info.captures(&s) {
                    let major = cap[1].parse().unwrap();
                    let minor = cap[2].parse().unwrap();
                    // We expect a string like `cargo 1.50.0-nightly (a0f433460 2020-02-01)
                    // the version number either has `-nightly` `-beta` or empty for stable
                    let channel = match &cap[3] {
                        "-nightly" => Channel::Nightly,
                        "-beta" => Channel::Beta,
                        _ => Channel::Stable,
                    };
                    let year = cap[5].parse().unwrap();
                    let month = cap[6].parse().unwrap();
                    let day = cap[7].parse().unwrap();
                    Some(CargoVersionInfo {
                        major,
                        minor,
                        channel,
                        year,
                        month,
                        day,
                    })
                } else {
                    None
                }
            })?
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Cargo version output not parse-able",
                )
            })
    }
}

fn get_project_dir(project: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/")
        .join(project)
}

fn get_printout(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .lines()
        .map(|x| x.to_string())
        .collect()
}

#[derive(Debug, Clone)]
struct Run {
    profraw: PathBuf,
    binary: PathBuf,
}

fn run_coverage(project: &str) -> io::Result<Option<Run>> {
    let project = get_project_dir(project);
    let cargo_version = CargoVersionInfo::new()?;
    let rustflags = match cargo_version.channel {
        Channel::Nightly => "-Zinstrument-coverage",
        _ => "-Cinstrument-coverage",
    };

    let mut child = Command::new("cargo")
        .args(&["test", "--no-run", "--message-format", "json"])
        .env("RUSTFLAGS", rustflags)
        .env("LLVM_PROFILE_FILE", "default.profraw")
        .stdout(Stdio::piped())
        .current_dir(&project)
        .spawn()?;

    let reader = io::BufReader::new(child.stdout.take().unwrap());
    let mut binary = None;
    for msg in Message::parse_stream(reader) {
        if let Ok(Message::CompilerArtifact(art)) = msg {
            if let Some(path) = art.executable.as_ref() {
                binary = Some(PathBuf::from(path));
                break;
            }
        }
    }
    if binary.is_none() {
        // Okay this is our CI job with an old stable so can't build it. Just return Ok(None)
        return Ok(None);
    }
    let binary = binary.unwrap();

    Command::new(&binary).current_dir(&project).output()?;

    println!("{}", binary.display());

    let profraw = project.join("default.profraw");
    assert!(profraw.exists());

    Ok(Some(Run { profraw, binary }))
}

fn compare_reports(run: &Run) {
    let profdata = run.profraw.parent().unwrap().join("default.profdata");
    let merge = Command::new("cargo")
        .args(&["profdata", "--", "merge", "-sparse", "-o"])
        .args(&[&profdata, &run.profraw])
        .output()
        .unwrap();

    if !merge.status.success() {
        let out = String::from_utf8_lossy(&merge.stderr);
        panic!("{}", out);
    }

    let llvm_report = Command::new("cargo")
        .args(&[
            "cov",
            "--",
            "show",
            "--show-instantiations=false",
            "--instr-profile",
        ])
        .args(&[&profdata, &run.binary])
        .output()
        .unwrap();

    let profparser_report = assert_cmd::Command::cargo_bin("cov")
        .unwrap()
        .args(&["show", "--instr-profile"])
        .arg(&run.profraw)
        .arg("--object")
        .arg(&run.binary)
        .output()
        .unwrap();

    let llvm = get_printout(&llvm_report.stdout);
    let profparser = get_printout(&profparser_report.stdout);

    // Internally I use a BTreeMap for the file list so they're always printed in lexicographic
    // ordering. LLVM seems to do the same. But for 1

    assert!(llvm.len() > 0);
    assert!(profparser.len() > 0);

    for (llvm, me) in llvm.iter().zip(&profparser) {
        assert_eq!(llvm, me);
    }
}

#[test]
fn check_matches() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/");
    let profparser_report = assert_cmd::Command::cargo_bin("cov")
        .unwrap()
        .current_dir(&dir)
        .args(&[
            "show",
            "--instr-profile",
            "matches/merged.profdata",
            "--object",
            "matches/matches_bin",
        ])
        .output()
        .unwrap();

    let expected_out = fs::read(dir.join("matches/matches.stdout")).unwrap();
    let profparser = get_printout(&profparser_report.stdout);
    let baseline = get_printout(&expected_out);

    for (baseline, me) in baseline.iter().zip(&profparser) {
        assert_eq!(baseline, me);
    }
}

#[test]
fn check_stable_vec() {
    // Build the project and generate profraw and instrumented binary
    let run_result = run_coverage("stable_vec").unwrap();
    if let Some(run_result) = run_result {
        compare_reports(&run_result);
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

    if let Some(counts) = info.prof_counts.as_ref().map(|x| x.len()) {
        let expected_len = info
            .prof_data
            .as_ref()
            .unwrap()
            .iter()
            .map(|x| x.counters_len as usize)
            .sum::<usize>();
        assert_eq!(expected_len, counts);
    }
}
