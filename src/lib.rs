use nom::multi::count;
use nom::{error::Error, take_str, Err, IResult, Needed};

pub mod instrumentation_profile;
pub mod util;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CoverageData {
    file_ids: FileIdMapping,
    counter_exprs: Vec<Counter>,
    mapping_regions: (),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FileIdMapping {
    num_indices: u64,
    indices: Vec<u64>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum CounterType {
    Zero,
    ProfileInstrumentation,
    SubtractionExpr,
    AdditionExpr,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Counter {
    ty: CounterType,
    data: u64,
}

impl CoverageData {
    pub fn parse(input: &[u8]) -> Result<Self, Err<Error<&[u8]>>> {
        let (bytes, filename_indexes) = parse_filename_indexes(input)?;
        println!("filename indexes: {:?}", filename_indexes);
        let (bytes, counters) = parse_counter_expressions(bytes)?;
        println!("counters: {:?}", counters);
        todo!()
    }
}

pub(crate) fn parse_string(input: &[u8]) -> IResult<&[u8], &str> {
    let (remaining, len) = parse_leb128(input)?;
    if remaining.len() < len as _ {
        Err(Err::Incomplete(Needed::new(len as _)))
    } else {
        take_str!(remaining, len)
    }
}

fn parse_leb128(mut input: &[u8]) -> IResult<&[u8], u64> {
    let x = leb128::read::unsigned(&mut input).unwrap();
    Ok((input, x))
}

pub(crate) fn parse_filename_indexes(input: &[u8]) -> IResult<&[u8], Vec<u64>> {
    let (remaining, len) = parse_leb128(input)?;
    count(parse_leb128, len as usize)(remaining)
}

fn parse_counter_expressions(input: &[u8]) -> IResult<&[u8], Vec<Counter>> {
    let (remaining, len) = parse_leb128(input)?;
    count(parse_counter_expression, len as usize)(remaining)
}

fn parse_counter_expression(input: &[u8]) -> IResult<&[u8], Counter> {
    let (bytes, data) = parse_leb128(input)?;
    let ty = (0x3 & data) as u8;
    let ty = match ty {
        0 => CounterType::Zero,
        1 => CounterType::ProfileInstrumentation,
        2 => CounterType::SubtractionExpr,
        3 => CounterType::AdditionExpr,
        _ => unreachable!(),
    };
    let data = data >> 2;
    let counter = Counter { data, ty };
    Ok((bytes, counter))
}
