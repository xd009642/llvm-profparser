use nom::IResult;
use std::collections::HashMap;
use std::convert::TryFrom;

pub mod coverage_mapping;
pub mod reporting;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoverageMappingInfo {
    cov_map: HashMap<u64, Vec<String>>,
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
pub enum ExprKind {
    Subtract,
    Add,
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
    /// A Branch Region represents lead-level boolean expressions and is associated with two
    /// counters, each representing the number of times the expression evaluates to true or false.
    Branch = 4,
}

impl TryFrom<u64> for RegionKind {
    type Error = u64;

    fn try_from(v: u64) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(RegionKind::Code),
            1 => Ok(RegionKind::Expansion),
            2 => Ok(RegionKind::Skipped),
            3 => Ok(RegionKind::Gap),
            4 => Ok(RegionKind::Branch),
            e => Err(e),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum CounterType {
    Zero,
    ProfileInstrumentation,
    Expression(ExprKind),
}

impl Default for CounterType {
    fn default() -> Self {
        Self::Zero
    }
}

pub(crate) fn parse_expression(kind: CounterType, input: u64) -> Counter {
    let id = input >> Counter::ENCODING_TAG_BITS;
    Counter { kind, id }
}

// Attempts to simplify RawCoverageMappingReader::decodeCounter
pub(crate) fn parse_counter(input: u64) -> Counter {
    let ty = (Counter::ENCODING_TAG_MASK & input) as u8;
    let kind = match ty {
        0 => CounterType::Zero,
        1 => CounterType::ProfileInstrumentation,
        2 => CounterType::Expression(ExprKind::Subtract),
        3 => CounterType::Expression(ExprKind::Add),
        _ => unreachable!(),
    };
    let id = input >> 2; // For zero we don't actually care about this but we'll still do it
    Counter { kind, id }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Counter {
    pub kind: CounterType,
    pub id: u64,
}

impl Counter {
    pub fn is_expression(&self) -> bool {
        matches!(self.kind, CounterType::Expression(_))
    }

    /// Gets the kind of the expression
    ///
    /// # Panics
    ///
    /// Panics if not an kind of `CounterType::Expression`
    pub fn get_expr_kind(&self) -> ExprKind {
        match self.kind {
            CounterType::Expression(e) => e,
            _ => panic!("Counter is not an expression"),
        }
    }
}

/// Is this equivalent to CounterExpression? Where's the ExprKind?
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Expression {
    kind: ExprKind,
    lhs: Counter,
    rhs: Counter,
}

impl Expression {
    pub fn new(lhs: Counter, rhs: Counter) -> Self {
        Self {
            kind: ExprKind::Subtract,
            lhs,
            rhs,
        }
    }

    pub fn set_kind(&mut self, kind: ExprKind) {
        self.kind = kind;
    }
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
    /// Primary counter that is also used for true branches
    count: Counter,
    /// Secondary counter that is also used for false branches
    false_count: Counter,
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
    expressions: Vec<Expression>,
}

pub struct CoverageMappingRecord {
    fn_name: String,
    fn_hash: u64,
    file_names: Vec<String>,
    expressions: Vec<Expression>,
    mapping_regions: Vec<CounterMappingRegion>,
}
