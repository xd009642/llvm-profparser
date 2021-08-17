use crate::instrumentation_profile::types::*;
use crate::util::*;
use object::{Endian, Endianness, Object, ObjectSection, Section};
use std::convert::TryInto;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

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

            let covfun = object_file
                .section_by_name("__llvm_covfun")
                .or(object_file.section_by_name(".lcovfun$M"))
                .map(|x| parse_coverage_functions(object_file.endianess(), &x));
        }

        todo!()
    }
}

fn parse_coverage_mapping<'data, 'file>(
    endian: Endianness,
    section: &Section<'data, 'file>,
) -> Vec<String> {
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

        file_strings
    }
}

fn parse_coverage_functions<'data, 'file>(endian: Endianness, section: &Section<'data, 'file>) {
    todo!()
}
