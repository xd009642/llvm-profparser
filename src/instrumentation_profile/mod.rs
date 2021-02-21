use crate::instrumentation_profile::raw_profile::*;
use nom::IResult;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

pub mod raw_profile;
pub mod symtab;

pub const fn get_num_padding_bytes(len: u64) -> u8 {
    7 & (8 - (len % 8) as u8)
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrumentationProfile;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Header {
    version: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct IndexedInstrProf;
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TextInstrProf;

pub fn parse(filename: impl AsRef<Path>) -> io::Result<InstrumentationProfile> {
    let mut buffer = Vec::new();
    let mut f = File::open(filename)?;
    f.read_to_end(&mut buffer)?;
    let nom_res = if IndexedInstrProf::has_format(buffer.as_slice()) {
        IndexedInstrProf::parse_bytes(&buffer)
    } else if RawInstrProf64::has_format(buffer.as_slice()) {
        RawInstrProf64::parse_bytes(&buffer)
    } else if RawInstrProf32::has_format(buffer.as_slice()) {
        RawInstrProf32::parse_bytes(&buffer)
    } else if TextInstrProf::has_format(buffer.as_slice()) {
        TextInstrProf::parse_bytes(&buffer)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Unsupported instrumentation profile format",
        ));
    };
    nom_res.map(|(_bytes, res)| res).map_err(|e| {
        println!("Parsing failed: {}", e);
        io::Error::new(io::ErrorKind::Other, "Parsing failed, don't ask me")
    })
}

pub trait InstrProfReader {
    type Header;
    /// Parse the profile no lazy parsing here!
    fn parse_bytes(input: &[u8]) -> IResult<&[u8], InstrumentationProfile>;
    /// Parses a header
    fn parse_header(input: &[u8]) -> IResult<&[u8], Self::Header>;
    /// Detects that the bytes match the current reader format if it can't read the format it will
    /// return false
    fn has_format(input: impl Read) -> bool;
}

impl InstrProfReader for IndexedInstrProf {
    type Header = Header;

    fn parse_bytes(input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Self::Header> {
        todo!()
    }

    fn has_format(mut input: impl Read) -> bool {
        const MAGIC: [u8; 8] = [0xff, 0x6c, 0x70, 0x72, 0x6f, 0x66, 0x69, 0x81];
        let mut buffer: [u8; 8] = [0; 8];
        if input.read_exact(&mut buffer).is_ok() {
            buffer == MAGIC
        } else {
            false
        }
    }
}

impl InstrProfReader for TextInstrProf {
    type Header = Header;
    fn parse_bytes(input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        todo!()
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Self::Header> {
        todo!()
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
