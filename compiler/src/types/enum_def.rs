//! VB6 Enum definitions and automatic member value evaluation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumMemberDef {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDefinition {
    pub name: String,
    pub members: Vec<EnumMemberDef>,
    values_by_name: HashMap<String, i32>,
}

impl EnumDefinition {
    /// Create a new EnumDefinition from AST member definitions, computing auto-increment values.
    pub fn new(name: &str, raw_members: &[(String, Option<i32>)]) -> Self {
        let mut members = Vec::new();
        let mut values_by_name = HashMap::new();
        let mut current_val = 0i32;

        for (mem_name, explicit_val) in raw_members {
            let val = match explicit_val {
                Some(v) => *v,
                None => current_val,
            };
            members.push(EnumMemberDef {
                name: mem_name.clone(),
                value: val,
            });
            values_by_name.insert(mem_name.to_ascii_uppercase(), val);
            current_val = val.wrapping_add(1);
        }

        Self {
            name: name.to_string(),
            members,
            values_by_name,
        }
    }

    /// Lookup enum member value by case-insensitive name.
    pub fn get_value(&self, member_name: &str) -> Option<i32> {
        self.values_by_name.get(&member_name.to_ascii_uppercase()).copied()
    }
}
