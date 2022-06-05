use flate2::read::ZlibDecoder;
use nom::{error::Error, IResult};
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
        decoder.read_to_end(&mut output).unwrap();
        let name = String::from_utf8(output);
        let name = name.unwrap();
        Ok((&input[compressed_size..], name))
    } else {
        let uncompressed_size = uncompressed_size as usize;
        let name = String::from_utf8(input[..uncompressed_size].to_vec()).unwrap();
        Ok((&input[uncompressed_size..], name))
    }
}

pub fn parse_string_list(input: &[u8]) -> IResult<&[u8], Vec<String>> {
    let (input, list_length) = parse_leb128(input)?;
    let (input, uncompressed_size) = parse_leb128(input)?;
    let (input, compressed_size) = parse_leb128(input)?;

    let compressed_size = compressed_size as usize;
    let uncompressed_size = uncompressed_size as usize;

    if compressed_size == 0 {
        let (input, values) = parse_uncompressed_list(input)?;
        if values.len() != list_length as usize {
            println!(
                "CGot {} instead of {} names: {:?}",
                values.len(),
                list_length,
                values
            );
        }
        Ok((input, values))
    } else {
        let mut decoder = ZlibDecoder::new(&input[..compressed_size]);
        let mut output = vec![];
        output.reserve(uncompressed_size);
        decoder.read_to_end(&mut output).unwrap();
        let (compressed_input, values) = match parse_uncompressed_list(&output[..]) {
            Ok((i, v)) => (i, v),
            Err(e) => {
                // substitute the decompressed slice start as the error location
                let e = match e {
                    nom::Err::Error(e) => nom::Err::Error(Error {
                        input,
                        code: e.code,
                    }),
                    nom::Err::Failure(e) => nom::Err::Failure(Error {
                        input,
                        code: e.code,
                    }),
                    nom::Err::Incomplete(n) => nom::Err::Incomplete(n),
                };
                return Err(e);
            }
        };
        if values.len() != list_length as usize {
            println!(
                "UCGot {} names instead of {}: {:?}",
                values.len(),
                list_length,
                values
            );
        }
        Ok((&input[compressed_size..], values))
    }
}

fn parse_uncompressed_list(mut input: &[u8]) -> IResult<&[u8], Vec<String>> {
    let mut res = vec![];
    while !input.is_empty() {
        let (bytes, len) = parse_leb128(input)?;
        let len = len as usize;
        let string = String::from_utf8_lossy(&bytes[..len]).to_string();
        res.push(string);

        input = &bytes[len..];
    }
    Ok((input, res))
}
