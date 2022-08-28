use nom::number::Endianness;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt;

/// ~VARIANT_MASKS_ALL & Header.version is the version number
pub(crate) const VARIANT_MASKS_ALL: u64 = 0xff00_0000_0000_0000;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
pub(crate) const VARIANT_MASK_IR_PROF: u64 = 1u64 << 56;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
pub(crate) const VARIANT_MASK_CSIR_PROF: u64 = 1u64 << 57;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
pub(crate) const VARIANT_MASK_BYTE_COVERAGE: u64 = 1u64 << 60;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
pub(crate) const VARIANT_MASK_FUNCTION_ENTRY_ONLY: u64 = 1u64 << 61;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
pub(crate) const VARIANT_MASK_MEMORY_PROFILE: u64 = 1u64 << 62;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum ValueKind {
    IndirectCallTarget = 0,
    MemOpSize = 1,
}

impl ValueKind {
    pub const fn len() -> usize {
        2
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Symtab {
    pub names: BTreeMap<u64, String>,
}

pub fn compute_hash(data: impl AsRef<[u8]>) -> u64 {
    let hash = md5::compute(data).0[..8].try_into().unwrap_or_default();
    u64::from_le_bytes(hash)
}

fn compute_be_hash(data: impl AsRef<[u8]>) -> u64 {
    let hash = md5::compute(data).0[..8].try_into().unwrap_or_default();
    u64::from_be_bytes(hash)
}

impl Symtab {
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Some formats such as the Raw profiles have configurable endianness. I think these may
    /// require a matching endian hash. However, this doesn't seem to be represented in any of the
    /// llvm test files so is largely a mystery. Computes a little endian hash unless specified
    /// otherwise.
    pub fn add_func_name(&mut self, name: String, endianness: Option<Endianness>) {
        let hash = match endianness {
            Some(Endianness::Little) => compute_hash(&name),
            Some(Endianness::Big) => compute_be_hash(&name),
            _ => compute_hash(&name),
        };
        self.names.insert(hash, name);
    }

    pub fn contains(&self, hash: u64) -> bool {
        self.names.contains_key(&hash)
    }

    pub fn get(&self, hash: u64) -> Option<&String> {
        self.names.get(&hash)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &String)> {
        self.names.iter()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum InstrumentationLevel {
    FrontEnd,
    Ir,
}

impl fmt::Display for InstrumentationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrontEnd => write!(f, "Front-end"),
            Self::Ir => write!(f, "IR"),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrumentationProfile {
    pub(crate) version: Option<u64>,
    pub(crate) has_csir: bool,
    pub(crate) is_ir: bool,
    pub(crate) is_entry_first: bool,
    pub(crate) is_byte_coverage: bool,
    pub(crate) fn_entry_only: bool,
    pub(crate) memory_profiling: bool,
    pub records: Vec<NamedInstrProfRecord>,
    pub symtab: Symtab,
}

impl InstrumentationProfile {
    pub fn version(&self) -> Option<u64> {
        self.version
    }

    pub fn version_unchecked(&self) -> u64 {
        *self.version.as_ref().unwrap()
    }

    pub fn is_ir_level_profile(&self) -> bool {
        self.is_ir
    }

    pub fn has_csir_level_profile(&self) -> bool {
        self.has_csir
    }

    pub fn is_entry_first(&self) -> bool {
        self.is_entry_first
    }

    pub fn has_memory_profile(&self) -> bool {
        self.memory_profiling
    }

    pub fn get_level(&self) -> InstrumentationLevel {
        if self.is_ir_level_profile() {
            InstrumentationLevel::Ir
        } else {
            InstrumentationLevel::FrontEnd
        }
    }

    /// Byte coverage switches things around to make `0` equivalent to coverage and !0 uncovered it
    /// seems. This currently is not supported but also not output by any rust tools (to my
    /// knowledge)
    pub fn is_byte_coverage(&self) -> bool {
        self.is_byte_coverage
    }

    pub fn fn_entry_only(&self) -> bool {
        self.fn_entry_only
    }

    pub fn merge(&mut self, other: &Self) {
        for func in &other.records {
            self.merge_record(&func);
        }
    }

    pub fn merge_record(&mut self, record: &NamedInstrProfRecord) {
        if let Some(name) = record.name.as_ref() {
            let hash = compute_hash(name);
            if self.symtab.contains(hash) {
                // Find the record and merge things. 0 hashed records should have no counters in the
                // code and otherwise we'll ignore the change that truncated md5 hashes can collide
                if let Some(rec) = self.records.iter_mut().find(|x| x.name == record.name) {
                    rec.record.merge(&record.record);
                }
            } else {
                self.symtab.names.insert(hash, record.name_unchecked());
                self.records.push(record.clone());
            }
        }
    }

    pub fn get_record(&self, name: &str) -> Option<&NamedInstrProfRecord> {
        self.records
            .iter()
            .find(|x| x.name.as_deref() == Some(name))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NamedInstrProfRecord {
    pub name: Option<String>,
    pub hash: Option<u64>,
    pub record: InstrProfRecord,
}

impl NamedInstrProfRecord {
    /// This bit is reserved as the flag for the context sensitive profile record
    const CS_FLAG_IN_FUNC_HASH: u64 = 60;

    pub fn num_value_sites(&self, valuekind: ValueKind) -> usize {
        use ValueKind::*;
        let record_data = self.record.data.as_ref();
        match valuekind {
            IndirectCallTarget => record_data.map(|x| x.indirect_callsites.len()),
            MemOpSize => record_data.map(|x| x.mem_op_sizes.len()),
        }
        .unwrap_or_default()
    }

    pub fn has_cs_flag(&self) -> bool {
        let hash = self.hash.unwrap_or_default();
        ((hash >> Self::CS_FLAG_IN_FUNC_HASH) & 1) != 0
    }

    pub fn set_cs_flag(&mut self) {
        let x = self.hash.get_or_insert(0);
        *x &= Self::CS_FLAG_IN_FUNC_HASH;
    }

    pub fn counts(&self) -> &[u64] {
        &self.record.counts
    }

    pub fn hash_unchecked(&self) -> u64 {
        self.hash.unwrap_or_default()
    }

    pub fn name_unchecked(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrProfRecord {
    pub counts: Vec<u64>,
    pub data: Option<Box<ValueProfDataRecord>>,
}

impl InstrProfRecord {
    pub fn merge(&mut self, other: &Self) {
        if self.counts.len() != other.counts.len() {
            return;
        }
        for (own, other) in self.counts.iter_mut().zip(other.counts.iter()) {
            let res = own.checked_add(*other);
            *own = match res {
                Some(s) => s,
                None => u64::MAX, // TODO handle the warnings?
            };
        }
        // TODO merge the data
        if let Some((own, other)) = self.data.as_mut().zip(other.data.as_ref()) {
            if own.indirect_callsites.len() == other.indirect_callsites.len() {
                for (own, other) in own
                    .indirect_callsites
                    .iter_mut()
                    .zip(other.indirect_callsites.iter())
                {
                    merge_site_records(own, other);
                }
            }
            if own.mem_op_sizes.len() == other.mem_op_sizes.len() {
                for (own, other) in own.mem_op_sizes.iter_mut().zip(other.mem_op_sizes.iter()) {
                    merge_site_records(own, other);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ValueProfDataRecord {
    pub indirect_callsites: Vec<InstrProfValueSiteRecord>,
    pub mem_op_sizes: Vec<InstrProfValueSiteRecord>,
}

type InstrProfValueSiteRecord = Vec<InstrProfValueData>;

fn merge_site_records(dst: &mut InstrProfValueSiteRecord, src: &InstrProfValueSiteRecord) {
    if dst.len() == src.len() {
        dst.sort_unstable();
        let mut other_vals = src.iter().map(|x| x.value).collect::<Vec<u64>>();
        other_vals.sort_unstable();
        let mut i = 0;
        for j in src {
            let current = dst
                .iter_mut()
                .enumerate()
                .skip(i)
                .skip_while(|x| x.1.value < j.value)
                .nth(0);

            match current {
                Some((index, element)) if element.value == j.value => {
                    element.count = element.count.checked_add(j.count).unwrap_or(u64::MAX);
                    dst.insert(index + 1, j.clone());
                    i = index + 1;
                }
                Some((index, _)) => {
                    dst.insert(index, j.clone());
                    i = index + 1;
                }
                None => {
                    i = dst.len();
                    dst.push(j.clone());
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct InstrProfValueData {
    pub value: u64,
    pub count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueProfData {
    pub(crate) total_size: u32,
    pub(crate) num_value_kinds: u32,
}

/// TODO This is currently unused but unsure on if it's used in llvm coverage. Every file I've
/// tried has had the number of these set to zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueProfRecord {
    kind: u32,
    num_value_sites: u32,
    site_count: u8,
}

impl ValueProfData {
    fn deserialize_to(&self, record: &mut InstrProfRecord, symtab: Option<&Symtab>) {
        if self.num_value_kinds == 0 {
            return;
        }

        todo!()
    }
}

impl PartialOrd for InstrProfValueData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InstrProfValueData {
    fn cmp(&self, other: &Self) -> Ordering {
        // Do the reverse here
        self.value.cmp(&other.value)
    }
}

impl PartialEq for InstrProfValueData {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
