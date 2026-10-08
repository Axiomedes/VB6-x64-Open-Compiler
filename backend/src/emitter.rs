//! LLVM IR text emitter from VB6 IR.
//! Emits target-specific LLVM IR for Windows x64 (AMD64 / PE32+).

use std::collections::HashMap;
use vb6_compiler::ir::model::*;
use vb6_compiler::types::VbType;

pub struct LlvmEmitter {
    target_triple: String,
    data_layout: String,
}

impl Default for LlvmEmitter {
    fn default() -> Self {
        Self::new("x86_64-pc-windows-msvc")
    }
}

impl LlvmEmitter {
    pub fn new(target_triple: &str) -> Self {
        Self {
            target_triple: target_triple.to_string(),
            data_layout: "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
                .to_string(),
        }
    }

    /// Map a VB6 type to its corresponding LLVM IR type string.
    pub fn llvm_type(vb_type: &VbType) -> &'static str {
        match vb_type {
            VbType::Byte => "i8",
            VbType::Integer | VbType::Boolean => "i16",
            VbType::Long | VbType::Enum(_) => "i32",
            VbType::LongPtr | VbType::LongLong | VbType::Currency => "i64",
            VbType::Single => "float",
            VbType::Double | VbType::Date => "double",
            VbType::String | VbType::Object | VbType::Array(_) => "ptr",
            VbType::FixedString(_) => "ptr",
            VbType::Variant => "[24 x i8]",
            VbType::UserDefined(_) => "ptr",
            VbType::Unknown => "i32",
        }
    }

    /// Return default LLVM constant value for a type.
    pub fn llvm_default_val(vb_type: &VbType) -> &'static str {
        match vb_type {
            VbType::Byte | VbType::Integer | VbType::Boolean | VbType::Long | VbType::Enum(_) => "0",
            VbType::LongPtr | VbType::LongLong | VbType::Currency => "0",
            VbType::Single | VbType::Double | VbType::Date => "0.0",
            VbType::String | VbType::Object | VbType::Array(_) | VbType::FixedString(_) => "null",
            VbType::Variant => "zeroinitializer",
            VbType::UserDefined(_) => "null",
            VbType::Unknown => "0",
        }
    }

    /// Translate an entire IrModule into LLVM IR.
    pub fn emit_module(&self, module: &IrModule, emit_main_wrapper: bool) -> String {
        let mut out = String::new();

        // Header: Data layout and target triple
        out.push_str(&format!("; ModuleID = '{}'\n", module.name));
        out.push_str(&format!("target datalayout = \"{}\"\n", self.data_layout));
        out.push_str(&format!("target triple = \"{}\"\n\n", self.target_triple));

        // Declarations of external runtime helpers
        out.push_str("; Runtime externals\n");
        out.push_str("declare void @vb6_init()\n");
        out.push_str("declare void @vb6_shutdown()\n");
        out.push_str("declare i32 @vb6_msgbox(ptr, ptr, i32)\n");
        out.push_str("declare void @vb6_debug_print(ptr)\n\n");

        // Module-level global variables
        for (name, ty) in &module.globals {
            let llvm_ty = Self::llvm_type(ty);
            let def_val = Self::llvm_default_val(ty);
            let align = ty.alignment(true);
            out.push_str(&format!(
                "@\"{}\" = global {} {}, align {}\n",
                name, llvm_ty, def_val, align
            ));
        }
        if !module.globals.is_empty() {
            out.push('\n');
        }

        let mut has_main_sub = false;

        // Functions
        for func in &module.functions {
            if func.name.eq_ignore_ascii_case("Main") {
                has_main_sub = true;
            }
            out.push_str(&self.emit_function(func));
            out.push('\n');
        }

        // Standard C main entry point wrapper
        if emit_main_wrapper && has_main_sub {
            out.push_str("; Standard executable entry point wrapper\n");
            out.push_str("define i32 @main() {\n");
            out.push_str("entry:\n");
            out.push_str("  call void @vb6_init()\n");
            out.push_str("  call void @\"Main\"()\n");
            out.push_str("  call void @vb6_shutdown()\n");
            out.push_str("  ret i32 0\n");
            out.push_str("}\n");
        }

        out
    }

    fn emit_function(&self, func: &IrFunction) -> String {
        let mut out = String::new();

        let ret_llvm_ty = func
            .return_type
            .as_ref()
            .map(Self::llvm_type)
            .unwrap_or("void");

        // Format parameters
        let params_str: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                if p.is_by_val {
                    format!("{} %\"arg_{}\"", Self::llvm_type(&p.vb_type), p.name)
                } else {
                    format!("ptr %\"arg_{}\"", p.name)
                }
            })
            .collect();

        out.push_str(&format!(
            "define {} @\"{}\"({}) {{\n",
            ret_llvm_ty,
            func.name,
            params_str.join(", ")
        ));

        // Track local alloca types: name -> (llvm_type, align)
        let mut local_types: HashMap<String, (String, usize)> = HashMap::new();
        // Track temp types: temp_id -> llvm_type
        let mut temp_types: HashMap<usize, String> = HashMap::new();

        // First, inspect instructions in all blocks to pre-collect local alloca types
        for block in &func.blocks {
            for inst in &block.instructions {
                if let IrInstruction::AllocLocal { name, vb_type, .. } = inst {
                    local_types.insert(
                        name.to_ascii_uppercase(),
                        (
                            Self::llvm_type(vb_type).to_string(),
                            vb_type.alignment(true).max(1),
                        ),
                    );
                }
            }
        }

        // Emit blocks
        for (block_idx, block) in func.blocks.iter().enumerate() {
            out.push_str(&format!("{}:\n", block.label));

            // In entry block, store ByVal arguments into locals if matching
            if block_idx == 0 {
                for p in &func.params {
                    let key = p.name.to_ascii_uppercase();
                    if let Some((llvm_ty, _)) = local_types.get(&key) {
                        if p.is_by_val {
                            out.push_str(&format!(
                                "  store {} %\"arg_{}\", ptr %\"local_{}\"\n",
                                llvm_ty, p.name, p.name
                            ));
                        } else {
                            // ByRef is passed as pointer
                            out.push_str(&format!(
                                "  store ptr %\"arg_{}\", ptr %\"local_{}\"\n",
                                p.name, p.name
                            ));
                        }
                    }
                }
            }

            for inst in &block.instructions {
                self.emit_instruction(
                    inst,
                    &mut out,
                    &mut local_types,
                    &mut temp_types,
                    &func.return_type,
                );
            }

            // Ensure basic block ends with a terminator (br or ret)
            if !block_ends_with_terminator(block) {
                if let Some(ref ret_ty) = func.return_type {
                    let def_val = Self::llvm_default_val(ret_ty);
                    out.push_str(&format!(
                        "  ret {} {}\n",
                        Self::llvm_type(ret_ty),
                        def_val
                    ));
                } else {
                    out.push_str("  ret void\n");
                }
            }
        }

        out.push_str("}\n");
        out
    }

    fn emit_instruction(
        &self,
        inst: &IrInstruction,
        out: &mut String,
        local_types: &mut HashMap<String, (String, usize)>,
        temp_types: &mut HashMap<usize, String>,
        func_ret_type: &Option<VbType>,
    ) {
        match inst {
            IrInstruction::AllocLocal {
                name,
                vb_type,
                ..
            } => {
                let llvm_ty = Self::llvm_type(vb_type);
                let align = vb_type.alignment(true).max(1);
                out.push_str(&format!(
                    "  %\"local_{}\" = alloca {}, align {}\n",
                    name, llvm_ty, align
                ));
            }
            IrInstruction::Load { dest, src } => {
                let (src_val, src_ty) = self.format_operand(src, local_types, temp_types);
                temp_types.insert(*dest, src_ty.clone());
                // If src was a local variable, load from its alloca pointer
                if let Operand::Local(ref name) = src {
                    out.push_str(&format!(
                        "  %t{} = load {}, ptr %\"local_{}\"\n",
                        dest, src_ty, name
                    ));
                } else {
                    // Direct copy
                    out.push_str(&format!("  %t{} = add {} {}, 0\n", dest, src_ty, src_val));
                }
            }
            IrInstruction::Store { dest, src } => {
                let (src_val, src_ty) = self.format_operand(src, local_types, temp_types);
                if let Operand::Local(ref name) = dest {
                    out.push_str(&format!(
                        "  store {} {}, ptr %\"local_{}\"\n",
                        src_ty, src_val, name
                    ));
                }
            }
            IrInstruction::BinaryOp {
                dest,
                op,
                left,
                right,
                result_type,
            } => {
                let (left_val, left_ty) = self.format_operand(left, local_types, temp_types);
                let (right_val, _) = self.format_operand(right, local_types, temp_types);
                let res_llvm_ty = Self::llvm_type(result_type);
                temp_types.insert(*dest, res_llvm_ty.to_string());

                let is_float = result_type.is_floating();
                let op_str = match op {
                    IrBinaryOp::Add => if is_float { "fadd" } else { "add" },
                    IrBinaryOp::Sub => if is_float { "fsub" } else { "sub" },
                    IrBinaryOp::Mul => if is_float { "fmul" } else { "mul" },
                    IrBinaryOp::Div => "fdiv",
                    IrBinaryOp::IntDiv => "sdiv",
                    IrBinaryOp::Mod => "srem",
                    IrBinaryOp::BitAnd => "and",
                    IrBinaryOp::BitOr => "or",
                    IrBinaryOp::BitXor => "xor",
                    _ => "add",
                };

                out.push_str(&format!(
                    "  %t{} = {} {} {}, {}\n",
                    dest, op_str, left_ty, left_val, right_val
                ));
            }
            IrInstruction::UnaryOp {
                dest,
                op,
                operand,
                result_type,
            } => {
                let (op_val, op_ty) = self.format_operand(operand, local_types, temp_types);
                let res_llvm_ty = Self::llvm_type(result_type);
                temp_types.insert(*dest, res_llvm_ty.to_string());

                match op {
                    IrUnaryOp::Negate => {
                        if result_type.is_floating() {
                            out.push_str(&format!("  %t{} = fneg {} {}\n", dest, op_ty, op_val));
                        } else {
                            out.push_str(&format!("  %t{} = sub {} 0, {}\n", dest, op_ty, op_val));
                        }
                    }
                    IrUnaryOp::BitNot => {
                        out.push_str(&format!("  %t{} = xor {} {}, -1\n", dest, op_ty, op_val));
                    }
                }
            }
            IrInstruction::Compare {
                dest,
                op,
                left,
                right,
            } => {
                let (left_val, left_ty) = self.format_operand(left, local_types, temp_types);
                let (right_val, _) = self.format_operand(right, local_types, temp_types);
                temp_types.insert(*dest, "i16".to_string());

                let cmp_op = match op {
                    IrCompareOp::Equal => "eq",
                    IrCompareOp::NotEqual => "ne",
                    IrCompareOp::LessThan => "slt",
                    IrCompareOp::LessThanOrEqual => "sle",
                    IrCompareOp::GreaterThan => "sgt",
                    IrCompareOp::GreaterThanOrEqual => "sge",
                    _ => "eq",
                };

                // Compare generates i1, then we extend to VB6 Boolean (True = -1, False = 0)
                out.push_str(&format!(
                    "  %cmp{} = icmp {} {} {}, {}\n",
                    dest, cmp_op, left_ty, left_val, right_val
                ));
                out.push_str(&format!(
                    "  %t{} = select i1 %cmp{}, i16 -1, i16 0\n",
                    dest, dest
                ));
            }
            IrInstruction::Branch { target } => {
                out.push_str(&format!("  br label %{}\n", target));
            }
            IrInstruction::BranchIf {
                condition,
                true_target,
                false_target,
            } => {
                let (cond_val, cond_ty) = self.format_operand(condition, local_types, temp_types);
                out.push_str(&format!(
                    "  %cond_bool_{} = icmp ne {} {}, 0\n",
                    true_target, cond_ty, cond_val
                ));
                out.push_str(&format!(
                    "  br i1 %cond_bool_{}, label %{}, label %{}\n",
                    true_target, true_target, false_target
                ));
            }
            IrInstruction::Call { dest, name, args } => {
                let args_formatted: Vec<String> = args
                    .iter()
                    .map(|arg| {
                        let (val, ty) = self.format_operand(arg, local_types, temp_types);
                        format!("{} {}", ty, val)
                    })
                    .collect();

                if let Some(dest_id) = dest {
                    temp_types.insert(*dest_id, "i32".to_string());
                    out.push_str(&format!(
                        "  %t{} = call i32 @\"{}\"({})\n",
                        dest_id,
                        name,
                        args_formatted.join(", ")
                    ));
                } else {
                    out.push_str(&format!(
                        "  call void @\"{}\"({})\n",
                        name,
                        args_formatted.join(", ")
                    ));
                }
            }
            IrInstruction::Return { value } => {
                if let Some(val_op) = value {
                    let (v_str, v_ty) = self.format_operand(val_op, local_types, temp_types);
                    out.push_str(&format!("  ret {} {}\n", v_ty, v_str));
                } else if let Some(ref ret_ty) = func_ret_type {
                    let def_val = Self::llvm_default_val(ret_ty);
                    out.push_str(&format!(
                        "  ret {} {}\n",
                        Self::llvm_type(ret_ty),
                        def_val
                    ));
                } else {
                    out.push_str("  ret void\n");
                }
            }
            IrInstruction::Coerce {
                dest,
                src,
                from_type,
                to_type,
            } => {
                let (src_val, _) = self.format_operand(src, local_types, temp_types);
                let to_llvm_ty = Self::llvm_type(to_type);
                temp_types.insert(*dest, to_llvm_ty.to_string());

                let from_size = from_type.size_in_bytes(true);
                let to_size = to_type.size_in_bytes(true);

                if from_type.is_integer() && to_type.is_integer() {
                    if from_size < to_size {
                        out.push_str(&format!(
                            "  %t{} = sext {} {} to {}\n",
                            dest,
                            Self::llvm_type(from_type),
                            src_val,
                            to_llvm_ty
                        ));
                    } else if from_size > to_size {
                        out.push_str(&format!(
                            "  %t{} = trunc {} {} to {}\n",
                            dest,
                            Self::llvm_type(from_type),
                            src_val,
                            to_llvm_ty
                        ));
                    } else {
                        out.push_str(&format!(
                            "  %t{} = bitcast {} {} to {}\n",
                            dest,
                            Self::llvm_type(from_type),
                            src_val,
                            to_llvm_ty
                        ));
                    }
                } else if from_type.is_integer() && to_type.is_floating() {
                    out.push_str(&format!(
                        "  %t{} = sitofp {} {} to {}\n",
                        dest,
                        Self::llvm_type(from_type),
                        src_val,
                        to_llvm_ty
                    ));
                } else if from_type.is_floating() && to_type.is_integer() {
                    out.push_str(&format!(
                        "  %t{} = fptosi {} {} to {}\n",
                        dest,
                        Self::llvm_type(from_type),
                        src_val,
                        to_llvm_ty
                    ));
                } else {
                    out.push_str(&format!(
                        "  %t{} = bitcast {} {} to {}\n",
                        dest,
                        Self::llvm_type(from_type),
                        src_val,
                        to_llvm_ty
                    ));
                }
            }
            IrInstruction::DebugPrint { args } => {
                for arg in args {
                    let (arg_val, arg_ty) = self.format_operand(arg, local_types, temp_types);
                    out.push_str(&format!(
                        "  call void @vb6_debug_print({} {})\n",
                        arg_ty, arg_val
                    ));
                }
            }
            _ => {}
        }
    }

    fn format_operand(
        &self,
        op: &Operand,
        local_types: &HashMap<String, (String, usize)>,
        temp_types: &HashMap<usize, String>,
    ) -> (String, String) {
        match op {
            Operand::Temp(id) => {
                let ty = temp_types
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| "i32".to_string());
                (format!("%t{}", id), ty)
            }
            Operand::Local(name) => {
                let ty = local_types
                    .get(&name.to_ascii_uppercase())
                    .map(|(t, _)| t.clone())
                    .unwrap_or_else(|| "i32".to_string());
                (format!("%\"local_{}\"", name), ty)
            }
            Operand::Constant(c) => match c {
                IrConstant::Int(v, ty) => (v.to_string(), Self::llvm_type(ty).to_string()),
                IrConstant::Float(v) => (format!("{:.6}", v), "double".to_string()),
                IrConstant::String(_) => ("null".to_string(), "ptr".to_string()),
                IrConstant::Bool(b) => {
                    let val = if *b { "-1" } else { "0" };
                    (val.to_string(), "i16".to_string())
                }
                IrConstant::Date(_) => ("0.0".to_string(), "double".to_string()),
                IrConstant::Null | IrConstant::Empty | IrConstant::Nothing => {
                    ("null".to_string(), "ptr".to_string())
                }
            },
        }
    }
}

fn block_ends_with_terminator(block: &BasicBlock) -> bool {
    if let Some(last) = block.instructions.last() {
        matches!(
            last,
            IrInstruction::Branch { .. }
                | IrInstruction::BranchIf { .. }
                | IrInstruction::Return { .. }
                | IrInstruction::Resume { .. }
        )
    } else {
        false
    }
}
