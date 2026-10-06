//! Symbol table and lexical scope management for VB6 identifiers.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Subroutine,
    Function,
    Constant,
    UserDefinedType,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub type_name: String,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    pub fn insert(&mut self, symbol: Symbol) {
        // VB6 is case-insensitive
        self.symbols.insert(symbol.name.to_ascii_uppercase(), symbol);
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(&name.to_ascii_uppercase())
    }
}
