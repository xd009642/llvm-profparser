use std::collections::BTreeMap;
use std::convert::TryInto;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
