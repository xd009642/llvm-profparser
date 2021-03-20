use nom::{number::complete::*, IResult};
use std::borrow::Cow;

struct KeyDataLen {
    key_len: u64,
    data_len: u64,
}

pub(crate) struct HashTable {}

fn read_key_data_len(input: &[u8]) -> IResult<&[u8], KeyDataLen> {
    let (bytes, key_len) = le_u64(input)?;
    let (bytes, data_len) = le_u64(bytes)?;
    let res = KeyDataLen { key_len, data_len };
    Ok((bytes, res))
}

fn read_key(input: &[u8], key_len: usize) -> IResult<&[u8], Cow<'_, str>> {
    let res = String::from_utf8_lossy(&input[..key_len]);
    Ok((&input[key_len..], res))
}

impl HashTable {
    /// buckets is the data the hash table buckets start at - the start of the `HashTable` in memory.
    /// hash. offset shows the offset from the base address to the start of the `HashTable` as this
    /// will be used to correct any offsets
    pub(crate) fn parse(input: &[u8], offset: usize, bucket_start: usize) -> IResult<&[u8], Self> {
        assert!(bucket_start > 0);
        println!("Table starts at: {}", offset);
        println!(
            "Buckets start at {} or {}",
            bucket_start,
            offset + bucket_start
        );
        let (bytes, num_buckets) = le_u64(&input[bucket_start..])?;
        let (bytes, num_entries) = le_u64(bytes)?;
        todo!()
    }
}
