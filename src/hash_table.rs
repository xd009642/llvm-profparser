use crate::instrumentation_profile::types::*;
use nom::{number::complete::*, take, IResult};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug)]
struct KeyDataLen {
    key_len: u64,
    data_len: u64,
}

impl KeyDataLen {
    fn len(&self) -> usize {
        (self.key_len + self.data_len) as usize
    }
}

pub(crate) struct HashTable(pub HashMap<String, Vec<InstrProfRecord>>);

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

fn read_value(input: &[u8], data_len: usize) -> IResult<&[u8], Vec<InstrProfRecord>> {
    todo!();
}

impl HashTable {
    fn new() -> Self {
        Self(HashMap::new())
    }

    /// buckets is the data the hash table buckets start at - the start of the `HashTable` in memory.
    /// hash. offset shows the offset from the base address to the start of the `HashTable` as this
    /// will be used to correct any offsets
    pub(crate) fn parse(input: &[u8], offset: usize, bucket_start: usize) -> IResult<&[u8], Self> {
        assert!(bucket_start > 0);
        let (bytes, num_buckets) = le_u64(&input[bucket_start..])?;
        let (bytes, mut num_entries) = le_u64(bytes)?;
        let mut payload = input;
        let mut result = Self::new();
        while num_entries > 0 {
            let (bytes, entries) = result.parse_bucket(payload, num_entries)?;
            payload = bytes;
            num_entries = entries;
        }

        Ok((payload, result))
    }

    fn parse_bucket<'a>(
        &mut self,
        input: &'a [u8],
        mut num_entries: u64,
    ) -> IResult<&'a [u8], u64> {
        let (bytes, num_items_in_bucket) = le_u16(input)?;
        let mut remaining = bytes;
        for i in 0..num_items_in_bucket {
            let (bytes, hash) = le_u64(remaining)?;
            let (bytes, lens) = read_key_data_len(bytes)?;
            let (bytes, key) = read_key(bytes, lens.key_len as usize)?;
            let (bytes, value) = read_value(bytes, lens.data_len as usize)?;
            self.0.insert(key.to_string(), value);
            remaining = bytes;
            assert!(num_entries > 0);
            num_entries -= 1;
        }
        Ok((remaining, num_entries))
    }
}
