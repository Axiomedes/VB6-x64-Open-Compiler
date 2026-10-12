//! Semantic Analysis, Type Resolution, and Validation for VB6 AST.

use crate::ast::decl::*;
use crate::ast::expr::*;
use crate::ast::span::Span;
use crate::ast::stmt::*;
use crate::ast::SourceFile;
use crate::symbols::*;
use crate::types::*;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum SemanticError {
    #[error("Type mismatch: cannot assign/coerce '{found:?}' to expected type '{expected:?}' at line {}, col {}", span.start_line, span.start_col)]
    TypeMismatch {
        expected: VbType,
        found: VbType,
        span: Span,
    },
    #[error("Variable or identifier '{name}' is not declared at line {}, col {}", span.start_line, span.start_col)]
    UndeclaredIdentifier { name: String, span: Span },
    #[error("Duplicate declaration of '{name}' at line {}, col {}", duplicate_span.start_line, duplicate_span.start_col)]
    DuplicateSymbol {
        name: String,
        original_span: Span,
        duplicate_span: Span,
    },
    #[error("Argument count mismatch in call to '{proc_name}': expected {expected}, found {found} at line {}, col {}", span.start_line, span.start_col)]
    ArgumentCountMismatch {
        proc_name: String,
        expected: usize,
        found: usize,
        span: Span,
    },
    #[error("Invalid assignment target at line {}, col {}", span.start_line, span.start_col)]
    InvalidAssignmentTarget { span: Span },
    #[error("Overflow: value '{value}' is out of range for type '{target_type:?}' at line {}, col {}", span.start_line, span.start_col)]
    Overflow {
        value: String,
        target_type: VbType,
        span: Span,
    },
    #[error("Optional parameter '{param_name}' must be followed by optional parameter or ParamArray at line {}, col {}", span.start_line, span.start_col)]
    OptionalParamMustFollowRequired {
        param_name: String,
        span: Span,
    },
    #[error("ParamArray parameter must be the last parameter in the list at line {}, col {}", span.start_line, span.start_col)]
    ParamArrayMustBeLast {
        span: Span,
    },
    #[error("ParamArray parameter cannot be passed ByVal at line {}, col {}", span.start_line, span.start_col)]
    ParamArrayCannotBeByVal {
        span: Span,
    },
    #[error("ParamArray parameter cannot be Optional at line {}, col {}", span.start_line, span.start_col)]
    ParamArrayCannotBeOptional {
        span: Span,
    },
    #[error("ByRef type mismatch for argument '{param_name}': expected variable of type '{expected:?}', found '{found:?}' at line {}, col {}", span.start_line, span.start_col)]
    ByRefTypeMismatch {
        param_name: String,
        expected: VbType,
        found: VbType,
        span: Span,
    },
    #[error("Object required at line {}, col {}", span.start_line, span.start_col)]
    ObjectRequired {
        span: Span,
    },
    #[error("Exit {target} not within {target} loop at line {}, col {}", span.start_line, span.start_col)]
    InvalidExitStatement {
        target: String,
        span: Span,
    },
}

