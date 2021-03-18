
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Instr,
    CsInstr,
    Sample,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ProfileSummary {
    kind: Kind,
    total_count: u64,
    max_count: u64,
    max_internal_count: u64,
    max_function_count: u64,
    num_counts: u32,
    num_fns: u32,
    partial: bool,
    partial_profile_ratio: f64,
}
