use crate::instrumentation_profile::*;
use core::hash::Hash;
use nom::number::streaming::u64 as nom_u64;
use nom::number::Endianness;
use nom::{
    error::{Error, ErrorKind},
    Err, IResult, Needed,
};
use std::convert::TryInto;
use std::fmt::{Debug, Display};
use std::io;
use std::marker::PhantomData;

pub type RawInstrProf32 = RawInstrProf<u32>;
pub type RawInstrProf64 = RawInstrProf<u64>;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    phantom: PhantomData<T>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Header {
    pub version: u64,
    pub data_len: u64,
    pub padding_bytes_before_counters: u64,
    pub counters_len: u64,
    pub padding_bytes_after_counters: u64,
    pub names_len: u64,
    pub counters_delta: u64,
    pub names_delta: u64,
    pub value_kind_last: u64,
}

pub trait MemoryWidthExt:
    Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd + Display
{
    const MAGIC: u64;
}

impl MemoryWidthExt for u32 {
    const MAGIC: u64 = (255 << 56)
        | ('l' as u64) << 48
        | ('p' as u64) << 40
        | ('r' as u64) << 32
        | ('o' as u64) << 24
        | ('f' as u64) << 16
        | ('R' as u64) << 8
        | 129;
}
impl MemoryWidthExt for u64 {
    const MAGIC: u64 = (255 << 56)
        | ('l' as u64) << 48
        | ('p' as u64) << 40
        | ('r' as u64) << 32
        | ('o' as u64) << 24
        | ('f' as u64) << 16
        | ('r' as u64) << 8
        | 129;
}

fn file_endianness<T>(magic: &[u8; 8]) -> Endianness
where
    T: MemoryWidthExt,
{
    // native endian and reversed endian
    let provided = u64::from_le_bytes(*magic);
    if provided == T::MAGIC {
        Endianness::Little
    } else if provided.swap_bytes() == T::MAGIC {
        Endianness::Big
    } else {
        unreachable!("Invalid magic provided");
    }
}

impl<T> RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    fn read_name(input: &[u8], header: &Header) {
        let input = &input[(header.names_len as usize)..];
        println!("thing? {:?}", input);
    }
}

impl<T> InstrProfReader for RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    type Header = Header;

    fn parse_bytes(input: &[u8]) -> io::Result<InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input).unwrap();
        println!("File header: {:?}", header);
        Self::read_name(bytes, &header);
        // read name
        // read func hash
        // read raw counts
        // read value profiling data
        // Next record
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Self::Header> {
        if Self::has_format(input) {
            let endianness = file_endianness::<T>(&input[..8].try_into().unwrap());
            let (bytes, version) = nom_u64(endianness)(&input[8..])?;
            let (bytes, data_len) = nom_u64(endianness)(&bytes[..])?;
            let (bytes, padding_bytes_before_counters) = nom_u64(endianness)(&bytes[..])?;
            let (bytes, counters_len) = nom_u64(endianness)(&bytes[..])?;
            let (bytes, padding_bytes_after_counters) = nom_u64(endianness)(&bytes[..])?;
            let (bytes, names_len) = nom_u64(endianness)(&bytes[..])?;
            let (bytes, counters_delta) = nom_u64(endianness)(&bytes[..])?;
            let (bytes, names_delta) = nom_u64(endianness)(&bytes[..])?;
            let (bytes, value_kind_last) = nom_u64(endianness)(&bytes[..])?;

            let padding_size = get_num_padding_bytes(names_len);
            let result = Header {
                version,
                data_len,
                padding_bytes_before_counters,
                counters_len,
                padding_bytes_after_counters,
                names_len,
                counters_delta,
                names_delta,
                value_kind_last,
            };
            Ok((bytes, result))
        } else {
            Err(Err::Failure(Error::new(input, ErrorKind::IsNot)))
        }
    }

    fn has_format(mut input: impl Read) -> bool {
        let mut buffer: [u8; 8] = [0; 8];
        if input.read_exact(&mut buffer).is_ok() {
            let magic = u64::from_ne_bytes(buffer);
            T::MAGIC == magic || T::MAGIC == magic.swap_bytes()
        } else {
            false
        }
    }
}
