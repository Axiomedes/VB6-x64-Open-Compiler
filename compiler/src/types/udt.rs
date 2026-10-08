//! User-Defined Type (UDT / struct) layout calculation and memory alignment.

use crate::types::VbType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UdtMemberLayout {
    pub name: String,
    pub vb_type: VbType,
    pub offset: usize,
    pub size: usize,
    pub alignment: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UdtLayout {
    pub name: String,
    pub members: Vec<UdtMemberLayout>,
    pub total_size: usize,
    pub alignment: usize,
}

impl UdtLayout {
    /// Calculate the memory layout of a User-Defined Type given member specifications.
    pub fn compute(
        name: &str,
        raw_members: &[(String, VbType)],
        is_64bit: bool,
    ) -> Self {
        let mut members = Vec::new();
        let mut current_offset = 0usize;
        let mut max_alignment = 1usize;

        for (mem_name, mem_type) in raw_members {
            let size = mem_type.size_in_bytes(is_64bit);
            let align = mem_type.alignment(is_64bit).max(1);
            if align > max_alignment {
                max_alignment = align;
            }

            // Align current offset to member alignment
            let padding = (align - (current_offset % align)) % align;
            current_offset += padding;

            members.push(UdtMemberLayout {
                name: mem_name.clone(),
                vb_type: mem_type.clone(),
                offset: current_offset,
                size,
                alignment: align,
            });

            current_offset += size;
        }

        // Align total size to maximum alignment boundary
        let final_padding = (max_alignment - (current_offset % max_alignment)) % max_alignment;
        let total_size = current_offset + final_padding;

        Self {
            name: name.to_string(),
            members,
            total_size,
            alignment: max_alignment,
        }
    }

    /// Find a member layout by case-insensitive name.
    pub fn find_member(&self, member_name: &str) -> Option<&UdtMemberLayout> {
        self.members
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(member_name))
    }
}
