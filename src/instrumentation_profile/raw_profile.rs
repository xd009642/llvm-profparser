use crate::instrumentation_profile::types::*;
use crate::instrumentation_profile::*;
use crate::util::parse_string_ref;
use core::hash::Hash;
use nom::lib::std::ops::RangeFrom;
use nom::number::streaming::{u16 as nom_u16, u32 as nom_u32, u64 as nom_u64};
use nom::number::Endianness;
use nom::{
    error::{Error, ErrorKind},
    take, Err, IResult, Needed,
};
use nom::{InputIter, InputLength, Slice};
use std::convert::TryInto;
use std::fmt::{Debug, Display};
use std::mem::size_of;

const VARIANT_MASKS_ALL: u64 = 0xff00_0000_0000_0000;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
const VARIANT_MASK_IR_PROF: u64 = 1u64 << 56;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
const VARIANT_MASK_CSIR_PROF: u64 = 1u64 << 57;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum RawProfileError {
    Eof,
    UnrecognizedFormat,
    BadMagic(u64),
    UnsupportedVersion(usize),
    UnsupportedHashType,
    TooLarge,
    Truncated,
    Malformed,
    UnknownFunction,
    HashMismatch,
    CountMismatch,
    CounterOverflow,
    ValueSiteCountMismatch,
    CompressFailed,
    UncompressFailed,
    EmptyRawProfile,
}

const INSTR_PROF_NAME_SEP: char = '\u{1}';

pub type RawInstrProf32 = RawInstrProf<u32>;
pub type RawInstrProf64 = RawInstrProf<u64>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    header: Header,
    data: Vec<ProfileData<T>>,
    records: Vec<InstrProfRecord>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Header {
    endianness: Endianness,
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

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ProfileData<T> {
    name_ref: u64,
    func_hash: u64,
    counter_ptr: T,
    function_addr: T,
    values_ptr_expr: T,
    num_counters: u32,
    /// This might just be two values?
    num_value_sites: [u16; ValueKind::MemOpSize as usize + 1],
}

impl Header {
    pub fn max_counters_len(&self) -> i64 {
        ((8 * self.counters_len) + self.padding_bytes_after_counters) as i64
    }
}

/// Trait to represent memory widths. Currently just 32 or 64 bit. This implements Into<u64> so if
/// we ever move beyond 64 bit systems this code will have to change to Into<u128> or whatever the
/// next thing is.
pub trait MemoryWidthExt:
    Debug + Copy + Clone + Eq + PartialEq + Hash + Ord + PartialOrd + Display + Into<u64>
{
    const MAGIC: u64;

    fn nom_parse_fn<I>(endianness: Endianness) -> fn(_: I) -> IResult<I, Self>
    where
        I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength;
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

    fn nom_parse_fn<I>(endianness: Endianness) -> fn(_: I) -> IResult<I, Self>
    where
        I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
    {
        nom_u32(endianness)
    }
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

    fn nom_parse_fn<I>(endianness: Endianness) -> fn(_: I) -> IResult<I, Self>
    where
        I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
    {
        nom_u64(endianness)
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

impl<T> RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    fn read_raw_counts<'a>(
        header: &Header,
        data: &ProfileData<T>,
        mut bytes: &'a [u8],
    ) -> IResult<&'a [u8], InstrProfRecord> {
        let max_counters = header.max_counters_len();
        let counter_offset = (data.counter_ptr.into() as i64 - header.counters_delta as i64)
            / size_of::<u64>() as i64;
        if data.num_counters == 0
            || max_counters < 0
            || data.num_counters as i64 > max_counters
            || counter_offset < 0
            || counter_offset > max_counters
            || counter_offset + data.num_counters as i64 > max_counters
        {
            Err(Err::Failure(Error::new(bytes, ErrorKind::Satisfy)))
        } else {
            let mut counts = Vec::<u64>::new();
            counts.reserve(data.num_counters as usize);
            for _ in 0..(data.num_counters as usize) {
                let (b, counter) = nom_u64(header.endianness)(bytes)?;
                bytes = b;
                counts.push(counter);
            }
            let record = InstrProfRecord {
                counts,
                ..Default::default()
            };
            Ok((bytes, record))
        }
    }

    fn read_value_profiling_data<'a>(
        header: &Header,
        data: &ProfileData<T>,
        bytes: &'a [u8],
        record: &mut InstrProfRecord,
    ) -> IResult<&'a [u8], ()> {
        // record clear value data
        if data.num_value_sites.iter().all(|x| *x == 0) {
            // Okay so there's no value profiling data. So the next byte is actually a header
            // wewww
            Ok((bytes, ()))
        } else {
            let (bytes, total_size) = nom_u32(header.endianness)(bytes)?;
            todo!()
        }
    }
}

