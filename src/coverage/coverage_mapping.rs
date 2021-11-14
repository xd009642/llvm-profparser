use crate::coverage::*;
use crate::instrumentation_profile::types::*;
use crate::util::*;
use object::{Endian, Endianness, Object, ObjectSection, Section};
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CoverageMapping<'a> {
    profile: &'a InstrumentationProfile,
}

#[derive(Copy, Clone, Debug)]
pub enum LlvmSection {
    CoverageMap,
    ProfileNames,
    ProfileCounts,
    ProfileData,
    CoverageFunctions,
}

#[derive(Copy, Clone, Debug)]
pub enum SectionReadError {
    EmptySection(LlvmSection),
    MissingSection(LlvmSection),
}

impl fmt::Display for SectionReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySection(s) => write!(f, "empty section: {:?}", s),
            Self::MissingSection(s) => write!(f, "missing section: {:?}", s),
        }
    }
}

impl Error for SectionReadError {}

pub fn read_object_file(object: &Path) -> Result<CoverageMappingInfo, Box<dyn Error>> {
    // I believe vnode sections added by llvm are unnecessary

    let binary_data = fs::read(object)?;
    let object_file = object::File::parse(&*binary_data)?;

    let cov_fun = object_file
        .section_by_name("__llvm_covfun")
        .or(object_file.section_by_name(".lcovfun$M"))
        .map(|x| parse_coverage_functions(object_file.endianness(), &x))
        .ok_or(SectionReadError::MissingSection(
            LlvmSection::CoverageFunctions,
        ))??;

    let cov_map = object_file
        .section_by_name("__llvm_covmap")
        .or(object_file.section_by_name(".lcovmap$M"))
        .map(|x| parse_coverage_mapping(object_file.endianness(), &x))
        .ok_or(SectionReadError::MissingSection(LlvmSection::CoverageMap))??;

    let prof_names = object_file
        .section_by_name("__llvm_prf_names")
        .or(object_file.section_by_name(".lprfn$M"))
        .map(|x| parse_profile_names(&x))
        .ok_or(SectionReadError::MissingSection(LlvmSection::ProfileNames))??;

    let prof_counts = object_file
        .section_by_name("__llvm_prf_cnts")
        .or(object_file.section_by_name(".lprfc$M"))
        .map(|x| parse_profile_counters(object_file.endianness(), &x))
        .ok_or(SectionReadError::MissingSection(LlvmSection::ProfileCounts))??;

    let prof_data = object_file
        .section_by_name("__llvm_prf_data")
        .or(object_file.section_by_name(".lprfd$M"))
        .map(|x| parse_profile_data(object_file.endianness(), &x))
        .ok_or(SectionReadError::MissingSection(LlvmSection::ProfileData))??;

    Ok(CoverageMappingInfo {
        cov_map,
        cov_fun,
        prof_names,
        prof_counts,
        prof_data,
    })
}

impl<'a> CoverageMapping<'a> {
    pub fn new(
        object_files: &[PathBuf],
        profile: &'a InstrumentationProfile,
    ) -> Result<Self, Box<dyn Error>> {
        let mut mappings = vec![];
        println!("profile:\n{:#?}", profile);
        for file in object_files {
            mappings.push(read_object_file(file.as_path())?);
        }
        println!("Mappings:\n{:#?}", mappings);
        todo!("Mapping of coverage profile and counters");
    }
}

fn parse_coverage_mapping<'data, 'file>(
    endian: Endianness,
    section: &Section<'data, 'file>,
) -> Result<HashMap<u64, Vec<String>>, SectionReadError> {
    if let Ok(mut data) = section.data() {
        let mut result = HashMap::new();
        while !data.is_empty() {
            let data_len = data.len();
            // Read the number of affixed function records (now just 0 as not in this header)
            debug_assert_eq!(endian.read_i32_bytes(data[0..4].try_into().unwrap()), 0);
            let filename_data_len = endian.read_i32_bytes(data[4..8].try_into().unwrap());
            // Read the length of the affixed string that contains encoded coverage mapping data (now 0
            // as not in this header)
            debug_assert_eq!(endian.read_i32_bytes(data[8..12].try_into().unwrap()), 0);
            let format_version = endian.read_i32_bytes(data[12..16].try_into().unwrap());

            let hash = md5::compute(&data[16..(filename_data_len as usize + 16)]);
            let hash = endian.read_u64_bytes(hash.0[..8].try_into().unwrap());

            //let bytes = &data[16..(16 + filename_data_len as usize)];
            let bytes = &data[16..];
            let (bytes, file_strings) = parse_string_list(bytes).unwrap();
            result.insert(hash, file_strings);
            let read_len = data_len - bytes.len();
            let padding = if !bytes.is_empty() && (read_len & 0x07) != 0 {
                8 - (read_len & 0x07)
            } else {
                0
            };
            if padding > bytes.len() {
                break;
            }
            data = &bytes[padding..];
        }
        Ok(result)
    } else {
        Err(SectionReadError::EmptySection(LlvmSection::CoverageMap))
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
            let name_hash = endian.read_u64_bytes(bytes[0..8].try_into().unwrap());
            let data_len = endian.read_u32_bytes(bytes[8..12].try_into().unwrap());
            let func_hash = endian.read_u64_bytes(bytes[12..20].try_into().unwrap());
            let filenames_ref = endian.read_u64_bytes(bytes[20..28].try_into().unwrap());
            let header = FunctionRecordHeader {
                name_hash,
                data_len,
                func_hash,
                filenames_ref,
            };
            let start_len = bytes[28..].len();
            bytes = &bytes[28..];

            let (data, id_len) = parse_leb128(bytes).unwrap();
            bytes = data;
            let mut filename_indices = vec![];
            for _ in 0..id_len {
                let (data, id) = parse_leb128(bytes).unwrap(); // Issue
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

            let (data, regions) = parse_mapping_regions(bytes, &filename_indices).unwrap();

            if func_hash != 0 {
                res.push(FunctionRecordV3 { header, regions });
            }

            bytes = data;
            let function_len = section_len - bytes.len(); // this should match header

            let padding = if function_len < section_len && (function_len & 0x07) != 0 {
                8 - (function_len & 0x07)
            } else {
                0
            };

            if padding > bytes.len() {
                break;
            }
            // Now apply padding, and if hash is 0 move on as it's a dummy otherwise add to result
            // And decide what end type will be
            bytes = &bytes[padding..];
        }
        Ok(res)
    } else {
        Err(SectionReadError::EmptySection(
            LlvmSection::CoverageFunctions,
        ))
    }
}

