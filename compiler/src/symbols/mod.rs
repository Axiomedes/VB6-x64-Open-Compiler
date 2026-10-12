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
pub struct ParameterInfo {
    pub name: String,
    pub vb_type: VbType,
    pub is_by_val: bool,
    pub is_optional: bool,
    pub is_param_array: bool,
    pub has_default: bool,
}

impl ParameterInfo {
    pub fn new(name: impl Into<String>, vb_type: VbType) -> Self {
        Self {
            name: name.into(),
            vb_type,
            is_by_val: false,
            is_optional: false,
            is_param_array: false,
            has_default: false,
        }
    }

    pub fn by_val(mut self) -> Self {
        self.is_by_val = true;
        self
    }

    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    pub fn param_array(mut self) -> Self {
        self.is_param_array = true;
        self
    }
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
        is_param_array: bool,
    },
    Procedure {
        kind: ProcKind,
        params: Vec<ParameterInfo>,
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
                params: vec![
                    ParameterInfo::new("Prompt", VbType::Variant),
                    ParameterInfo::new("Buttons", VbType::Long).optional(),
                    ParameterInfo::new("Title", VbType::Variant).optional(),
                    ParameterInfo::new("HelpFile", VbType::Variant).optional(),
                    ParameterInfo::new("Context", VbType::Variant).optional(),
                ],
                return_type: Some(VbType::Integer),
            },
            vb_type: VbType::Integer,
            visibility: Visibility::Public,
            span: Span::default(),
        };
        self.scopes[0]
            .symbols
            .insert("MSGBOX".to_string(), msgbox_symbol);

        let doevents_symbol = Symbol {
            name: "DoEvents".to_string(),
            kind: SymbolKind::Procedure {
                kind: ProcKind::Function,
                params: Vec::new(),
                return_type: Some(VbType::Long),
            },
            vb_type: VbType::Long,
            visibility: Visibility::Public,
            span: Span::default(),
        };
        self.scopes[0]
            .symbols
            .insert("DOEVENTS".to_string(), doevents_symbol);

        let len_symbol = Symbol {
            name: "Len".to_string(),
            kind: SymbolKind::Procedure {
                kind: ProcKind::Function,
                params: vec![ParameterInfo::new("Expression", VbType::Variant)],
                return_type: Some(VbType::Long),
            },
            vb_type: VbType::Long,
            visibility: Visibility::Public,
            span: Span::default(),
        };
        self.scopes[0]
            .symbols
            .insert("LEN".to_string(), len_symbol);

        let rt_funcs = [
            ("vb6_form_create", Some(VbType::LongLong)),
            ("vb6_form_show", None),
            ("vb6_form_hide", None),
            ("vb6_form_close", None),
            ("vb6_do_events", Some(VbType::Long)),
            ("vb6_control_create_button", Some(VbType::LongLong)),
            ("vb6_control_create_textbox", Some(VbType::LongLong)),
            ("vb6_control_create_label", Some(VbType::LongLong)),
            ("vb6_control_create_checkbox", Some(VbType::LongLong)),
            ("vb6_control_create_option", Some(VbType::LongLong)),
            ("vb6_control_create_frame", Some(VbType::LongLong)),
            ("vb6_control_create_timer", Some(VbType::LongLong)),
            ("vb6_control_set_text", None),
            ("vb6_control_get_text", Some(VbType::String)),
            ("vb6_control_set_value", None),
            ("vb6_control_get_value", Some(VbType::Long)),
            ("vb6_control_set_bounds", None),
            ("vb6_control_set_visible", None),
            ("vb6_control_set_enabled", None),
            ("vb6_register_event_handler", None),
            ("vb6_trigger_event", None),
            ("vb6_control_trigger_click", None),
            ("Load", None),
            ("Unload", None),
        ];
        for (f_name, ret_ty) in rt_funcs {
            let sym = Symbol {
                name: f_name.to_string(),
                kind: SymbolKind::Procedure {
                    kind: if ret_ty.is_some() { ProcKind::Function } else { ProcKind::Sub },
                    params: vec![ParameterInfo::new("args", VbType::Variant).param_array()],
                    return_type: ret_ty.clone(),
                },
                vb_type: ret_ty.unwrap_or(VbType::Variant),
                visibility: Visibility::Public,
                span: Span::default(),
            };
            self.scopes[0].symbols.insert(f_name.to_ascii_uppercase(), sym);
        }
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
            // In VB6, a global variable can have the same name as a class (e.g. Form default instance: Public Form1 As Form1)
            if matches!(existing.kind, SymbolKind::Class) && matches!(symbol.kind, SymbolKind::Variable { .. }) {
                self.scopes[self.current_scope].symbols.insert(key, symbol);
                return Ok(());
            }
            if matches!(existing.kind, SymbolKind::Variable { .. }) && matches!(symbol.kind, SymbolKind::Class) {
                return Ok(());
            }
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
