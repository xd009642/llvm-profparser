use crate::instrumentation_profile::symtab::*;
use crate::instrumentation_profile::*;
use crate::util::parse_string_ref;
use core::hash::Hash;
use nom::lib::std::ops::RangeFrom;
use nom::number::streaming::{u16 as nom_u16, u32 as nom_u32, u64 as nom_u64};
use nom::number::Endianness;
use nom::{
    error::{Error, ErrorKind},
    Err, IResult, Needed,
};
use nom::{InputIter, InputLength, Slice};
use std::convert::TryInto;
use std::fmt::{Debug, Display};
use std::io;
use std::marker::PhantomData;
use std::mem::size_of;

const INDIRECT_CALL_TARGET: usize = 0;
const MEM_OP_SIZE: usize = 1;

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

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    phantom: PhantomData<T>,
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
    /// This depends on what type the header is on as it can be u32 or u64 width profiles
    pub data_size_bytes: u64,
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
    num_value_sites: [u16; MEM_OP_SIZE + 1],
}

#[derive(Clone, Debug, Default)]
pub struct InstrProfRecord {
    counts: Vec<u64>,
    value_prof_data: Option<Box<ValueProfData>>,
}

#[derive(Clone, Debug)]
pub struct ValueProfData {
    indirect_callsites: Vec<InstrProfValueSiteRecord>,
    mem_op_sizes: Vec<InstrProfValueSiteRecord>,
}

type InstrProfValueSiteRecord = Vec<InstrProfValueData>;

#[derive(Clone, Copy, Debug)]
pub struct InstrProfValueData {
    value: u64,
    count: u64,
}

impl Header {
    // I'm not storing the magic here or relying on repr(c) so hardcoding the size in
    // bytes of the header
    const HEADER_SIZE: usize = 80;

    pub fn counters_start(&self) -> u64 {
        Self::HEADER_SIZE as u64 + self.data_size_bytes + self.padding_bytes_before_counters
    }

    pub fn names_start(&self) -> u64 {
        self.counters_start() + (8 * self.counters_len) + self.padding_bytes_after_counters
    }

    pub fn value_data_start(&self) -> u64 {
        self.names_start() + self.names_len + get_num_padding_bytes(self.names_len) as u64
    }

    pub fn max_counters_len(&self) -> i64 {
        self.names_start() as i64 - self.counters_start() as i64
    }
}

/// Trait to represent memory widths. Currently just 32 or 64 bit. This implements Into<u64> so if
/// we ever move beyond 64 bit systems this code will have to change to Into<u128> or whatever the
/// next thing is.
pub trait MemoryWidthExt:
    Debug + Copy + Clone + Eq + PartialEq + Hash + Ord + PartialOrd + Display + Into<u64>
{
    const MAGIC: u64;

    fn profile_data_size() -> u64;

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

    fn profile_data_size() -> u64 {
        todo!()
    }

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

    fn profile_data_size() -> u64 {
        48 // TODO what I don't remember where this came from?
    }

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
        header: &'a Header,
        data: &'a ProfileData<T>,
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
            // I AM MALFORMED
            todo!()
        } else {
            let mut counts = Vec::<u64>::new();
            counts.reserve(data.num_counters as usize);
            for i in 0..(data.num_counters as usize) {
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
}

impl<T> InstrProfReader for RawInstrProf<T>
where
    T: MemoryWidthExt,
{
    type Header = Header;

    fn parse_bytes(input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input)?;
        let (_, data) = ProfileData::<T>::parse(&input[Header::HEADER_SIZE..], header.endianness)?;
        println!("File header: {:?}", header);
        println!("Profile data: {:?}", data);
        let mut names_remaining = header.names_len as usize;
        let mut bytes = &input[(header.names_start() as usize)..];
        let mut symtab = Symtab::default();
        while names_remaining > 0 {
            let start_len = bytes.len();
            let (new_bytes, names) = parse_string_ref(bytes)?;
            names_remaining -= (start_len - new_bytes.len());
            bytes = new_bytes;
            for name in names.split(INSTR_PROF_NAME_SEP) {
                symtab.add_func_name(name.to_string());
            }
        }
        println!("GOT the names. {:?}", symtab);
        // Missed out func hash but in source it appears to just be copying value from the Data
        // type above into some other types and fixing endianness if needed
        let raw_counters =
            Self::read_raw_counts(&header, &data, &input[(header.counters_start() as usize)..]);
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

            let data_size_bytes = T::profile_data_size() * data_len;

            let padding_size = get_num_padding_bytes(names_len);
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
                data_size_bytes,
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
