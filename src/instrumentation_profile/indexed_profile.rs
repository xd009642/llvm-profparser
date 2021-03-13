use crate::instrumentation_profile::*;
use nom::IResult;
use std::io::Read;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct IndexedInstrProf;

impl InstrProfReader for IndexedInstrProf {
    type Header = Header;

    fn parse_bytes(_input: &[u8]) -> IResult<&[u8], InstrumentationProfile> {
        todo!()
    }

    fn parse_header(_input: &[u8]) -> IResult<&[u8], Self::Header> {
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