pub struct SemanticAnalyzer {
    pub symbol_table: SymbolTable,
    pub errors: Vec<SemanticError>,
    pub option_explicit: bool,
    pub udt_layouts: HashMap<String, crate::types::UdtLayout>,
    pub enum_defs: HashMap<String, crate::types::EnumDefinition>,
    pub class_layouts: HashMap<String, crate::types::ClassLayout>,
    pub current_class: Option<String>,
    pub with_stack: Vec<VbType>,
    loop_depth_for: usize,
    loop_depth_do: usize,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
            option_explicit: false,
            udt_layouts: HashMap::new(),
            enum_defs: HashMap::new(),
            class_layouts: HashMap::new(),
            current_class: None,
            with_stack: Vec::new(),
            loop_depth_for: 0,
            loop_depth_do: 0,
        }
    }

    pub fn resolve_type_spec(&self, ts: &crate::ast::types::TypeSpec) -> VbType {
        let key = ts.name.to_ascii_uppercase();
        if self.enum_defs.contains_key(&key) {
            VbType::Enum(ts.name.clone())
        } else if self.udt_layouts.contains_key(&key) {
            VbType::UserDefined(ts.name.clone())
        } else if self.class_layouts.contains_key(&key) {
            VbType::Class(ts.name.clone())
        } else if let Some(len) = ts.fixed_length {
            if ts.name.eq_ignore_ascii_case("String") {
                VbType::FixedString(len)
            } else {
                VbType::from_name(&ts.name)
            }
        } else {
            VbType::from_name(&ts.name)
        }
    }

    pub fn analyze_file(&mut self, file: &SourceFile) -> Result<(), Vec<SemanticError>> {
        self.symbol_table.enter_scope(ScopeKind::Module);

        // Pre-pass 0: register all Class declarations so their names and member layouts are known
        for decl in &file.declarations {
            if let Declaration::Class(c) = decl {
                let mut raw_fields = Vec::new();
                for inner in &c.declarations {
                    if let Declaration::ModuleVariables(mv) = inner {
                        for var in &mv.variables {
                            let mut f_type = var
                                .type_spec
                                .as_ref()
                                .map(|t| self.resolve_type_spec(t))
                                .unwrap_or(VbType::Variant);
                            if var.dimensions.is_some() {
                                f_type = VbType::Array(Box::new(f_type));
                            }
                            raw_fields.push((var.name.clone(), f_type, mv.visibility == Visibility::Public));
                        }
                    }
                }
                let mut layout = crate::types::ClassLayout::compute(&c.name, &raw_fields, &self.udt_layouts, true);
                for inner in &c.declarations {
                    match inner {
                        Declaration::Sub(s) => {
                            if s.name.eq_ignore_ascii_case("Class_Initialize") {
                                layout.has_initialize = true;
                            } else if s.name.eq_ignore_ascii_case("Class_Terminate") {
                                layout.has_terminate = true;
                            }
                            let params = self.validate_and_convert_parameters(&s.params, s.span);
                            let param_types = params.into_iter().map(|p| p.vb_type).collect();
                            layout.methods.insert(
                                s.name.to_ascii_uppercase(),
                                crate::types::ClassMethod {
                                    name: s.name.clone(),
                                    mangled_name: format!("{}__Function_{}", c.name, s.name),
                                    is_sub: true,
                                    is_public: s.visibility == Visibility::Public,
                                    return_type: None,
                                    param_types,
                                },
                            );
                        }
                        Declaration::Function(f) => {
                            let ret_ty = f.return_type.as_ref().map(|t| self.resolve_type_spec(t)).unwrap_or(VbType::Variant);
                            let params = self.validate_and_convert_parameters(&f.params, f.span);
                            let param_types = params.into_iter().map(|p| p.vb_type).collect();
                            layout.methods.insert(
                                f.name.to_ascii_uppercase(),
                                crate::types::ClassMethod {
                                    name: f.name.clone(),
                                    mangled_name: format!("{}__Function_{}", c.name, f.name),
                                    is_sub: false,
                                    is_public: f.visibility == Visibility::Public,
                                    return_type: Some(ret_ty),
                                    param_types,
                                },
                            );
                        }
                        Declaration::Property(p) => {
                            let prop_type = p.return_type.as_ref().map(|t| self.resolve_type_spec(t)).unwrap_or(VbType::Variant);
                            let entry = layout.properties.entry(p.name.to_ascii_uppercase()).or_insert_with(|| {
                                crate::types::ClassProperty {
                                    name: p.name.clone(),
                                    has_get: false,
                                    has_let: false,
                                    has_set: false,
                                    is_public: p.visibility == Visibility::Public,
                                    prop_type: prop_type.clone(),
                                }
                            });
                            match p.kind {
                                PropertyKind::Get => entry.has_get = true,
                                PropertyKind::Let => entry.has_let = true,
                                PropertyKind::Set => entry.has_set = true,
                            }
                        }
                        _ => {}
                    }
                }
                self.class_layouts.insert(c.name.to_ascii_uppercase(), layout);
            }
        }

        // Preliminary pass: register all Type and Enum declarations first
        for decl in &file.declarations {
            if matches!(decl, Declaration::Type(_) | Declaration::Enum(_) | Declaration::Class(_)) {
                self.declare_symbol(decl);
            }
        }

        // First pass: register procedure headers and module-level symbols
        for decl in &file.declarations {
            if !matches!(decl, Declaration::Type(_) | Declaration::Enum(_) | Declaration::Class(_)) {
                self.declare_symbol(decl);
            }
        }

        // Second pass: analyze bodies, statements, and expressions
        for decl in &file.declarations {
            self.analyze_declaration(decl);
        }

        self.symbol_table.exit_scope();

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn validate_and_convert_parameters(
        &mut self,
        params: &[Parameter],
        proc_span: Span,
    ) -> Vec<ParameterInfo> {
        let mut param_infos = Vec::new();
        let mut seen_optional = false;

        for (idx, p) in params.iter().enumerate() {
            let is_last = idx == params.len() - 1;

            if p.is_param_array {
                if !is_last {
                    self.errors
                        .push(SemanticError::ParamArrayMustBeLast { span: proc_span });
                }
                if p.is_by_val {
                    self.errors
                        .push(SemanticError::ParamArrayCannotBeByVal { span: proc_span });
                }
                if p.is_optional {
                    self.errors
                        .push(SemanticError::ParamArrayCannotBeOptional { span: proc_span });
                }
            }

            if p.is_optional {
                seen_optional = true;
            } else if seen_optional && !p.is_param_array {
                self.errors
                    .push(SemanticError::OptionalParamMustFollowRequired {
                        param_name: p.name.clone(),
                        span: proc_span,
                    });
            }

            let mut ty = p
                .param_type
                .as_ref()
                .map(|t| self.resolve_type_spec(t))
                .unwrap_or(VbType::Variant);
            if let Some(ref pt) = p.param_type {
                if !pt.array_dimensions.is_empty() {
                    ty = VbType::Array(Box::new(ty));
                }
            }

            param_infos.push(ParameterInfo {
                name: p.name.clone(),
                vb_type: ty,
                is_by_val: p.is_by_val,
                is_optional: p.is_optional,
                is_param_array: p.is_param_array,
                has_default: p.default_value.is_some(),
            });
        }

        param_infos
    }

    fn declare_symbol(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Sub(s) => {
                let params = self.validate_and_convert_parameters(&s.params, s.span);
                let sym = Symbol {
                    name: s.name.clone(),
                    kind: SymbolKind::Procedure {
                        kind: ProcKind::Sub,
                        params,
                        return_type: None,
                    },
                    vb_type: VbType::Unknown,
                    visibility: s.visibility,
                    span: s.span,
                };
                if let Err(orig) = self.symbol_table.define(sym) {
                    self.errors.push(SemanticError::DuplicateSymbol {
                        name: s.name.clone(),
                        original_span: orig.span,
                        duplicate_span: s.span,
                    });
                }
            }
            Declaration::Function(f) => {
                let params = self.validate_and_convert_parameters(&f.params, f.span);
                let ret_ty = f
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_type_spec(t))
                    .unwrap_or(VbType::Variant);

                let sym = Symbol {
                    name: f.name.clone(),
                    kind: SymbolKind::Procedure {
                        kind: ProcKind::Function,
                        params,
                        return_type: Some(ret_ty.clone()),
                    },
                    vb_type: ret_ty,
                    visibility: f.visibility,
                    span: f.span,
                };
                if let Err(orig) = self.symbol_table.define(sym) {
                    self.errors.push(SemanticError::DuplicateSymbol {
                        name: f.name.clone(),
                        original_span: orig.span,
                        duplicate_span: f.span,
                    });
                }
            }
            Declaration::ModuleVariables(v) => {
                for var in &v.variables {
                    let mut vb_type = var
                        .type_spec
                        .as_ref()
                        .map(|t| self.resolve_type_spec(t))
                        .unwrap_or(VbType::Variant);
                    if var.dimensions.is_some() {
                        vb_type = VbType::Array(Box::new(vb_type));
                    }

                    let sym = Symbol {
                        name: var.name.clone(),
                        kind: SymbolKind::Variable {
                            is_const: v.is_const,
                            is_static: false,
                        },
                        vb_type,
                        visibility: v.visibility,
                        span: v.span,
                    };
                    if let Err(orig) = self.symbol_table.define(sym) {
                        self.errors.push(SemanticError::DuplicateSymbol {
                            name: var.name.clone(),
                            original_span: orig.span,
                            duplicate_span: v.span,
                        });
                    }
                }
            }
            Declaration::Type(t) => {
                let raw_members: Vec<(String, VbType)> = t
                    .members
                    .iter()
                    .map(|m| (m.name.clone(), self.resolve_type_spec(&m.type_spec)))
                    .collect();
                let layout = crate::types::UdtLayout::compute_with_layouts(&t.name, &raw_members, &self.udt_layouts, true);
                self.udt_layouts.insert(t.name.to_ascii_uppercase(), layout);
                let sym = Symbol {
                    name: t.name.clone(),
                    kind: SymbolKind::UserDefinedType,
                    vb_type: VbType::UserDefined(t.name.clone()),
                    visibility: t.visibility,
                    span: t.span,
                };
                let _ = self.symbol_table.define(sym);
            }
            Declaration::Enum(e) => {
                let raw_members: Vec<(String, Option<i32>)> = e
                    .members
                    .iter()
                    .map(|m| (m.name.clone(), m.value))
                    .collect();
                let enum_def = crate::types::EnumDefinition::new(&e.name, &raw_members);
                let sym = Symbol {
                    name: e.name.clone(),
                    kind: SymbolKind::Enum,
                    vb_type: VbType::Enum(e.name.clone()),
                    visibility: e.visibility,
                    span: e.span,
                };
                let _ = self.symbol_table.define(sym);

                // Register enum members as Long constants in scope
                for m in &enum_def.members {
                    let member_sym = Symbol {
                        name: m.name.clone(),
                        kind: SymbolKind::Variable {
                            is_const: true,
                            is_static: false,
                        },
                        vb_type: VbType::Long,
                        visibility: e.visibility,
                        span: e.span,
                    };
                    let _ = self.symbol_table.define(member_sym);
                }
            }
            Declaration::Class(c) => {
                let sym = Symbol {
                    name: c.name.clone(),
                    kind: SymbolKind::Class,
                    vb_type: VbType::Class(c.name.clone()),
                    visibility: c.visibility,
                    span: c.span,
                };
                let _ = self.symbol_table.define(sym);
            }
            Declaration::Property(p) => {
                let params = self.validate_and_convert_parameters(&p.params, p.span);
                let ret_ty = p
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_type_spec(t))
                    .unwrap_or(VbType::Variant);
                let proc_kind = match p.kind {
                    PropertyKind::Get => ProcKind::PropertyGet,
                    PropertyKind::Let => ProcKind::PropertyLet,
                    PropertyKind::Set => ProcKind::PropertySet,
                };
                let sym = Symbol {
                    name: p.name.clone(),
                    kind: SymbolKind::Procedure {
                        kind: proc_kind,
                        params,
                        return_type: if p.kind == PropertyKind::Get {
                            Some(ret_ty.clone())
                        } else {
                            None
                        },
                    },
                    vb_type: ret_ty,
                    visibility: p.visibility,
                    span: p.span,
                };
                let _ = self.symbol_table.define(sym);
            }
            _ => {}
        }
    }

    fn analyze_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Sub(s) => {
                self.symbol_table.enter_scope(ScopeKind::Procedure);
                // Define parameters in procedure scope
                for p in &s.params {
                    let mut vb_type = p
                        .param_type
                        .as_ref()
                        .map(|t| self.resolve_type_spec(t))
                        .unwrap_or(VbType::Variant);
                    if let Some(ref pt) = p.param_type {
                        if !pt.array_dimensions.is_empty() {
                            vb_type = VbType::Array(Box::new(vb_type));
                        }
                    }
                    let sym = Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Parameter {
                            is_by_val: p.is_by_val,
                            is_optional: p.is_optional,
                            is_param_array: p.is_param_array,
                        },
                        vb_type,
                        visibility: Visibility::Default,
                        span: s.span,
                    };
                    let _ = self.symbol_table.define(sym);
                }

                for stmt in &s.body {
                    self.analyze_statement(stmt);
                }
                self.symbol_table.exit_scope();
            }
            Declaration::Function(f) => {
                self.symbol_table.enter_scope(ScopeKind::Procedure);
                for p in &f.params {
                    let mut vb_type = p
                        .param_type
                        .as_ref()
                        .map(|t| self.resolve_type_spec(t))
                        .unwrap_or(VbType::Variant);
                    if let Some(ref pt) = p.param_type {
                        if !pt.array_dimensions.is_empty() {
                            vb_type = VbType::Array(Box::new(vb_type));
                        }
                    }
                    let sym = Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Parameter {
                            is_by_val: p.is_by_val,
                            is_optional: p.is_optional,
                            is_param_array: p.is_param_array,
                        },
                        vb_type,
                        visibility: Visibility::Default,
                        span: f.span,
                    };
                    let _ = self.symbol_table.define(sym);
                }

                for stmt in &f.body {
                    self.analyze_statement(stmt);
                }
                self.symbol_table.exit_scope();
            }
            Declaration::Property(p) => {
                self.symbol_table.enter_scope(ScopeKind::Procedure);
                for p_param in &p.params {
                    let mut vb_type = p_param
                        .param_type
                        .as_ref()
                        .map(|t| self.resolve_type_spec(t))
                        .unwrap_or(VbType::Variant);
                    if let Some(ref pt) = p_param.param_type {
                        if !pt.array_dimensions.is_empty() {
                            vb_type = VbType::Array(Box::new(vb_type));
                        }
                    }
                    let sym = Symbol {
                        name: p_param.name.clone(),
                        kind: SymbolKind::Parameter {
                            is_by_val: p_param.is_by_val,
                            is_optional: p_param.is_optional,
                            is_param_array: p_param.is_param_array,
                        },
                        vb_type,
                        visibility: Visibility::Default,
                        span: p.span,
                    };
                    let _ = self.symbol_table.define(sym);
                }

                for stmt in &p.body {
                    self.analyze_statement(stmt);
                }
                self.symbol_table.exit_scope();
            }
            Declaration::Class(c) => {
                let prev_class = self.current_class.replace(c.name.clone());
                self.symbol_table.enter_scope(ScopeKind::Module);

                // Define 'Me' in class scope
                let me_sym = Symbol {
                    name: "Me".to_string(),
                    kind: SymbolKind::Variable {
                        is_const: true,
                        is_static: false,
                    },
                    vb_type: VbType::Class(c.name.clone()),
                    visibility: Visibility::Public,
                    span: c.span,
                };
                let _ = self.symbol_table.define(me_sym);

                // Define fields in class scope
                for inner in &c.declarations {
                    if let Declaration::ModuleVariables(mv) = inner {
                        for var in &mv.variables {
                            let mut vb_type = var
                                .type_spec
                                .as_ref()
                                .map(|t| self.resolve_type_spec(t))
                                .unwrap_or(VbType::Variant);
                            if var.dimensions.is_some() {
                                vb_type = VbType::Array(Box::new(vb_type));
                            }
                            let var_sym = Symbol {
                                name: var.name.clone(),
                                kind: SymbolKind::Variable {
                                    is_const: mv.is_const,
                                    is_static: false,
                                },
                                vb_type,
                                visibility: mv.visibility,
                                span: mv.span,
                            };
                            let _ = self.symbol_table.define(var_sym);
                        }
                    }
                }

                // Declare methods & properties in class scope
                for inner in &c.declarations {
                    match inner {
                        Declaration::Sub(_) | Declaration::Function(_) | Declaration::Property(_) => {
                            self.declare_symbol(inner);
                        }
                        _ => {}
                    }
                }

                // Analyze bodies of inner declarations
                for inner in &c.declarations {
                    self.analyze_declaration(inner);
                }

                self.symbol_table.exit_scope();
                self.current_class = prev_class;
            }
            _ => {}
        }
    }

    fn analyze_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration {
                kind,
                variables,
                span,
            } => {
                for v in variables {
                    let mut vb_type = v
                        .type_spec
                        .as_ref()
                        .map(|t| self.resolve_type_spec(t))
                        .unwrap_or(VbType::Variant);
                    if v.dimensions.is_some() {
                        vb_type = VbType::Array(Box::new(vb_type));
                    }

                    let sym = Symbol {
                        name: v.name.clone(),
                        kind: SymbolKind::Variable {
                            is_const: *kind == VarDeclKind::Const,
                            is_static: *kind == VarDeclKind::Static,
                        },
                        vb_type,
                        visibility: Visibility::Default,
                        span: *span,
                    };
                    if let Err(orig) = self.symbol_table.define(sym) {
                        if *kind != VarDeclKind::ReDim && *kind != VarDeclKind::ReDimPreserve {
                            self.errors.push(SemanticError::DuplicateSymbol {
                                name: v.name.clone(),
                                original_span: orig.span,
                                duplicate_span: *span,
                            });
                        }
                    }

                    if let Some(init) = &v.init_value {
                        let init_ty = self.infer_expression_type(init);
                        if let Some(ts) = &v.type_spec {
                            let expected = VbType::from_name(&ts.name);
                            if !init_ty.can_coerce_to(&expected) {
                                self.errors.push(SemanticError::TypeMismatch {
                                    expected,
                                    found: init_ty,
                                    span: *span,
                                });
                            }
                        }
                    }
                }
            }
            Statement::Assignment {
                target,
                value,
                is_set,
                span,
            } => {
                let target_ty = self.infer_expression_type(target);
                let value_ty = self.infer_expression_type(value);

                if *is_set {
                    if !target_ty.is_object_or_variant() && target_ty != VbType::Unknown {
                        self.errors.push(SemanticError::ObjectRequired {
                            span: *span,
                        });
                        return;
                    }
                    if !value_ty.is_object_or_variant() && value_ty != VbType::Unknown {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: VbType::Object,
                            found: value_ty,
                            span: *span,
                        });
                        return;
                    }
                }

                // Check integer literal overflow
                if let Expression::Literal(Literal::Integer(i)) = value {
                    if target_ty.is_integer() && !target_ty.can_hold_integer(*i as i64) {
                        self.errors.push(SemanticError::Overflow {
                            value: i.to_string(),
                            target_type: target_ty.clone(),
                            span: *span,
                        });
                    }
                }

                if target_ty != VbType::Unknown
                    && value_ty != VbType::Unknown
                    && !value_ty.can_coerce_to(&target_ty)
                {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: target_ty,
                        found: value_ty,
                        span: *span,
                    });
                }
            }
            Statement::If {
                branches,
                else_branch,
                ..
            } => {
                for b in branches {
                    let _ = self.infer_expression_type(&b.condition);
                    for s in &b.body {
                        self.analyze_statement(s);
                    }
                }
                if let Some(el) = else_branch {
                    for s in el {
                        self.analyze_statement(s);
                    }
                }
            }
            Statement::For {
                counter,
                start,
                end,
                step,
                body,
                span,
            } => {
                if let Some(sym) = self.symbol_table.lookup(counter) {
                    if !sym.vb_type.is_numeric() && sym.vb_type != VbType::Variant {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: VbType::Long,
                            found: sym.vb_type.clone(),
                            span: *span,
                        });
                    }
                } else if self.option_explicit {
                    self.errors.push(SemanticError::UndeclaredIdentifier {
                        name: counter.clone(),
                        span: *span,
                    });
                }

                let _ = self.infer_expression_type(start);
                let _ = self.infer_expression_type(end);
                if let Some(st) = step {
                    let _ = self.infer_expression_type(st);
                }

                self.loop_depth_for += 1;
                for s in body {
                    self.analyze_statement(s);
                }
                self.loop_depth_for -= 1;
            }
            Statement::DoLoop {
                condition, body, ..
            } => {
                if let Some(cond) = condition {
                    match cond {
                        LoopCondition::While(e) | LoopCondition::Until(e) => {
                            let _ = self.infer_expression_type(e);
                        }
                    }
                }
                self.loop_depth_do += 1;
                for s in body {
                    self.analyze_statement(s);
                }
                self.loop_depth_do -= 1;
            }
            Statement::WhileWend {
                condition, body, ..
            } => {
                let _ = self.infer_expression_type(condition);
                for s in body {
                    self.analyze_statement(s);
                }
            }
            Statement::SelectCase {
                expression,
                cases,
                else_case,
                ..
            } => {
                let _ = self.infer_expression_type(expression);
                for c in cases {
                    for v in &c.values {
                        match v {
                            crate::ast::stmt::CaseValue::Exact(e) => {
                                let _ = self.infer_expression_type(e);
                            }
                            crate::ast::stmt::CaseValue::Range(low, high) => {
                                let _ = self.infer_expression_type(low);
                                let _ = self.infer_expression_type(high);
                            }
                            crate::ast::stmt::CaseValue::Comparison(_, e) => {
                                let _ = self.infer_expression_type(e);
                            }
                        }
                    }
                    for s in &c.body {
                        self.analyze_statement(s);
                    }
                }
                if let Some(eb) = else_case {
                    for s in eb {
                        self.analyze_statement(s);
                    }
                }
            }
            Statement::Exit { target, span } => match target {
                crate::ast::stmt::ExitTarget::For => {
                    if self.loop_depth_for == 0 {
                        self.errors.push(SemanticError::InvalidExitStatement {
                            target: "For".to_string(),
                            span: *span,
                        });
                    }
                }
                crate::ast::stmt::ExitTarget::Do => {
                    if self.loop_depth_do == 0 {
                        self.errors.push(SemanticError::InvalidExitStatement {
                            target: "Do".to_string(),
                            span: *span,
                        });
                    }
                }
                _ => {}
            },
            Statement::Call {
                name,
                arguments,
                span,
                ..
            } => {
                let proc_params = if let Some(sym) = self.symbol_table.lookup(name) {
                    if let SymbolKind::Procedure { params, .. } = &sym.kind {
                        Some(params.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(params) = proc_params {
                    self.validate_procedure_call(name, &params, arguments, *span);
                } else if self.option_explicit && !is_intrinsic_function(name) {
                    self.errors.push(SemanticError::UndeclaredIdentifier {
                        name: name.clone(),
                        span: *span,
                    });
                }

                for arg in arguments {
                    let _ = self.infer_expression_type(arg);
                }
            }
            Statement::DebugPrint { arguments, .. } => {
                for arg in arguments {
                    let _ = self.infer_expression_type(arg);
                }
            }
            Statement::With { target, body, .. } => {
                let target_ty = self.infer_expression_type(target);
                self.with_stack.push(target_ty);
                for s in body {
                    self.analyze_statement(s);
                }
                self.with_stack.pop();
            }
            _ => {}
        }
    }

    fn validate_procedure_call(
        &mut self,
        proc_name: &str,
        params: &[ParameterInfo],
        arguments: &[Expression],
        span: Span,
    ) {
        let has_param_array = params.last().map(|p| p.is_param_array).unwrap_or(false);
        let min_args = params
            .iter()
            .filter(|p| !p.is_optional && !p.is_param_array)
            .count();
        let max_args = if has_param_array {
            usize::MAX
        } else {
            params.len()
        };

        if arguments.len() < min_args || arguments.len() > max_args {
            self.errors.push(SemanticError::ArgumentCountMismatch {
                proc_name: proc_name.to_string(),
                expected: min_args,
                found: arguments.len(),
                span,
            });
            return;
        }

        for (idx, arg) in arguments.iter().enumerate() {
            let param = if idx < params.len() {
                &params[idx]
            } else if has_param_array {
                params.last().unwrap()
            } else {
                break;
            };

            if param.is_param_array && idx >= params.len() - 1 {
                let arg_ty = self.infer_expression_type(arg);
                let elem_type = match &param.vb_type {
                    VbType::Array(elem) => &**elem,
                    _ => &VbType::Variant,
                };
                if !arg_ty.can_coerce_to(elem_type) && arg_ty != VbType::Unknown {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: elem_type.clone(),
                        found: arg_ty,
                        span,
                    });
                }
                continue;
            }

            let arg_ty = self.infer_expression_type(arg);

            if !arg_ty.can_coerce_to(&param.vb_type) && arg_ty != VbType::Unknown {
                self.errors.push(SemanticError::TypeMismatch {
                    expected: param.vb_type.clone(),
                    found: arg_ty,
                    span,
                });
            } else if !param.is_by_val && !param.is_param_array {
                // If a variable is passed ByRef, its type must match the parameter type in VB6
                if let Expression::Variable(_) = arg {
                    if arg_ty != param.vb_type
                        && param.vb_type != VbType::Variant
                        && arg_ty != VbType::Variant
                    {
                        self.errors.push(SemanticError::ByRefTypeMismatch {
                            param_name: param.name.clone(),
                            expected: param.vb_type.clone(),
                            found: arg_ty,
                            span,
                        });
                    }
                }
            }
        }
    }

    pub fn infer_expression_type(&mut self, expr: &Expression) -> VbType {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Integer(_) => VbType::Integer,
                Literal::Double(_) => VbType::Double,
                Literal::String(_) => VbType::String,
                Literal::Boolean(_) => VbType::Boolean,
                Literal::Date(_) => VbType::Date,
                Literal::Null | Literal::Empty => VbType::Variant,
                Literal::Nothing => VbType::Object,
            },
            Expression::Variable(name) => {
                if let Some(sym) = self.symbol_table.lookup(name) {
                    sym.vb_type.clone()
                } else if self.option_explicit {
                    self.errors.push(SemanticError::UndeclaredIdentifier {
                        name: name.clone(),
                        span: Span::default(),
                    });
                    VbType::Unknown
                } else {
                    // Undeclared variable in VB6 without Option Explicit is Variant
                    VbType::Variant
                }
            }
            Expression::IndexAccess { target, indices } => {
                if let Expression::Variable(name) = &**target {
                    if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Long;
                    }
                    if name.eq_ignore_ascii_case("Len") || name.eq_ignore_ascii_case("InStr") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Long;
                    }
                    if name.eq_ignore_ascii_case("Asc") || name.eq_ignore_ascii_case("AscB") || name.eq_ignore_ascii_case("AscW") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Integer;
                    }
                    if name.eq_ignore_ascii_case("Val") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Double;
                    }
                    if name.eq_ignore_ascii_case("Left") || name.eq_ignore_ascii_case("Left$")
                        || name.eq_ignore_ascii_case("Right") || name.eq_ignore_ascii_case("Right$")
                        || name.eq_ignore_ascii_case("Mid") || name.eq_ignore_ascii_case("Mid$")
                        || name.eq_ignore_ascii_case("UCase") || name.eq_ignore_ascii_case("UCase$")
                        || name.eq_ignore_ascii_case("LCase") || name.eq_ignore_ascii_case("LCase$")
                        || name.eq_ignore_ascii_case("Trim") || name.eq_ignore_ascii_case("Trim$")
                        || name.eq_ignore_ascii_case("LTrim") || name.eq_ignore_ascii_case("LTrim$")
                        || name.eq_ignore_ascii_case("RTrim") || name.eq_ignore_ascii_case("RTrim$")
                        || name.eq_ignore_ascii_case("Space") || name.eq_ignore_ascii_case("Space$")
                        || name.eq_ignore_ascii_case("String") || name.eq_ignore_ascii_case("String$")
                        || name.eq_ignore_ascii_case("Replace")
                        || name.eq_ignore_ascii_case("Chr") || name.eq_ignore_ascii_case("Chr$")
                        || name.eq_ignore_ascii_case("ChrW") || name.eq_ignore_ascii_case("ChrW$")
                        || name.eq_ignore_ascii_case("Str") || name.eq_ignore_ascii_case("Str$")
                    {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::String;
                    }
                    if name.eq_ignore_ascii_case("VarType") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Integer;
                    }
                    if name.eq_ignore_ascii_case("TypeName") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::String;
                    }
                    if name.eq_ignore_ascii_case("IsEmpty")
                        || name.eq_ignore_ascii_case("IsNull")
                        || name.eq_ignore_ascii_case("IsNumeric")
                        || name.eq_ignore_ascii_case("IsDate")
                        || name.eq_ignore_ascii_case("IsArray")
                    {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Boolean;
                    }
                    if name.eq_ignore_ascii_case("CBool") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Boolean;
                    }
                    if name.eq_ignore_ascii_case("CByte") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Byte;
                    }
                    if name.eq_ignore_ascii_case("CInt") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Integer;
                    }
                    if name.eq_ignore_ascii_case("CLng") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Long;
                    }
                    if name.eq_ignore_ascii_case("CSng") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Single;
                    }
                    if name.eq_ignore_ascii_case("CDbl") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Double;
                    }
                    if name.eq_ignore_ascii_case("CStr") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::String;
                    }
                    if name.eq_ignore_ascii_case("CDate") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Date;
                    }
                    if name.eq_ignore_ascii_case("CCur") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Currency;
                    }
                    if name.eq_ignore_ascii_case("CVar") {
                        for idx in indices {
                            let _ = self.infer_expression_type(idx);
                        }
                        return VbType::Variant;
                    }
                }

                let proc_info = if let Expression::Variable(name) = &**target {
                    if let Some(sym) = self.symbol_table.lookup(name) {
                        match &sym.kind {
                            SymbolKind::Procedure {
                                params,
                                return_type,
                                ..
                            } => Some((name.clone(), params.clone(), return_type.clone())),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((proc_name, params, return_type)) = proc_info {
                    self.validate_procedure_call(
                        &proc_name,
                        &params,
                        indices,
                        Span::default(),
                    );
                    return return_type.unwrap_or(VbType::Variant);
                }

                let array_elem = if let Expression::Variable(name) = &**target {
                    if let Some(sym) = self.symbol_table.lookup(name) {
                        if let VbType::Array(elem) = &sym.vb_type {
                            Some(*elem.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                for idx in indices {
                    let _ = self.infer_expression_type(idx);
                }

                if let Some(elem) = array_elem {
                    return elem;
                }

                VbType::Variant
            }
            Expression::BinaryOp { op, left, right } => {
                let left_ty = self.infer_expression_type(left);
                let right_ty = self.infer_expression_type(right);

                crate::types::promote_binary_operator(&left_ty, op, &right_ty)
                    .unwrap_or(VbType::Variant)
            }
            Expression::UnaryOp { op, operand } => {
                let ty = self.infer_expression_type(operand);
                match op {
                    UnaryOperator::Not => {
                        if ty == VbType::Boolean {
                            VbType::Boolean
                        } else {
                            ty
                        }
                    }
                    UnaryOperator::Negate => {
                        if ty == VbType::Byte {
                            VbType::Integer
                        } else {
                            ty
                        }
                    }
                    UnaryOperator::Plus => ty,
                }
            }
            Expression::Paren(inner) => self.infer_expression_type(inner),
            Expression::New(cls_name) => VbType::Class(cls_name.clone()),
            Expression::Me => {
                if let Some(ref cls) = self.current_class {
                    VbType::Class(cls.clone())
                } else {
                    VbType::Object
                }
            }
            Expression::MemberAccess { object, member } => {
                let obj_ty = self.infer_expression_type(object);
                if let VbType::UserDefined(ref udt_name) = obj_ty {
                    if let Some(layout) = self.udt_layouts.get(&udt_name.to_ascii_uppercase()) {
                        if let Some(mem) = layout.find_member(member) {
                            return mem.vb_type.clone();
                        }
                    }
                } else if let VbType::Class(ref cls_name) = obj_ty {
                    if let Some(layout) = self.class_layouts.get(&cls_name.to_ascii_uppercase()) {
                        if let Some(f) = layout.find_field(member) {
                            return f.vb_type.clone();
                        }
                        if let Some(p) = layout.find_property(member) {
                            return p.prop_type.clone();
                        }
                        if let Some(m) = layout.find_method(member) {
                            return m.return_type.clone().unwrap_or(VbType::Variant);
                        }
                    }
                }
                VbType::Variant
            }
            Expression::WithMemberAccess(member) => {
                if let Some(top_ty) = self.with_stack.last() {
                    if let VbType::UserDefined(ref udt_name) = top_ty {
                        if let Some(layout) = self.udt_layouts.get(&udt_name.to_ascii_uppercase()) {
                            if let Some(mem) = layout.find_member(member) {
                                return mem.vb_type.clone();
                            }
                        }
                    } else if let VbType::Class(ref cls_name) = top_ty {
                        if let Some(layout) = self.class_layouts.get(&cls_name.to_ascii_uppercase()) {
                            if let Some(f) = layout.find_field(member) {
                                return f.vb_type.clone();
                            }
                            if let Some(p) = layout.find_property(member) {
                                return p.prop_type.clone();
                            }
                            if let Some(m) = layout.find_method(member) {
                                return m.return_type.clone().unwrap_or(VbType::Variant);
                            }
                        }
                    }
                }
                VbType::Variant
            }
        }
    }
}

pub fn is_intrinsic_function(name: &str) -> bool {
    let clean = name.trim_end_matches('$');
    matches!(
        clean.to_ascii_uppercase().as_str(),
        "LBOUND"
            | "UBOUND"
            | "LEN"
            | "INSTR"
            | "ASC"
            | "ASCB"
            | "ASCW"
            | "VAL"
            | "LEFT"
            | "RIGHT"
            | "MID"
            | "UCASE"
            | "LCASE"
            | "TRIM"
            | "LTRIM"
            | "RTRIM"
            | "SPACE"
            | "STRING"
            | "REPLACE"
            | "CHR"
            | "CHRW"
            | "STR"
            | "VARTYPE"
            | "TYPENAME"
            | "ISEMPTY"
            | "ISNULL"
            | "ISNUMERIC"
            | "ISDATE"
            | "ISARRAY"
            | "CBOOL"
            | "CBYTE"
            | "CINT"
            | "CLNG"
            | "CSNG"
            | "CDBL"
            | "CSTR"
            | "CDATE"
            | "CCUR"
            | "CVAR"
    )
}
