use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::ast::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct MemberEntry {
    pub member_type: Type,
    pub offset: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructEntry {
    pub size: i64,
    pub alignment: i64,
    pub members: HashMap<String, MemberEntry>,
    pub order: Vec<String>,
}

fn table() -> &'static Mutex<HashMap<String, StructEntry>> {
    static TABLE: OnceLock<Mutex<HashMap<String, StructEntry>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn declared() -> &'static Mutex<std::collections::HashSet<String>> {
    static DECLARED: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    DECLARED.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

pub fn reset() {
    table().lock().map(|mut t| t.clear()).ok();
    declared().lock().map(|mut d| d.clear()).ok();
}

pub fn declare(tag: &str) {
    if let Ok(mut d) = declared().lock() {
        d.insert(tag.to_string());
    }
}

pub fn add(tag: String, entry: StructEntry) {
    declare(&tag);
    if let Ok(mut t) = table().lock() {
        t.insert(tag, entry);
    }
}

pub fn is_declared(tag: &str) -> bool {
    declared().lock().is_ok_and(|d| d.contains(tag))
}

pub fn contains(tag: &str) -> bool {
    table().lock().is_ok_and(|t| t.contains_key(tag))
}

pub fn is_complete(tag: &str) -> bool {
    contains(tag)
}

pub fn get(tag: &str) -> Option<StructEntry> {
    table().lock().ok().and_then(|t| t.get(tag).cloned())
}

pub fn member(tag: &str, name: &str) -> Option<MemberEntry> {
    get(tag).and_then(|entry| entry.members.get(name).cloned())
}

pub fn members_in_order(tag: &str) -> Vec<MemberEntry> {
    get(tag)
        .map(|entry| {
            entry
                .order
                .iter()
                .filter_map(|name| entry.members.get(name).cloned())
                .collect()
        })
        .unwrap_or_default()
}
