//! Visual Basic 6.0 Type System and coercion rules.

pub mod coercion;
pub mod enum_def;
pub mod udt;

pub use coercion::*;
pub use enum_def::*;
pub use udt::*;

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
    FixedString(usize), // Fixed-length string: String * n
    Variant,
    Object,
    Array(Box<VbType>),
    UserDefined(String),
    Enum(String),
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
            VbType::FixedString(len) => *len,
            VbType::Variant => if is_64bit { 24 } else { 16 },
            VbType::Object => if is_64bit { 8 } else { 4 }, // Interface pointer
            VbType::Array(_) => if is_64bit { 8 } else { 4 }, // SAFEARRAY pointer
            VbType::Enum(_) => 4,
            VbType::UserDefined(_) => 0, // Computed dynamically via UdtLayout
            VbType::Unknown => 0,
        }
    }

    /// Return natural memory alignment in bytes on the target architecture.
    pub fn alignment(&self, is_64bit: bool) -> usize {
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
            VbType::String => if is_64bit { 8 } else { 4 },
            VbType::FixedString(_) => 1,
            VbType::Variant => 8,
            VbType::Object => if is_64bit { 8 } else { 4 },
            VbType::Array(_) => if is_64bit { 8 } else { 4 },
            VbType::Enum(_) => 4,
            VbType::UserDefined(_) => 1,
            VbType::Unknown => 1,
        }
    }

    /// Check if this type can be implicitly coerced/assigned to `target`.
    pub fn can_coerce_to(&self, target: &VbType) -> bool {
        let kind = check_coercion(self, target);
        kind != CoercionKind::Incompatible
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
                | VbType::Enum(_)
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            VbType::Byte
                | VbType::Integer
                | VbType::Long
                | VbType::LongPtr
                | VbType::LongLong
                | VbType::Enum(_)
        )
    }

    pub fn is_floating(&self) -> bool {
        matches!(self, VbType::Single | VbType::Double | VbType::Date)
    }

    pub fn is_reference_type(&self) -> bool {
        matches!(self, VbType::Object | VbType::Array(_))
    }

    /// Return integer value bounds `(min, max)` for integer types.
    pub fn integer_bounds(&self) -> Option<(i64, i64)> {
        match self {
            VbType::Byte => Some((0, 255)),
            VbType::Integer => Some((-32768, 32767)),
            VbType::Long => Some((-2147483648, 2147483647)),
            VbType::LongLong => Some((i64::MIN, i64::MAX)),
            VbType::Enum(_) => Some((-2147483648, 2147483647)),
            _ => None,
        }
    }

    /// Check if an integer value fits within this type without overflow.
    pub fn can_hold_integer(&self, val: i64) -> bool {
        if let Some((min, max)) = self.integer_bounds() {
            val >= min && val <= max
        } else {
            false
        }
    }

    /// Default literal value string representation.
    pub fn default_value_str(&self) -> &'static str {
        match self {
            VbType::Byte | VbType::Integer | VbType::Long | VbType::LongPtr | VbType::LongLong => "0",
            VbType::Single | VbType::Double => "0.0",
            VbType::Currency => "0.0000",
            VbType::Date => "1899-12-30",
            VbType::Boolean => "False",
            VbType::String | VbType::FixedString(_) => "\"\"",
            VbType::Variant => "Empty",
            VbType::Object => "Nothing",
            VbType::Array(_) => "Null",
            _ => "0",
        }
    }
}
