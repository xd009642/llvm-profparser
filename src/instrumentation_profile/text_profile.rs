use crate::instrumentation_profile::types::*;
use crate::instrumentation_profile::InstrProfReader;
use nom::error::{Error, ErrorKind};
use nom::*;
use std::io::Read;

const IR_TAG: &[u8] = b"ir";
const FE_TAG: &[u8] = b"fe";
const CSIR_TAG: &[u8] = b"csir";

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TextInstrProf;

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Header {
    is_ir_level: bool,
    has_csir: bool,
}

fn check_tag(data: &[u8], tag: &[u8]) -> bool {
    if let Ok(data) = std::str::from_utf8(data) {
        let tag = std::str::from_utf8(tag).unwrap_or_default();
        data == tag || data == tag.to_uppercase()
    } else {
        false
    }
}

named!(strip_whitespace<&[u8], ()>, map!(one_of!(&b" \n\r\t"[..]), |_|()));

named!(strip_comments<&[u8], ()>,
    map!(delimited!(tag!(b"#"), take_until!("\n"), tag!("\n")), |_|())
);

named!(skip_to_content<&[u8], ()>, map!(many0!(alt!(strip_whitespace| strip_comments)), |_|()));

named!(
    parse_header<&[u8], Option<&[u8]>>,
    opt!(delimited!(
        tag!(b":"),
        alt!(tag_no_case!(IR_TAG) | tag_no_case!(FE_TAG) | tag_no_case!(CSIR_TAG)| take_until!("\n")),
        tag!(b"\n")
    ))
);

impl InstrProfReader for TextInstrProf {
    type Header = Header;
    fn parse_bytes(mut input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        let (bytes, header) = Self::parse_header(input)?;
        let (bytes, _) = skip_to_content(bytes)?;
        input = bytes;
        println!("Got header: {:?}", header);
        while !input.is_empty() {
            // function name
            // function hash
            // number of counters
            // counter values
        }
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Self::Header> {
        let (input, _) = skip_to_content(input)?;
        let (bytes, name) = parse_header(input)?;
        let (is_ir_level, has_csir) = match name {
            Some(name) => {
                if check_tag(name, IR_TAG) {
                    (true, false)
                } else if check_tag(name, FE_TAG) {
                    (false, false)
                } else if check_tag(name, CSIR_TAG) {
                    (true, true)
                } else {
                    return Err(Err::Failure(Error::new(bytes, ErrorKind::Tag)));
                }
            }
            None => (false, false),
        };
        Ok((
            bytes,
            Header {
                is_ir_level,
                has_csir,
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
        println!("Header: {:?}", header);
        assert!(header.is_ir_level);
        assert!(header.has_csir);

        let csir_header = b"# CSIR flag\n:CSIR\n";
        let (_, header) = TextInstrProf::parse_header(&csir_header[..]).unwrap();
        println!("Header: {:?}", header);
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
    fn invalid_header() {
        let bad_header = b"# CSIR flag\n:\n";
        let header = TextInstrProf::parse_header(&bad_header[..]);
        assert!(header.is_err(), "Valid header: {:?}", header);
        let bad_header = b"# CSIR flag\n:CSI\n";
        let header = TextInstrProf::parse_header(&bad_header[..]);
        assert!(header.is_err(), "Valid header: {:?}", header);
    }
}
