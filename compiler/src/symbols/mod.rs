//! Hierarchical symbol table and lexical scope management for VB6.

use crate::ast::decl::Visibility;
use crate::ast::span::Span;
use crate::types::VbType;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Module,
    Procedure,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcKind {
    Sub,
    Function,
    PropertyGet,
    PropertyLet,
    PropertySet,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable {
        is_const: bool,
        is_static: bool,
    },
    Parameter {
        is_by_val: bool,
        is_optional: bool,
    },
    Procedure {
        kind: ProcKind,
        params: Vec<(String, VbType)>,
        return_type: Option<VbType>,
    },
    Class,
    UserDefinedType,
    Enum,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub vb_type: VbType,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<usize>,
    pub symbols: HashMap<String, Symbol>,
}

#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope: usize,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        let global_scope = Scope {
            kind: ScopeKind::Global,
            parent: None,
            symbols: HashMap::new(),
        };

        let mut table = Self {
            scopes: vec![global_scope],
            current_scope: 0,
        };

        table.register_builtins();
        table
    }

    fn register_builtins(&mut self) {
        // Register common built-in procedures
        let msgbox_symbol = Symbol {
            name: "MsgBox".to_string(),
            kind: SymbolKind::Procedure {
                kind: ProcKind::Function,
                params: vec![("Prompt".to_string(), VbType::Variant)],
                return_type: Some(VbType::Integer),
            },
            vb_type: VbType::Integer,
            visibility: Visibility::Public,
            span: Span::default(),
        };
        self.scopes[0]
            .symbols
            .insert("MSGBOX".to_string(), msgbox_symbol);
    }

    pub fn enter_scope(&mut self, kind: ScopeKind) -> usize {
        let new_scope = Scope {
            kind,
            parent: Some(self.current_scope),
            symbols: HashMap::new(),
        };
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
        self.current_scope
    }

    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    /// Define a symbol in the current scope. Returns Err if duplicate in same scope.
    pub fn define(&mut self, symbol: Symbol) -> Result<(), Symbol> {
        let key = symbol.name.to_ascii_uppercase();
        if let Some(existing) = self.scopes[self.current_scope].symbols.get(&key) {
            return Err(existing.clone());
        }
        self.scopes[self.current_scope].symbols.insert(key, symbol);
        Ok(())
    }

    /// Lookup a symbol in current scope or ascending parent scopes.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let key = name.to_ascii_uppercase();
        let mut curr = Some(self.current_scope);
        while let Some(scope_idx) = curr {
            let scope = &self.scopes[scope_idx];
            if let Some(sym) = scope.symbols.get(&key) {
                return Some(sym);
            }
            curr = scope.parent;
        }
        None
    }

    /// Lookup a symbol only in the current scope.
    pub fn lookup_current_only(&self, name: &str) -> Option<&Symbol> {
        let key = name.to_ascii_uppercase();
        self.scopes[self.current_scope].symbols.get(&key)
    }
}