impl<T> InstrProfReader for RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    type Header = Header;

    fn parse_bytes(mut input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        if !input.is_empty() {
            let mut result = InstrumentationProfile::default();
            let (bytes, header) = Self::parse_header(input)?;

            result.version = Some(header.version & !VARIANT_MASKS_ALL);
            result.is_ir = (header.version & VARIANT_MASK_IR_PROF) != 0;
            result.has_csir = (header.version & VARIANT_MASK_CSIR_PROF) != 0;
            input = bytes;
            let mut data_section = vec![];
            for _ in 0..header.data_len {
                let (bytes, data) = ProfileData::<T>::parse(input, header.endianness)?;
                data_section.push(data);
                input = bytes;
            }
            let (bytes, _) = take!(input, header.padding_bytes_before_counters)?;
            input = bytes;
            let mut counters = vec![];
            for data in &data_section {
                let (bytes, record) = Self::read_raw_counts(&header, data, input)?;
                counters.push(record);
                input = bytes;
            }
            let (bytes, _) = take!(input, header.padding_bytes_after_counters)?;
            input = bytes;
            let end_length = input.len() - header.names_len as usize;
            let mut symtab = Symtab::default();
            while input.len() > end_length {
                let (new_bytes, names) = parse_string_ref(input)?;
                input = new_bytes;
                for name in names.split(INSTR_PROF_NAME_SEP) {
                    symtab.add_func_name(name.to_string());
                }
            }
            let padding = get_num_padding_bytes(header.names_len);
            let (bytes, _) = take!(input, padding)?;
            input = bytes;
            for (data, mut record) in data_section.iter().zip(counters.drain(..)) {
                let (bytes, _) =
                    Self::read_value_profiling_data(&header, &data, input, &mut record)?;
                input = bytes;
                let name = symtab.names.get(&data.name_ref).cloned(); // TODO are name_ref and func_hash right on data?
                let hash = if name.is_some() {
                    Some(data.func_hash)
                } else {
                    None
                };
                result
                    .records
                    .push(NamedInstrProfRecord { name, hash, record });
            }
            result.symtab = symtab;
            Ok((input, result))
        } else {
            // Okay return an error here
            todo!()
        }
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

            let result = Header {
                endianness,
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

impl<T> ProfileData<T>
where
    T: MemoryWidthExt,
{
    fn parse(bytes: &[u8], endianness: Endianness) -> IResult<&[u8], Self> {
        let parse = T::nom_parse_fn(endianness);

        let (bytes, name_ref) = nom_u64(endianness)(&bytes[..])?;
        let (bytes, func_hash) = nom_u64(endianness)(&bytes[..])?;
        let (bytes, counter_ptr) = parse(&bytes[..])?;
        let (bytes, function_addr) = parse(&bytes[..])?;
        let (bytes, values_ptr_expr) = parse(&bytes[..])?;
        let (bytes, num_counters) = nom_u32(endianness)(&bytes[..])?;
        let (bytes, value_0) = nom_u16(endianness)(&bytes[..])?;
        let (bytes, value_1) = nom_u16(endianness)(&bytes[..])?;

        Ok((
            bytes,
            Self {
                name_ref,
                func_hash,
                counter_ptr,
                function_addr,
                values_ptr_expr,
                num_counters,
                num_value_sites: [value_0, value_1],
            },
        ))
    }
}
