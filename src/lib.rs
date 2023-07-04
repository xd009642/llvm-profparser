use crate::instrumentation_profile::types::InstrumentationProfile;
use rayon::prelude::*;
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
    T: AsRef<Path> + Sync + Send,
{
    if files.is_empty() {
        Ok(InstrumentationProfile::default())
    } else {
        let mut profiles = files
            .par_iter()
            .map(|input| parse(input))
            .collect::<Vec<_>>();

        let mut base = profiles.remove(0)?;
        for profile in profiles.drain(..) {
            let profile = profile?;
            base.merge(&profile);
        }
        Ok(base)
    }
}
