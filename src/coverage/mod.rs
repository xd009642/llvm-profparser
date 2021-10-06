use nom::IResult;
use std::convert::TryFrom;

pub mod coverage_mapping;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CoverageMappingInfo {
    cov_map: Vec<String>,
    cov_fun: Vec<FunctionRecordV3>,
    prof_names: Vec<String>,
    prof_counts: Vec<u64>,
    prof_data: Vec<ProfileData>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ProfileData {
    name_md5: u64,
    structural_hash: u64,
    counters_len: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum CounterKind {
    Zero,
    ValueReference,
    Expression,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ExprKind {
    Add,
    Subtract,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum RegionKind {
    /// A Code Region associates some code with a counter
    Code = 0,
    /// An Expansion Region represents a file expansion region that associates a source range with
    /// the expansion of a virtual source file, such as for a macro instantiation or include file
    Expansion = 1,
    /// A Skipped  Region represents a source range with code that was skipped by a preprocessor or
    /// similar means
    Skipped = 2,
    /// A Gap Region is like a Code Region but its count is only set as the line execution count
    /// when its the only region in the line
    Gap = 3,
}

impl TryFrom<u64> for RegionKind {
    type Error = ();

    fn try_from(v: u64) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(RegionKind::Code),
            1 => Ok(RegionKind::Expansion),
            2 => Ok(RegionKind::Skipped),
            3 => Ok(RegionKind::Gap),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum CounterType {
    Zero,
    ProfileInstrumentation,
    SubtractionExpr,
    AdditionExpr,
}

pub(crate) fn parse_counter(input: u64) -> Counter {
    let ty = (Counter::ENCODING_TAG_MASK & input) as u8;
    let kind = match ty {
        0 => CounterType::Zero,
        1 => CounterType::ProfileInstrumentation,
        2 => CounterType::SubtractionExpr,
        3 => CounterType::AdditionExpr,
        _ => unreachable!(),
    };
    let id = input >> 2; // For zero we don't actually care about this but we'll still do it
    Counter { kind, id }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Counter {
    pub kind: CounterType,
    id: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Expression {
    lhs: Counter,
    rhs: Counter,
}

impl Counter {
    const ENCODING_TAG_BITS: u64 = 2;
    const ENCODING_TAG_MASK: u64 = 3;
    const ENCODING_TAG_AND_EXP_REGION_BITS: u64 = 3;
    const ENCODING_EXPANSION_REGION_BIT: u64 = 4;
}

/// Associates a source code reader with a specific counter
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CounterMappingRegion {
    kind: RegionKind,
    count: Counter,
    file_id: usize,
    expanded_file_id: usize,
    line_start: usize,
    column_start: usize,
    line_end: usize,
    column_end: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CoverageSegment {
    line: usize,
    col: usize,
    count: usize,
    has_count: bool,
    is_region_entry: usize,
    is_gap_region: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FunctionRecordHeader {
    name_hash: u64,
    data_len: u32,
    func_hash: u64,
    filenames_ref: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FunctionRecordV3 {
    header: FunctionRecordHeader,
    regions: Vec<CounterMappingRegion>,
}
