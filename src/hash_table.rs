use nom::{number::complete::*, IResult};

pub(crate) struct HashTable {}

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
