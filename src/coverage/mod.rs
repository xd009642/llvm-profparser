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

/// This is the type of a counter expression. The equivalent type in llvm would be
/// `CounterExpression::ExprKind` an inner enum.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ExprKind {
    /// Subtracts a counter from another
    Subtract,
    /// Adds a counter to another
    Add,
}

/// Defines what type of region a counter maps to. The equivalent type in llvm would be
/// `CounterMappingRegion::RegionKind`.
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

/// Represents the type of a counter. The equivalent type in llvm would be `Counter::CounterKind`.
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

/// A `Counter` is an abstract value that describes how to compute the execution count for a region
/// of code using the collected profile count data. The equivalent type in llvm would be `Counter`.
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

/// A counter expression is a value that represents an arithmetic operation between two counters.
/// The equivalent llvm type would be `CounterExpression`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Expression {
    pub kind: ExprKind,
    pub lhs: Counter,
    pub rhs: Counter,
}

impl Default for Expression {
    fn default() -> Self {
        Expression::new(Counter::default(), Counter::default())
    }
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

/// Associates a source code reader with a specific counter. The equivalent type in llvm would be
/// `CounterMappingRegion`.
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

/// The execution count information starting at a point in a file. A sequence of execution counters
/// for a file in a format hat's simple to iterate over for processing. The equivalent llvm type is
/// `CoverageSegment`.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CoverageSegment {
    /// The line the segment begins
    line: usize,
    /// The column the segment begins
    col: usize,
    /// The execution count, or zero if not executed
    count: usize,
    /// When false the segment is not instrumented or skipped
    has_count: bool,
    /// whether this enters a new region or returns to a previous count
    is_region_entry: usize,
    /// Whether this enters a gap region
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

/// Coverage mapping information for a single function. The equivalent llvm type is
/// `CoverageMappingRecord`.
pub struct CoverageMappingRecord {
    fn_name: String,
    fn_hash: u64,
    file_names: Vec<String>,
    expressions: Vec<Expression>,
    mapping_regions: Vec<CounterMappingRegion>,
}
