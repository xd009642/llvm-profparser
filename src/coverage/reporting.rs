use crate::coverage::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, Default)]
pub struct CoverageReport {
    pub files: BTreeMap<PathBuf, CoverageResult>,
}

pub struct RegionCoverage {}

#[derive(Clone, Debug, Default)]
pub struct CoverageResult {
    pub hits: BTreeMap<SourceLocation, usize>,
}

impl CoverageResult {
    pub fn insert(&mut self, loc: SourceLocation, count: usize) {
        self.hits
            .entry(loc)
            .and_modify(|x| *x += count)
            .or_insert(count);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum RemappingParseError {
    #[error("Path remapping is empty")]
    EmptyRemapping,
    #[error("Missing source path (remapping should be 'source,dest'")]
    MissingSourcePath,
    #[error("Missing destination path (remapping should be 'source,dest'")]
    MissingDestinationPath,
    #[error("Too many paths, two expected")]
    TooManyPaths,
}

/// Map the paths in the coverage data to local source file paths. This allows
/// you to generate the coverage data on one machine, and then use llvm-cov on a
/// different machine where you have the same files on a different path.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PathRemapping {
    /// Path used on source machine
    source: PathBuf,
    /// Path used in destination machine
    dest: PathBuf,
}

impl FromStr for PathRemapping {
    type Err = RemappingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let paths: Vec<&str> = s.split(",").collect();

        if paths.is_empty() {
            Err(Self::Err::EmptyRemapping)
        } else if paths.len() > 2 {
            Err(Self::Err::TooManyPaths)
        } else if paths[0].is_empty() {
            Err(Self::Err::MissingSourcePath)
        } else if paths[1].is_empty() {
            Err(Self::Err::MissingDestinationPath)
        } else {
            let source = PathBuf::from(paths[0]);
            let dest = PathBuf::from(paths[1]);
            Ok(Self { source, dest })
        }
    }
}
