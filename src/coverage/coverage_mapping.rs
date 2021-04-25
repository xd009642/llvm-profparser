use crate::instrumentation_profile::types::*;
use std::path::PathBuf;

pub struct CoverageMapping<'a> {
    profile: &'a InstrumentationProfile,
    //objects:
}

impl<'a> CoverageMapping<'a> {
    pub fn new(_objects: &[PathBuf], profile: &'a InstrumentationProfile) -> Self {
        Self { profile }
    }
}
