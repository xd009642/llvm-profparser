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

/// So what the LLVM one has that this one doesn't yet:
/// 1. DenseMap<size_t, DenseSet<size_t>> RecordProvenance
/// 2. std::vector<FunctionRecord> functions (this is probably taken straight from
/// InstrumentationProfile
/// 3. DenseMap<size_t, SmallVector<unsigned, 0>> FilenameHash2RecordIndices
/// 4. Vec<Pair<String, u64>> FuncHashMismatches
#[derive(Debug)]
pub struct CoverageMapping<'a> {
    profile: &'a InstrumentationProfile,
    pub mapping_info: Vec<CoverageMappingInfo>,
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
        let mut mapping_info = vec![];
        println!("profile:\n{:#?}", profile);
        for file in object_files {
            mapping_info.push(read_object_file(file.as_path())?);
        }
        println!("Mappings:\n{:#?}", mapping_info);
        Ok(Self {
            profile,
            mapping_info,
        })
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
            let fn_hash = endian.read_u64_bytes(bytes[12..20].try_into().unwrap());
            let filenames_ref = endian.read_u64_bytes(bytes[20..28].try_into().unwrap());
            let header = FunctionRecordHeader {
                name_hash,
                data_len,
                fn_hash,
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
            let expr_len = expr_len as usize;
            bytes = data;
            let mut exprs = vec![Expression::default(); expr_len];
            for i in 0..expr_len {
                let (data, lhs) = parse_leb128(bytes).unwrap();
                let (data, rhs) = parse_leb128(data).unwrap();
                let lhs = parse_counter(lhs, &mut exprs);
                let rhs = parse_counter(rhs, &mut exprs);
                exprs[i].lhs = lhs;
                exprs[i].rhs = rhs;
                bytes = data;
            }

            let (data, regions) =
                parse_mapping_regions(bytes, &filename_indices, &mut exprs).unwrap();

            if fn_hash != 0 {
                res.push(FunctionRecordV3 {
                    header,
                    regions,
                    expressions: exprs,
                });
            }

            // Todo set couners for expansion regions - counter of expansion region is the counter
            // of the first region from the expanded file. This requires multiple passes to
            // correctly propagate across all nested regions. N.B. I haven't seen any expansion
            // regions in use so may not be an issue!

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

/// This code is ported from `RawCoverageMappingReader::readMappingRegionsSubArray`
fn parse_mapping_regions<'a>(
    mut bytes: &'a [u8],
    file_indices: &[u64],
    expressions: &mut Vec<Expression>,
) -> IResult<&'a [u8], Vec<CounterMappingRegion>> {
    let mut mapping = vec![];
    for i in file_indices {
        let (data, regions_len) = parse_leb128(bytes)?;
        bytes = data;
        let mut last_line = 0;
        for _ in 0..regions_len {
            let mut false_count = Counter::default();
            let mut kind = RegionKind::Code;
            let (data, raw_header) = parse_leb128(bytes)?;
            bytes = data;
            let mut expanded_file_id = 0;
            let mut counter = parse_counter(raw_header, expressions);
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
                        Ok(RegionKind::Branch) => {
                            kind = RegionKind::Branch;
                            let (data, c1) = parse_leb128(bytes)?;
                            let (data, c2) = parse_leb128(bytes)?;

                            counter = parse_counter(c1, expressions);
                            false_count = parse_counter(c2, expressions);
                            bytes = data;
                        }
                        e => panic!("Malformed: {:?}", e),
                    }
                }
            }

            let (data, delta_line) = parse_leb128(bytes)?;
            let (data, column_start) = parse_leb128(data)?;
            let (data, lines_len) = parse_leb128(data)?;
            let (data, column_end) = parse_leb128(data)?;
            bytes = data;

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
                false_count,
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

/// The equivalent llvm function is `RawCoverageMappingReader::decodeCounter`. This makes it
/// stateless as I don't want to be maintaining an expression vector and clearing it and
/// repopulating for every function record.
fn parse_counter(input: u64, exprs: &mut Vec<Expression>) -> Counter {
    let ty = (Counter::ENCODING_TAG_MASK & input) as u8;
    let id = input >> 2; // For zero we don't actually care about this but we'll still do it
    let kind = match ty {
        0 => CounterType::Zero,
        1 => CounterType::ProfileInstrumentation,
        2 | 3 => {
            let expr_kind = if ty == 2 {
                ExprKind::Subtract
            } else {
                ExprKind::Add
            };
            exprs[id as usize].set_kind(expr_kind);
            CounterType::Expression(expr_kind)
        }
        _ => unreachable!(),
    };
    Counter { kind, id }
}
