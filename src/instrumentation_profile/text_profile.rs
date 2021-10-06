use crate::instrumentation_profile::types::*;
use crate::instrumentation_profile::InstrProfReader;
use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case, take_until};
use nom::character::is_digit;
use nom::error::{Error, ErrorKind};
use nom::multi::*;
use nom::sequence::*;
use nom::*;
use std::io::Read;

const IR_TAG: &[u8] = b"ir";
const FE_TAG: &[u8] = b"fe";
const CSIR_TAG: &[u8] = b"csir";
const ENTRY_TAG: &[u8] = b"entry_first";
const NOT_ENTRY_TAG: &[u8] = b"not_entry_first";
const EXTERNAL_SYMBOL: &[u8] = b"** External Symbol **";

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TextInstrProf;

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Header {
    is_ir_level: bool,
    has_csir: bool,
    entry_first: bool,
}

fn check_tag(data: &[u8], tag: &[u8]) -> bool {
    if let Ok(data) = std::str::from_utf8(data) {
        let tag = std::str::from_utf8(tag).unwrap_or_default();
        data == tag || data == tag.to_uppercase()
    } else {
        false
    }
}

fn str_to_digit(bytes: &[u8]) -> u64 {
    // As I'm only using this on lines nom identifies as just digits it won't fail
    std::str::from_utf8(bytes)
        .unwrap()
        .parse()
        .unwrap_or_default()
}

fn valid_name_char(character: u8) -> bool {
    let c = character as char;
    // Whitespace is allowed!
    c.is_ascii() && c != '\n' && c != '\r'
}

named!(strip_whitespace<&[u8], ()>, map!(one_of!(&b" \n\r\t"[..]), |_|()));

named!(strip_comments<&[u8], ()>,
    map!(delimited!(tag!(b"#"), take_until!("\n"), tag!("\n")), |_|())
);

named!(skip_to_content<&[u8], ()>, map!(many0!(alt!(strip_whitespace | strip_comments)), |_|()));

fn match_header_tags(s: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        tag_no_case(IR_TAG),
        tag_no_case(FE_TAG),
        tag_no_case(CSIR_TAG),
        tag_no_case(ENTRY_TAG),
        take_until("\n"),
    ))(s)
}

fn parse_header_tags(s: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
    many0(delimited(tag(b":"), match_header_tags, tag("\n")))(s)
}

named!(
    read_line,
    map!(tuple!(take_while1!(valid_name_char), tag!(b"\n")), |x| x.0)
);

named!(read_digit<&[u8], u64>, map!(tuple!(take_while1!(is_digit), tag!(b"\n")), |x| str_to_digit(x.0)));

named!(
    indirect_value_site<&[u8], (&[u8], u64)>,
    map!(tuple!(take_until!(":"), tag!(":"), take_while1!(is_digit)), |x| (x.0, str_to_digit(x.2)))
);

named!(
    memop_value_site<&[u8], (u64, u64)>,
    map!(tuple!(take_while1!(is_digit), tag!(":"), take_while1!(is_digit)), |x| (str_to_digit(x.0), str_to_digit(x.2)))
);

fn read_value_profile_data(mut input: &[u8]) -> IResult<&[u8], Option<Box<ValueProfDataRecord>>> {
    if let Ok((bytes, n_kinds)) = read_digit(input) {
        let mut record = Box::new(ValueProfDataRecord::default());
        // We have value profiling data!
        if n_kinds == 0 || n_kinds > ValueKind::len() as u64 {
            // TODO I am malformed
            todo!()
        }
        input = bytes;
        for _i in 0..n_kinds {
            let (bytes, _) = skip_to_content(input)?;
            let (bytes, kind) = read_digit(bytes)?;
            let (bytes, _) = skip_to_content(bytes)?;
            let (bytes, n_sites) = match read_digit(bytes) {
                Ok(s) => s,
                Err(_) => {
                    input = bytes;
                    continue;
                }
            };
            // TODO is there a tidier way to go from discriminant to enum
            let kind = match kind {
                0 => ValueKind::IndirectCallTarget,
                1 => ValueKind::MemOpSize,
                _ => todo!(),
            };
            // let mut sites = vec![];
            input = bytes;
            for _j in 0..n_sites {
                let (bytes, _) = skip_to_content(input)?;
                let (bytes, n_val_data) = read_digit(bytes)?;
                input = bytes;
                let mut site_records = vec![];
                for _k in 0..n_val_data {
                    let (bytes, _) = skip_to_content(input)?;
                    input = match kind {
                        ValueKind::IndirectCallTarget => {
                            let (bytes, (sym, count)) = indirect_value_site(bytes)?;
                            let value = if sym == EXTERNAL_SYMBOL {
                                0
                            } else {
                                compute_hash(sym)
                            };
                            site_records.push(InstrProfValueData { value, count });
                            bytes
                        }
                        ValueKind::MemOpSize => {
                            let (bytes, (value, count)) = memop_value_site(bytes)?;
                            site_records.push(InstrProfValueData { value, count });
                            bytes
                        }
                    };
                }
                match kind {
                    ValueKind::IndirectCallTarget => record.indirect_callsites.push(site_records),
                    ValueKind::MemOpSize => record.mem_op_sizes.push(site_records),
                }
            }
        }
        Ok((input, Some(record)))
    } else {
        Ok((input, None))
    }
}

