//! Lowering from AST + Semantic Model to VB6 Intermediate Representation (IR).

use crate::ast::decl::*;
use crate::ast::expr::*;
use crate::ast::stmt::*;
use crate::ast::SourceFile;
use crate::ir::model::*;
use crate::types::VbType;
use std::collections::HashMap;

pub fn default_constant_for_type(ty: &VbType) -> IrConstant {
    match ty {
        VbType::Byte => IrConstant::Int(0, VbType::Byte),
        VbType::Integer => IrConstant::Int(0, VbType::Integer),
        VbType::Long => IrConstant::Int(0, VbType::Long),
        VbType::LongPtr => IrConstant::Int(0, VbType::LongPtr),
        VbType::LongLong => IrConstant::Int(0, VbType::LongLong),
        VbType::Single => IrConstant::Float(0.0),
        VbType::Double => IrConstant::Float(0.0),
        VbType::Currency => IrConstant::Int(0, VbType::Currency),
        VbType::Boolean => IrConstant::Bool(false),
        VbType::Date => IrConstant::Date("1899-12-30".to_string()),
        VbType::String => IrConstant::Null,
        VbType::Object | VbType::Class(_) | VbType::UserDefined(_) => IrConstant::Nothing,
        VbType::Variant => IrConstant::Empty,
        _ => IrConstant::Int(0, VbType::Long),
    }
}

#[derive(Debug, Clone)]
pub struct WithContext {
    pub base_operand: Operand,
    pub vb_type: VbType,
}

pub struct AstToIrLowering {
    temp_counter: usize,
    block_counter: usize,
    current_blocks: Vec<BasicBlock>,
    current_block_idx: usize,
    current_procedure_name: Option<String>,
    module_globals: HashMap<String, (String, VbType)>,
    globals_list: Vec<IrGlobal>,
    functions: HashMap<String, (String, VbType)>,
    local_vars: HashMap<String, (String, VbType)>,
    static_vars: HashMap<String, (String, VbType)>,
    temp_types: HashMap<usize, VbType>,
    for_exit_stack: Vec<String>,
    do_exit_stack: Vec<String>,
    pub udt_layouts: HashMap<String, crate::types::UdtLayout>,
    pub enum_defs: HashMap<String, crate::types::EnumDefinition>,
    pub enum_constants: HashMap<String, (i32, VbType)>,
    pub class_layouts: HashMap<String, crate::types::ClassLayout>,
    pub current_class: Option<String>,
    pub with_stack: Vec<WithContext>,
    pub declared_locals: Vec<(String, VbType)>,
}

impl Default for AstToIrLowering {
    fn default() -> Self {
        Self::new()
    }
}

impl AstToIrLowering {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            block_counter: 0,
            current_blocks: Vec::new(),
            current_block_idx: 0,
            current_procedure_name: None,
            module_globals: HashMap::new(),
            globals_list: Vec::new(),
            functions: HashMap::new(),
            local_vars: HashMap::new(),
            static_vars: HashMap::new(),
            temp_types: HashMap::new(),
            for_exit_stack: Vec::new(),
            do_exit_stack: Vec::new(),
            udt_layouts: HashMap::new(),
            enum_defs: HashMap::new(),
            enum_constants: HashMap::new(),
            class_layouts: HashMap::new(),
            current_class: None,
            with_stack: Vec::new(),
            declared_locals: Vec::new(),
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

    pub fn resolve_variable(&self, name: &str) -> (Operand, VbType) {
        let key = name.to_ascii_uppercase();
        if let Some((local_name, ty)) = self.local_vars.get(&key) {
            (Operand::Local(local_name.clone()), ty.clone())
        } else if let Some((static_name, ty)) = self.static_vars.get(&key) {
            (Operand::Global(static_name.clone()), ty.clone())
        } else if let Some((global_name, ty)) = self.module_globals.get(&key) {
            (Operand::Global(global_name.clone()), ty.clone())
        } else if let Some((val, ty)) = self.enum_constants.get(&key) {
            (Operand::Constant(IrConstant::Int(*val as i64, ty.clone())), ty.clone())
        } else {
            (Operand::Local(name.to_string()), VbType::Variant)
        }
    }

    pub fn infer_operand_type(&self, op: &Operand) -> VbType {
        match op {
            Operand::Constant(c) => match c {
                IrConstant::Int(_, ty) => ty.clone(),
                IrConstant::Float(_) => VbType::Double,
                IrConstant::String(_) => VbType::String,
                IrConstant::Bool(_) => VbType::Boolean,
                IrConstant::Date(_) => VbType::Date,
                IrConstant::Null | IrConstant::Empty => VbType::Variant,
                IrConstant::Nothing => VbType::Object,
            },
            Operand::Local(name) => self.resolve_variable(name).1,
            Operand::Global(name) => {
                let key = name.to_ascii_uppercase();
                if let Some((_, ty)) = self.static_vars.get(&key) {
                    ty.clone()
                } else if let Some((_, ty)) = self.module_globals.get(&key) {
                    ty.clone()
                } else {
                    VbType::Unknown
                }
            }
            Operand::Temp(id) => self.temp_types.get(id).cloned().unwrap_or(VbType::Unknown),
        }
    }

    pub fn coerce_to_string(&mut self, op: Operand) -> Operand {
        let ty = self.infer_operand_type(&op);
        if ty != VbType::String {
            let dest = self.next_temp();
            self.temp_types.insert(dest, VbType::String);
            self.emit(IrInstruction::Coerce {
                dest,
                src: op,
                from_type: ty,
                to_type: VbType::String,
            });
            Operand::Temp(dest)
        } else {
            op
        }
    }

    pub fn coerce_to_variant(&mut self, op: Operand) -> Operand {
        let ty = self.infer_operand_type(&op);
        if ty != VbType::Variant {
            let dest = self.next_temp();
            self.temp_types.insert(dest, VbType::Variant);
            self.emit(IrInstruction::Coerce {
                dest,
                src: op,
                from_type: ty,
                to_type: VbType::Variant,
            });
            Operand::Temp(dest)
        } else {
            op
        }
    }

