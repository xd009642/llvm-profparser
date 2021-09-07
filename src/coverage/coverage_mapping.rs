use crate::coverage::*;
use crate::instrumentation_profile::types::*;
use crate::util::*;
use object::{Endian, Endianness, Object, ObjectSection, Section};
use std::convert::TryInto;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Copy, Clone, Debug)]
pub enum SectionReadError {
    EmptyData,
}

pub struct CoverageMapping<'a> {
    profile: &'a InstrumentationProfile,
}

impl<'a> CoverageMapping<'a> {
    pub fn new(
        object_files: &[PathBuf],
        profile: &'a InstrumentationProfile,
    ) -> Result<Self, Box<dyn Error>> {
        for object in object_files {
            let binary_data = fs::read(object)?;
            let object_file = object::File::parse(&*binary_data)?;
            let covmap = object_file
                .section_by_name("__llvm_covmap")
                .or(object_file.section_by_name(".lcovmap$M"))
                .map(|x| parse_coverage_mapping(object_file.endianness(), &x));

            // names
            let prof_names = object_file
                .section_by_name("__llvm_prf_names")
                .or(object_file.section_by_name(".lprfn$M"))
                .map(|x| parse_profile_names(&x));

            // counters
            let prof_counts = object_file
                .section_by_name("__llvm_prf_cnts")
                .or(object_file.section_by_name(".lprfc$M"))
                .map(|x| parse_profile_counters(object_file.endianness(), &x));

            // Data
            let prof_data = object_file
                .section_by_name("__llvm_prf_data")
                .or(object_file.section_by_name(".lprfd$M"))
                .map(|x| parse_profile_data(object_file.endianness(), &x));

            // I don't think I need vnodes currently?
            println!("{:?}", covmap);
            println!("{:?}", prof_names);
            println!("{:?}", prof_counts);
            println!("{:?}", prof_data);

            let covfun = object_file
                .section_by_name("__llvm_covfun")
                .or(object_file.section_by_name(".lcovfun$M"))
                .map(|x| parse_coverage_functions(object_file.endianness(), &x));
            println!("{:?}", covfun);
        }

        todo!()
    }
}

fn parse_coverage_mapping<'data, 'file>(
    endian: Endianness,
    section: &Section<'data, 'file>,
) -> Result<Vec<String>, SectionReadError> {
    if let Ok(data) = section.data() {
        println!("Length: {}", data.len());
        // Read the number of affixed function records (now just 0 as not in this header)
        debug_assert_eq!(endian.read_i32_bytes(data[0..4].try_into().unwrap()), 0);
        let filename_data_len = endian.read_i32_bytes(data[4..8].try_into().unwrap());
        // Read the length of the affixed string that contains encoded coverage mapping data (now 0
        // as not in this header)
        debug_assert_eq!(endian.read_i32_bytes(data[8..12].try_into().unwrap()), 0);
        let format_version = endian.read_i32_bytes(data[12..16].try_into().unwrap());
        println!(
            "Filename len {} format_version {}",
            filename_data_len, format_version
        );

        //let bytes = &data[16..(16 + filename_data_len as usize)];
        let bytes = &data[16..];
        let (bytes, file_count) = parse_leb128(bytes).unwrap();
        let mut file_strings = vec![];
        let mut bytes = bytes;
        for _ in 0..file_count {
            let (by, string) = parse_string_ref(bytes).unwrap();
            bytes = by;
            file_strings.push(string.trim().to_string());
        }

        println!(
            "Filecount {} remaining: {}\n strings: {:?}",
            file_count,
            bytes.len(),
            file_strings
        );

        // What do I do with the rest of the bytes? Who knows?
        println!("leftovers?: {:?}", bytes);

        Ok(file_strings)
    } else {
        Err(SectionReadError::EmptyData)
    }
}

