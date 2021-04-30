use crate::instrumentation_profile::types::*;
use std::fmt;

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ValueSiteStats {
    total_num_value_sites: usize,
    total_value_sites_with_value_profile: usize,
    total_num_values: usize,
}

impl ValueSiteStats {
    pub fn traverse_sites(
        &mut self,
        func: &InstrProfRecord,
        value: ValueKind,
        symtab: Option<&Symtab>,
    ) {
        todo!()
    }
}

impl fmt::Display for ValueSiteStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Total number of sites: {}", self.total_num_value_sites)?;
        writeln!(
            f,
            "  Total number of sites with values: {}",
            self.total_value_sites_with_value_profile
        )?;
        writeln!(
            f,
            "  Total number of profiled values: {}",
            self.total_num_values
        )?;
        write!(f, "  Value sites historgram:\n\tNumTargets, SiteCount")
    }
}
