//! Variant data type and dynamic coercions for VB6 runtime.

#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    Empty,
    Null,
    Integer(i16),
    Long(i32),
    Single(f32),
    Double(f64),
    Currency(i64),
    Date(f64),
    String(String),
    Boolean(bool),
    Byte(u8),
    Error(i32),
}

impl Variant {
    pub fn is_empty(&self) -> bool {
        matches!(self, Variant::Empty)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Variant::Null)
    }
}
