#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Instr,
    CsInstr,
    Sample,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProfileSummaryEntry {
    pub cutoff: u64,
    pub min_count: u64,
    pub num_counts: u64,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ProfileSummary {
    pub kind: Kind,
    pub total_count: u64,
    pub max_count: u64,
    pub max_internal_count: u64,
    pub max_function_count: u64,
    pub num_counts: u32,
    pub num_fns: u32,
    pub partial: bool,
    pub partial_profile_ratio: f64,
    pub detailed_summary: Vec<ProfileSummaryEntry>,
}
