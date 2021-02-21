use flate2::read::ZlibDecoder;
use nom::IResult;
use std::io::Read;

pub fn parse_leb128(mut input: &[u8]) -> IResult<&[u8], u64> {
    let x = leb128::read::unsigned(&mut input).unwrap();
    Ok((input, x))
}

pub fn parse_string_ref(input: &[u8]) -> IResult<&[u8], String> {
    let (input, uncompressed_size) = parse_leb128(input)?;
    let (input, compressed_size) = parse_leb128(input)?;
    if compressed_size != 0 {
        let compressed_size = compressed_size as usize;
        let mut decoder = ZlibDecoder::new(&input[..compressed_size]);
        let mut output = vec![];
        let contents = decoder.read_to_end(&mut output).unwrap();
        let name = String::from_utf8(output);
        let name = name.unwrap();
        Ok((&input[compressed_size..], name))
    } else {
        let uncompressed_size = uncompressed_size as usize;
        let name = String::from_utf8(input[..uncompressed_size].to_vec()).unwrap();
        Ok((&input[uncompressed_size..], name))
    }
}
