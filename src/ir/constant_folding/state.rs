use std::collections::{BTreeSet, HashMap};

use crate::ir::const_eval::ConstVal;

pub(super) struct ConstState<'a> {
    pub(super) constants: &'a mut HashMap<String, ConstVal>,
    pub(super) static_vars: &'a BTreeSet<String>,
}

impl ConstState<'_> {
    pub(super) fn remember(&mut self, dst: &str, value: ConstVal) {
        if self.static_vars.contains(dst) {
            self.constants.remove(dst);
        } else {
            self.constants.insert(dst.to_string(), value);
        }
    }

    pub(super) fn forget(&mut self, dst: &str) {
        self.constants.remove(dst);
    }

    pub(super) fn clear(&mut self) {
        self.constants.clear();
    }
}
