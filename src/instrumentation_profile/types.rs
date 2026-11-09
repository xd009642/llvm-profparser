use nom::number::Endianness;
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt;
use std::sync::Arc;
use tracing::warn;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Symtab {
    pub names: FxHashMap<u64, Arc<str>>,
}

impl Symtab {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let mut names = FxHashMap::default();
        names.reserve(capacity);
        Self { names }
    }
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
    /// Number of symbols in the table
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// True if there are no symbols in the table
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Some formats such as the Raw profiles have configurable endianness. I think these may
    /// require a matching endian hash. However, this doesn't seem to be represented in any of the
    /// llvm test files so is largely a mystery. Computes a little endian hash unless specified
    /// otherwise.
    pub fn add_func_name(&mut self, name: Arc<str>, endianness: Option<Endianness>) {
        let hash = match endianness {
            Some(Endianness::Big) => compute_be_hash(name.as_bytes()),
            _ => compute_hash(name.as_bytes()),
        };
        self.names.insert(hash, name);
    }

    pub fn add_func_name_with_hash(&mut self, name: Arc<str>, hash: u64) {
        self.names.insert(hash, name);
    }

    pub fn contains(&self, hash: u64) -> bool {
        self.names.contains_key(&hash)
    }

    pub fn get(&self, hash: u64) -> Option<&str> {
        self.names.get(&hash).map(AsRef::as_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &str)> {
        self.names.iter().map(|(hash, name)| (hash, name.as_ref()))
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

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct InstrumentationProfile {
    pub(crate) version: Option<u64>,
    pub(crate) has_csir: bool,
    pub(crate) is_ir: bool,
    pub(crate) is_entry_first: bool,
    pub(crate) is_byte_coverage: bool,
    pub(crate) fn_entry_only: bool,
    pub(crate) memory_profiling: bool,
    records: Vec<NamedInstrProfRecord>,
    record_name_lookup: FxHashMap<Arc<str>, usize>,
    record_key_lookup: FxHashMap<(Arc<str>, Option<u64>), usize>,
    pub symtab: Symtab,
}

impl InstrumentationProfile {
    pub fn new(version: Option<u64>, has_csir: bool, is_ir: bool, is_entry_first: bool) -> Self {
        Self {
            version,
            has_csir,
            is_ir,
            is_entry_first,
            ..Default::default()
        }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let mut record_name_lookup = FxHashMap::default();
        record_name_lookup.reserve(capacity);
        Self {
            records: Vec::with_capacity(capacity),
            record_name_lookup,
            record_key_lookup: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            symtab: Symtab::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub(crate) fn reserve_records(&mut self, additional: usize) {
        self.records.reserve(additional);
        self.record_name_lookup.reserve(additional);
        self.record_key_lookup.reserve(additional);
    }

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

    pub fn records(&self) -> &[NamedInstrProfRecord] {
        &self.records
    }

    pub fn push_record(&mut self, record: NamedInstrProfRecord) {
        if let Some(name) = record.name.as_ref() {
            self.record_key_lookup
                .insert((name.clone(), record.hash), self.records.len());
            self.record_name_lookup
                .insert(name.clone(), self.records.len());
        }
        self.records.push(record);
    }

    /// Finds the most recently contributed structural version of a named function.
    /// Coverage mappings merge contributions and select the matching structural hash.
    pub fn find_record_by_name(&self, name: &str) -> Option<&NamedInstrProfRecord> {
        self.record_name_lookup.get(name).map(|x| &self.records[*x])
    }

    /// Finds the record matching both the name hash and structural function hash.
    /// Functions with different structural hashes have distinct counter layouts.
    pub(crate) fn find_record_by_hashes(
        &self,
        name_hash: u64,
        function_hash: u64,
    ) -> Option<&NamedInstrProfRecord> {
        let name = self.symtab.names.get(&name_hash)?;
        self.record_key_lookup
            .get(&(name.clone(), Some(function_hash)))
            .map(|index| &self.records[*index])
    }

    pub fn find_record_by_hash(&self, hash: u64) -> Option<&NamedInstrProfRecord> {
        let name = self.symtab.get(hash)?;
        self.find_record_by_name(name)
    }

    pub fn find_record_by_name_mut(&mut self, name: &str) -> Option<&mut NamedInstrProfRecord> {
        if let Some(index) = self.record_name_lookup.get(name) {
            Some(&mut self.records[*index])
        } else {
            None
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
        if self.version.is_none() && other.version.is_some() {
            self.version = other.version;
        }
        for func in &other.records {
            self.merge_record(func);
        }
    }

    pub fn merge_record(&mut self, record: &NamedInstrProfRecord) {
        if let Some(hash) = record.name_hash {
            self.symtab.names.insert(hash, record.name_arc_unchecked());
            if let Some(name) = record.name.as_ref() {
                if let Some(index) = self.record_key_lookup.get(&(name.clone(), record.hash)) {
                    self.records[*index].record.merge(&record.record);
                    self.record_name_lookup.insert(name.clone(), *index);
                    return;
                }
            }
            self.push_record(record.clone());
        }
    }

    pub(crate) fn has_duplicate_records(&self) -> bool {
        self.record_key_lookup.len()
            < self
                .records
                .iter()
                .filter(|record| record.name.is_some())
                .count()
    }

    pub(crate) fn try_merge(&mut self, other: &Self) -> std::io::Result<()> {
        if self.version.is_none() && other.version.is_some() {
            self.version = other.version;
        }
        for record in &other.records {
            self.try_merge_record(record)?;
        }
        Ok(())
    }

    pub(crate) fn try_merge_record(
        &mut self,
        record: &NamedInstrProfRecord,
    ) -> std::io::Result<()> {
        if let Some(name) = record.name.as_ref() {
            if let Some(index) = self.record_key_lookup.get(&(name.clone(), record.hash)) {
                if self.records[*index].record.counts.len() != record.record.counts.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("counter count mismatch for function {name}"),
                    ));
                }
            }
        }
        self.merge_record(record);
        Ok(())
    }

    /// Parsing retains individual records for `show`; merging and coverage need
    /// one summed contribution per function identity, including the first file.
    pub(crate) fn normalize(mut self) -> std::io::Result<Self> {
        let records = std::mem::take(&mut self.records);
        self.record_name_lookup.clear();
        self.record_key_lookup.clear();
        self.records.reserve(records.len());
        for record in records {
            if record.name_hash.is_some() {
                self.try_merge_record(&record)?;
            } else {
                self.push_record(record);
            }
        }
        Ok(self)
    }

    /// Gets the instrumentation record for the give function
    pub fn get_record(&self, name: &str) -> Option<&NamedInstrProfRecord> {
        self.records.iter().find(|x| x.name() == Some(name))
    }

    /// Returns true if there are no instrumentation records associated with the profile
    pub fn is_empty(&self) -> bool {
        self.records.is_empty() && self.symtab.is_empty()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NamedInstrProfRecord {
    pub name: Option<Arc<str>>,
    pub name_hash: Option<u64>,
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

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn hash_unchecked(&self) -> u64 {
        self.hash.unwrap_or_default()
    }

    pub(crate) fn name_arc_unchecked(&self) -> Arc<str> {
        self.name.clone().unwrap_or_default()
    }

    pub fn name_unchecked(&self) -> String {
        self.name().unwrap_or_default().to_string()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InstrProfRecord {
    pub counts: Vec<u64>,
    pub data: Option<Box<ValueProfDataRecord>>,
}

impl InstrProfRecord {
    /// Adds counters and value observations, saturating counts on overflow.
    /// A counter-count mismatch warns and leaves the record unchanged. A value-site
    /// count mismatch warns and skips that value kind, retaining merged counters
    /// and independently merging compatible kinds, as LLVM does.
    pub fn merge(&mut self, other: &Self) {
        if self.counts.len() != other.counts.len() {
            warn!("counter count mismatch while merging profile records");
            return;
        }
        for (own, other) in self.counts.iter_mut().zip(other.counts.iter()) {
            let res = own.checked_add(*other);
            *own = match res {
                Some(s) => s,
                None => u64::MAX, // TODO handle the warnings?
            };
        }
        for kind in [ValueKind::IndirectCallTarget, ValueKind::MemOpSize] {
            let own = self.data.as_mut().map(|data| match kind {
                ValueKind::IndirectCallTarget => &mut data.indirect_callsites,
                ValueKind::MemOpSize => &mut data.mem_op_sizes,
            });
            let other = other.data.as_ref().map(|data| match kind {
                ValueKind::IndirectCallTarget => &data.indirect_callsites,
                ValueKind::MemOpSize => &data.mem_op_sizes,
            });
            let own_len = own.as_ref().map_or(0, |sites| sites.len());
            let other_len = other.map_or(0, |sites| sites.len());
            // Absent value data represents zero sites, not an empty observation list
            // at every site. LLVM skips mismatched kinds after merging counters.
            if own_len != other_len {
                warn!(
                    ?kind,
                    own_len, other_len, "function value site count mismatch"
                );
                continue;
            }
            if let Some((own, other)) = own.zip(other) {
                for (own, other) in own.iter_mut().zip(other) {
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
    dst.extend_from_slice(src);
    dst.sort_unstable();
    dst.dedup_by(|incoming, existing| {
        if incoming.value == existing.value {
            existing.count = existing.count.saturating_add(incoming.count);
            true
        } else {
            false
        }
    });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BranchParameters {
    pub id: i16,
    pub false_cond: i16,
    pub true_cond: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionParameters {
    pub bitmap_idx: u32,
    pub num_conditions: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MCDCParams {
    Branch(BranchParameters),
    Decision(DecisionParameters),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LLVM merges all observations at a site even when runs observe different sets of values.
    #[test]
    fn duplicate_value_sites_preserve_observations_in_first_file() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/profdata/duplicate-value-sites.proftext");
        let profile = crate::merge_profiles(&[fixture]).expect("valid duplicate value sites");
        assert_eq!(profile.records().len(), 1);
        let record = &profile.records()[0];
        assert_eq!(record.counts(), &[7]);
        let data = record
            .record
            .data
            .as_ref()
            .expect("memory size observations");
        assert_eq!(data.mem_op_sizes.len(), 1);
        assert_eq!(
            data.mem_op_sizes[0]
                .iter()
                .map(|entry| (entry.value, entry.count))
                .collect::<Vec<_>>(),
            [(8, 8), (16, 7)]
        );
    }

    /// Site observations are keyed by value, independent of input ordering or cardinality.
    #[test]
    fn merge_value_site_observations_across_repeated_contributions() {
        let mut site = vec![
            InstrProfValueData {
                value: 300,
                count: 2,
            },
            InstrProfValueData {
                value: 100,
                count: 3,
            },
        ];
        merge_site_records(
            &mut site,
            &vec![
                InstrProfValueData {
                    value: 100,
                    count: 5,
                },
                InstrProfValueData {
                    value: 200,
                    count: 7,
                },
                InstrProfValueData {
                    value: 300,
                    count: 11,
                },
            ],
        );
        merge_site_records(
            &mut site,
            &vec![
                InstrProfValueData {
                    value: 200,
                    count: 17,
                },
                InstrProfValueData {
                    value: 100,
                    count: 19,
                },
            ],
        );
        assert_eq!(
            site.iter()
                .map(|entry| (entry.value, entry.count))
                .collect::<Vec<_>>(),
            [(100, 27), (200, 24), (300, 13)]
        );
        merge_site_records(&mut site, &vec![]);
        let mut empty = vec![];
        merge_site_records(&mut empty, &site);
        assert_eq!(
            empty
                .iter()
                .map(|entry| (entry.value, entry.count))
                .collect::<Vec<_>>(),
            [(100, 27), (200, 24), (300, 13)]
        );
        merge_site_records(
            &mut site,
            &vec![InstrProfValueData {
                value: 100,
                count: u64::MAX,
            }],
        );
        assert_eq!(site[0].count, u64::MAX);
    }

    /// Like LLVM, mismatched kinds retain destination observations while compatible kinds merge.
    #[test]
    fn merge_value_site_count_mismatch_preserves_other_kinds() {
        let mut base = InstrProfRecord {
            counts: vec![2],
            data: Some(Box::new(ValueProfDataRecord {
                indirect_callsites: vec![vec![InstrProfValueData {
                    value: 100,
                    count: 3,
                }]],
                mem_op_sizes: vec![vec![InstrProfValueData { value: 8, count: 5 }]],
            })),
        };
        let other = InstrProfRecord {
            counts: vec![7],
            data: Some(Box::new(ValueProfDataRecord {
                indirect_callsites: vec![],
                mem_op_sizes: vec![vec![InstrProfValueData {
                    value: 8,
                    count: 11,
                }]],
            })),
        };
        base.merge(&other);
        assert_eq!(base.counts, [9]);
        let data = base.data.as_ref().expect("existing observations retained");
        assert_eq!(data.indirect_callsites[0][0].count, 3);
        assert_eq!(data.mem_op_sizes[0][0].count, 16);

        let mut absent = InstrProfRecord {
            counts: vec![13],
            data: None,
        };
        absent.merge(&base);
        assert_eq!(absent.counts, [22]);
        assert!(absent.data.is_none());
        base.merge(&InstrProfRecord {
            counts: vec![17],
            data: None,
        });
        assert_eq!(base.counts, [26]);
        let data = base
            .data
            .as_ref()
            .expect("mismatched source does not erase observations");
        assert_eq!(data.indirect_callsites[0][0].count, 3);
        assert_eq!(data.mem_op_sizes[0][0].count, 16);
    }

    /// Merges value profile sites by profiled value so repeated targets are combined once.
    #[test]
    fn merge_value_profile_sites_combines_matching_values() {
        let mut base = InstrProfRecord {
            counts: vec![10, 20],
            data: Some(Box::new(ValueProfDataRecord {
                indirect_callsites: vec![vec![
                    InstrProfValueData { value: 1, count: 1 },
                    InstrProfValueData { value: 2, count: 2 },
                ]],
                mem_op_sizes: vec![vec![
                    InstrProfValueData {
                        value: 16,
                        count: 3,
                    },
                    InstrProfValueData {
                        value: 32,
                        count: 4,
                    },
                ]],
            })),
        };
        let other = InstrProfRecord {
            counts: vec![5, 7],
            data: Some(Box::new(ValueProfDataRecord {
                indirect_callsites: vec![vec![
                    InstrProfValueData { value: 2, count: 5 },
                    InstrProfValueData { value: 3, count: 8 },
                ]],
                mem_op_sizes: vec![vec![
                    InstrProfValueData {
                        value: 32,
                        count: 6,
                    },
                    InstrProfValueData {
                        value: 64,
                        count: 9,
                    },
                ]],
            })),
        };

        base.merge(&other);

        let value_data = base
            .data
            .as_ref()
            .expect("merged record should retain value profile data");
        assert_eq!(base.counts, vec![15, 27]);
        assert_eq!(
            value_data.indirect_callsites,
            vec![vec![
                InstrProfValueData { value: 1, count: 1 },
                InstrProfValueData { value: 2, count: 7 },
                InstrProfValueData { value: 3, count: 8 },
            ]]
        );
        assert_eq!(
            value_data.mem_op_sizes,
            vec![vec![
                InstrProfValueData {
                    value: 16,
                    count: 3,
                },
                InstrProfValueData {
                    value: 32,
                    count: 10,
                },
                InstrProfValueData {
                    value: 64,
                    count: 9,
                },
            ]]
        );
    }

    fn record(hash: u64, counts: &[u64]) -> NamedInstrProfRecord {
        NamedInstrProfRecord {
            name: Some(Arc::from("function")),
            name_hash: Some(compute_hash("function")),
            hash: Some(hash),
            record: InstrProfRecord {
                counts: counts.to_vec(),
                data: None,
            },
        }
    }

    /// Normalization combines duplicate contributions independent of ordering.
    #[test]
    fn duplicate_records_sum_counters_in_either_order() {
        let records = [record(7, &[2, 0]), record(7, &[0, 3])];
        for reverse in [false, true] {
            let mut profile = InstrumentationProfile::default();
            for index in if reverse { [1, 0] } else { [0, 1] } {
                profile.push_record(records[index].clone());
            }
            let profile = profile.normalize().expect("matching duplicate layouts");
            assert_eq!(profile.records().len(), 1);
            assert_eq!(
                profile
                    .find_record_by_name("function")
                    .expect("merged function")
                    .record
                    .counts,
                [2, 3]
            );
        }
    }

    /// Merging profiles must preserve different versions and merge only matching layouts.
    #[test]
    fn duplicate_records_preserve_structural_versions() {
        let mut profile = InstrumentationProfile::default();
        profile.merge_record(&record(7, &[2, 0]));
        profile.merge_record(&record(8, &[5]));
        let mut other = InstrumentationProfile::default();
        other.merge_record(&record(7, &[0, 3]));
        profile.merge(&other);
        let hash = compute_hash("function");
        assert_eq!(profile.records().len(), 2);
        assert_eq!(
            profile
                .find_record_by_hashes(hash, 7)
                .expect("first layout")
                .record
                .counts,
            [2, 3]
        );
        assert_eq!(
            profile
                .find_record_by_hashes(hash, 8)
                .expect("second layout")
                .record
                .counts,
            [5]
        );
        assert!(profile.find_record_by_hashes(hash, 9).is_none());
        assert_eq!(
            profile
                .find_record_by_name("function")
                .expect("legacy name lookup")
                .hash,
            Some(7)
        );
    }
    /// LLVM's counter-mismatch fixture is displayable but cannot be merged safely.
    #[test]
    fn duplicate_records_reject_incompatible_counter_layouts() {
        let mut profile = InstrumentationProfile::new(Some(7), false, false, false);
        profile.push_record(record(7, &[1]));
        profile.push_record(record(7, &[2, 3]));
        assert_eq!(
            profile
                .clone()
                .normalize()
                .expect_err("mismatched counters")
                .kind(),
            std::io::ErrorKind::InvalidData
        );
        let error = crate::CoverageMapping::new(&[], &profile, false)
            .expect_err("mapping must reject mismatched counters");
        assert!(error.to_string().contains("counter count mismatch"));
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/profdata/llvm-15/counter-mismatch-1.proftext");
        assert_eq!(
            crate::merge_profiles(&[fixture])
                .expect_err("invalid LLVM merge fixture")
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }
    /// Text profiles omit the raw format version, which a later raw input supplies.
    #[test]
    fn checked_merge_retains_version_from_later_profile() {
        for version in [7, 8] {
            let mut text = InstrumentationProfile::default();
            text.merge_record(&record(7, &[1]));
            let mut raw = InstrumentationProfile::new(Some(version), false, false, false);
            raw.merge_record(&record(7, &[2]));
            text.try_merge(&raw).expect("matching counter layouts");
            assert_eq!(text.version(), Some(version));
            assert_eq!(text.records()[0].record.counts, [3]);
        }
    }
}
