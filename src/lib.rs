use crate::instrumentation_profile::types::InstrumentationProfile;
use std::path::Path;

pub mod coverage;
mod hash_table;
pub mod instrumentation_profile;
pub mod summary;
pub mod util;

pub use crate::instrumentation_profile::{parse, parse_bytes};
pub use coverage::coverage_mapping::CoverageMapping;
pub use coverage::reporting::*;
pub use coverage::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ProfileFormat {
    Binary,
    CompactBinary,
    ExtBinary,
    Text,
    Gcc,
}

pub fn merge_profiles<T>(files: &[T]) -> std::io::Result<InstrumentationProfile>
where
    T: AsRef<Path>,
{
    let Some((first, rest)) = files.split_first() else {
        return Ok(InstrumentationProfile::default());
    };

    let mut base = parse(first)?;
    for input in rest {
        let profile = parse(input)?;
        base.merge(&profile);
    }
    Ok(base)
}
