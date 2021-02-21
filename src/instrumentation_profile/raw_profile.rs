use crate::instrumentation_profile::symtab::*;
use crate::instrumentation_profile::*;
use crate::util::parse_string_ref;
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
    num_value_sites: Vec<u16>,
}

impl Header {
    // I'm not storing the magic here or relying on repr(c) so hardcoding the size in
    // bytes of the header
    const HEADER_SIZE: u64 = 80;

    pub fn counter_offset(&self) -> u64 {
        Self::HEADER_SIZE + self.data_size_bytes + self.padding_bytes_before_counters
    }

    pub fn names_offset(&self) -> u64 {
        self.counter_offset() + (8 * self.counters_len) + self.padding_bytes_after_counters
    }

    pub fn value_data_offset(&self) -> u64 {
        self.names_offset() + self.names_len + get_num_padding_bytes(self.names_len) as u64
    }
}

pub trait MemoryWidthExt:
    Debug + Clone + Eq + PartialEq + Hash + Ord + PartialOrd + Display
{
    const MAGIC: u64;

    fn profile_data_size() -> u64;
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
        48
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
    type Header = Header;

    fn parse_bytes(input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input)?;
        println!("File header: {:?}", header);
        let mut names_remaining = header.names_len as usize;
        let mut bytes = &input[(header.names_offset() as usize)..];
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

            let data_size_bytes = T::profile_data_size() * data_len;

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
