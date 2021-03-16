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
    u64::from_ne_bytes(hash)
}

impl Symtab {
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn add_func_name(&mut self, name: String) {
        let hash = compute_hash(&name);
        self.names.insert(hash, name);
    }

    pub fn contains(&self, hash: u64) -> bool {
        self.names.contains_key(&hash)
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
    pub records: Vec<NamedInstrProfRecord>,
    pub symtab: Symtab,
}

impl InstrumentationProfile {
    pub fn version(&self) -> Option<u64> {
        self.version
    }

    pub fn is_ir_level_profile(&self) -> bool {
        self.is_ir
    }

    pub fn has_csir_level_profile(&self) -> bool {
        self.has_csir
    }

    pub fn get_level(&self) -> InstrumentationLevel {
        if self.is_ir_level_profile() {
            InstrumentationLevel::Ir
        } else {
            InstrumentationLevel::FrontEnd
        }
    }

    pub fn merge(&mut self, other: &Self) {
        for func in &other.records {
            self.merge_record(&func);
        }
    }

    pub fn merge_record(&mut self, record: &NamedInstrProfRecord) {
        if self.symtab.contains(record.hash_unchecked()) {
            // Find the record and merge tings
            if let Some(rec) = self.records.iter_mut().find(|x| x.name == record.name) {
                rec.record.merge(&record.record);
            } else {
                panic!("We've fallen victim to a hash collision.");
            }
        } else {
            self.symtab
                .names
                .insert(record.hash_unchecked(), record.name_unchecked());
            self.records.push(record.clone());
        }
        todo!();
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
    total_size: u32,
    num_value_kinds: u32,
}

impl PartialOrd for InstrProfValueData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InstrProfValueData {
    fn cmp(&self, other: &Self) -> Ordering {
        // Do the reverse here
        self.value.cmp(&self.value)
    }
}

impl PartialEq for InstrProfValueData {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
