//! VB6 Type specification in AST.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeSpec {
    pub name: String,
    pub is_new: bool,
    pub fixed_length: Option<usize>, // e.g. String * 30
    pub array_dimensions: Vec<ArrayDimension>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayDimension {
    pub lower: Option<i32>, // Default 0 (or 1 if Option Base 1)
    pub upper: i32,
}

impl TypeSpec {
    pub fn simple(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_new: false,
            fixed_length: None,
            array_dimensions: Vec::new(),
        }
    }
}
