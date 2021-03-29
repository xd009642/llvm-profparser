mod hash_table;
pub mod instrumentation_profile;
pub mod summary;
pub mod util;

pub use crate::instrumentation_profile::{parse, parse_bytes};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ProfileFormat {
    Binary,
    CompactBinary,
    ExtBinary,
    Text,
    Gcc,
}
