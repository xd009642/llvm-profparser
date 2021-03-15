use crate::instrumentation_profile::types::*;
use crate::instrumentation_profile::*;
use std::io;

/// The `BinaryProfWriter` writes out the file as an Indexed Instrumentation file.
#[derive(Debug, Clone, Copy, Default)]
pub struct BinaryProfWriter;

impl BinaryProfWriter {
    pub fn new() -> Self {
        Default::default()
    }
}

impl InstrProfWriter for BinaryProfWriter {
    fn write(&self, _profile: &InstrumentationProfile, _writer: &mut impl Write) -> io::Result<()> {
        todo!();
    }
}
