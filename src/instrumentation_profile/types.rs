use std::collections::BTreeMap;
use std::convert::TryInto;

/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
const VARIANT_MASK_IR_PROF: u64 = 1u64 << 56;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
const VARIANT_MASK_CSIR_PROF: u64 = 1u64 << 57;

#[derive(Debug, Clone, Default, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Symtab {
    pub names: BTreeMap<u64, String>,
}

impl Symtab {
    pub fn add_func_name(&mut self, name: String) {
        let hash = md5::compute(&name).0[..8].try_into().unwrap();
        let hash = u64::from_ne_bytes(hash);
        self.names.insert(hash, name);
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrumentationProfile {
    pub records: Vec<NamedInstrProfRecord>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NamedInstrProfRecord {
    pub name: Option<String>,
    pub hash: Option<u64>,
    pub record: InstrProfRecord,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrProfRecord {
    pub(crate) counts: Vec<u64>,
    pub(crate) data: Option<Box<ValueProfDataRecord>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ValueProfDataRecord {
    pub(crate) indirect_callsites: Vec<InstrProfValueSiteRecord>,
    pub(crate) mem_op_sizes: Vec<InstrProfValueSiteRecord>,
}

type InstrProfValueSiteRecord = Vec<InstrProfValueData>;

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrProfValueData {
    value: u64,
    count: u64,
}

#[derive(Clone, Debug)]
pub struct ValueProfData {
    total_size: u32,
    num_value_kinds: u32,
}
