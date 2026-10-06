//! String manipulation functions for VB6 runtime (BSTR / UTF-16).

pub fn left(s: &str, length: usize) -> String {
    s.chars().take(length).collect()
}

pub fn right(s: &str, length: usize) -> String {
    let count = s.chars().count();
    if length >= count {
        s.to_string()
    } else {
        s.chars().skip(count - length).collect()
    }
}

pub fn mid(s: &str, start_1_based: usize, length: Option<usize>) -> String {
    if start_1_based == 0 {
        return String::new();
    }
    let skip = start_1_based - 1;
    let chars = s.chars().skip(skip);
    if let Some(len) = length {
        chars.take(len).collect()
    } else {
        chars.collect()
    }
}
