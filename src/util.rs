use flate2::read::ZlibDecoder;
use nom::{error::Error, IResult};
use std::io::Read;
use std::path::{Path, PathBuf};

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

/// Parses a list of paths - this is currently only used in parsing the sections in an instrumented
/// object file, and due to CWD joining is different to the other string parsing implemented
pub fn parse_path_list(input: &[u8], version: u64) -> IResult<&[u8], Vec<PathBuf>> {
    let (input, list_length) = parse_leb128(input)?;

    if version < 3 {
        // read_uncompressed
        let (input, values) = parse_uncompressed_file_list(input, list_length, version)?;
        Ok((input, values))
    } else {
        let (input, uncompressed_size) = parse_leb128(input)?;
        let (input, compressed_size) = parse_leb128(input)?;

        let compressed_size = compressed_size as usize;
        let uncompressed_size = uncompressed_size as usize;

        if compressed_size == 0 {
            let (input, values) = parse_uncompressed_file_list(input, list_length, version)?;
            Ok((input, values))
        } else {
            let mut decoder = ZlibDecoder::new(&input[..compressed_size]);
            let mut output = vec![];
            output.reserve(uncompressed_size);
            decoder.read_to_end(&mut output).unwrap();
            let (compressed_input, values) = match parse_uncompressed_string_list(&output[..]) {
                Ok((i, v)) => (i, v.iter().map(PathBuf::from).collect()),
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
            Ok((&input[compressed_size..], values))
        }
    }
}

fn read_string(bytes: &[u8]) -> IResult<&[u8], String> {
    let (bytes, len) = parse_leb128(bytes)?;
    let len = len as usize;
    let string = String::from_utf8_lossy(&bytes[..len]).to_string();
    Ok((&bytes[len..], string))
}

fn parse_uncompressed_file_list(
    mut input: &[u8],
    list_length: u64,
    version: u64,
) -> IResult<&[u8], Vec<PathBuf>> {
    let mut res = vec![];
    if version < 5 {
        for _ in 0..list_length {
            let (bytes, string) = read_string(input)?;
            res.push(PathBuf::from(string));
            input = bytes;
        }
        Ok((input, res))
    } else {
        let (bytes, cwd) = read_string(input)?;
        let cwd = Path::new(&cwd);
        res.push(cwd.to_path_buf());
        input = bytes;
        for _ in 1..list_length {
            let (bytes, path) = read_string(input)?;
            input = bytes;
            let tmp = Path::new(&path);
            if tmp.is_absolute() {
                res.push(tmp.to_path_buf());
            } else {
                // NB here the llvm code sometimes has the compilation dir available and joins
                // paths onto that instead.
                res.push(cwd.join(tmp));
            }
        }
        Ok((input, res))
    }
}

fn parse_uncompressed_string_list(mut input: &[u8]) -> IResult<&[u8], Vec<String>> {
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
