use crate::instrumentation_profile::*;
use crate::summary::ProfileSummary;
use nom::{number::complete::*, IResult};
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::io::Read;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct IndexedInstrProf;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, TryFromPrimitive)]
#[repr(u64)]
pub enum HashType {
    Md5,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Header {
    version: u64,
    pub hash_type: HashType,
    pub hash_offset: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum SummaryFieldKind {
    TotalNumFunctions,
    TotalNumBlocks,
    MaxFunctionCount,
    MaxBlockCount,
    MaxInternalBlockCount,
    TotalBlockCount,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProfileSummaryEntry {
    pub cutoff: u64,
    pub min_count: u64,
    pub num_counts: u64,
}

impl ProfileSummaryEntry {
    /// Size of the fields in bytes
    const SIZE: usize = 24;
}

pub struct Summary {
    num_summary_fields: usize,
    num_summary_entries: usize,
}

impl Summary {
    const SIZE: usize = 16;

    pub fn size(&self) -> usize {
        self.num_summary_entries * ProfileSummaryEntry::SIZE
            + Self::SIZE
            + (8 * self.num_summary_fields)
    }
}

impl Header {
    pub fn version(&self) -> u64 {
        self.version & !VARIANT_MASKS_ALL
    }
}

fn read_summary<'a>(input: &'a [u8], header: &Header) -> IResult<&'a [u8], Option<ProfileSummary>> {
    if header.version() >= 4 {
        let (bytes, n_fields) = le_u64(input)?;
        let (bytes, n_entries) = le_u64(input)?;

        let summary = Summary {
            num_summary_fields: n_fields as usize,
            num_summary_entries: n_entries as usize,
        };

        todo!()
    } else {
        Ok((input, None))
    }
}

impl InstrProfReader for IndexedInstrProf {
    type Header = Header;

    fn parse_bytes(input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input)?;
        println!("Indexed header: {:?}", header);
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Self::Header> {
        if Self::has_format(input) {
            let (bytes, version) = le_u64(&input[8..])?;
            let (bytes, _) = le_u64(bytes)?;
            let (bytes, hash_type) = le_u64(bytes)?;
            let (bytes, hash_offset) = le_u64(bytes)?;
            let hash_type = HashType::try_from(hash_type).expect("BAD ENUM BRUH");
            Ok((
                bytes,
                Self::Header {
                    version,
                    hash_type,
                    hash_offset,
                },
            ))
        } else {
            todo!();
        }
    }

    fn has_format(mut input: impl Read) -> bool {
        const MAGIC: u64 = u64::from_le_bytes([0xff, 0x6c, 0x70, 0x72, 0x6f, 0x66, 0x69, 0x81]);
        let mut buffer: [u8; 8] = [0; 8];
        if input.read_exact(&mut buffer).is_ok() {
            u64::from_le_bytes(buffer) == MAGIC
        } else {
            false
        }
    }
}
