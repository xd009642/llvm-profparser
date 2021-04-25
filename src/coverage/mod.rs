use nom::IResult;

pub mod coverage_mapping;

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
    Code,
    /// An Expansion Region represents a file expansion region that associates a source range with
    /// the expansion of a virtual source file, such as for a macro instantiation or include file
    Expansion,
    /// A Skipped  Region represents a source range with code that was skipped by a preprocessor or
    /// similar means
    Skipped,
    /// A Gap Region is like a Code Region but its count is only set as the line execution count
    /// when its the only region in the line
    Gap,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum CounterType {
    Zero,
    ProfileInstrumentation,
    SubtractionExpr,
    AdditionExpr,
}

fn parse_counter_type(input: &[u8]) -> IResult<&[u8], CounterType> {
    let ty = (0x3 & input[0]) as u8;
    let ty = match ty {
        0 => CounterType::Zero,
        1 => CounterType::ProfileInstrumentation,
        2 => CounterType::SubtractionExpr,
        3 => CounterType::AdditionExpr,
        _ => unreachable!(),
    };
    Ok((&input[1..], ty))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Counter {
    pub kind: CounterKind,
    id: usize,
}

impl Counter {
    const ENCODING_TAG_BITS: usize = 2;
    const ENCODING_TAG_MASK: usize = 3;
    const ENCODING_COUNTER_TAG_AND_EXP_REGION_TAG_BITS: usize = 4;
}

pub struct CounterMappingRegion {
    kind: RegionKind,
    count: Counter,
    file_id: usize,
    expanded_file_id: usize,
    line_start: usize,
    column_start: usize,
    line_end: usize,
    column_end: usize,
    /// This is in the CountedRegion, but I don't see the need to do another type like when they
    /// inherit in llvm. After all there are no overloads
    execution_count: Option<u64>,
}

pub struct CoverageSegment {
    line: usize,
    col: usize,
    count: usize,
    has_count: bool,
    is_region_entry: usize,
    is_gap_region: usize,
}
