use nom::multi::count;
use nom::{error::Error, take_str, Err, IResult, Needed};

pub mod instrumentation_profile;
pub mod util;

pub use crate::instrumentation_profile::parse;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
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