fn parse_mapping_regions<'a>(
    mut bytes: &'a [u8],
    file_indices: &[u64],
) -> IResult<&'a [u8], Vec<CounterMappingRegion>> {
    let mut mapping = vec![];
    for i in file_indices {
        let (data, regions_len) = parse_leb128(bytes)?;
        bytes = data;
        let mut last_line = 0;
        for _ in 0..regions_len {
            let mut kind = RegionKind::Code;
            let (data, raw_header) = parse_leb128(bytes)?;
            let (data, delta_line) = parse_leb128(data)?;
            let (data, column_start) = parse_leb128(data)?;
            let (data, lines_len) = parse_leb128(data)?;
            let (data, column_end) = parse_leb128(data)?;
            bytes = data;
            let mut expanded_file_id = 0;
            let counter = parse_counter(raw_header);
            if counter.kind == CounterType::Zero {
                if raw_header & Counter::ENCODING_EXPANSION_REGION_BIT > 0 {
                    kind = RegionKind::Expansion;
                    expanded_file_id = raw_header >> Counter::ENCODING_TAG_AND_EXP_REGION_BITS;
                    if expanded_file_id >= file_indices.len() as u64 {
                        todo!()
                    }
                } else {
                    let shifted_counter = raw_header >> Counter::ENCODING_TAG_AND_EXP_REGION_BITS;
                    match shifted_counter.try_into() {
                        Ok(RegionKind::Code) | Ok(RegionKind::Skipped) => {}
                        e => panic!("Malformed: {:?}", e),
                    }
                }
            }

            let (column_start, column_end) = if column_start == 0 && column_end == 0 {
                (1usize, usize::MAX)
            } else {
                (column_start as usize, column_end as usize)
            };

            let line_start = last_line + delta_line as usize;
            let line_end = line_start + lines_len as usize;
            last_line = line_start;

            // Add region working-out-stuff
            mapping.push(CounterMappingRegion {
                kind,
                count: counter,
                file_id: *i as usize,
                expanded_file_id: expanded_file_id as _,
                line_start,
                line_end,
                column_start,
                column_end,
            });
        }
    }
    Ok((bytes, mapping))
}

fn parse_profile_data<'data, 'file>(
    endian: Endianness,
    section: &Section<'data, 'file>,
) -> Result<Vec<ProfileData>, SectionReadError> {
    if let Ok(data) = section.data() {
        let mut bytes = &data[..];
        let mut res = vec![];
        while !bytes.is_empty() {
            let name_md5 = endian.read_u64_bytes(bytes[..8].try_into().unwrap());
            let structural_hash = endian.read_u64_bytes(bytes[8..16].try_into().unwrap());

            let counter_ptr = endian.read_u64_bytes(bytes[16..24].try_into().unwrap());
            bytes = &bytes[(24 + 16)..];
            let counters_len = endian.read_u32_bytes(bytes[..4].try_into().unwrap());
            // TODO Might need to get the counter offset and get the list of counters from this?
            // And potentially check against the maximum number of counters just to make sure that
            // it's not being exceeded?
            //
            // Also counters_len >= 1 so this should be checked to make sure it's not malformed

            bytes = &bytes[8..];

            res.push(ProfileData {
                name_md5,
                structural_hash,
                counters_len,
            });
        }
        Ok(res)
    } else {
        Err(SectionReadError::EmptySection(LlvmSection::ProfileData))
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
        Err(SectionReadError::EmptySection(LlvmSection::ProfileCounts))
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
        Err(SectionReadError::EmptySection(LlvmSection::ProfileNames))
    }
}
