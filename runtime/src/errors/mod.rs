//! Error handling (Err object, On Error, Resume).

#[derive(Debug, Default)]
pub struct ErrObject {
    pub number: i32,
    pub description: String,
    pub source: String,
}

impl ErrObject {
    pub fn clear(&mut self) {
        self.number = 0;
        self.description.clear();
        self.source.clear();
    }
}
