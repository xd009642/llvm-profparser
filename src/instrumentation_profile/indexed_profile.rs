use crate::hash_table::*;
use crate::instrumentation_profile::*;
use crate::summary::*;
use nom::{number::complete::*, IResult};
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, TryFromPrimitive)]
#[repr(u64)]
pub enum SummaryFieldKind {
    TotalNumFunctions,
    TotalNumBlocks,
    MaxFunctionCount,
    MaxBlockCount,
    MaxInternalBlockCount,
    TotalBlockCount,
}

impl Header {
    pub fn version(&self) -> u64 {
        self.version & !VARIANT_MASKS_ALL
    }

    pub fn is_csir_prof(&self) -> bool {
        (self.version & VARIANT_MASK_CSIR_PROF) > 0
    }

    pub fn is_ir_prof(&self) -> bool {
        (self.version & VARIANT_MASK_IR_PROF) > 0
    }
}

fn parse_summary<'a>(
    mut input: &'a [u8],
    header: &Header,
    use_cs: bool,
) -> IResult<&'a [u8], Option<ProfileSummary>> {
    if header.version() >= 4 {
        let (bytes, n_fields) = le_u64(input)?;
        let (bytes, n_entries) = le_u64(bytes)?;
        input = bytes;
        let mut fields = HashMap::new();
        for i in 0..n_fields {
            let (bytes, value) = le_u64(input)?;
            input = bytes;
            if let Ok(field) = SummaryFieldKind::try_from_primitive(i) {
                fields.insert(field, value);
            }
        }
        let mut detailed_summary = vec![];
        for _ in 0..n_entries {
            // Start getting the cutoffs
            let (bytes, cutoff) = le_u64(input)?;
            let (bytes, min_count) = le_u64(bytes)?;
            let (bytes, num_counts) = le_u64(bytes)?;
            input = bytes;
            detailed_summary.push(ProfileSummaryEntry {
                cutoff,
                min_count,
                num_counts,
            });
        }
        let kind = if use_cs { Kind::CsInstr } else { Kind::Instr };
        let total_count = fields
            .get(&SummaryFieldKind::TotalBlockCount)
            .copied()
            .unwrap_or_default();
        let max_count = fields
            .get(&SummaryFieldKind::MaxBlockCount)
            .copied()
            .unwrap_or_default();
        let max_internal_count = fields
            .get(&SummaryFieldKind::MaxInternalBlockCount)
            .copied()
            .unwrap_or_default();
        let max_function_count = fields
            .get(&SummaryFieldKind::MaxFunctionCount)
            .copied()
            .unwrap_or_default();
        let num_counts = fields
            .get(&SummaryFieldKind::TotalNumBlocks)
            .map(|x| *x as u32)
            .unwrap_or_default();
        let num_fns = fields
            .get(&SummaryFieldKind::TotalNumFunctions)
            .map(|x| *x as u32)
            .unwrap_or_default();
        let summary = ProfileSummary {
            kind,
            total_count,
            max_count,
            max_internal_count,
            max_function_count,
            num_counts,
            num_fns,
            partial: false,
            partial_profile_ratio: 0.0,
            detailed_summary,
        };
        Ok((input, Some(summary)))
    } else {
        Ok((input, None))
    }
}

impl InstrProfReader for IndexedInstrProf {
    type Header = Header;

    fn parse_bytes(mut input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input)?;
        let (bytes, summary) = parse_summary(bytes, &header, false)?;
        let (bytes, cs_summary) = if header.is_csir_prof() {
            parse_summary(bytes, &header, true)?
        } else {
            (bytes, None)
        };
        let mut profile = InstrumentationProfile::default();
        profile.version = Some(header.version);
        profile.has_csir = header.is_csir_prof();
        profile.is_ir = header.is_ir_prof();

        let table_start = input.len() - bytes.len();
        let (bytes, table) = HashTable::parse(
            header.version,
            bytes,
            table_start,
            header.hash_offset as usize - table_start,
        )?;
        input = bytes;
        for ((hash, name), v) in &table.0 {
            let name = name.to_string();
            profile.symtab.add_func_name(name.clone(), None);
            let record = NamedInstrProfRecord {
                name: Some(name),
                hash: Some(*hash),
                record: v.clone(),
            };
            profile.records.push(record);
        }
        Ok((input, profile))
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
