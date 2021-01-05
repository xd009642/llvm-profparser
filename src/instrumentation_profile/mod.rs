use core::hash::Hash;
use nom::multi::count;
use nom::number::streaming::u64 as nom_u64;
use nom::number::Endianness;
use nom::{error::Error, Err, IResult, Needed};
use std::convert::TryInto;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::marker::PhantomData;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrumentationProfile;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Header {
    version: u32,
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

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct IndexedInstrProf;
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    phantom: PhantomData<T>,
}
type RawInstrProf32 = RawInstrProf<u32>;
type RawInstrProf64 = RawInstrProf<u64>;
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TextInstrProf;

pub fn parse(filename: impl AsRef<Path>) -> io::Result<InstrumentationProfile> {
    let mut buffer = Vec::new();
    let mut f = File::open(filename)?;
    f.read_to_end(&mut buffer)?;
    if IndexedInstrProf::has_format(buffer.as_slice()) {
        IndexedInstrProf::parse_bytes(&buffer)
    } else if RawInstrProf64::has_format(buffer.as_slice()) {
        RawInstrProf64::parse_bytes(&buffer)
    } else if RawInstrProf32::has_format(buffer.as_slice()) {
        RawInstrProf32::parse_bytes(&buffer)
    } else if TextInstrProf::has_format(buffer.as_slice()) {
        TextInstrProf::parse_bytes(&buffer)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unsupported instrumentation profile format",
        ))
    }
}

pub trait InstrProfReader {
    /// Parse the profile no lazy parsing here!
    fn parse_bytes(input: &[u8]) -> io::Result<InstrumentationProfile>;
    /// Parses a header
    fn parse_header(input: &[u8]) -> IResult<&[u8], Header>;
    /// Detects that the bytes match the current reader format if it can't read the format it will
    /// return false
    fn has_format(input: impl Read) -> bool;
}

impl InstrProfReader for IndexedInstrProf {
    fn parse_bytes(input: &[u8]) -> io::Result<InstrumentationProfile> {
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Header> {
        todo!()
    }

    fn has_format(mut input: impl Read) -> bool {
        const MAGIC: [u8; 8] = [0xff, 0x6c, 0x70, 0x72, 0x6f, 0x66, 0x69, 0x81];
        let mut buffer: [u8; 8] = [0; 8];
        if input.read_exact(&mut buffer).is_ok() {
            buffer == MAGIC
        } else {
            false
        }
    }
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

impl<T> InstrProfReader for RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    fn parse_bytes(input: &[u8]) -> io::Result<InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input).unwrap();
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Header> {
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
        } else {
            //Err(Err::Error(Needed::new(len as _)))
        }
        todo!()
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

impl InstrProfReader for TextInstrProf {
    fn parse_bytes(input: &[u8]) -> io::Result<InstrumentationProfile> {
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Header> {
        todo!()
    }

    fn has_format(mut input: impl Read) -> bool {
        // looking at the code it looks like with file memory buffers in llvm it sets the buffer
        // size to the size of the file meaning it checks all the characters
        let mut s = String::new();
        if input.read_to_string(&mut s).is_ok() {
            s.is_ascii()
        } else {
            false
        }
    }
}
