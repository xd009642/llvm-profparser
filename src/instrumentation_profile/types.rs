use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt;

const VARIANT_MASKS_ALL: u64 = 0xff00_0000_0000_0000;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
const VARIANT_MASK_IR_PROF: u64 = 1u64 << 56;
/// This is taken from `llvm/include/llvm/ProfileData/InstrProfileData.inc`
const VARIANT_MASK_CSIR_PROF: u64 = 1u64 << 57;

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

impl Symtab {
    pub fn len(&self) -> usize {
        self.names.len()
    }
}

impl Symtab {
    pub fn add_func_name(&mut self, name: String) {
        let hash = md5::compute(&name).0[..8].try_into().unwrap();
        let hash = u64::from_ne_bytes(hash);
        self.names.insert(hash, name);
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
    pub(crate) version: u64,
    pub records: Vec<NamedInstrProfRecord>,
    pub symtab: Symtab,
}

impl InstrumentationProfile {
    pub fn version(&self) -> u64 {
        self.version & !VARIANT_MASKS_ALL
    }

    pub fn is_ir_level_profile(&self) -> bool {
        (self.version & VARIANT_MASK_IR_PROF) != 0
    }

    pub fn has_csir_level_profile(&self) -> bool {
        (self.version & VARIANT_MASK_CSIR_PROF) != 0
    }

    pub fn get_level(&self) -> InstrumentationLevel {
        if self.is_ir_level_profile() {
            InstrumentationLevel::Ir
        } else {
            InstrumentationLevel::FrontEnd
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NamedInstrProfRecord {
    pub name: Option<String>,
    pub hash: Option<u64>,
    pub record: InstrProfRecord,
}

impl NamedInstrProfRecord {
    pub fn num_value_sites(&self, valuekind: ValueKind) -> usize {
        use ValueKind::*;
        let record_data = self.record.data.as_ref();
        match valuekind {
            IndirectCallTarget => record_data.map(|x| x.indirect_callsites.len()),
            MemOpSize => record_data.map(|x| x.mem_op_sizes.len()),
        }
        .unwrap_or_default()
    }

    pub fn counts(&self) -> &[u64] {
        &self.record.counts
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrProfRecord {
    pub counts: Vec<u64>,
    pub data: Option<Box<ValueProfDataRecord>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ValueProfDataRecord {
    pub indirect_callsites: Vec<InstrProfValueSiteRecord>,
    pub mem_op_sizes: Vec<InstrProfValueSiteRecord>,
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
