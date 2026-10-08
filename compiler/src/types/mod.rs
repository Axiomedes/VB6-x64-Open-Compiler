//! Visual Basic 6.0 Type System and coercion rules.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VbType {
    Byte,
    Boolean,
    Integer,
    Long,
    LongPtr,  // Pointer-sized integer (32-bit in x86, 64-bit in x64)
    LongLong, // 64-bit signed integer
    Single,
    Double,
    Currency,
    Date,
    String,
    Variant,
    Object,
    Array(Box<VbType>),
    UserDefined(String),
    Unknown,
}

impl VbType {
    /// Parse a type name string into a VbType (case-insensitive).
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_uppercase().as_str() {
            "BYTE" => VbType::Byte,
            "BOOLEAN" => VbType::Boolean,
            "INTEGER" => VbType::Integer,
            "LONG" => VbType::Long,
            "LONGPTR" => VbType::LongPtr,
            "LONGLONG" => VbType::LongLong,
            "SINGLE" => VbType::Single,
            "DOUBLE" => VbType::Double,
            "CURRENCY" => VbType::Currency,
            "DATE" => VbType::Date,
            "STRING" => VbType::String,
            "VARIANT" => VbType::Variant,
            "OBJECT" => VbType::Object,
            other => VbType::UserDefined(other.to_string()),
        }
    }

    /// Return the size in bytes on the target architecture.
    pub fn size_in_bytes(&self, is_64bit: bool) -> usize {
        match self {
            VbType::Byte => 1,
            VbType::Boolean => 2,
            VbType::Integer => 2,
            VbType::Long => 4,
            VbType::LongPtr => if is_64bit { 8 } else { 4 },
            VbType::LongLong => 8,
            VbType::Single => 4,
            VbType::Double => 8,
            VbType::Currency => 8,
            VbType::Date => 8,
            VbType::String => if is_64bit { 8 } else { 4 }, // BSTR pointer
            VbType::Variant => if is_64bit { 24 } else { 16 },
            VbType::Object => if is_64bit { 8 } else { 4 },
            VbType::Array(_) => if is_64bit { 8 } else { 4 }, // SAFEARRAY pointer
            VbType::UserDefined(_) => 0,
            VbType::Unknown => 0,
        }
    }

    /// Check if this type can be implicitly coerced/assigned to `target`.
    pub fn can_coerce_to(&self, target: &VbType) -> bool {
        if self == target {
            return true;
        }

        // Variant can hold and convert to/from any type dynamically
        if *self == VbType::Variant || *target == VbType::Variant {
            return true;
        }

        // Numeric types can coerce among themselves
        if self.is_numeric() && target.is_numeric() {
            return true;
        }

        // Booleans coerce to numeric (-1 / 0) and vice-versa
        if (*self == VbType::Boolean && target.is_numeric())
            || (self.is_numeric() && *target == VbType::Boolean)
        {
            return true;
        }

        // In static type checking, String cannot be directly coerced to numeric without runtime conversion function
        // Assigning String literal "Hola" to Long is a compile-time Type Mismatch error!
        false
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            VbType::Byte
                | VbType::Integer
                | VbType::Long
                | VbType::LongPtr
                | VbType::LongLong
                | VbType::Single
                | VbType::Double
                | VbType::Currency
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            VbType::Byte | VbType::Integer | VbType::Long | VbType::LongPtr | VbType::LongLong
        )
    }
}
