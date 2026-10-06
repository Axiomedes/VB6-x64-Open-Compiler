//! Visual Basic 6.0 Type System definitions.

#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl VbType {
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
            VbType::UserDefined(_) => 0, // Determined by UDT fields
        }
    }
}
