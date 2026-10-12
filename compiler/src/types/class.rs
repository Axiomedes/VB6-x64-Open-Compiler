//! VB6 Class Layout and Object Model Metadata.

use crate::types::udt::UdtLayout;
use crate::types::VbType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassField {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub alignment: usize,
    pub vb_type: VbType,
    pub is_public: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassMethod {
    pub name: String,
    pub mangled_name: String,
    pub is_sub: bool,
    pub is_public: bool,
    pub return_type: Option<VbType>,
    pub param_types: Vec<VbType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassProperty {
    pub name: String,
    pub has_get: bool,
    pub has_let: bool,
    pub has_set: bool,
    pub is_public: bool,
    pub prop_type: VbType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassLayout {
    pub name: String,
    pub fields: Vec<ClassField>,
    pub methods: HashMap<String, ClassMethod>,
    pub properties: HashMap<String, ClassProperty>,
    pub has_initialize: bool,
    pub has_terminate: bool,
    pub total_size: usize,
    pub alignment: usize,
}

impl ClassLayout {
    pub fn compute(
        name: &str,
        raw_fields: &[(String, VbType, bool)],
        udt_layouts: &HashMap<String, UdtLayout>,
        is_64bit: bool,
    ) -> Self {
        let mut fields = Vec::new();
        let mut current_offset = 0usize;
        let mut max_alignment = 8usize; // Minimum alignment 8 bytes for object pointers/alignment

        for (f_name, f_type, is_public) in raw_fields {
            let (size, align) = match f_type {
                VbType::UserDefined(nested_name) => {
                    if let Some(nested) = udt_layouts.get(&nested_name.to_ascii_uppercase()) {
                        (nested.total_size, nested.alignment)
                    } else {
                        (f_type.size_in_bytes(is_64bit), f_type.alignment(is_64bit).max(1))
                    }
                }
                _ => (f_type.size_in_bytes(is_64bit), f_type.alignment(is_64bit).max(1)),
            };

            if align > max_alignment {
                max_alignment = align;
            }

            let padding = (align - (current_offset % align)) % align;
            current_offset += padding;

            fields.push(ClassField {
                name: f_name.clone(),
                offset: current_offset,
                size,
                alignment: align,
                vb_type: f_type.clone(),
                is_public: *is_public,
            });

            current_offset += size;
        }

        let total_padding = (max_alignment - (current_offset % max_alignment)) % max_alignment;
        let total_size = (current_offset + total_padding).max(8);

        Self {
            name: name.to_string(),
            fields,
            methods: HashMap::new(),
            properties: HashMap::new(),
            has_initialize: false,
            has_terminate: false,
            total_size,
            alignment: max_alignment,
        }
    }

    pub fn find_field(&self, name: &str) -> Option<&ClassField> {
        self.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn find_method(&self, name: &str) -> Option<&ClassMethod> {
        self.methods.get(&name.to_ascii_uppercase())
    }

    pub fn find_property(&self, name: &str) -> Option<&ClassProperty> {
        self.properties.get(&name.to_ascii_uppercase())
    }
}