impl InstrProfReader for TextInstrProf {
    type Header = Header;
    fn parse_bytes(mut input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input)?;
        let (bytes, _) = skip_to_content(bytes)?;
        input = bytes;
        let mut result = InstrumentationProfile {
            has_csir: header.has_csir,
            is_ir: header.is_ir_level,
            is_entry_first: header.entry_first,
            ..Default::default()
        };
        while !input.is_empty() {
            // function name (demangled)
            let (bytes, name) = read_line(input)?;
            let (bytes, _) = skip_to_content(bytes)?;
            // function hash
            let (bytes, hash) = read_digit(bytes)?;
            let (bytes, _) = skip_to_content(bytes)?;
            // number of counters
            let (bytes, num_counters) = read_digit(bytes)?;
            let (bytes, _) = skip_to_content(bytes)?;
            let mut counters = vec![];
            // counter values
            input = bytes;
            for i in 0..num_counters {
                let (bytes, counter) = read_digit(input)?;
                counters.push(counter);
                match skip_to_content(bytes) {
                    Ok((bytes, _)) => {
                        input = bytes;
                    }
                    Err(_) if i + 1 == num_counters => {
                        input = &bytes[(bytes.len())..];
                        break;
                    }
                    Err(e) => {
                        Err(e)?;
                    }
                }
            }
            let (bytes, data) = read_value_profile_data(input)?;
            let record = InstrProfRecord {
                counts: counters,
                data,
            };
            let name = std::str::from_utf8(name).map(|x| x.to_string()).ok();
            result.records.push(NamedInstrProfRecord {
                name: name.clone(),
                hash: Some(hash),
                record,
            });
            if let Some(name) = name {
                result.symtab.names.insert(hash, name);
            }
            input = match skip_to_content(bytes) {
                Ok((bytes, _)) => bytes,
                Err(_) => &bytes[(bytes.len())..],
            };
        }
        Ok((bytes, result))
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Self::Header> {
        let (input, _) = skip_to_content(input)?;
        let (bytes, names) = parse_header_tags(input)?;
        let mut is_ir_level = false;
        let mut has_csir = false;
        let mut entry_first = false;
        for name in &names {
            if check_tag(name, IR_TAG) | check_tag(name, NOT_ENTRY_TAG) {
                is_ir_level = true;
            } else if check_tag(name, CSIR_TAG) {
                has_csir = true;
                is_ir_level = true;
            } else if check_tag(name, ENTRY_TAG) {
                entry_first = true;
            } else if !check_tag(name, FE_TAG) {
                return Err(Err::Failure(Error::new(bytes, ErrorKind::Tag)));
            }
        }
        Ok((
            bytes,
            Header {
                is_ir_level,
                has_csir,
                entry_first,
            },
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let csir_header = b"# CSIR flag\n:csir\n";
        let (_, header) = TextInstrProf::parse_header(&csir_header[..]).unwrap();
        assert!(header.is_ir_level);
        assert!(header.has_csir);

        let csir_header = b"# CSIR flag\n:CSIR\n";
        let (_, header) = TextInstrProf::parse_header(&csir_header[..]).unwrap();
        assert!(header.is_ir_level);
        assert!(header.has_csir);

        let ir_header = b"# IR flag\n\n:ir\n";
        let (_, header) = TextInstrProf::parse_header(&ir_header[..]).unwrap();
        assert!(header.is_ir_level);
        assert!(!header.has_csir);

        let ir_header = b"# IR flag\n\n:IR\n";
        let (_, header) = TextInstrProf::parse_header(&ir_header[..]).unwrap();
        assert!(header.is_ir_level);
        assert!(!header.has_csir);

        let fe_header = b"# FE flag\n\n:fe\n";
        let (_, header) = TextInstrProf::parse_header(&fe_header[..]).unwrap();
        assert!(!header.is_ir_level);
        assert!(!header.has_csir);

        let fe_header = b"# FE flag\n\n:FE\n";
        let (_, header) = TextInstrProf::parse_header(&fe_header[..]).unwrap();
        assert!(!header.is_ir_level);
        assert!(!header.has_csir);

        let no_header = b"# Straight to funcs\nfoobar";
        let (_, header) = TextInstrProf::parse_header(&no_header[..]).unwrap();
        assert!(!header.is_ir_level);
        assert!(!header.has_csir);
    }

    #[test]
    fn parse_multiline_header() {
        let header = b":entry_first\n:ir\n#content";
        let (_, header) = TextInstrProf::parse_header(&header[..]).unwrap();
        assert!(header.is_ir_level);
        assert!(header.entry_first);
    }

    #[test]
    fn invalid_header() {
        let bad_header = b"# CSIR flag\n:\n";
        let header = TextInstrProf::parse_header(&bad_header[..]);
        assert!(header.is_err(), "Valid header: {:?}", header);
        let bad_header = b"# CSIR flag\n:CSI\n";
        let header = TextInstrProf::parse_header(&bad_header[..]);
        assert!(header.is_err(), "Valid header: {:?}", header);
    }

    #[test]
    fn content_strip() {
        let empty = b"\n";
        let (bytes, _) = strip_whitespace(empty).unwrap();
        assert_eq!(bytes.len(), 0);

        let comment = b"# I am a comment\n";
        let (bytes, _) = strip_comments(comment).unwrap();
        assert_eq!(bytes.len(), 0);
    }
}