    pub fn resolve_member_access(
        &mut self,
        object: &Expression,
        member: &str,
    ) -> Option<(Operand, usize, VbType)> {
        let (base_op, base_ty) = match object {
            Expression::Variable(name) => self.resolve_variable(name),
            Expression::WithMemberAccess(inner_member) => {
                let ctx = self.with_stack.last().cloned()?;
                let struct_name = match &ctx.vb_type {
                    VbType::UserDefined(s) => s.clone(),
                    _ => return None,
                };
                let layout = self
                    .udt_layouts
                    .get(&struct_name.to_ascii_uppercase())?
                    .clone();
                let m = layout
                    .members
                    .iter()
                    .find(|m| m.name.eq_ignore_ascii_case(inner_member))?
                    .clone();
                let dest = self.next_temp();
                self.temp_types.insert(dest, m.vb_type.clone());
                self.emit(IrInstruction::GetMember {
                    dest,
                    base: ctx.base_operand,
                    offset: m.offset,
                    member_type: m.vb_type.clone(),
                });
                (Operand::Temp(dest), m.vb_type)
            }
            Expression::MemberAccess {
                object: inner_obj,
                member: inner_member,
            } => {
                let (b_op, offset, m_ty) = self.resolve_member_access(inner_obj, inner_member)?;
                let dest = self.next_temp();
                self.temp_types.insert(dest, m_ty.clone());
                self.emit(IrInstruction::GetMember {
                    dest,
                    base: b_op,
                    offset,
                    member_type: m_ty.clone(),
                });
                (Operand::Temp(dest), m_ty)
            }
            _ => {
                let op = self.lower_expression(object);
                let ty = self.infer_operand_type(&op);
                (op, ty)
            }
        };

        match &base_ty {
            VbType::UserDefined(s) => {
                let layout = self.udt_layouts.get(&s.to_ascii_uppercase())?;
                let m = layout
                    .members
                    .iter()
                    .find(|m| m.name.eq_ignore_ascii_case(member))?;
                Some((base_op, m.offset, m.vb_type.clone()))
            }
            VbType::Class(cls_name) => {
                let (f_offset, f_type) = {
                    let layout = self.class_layouts.get(&cls_name.to_ascii_uppercase())?;
                    let f = layout.find_field(member)?;
                    (f.offset, f.vb_type.clone())
                };
                let actual_base = match base_op {
                    Operand::Local(_) => {
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Class(cls_name.clone()));
                        self.emit(IrInstruction::Load {
                            dest,
                            src: base_op,
                        });
                        Operand::Temp(dest)
                    }
                    _ => base_op,
                };
                Some((actual_base, f_offset, f_type))
            }
            _ => None,
        }
    }

    fn next_temp(&mut self) -> usize {
        let t = self.temp_counter;
        self.temp_counter += 1;
        t
    }

    fn next_label(&mut self, prefix: &str) -> String {
        self.block_counter += 1;
        format!("{}_{}", prefix, self.block_counter)
    }

    fn emit(&mut self, inst: IrInstruction) {
        if self.current_block_idx < self.current_blocks.len() {
            self.current_blocks[self.current_block_idx]
                .instructions
                .push(inst);
        }
    }

    fn create_block(&mut self, label: &str) -> usize {
        let block = BasicBlock::new(label);
        self.current_blocks.push(block);
        self.current_blocks.len() - 1
    }

    fn switch_to_block(&mut self, idx: usize) {
        self.current_block_idx = idx;
    }

    /// Lower an entire SourceFile into an IrModule.
    pub fn lower_source_file(&mut self, file: &SourceFile, module_name: &str) -> IrModule {
        self.module_globals.clear();
        self.globals_list.clear();
        self.functions.clear();
        self.udt_layouts.clear();
        self.enum_defs.clear();
        self.enum_constants.clear();

        self.class_layouts.clear();

        // Preliminary pass 0: register all Class declarations so their layouts and types are known
        for decl in &file.declarations {
            if let Declaration::Class(c) = decl {
                let mut raw_fields = Vec::new();
                for inner in &c.declarations {
                    if let Declaration::ModuleVariables(mv) = inner {
                        for var in &mv.variables {
                            let mut f_type = var
                                .type_spec
                                .as_ref()
                                .map(|ts| self.resolve_type_spec(ts))
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
                            let param_types = s.params.iter().map(|p| {
                                p.param_type.as_ref().map(|ts| self.resolve_type_spec(ts)).unwrap_or(VbType::Variant)
                            }).collect();
                            layout.methods.insert(
                                s.name.to_ascii_uppercase(),
                                crate::types::ClassMethod {
                                    name: s.name.clone(),
                                    mangled_name: format!("{}__{}", c.name, s.name),
                                    is_sub: true,
                                    is_public: s.visibility == Visibility::Public,
                                    return_type: None,
                                    param_types,
                                },
                            );
                        }
                        Declaration::Function(f) => {
                            let ret_ty = f.return_type.as_ref().map(|ts| self.resolve_type_spec(ts)).unwrap_or(VbType::Variant);
                            let param_types = f.params.iter().map(|p| {
                                p.param_type.as_ref().map(|ts| self.resolve_type_spec(ts)).unwrap_or(VbType::Variant)
                            }).collect();
                            layout.methods.insert(
                                f.name.to_ascii_uppercase(),
                                crate::types::ClassMethod {
                                    name: f.name.clone(),
                                    mangled_name: format!("{}__{}", c.name, f.name),
                                    is_sub: false,
                                    is_public: f.visibility == Visibility::Public,
                                    return_type: Some(ret_ty),
                                    param_types,
                                },
                            );
                        }
                        Declaration::Property(p) => {
                            let prop_type = p.return_type.as_ref().map(|ts| self.resolve_type_spec(ts)).unwrap_or(VbType::Variant);
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
            match decl {
                Declaration::Type(t) => {
                    let raw_members: Vec<(String, VbType)> = t
                        .members
                        .iter()
                        .map(|m| (m.name.clone(), self.resolve_type_spec(&m.type_spec)))
                        .collect();
                    let layout = crate::types::UdtLayout::compute_with_layouts(&t.name, &raw_members, &self.udt_layouts, true);
                    self.udt_layouts.insert(t.name.to_ascii_uppercase(), layout);
                }
                Declaration::Enum(e) => {
                    let raw_members: Vec<(String, Option<i32>)> = e
                        .members
                        .iter()
                        .map(|m| (m.name.clone(), m.value))
                        .collect();
                    let enum_def = crate::types::EnumDefinition::new(&e.name, &raw_members);
                    self.enum_defs.insert(e.name.to_ascii_uppercase(), enum_def.clone());
                    for m in &enum_def.members {
                        self.enum_constants.insert(
                            m.name.to_ascii_uppercase(),
                            (m.value, VbType::Long),
                        );
                    }
                }
                _ => {}
            }
        }

        // First pass: register module variables and function signatures
        for decl in &file.declarations {
            match decl {
                Declaration::ModuleVariables(v) => {
                    for var in &v.variables {
                        let mut vb_type = var
                            .type_spec
                            .as_ref()
                            .map(|ts| self.resolve_type_spec(ts))
                            .unwrap_or(VbType::Variant);
                        if var.dimensions.is_some() {
                            vb_type = VbType::Array(Box::new(vb_type));
                        }
                        let is_public = v.visibility == Visibility::Public;
                        let key = var.name.to_ascii_uppercase();
                        self.module_globals.insert(key, (var.name.clone(), vb_type.clone()));
                        self.globals_list.push(IrGlobal {
                            name: var.name.clone(),
                            vb_type,
                            is_public,
                            initial_value: None,
                        });
                    }
                }
                Declaration::Function(f) => {
                    let ret_ty = f
                        .return_type
                        .as_ref()
                        .map(|ts| self.resolve_type_spec(ts))
                        .unwrap_or(VbType::Variant);
                    let key = f.name.to_ascii_uppercase();
                    self.functions.insert(key, (f.name.clone(), ret_ty));
                }
                _ => {}
            }
        }

        let mut functions = Vec::new();
        for decl in &file.declarations {
            match decl {
                Declaration::Sub(s) => {
                    let func = self.lower_sub(s);
                    functions.push(func);
                }
                Declaration::Function(f) => {
                    let func = self.lower_function(f);
                    functions.push(func);
                }
                Declaration::Class(c) => {
                    for inner in &c.declarations {
                        match inner {
                            Declaration::Sub(s) => {
                                let func = self.lower_class_sub(&c.name, s);
                                functions.push(func);
                            }
                            Declaration::Function(f) => {
                                let func = self.lower_class_function(&c.name, f);
                                functions.push(func);
                            }
                            Declaration::Property(p) => {
                                let func = self.lower_class_property(&c.name, p);
                                functions.push(func);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        IrModule {
            name: module_name.to_string(),
            globals: self.globals_list.clone(),
            functions,
            udt_layouts: self.udt_layouts.clone(),
            class_layouts: self.class_layouts.clone(),
        }
    }

    fn lower_sub(&mut self, sub: &SubDeclaration) -> IrFunction {
        self.temp_counter = 0;
        self.block_counter = 0;
        self.current_blocks.clear();
        self.local_vars.clear();
        self.declared_locals.clear();
        self.static_vars.clear();
        self.temp_types.clear();
        self.for_exit_stack.clear();
        self.do_exit_stack.clear();
        self.current_procedure_name = Some(sub.name.clone());

        let entry_idx = self.create_block("entry");
        self.switch_to_block(entry_idx);

        let params: Vec<IrParam> = sub
            .params
            .iter()
            .map(|p| {
                let mut ty = p
                    .param_type
                    .as_ref()
                    .map(|ts| self.resolve_type_spec(ts))
                    .unwrap_or(VbType::Variant);
                if let Some(ref pt) = p.param_type {
                    if !pt.array_dimensions.is_empty() {
                        ty = VbType::Array(Box::new(ty));
                    }
                }
                IrParam {
                    name: p.name.clone(),
                    vb_type: ty,
                    is_by_val: p.is_by_val,
                    is_optional: p.is_optional,
                    is_param_array: p.is_param_array,
                }
            })
            .collect();

        // Parameter copies into locals
        for param in &params {
            self.local_vars.insert(
                param.name.to_ascii_uppercase(),
                (param.name.clone(), param.vb_type.clone()),
            );
            let temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: temp,
                name: param.name.clone(),
                vb_type: param.vb_type.clone(),
            });
        }

        // Lower body statements
        for stmt in &sub.body {
            self.lower_statement(stmt);
        }

        // Ensure terminating return in the last block
        let last_idx = self.current_blocks.len() - 1;
        let needs_return = if let Some(last_inst) = self.current_blocks[last_idx].instructions.last() {
            !matches!(last_inst, IrInstruction::Return { .. } | IrInstruction::Branch { .. })
        } else {
            true
        };

        if needs_return {
            self.emit_local_reference_cleanup();
            self.current_blocks[last_idx]
                .instructions
                .push(IrInstruction::Return { value: None });
        }

        IrFunction {
            name: sub.name.clone(),
            params,
            return_type: None,
            blocks: self.current_blocks.clone(),
        }
    }

    fn lower_function(&mut self, func: &FunctionDeclaration) -> IrFunction {
        self.temp_counter = 0;
        self.block_counter = 0;
        self.current_blocks.clear();
        self.local_vars.clear();
        self.declared_locals.clear();
        self.static_vars.clear();
        self.temp_types.clear();
        self.for_exit_stack.clear();
        self.do_exit_stack.clear();
        self.current_procedure_name = Some(func.name.clone());

        let entry_idx = self.create_block("entry");
        self.switch_to_block(entry_idx);

        let params: Vec<IrParam> = func
            .params
            .iter()
            .map(|p| {
                let mut ty = p
                    .param_type
                    .as_ref()
                    .map(|ts| self.resolve_type_spec(ts))
                    .unwrap_or(VbType::Variant);
                if let Some(ref pt) = p.param_type {
                    if !pt.array_dimensions.is_empty() {
                        ty = VbType::Array(Box::new(ty));
                    }
                }
                IrParam {
                    name: p.name.clone(),
                    vb_type: ty,
                    is_by_val: p.is_by_val,
                    is_optional: p.is_optional,
                    is_param_array: p.is_param_array,
                }
            })
            .collect();

        let ret_ty = func
            .return_type
            .as_ref()
            .map(|ts| self.resolve_type_spec(ts))
            .unwrap_or(VbType::Variant);

        for param in &params {
            self.local_vars.insert(
                param.name.to_ascii_uppercase(),
                (param.name.clone(), param.vb_type.clone()),
            );
            let temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: temp,
                name: param.name.clone(),
                vb_type: param.vb_type.clone(),
            });
        }

        // Allocate return value variable (named after function in VB6)
        self.local_vars.insert(
            func.name.to_ascii_uppercase(),
            (func.name.clone(), ret_ty.clone()),
        );
        let ret_temp = self.next_temp();
        self.emit(IrInstruction::AllocLocal {
            dest: ret_temp,
            name: func.name.clone(),
            vb_type: ret_ty.clone(),
        });
        let def_const = default_constant_for_type(&ret_ty);
        self.emit(IrInstruction::Store {
            dest: Operand::Local(func.name.clone()),
            src: Operand::Constant(def_const),
        });

        for stmt in &func.body {
            self.lower_statement(stmt);
        }

        let last_idx = self.current_blocks.len() - 1;
        let needs_return = if let Some(last_inst) = self.current_blocks[last_idx].instructions.last() {
            !matches!(last_inst, IrInstruction::Return { .. } | IrInstruction::Branch { .. })
        } else {
            true
        };

        if needs_return {
            let ret_val_temp = self.next_temp();
            self.current_blocks[last_idx].instructions.push(IrInstruction::Load {
                dest: ret_val_temp,
                src: Operand::Local(func.name.clone()),
            });
            self.emit_local_reference_cleanup();
            self.current_blocks[last_idx].instructions.push(IrInstruction::Return {
                value: Some(Operand::Temp(ret_val_temp)),
            });
        }

        IrFunction {
            name: func.name.clone(),
            params,
            return_type: Some(ret_ty),
            blocks: self.current_blocks.clone(),
        }
    }

    fn emit_local_reference_cleanup(&mut self) {
        let vars: Vec<(String, VbType)> = self
            .declared_locals
            .iter()
            .filter(|(_, ty)| ty.is_reference())
            .cloned()
            .collect();
        for (name, ty) in vars {
            let temp = self.next_temp();
            self.temp_types.insert(temp, ty);
            self.emit(IrInstruction::Load {
                dest: temp,
                src: Operand::Local(name),
            });
            self.emit(IrInstruction::Release {
                object: Operand::Temp(temp),
            });
        }
    }

    fn lower_class_sub(&mut self, class_name: &str, sub: &SubDeclaration) -> IrFunction {
        self.temp_counter = 0;
        self.block_counter = 0;
        self.current_blocks.clear();
        self.local_vars.clear();
        self.declared_locals.clear();
        self.static_vars.clear();
        self.temp_types.clear();
        self.for_exit_stack.clear();
        self.do_exit_stack.clear();
        self.current_class = Some(class_name.to_string());
        self.current_procedure_name = Some(sub.name.clone());

        let entry_idx = self.create_block("entry");
        self.switch_to_block(entry_idx);

        let mut params = Vec::new();
        // Parameter 0: this
        params.push(IrParam {
            name: "this".to_string(),
            vb_type: VbType::Class(class_name.to_string()),
            is_by_val: true,
            is_optional: false,
            is_param_array: false,
        });
        self.local_vars.insert("THIS".to_string(), ("this".to_string(), VbType::Class(class_name.to_string())));
        self.local_vars.insert("ME".to_string(), ("this".to_string(), VbType::Class(class_name.to_string())));
        let temp_this = self.next_temp();
        self.emit(IrInstruction::AllocLocal {
            dest: temp_this,
            name: "this".to_string(),
            vb_type: VbType::Class(class_name.to_string()),
        });

        for p in &sub.params {
            let mut ty = p
                .param_type
                .as_ref()
                .map(|ts| self.resolve_type_spec(ts))
                .unwrap_or(VbType::Variant);
            if let Some(ref pt) = p.param_type {
                if !pt.array_dimensions.is_empty() {
                    ty = VbType::Array(Box::new(ty));
                }
            }
            params.push(IrParam {
                name: p.name.clone(),
                vb_type: ty.clone(),
                is_by_val: p.is_by_val,
                is_optional: p.is_optional,
                is_param_array: p.is_param_array,
            });
            self.local_vars.insert(p.name.to_ascii_uppercase(), (p.name.clone(), ty.clone()));
            let temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: temp,
                name: p.name.clone(),
                vb_type: ty,
            });
        }

        for stmt in &sub.body {
            self.lower_statement(stmt);
        }

        let last_idx = self.current_blocks.len() - 1;
        let needs_return = if let Some(last_inst) = self.current_blocks[last_idx].instructions.last() {
            !matches!(last_inst, IrInstruction::Return { .. } | IrInstruction::Branch { .. })
        } else {
            true
        };

        if needs_return {
            self.emit_local_reference_cleanup();
            self.current_blocks[last_idx]
                .instructions
                .push(IrInstruction::Return { value: None });
        }

        self.current_class = None;

        IrFunction {
            name: format!("{}__{}", class_name, sub.name),
            params,
            return_type: None,
            blocks: self.current_blocks.clone(),
        }
    }

    fn lower_class_function(&mut self, class_name: &str, func: &FunctionDeclaration) -> IrFunction {
        self.temp_counter = 0;
        self.block_counter = 0;
        self.current_blocks.clear();
        self.local_vars.clear();
        self.declared_locals.clear();
        self.static_vars.clear();
        self.temp_types.clear();
        self.for_exit_stack.clear();
        self.do_exit_stack.clear();
        self.current_class = Some(class_name.to_string());
        self.current_procedure_name = Some(func.name.clone());

        let entry_idx = self.create_block("entry");
        self.switch_to_block(entry_idx);

        let mut params = Vec::new();
        // Parameter 0: this
        params.push(IrParam {
            name: "this".to_string(),
            vb_type: VbType::Class(class_name.to_string()),
            is_by_val: true,
            is_optional: false,
            is_param_array: false,
        });
        self.local_vars.insert("THIS".to_string(), ("this".to_string(), VbType::Class(class_name.to_string())));
        self.local_vars.insert("ME".to_string(), ("this".to_string(), VbType::Class(class_name.to_string())));
        let temp_this = self.next_temp();
        self.emit(IrInstruction::AllocLocal {
            dest: temp_this,
            name: "this".to_string(),
            vb_type: VbType::Class(class_name.to_string()),
        });

        for p in &func.params {
            let mut ty = p
                .param_type
                .as_ref()
                .map(|ts| self.resolve_type_spec(ts))
                .unwrap_or(VbType::Variant);
            if let Some(ref pt) = p.param_type {
                if !pt.array_dimensions.is_empty() {
                    ty = VbType::Array(Box::new(ty));
                }
            }
            params.push(IrParam {
                name: p.name.clone(),
                vb_type: ty.clone(),
                is_by_val: p.is_by_val,
                is_optional: p.is_optional,
                is_param_array: p.is_param_array,
            });
            self.local_vars.insert(p.name.to_ascii_uppercase(), (p.name.clone(), ty.clone()));
            let temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: temp,
                name: p.name.clone(),
                vb_type: ty,
            });
        }

        let ret_ty = func
            .return_type
            .as_ref()
            .map(|ts| self.resolve_type_spec(ts))
            .unwrap_or(VbType::Variant);

        self.local_vars.insert(
            func.name.to_ascii_uppercase(),
            (func.name.clone(), ret_ty.clone()),
        );
        let ret_temp = self.next_temp();
        self.emit(IrInstruction::AllocLocal {
            dest: ret_temp,
            name: func.name.clone(),
            vb_type: ret_ty.clone(),
        });
        let def_const = default_constant_for_type(&ret_ty);
        self.emit(IrInstruction::Store {
            dest: Operand::Local(func.name.clone()),
            src: Operand::Constant(def_const),
        });

        for stmt in &func.body {
            self.lower_statement(stmt);
        }

        let last_idx = self.current_blocks.len() - 1;
        let needs_return = if let Some(last_inst) = self.current_blocks[last_idx].instructions.last() {
            !matches!(last_inst, IrInstruction::Return { .. } | IrInstruction::Branch { .. })
        } else {
            true
        };

        if needs_return {
            let ret_val_temp = self.next_temp();
            self.current_blocks[last_idx].instructions.push(IrInstruction::Load {
                dest: ret_val_temp,
                src: Operand::Local(func.name.clone()),
            });
            self.emit_local_reference_cleanup();
            self.current_blocks[last_idx].instructions.push(IrInstruction::Return {
                value: Some(Operand::Temp(ret_val_temp)),
            });
        }

        self.current_class = None;

        IrFunction {
            name: format!("{}__{}", class_name, func.name),
            params,
            return_type: Some(ret_ty),
            blocks: self.current_blocks.clone(),
        }
    }

    fn lower_class_property(&mut self, class_name: &str, prop: &PropertyDeclaration) -> IrFunction {
        self.temp_counter = 0;
        self.block_counter = 0;
        self.current_blocks.clear();
        self.local_vars.clear();
        self.declared_locals.clear();
        self.static_vars.clear();
        self.temp_types.clear();
        self.for_exit_stack.clear();
        self.do_exit_stack.clear();
        self.current_class = Some(class_name.to_string());
        self.current_procedure_name = Some(prop.name.clone());

        let entry_idx = self.create_block("entry");
        self.switch_to_block(entry_idx);

        let mut params = Vec::new();
        // Parameter 0: this
        params.push(IrParam {
            name: "this".to_string(),
            vb_type: VbType::Class(class_name.to_string()),
            is_by_val: true,
            is_optional: false,
            is_param_array: false,
        });
        self.local_vars.insert("THIS".to_string(), ("this".to_string(), VbType::Class(class_name.to_string())));
        self.local_vars.insert("ME".to_string(), ("this".to_string(), VbType::Class(class_name.to_string())));
        let temp_this = self.next_temp();
        self.emit(IrInstruction::AllocLocal {
            dest: temp_this,
            name: "this".to_string(),
            vb_type: VbType::Class(class_name.to_string()),
        });

        for p in &prop.params {
            let mut ty = p
                .param_type
                .as_ref()
                .map(|ts| self.resolve_type_spec(ts))
                .unwrap_or(VbType::Variant);
            if let Some(ref pt) = p.param_type {
                if !pt.array_dimensions.is_empty() {
                    ty = VbType::Array(Box::new(ty));
                }
            }
            params.push(IrParam {
                name: p.name.clone(),
                vb_type: ty.clone(),
                is_by_val: p.is_by_val,
                is_optional: p.is_optional,
                is_param_array: p.is_param_array,
            });
            self.local_vars.insert(p.name.to_ascii_uppercase(), (p.name.clone(), ty.clone()));
            let temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: temp,
                name: p.name.clone(),
                vb_type: ty,
            });
        }

        let ret_ty = prop
            .return_type
            .as_ref()
            .map(|ts| self.resolve_type_spec(ts))
            .unwrap_or(VbType::Variant);

        if prop.kind == PropertyKind::Get {
            self.local_vars.insert(
                prop.name.to_ascii_uppercase(),
                (prop.name.clone(), ret_ty.clone()),
            );
            let ret_temp = self.next_temp();
            self.emit(IrInstruction::AllocLocal {
                dest: ret_temp,
                name: prop.name.clone(),
                vb_type: ret_ty.clone(),
            });
            let def_const = default_constant_for_type(&ret_ty);
            self.emit(IrInstruction::Store {
                dest: Operand::Local(prop.name.clone()),
                src: Operand::Constant(def_const),
            });
        }

        for stmt in &prop.body {
            self.lower_statement(stmt);
        }

        let last_idx = self.current_blocks.len() - 1;
        let needs_return = if let Some(last_inst) = self.current_blocks[last_idx].instructions.last() {
            !matches!(last_inst, IrInstruction::Return { .. } | IrInstruction::Branch { .. })
        } else {
            true
        };

        if needs_return {
            if prop.kind == PropertyKind::Get {
                let ret_val_temp = self.next_temp();
                self.current_blocks[last_idx].instructions.push(IrInstruction::Load {
                    dest: ret_val_temp,
                    src: Operand::Local(prop.name.clone()),
                });
                self.emit_local_reference_cleanup();
                self.current_blocks[last_idx].instructions.push(IrInstruction::Return {
                    value: Some(Operand::Temp(ret_val_temp)),
                });
            } else {
                self.emit_local_reference_cleanup();
                self.current_blocks[last_idx]
                    .instructions
                    .push(IrInstruction::Return { value: None });
            }
        }

        self.current_class = None;

        let suffix = match prop.kind {
            PropertyKind::Get => "Get",
            PropertyKind::Let => "Let",
            PropertyKind::Set => "Set",
        };

        IrFunction {
            name: format!("{}__{}__{}", class_name, prop.name, suffix),
            params,
            return_type: if prop.kind == PropertyKind::Get { Some(ret_ty) } else { None },
            blocks: self.current_blocks.clone(),
        }
    }

    fn lower_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration { kind, variables, .. } => {
                match kind {
                    VarDeclKind::Static => {
                        for v in variables {
                            let mut vb_type = v
                                .type_spec
                                .as_ref()
                                .map(|ts| self.resolve_type_spec(ts))
                                .unwrap_or(VbType::Variant);
                            if v.dimensions.is_some() {
                                vb_type = VbType::Array(Box::new(vb_type));
                            }
                            let proc_name = self.current_procedure_name.as_deref().unwrap_or("proc");
                            let mangled_name = format!("stat_{}_{}", proc_name, v.name);
                            let key = v.name.to_ascii_uppercase();
                            self.static_vars.insert(key, (mangled_name.clone(), vb_type.clone()));
                            self.globals_list.push(IrGlobal {
                                name: mangled_name,
                                vb_type,
                                is_public: false,
                                initial_value: None,
                            });
                        }
                    }
                    VarDeclKind::ReDim | VarDeclKind::ReDimPreserve => {
                        for v in variables {
                            let (_prev_target_op, target_ty) = self.resolve_variable(&v.name);
                            let elem_type = if let Some(ts) = &v.type_spec {
                                self.resolve_type_spec(ts)
                            } else {
                                match &target_ty {
                                    VbType::Array(elem) => (**elem).clone(),
                                    _ => VbType::Long,
                                }
                            };

                            let key = v.name.to_ascii_uppercase();
                            let full_arr_ty = VbType::Array(Box::new(elem_type.clone()));
                            if !self.local_vars.contains_key(&key) && !self.module_globals.contains_key(&key) {
                                self.local_vars.insert(key.clone(), (v.name.clone(), full_arr_ty.clone()));
                                let alloc_temp = self.next_temp();
                                self.emit(IrInstruction::AllocLocal {
                                    dest: alloc_temp,
                                    name: v.name.clone(),
                                    vb_type: full_arr_ty,
                                });
                            }

                            let mut bounds = Vec::new();
                            if let Some(dims) = &v.dimensions {
                                for dim in dims {
                                    let lower_op = if let Some(low) = &dim.lower {
                                        self.lower_expression(low)
                                    } else {
                                        Operand::Constant(IrConstant::Int(0, VbType::Long))
                                    };
                                    let upper_op = self.lower_expression(&dim.upper);
                                    bounds.push((lower_op, upper_op));
                                }
                            }

                            let (target_op, _) = self.resolve_variable(&v.name);
                            let arr_ptr_temp = self.next_temp();
                            self.emit(IrInstruction::Load {
                                dest: arr_ptr_temp,
                                src: target_op.clone(),
                            });

                            let redim_dest = self.next_temp();
                            let preserve = matches!(kind, VarDeclKind::ReDimPreserve);
                            self.emit(IrInstruction::ArrayReDim {
                                dest: redim_dest,
                                array: Operand::Temp(arr_ptr_temp),
                                dims: bounds.len(),
                                bounds,
                                elem_type,
                                preserve,
                            });

                            self.emit(IrInstruction::Store {
                                dest: target_op,
                                src: Operand::Temp(redim_dest),
                            });
                        }
                    }
                    _ => {
                        for v in variables {
                            let mut vb_type = v
                                .type_spec
                                .as_ref()
                                .map(|ts| self.resolve_type_spec(ts))
                                .unwrap_or(VbType::Variant);
                            if v.dimensions.is_some() {
                                vb_type = VbType::Array(Box::new(vb_type));
                            }

                            let key = v.name.to_ascii_uppercase();
                            self.local_vars.insert(key, (v.name.clone(), vb_type.clone()));
                            self.declared_locals.push((v.name.clone(), vb_type.clone()));

                            let dest = self.next_temp();
                            self.emit(IrInstruction::AllocLocal {
                                dest,
                                name: v.name.clone(),
                                vb_type: vb_type.clone(),
                            });

                            if let Some(dims) = &v.dimensions {
                                if !dims.is_empty() {
                                    // Fixed-size array
                                    let elem_ty = match &vb_type {
                                        VbType::Array(elem) => (**elem).clone(),
                                        _ => VbType::Long,
                                    };
                                    let mut bounds = Vec::new();
                                    for dim in dims {
                                        let lower_op = if let Some(low) = &dim.lower {
                                            self.lower_expression(low)
                                        } else {
                                            Operand::Constant(IrConstant::Int(0, VbType::Long))
                                        };
                                        let upper_op = self.lower_expression(&dim.upper);
                                        bounds.push((lower_op, upper_op));
                                    }
                                    let create_dest = self.next_temp();
                                    self.temp_types.insert(create_dest, vb_type.clone());
                                    self.emit(IrInstruction::ArrayCreate {
                                        dest: create_dest,
                                        dims: bounds.len(),
                                        bounds,
                                        elem_type: elem_ty,
                                     });
                                    self.emit(IrInstruction::Store {
                                        dest: Operand::Local(v.name.clone()),
                                        src: Operand::Temp(create_dest),
                                    });
                                } else {
                                    // Dynamic array: initially null
                                    self.emit(IrInstruction::Store {
                                        dest: Operand::Local(v.name.clone()),
                                        src: Operand::Constant(IrConstant::Null),
                                    });
                                }
                            } else if let Some(init) = &v.init_value {
                                let init_op = self.lower_expression(init);
                                self.emit(IrInstruction::Store {
                                    dest: Operand::Local(v.name.clone()),
                                    src: init_op,
                                });
                            } else if let Some(ref ts) = v.type_spec {
                                if ts.is_new {
                                    let new_dest = self.next_temp();
                                    self.temp_types.insert(new_dest, vb_type.clone());
                                    self.emit(IrInstruction::NewObject {
                                        dest: new_dest,
                                        class_name: ts.name.clone(),
                                    });
                                    self.emit(IrInstruction::Store {
                                        dest: Operand::Local(v.name.clone()),
                                        src: Operand::Temp(new_dest),
                                    });
                                } else if !matches!(vb_type, VbType::UserDefined(_)) {
                                    let def_const = default_constant_for_type(&vb_type);
                                    self.emit(IrInstruction::Store {
                                        dest: Operand::Local(v.name.clone()),
                                        src: Operand::Constant(def_const),
                                    });
                                }
                            } else if !matches!(vb_type, VbType::UserDefined(_)) {
                                let def_const = default_constant_for_type(&vb_type);
                                self.emit(IrInstruction::Store {
                                    dest: Operand::Local(v.name.clone()),
                                    src: Operand::Constant(def_const),
                                });
                            }
                        }
                    }
                }
            }
            Statement::Assignment { target, value, is_set, .. } => {
                if let Expression::MemberAccess { object, member } = target {
                    let obj_op = self.lower_expression(object);
                    let obj_ty = self.infer_operand_type(&obj_op);
                    if let VbType::Class(ref cls_name) = obj_ty {
                        let prop_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_property(member).map(|p| (p.has_set, p.prop_type.is_reference())));
                        if let Some((has_set, prop_is_ref)) = prop_info {
                            let val_op = self.lower_expression(value);
                            let prop_fn = if has_set && (*is_set || prop_is_ref) {
                                format!("{}__{}__Set", cls_name, member)
                            } else {
                                format!("{}__{}__Let", cls_name, member)
                            };
                            let actual_base = match obj_op {
                                Operand::Local(_) => {
                                    let dest = self.next_temp();
                                    self.temp_types.insert(dest, VbType::Class(cls_name.clone()));
                                    self.emit(IrInstruction::Load {
                                        dest,
                                        src: obj_op,
                                    });
                                    Operand::Temp(dest)
                                }
                                _ => obj_op,
                            };
                            self.emit(IrInstruction::Call {
                                dest: None,
                                name: prop_fn,
                                args: vec![actual_base, val_op],
                                return_type: None,
                            });
                            return;
                        }
                    }
                    if let Some((base_op, offset, member_type)) = self.resolve_member_access(object, member) {
                        let val_op = self.lower_expression(value);
                        let val_ty = self.infer_operand_type(&val_op);
                        let final_val_op = if matches!(member_type, VbType::FixedString(_)) {
                            self.coerce_to_string(val_op)
                        } else if member_type != VbType::Unknown
                            && val_ty != VbType::Unknown
                            && member_type != val_ty
                            && val_ty.can_coerce_to(&member_type)
                        {
                            match (&val_op, &member_type) {
                                (Operand::Constant(IrConstant::Int(n, _)), VbType::Single | VbType::Double) => {
                                    Operand::Constant(IrConstant::Float(*n as f64))
                                }
                                (Operand::Constant(IrConstant::Int(n, _)), VbType::Byte | VbType::Integer | VbType::Long | VbType::Currency | VbType::Enum(_)) => {
                                    Operand::Constant(IrConstant::Int(*n, member_type.clone()))
                                }
                                _ => {
                                    let cast_temp = self.next_temp();
                                    self.temp_types.insert(cast_temp, member_type.clone());
                                    self.emit(IrInstruction::Coerce {
                                        dest: cast_temp,
                                        src: val_op,
                                        from_type: val_ty,
                                        to_type: member_type.clone(),
                                    });
                                    Operand::Temp(cast_temp)
                                }
                            }
                        } else {
                            val_op
                        };

                        self.emit(IrInstruction::SetMember {
                            base: base_op,
                            offset,
                            value: final_val_op,
                            member_type,
                        });
                        return;
                    }
                } else if let Expression::WithMemberAccess(member) = target {
                    if let Some(ctx) = self.with_stack.last().cloned() {
                        if let VbType::Class(ref cls_name) = ctx.vb_type {
                            let prop_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                                .and_then(|l| l.find_property(member).map(|p| (p.has_set, p.prop_type.is_reference())));
                            if let Some((has_set, prop_is_ref)) = prop_info {
                                let val_op = self.lower_expression(value);
                                let prop_fn = if has_set && (*is_set || prop_is_ref) {
                                    format!("{}__{}__Set", cls_name, member)
                                } else {
                                    format!("{}__{}__Let", cls_name, member)
                                };
                                let actual_base = match ctx.base_operand {
                                    Operand::Local(_) => {
                                        let dest = self.next_temp();
                                        self.temp_types.insert(dest, VbType::Class(cls_name.clone()));
                                        self.emit(IrInstruction::Load {
                                            dest,
                                            src: ctx.base_operand,
                                        });
                                        Operand::Temp(dest)
                                    }
                                    _ => ctx.base_operand,
                                };
                                self.emit(IrInstruction::Call {
                                    dest: None,
                                    name: prop_fn,
                                    args: vec![actual_base, val_op],
                                    return_type: None,
                                });
                                return;
                            } else if let Some((offset, member_type)) = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                                .and_then(|l| l.find_field(member).map(|f| (f.offset, f.vb_type.clone())))
                            {
                                let actual_base = match ctx.base_operand {
                                    Operand::Local(_) => {
                                        let dest = self.next_temp();
                                        self.temp_types.insert(dest, VbType::Class(cls_name.clone()));
                                        self.emit(IrInstruction::Load {
                                            dest,
                                            src: ctx.base_operand,
                                        });
                                        Operand::Temp(dest)
                                    }
                                    _ => ctx.base_operand,
                                };
                                let val_op = self.lower_expression(value);
                                self.emit(IrInstruction::SetMember {
                                    base: actual_base,
                                    offset,
                                    value: val_op,
                                    member_type,
                                });
                                return;
                            }
                        }
                    }

                    let with_res = if let Some(ctx) = self.with_stack.last().cloned() {
                        let struct_name = match &ctx.vb_type {
                            VbType::UserDefined(s) => s.clone(),
                            _ => "".to_string(),
                        };
                        if let Some(layout) = self.udt_layouts.get(&struct_name.to_ascii_uppercase()) {
                            layout
                                .members
                                .iter()
                                .find(|m| m.name.eq_ignore_ascii_case(member))
                                .map(|m| (ctx.base_operand.clone(), m.offset, m.vb_type.clone()))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some((base_op, offset, member_type)) = with_res {
                        let val_op = self.lower_expression(value);
                        let val_ty = self.infer_operand_type(&val_op);
                        let final_val_op = if matches!(member_type, VbType::FixedString(_)) {
                            self.coerce_to_string(val_op)
                        } else if member_type != VbType::Unknown
                            && val_ty != VbType::Unknown
                            && member_type != val_ty
                            && val_ty.can_coerce_to(&member_type)
                        {
                            match (&val_op, &member_type) {
                                (Operand::Constant(IrConstant::Int(n, _)), VbType::Single | VbType::Double) => {
                                    Operand::Constant(IrConstant::Float(*n as f64))
                                }
                                (Operand::Constant(IrConstant::Int(n, _)), VbType::Byte | VbType::Integer | VbType::Long | VbType::Currency | VbType::Enum(_)) => {
                                    Operand::Constant(IrConstant::Int(*n, member_type.clone()))
                                }
                                _ => {
                                    let cast_temp = self.next_temp();
                                    self.temp_types.insert(cast_temp, member_type.clone());
                                    self.emit(IrInstruction::Coerce {
                                        dest: cast_temp,
                                        src: val_op,
                                        from_type: val_ty,
                                        to_type: member_type.clone(),
                                    });
                                    Operand::Temp(cast_temp)
                                }
                            }
                        } else {
                            val_op
                        };

                        self.emit(IrInstruction::SetMember {
                            base: base_op,
                            offset,
                            value: final_val_op,
                            member_type,
                        });
                        return;
                    }
                }

                if let Expression::IndexAccess { target: arr_target, indices } = target {
                    let val_op = self.lower_expression(value);
                    let val_ty = self.infer_operand_type(&val_op);
                    let arr_op = self.lower_expression(arr_target);
                    let arr_ty = self.infer_operand_type(&arr_op);
                    let elem_type = match arr_ty {
                        VbType::Array(elem) => *elem,
                        _ => VbType::Long,
                    };

                    let final_val_op = if elem_type != VbType::Unknown
                        && val_ty != VbType::Unknown
                        && elem_type != val_ty
                        && val_ty.can_coerce_to(&elem_type)
                    {
                        match (&val_op, &elem_type) {
                            (Operand::Constant(IrConstant::Int(n, _)), VbType::Single | VbType::Double) => {
                                Operand::Constant(IrConstant::Float(*n as f64))
                            }
                            (Operand::Constant(IrConstant::Int(n, _)), VbType::Byte | VbType::Integer | VbType::Long | VbType::Currency | VbType::Enum(_)) => {
                                Operand::Constant(IrConstant::Int(*n, elem_type.clone()))
                            }
                            _ => {
                                let cast_temp = self.next_temp();
                                self.temp_types.insert(cast_temp, elem_type.clone());
                                self.emit(IrInstruction::Coerce {
                                    dest: cast_temp,
                                    src: val_op,
                                    from_type: val_ty,
                                    to_type: elem_type.clone(),
                                });
                                Operand::Temp(cast_temp)
                            }
                        }
                    } else {
                        val_op
                    };

                    let idx_ops: Vec<Operand> = indices.iter().map(|idx| self.lower_expression(idx)).collect();
                    self.emit(IrInstruction::ArraySetElement {
                        array: arr_op,
                        indices: idx_ops,
                        value: final_val_op,
                        elem_type,
                    });
                } else {
                    if let Expression::Variable(name) = target {
                        if let Some(cls_name) = self.current_class.clone() {
                            if !self.local_vars.contains_key(&name.to_ascii_uppercase()) {
                                let prop_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                                    .and_then(|l| l.find_property(name).map(|p| (p.has_set, p.prop_type.is_reference())));
                                if let Some((has_set, prop_is_ref)) = prop_info {
                                    let val_op = self.lower_expression(value);
                                    let prop_fn = if has_set && (*is_set || prop_is_ref) {
                                        format!("{}__{}__Set", cls_name, name)
                                    } else {
                                        format!("{}__{}__Let", cls_name, name)
                                    };
                                    let this_temp = self.next_temp();
                                    self.temp_types.insert(this_temp, VbType::Class(cls_name.clone()));
                                    self.emit(IrInstruction::Load {
                                        dest: this_temp,
                                        src: Operand::Local("this".to_string()),
                                    });
                                    self.emit(IrInstruction::Call {
                                        dest: None,
                                        name: prop_fn,
                                        args: vec![Operand::Temp(this_temp), val_op],
                                        return_type: None,
                                    });
                                    return;
                                }

                                if let Some((f_offset, f_type)) = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                                    .and_then(|l| l.find_field(name).map(|f| (f.offset, f.vb_type.clone())))
                                {
                                    let val_op = self.lower_expression(value);
                                    let this_temp = self.next_temp();
                                    self.temp_types.insert(this_temp, VbType::Class(cls_name.clone()));
                                    self.emit(IrInstruction::Load {
                                        dest: this_temp,
                                        src: Operand::Local("this".to_string()),
                                    });
                                    self.emit(IrInstruction::SetMember {
                                        base: Operand::Temp(this_temp),
                                        offset: f_offset,
                                        value: val_op,
                                        member_type: f_type,
                                    });
                                    return;
                                }
                            }
                        }
                    }

                    let (target_op, target_ty) = match target {
                        Expression::Variable(name) => self.resolve_variable(name),
                        _ => (Operand::Local("Unknown".to_string()), VbType::Unknown),
                    };
                    let val_op = self.lower_expression(value);
                    let val_ty = self.infer_operand_type(&val_op);

                    let final_val_op = if matches!(target_ty, VbType::UserDefined(_)) {
                        val_op
                    } else if target_ty != VbType::Unknown
                        && val_ty != VbType::Unknown
                        && target_ty != val_ty
                        && val_ty.can_coerce_to(&target_ty)
                    {
                        match (&val_op, &target_ty) {
                            (Operand::Constant(IrConstant::Int(n, _)), VbType::Single | VbType::Double) => {
                                Operand::Constant(IrConstant::Float(*n as f64))
                            }
                            (Operand::Constant(IrConstant::Int(n, _)), VbType::Byte | VbType::Integer | VbType::Long | VbType::Currency | VbType::Enum(_)) => {
                                Operand::Constant(IrConstant::Int(*n, target_ty.clone()))
                            }
                            _ => {
                                let cast_temp = self.next_temp();
                                self.temp_types.insert(cast_temp, target_ty.clone());
                                self.emit(IrInstruction::Coerce {
                                    dest: cast_temp,
                                    src: val_op,
                                    from_type: val_ty,
                                    to_type: target_ty.clone(),
                                });
                                Operand::Temp(cast_temp)
                            }
                        }
                    } else {
                        val_op
                    };

                    if target_ty.is_reference() || *is_set {
                        let old_val_temp = self.next_temp();
                        self.temp_types.insert(old_val_temp, target_ty.clone());
                        self.emit(IrInstruction::Load {
                            dest: old_val_temp,
                            src: target_op.clone(),
                        });
                        if !matches!(value, Expression::New(_)) {
                            self.emit(IrInstruction::AddRef {
                                object: final_val_op.clone(),
                            });
                        }
                        self.emit(IrInstruction::Store {
                            dest: target_op,
                            src: final_val_op,
                        });
                        self.emit(IrInstruction::Release {
                            object: Operand::Temp(old_val_temp),
                        });
                    } else {
                        self.emit(IrInstruction::Store {
                            dest: target_op,
                            src: final_val_op,
                        });
                    }
                }
            }
            Statement::If {
                branches,
                else_branch,
                ..
            } => {
                let merge_label = self.next_label("if_merge");

                for (i, branch) in branches.iter().enumerate() {
                    let cond_op = self.lower_expression(&branch.condition);
                    let then_label = self.next_label("if_then");
                    let next_test_label = if i + 1 < branches.len() || else_branch.is_some() {
                        self.next_label("if_next")
                    } else {
                        merge_label.clone()
                    };

                    self.emit(IrInstruction::BranchIf {
                        condition: cond_op,
                        true_target: then_label.clone(),
                        false_target: next_test_label.clone(),
                    });

                    // Then block
                    let then_idx = self.create_block(&then_label);
                    self.switch_to_block(then_idx);
                    for s in &branch.body {
                        self.lower_statement(s);
                    }
                    self.emit(IrInstruction::Branch {
                        target: merge_label.clone(),
                    });

                    // Next test block
                    if next_test_label != merge_label {
                        let next_idx = self.create_block(&next_test_label);
                        self.switch_to_block(next_idx);
                    }
                }

                if let Some(else_stmts) = else_branch {
                    for s in else_stmts {
                        self.lower_statement(s);
                    }
                    self.emit(IrInstruction::Branch {
                        target: merge_label.clone(),
                    });
                }

                let merge_idx = self.create_block(&merge_label);
                self.switch_to_block(merge_idx);
            }
            Statement::For {
                counter,
                start,
                end,
                step,
                body,
                ..
            } => {
                let (counter_target, counter_ty) = self.resolve_variable(counter);
                // Initialize counter
                let start_op = self.lower_expression(start);
                self.emit(IrInstruction::Store {
                    dest: counter_target.clone(),
                    src: start_op,
                });

                let end_op = self.lower_expression(end);
                let step_op = if let Some(st) = step {
                    self.lower_expression(st)
                } else {
                    Operand::Constant(IrConstant::Int(1, counter_ty.clone()))
                };

                let header_label = self.next_label("for_header");
                let body_label = self.next_label("for_body");
                let exit_label = self.next_label("for_exit");

                self.for_exit_stack.push(exit_label.clone());

                self.emit(IrInstruction::Branch {
                    target: header_label.clone(),
                });

                // Header block (condition test)
                let header_idx = self.create_block(&header_label);
                self.switch_to_block(header_idx);

                let counter_load_temp = self.next_temp();
                self.emit(IrInstruction::Load {
                    dest: counter_load_temp,
                    src: counter_target.clone(),
                });

                // Check step direction (positive vs negative)
                let is_static_negative = match &step_op {
                    Operand::Constant(IrConstant::Int(val, _)) if *val < 0 => Some(true),
                    Operand::Constant(IrConstant::Int(val, _)) if *val >= 0 => Some(false),
                    _ => None,
                };

                match is_static_negative {
                    Some(true) => {
                        let cmp_temp = self.next_temp();
                        self.emit(IrInstruction::Compare {
                            dest: cmp_temp,
                            op: IrCompareOp::GreaterThanOrEqual,
                            left: Operand::Temp(counter_load_temp),
                            right: end_op,
                        });
                        self.emit(IrInstruction::BranchIf {
                            condition: Operand::Temp(cmp_temp),
                            true_target: body_label.clone(),
                            false_target: exit_label.clone(),
                        });
                    }
                    Some(false) => {
                        let cmp_temp = self.next_temp();
                        self.emit(IrInstruction::Compare {
                            dest: cmp_temp,
                            op: IrCompareOp::LessThanOrEqual,
                            left: Operand::Temp(counter_load_temp),
                            right: end_op,
                        });
                        self.emit(IrInstruction::BranchIf {
                            condition: Operand::Temp(cmp_temp),
                            true_target: body_label.clone(),
                            false_target: exit_label.clone(),
                        });
                    }
                    None => {
                        // Dynamic step check
                        let pos_label = self.next_label("for_pos");
                        let neg_label = self.next_label("for_neg");
                        let is_pos_temp = self.next_temp();
                        self.emit(IrInstruction::Compare {
                            dest: is_pos_temp,
                            op: IrCompareOp::GreaterThanOrEqual,
                            left: step_op.clone(),
                            right: Operand::Constant(IrConstant::Int(0, counter_ty.clone())),
                        });
                        self.emit(IrInstruction::BranchIf {
                            condition: Operand::Temp(is_pos_temp),
                            true_target: pos_label.clone(),
                            false_target: neg_label.clone(),
                        });

                        let pos_idx = self.create_block(&pos_label);
                        self.switch_to_block(pos_idx);
                        let cmp_pos = self.next_temp();
                        self.emit(IrInstruction::Compare {
                            dest: cmp_pos,
                            op: IrCompareOp::LessThanOrEqual,
                            left: Operand::Temp(counter_load_temp),
                            right: end_op.clone(),
                        });
                        self.emit(IrInstruction::BranchIf {
                            condition: Operand::Temp(cmp_pos),
                            true_target: body_label.clone(),
                            false_target: exit_label.clone(),
                        });

                        let neg_idx = self.create_block(&neg_label);
                        self.switch_to_block(neg_idx);
                        let cmp_neg = self.next_temp();
                        self.emit(IrInstruction::Compare {
                            dest: cmp_neg,
                            op: IrCompareOp::GreaterThanOrEqual,
                            left: Operand::Temp(counter_load_temp),
                            right: end_op,
                        });
                        self.emit(IrInstruction::BranchIf {
                            condition: Operand::Temp(cmp_neg),
                            true_target: body_label.clone(),
                            false_target: exit_label.clone(),
                        });
                    }
                }

                // Body block
                let body_idx = self.create_block(&body_label);
                self.switch_to_block(body_idx);
                for s in body {
                    self.lower_statement(s);
                }

                // Step increment
                let curr_val_temp = self.next_temp();
                self.emit(IrInstruction::Load {
                    dest: curr_val_temp,
                    src: counter_target.clone(),
                });

                let add_temp = self.next_temp();
                self.emit(IrInstruction::BinaryOp {
                    dest: add_temp,
                    op: IrBinaryOp::Add,
                    left: Operand::Temp(curr_val_temp),
                    right: step_op,
                    result_type: counter_ty,
                });

                self.emit(IrInstruction::Store {
                    dest: counter_target,
                    src: Operand::Temp(add_temp),
                });

                self.emit(IrInstruction::Branch {
                    target: header_label,
                });

                self.for_exit_stack.pop();

                // Exit block
                let exit_idx = self.create_block(&exit_label);
                self.switch_to_block(exit_idx);
            }
            Statement::DoLoop {
                condition,
                condition_at_start,
                body,
                ..
            } => {
                let header_label = self.next_label("loop_header");
                let body_label = self.next_label("loop_body");
                let exit_label = self.next_label("loop_exit");

                self.do_exit_stack.push(exit_label.clone());

                self.emit(IrInstruction::Branch {
                    target: header_label.clone(),
                });

                let header_idx = self.create_block(&header_label);
                self.switch_to_block(header_idx);

                if *condition_at_start {
                    if let Some(cond) = condition {
                        let (cond_op, is_until) = match cond {
                            LoopCondition::While(e) => (self.lower_expression(e), false),
                            LoopCondition::Until(e) => (self.lower_expression(e), true),
                        };

                        let (true_t, false_t) = if is_until {
                            (exit_label.clone(), body_label.clone())
                        } else {
                            (body_label.clone(), exit_label.clone())
                        };

                        self.emit(IrInstruction::BranchIf {
                            condition: cond_op,
                            true_target: true_t,
                            false_target: false_t,
                        });
                    } else {
                        self.emit(IrInstruction::Branch {
                            target: body_label.clone(),
                        });
                    }
                } else {
                    self.emit(IrInstruction::Branch {
                        target: body_label.clone(),
                    });
                }

                let body_idx = self.create_block(&body_label);
                self.switch_to_block(body_idx);
                for s in body {
                    self.lower_statement(s);
                }

                if !condition_at_start {
                    if let Some(cond) = condition {
                        let (cond_op, is_until) = match cond {
                            LoopCondition::While(e) => (self.lower_expression(e), false),
                            LoopCondition::Until(e) => (self.lower_expression(e), true),
                        };

                        let (true_t, false_t) = if is_until {
                            (exit_label.clone(), header_label.clone())
                        } else {
                            (header_label.clone(), exit_label.clone())
                        };

                        self.emit(IrInstruction::BranchIf {
                            condition: cond_op,
                            true_target: true_t,
                            false_target: false_t,
                        });
                    } else {
                        self.emit(IrInstruction::Branch {
                            target: header_label,
                        });
                    }
                } else {
                    self.emit(IrInstruction::Branch {
                        target: header_label,
                    });
                }

                self.do_exit_stack.pop();

                let exit_idx = self.create_block(&exit_label);
                self.switch_to_block(exit_idx);
            }
            Statement::WhileWend {
                condition,
                body,
                ..
            } => {
                let header_label = self.next_label("while_header");
                let body_label = self.next_label("while_body");
                let exit_label = self.next_label("while_exit");

                self.emit(IrInstruction::Branch {
                    target: header_label.clone(),
                });

                let header_idx = self.create_block(&header_label);
                self.switch_to_block(header_idx);

                let cond_op = self.lower_expression(condition);
                self.emit(IrInstruction::BranchIf {
                    condition: cond_op,
                    true_target: body_label.clone(),
                    false_target: exit_label.clone(),
                });

                let body_idx = self.create_block(&body_label);
                self.switch_to_block(body_idx);
                for s in body {
                    self.lower_statement(s);
                }

                self.emit(IrInstruction::Branch {
                    target: header_label,
                });

                let exit_idx = self.create_block(&exit_label);
                self.switch_to_block(exit_idx);
            }
            Statement::With { target, body, .. } => {
                let (base_op, base_ty) = match target {
                    Expression::Variable(name) => self.resolve_variable(name),
                    Expression::WithMemberAccess(member) => {
                        if let Some(ctx) = self.with_stack.last().cloned() {
                            let struct_name = match &ctx.vb_type {
                                VbType::UserDefined(s) => s.clone(),
                                _ => "".to_string(),
                            };
                            if let Some(layout) = self.udt_layouts.get(&struct_name.to_ascii_uppercase()).cloned() {
                                if let Some(m) = layout.members.iter().find(|m| m.name.eq_ignore_ascii_case(member)).cloned() {
                                    let dest = self.next_temp();
                                    self.temp_types.insert(dest, m.vb_type.clone());
                                    self.emit(IrInstruction::GetMember {
                                        dest,
                                        base: ctx.base_operand,
                                        offset: m.offset,
                                        member_type: m.vb_type.clone(),
                                    });
                                    (Operand::Temp(dest), m.vb_type)
                                } else {
                                    (Operand::Local("unknown".to_string()), VbType::Unknown)
                                }
                            } else {
                                (Operand::Local("unknown".to_string()), VbType::Unknown)
                            }
                        } else {
                            (Operand::Local("unknown".to_string()), VbType::Unknown)
                        }
                    }
                    Expression::MemberAccess { object, member } => {
                        if let Some((b_op, offset, m_ty)) = self.resolve_member_access(object, member) {
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, m_ty.clone());
                            self.emit(IrInstruction::GetMember {
                                dest,
                                base: b_op,
                                offset,
                                member_type: m_ty.clone(),
                            });
                            (Operand::Temp(dest), m_ty)
                        } else {
                            (Operand::Local("unknown".to_string()), VbType::Unknown)
                        }
                    }
                    _ => {
                        let op = self.lower_expression(target);
                        let ty = self.infer_operand_type(&op);
                        (op, ty)
                    }
                };

                self.with_stack.push(WithContext {
                    base_operand: base_op,
                    vb_type: base_ty,
                });
                for s in body {
                    self.lower_statement(s);
                }
                self.with_stack.pop();
            }
            Statement::SelectCase {
                expression,
                cases,
                else_case,
                ..
            } => {
                let test_op = self.lower_expression(expression);
                let test_ty = self.infer_operand_type(&test_op);
                let select_val_name = format!("__select_val_{}", self.temp_counter);
                self.temp_counter += 1;
                self.local_vars.insert(
                    select_val_name.to_ascii_uppercase(),
                    (select_val_name.clone(), test_ty.clone()),
                );
                let select_dest = self.next_temp();
                self.emit(IrInstruction::AllocLocal {
                    dest: select_dest,
                    name: select_val_name.clone(),
                    vb_type: test_ty.clone(),
                });
                self.emit(IrInstruction::Store {
                    dest: Operand::Local(select_val_name.clone()),
                    src: test_op,
                });

                let exit_label = self.next_label("select_exit");

                for case_clause in cases {
                    let case_body_label = self.next_label("case_body");
                    let next_case_label = self.next_label("case_next");

                    for (val_idx, case_val) in case_clause.values.iter().enumerate() {
                        let is_last_val = val_idx + 1 == case_clause.values.len();
                        let on_fail_label = if is_last_val {
                            next_case_label.clone()
                        } else {
                            self.next_label("case_val_next")
                        };

                        let curr_test = self.next_temp();
                        self.emit(IrInstruction::Load {
                            dest: curr_test,
                            src: Operand::Local(select_val_name.clone()),
                        });

                        match case_val {
                            CaseValue::Exact(e) => {
                                let val_op = self.lower_expression(e);
                                let cmp_temp = self.next_temp();
                                self.emit(IrInstruction::Compare {
                                    dest: cmp_temp,
                                    op: IrCompareOp::Equal,
                                    left: Operand::Temp(curr_test),
                                    right: val_op,
                                });
                                self.emit(IrInstruction::BranchIf {
                                    condition: Operand::Temp(cmp_temp),
                                    true_target: case_body_label.clone(),
                                    false_target: on_fail_label.clone(),
                                });
                            }
                            CaseValue::Comparison(bin_op, e) => {
                                let val_op = self.lower_expression(e);
                                let ir_op = match bin_op {
                                    BinaryOperator::Equal => IrCompareOp::Equal,
                                    BinaryOperator::NotEqual => IrCompareOp::NotEqual,
                                    BinaryOperator::LessThan => IrCompareOp::LessThan,
                                    BinaryOperator::LessThanOrEqual => IrCompareOp::LessThanOrEqual,
                                    BinaryOperator::GreaterThan => IrCompareOp::GreaterThan,
                                    BinaryOperator::GreaterThanOrEqual => IrCompareOp::GreaterThanOrEqual,
                                    _ => IrCompareOp::Equal,
                                };
                                let cmp_temp = self.next_temp();
                                self.emit(IrInstruction::Compare {
                                    dest: cmp_temp,
                                    op: ir_op,
                                    left: Operand::Temp(curr_test),
                                    right: val_op,
                                });
                                self.emit(IrInstruction::BranchIf {
                                    condition: Operand::Temp(cmp_temp),
                                    true_target: case_body_label.clone(),
                                    false_target: on_fail_label.clone(),
                                });
                            }
                            CaseValue::Range(low_e, high_e) => {
                                let low_op = self.lower_expression(low_e);
                                let high_op = self.lower_expression(high_e);
                                let low_cmp = self.next_temp();
                                self.emit(IrInstruction::Compare {
                                    dest: low_cmp,
                                    op: IrCompareOp::GreaterThanOrEqual,
                                    left: Operand::Temp(curr_test),
                                    right: low_op,
                                });
                                let check_high_label = self.next_label("range_check_high");
                                self.emit(IrInstruction::BranchIf {
                                    condition: Operand::Temp(low_cmp),
                                    true_target: check_high_label.clone(),
                                    false_target: on_fail_label.clone(),
                                });

                                let check_high_idx = self.create_block(&check_high_label);
                                self.switch_to_block(check_high_idx);

                                let high_test = self.next_temp();
                                self.emit(IrInstruction::Load {
                                    dest: high_test,
                                    src: Operand::Local(select_val_name.clone()),
                                });
                                let high_cmp = self.next_temp();
                                self.emit(IrInstruction::Compare {
                                    dest: high_cmp,
                                    op: IrCompareOp::LessThanOrEqual,
                                    left: Operand::Temp(high_test),
                                    right: high_op,
                                });
                                self.emit(IrInstruction::BranchIf {
                                    condition: Operand::Temp(high_cmp),
                                    true_target: case_body_label.clone(),
                                    false_target: on_fail_label.clone(),
                                });
                            }
                        }

                        if !is_last_val {
                            let next_val_idx = self.create_block(&on_fail_label);
                            self.switch_to_block(next_val_idx);
                        }
                    }

                    // Body of this case clause
                    let body_idx = self.create_block(&case_body_label);
                    self.switch_to_block(body_idx);
                    for s in &case_clause.body {
                        self.lower_statement(s);
                    }
                    self.emit(IrInstruction::Branch {
                        target: exit_label.clone(),
                    });

                    // Switch to next case
                    let next_case_idx = self.create_block(&next_case_label);
                    self.switch_to_block(next_case_idx);
                }

                if let Some(eb) = else_case {
                    for s in eb {
                        self.lower_statement(s);
                    }
                    self.emit(IrInstruction::Branch {
                        target: exit_label.clone(),
                    });
                } else {
                    self.emit(IrInstruction::Branch {
                        target: exit_label.clone(),
                    });
                }

                let exit_idx = self.create_block(&exit_label);
                self.switch_to_block(exit_idx);
            }
            Statement::Exit { target, .. } => match target {
                ExitTarget::Sub | ExitTarget::Function | ExitTarget::Property => {
                    self.emit(IrInstruction::Return { value: None });
                    let dead_label = self.next_label("after_exit_proc");
                    let dead_idx = self.create_block(&dead_label);
                    self.switch_to_block(dead_idx);
                }
                ExitTarget::For => {
                    if let Some(target_label) = self.for_exit_stack.last() {
                        self.emit(IrInstruction::Branch {
                            target: target_label.clone(),
                        });
                        let dead_label = self.next_label("after_exit_for");
                        let dead_idx = self.create_block(&dead_label);
                        self.switch_to_block(dead_idx);
                    }
                }
                ExitTarget::Do => {
                    if let Some(target_label) = self.do_exit_stack.last() {
                        self.emit(IrInstruction::Branch {
                            target: target_label.clone(),
                        });
                        let dead_label = self.next_label("after_exit_do");
                        let dead_idx = self.create_block(&dead_label);
                        self.switch_to_block(dead_idx);
                    }
                }
            },
            Statement::Call {
                name, arguments, ..
            } => {
                let mut args_ops: Vec<Operand> = arguments
                    .iter()
                    .map(|arg| self.lower_expression(arg))
                    .collect();

                if name.contains('.') {
                    let parts: Vec<&str> = name.split('.').collect();
                    if parts.len() == 2 {
                        let obj_name = parts[0];
                        let method_name = parts[1];
                        let (base_op, cls_name) = if obj_name.eq_ignore_ascii_case("Me") {
                            let this_temp = self.next_temp();
                            let cls = self.current_class.clone().unwrap_or_default();
                            self.temp_types.insert(this_temp, VbType::Class(cls.clone()));
                            self.emit(IrInstruction::Load {
                                dest: this_temp,
                                src: Operand::Local("this".to_string()),
                            });
                            (Operand::Temp(this_temp), cls)
                        } else {
                            let (target_op, target_ty) = self.resolve_variable(obj_name);
                            let cls = match target_ty {
                                VbType::Class(c) => c,
                                _ => "".to_string(),
                            };
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, VbType::Class(cls.clone()));
                            self.emit(IrInstruction::Load {
                                dest,
                                src: target_op,
                            });
                            (Operand::Temp(dest), cls)
                        };
                        args_ops.insert(0, base_op);
                        self.emit(IrInstruction::Call {
                            dest: None,
                            name: format!("{}__{}", cls_name, method_name),
                            args: args_ops,
                            return_type: None,
                        });
                        return;
                    }
                }

                if let Some(cls_name) = self.current_class.clone() {
                    let is_method = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                        .map_or(false, |l| l.find_method(name).is_some());
                    if is_method {
                        let this_temp = self.next_temp();
                        self.temp_types.insert(this_temp, VbType::Class(cls_name.clone()));
                        self.emit(IrInstruction::Load {
                            dest: this_temp,
                            src: Operand::Local("this".to_string()),
                        });
                        args_ops.insert(0, Operand::Temp(this_temp));
                        self.emit(IrInstruction::Call {
                            dest: None,
                            name: format!("{}__{}", cls_name, name),
                            args: args_ops,
                            return_type: None,
                        });
                        return;
                    }
                }

                self.emit(IrInstruction::Call {
                    dest: None,
                    name: name.clone(),
                    args: args_ops,
                    return_type: None,
                });
            }
            Statement::DebugPrint { arguments, .. } => {
                let args_ops = arguments
                    .iter()
                    .map(|arg| self.lower_expression(arg))
                    .collect();
                self.emit(IrInstruction::DebugPrint { args: args_ops });
            }
            Statement::OnError { action, .. } => match action {
                OnErrorAction::GoTo(label) => {
                    self.emit(IrInstruction::PushErrorHandler {
                        target: label.clone(),
                    });
                }
                OnErrorAction::ResumeNext => {
                    self.emit(IrInstruction::PushErrorHandler {
                        target: "__resume_next".to_string(),
                    });
                }
                OnErrorAction::GoToZero => {
                    self.emit(IrInstruction::PopErrorHandler);
                }
            },
            Statement::Resume { action, .. } => match action {
                ResumeAction::Label(lbl) => {
                    self.emit(IrInstruction::Resume {
                        target: Some(lbl.clone()),
                    });
                }
                ResumeAction::Next => {
                    self.emit(IrInstruction::Resume {
                        target: Some("__resume_next".to_string()),
                    });
                }
                ResumeAction::Current => {
                    self.emit(IrInstruction::Resume { target: None });
                }
            },
            Statement::GoTo { label, .. } => {
                self.emit(IrInstruction::Branch {
                    target: label.clone(),
                });
            }
            Statement::LineLabel { name, .. } => {
                self.emit(IrInstruction::Branch {
                    target: name.clone(),
                });
                let new_idx = self.create_block(name);
                self.switch_to_block(new_idx);
            }
            _ => {}
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> Operand {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Integer(i) => Operand::Constant(IrConstant::Int(*i as i64, VbType::Long)),
                Literal::Double(d) => Operand::Constant(IrConstant::Float(*d)),
                Literal::String(s) => Operand::Constant(IrConstant::String(s.clone())),
                Literal::Boolean(b) => Operand::Constant(IrConstant::Bool(*b)),
                Literal::Date(d) => Operand::Constant(IrConstant::Date(d.clone())),
                Literal::Null => Operand::Constant(IrConstant::Null),
                Literal::Empty => Operand::Constant(IrConstant::Empty),
                Literal::Nothing => Operand::Constant(IrConstant::Nothing),
            },
            Expression::Variable(name) => {
                if let Some(cls_name) = self.current_class.clone() {
                    if !self.local_vars.contains_key(&name.to_ascii_uppercase()) {
                        let prop_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_property(name).cloned());
                        if let Some(p) = prop_info {
                            let this_temp = self.next_temp();
                            self.temp_types.insert(this_temp, VbType::Class(cls_name.clone()));
                            self.emit(IrInstruction::Load {
                                dest: this_temp,
                                src: Operand::Local("this".to_string()),
                            });
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, p.prop_type.clone());
                            self.emit(IrInstruction::Call {
                                dest: Some(dest),
                                name: format!("{}__{}__Get", cls_name, name),
                                args: vec![Operand::Temp(this_temp)],
                                return_type: Some(p.prop_type),
                            });
                            return Operand::Temp(dest);
                        }

                        let field_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_field(name).map(|f| (f.offset, f.vb_type.clone())));
                        if let Some((f_offset, f_type)) = field_info {
                            let this_temp = self.next_temp();
                            self.temp_types.insert(this_temp, VbType::Class(cls_name.clone()));
                            self.emit(IrInstruction::Load {
                                dest: this_temp,
                                src: Operand::Local("this".to_string()),
                            });
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, f_type.clone());
                            self.emit(IrInstruction::GetMember {
                                dest,
                                base: Operand::Temp(this_temp),
                                offset: f_offset,
                                member_type: f_type,
                            });
                            return Operand::Temp(dest);
                        }
                    }
                }

                let (target_op, target_ty) = self.resolve_variable(name);
                if matches!(target_ty, VbType::UserDefined(_)) {
                    return target_op;
                }
                if matches!(target_op, Operand::Constant(_)) {
                    return target_op;
                }
                let temp = self.next_temp();
                self.temp_types.insert(temp, target_ty);
                self.emit(IrInstruction::Load {
                    dest: temp,
                    src: target_op,
                });
                Operand::Temp(temp)
            }
            Expression::BinaryOp { op, left, right } => {
                let left_op = self.lower_expression(left);
                let right_op = self.lower_expression(right);
                let dest = self.next_temp();

                match op {
                    BinaryOperator::Concat => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Concat,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            let l_op = self.coerce_to_string(left_op);
                            let r_op = self.coerce_to_string(right_op);
                            self.temp_types.insert(dest, VbType::String);
                            self.emit(IrInstruction::StringConcat {
                                dest,
                                left: l_op,
                                right: r_op,
                            });
                        }
                    }
                    BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        let (l_op, r_op) = if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            (self.coerce_to_variant(left_op), self.coerce_to_variant(right_op))
                        } else if left_ty == VbType::String || right_ty == VbType::String {
                            (self.coerce_to_string(left_op), self.coerce_to_string(right_op))
                        } else {
                            (left_op, right_op)
                        };
                        let cmp_op = match op {
                            BinaryOperator::Equal => IrCompareOp::Equal,
                            BinaryOperator::NotEqual => IrCompareOp::NotEqual,
                            BinaryOperator::LessThan => IrCompareOp::LessThan,
                            BinaryOperator::LessThanOrEqual => IrCompareOp::LessThanOrEqual,
                            BinaryOperator::GreaterThan => IrCompareOp::GreaterThan,
                            BinaryOperator::GreaterThanOrEqual => IrCompareOp::GreaterThanOrEqual,
                            _ => unreachable!(),
                        };
                        self.temp_types.insert(dest, VbType::Boolean);
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: cmp_op,
                            left: l_op,
                            right: r_op,
                        });
                    }
                    BinaryOperator::Is => {
                        self.temp_types.insert(dest, VbType::Boolean);
                        self.emit(IrInstruction::Compare {
                            dest,
                            op: IrCompareOp::Is,
                            left: left_op,
                            right: right_op,
                        });
                    }
                    BinaryOperator::Add => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Add,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else if left_ty == VbType::String || right_ty == VbType::String {
                            let l_op = self.coerce_to_string(left_op);
                            let r_op = self.coerce_to_string(right_op);
                            self.temp_types.insert(dest, VbType::String);
                            self.emit(IrInstruction::StringConcat {
                                dest,
                                left: l_op,
                                right: r_op,
                            });
                        } else {
                            let res_ty = crate::types::promote_binary_operator(&left_ty, op, &right_ty)
                                .unwrap_or(VbType::Long);
                            self.temp_types.insert(dest, res_ty.clone());
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Add,
                                left: left_op,
                                right: right_op,
                                result_type: res_ty,
                            });
                        }
                    }
                    BinaryOperator::Subtract => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Sub,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            let res_ty = crate::types::promote_binary_operator(&left_ty, op, &right_ty)
                                .unwrap_or(VbType::Long);
                            self.temp_types.insert(dest, res_ty.clone());
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Sub,
                                left: left_op,
                                right: right_op,
                                result_type: res_ty,
                            });
                        }
                    }
                    BinaryOperator::Multiply => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Mul,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            let res_ty = crate::types::promote_binary_operator(&left_ty, op, &right_ty)
                                .unwrap_or(VbType::Long);
                            self.temp_types.insert(dest, res_ty.clone());
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Mul,
                                left: left_op,
                                right: right_op,
                                result_type: res_ty,
                            });
                        }
                    }
                    BinaryOperator::Divide => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Div,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Div,
                                left: left_op,
                                right: right_op,
                                result_type: VbType::Double,
                            });
                        }
                    }
                    BinaryOperator::IntegerDivide => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::IntDiv,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::IntDiv,
                                left: left_op,
                                right: right_op,
                                result_type: VbType::Long,
                            });
                        }
                    }
                    BinaryOperator::Modulo => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Mod,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::Mod,
                                left: left_op,
                                right: right_op,
                                result_type: VbType::Long,
                            });
                        }
                    }
                    BinaryOperator::And => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::BitAnd,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::BitAnd,
                                left: left_op,
                                right: right_op,
                                result_type: VbType::Boolean,
                            });
                        }
                    }
                    BinaryOperator::Or => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::BitOr,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::BitOr,
                                left: left_op,
                                right: right_op,
                                result_type: VbType::Boolean,
                            });
                        }
                    }
                    BinaryOperator::Xor => {
                        let left_ty = self.infer_operand_type(&left_op);
                        let right_ty = self.infer_operand_type(&right_op);
                        if left_ty == VbType::Variant || right_ty == VbType::Variant {
                            let l_op = self.coerce_to_variant(left_op);
                            let r_op = self.coerce_to_variant(right_op);
                            self.temp_types.insert(dest, VbType::Variant);
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::BitXor,
                                left: l_op,
                                right: r_op,
                                result_type: VbType::Variant,
                            });
                        } else {
                            self.emit(IrInstruction::BinaryOp {
                                dest,
                                op: IrBinaryOp::BitXor,
                                left: left_op,
                                right: right_op,
                                result_type: VbType::Boolean,
                            });
                        }
                    }
                    _ => {
                        self.emit(IrInstruction::BinaryOp {
                            dest,
                            op: IrBinaryOp::Add,
                            left: left_op,
                            right: right_op,
                            result_type: VbType::Variant,
                        });
                    }
                }

                Operand::Temp(dest)
            }
            Expression::UnaryOp { op, operand } => {
                let opnd = self.lower_expression(operand);
                let opnd_ty = self.infer_operand_type(&opnd);
                let dest = self.next_temp();
                if opnd_ty == VbType::Variant {
                    let var_op = self.coerce_to_variant(opnd);
                    let ir_op = match op {
                        UnaryOperator::Negate => IrUnaryOp::Negate,
                        UnaryOperator::Not => IrUnaryOp::BitNot,
                        UnaryOperator::Plus => return var_op,
                    };
                    self.temp_types.insert(dest, VbType::Variant);
                    self.emit(IrInstruction::UnaryOp {
                        dest,
                        op: ir_op,
                        operand: var_op,
                        result_type: VbType::Variant,
                    });
                } else {
                    let ir_op = match op {
                        UnaryOperator::Negate => IrUnaryOp::Negate,
                        UnaryOperator::Not => IrUnaryOp::BitNot,
                        UnaryOperator::Plus => return opnd,
                    };
                    let res_ty = if opnd_ty == VbType::Boolean {
                        VbType::Boolean
                    } else if opnd_ty == VbType::Integer {
                        VbType::Integer
                    } else if opnd_ty == VbType::Byte {
                        VbType::Byte
                    } else if opnd_ty == VbType::Long {
                        VbType::Long
                    } else {
                        opnd_ty.clone()
                    };
                    self.temp_types.insert(dest, res_ty.clone());
                    self.emit(IrInstruction::UnaryOp {
                        dest,
                        op: ir_op,
                        operand: opnd,
                        result_type: res_ty,
                    });
                }
                Operand::Temp(dest)
            }
            Expression::Paren(inner) => self.lower_expression(inner),
            Expression::IndexAccess { target, indices } => {
                // 1. Check LBound / UBound
                if let Expression::Variable(name) = &**target {
                    if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
                        let is_ubound = name.eq_ignore_ascii_case("UBound");
                        let array_op = self.lower_expression(&indices[0]);
                        let dim_op = if indices.len() > 1 {
                            self.lower_expression(&indices[1])
                        } else {
                            Operand::Constant(IrConstant::Int(1, VbType::Long))
                        };
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Long);
                        self.emit(IrInstruction::ArrayBound {
                            dest,
                            array: array_op,
                            dim: dim_op,
                            is_ubound,
                        });
                        return Operand::Temp(dest);
                    }
                }

                // 2. Check intrinsic string functions
                if let Expression::Variable(name) = &**target {
                    if name.eq_ignore_ascii_case("Len") {
                        let arg_raw = self.lower_expression(&indices[0]);
                        let arg_op = self.coerce_to_string(arg_raw);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Long);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_len".to_string(),
                            args: vec![arg_op],
                            return_type: Some(VbType::Long),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Left") || name.eq_ignore_ascii_case("Left$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let n_op = self.lower_expression(&indices[1]);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_left".to_string(),
                            args: vec![s_op, n_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Right") || name.eq_ignore_ascii_case("Right$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let n_op = self.lower_expression(&indices[1]);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_right".to_string(),
                            args: vec![s_op, n_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Mid") || name.eq_ignore_ascii_case("Mid$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let start_op = self.lower_expression(&indices[1]);
                        let len_op = if indices.len() > 2 {
                            self.lower_expression(&indices[2])
                        } else {
                            Operand::Constant(IrConstant::Int(-1, VbType::Long))
                        };
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_mid".to_string(),
                            args: vec![s_op, start_op, len_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("InStr") {
                        let (start_op, s1_raw, s2_raw) = if indices.len() >= 3 {
                            (self.lower_expression(&indices[0]), self.lower_expression(&indices[1]), self.lower_expression(&indices[2]))
                        } else {
                            (Operand::Constant(IrConstant::Int(1, VbType::Long)), self.lower_expression(&indices[0]), self.lower_expression(&indices[1]))
                        };
                        let s1_op = self.coerce_to_string(s1_raw);
                        let s2_op = self.coerce_to_string(s2_raw);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Long);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_instr".to_string(),
                            args: vec![start_op, s1_op, s2_op],
                            return_type: Some(VbType::Long),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("UCase") || name.eq_ignore_ascii_case("UCase$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_ucase".to_string(),
                            args: vec![s_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("LCase") || name.eq_ignore_ascii_case("LCase$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_lcase".to_string(),
                            args: vec![s_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Trim") || name.eq_ignore_ascii_case("Trim$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_trim".to_string(),
                            args: vec![s_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("LTrim") || name.eq_ignore_ascii_case("LTrim$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_ltrim".to_string(),
                            args: vec![s_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("RTrim") || name.eq_ignore_ascii_case("RTrim$") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_rtrim".to_string(),
                            args: vec![s_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Space") || name.eq_ignore_ascii_case("Space$") {
                        let n_op = self.lower_expression(&indices[0]);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_space".to_string(),
                            args: vec![n_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("String") || name.eq_ignore_ascii_case("String$") {
                        let n_op = self.lower_expression(&indices[0]);
                        let raw_char = self.lower_expression(&indices[1]);
                        let raw_ty = self.infer_operand_type(&raw_char);
                        let char_op = if raw_ty.is_integer() {
                            let chr_dest = self.next_temp();
                            self.temp_types.insert(chr_dest, VbType::String);
                            self.emit(IrInstruction::Call {
                                dest: Some(chr_dest),
                                name: "vb6_string_chr".to_string(),
                                args: vec![raw_char],
                                return_type: Some(VbType::String),
                            });
                            Operand::Temp(chr_dest)
                        } else {
                            self.coerce_to_string(raw_char)
                        };
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_string".to_string(),
                            args: vec![n_op, char_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Replace") {
                        let raw_expr = self.lower_expression(&indices[0]);
                        let expr_op = self.coerce_to_string(raw_expr);
                        let raw_find = self.lower_expression(&indices[1]);
                        let find_op = self.coerce_to_string(raw_find);
                        let raw_repl = self.lower_expression(&indices[2]);
                        let repl_op = self.coerce_to_string(raw_repl);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_replace".to_string(),
                            args: vec![expr_op, find_op, repl_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Chr") || name.eq_ignore_ascii_case("Chr$")
                        || name.eq_ignore_ascii_case("ChrW") || name.eq_ignore_ascii_case("ChrW$")
                    {
                        let code_op = self.lower_expression(&indices[0]);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_chr".to_string(),
                            args: vec![code_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Asc") || name.eq_ignore_ascii_case("AscB") || name.eq_ignore_ascii_case("AscW") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Integer);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_asc".to_string(),
                            args: vec![s_op],
                            return_type: Some(VbType::Integer),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Val") {
                        let raw_s = self.lower_expression(&indices[0]);
                        let s_op = self.coerce_to_string(raw_s);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Double);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_val".to_string(),
                            args: vec![s_op],
                            return_type: Some(VbType::Double),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("Str") || name.eq_ignore_ascii_case("Str$") {
                        let v_op = self.lower_expression(&indices[0]);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_string_str".to_string(),
                            args: vec![v_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("VarType") {
                        let raw_op = self.lower_expression(&indices[0]);
                        let var_op = self.coerce_to_variant(raw_op);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Integer);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_variant_vartype".to_string(),
                            args: vec![var_op],
                            return_type: Some(VbType::Integer),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("TypeName") {
                        let raw_op = self.lower_expression(&indices[0]);
                        let var_op = self.coerce_to_variant(raw_op);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::String);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_variant_typename".to_string(),
                            args: vec![var_op],
                            return_type: Some(VbType::String),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("IsEmpty") {
                        let raw_op = self.lower_expression(&indices[0]);
                        let var_op = self.coerce_to_variant(raw_op);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Boolean);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_variant_is_empty".to_string(),
                            args: vec![var_op],
                            return_type: Some(VbType::Boolean),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("IsNull") {
                        let raw_op = self.lower_expression(&indices[0]);
                        let var_op = self.coerce_to_variant(raw_op);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Boolean);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_variant_is_null".to_string(),
                            args: vec![var_op],
                            return_type: Some(VbType::Boolean),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("IsNumeric") {
                        let raw_op = self.lower_expression(&indices[0]);
                        let var_op = self.coerce_to_variant(raw_op);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Boolean);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_variant_is_numeric".to_string(),
                            args: vec![var_op],
                            return_type: Some(VbType::Boolean),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("IsDate") {
                        let raw_op = self.lower_expression(&indices[0]);
                        let var_op = self.coerce_to_variant(raw_op);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Boolean);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_variant_is_date".to_string(),
                            args: vec![var_op],
                            return_type: Some(VbType::Boolean),
                        });
                        return Operand::Temp(dest);
                    }
                    if name.eq_ignore_ascii_case("IsArray") {
                        let raw_op = self.lower_expression(&indices[0]);
                        let var_op = self.coerce_to_variant(raw_op);
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, VbType::Boolean);
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: "vb6_variant_is_array".to_string(),
                            args: vec![var_op],
                            return_type: Some(VbType::Boolean),
                        });
                        return Operand::Temp(dest);
                    }
                    let conv_target = if name.eq_ignore_ascii_case("CBool") {
                        Some(VbType::Boolean)
                    } else if name.eq_ignore_ascii_case("CByte") {
                        Some(VbType::Byte)
                    } else if name.eq_ignore_ascii_case("CInt") {
                        Some(VbType::Integer)
                    } else if name.eq_ignore_ascii_case("CLng") {
                        Some(VbType::Long)
                    } else if name.eq_ignore_ascii_case("CSng") {
                        Some(VbType::Single)
                    } else if name.eq_ignore_ascii_case("CDbl") {
                        Some(VbType::Double)
                    } else if name.eq_ignore_ascii_case("CStr") {
                        Some(VbType::String)
                    } else if name.eq_ignore_ascii_case("CDate") {
                        Some(VbType::Date)
                    } else if name.eq_ignore_ascii_case("CCur") {
                        Some(VbType::Currency)
                    } else if name.eq_ignore_ascii_case("CVar") {
                        Some(VbType::Variant)
                    } else {
                        None
                    };
                    if let Some(target_type) = conv_target {
                        let raw_op = self.lower_expression(&indices[0]);
                        let from_ty = self.infer_operand_type(&raw_op);
                        if from_ty == target_type {
                            return raw_op;
                        }
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, target_type.clone());
                        self.emit(IrInstruction::Coerce {
                            dest,
                            src: raw_op,
                            from_type: from_ty,
                            to_type: target_type,
                        });
                        return Operand::Temp(dest);
                    }
                }

                // Check if target is a class method call: p.Method(args)
                if let Expression::MemberAccess { object, member } = &**target {
                    let obj_op = self.lower_expression(object);
                    let obj_ty = self.infer_operand_type(&obj_op);
                    if let VbType::Class(ref cls_name) = obj_ty {
                        let method_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_method(member).cloned());
                        if let Some(m) = method_info {
                            let actual_base = match obj_op {
                                Operand::Local(_) => {
                                    let b_dest = self.next_temp();
                                    self.temp_types.insert(b_dest, VbType::Class(cls_name.clone()));
                                    self.emit(IrInstruction::Load {
                                        dest: b_dest,
                                        src: obj_op,
                                    });
                                    Operand::Temp(b_dest)
                                }
                                _ => obj_op,
                            };
                            let mut args_ops = vec![actual_base];
                            for idx in indices {
                                args_ops.push(self.lower_expression(idx));
                            }
                            let ret_ty = m.return_type.unwrap_or(VbType::Variant);
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, ret_ty.clone());
                            self.emit(IrInstruction::Call {
                                dest: Some(dest),
                                name: format!("{}__{}", cls_name, member),
                                args: args_ops,
                                return_type: Some(ret_ty),
                            });
                            return Operand::Temp(dest);
                        }
                    }
                }

                if let Expression::WithMemberAccess(member) = &**target {
                    if let Some(ctx) = self.with_stack.last().cloned() {
                        if let VbType::Class(ref cls_name) = ctx.vb_type {
                            let method_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                                .and_then(|l| l.find_method(member).cloned());
                            if let Some(m) = method_info {
                                let actual_base = match ctx.base_operand {
                                    Operand::Local(_) => {
                                        let b_dest = self.next_temp();
                                        self.temp_types.insert(b_dest, VbType::Class(cls_name.clone()));
                                        self.emit(IrInstruction::Load {
                                            dest: b_dest,
                                            src: ctx.base_operand,
                                        });
                                        Operand::Temp(b_dest)
                                    }
                                    _ => ctx.base_operand,
                                };
                                let mut args_ops = vec![actual_base];
                                for idx in indices {
                                    args_ops.push(self.lower_expression(idx));
                                }
                                let ret_ty = m.return_type.unwrap_or(VbType::Variant);
                                let dest = self.next_temp();
                                self.temp_types.insert(dest, ret_ty.clone());
                                self.emit(IrInstruction::Call {
                                    dest: Some(dest),
                                    name: format!("{}__{}", cls_name, member),
                                    args: args_ops,
                                    return_type: Some(ret_ty),
                                });
                                return Operand::Temp(dest);
                            }
                        }
                    }
                }

                // Check function call or current_class method
                if let Expression::Variable(name) = &**target {
                    if let Some(cls_name) = self.current_class.clone() {
                        let method_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_method(name).cloned());
                        if let Some(m) = method_info {
                            let this_temp = self.next_temp();
                            self.temp_types.insert(this_temp, VbType::Class(cls_name.clone()));
                            self.emit(IrInstruction::Load {
                                dest: this_temp,
                                src: Operand::Local("this".to_string()),
                            });
                            let mut args_ops = vec![Operand::Temp(this_temp)];
                            for idx in indices {
                                args_ops.push(self.lower_expression(idx));
                            }
                            let ret_ty = m.return_type.unwrap_or(VbType::Variant);
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, ret_ty.clone());
                            self.emit(IrInstruction::Call {
                                dest: Some(dest),
                                name: format!("{}__{}", cls_name, name),
                                args: args_ops,
                                return_type: Some(ret_ty),
                            });
                            return Operand::Temp(dest);
                        }
                    }

                    let key = name.to_ascii_uppercase();
                    if let Some((func_real_name, ret_ty)) = self.functions.get(&key).cloned() {
                        let args_ops: Vec<Operand> = indices.iter().map(|arg| self.lower_expression(arg)).collect();
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, ret_ty.clone());
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: func_real_name,
                            args: args_ops,
                            return_type: Some(ret_ty),
                        });
                        return Operand::Temp(dest);
                    }
                }

                // Array element indexing
                let arr_op = self.lower_expression(target);
                let arr_ty = self.infer_operand_type(&arr_op);
                let elem_type = match arr_ty {
                    VbType::Array(elem) => *elem,
                    _ => VbType::Long,
                };
                let idx_ops: Vec<Operand> = indices.iter().map(|idx| self.lower_expression(idx)).collect();
                let dest = self.next_temp();
                self.temp_types.insert(dest, elem_type.clone());
                self.emit(IrInstruction::ArrayGetElement {
                    dest,
                    array: arr_op,
                    indices: idx_ops,
                    elem_type,
                });
                Operand::Temp(dest)
            }
            Expression::MemberAccess { object, member } => {
                if let Expression::Variable(enum_name) = object.as_ref() {
                    let key = enum_name.to_ascii_uppercase();
                    if let Some(enum_def) = self.enum_defs.get(&key) {
                        if let Some(m) = enum_def.members.iter().find(|m| m.name.eq_ignore_ascii_case(member)) {
                            return Operand::Constant(IrConstant::Int(m.value as i64, VbType::Long));
                        }
                    }
                }

                let obj_op = self.lower_expression(object);
                let obj_ty = self.infer_operand_type(&obj_op);
                if let VbType::Class(ref cls_name) = obj_ty {
                    let prop_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                        .and_then(|l| l.find_property(member).cloned());
                    if let Some(p) = prop_info {
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, p.prop_type.clone());
                        let actual_base = match obj_op {
                            Operand::Local(_) => {
                                let b_dest = self.next_temp();
                                self.temp_types.insert(b_dest, VbType::Class(cls_name.clone()));
                                self.emit(IrInstruction::Load {
                                    dest: b_dest,
                                    src: obj_op,
                                });
                                Operand::Temp(b_dest)
                            }
                            _ => obj_op,
                        };
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: format!("{}__{}__Get", cls_name, member),
                            args: vec![actual_base],
                            return_type: Some(p.prop_type),
                        });
                        return Operand::Temp(dest);
                    }

                    let method_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                        .and_then(|l| l.find_method(member).cloned());
                    if let Some(m) = method_info {
                        let dest = self.next_temp();
                        let ret_ty = m.return_type.unwrap_or(VbType::Variant);
                        self.temp_types.insert(dest, ret_ty.clone());
                        let actual_base = match obj_op {
                            Operand::Local(_) => {
                                let b_dest = self.next_temp();
                                self.temp_types.insert(b_dest, VbType::Class(cls_name.clone()));
                                self.emit(IrInstruction::Load {
                                    dest: b_dest,
                                    src: obj_op,
                                });
                                Operand::Temp(b_dest)
                            }
                            _ => obj_op,
                        };
                        self.emit(IrInstruction::Call {
                            dest: Some(dest),
                            name: format!("{}__{}", cls_name, member),
                            args: vec![actual_base],
                            return_type: Some(ret_ty),
                        });
                        return Operand::Temp(dest);
                    }
                }

                if let Some((base_op, offset, member_type)) = self.resolve_member_access(object, member) {
                    let dest = self.next_temp();
                    self.temp_types.insert(dest, member_type.clone());
                    self.emit(IrInstruction::GetMember {
                        dest,
                        base: base_op,
                        offset,
                        member_type,
                    });
                    Operand::Temp(dest)
                } else {
                    Operand::Constant(IrConstant::Int(0, VbType::Long))
                }
            }
            Expression::WithMemberAccess(member) => {
                if let Some(ctx) = self.with_stack.last().cloned() {
                    if let VbType::Class(ref cls_name) = ctx.vb_type {
                        let prop_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_property(member).cloned());
                        if let Some(p) = prop_info {
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, p.prop_type.clone());
                            let actual_base = match ctx.base_operand {
                                Operand::Local(_) => {
                                    let b_dest = self.next_temp();
                                    self.temp_types.insert(b_dest, VbType::Class(cls_name.clone()));
                                    self.emit(IrInstruction::Load {
                                        dest: b_dest,
                                        src: ctx.base_operand,
                                    });
                                    Operand::Temp(b_dest)
                                }
                                _ => ctx.base_operand,
                            };
                            self.emit(IrInstruction::Call {
                                dest: Some(dest),
                                name: format!("{}__{}__Get", cls_name, member),
                                args: vec![actual_base],
                                return_type: Some(p.prop_type),
                            });
                            return Operand::Temp(dest);
                        }

                        let method_info = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_method(member).cloned());
                        if let Some(m) = method_info {
                            let dest = self.next_temp();
                            let ret_ty = m.return_type.unwrap_or(VbType::Variant);
                            self.temp_types.insert(dest, ret_ty.clone());
                            let actual_base = match ctx.base_operand {
                                Operand::Local(_) => {
                                    let b_dest = self.next_temp();
                                    self.temp_types.insert(b_dest, VbType::Class(cls_name.clone()));
                                    self.emit(IrInstruction::Load {
                                        dest: b_dest,
                                        src: ctx.base_operand,
                                    });
                                    Operand::Temp(b_dest)
                                }
                                _ => ctx.base_operand,
                            };
                            self.emit(IrInstruction::Call {
                                dest: Some(dest),
                                name: format!("{}__{}", cls_name, member),
                                args: vec![actual_base],
                                return_type: Some(ret_ty),
                            });
                            return Operand::Temp(dest);
                        }

                        if let Some((offset, member_type)) = self.class_layouts.get(&cls_name.to_ascii_uppercase())
                            .and_then(|l| l.find_field(member).map(|f| (f.offset, f.vb_type.clone())))
                        {
                            let dest = self.next_temp();
                            self.temp_types.insert(dest, member_type.clone());
                            let actual_base = match ctx.base_operand {
                                Operand::Local(_) => {
                                    let b_dest = self.next_temp();
                                    self.temp_types.insert(b_dest, VbType::Class(cls_name.clone()));
                                    self.emit(IrInstruction::Load {
                                        dest: b_dest,
                                        src: ctx.base_operand,
                                    });
                                    Operand::Temp(b_dest)
                                }
                                _ => ctx.base_operand,
                            };
                            self.emit(IrInstruction::GetMember {
                                dest,
                                base: actual_base,
                                offset,
                                member_type,
                            });
                            return Operand::Temp(dest);
                        }
                    }

                    let struct_name = match &ctx.vb_type {
                        VbType::UserDefined(s) => s.clone(),
                        _ => "".to_string(),
                    };
                    let member_info = if let Some(layout) = self.udt_layouts.get(&struct_name.to_ascii_uppercase()) {
                        layout
                            .members
                            .iter()
                            .find(|m| m.name.eq_ignore_ascii_case(member))
                            .map(|m| (m.offset, m.vb_type.clone()))
                    } else {
                        None
                    };
                    if let Some((offset, member_type)) = member_info {
                        let dest = self.next_temp();
                        self.temp_types.insert(dest, member_type.clone());
                        self.emit(IrInstruction::GetMember {
                            dest,
                            base: ctx.base_operand,
                            offset,
                            member_type,
                        });
                        return Operand::Temp(dest);
                    }
                }
                Operand::Constant(IrConstant::Int(0, VbType::Long))
            }
            Expression::New(cls) => {
                let dest = self.next_temp();
                self.temp_types.insert(dest, VbType::Class(cls.clone()));
                self.emit(IrInstruction::NewObject {
                    dest,
                    class_name: cls.clone(),
                });
                Operand::Temp(dest)
            }
            Expression::Me => {
                let temp = self.next_temp();
                let ty = self.current_class.as_ref().map(|c| VbType::Class(c.clone())).unwrap_or(VbType::Object);
                self.temp_types.insert(temp, ty);
                self.emit(IrInstruction::Load {
                    dest: temp,
                    src: Operand::Local("this".to_string()),
                });
                Operand::Temp(temp)
            }
        }
    }
}