fn parse_coverage_functions<'data, 'file>(
    endian: Endianness,
    section: &Section<'data, 'file>,
) -> Result<Vec<FunctionRecordV3>, SectionReadError> {
    if let Ok(data) = section.data() {
        let mut bytes = data;
        let mut res = vec![];
        let section_len = bytes.len();
        while !bytes.is_empty() {
            let name_hash = endian.read_i64_bytes(bytes[0..8].try_into().unwrap());
            let data_len = endian.read_u32_bytes(bytes[8..12].try_into().unwrap());
            let func_hash = endian.read_i64_bytes(bytes[12..20].try_into().unwrap());
            let filenames_ref = endian.read_u64_bytes(bytes[20..28].try_into().unwrap());
            let header = FunctionRecordHeader {
                name_hash,
                data_len,
                func_hash,
                filenames_ref,
            };
            bytes = &bytes[28..];
            let start_len = bytes.len();

            let (data, id_len) = parse_leb128(bytes).unwrap();
            bytes = data;
            let mut filename_indices = vec![];
            for _ in 0..id_len {
                let (data, id) = parse_leb128(bytes).unwrap();
                filename_indices.push(id);
                bytes = data;
            }
            let (data, expr_len) = parse_leb128(bytes).unwrap();
            bytes = data;
            let mut exprs = vec![];
            for _ in 0..expr_len {
                let (data, lhs) = parse_leb128(bytes).unwrap();
                let (data, rhs) = parse_leb128(data).unwrap();
                let lhs = parse_counter(lhs);
                let rhs = parse_counter(rhs);
                exprs.push(Expression { lhs, rhs });
                bytes = data;
            }
            let function_len = start_len - bytes.len(); // this should match header

            let padding = if function_len < section_len && (function_len & 0x07) != 0 {
                8 - (function_len & 0x07)
            } else {
                0
            };
            bytes = &bytes[padding..];

            // Now apply padding, and if hash is 0 move on as it's a dummy otherwise add to result
            // And decide what end type will be
        }
        Ok(res)
    } else {
        Err(SectionReadError::EmptyData)
    }
}

#[derive(Debug, Clone)]
pub struct ProfileData {
    name_md5: i64,
    structural_hash: u64,
    counters_len: u32,
}

fn parse_profile_data<'data, 'file>(
    endian: Endianness,
    section: &Section<'data, 'file>,
) -> Result<Vec<ProfileData>, SectionReadError> {
    if let Ok(data) = section.data() {
        let mut bytes = &data[..];
        let mut res = vec![];
        let mut next_expected_pointer = None;
        while !bytes.is_empty() {
            let name_md5 = endian.read_i64_bytes(bytes[..8].try_into().unwrap());
            let structural_hash = endian.read_u64_bytes(bytes[8..16].try_into().unwrap());
            let counter_ptr = endian.read_u64_bytes(bytes[16..24].try_into().unwrap());
            bytes = &bytes[(24 + 16)..];
            let counters_len = endian.read_u32_bytes(bytes[..4].try_into().unwrap());
            if let Some(next_ptr) = next_expected_pointer {
                if next_ptr != counter_ptr {
                    panic!("The pointers don't match");
                }
            }
            next_expected_pointer = Some(counter_ptr + 8 * counters_len as u64);
            bytes = &bytes[8..];
            res.push(ProfileData {
                name_md5,
                structural_hash,
                counters_len,
            });
        }
        Ok(res)
    } else {
        Err(SectionReadError::EmptyData)
    }
}

fn parse_profile_counters<'data, 'file>(
    endian: Endianness,
    section: &Section<'data, 'file>,
) -> Result<Vec<u64>, SectionReadError> {
    if let Ok(data) = section.data() {
        let mut result = vec![];
        for i in (0..data.len()).step_by(8) {
            if data.len() < (i + 8) {
                break;
            }
            result.push(endian.read_u64_bytes(data[i..(i + 8)].try_into().unwrap()));
        }
        Ok(result)
    } else {
        Err(SectionReadError::EmptyData)
    }
}

fn parse_profile_names<'data, 'file>(
    section: &Section<'data, 'file>,
) -> Result<Vec<String>, SectionReadError> {
    if let Ok(data) = section.data() {
        let mut bytes = &data[..];
        let mut res = vec![];
        while !bytes.is_empty() {
            let (new_bytes, string) = parse_string_ref(bytes).unwrap();
            bytes = new_bytes;
            res.push(string);
        }
        Ok(res)
    } else {
        Err(SectionReadError::EmptyData)
    }
}
