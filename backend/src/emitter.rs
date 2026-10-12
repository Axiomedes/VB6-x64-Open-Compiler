//! LLVM IR text emitter from VB6 IR.
//! Emits target-specific LLVM IR for Windows x64 (AMD64 / PE32+).

use std::collections::{HashMap, HashSet};
use vb6_compiler::ir::model::*;
use vb6_compiler::types::{UdtLayout, VbType};

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

    /// Return expected return type and parameter types for runtime functions.
    pub fn get_runtime_func_sig(name: &str) -> Option<(&'static str, &'static [&'static str])> {
        match name {
            "vb6_init" => Some(("void", &[])),
            "vb6_shutdown" => Some(("void", &[])),
            "vb6_msgbox" => Some(("i32", &["ptr", "ptr", "i32"])),
            "vb6_debug_print" => Some(("void", &["ptr"])),
            "vb6_debug_print_i8" => Some(("void", &["i8"])),
            "vb6_debug_print_i16" => Some(("void", &["i16"])),
            "vb6_debug_print_i32" => Some(("void", &["i32"])),
            "vb6_debug_print_i64" => Some(("void", &["i64"])),
            "vb6_debug_print_f32" => Some(("void", &["float"])),
            "vb6_debug_print_f64" => Some(("void", &["double"])),
            "vb6_array_create" => Some(("ptr", &["i32", "ptr", "i32"])),
            "vb6_array_element_ptr" => Some(("ptr", &["ptr", "i32", "ptr"])),
            "vb6_array_lbound" => Some(("i32", &["ptr", "i32"])),
            "vb6_array_ubound" => Some(("i32", &["ptr", "i32"])),
            "vb6_array_redim" => Some(("ptr", &["ptr", "i32", "ptr", "i32", "i32"])),
            "vb6_array_free" => Some(("void", &["ptr"])),
            "vb6_string_alloc" => Some(("ptr", &["ptr"])),
            "vb6_string_free" => Some(("void", &["ptr"])),
            "vb6_string_concat" => Some(("ptr", &["ptr", "ptr"])),
            "vb6_string_len" => Some(("i32", &["ptr"])),
            "vb6_string_left" => Some(("ptr", &["ptr", "i32"])),
            "vb6_string_right" => Some(("ptr", &["ptr", "i32"])),
            "vb6_string_mid" => Some(("ptr", &["ptr", "i32", "i32"])),
            "vb6_string_instr" => Some(("i32", &["i32", "ptr", "ptr"])),
            "vb6_string_ucase" => Some(("ptr", &["ptr"])),
            "vb6_string_lcase" => Some(("ptr", &["ptr"])),
            "vb6_string_trim" => Some(("ptr", &["ptr"])),
            "vb6_string_ltrim" => Some(("ptr", &["ptr"])),
            "vb6_string_rtrim" => Some(("ptr", &["ptr"])),
            "vb6_string_space" => Some(("ptr", &["i32"])),
            "vb6_string_string" => Some(("ptr", &["i32", "ptr"])),
            "vb6_string_replace" => Some(("ptr", &["ptr", "ptr", "ptr"])),
            "vb6_string_chr" => Some(("ptr", &["i32"])),
            "vb6_string_asc" => Some(("i32", &["ptr"])),
            "vb6_string_val" => Some(("double", &["ptr"])),
            "vb6_string_str" => Some(("ptr", &["double"])),
            "vb6_str_from_i64" => Some(("ptr", &["i64"])),
            "vb6_str_from_f64" => Some(("ptr", &["double"])),
            "vb6_str_from_bool" => Some(("ptr", &["i32"])),
            "vb6_string_equal" => Some(("i16", &["ptr", "ptr"])),
            "vb6_string_compare" => Some(("i32", &["ptr", "ptr"])),
            "vb6_variant_init" => Some(("void", &["ptr"])),
            "vb6_variant_clear" => Some(("void", &["ptr"])),
            "vb6_variant_copy" => Some(("void", &["ptr", "ptr"])),
            "vb6_variant_from_i8" => Some(("void", &["ptr", "i8"])),
            "vb6_variant_from_i16" => Some(("void", &["ptr", "i16"])),
            "vb6_variant_from_i32" => Some(("void", &["ptr", "i32"])),
            "vb6_variant_from_i64" => Some(("void", &["ptr", "i64"])),
            "vb6_variant_from_f32" => Some(("void", &["ptr", "float"])),
            "vb6_variant_from_f64" => Some(("void", &["ptr", "double"])),
            "vb6_variant_from_bool" => Some(("void", &["ptr", "i16"])),
            "vb6_variant_from_str" => Some(("void", &["ptr", "ptr"])),
            "vb6_variant_from_date" => Some(("void", &["ptr", "double"])),
            "vb6_variant_from_empty" => Some(("void", &["ptr"])),
            "vb6_variant_from_null" => Some(("void", &["ptr"])),
            "vb6_variant_to_i8" => Some(("i8", &["ptr"])),
            "vb6_variant_to_i16" => Some(("i16", &["ptr"])),
            "vb6_variant_to_i32" => Some(("i32", &["ptr"])),
            "vb6_variant_to_i64" => Some(("i64", &["ptr"])),
            "vb6_variant_to_f32" => Some(("float", &["ptr"])),
            "vb6_variant_to_f64" => Some(("double", &["ptr"])),
            "vb6_variant_to_bool" => Some(("i16", &["ptr"])),
            "vb6_variant_to_str" => Some(("ptr", &["ptr"])),
            "vb6_variant_vartype" => Some(("i32", &["ptr"])),
            "vb6_variant_typename" => Some(("ptr", &["ptr"])),
            "vb6_variant_is_empty" => Some(("i16", &["ptr"])),
            "vb6_variant_is_null" => Some(("i16", &["ptr"])),
            "vb6_variant_is_numeric" => Some(("i16", &["ptr"])),
            "vb6_variant_is_date" => Some(("i16", &["ptr"])),
            "vb6_variant_is_array" => Some(("i16", &["ptr"])),
            "vb6_variant_add" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_sub" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_mul" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_div" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_int_div" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_mod" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_concat" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_neg" => Some(("void", &["ptr", "ptr"])),
            "vb6_variant_not" => Some(("void", &["ptr", "ptr"])),
            "vb6_variant_and" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_or" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_xor" => Some(("void", &["ptr", "ptr", "ptr"])),
            "vb6_variant_compare" => Some(("i32", &["ptr", "ptr"])),
            "vb6_variant_equal" => Some(("i16", &["ptr", "ptr"])),
            "vb6_debug_print_variant" => Some(("void", &["ptr"])),
            "vb6_memory_copy" => Some(("void", &["ptr", "ptr", "i64"])),
            "vb6_memory_zero" => Some(("void", &["ptr", "i64"])),
            "vb6_fixed_str_set" => Some(("void", &["ptr", "i64", "ptr"])),
            "vb6_fixed_str_get" => Some(("ptr", &["ptr", "i64"])),
            _ => None,
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
        out.push_str("declare void @vb6_debug_print(ptr)\n");
        out.push_str("declare void @vb6_debug_print_i8(i8)\n");
        out.push_str("declare void @vb6_debug_print_i16(i16)\n");
        out.push_str("declare void @vb6_debug_print_i32(i32)\n");
        out.push_str("declare void @vb6_debug_print_i64(i64)\n");
        out.push_str("declare void @vb6_debug_print_f32(float)\n");
        out.push_str("declare void @vb6_debug_print_f64(double)\n");
        out.push_str("declare ptr @vb6_array_create(i32, ptr, i32)\n");
        out.push_str("declare ptr @vb6_array_element_ptr(ptr, i32, ptr)\n");
        out.push_str("declare i32 @vb6_array_lbound(ptr, i32)\n");
        out.push_str("declare i32 @vb6_array_ubound(ptr, i32)\n");
        out.push_str("declare ptr @vb6_array_redim(ptr, i32, ptr, i32, i32)\n");
        out.push_str("declare void @vb6_array_free(ptr)\n");
        out.push_str("declare ptr @vb6_string_alloc(ptr)\n");
        out.push_str("declare void @vb6_string_free(ptr)\n");
        out.push_str("declare ptr @vb6_string_concat(ptr, ptr)\n");
        out.push_str("declare i32 @vb6_string_len(ptr)\n");
        out.push_str("declare ptr @vb6_string_left(ptr, i32)\n");
        out.push_str("declare ptr @vb6_string_right(ptr, i32)\n");
        out.push_str("declare ptr @vb6_string_mid(ptr, i32, i32)\n");
        out.push_str("declare i32 @vb6_string_instr(i32, ptr, ptr)\n");
        out.push_str("declare ptr @vb6_string_ucase(ptr)\n");
        out.push_str("declare ptr @vb6_string_lcase(ptr)\n");
        out.push_str("declare ptr @vb6_string_trim(ptr)\n");
        out.push_str("declare ptr @vb6_string_ltrim(ptr)\n");
        out.push_str("declare ptr @vb6_string_rtrim(ptr)\n");
        out.push_str("declare ptr @vb6_string_space(i32)\n");
        out.push_str("declare ptr @vb6_string_string(i32, ptr)\n");
        out.push_str("declare ptr @vb6_string_replace(ptr, ptr, ptr)\n");
        out.push_str("declare ptr @vb6_string_chr(i32)\n");
        out.push_str("declare i32 @vb6_string_asc(ptr)\n");
        out.push_str("declare double @vb6_string_val(ptr)\n");
        out.push_str("declare ptr @vb6_string_str(double)\n");
        out.push_str("declare ptr @vb6_str_from_i64(i64)\n");
        out.push_str("declare ptr @vb6_str_from_f64(double)\n");
        out.push_str("declare ptr @vb6_str_from_bool(i32)\n");
        out.push_str("declare i16 @vb6_string_equal(ptr, ptr)\n");
        out.push_str("declare i32 @vb6_string_compare(ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_init(ptr)\n");
        out.push_str("declare void @vb6_variant_clear(ptr)\n");
        out.push_str("declare void @vb6_variant_copy(ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_from_i8(ptr, i8)\n");
        out.push_str("declare void @vb6_variant_from_i16(ptr, i16)\n");
        out.push_str("declare void @vb6_variant_from_i32(ptr, i32)\n");
        out.push_str("declare void @vb6_variant_from_i64(ptr, i64)\n");
        out.push_str("declare void @vb6_variant_from_f32(ptr, float)\n");
        out.push_str("declare void @vb6_variant_from_f64(ptr, double)\n");
        out.push_str("declare void @vb6_variant_from_bool(ptr, i16)\n");
        out.push_str("declare void @vb6_variant_from_str(ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_from_date(ptr, double)\n");
        out.push_str("declare void @vb6_variant_from_empty(ptr)\n");
        out.push_str("declare void @vb6_variant_from_null(ptr)\n");
        out.push_str("declare i8 @vb6_variant_to_i8(ptr)\n");
        out.push_str("declare i16 @vb6_variant_to_i16(ptr)\n");
        out.push_str("declare i32 @vb6_variant_to_i32(ptr)\n");
        out.push_str("declare i64 @vb6_variant_to_i64(ptr)\n");
        out.push_str("declare float @vb6_variant_to_f32(ptr)\n");
        out.push_str("declare double @vb6_variant_to_f64(ptr)\n");
        out.push_str("declare i16 @vb6_variant_to_bool(ptr)\n");
        out.push_str("declare ptr @vb6_variant_to_str(ptr)\n");
        out.push_str("declare i32 @vb6_variant_vartype(ptr)\n");
        out.push_str("declare ptr @vb6_variant_typename(ptr)\n");
        out.push_str("declare i16 @vb6_variant_is_empty(ptr)\n");
        out.push_str("declare i16 @vb6_variant_is_null(ptr)\n");
        out.push_str("declare i16 @vb6_variant_is_numeric(ptr)\n");
        out.push_str("declare i16 @vb6_variant_is_date(ptr)\n");
        out.push_str("declare i16 @vb6_variant_is_array(ptr)\n");
        out.push_str("declare void @vb6_variant_add(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_sub(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_mul(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_div(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_int_div(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_mod(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_concat(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_neg(ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_not(ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_and(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_or(ptr, ptr, ptr)\n");
        out.push_str("declare void @vb6_variant_xor(ptr, ptr, ptr)\n");
        out.push_str("declare i32 @vb6_variant_compare(ptr, ptr)\n");
        out.push_str("declare i16 @vb6_variant_equal(ptr, ptr)\n");
        out.push_str("declare void @vb6_debug_print_variant(ptr)\n");
        out.push_str("declare void @vb6_memory_copy(ptr, ptr, i64)\n");
        out.push_str("declare void @vb6_memory_zero(ptr, i64)\n");
        out.push_str("declare void @vb6_fixed_str_set(ptr, i64, ptr)\n");
        out.push_str("declare ptr @vb6_fixed_str_get(ptr, i64)\n\n");

        // Module-level global and static variables
        let mut global_types: HashMap<String, String> = HashMap::new();
        for glob in &module.globals {
            let (llvm_ty, def_val, align) = if let VbType::UserDefined(ref name) = glob.vb_type {
                if let Some(layout) = module.udt_layouts.get(&name.to_ascii_uppercase()) {
                    (format!("[{} x i8]", layout.total_size), "zeroinitializer".to_string(), layout.alignment.max(1))
                } else {
                    ("[8 x i8]".to_string(), "zeroinitializer".to_string(), 8)
                }
            } else {
                (Self::llvm_type(&glob.vb_type).to_string(), Self::llvm_default_val(&glob.vb_type).to_string(), glob.vb_type.alignment(true).max(1))
            };
            let linkage = if glob.is_public { "global" } else { "internal global" };
            out.push_str(&format!(
                "@\"{}\" = {} {} {}, align {}\n",
                glob.name, linkage, llvm_ty, def_val, align
            ));
            global_types.insert(glob.name.to_ascii_uppercase(), llvm_ty);
        }
        if !module.globals.is_empty() {
            out.push('\n');
        }

        // Collect and emit string constants
        let mut string_constants: HashMap<String, String> = HashMap::new();
        let mut str_counter = 0usize;

        for func in &module.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    self.collect_strings_from_instruction(inst, &mut string_constants, &mut str_counter);
                }
            }
        }

        for (str_val, global_name) in &string_constants {
            let (escaped, len) = escape_llvm_string(str_val);
            out.push_str(&format!(
                "{} = private unnamed_addr constant [{} x i8] {}, align 1\n",
                global_name, len, escaped
            ));
        }
        if !string_constants.is_empty() {
            out.push('\n');
        }

        let mut has_main_sub = false;

        // Functions
        for func in &module.functions {
            if func.name.eq_ignore_ascii_case("Main") {
                has_main_sub = true;
            }
            out.push_str(&self.emit_function(func, &string_constants, &global_types, &module.udt_layouts));
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

    fn collect_strings_from_instruction(
        &self,
        inst: &IrInstruction,
        string_constants: &mut HashMap<String, String>,
        str_counter: &mut usize,
    ) {
        let mut check_op = |op: &Operand| {
            if let Operand::Constant(IrConstant::String(ref s)) = op {
                if !string_constants.contains_key(s) {
                    let name = format!("@.str.{}", str_counter);
                    *str_counter += 1;
                    string_constants.insert(s.clone(), name);
                }
            }
        };

        match inst {
            IrInstruction::Store { src, .. } => check_op(src),
            IrInstruction::Call { args, .. } => {
                for arg in args {
                    check_op(arg);
                }
            }
            IrInstruction::BinaryOp { left, right, .. } => {
                check_op(left);
                check_op(right);
            }
            IrInstruction::Compare { left, right, .. } => {
                check_op(left);
                check_op(right);
            }
            IrInstruction::Return { value: Some(op) } => check_op(op),
            IrInstruction::DebugPrint { args } => {
                for arg in args {
                    check_op(arg);
                }
            }
            IrInstruction::StringConcat { left, right, .. } => {
                check_op(left);
                check_op(right);
            }
            IrInstruction::Coerce { src, .. } => check_op(src),
            IrInstruction::SetMember { value, .. } => check_op(value),
            IrInstruction::ArraySetElement { value, indices, .. } => {
                check_op(value);
                for idx in indices {
                    check_op(idx);
                }
            }
            _ => {}
        }
    }

    fn emit_function(
        &self,
        func: &IrFunction,
        string_constants: &HashMap<String, String>,
        global_types: &HashMap<String, String>,
        udt_layouts: &HashMap<String, UdtLayout>,
    ) -> String {
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

        let byref_params: HashSet<String> = func
            .params
            .iter()
            .filter(|p| !p.is_by_val)
            .map(|p| p.name.to_ascii_uppercase())
            .collect();

        // Track local alloca types: name -> (llvm_type, align)
        let mut local_types: HashMap<String, (String, usize)> = HashMap::new();
        // Track temp types: temp_id -> llvm_type
        let mut temp_types: HashMap<usize, String> = HashMap::new();

        // First, inspect instructions in all blocks to pre-collect local alloca types
        for block in &func.blocks {
            for inst in &block.instructions {
                if let IrInstruction::AllocLocal { name, vb_type, .. } = inst {
                    let key = name.to_ascii_uppercase();
                    if byref_params.contains(&key) {
                        local_types.insert(key, ("ptr".to_string(), 8));
                    } else if let VbType::UserDefined(ref udt_name) = vb_type {
                        if let Some(layout) = udt_layouts.get(&udt_name.to_ascii_uppercase()) {
                            local_types.insert(
                                key,
                                (format!("[{} x i8]", layout.total_size), layout.alignment.max(1)),
                            );
                        } else {
                            local_types.insert(key, ("[8 x i8]".to_string(), 8));
                        }
                    } else {
                        local_types.insert(
                            key,
                            (
                                Self::llvm_type(vb_type).to_string(),
                                vb_type.alignment(true).max(1),
                            ),
                        );
                    }
                }
            }
        }

        let mut branch_counter = 0usize;

        // Emit blocks
        for (block_idx, block) in func.blocks.iter().enumerate() {
            out.push_str(&format!("{}:\n", block.label));

            // In entry block, emit all allocas first so they dominate the whole function
            if block_idx == 0 {
                let mut emitted_allocas = std::collections::HashSet::new();
                for b in &func.blocks {
                    for inst in &b.instructions {
                        if let IrInstruction::AllocLocal { name, vb_type, .. } = inst {
                            let key = name.to_ascii_uppercase();
                            if emitted_allocas.insert(key.clone()) {
                                if byref_params.contains(&key) {
                                    out.push_str(&format!(
                                        "  %\"local_{}\" = alloca ptr, align 8\n",
                                        name
                                    ));
                                    out.push_str(&format!(
                                        "  store ptr null, ptr %\"local_{}\"\n",
                                        name
                                    ));
                                } else if let VbType::UserDefined(ref udt_name) = vb_type {
                                    let (total_sz, align) = udt_layouts
                                        .get(&udt_name.to_ascii_uppercase())
                                        .map(|l| (l.total_size, l.alignment.max(1)))
                                        .unwrap_or((8, 8));
                                    out.push_str(&format!(
                                        "  %\"local_{}\" = alloca [{} x i8], align {}\n",
                                        name, total_sz, align
                                    ));
                                    out.push_str(&format!(
                                        "  store [{} x i8] zeroinitializer, ptr %\"local_{}\"\n",
                                        total_sz, name
                                    ));
                                } else {
                                    let llvm_ty = Self::llvm_type(vb_type);
                                    let align = vb_type.alignment(true).max(1);
                                    out.push_str(&format!(
                                        "  %\"local_{}\" = alloca {}, align {}\n",
                                        name, llvm_ty, align
                                    ));
                                    let def_val = Self::llvm_default_val(vb_type);
                                    out.push_str(&format!(
                                        "  store {} {}, ptr %\"local_{}\"\n",
                                        llvm_ty, def_val, name
                                    ));
                                }
                            }
                        }
                    }
                }

                for p in &func.params {
                    if p.is_by_val {
                        let llvm_ty = Self::llvm_type(&p.vb_type);
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

            for inst in &block.instructions {
                self.emit_instruction(
                    inst,
                    &mut out,
                    &mut local_types,
                    global_types,
                    &mut temp_types,
                    string_constants,
                    &func.return_type,
                    &byref_params,
                    udt_layouts,
                    &mut branch_counter,
                );
                if matches!(
                    inst,
                    IrInstruction::Branch { .. }
                        | IrInstruction::BranchIf { .. }
                        | IrInstruction::Return { .. }
                        | IrInstruction::Resume { .. }
                ) {
                    break;
                }
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

    fn format_base_ptr(
        &self,
        base: &Operand,
        byref_params: &HashSet<String>,
        out: &mut String,
        branch_counter: &mut usize,
    ) -> String {
        match base {
            Operand::Local(name) => {
                if byref_params.contains(&name.to_ascii_uppercase()) {
                    *branch_counter += 1;
                    let ptr_temp = format!("%base_ptr_{}", branch_counter);
                    out.push_str(&format!("  {} = load ptr, ptr %\"local_{}\"\n", ptr_temp, name));
                    ptr_temp
                } else {
                    format!("%\"local_{}\"", name)
                }
            }
            Operand::Global(name) => {
                format!("@\"{}\"", name)
            }
            Operand::Temp(id) => {
                format!("%t{}", id)
            }
            _ => "null".to_string(),
        }
    }

    fn emit_instruction(
        &self,
        inst: &IrInstruction,
        out: &mut String,
        local_types: &mut HashMap<String, (String, usize)>,
        global_types: &HashMap<String, String>,
        temp_types: &mut HashMap<usize, String>,
        string_constants: &HashMap<String, String>,
        func_ret_type: &Option<VbType>,
        byref_params: &HashSet<String>,
        udt_layouts: &HashMap<String, UdtLayout>,
        branch_counter: &mut usize,
    ) {
        match inst {
            IrInstruction::AllocLocal { .. } => {
                // Pre-allocated in the function entry block.
            }
            IrInstruction::Load { dest, src } => {
                let (src_val, src_ty) = self.format_operand(src, local_types, global_types, temp_types, string_constants);
                temp_types.insert(*dest, src_ty.clone());
                match src {
                    Operand::Local(ref name) => {
                        out.push_str(&format!(
                            "  %t{} = load {}, ptr %\"local_{}\"\n",
                            dest, src_ty, name
                        ));
                    }
                    Operand::Global(ref name) => {
                        out.push_str(&format!(
                            "  %t{} = load {}, ptr @\"{}\"\n",
                            dest, src_ty, name
                        ));
                    }
                    _ => {
                        if src_ty == "[24 x i8]" {
                            *branch_counter += 1;
                            let c_id = *branch_counter;
                            out.push_str(&format!("  %load_tmp_{} = alloca [24 x i8], align 8\n", c_id));
                            out.push_str(&format!("  store [24 x i8] {}, ptr %load_tmp_{}\n", src_val, c_id));
                            out.push_str(&format!("  %t{} = load [24 x i8], ptr %load_tmp_{}\n", dest, c_id));
                        } else {
                            out.push_str(&format!("  %t{} = add {} {}, 0\n", dest, src_ty, src_val));
                        }
                    }
                }
            }
            IrInstruction::Store { dest, src } => {
                let dest_udt = match dest {
                    Operand::Local(ref name) => local_types.get(&name.to_ascii_uppercase()).and_then(|(t, _)| {
                        if t.starts_with('[') && t.ends_with(" x i8]") && t != "[24 x i8]" {
                            Some(t.clone())
                        } else {
                            None
                        }
                    }),
                    Operand::Global(ref name) => global_types.get(&name.to_ascii_uppercase()).and_then(|t| {
                        if t.starts_with('[') && t.ends_with(" x i8]") && t != "[24 x i8]" {
                            Some(t.clone())
                        } else {
                            None
                        }
                    }),
                    _ => None,
                };
                if let Some(ref arr_ty) = dest_udt {
                    let sz: usize = arr_ty.trim_start_matches('[').trim_end_matches(" x i8]").parse().unwrap_or(8);
                    let dest_ptr = self.format_base_ptr(dest, byref_params, out, branch_counter);
                    let src_ptr = self.format_base_ptr(src, byref_params, out, branch_counter);
                    out.push_str(&format!(
                        "  call void @vb6_memory_copy(ptr {}, ptr {}, i64 {})\n",
                        dest_ptr, src_ptr, sz
                    ));
                    return;
                }

                let dest_is_variant = match dest {
                    Operand::Local(ref name) => local_types.get(&name.to_ascii_uppercase()).map(|(t, _)| t == "[24 x i8]").unwrap_or(false),
                    Operand::Global(ref name) => global_types.get(&name.to_ascii_uppercase()).map(|t| t == "[24 x i8]").unwrap_or(false),
                    _ => false,
                };
                if dest_is_variant {
                    let dest_ptr = match dest {
                        Operand::Local(ref name) => format!("%\"local_{}\"", name),
                        Operand::Global(ref name) => format!("@\"{}\"", name),
                        _ => "".to_string(),
                    };
                    if matches!(src, Operand::Constant(IrConstant::Null)) {
                        out.push_str(&format!("  call void @vb6_variant_from_null(ptr {})\n", dest_ptr));
                    } else if matches!(src, Operand::Constant(IrConstant::Empty)) {
                        out.push_str(&format!("  call void @vb6_variant_from_empty(ptr {})\n", dest_ptr));
                    } else {
                        let (src_val, src_ty) = self.format_operand(src, local_types, global_types, temp_types, string_constants);
                        if src_ty == "[24 x i8]" {
                            *branch_counter += 1;
                            let c_id = *branch_counter;
                            out.push_str(&format!("  %var_src_slot_{} = alloca [24 x i8], align 8\n", c_id));
                            out.push_str(&format!("  store [24 x i8] {}, ptr %var_src_slot_{}\n", src_val, c_id));
                            out.push_str(&format!("  call void @vb6_variant_copy(ptr {}, ptr %var_src_slot_{})\n", dest_ptr, c_id));
                        } else {
                            out.push_str(&format!("  store {} {}, ptr {}\n", src_ty, src_val, dest_ptr));
                        }
                    }
                } else {
                    let (src_val, src_ty) = self.format_operand(src, local_types, global_types, temp_types, string_constants);
                    match dest {
                        Operand::Local(ref name) => {
                            out.push_str(&format!(
                                "  store {} {}, ptr %\"local_{}\"\n",
                                src_ty, src_val, name
                            ));
                        }
                        Operand::Global(ref name) => {
                            out.push_str(&format!(
                                "  store {} {}, ptr @\"{}\"\n",
                                src_ty, src_val, name
                            ));
                        }
                        _ => {}
                    }
                }
            }
            IrInstruction::BinaryOp {
                dest,
                op,
                left,
                right,
                result_type,
            } => {
                let (left_val, left_ty) = self.format_operand(left, local_types, global_types, temp_types, string_constants);
                let (right_val, right_ty) = self.format_operand(right, local_types, global_types, temp_types, string_constants);
                let res_llvm_ty = Self::llvm_type(result_type);
                temp_types.insert(*dest, res_llvm_ty.to_string());

                if result_type == &VbType::Variant {
                    *branch_counter += 1;
                    let c_id = *branch_counter;
                    let (l_ptr, _) = cast_arg(&left_val, &left_ty, "ptr", out, branch_counter);
                    let (r_ptr, _) = cast_arg(&right_val, &right_ty, "ptr", out, branch_counter);
                    out.push_str(&format!("  %var_res_slot_{} = alloca [24 x i8], align 8\n", c_id));
                    out.push_str(&format!("  call void @vb6_variant_init(ptr %var_res_slot_{})\n", c_id));
                    let fn_name = match op {
                        IrBinaryOp::Add => "vb6_variant_add",
                        IrBinaryOp::Sub => "vb6_variant_sub",
                        IrBinaryOp::Mul => "vb6_variant_mul",
                        IrBinaryOp::Div => "vb6_variant_div",
                        IrBinaryOp::IntDiv => "vb6_variant_int_div",
                        IrBinaryOp::Mod => "vb6_variant_mod",
                        IrBinaryOp::Concat => "vb6_variant_concat",
                        IrBinaryOp::BitAnd => "vb6_variant_and",
                        IrBinaryOp::BitOr => "vb6_variant_or",
                        IrBinaryOp::BitXor => "vb6_variant_xor",
                        _ => "vb6_variant_add",
                    };
                    out.push_str(&format!("  call void @{}(ptr %var_res_slot_{}, ptr {}, ptr {})\n", fn_name, c_id, l_ptr, r_ptr));
                    out.push_str(&format!("  %t{} = load [24 x i8], ptr %var_res_slot_{}\n", dest, c_id));
                } else {
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
            }
            IrInstruction::UnaryOp {
                dest,
                op,
                operand,
                result_type,
            } => {
                let (op_val, op_ty) = self.format_operand(operand, local_types, global_types, temp_types, string_constants);
                let res_llvm_ty = Self::llvm_type(result_type);
                temp_types.insert(*dest, res_llvm_ty.to_string());

                if result_type == &VbType::Variant {
                    *branch_counter += 1;
                    let c_id = *branch_counter;
                    let (op_ptr, _) = cast_arg(&op_val, &op_ty, "ptr", out, branch_counter);
                    out.push_str(&format!("  %var_unary_res_{} = alloca [24 x i8], align 8\n", c_id));
                    out.push_str(&format!("  call void @vb6_variant_init(ptr %var_unary_res_{})\n", c_id));
                    let fn_name = match op {
                        IrUnaryOp::Negate => "vb6_variant_neg",
                        IrUnaryOp::BitNot => "vb6_variant_not",
                    };
                    out.push_str(&format!("  call void @{}(ptr %var_unary_res_{}, ptr {})\n", fn_name, c_id, op_ptr));
                    out.push_str(&format!("  %t{} = load [24 x i8], ptr %var_unary_res_{}\n", dest, c_id));
                } else {
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
            }
            IrInstruction::Compare {
                dest,
                op,
                left,
                right,
            } => {
                let (left_val, left_ty) = self.format_operand(left, local_types, global_types, temp_types, string_constants);
                let (right_val, right_ty) = self.format_operand(right, local_types, global_types, temp_types, string_constants);
                temp_types.insert(*dest, "i16".to_string());

                if left_ty == "[24 x i8]" || right_ty == "[24 x i8]" {
                    *branch_counter += 1;
                    let c_id = *branch_counter;
                    let (l_ptr, _) = cast_arg(&left_val, &left_ty, "ptr", out, branch_counter);
                    let (r_ptr, _) = cast_arg(&right_val, &right_ty, "ptr", out, branch_counter);
                    out.push_str(&format!(
                        "  %var_cmp_{} = call i32 @vb6_variant_compare(ptr {}, ptr {})\n",
                        c_id, l_ptr, r_ptr
                    ));
                    let cmp_op = match op {
                        IrCompareOp::Equal => "eq",
                        IrCompareOp::NotEqual => "ne",
                        IrCompareOp::LessThan => "slt",
                        IrCompareOp::LessThanOrEqual => "sle",
                        IrCompareOp::GreaterThan => "sgt",
                        IrCompareOp::GreaterThanOrEqual => "sge",
                        _ => "eq",
                    };
                    out.push_str(&format!(
                        "  %cmp{} = icmp {} i32 %var_cmp_{}, 0\n",
                        dest, cmp_op, c_id
                    ));
                    out.push_str(&format!(
                        "  %t{} = select i1 %cmp{}, i16 -1, i16 0\n",
                        dest, dest
                    ));
                } else if left_ty == "ptr" || right_ty == "ptr" {
                    *branch_counter += 1;
                    let cmp_call_id = *branch_counter;
                    out.push_str(&format!(
                        "  %str_cmp_{} = call i32 @vb6_string_compare(ptr {}, ptr {})\n",
                        cmp_call_id, left_val, right_val
                    ));
                    let cmp_op = match op {
                        IrCompareOp::Equal => "eq",
                        IrCompareOp::NotEqual => "ne",
                        IrCompareOp::LessThan => "slt",
                        IrCompareOp::LessThanOrEqual => "sle",
                        IrCompareOp::GreaterThan => "sgt",
                        IrCompareOp::GreaterThanOrEqual => "sge",
                        _ => "eq",
                    };
                    out.push_str(&format!(
                        "  %cmp{} = icmp {} i32 %str_cmp_{}, 0\n",
                        dest, cmp_op, cmp_call_id
                    ));
                    out.push_str(&format!(
                        "  %t{} = select i1 %cmp{}, i16 -1, i16 0\n",
                        dest, dest
                    ));
                } else if left_ty == "float" || left_ty == "double" {
                    let fcmp_op = match op {
                        IrCompareOp::Equal => "oeq",
                        IrCompareOp::NotEqual => "one",
                        IrCompareOp::LessThan => "olt",
                        IrCompareOp::LessThanOrEqual => "ole",
                        IrCompareOp::GreaterThan => "ogt",
                        IrCompareOp::GreaterThanOrEqual => "oge",
                        _ => "oeq",
                    };
                    out.push_str(&format!(
                        "  %cmp{} = fcmp {} {} {}, {}\n",
                        dest, fcmp_op, left_ty, left_val, right_val
                    ));
                    out.push_str(&format!(
                        "  %t{} = select i1 %cmp{}, i16 -1, i16 0\n",
                        dest, dest
                    ));
                } else {
                    let cmp_op = match op {
                        IrCompareOp::Equal => "eq",
                        IrCompareOp::NotEqual => "ne",
                        IrCompareOp::LessThan => "slt",
                        IrCompareOp::LessThanOrEqual => "sle",
                        IrCompareOp::GreaterThan => "sgt",
                        IrCompareOp::GreaterThanOrEqual => "sge",
                        _ => "eq",
                    };
                    out.push_str(&format!(
                        "  %cmp{} = icmp {} {} {}, {}\n",
                        dest, cmp_op, left_ty, left_val, right_val
                    ));
                    out.push_str(&format!(
                        "  %t{} = select i1 %cmp{}, i16 -1, i16 0\n",
                        dest, dest
                    ));
                }
            }
            IrInstruction::Branch { target } => {
                out.push_str(&format!("  br label %{}\n", target));
            }
            IrInstruction::BranchIf {
                condition,
                true_target,
                false_target,
            } => {
                let (cond_val, cond_ty) = self.format_operand(condition, local_types, global_types, temp_types, string_constants);
                *branch_counter += 1;
                out.push_str(&format!(
                    "  %cond_bool_{} = icmp ne {} {}, 0\n",
                    *branch_counter, cond_ty, cond_val
                ));
                out.push_str(&format!(
                    "  br i1 %cond_bool_{}, label %{}, label %{}\n",
                    *branch_counter, true_target, false_target
                ));
            }
            IrInstruction::Call {
                dest,
                name,
                args,
                return_type,
            } => {
                if name.eq_ignore_ascii_case("MsgBox") {
                    let prompt_str = if let Some(arg0) = args.get(0) {
                        let (val, _) = self.format_operand(arg0, local_types, global_types, temp_types, string_constants);
                        val
                    } else {
                        "null".to_string()
                    };

                    let title_str = if let Some(arg1) = args.get(1) {
                        let (val, _) = self.format_operand(arg1, local_types, global_types, temp_types, string_constants);
                        val
                    } else {
                        "null".to_string()
                    };

                    let buttons_str = if let Some(arg2) = args.get(2) {
                        let (val, _) = self.format_operand(arg2, local_types, global_types, temp_types, string_constants);
                        val
                    } else {
                        "0".to_string()
                    };

                    if let Some(dest_id) = dest {
                        temp_types.insert(*dest_id, "i32".to_string());
                        out.push_str(&format!(
                            "  %t{} = call i32 @vb6_msgbox(ptr {}, ptr {}, i32 {})\n",
                            dest_id, prompt_str, title_str, buttons_str
                        ));
                    } else {
                        out.push_str(&format!(
                            "  call i32 @vb6_msgbox(ptr {}, ptr {}, i32 {})\n",
                            prompt_str, title_str, buttons_str
                        ));
                    }
                    return;
                }

                let sig_opt = Self::get_runtime_func_sig(name);
                let (fn_symbol, ret_llvm_ty) = if let Some((rt_ret_ty, _)) = sig_opt {
                    (format!("@{}", name), rt_ret_ty.to_string())
                } else if name.starts_with("vb6_") {
                    let rty = return_type.as_ref().map(Self::llvm_type).unwrap_or("void");
                    (format!("@{}", name), rty.to_string())
                } else {
                    let rty = return_type.as_ref().map(Self::llvm_type).unwrap_or("void");
                    (format!("@\"{}\"", name), rty.to_string())
                };

                let mut args_formatted: Vec<String> = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let (arg_val, arg_ty) = self.format_operand(arg, local_types, global_types, temp_types, string_constants);
                    if arg_ty.starts_with('[') && arg_ty.ends_with(" x i8]") && arg_ty != "[24 x i8]" {
                        let ptr_val = self.format_base_ptr(arg, byref_params, out, branch_counter);
                        args_formatted.push(format!("ptr {}", ptr_val));
                        continue;
                    }
                    if let Some((_, param_types)) = sig_opt {
                        if let Some(&expected_ty) = param_types.get(i) {
                            let (cast_val, final_ty) = cast_arg(&arg_val, &arg_ty, expected_ty, out, branch_counter);
                            args_formatted.push(format!("{} {}", final_ty, cast_val));
                            continue;
                        }
                    }
                    args_formatted.push(format!("{} {}", arg_ty, arg_val));
                }

                if let Some(dest_id) = dest {
                    temp_types.insert(*dest_id, ret_llvm_ty.clone());
                    out.push_str(&format!(
                        "  %t{} = call {} {}({})\n",
                        dest_id,
                        ret_llvm_ty,
                        fn_symbol,
                        args_formatted.join(", ")
                    ));
                } else {
                    out.push_str(&format!(
                        "  call {} {}({})\n",
                        ret_llvm_ty,
                        fn_symbol,
                        args_formatted.join(", ")
                    ));
                }
            }
            IrInstruction::Return { value } => {
                if let Some(val_op) = value {
                    let (v_str, v_ty) = self.format_operand(val_op, local_types, global_types, temp_types, string_constants);
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
            IrInstruction::StringConcat { dest, left, right } => {
                let (left_val, _) = self.format_operand(left, local_types, global_types, temp_types, string_constants);
                let (right_val, _) = self.format_operand(right, local_types, global_types, temp_types, string_constants);
                temp_types.insert(*dest, "ptr".to_string());
                out.push_str(&format!(
                    "  %t{} = call ptr @vb6_string_concat(ptr {}, ptr {})\n",
                    dest, left_val, right_val
                ));
            }
            IrInstruction::Coerce {
                dest,
                src,
                from_type,
                to_type,
            } => {
                let (src_val, _) = self.format_operand(src, local_types, global_types, temp_types, string_constants);
                let to_llvm_ty = Self::llvm_type(to_type);
                temp_types.insert(*dest, to_llvm_ty.to_string());

                if to_type == &VbType::Variant {
                    *branch_counter += 1;
                    let c_id = *branch_counter;
                    out.push_str(&format!("  %var_dest_slot_{} = alloca [24 x i8], align 8\n", c_id));
                    out.push_str(&format!("  call void @vb6_variant_init(ptr %var_dest_slot_{})\n", c_id));
                    match from_type {
                        VbType::Byte => {
                            out.push_str(&format!("  call void @vb6_variant_from_i8(ptr %var_dest_slot_{}, i8 {})\n", c_id, src_val));
                        }
                        VbType::Boolean => {
                            out.push_str(&format!("  call void @vb6_variant_from_bool(ptr %var_dest_slot_{}, i16 {})\n", c_id, src_val));
                        }
                        VbType::Integer => {
                            out.push_str(&format!("  call void @vb6_variant_from_i16(ptr %var_dest_slot_{}, i16 {})\n", c_id, src_val));
                        }
                        VbType::Long | VbType::Enum(_) => {
                            out.push_str(&format!("  call void @vb6_variant_from_i32(ptr %var_dest_slot_{}, i32 {})\n", c_id, src_val));
                        }
                        VbType::LongPtr | VbType::LongLong | VbType::Currency => {
                            out.push_str(&format!("  call void @vb6_variant_from_i64(ptr %var_dest_slot_{}, i64 {})\n", c_id, src_val));
                        }
                        VbType::Single => {
                            out.push_str(&format!("  call void @vb6_variant_from_f32(ptr %var_dest_slot_{}, float {})\n", c_id, src_val));
                        }
                        VbType::Double => {
                            out.push_str(&format!("  call void @vb6_variant_from_f64(ptr %var_dest_slot_{}, double {})\n", c_id, src_val));
                        }
                        VbType::Date => {
                            out.push_str(&format!("  call void @vb6_variant_from_date(ptr %var_dest_slot_{}, double {})\n", c_id, src_val));
                        }
                        VbType::String => {
                            out.push_str(&format!("  call void @vb6_variant_from_str(ptr %var_dest_slot_{}, ptr {})\n", c_id, src_val));
                        }
                        VbType::Variant => {
                            out.push_str(&format!("  %var_src_slot_{} = alloca [24 x i8], align 8\n", c_id));
                            out.push_str(&format!("  store [24 x i8] {}, ptr %var_src_slot_{}\n", src_val, c_id));
                            out.push_str(&format!("  call void @vb6_variant_copy(ptr %var_dest_slot_{}, ptr %var_src_slot_{})\n", c_id, c_id));
                        }
                        _ => {
                            out.push_str(&format!("  call void @vb6_variant_from_empty(ptr %var_dest_slot_{})\n", c_id));
                        }
                    }
                    out.push_str(&format!("  %t{} = load [24 x i8], ptr %var_dest_slot_{}\n", dest, c_id));
                } else if from_type == &VbType::Variant {
                    *branch_counter += 1;
                    let c_id = *branch_counter;
                    out.push_str(&format!("  %var_src_slot_{} = alloca [24 x i8], align 8\n", c_id));
                    out.push_str(&format!("  store [24 x i8] {}, ptr %var_src_slot_{}\n", src_val, c_id));
                    match to_type {
                        VbType::Byte => {
                            out.push_str(&format!("  %t{} = call i8 @vb6_variant_to_i8(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        VbType::Boolean => {
                            out.push_str(&format!("  %t{} = call i16 @vb6_variant_to_bool(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        VbType::Integer => {
                            out.push_str(&format!("  %t{} = call i16 @vb6_variant_to_i16(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        VbType::Long | VbType::Enum(_) => {
                            out.push_str(&format!("  %t{} = call i32 @vb6_variant_to_i32(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        VbType::LongPtr | VbType::LongLong | VbType::Currency => {
                            out.push_str(&format!("  %t{} = call i64 @vb6_variant_to_i64(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        VbType::Single => {
                            out.push_str(&format!("  %t{} = call float @vb6_variant_to_f32(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        VbType::Double | VbType::Date => {
                            out.push_str(&format!("  %t{} = call double @vb6_variant_to_f64(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        VbType::String => {
                            out.push_str(&format!("  %t{} = call ptr @vb6_variant_to_str(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                        _ => {
                            out.push_str(&format!("  %t{} = call i32 @vb6_variant_to_i32(ptr %var_src_slot_{})\n", dest, c_id));
                        }
                    }
                } else if to_type == &VbType::String {
                    if from_type == &VbType::String {
                        out.push_str(&format!(
                            "  %t{} = call ptr @vb6_string_alloc(ptr {})\n",
                            dest, src_val
                        ));
                    } else if from_type == &VbType::Boolean {
                        *branch_counter += 1;
                        let c_id = *branch_counter;
                        out.push_str(&format!("  %bool_cast_{} = sext i16 {} to i32\n", c_id, src_val));
                        out.push_str(&format!(
                            "  %t{} = call ptr @vb6_str_from_bool(i32 %bool_cast_{})\n",
                            dest, c_id
                        ));
                    } else if from_type.is_integer() {
                        *branch_counter += 1;
                        let c_id = *branch_counter;
                        let from_llvm_ty = Self::llvm_type(from_type);
                        if from_llvm_ty != "i64" {
                            out.push_str(&format!("  %int_cast_{} = sext {} {} to i64\n", c_id, from_llvm_ty, src_val));
                            out.push_str(&format!(
                                "  %t{} = call ptr @vb6_str_from_i64(i64 %int_cast_{})\n",
                                dest, c_id
                            ));
                        } else {
                            out.push_str(&format!(
                                "  %t{} = call ptr @vb6_str_from_i64(i64 {})\n",
                                dest, src_val
                            ));
                        }
                    } else if from_type.is_floating() || from_type == &VbType::Date {
                        *branch_counter += 1;
                        let c_id = *branch_counter;
                        let from_llvm_ty = Self::llvm_type(from_type);
                        if from_llvm_ty == "float" {
                            out.push_str(&format!("  %flt_cast_{} = fpext float {} to double\n", c_id, src_val));
                            out.push_str(&format!(
                                "  %t{} = call ptr @vb6_str_from_f64(double %flt_cast_{})\n",
                                dest, c_id
                            ));
                        } else {
                            out.push_str(&format!(
                                "  %t{} = call ptr @vb6_str_from_f64(double {})\n",
                                dest, src_val
                            ));
                        }
                    } else {
                        out.push_str(&format!(
                            "  %t{} = call ptr @vb6_string_alloc(ptr {})\n",
                            dest, src_val
                        ));
                    }
                } else if from_type == &VbType::String {
                    *branch_counter += 1;
                    let c_id = *branch_counter;
                    out.push_str(&format!(
                        "  %val_dbl_{} = call double @vb6_string_val(ptr {})\n",
                        c_id, src_val
                    ));
                    if to_type.is_floating() {
                        if to_type == &VbType::Single {
                            out.push_str(&format!("  %t{} = fptrunc double %val_dbl_{} to float\n", dest, c_id));
                        } else {
                            out.push_str(&format!("  %t{} = fadd double %val_dbl_{}, 0.0\n", dest, c_id));
                        }
                    } else if to_type.is_integer() {
                        out.push_str(&format!("  %t{} = fptosi double %val_dbl_{} to {}\n", dest, c_id, to_llvm_ty));
                    } else if to_type == &VbType::Boolean {
                        out.push_str(&format!("  %t{} = fptosi double %val_dbl_{} to i16\n", dest, c_id));
                    } else if to_llvm_ty != "ptr" {
                        out.push_str(&format!("  %t{} = fptosi double %val_dbl_{} to {}\n", dest, c_id, to_llvm_ty));
                    }
                } else if to_type == &VbType::Boolean {
                    if from_type.is_floating() {
                        *branch_counter += 1;
                        let c_id = *branch_counter;
                        out.push_str(&format!("  %cmp_bool_{} = fcmp one {} {}, 0.0\n", c_id, Self::llvm_type(from_type), src_val));
                        out.push_str(&format!("  %t{} = select i1 %cmp_bool_{}, i16 -1, i16 0\n", dest, c_id));
                    } else if from_type.is_integer() {
                        *branch_counter += 1;
                        let c_id = *branch_counter;
                        out.push_str(&format!("  %cmp_bool_{} = icmp ne {} {}, 0\n", c_id, Self::llvm_type(from_type), src_val));
                        out.push_str(&format!("  %t{} = select i1 %cmp_bool_{}, i16 -1, i16 0\n", dest, c_id));
                    } else {
                        out.push_str(&format!("  %t{} = add i16 {}, 0\n", dest, src_val));
                    }
                } else if from_type == &VbType::Boolean {
                    if to_type.is_floating() {
                        out.push_str(&format!("  %t{} = sitofp i16 {} to {}\n", dest, src_val, to_llvm_ty));
                    } else if to_type == &VbType::Byte {
                        out.push_str(&format!("  %t{} = trunc i16 {} to i8\n", dest, src_val));
                    } else if to_type.is_integer() {
                        let to_size = to_type.size_in_bytes(true);
                        if to_size > 2 {
                            out.push_str(&format!("  %t{} = sext i16 {} to {}\n", dest, src_val, to_llvm_ty));
                        } else {
                            out.push_str(&format!("  %t{} = add i16 {}, 0\n", dest, src_val));
                        }
                    } else {
                        out.push_str(&format!("  %t{} = add i16 {}, 0\n", dest, src_val));
                    }
                } else if from_type.is_integer() && to_type.is_integer() {
                    let from_size = from_type.size_in_bytes(true);
                    let to_size = to_type.size_in_bytes(true);
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
                            "  %t{} = add {} {}, 0\n",
                            dest,
                            Self::llvm_type(from_type),
                            src_val
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
                } else if from_type.is_floating() && to_type.is_floating() {
                    if from_type == &VbType::Single && to_type == &VbType::Double {
                        out.push_str(&format!("  %t{} = fpext float {} to double\n", dest, src_val));
                    } else if from_type == &VbType::Double && to_type == &VbType::Single {
                        out.push_str(&format!("  %t{} = fptrunc double {} to float\n", dest, src_val));
                    } else {
                        out.push_str(&format!("  %t{} = fadd {} {}, 0.0\n", dest, to_llvm_ty, src_val));
                    }
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
                    let (arg_val, arg_ty) = self.format_operand(arg, local_types, global_types, temp_types, string_constants);
                    match arg_ty.as_str() {
                        "[24 x i8]" => {
                            *branch_counter += 1;
                            let c_id = *branch_counter;
                            out.push_str(&format!("  %var_prn_slot_{} = alloca [24 x i8], align 8\n", c_id));
                            out.push_str(&format!("  store [24 x i8] {}, ptr %var_prn_slot_{}\n", arg_val, c_id));
                            out.push_str(&format!("  call void @vb6_debug_print_variant(ptr %var_prn_slot_{})\n", c_id));
                        }
                        "ptr" => out.push_str(&format!("  call void @vb6_debug_print(ptr {})\n", arg_val)),
                        "i8" => out.push_str(&format!("  call void @vb6_debug_print_i8(i8 {})\n", arg_val)),
                        "i16" => out.push_str(&format!("  call void @vb6_debug_print_i16(i16 {})\n", arg_val)),
                        "i32" => out.push_str(&format!("  call void @vb6_debug_print_i32(i32 {})\n", arg_val)),
                        "i64" => out.push_str(&format!("  call void @vb6_debug_print_i64(i64 {})\n", arg_val)),
                        "float" => out.push_str(&format!("  call void @vb6_debug_print_f32(float {})\n", arg_val)),
                        "double" => out.push_str(&format!("  call void @vb6_debug_print_f64(double {})\n", arg_val)),
                        _ => out.push_str(&format!("  call void @vb6_debug_print(ptr {})\n", arg_val)),
                    }
                }
            }
            IrInstruction::ArrayCreate {
                dest,
                dims,
                bounds,
                elem_type,
            } => {
                let bounds_count = dims * 2;
                out.push_str(&format!(
                    "  %arr_bounds_{} = alloca [{} x i32], align 4\n",
                    dest, bounds_count
                ));
                for (i, (lb, ub)) in bounds.iter().enumerate() {
                    let (lb_val, lb_ty) = self.format_operand(lb, local_types, global_types, temp_types, string_constants);
                    let (ub_val, ub_ty) = self.format_operand(ub, local_types, global_types, temp_types, string_constants);

                    let lb_i32 = if lb_ty != "i32" {
                        let cast_lb = format!("%arr_lb_cast_{}_{}", dest, i);
                        out.push_str(&format!("  {} = sext {} {} to i32\n", cast_lb, lb_ty, lb_val));
                        cast_lb
                    } else {
                        lb_val
                    };

                    let ub_i32 = if ub_ty != "i32" {
                        let cast_ub = format!("%arr_ub_cast_{}_{}", dest, i);
                        out.push_str(&format!("  {} = sext {} {} to i32\n", cast_ub, ub_ty, ub_val));
                        cast_ub
                    } else {
                        ub_val
                    };

                    out.push_str(&format!(
                        "  %arr_lb_ptr_{}_{} = getelementptr inbounds [{} x i32], ptr %arr_bounds_{}, i32 0, i32 {}\n",
                        dest, i, bounds_count, dest, i * 2
                    ));
                    out.push_str(&format!(
                        "  store i32 {}, ptr %arr_lb_ptr_{}_{}\n",
                        lb_i32, dest, i
                    ));

                    out.push_str(&format!(
                        "  %arr_ub_ptr_{}_{} = getelementptr inbounds [{} x i32], ptr %arr_bounds_{}, i32 0, i32 {}\n",
                        dest, i, bounds_count, dest, i * 2 + 1
                    ));
                    out.push_str(&format!(
                        "  store i32 {}, ptr %arr_ub_ptr_{}_{}\n",
                        ub_i32, dest, i
                    ));
                }

                out.push_str(&format!(
                    "  %arr_bounds_decay_{} = getelementptr inbounds [{} x i32], ptr %arr_bounds_{}, i32 0, i32 0\n",
                    dest, bounds_count, dest
                ));
                let elem_sz = elem_size_bytes(elem_type, udt_layouts);
                out.push_str(&format!(
                    "  %t{} = call ptr @vb6_array_create(i32 {}, ptr %arr_bounds_decay_{}, i32 {})\n",
                    dest, dims, dest, elem_sz
                ));
                temp_types.insert(*dest, "ptr".to_string());
            }
            IrInstruction::ArrayElementPtr {
                dest,
                array,
                indices,
                ..
            } => {
                let (arr_val, _) = self.format_operand(array, local_types, global_types, temp_types, string_constants);
                let dims = indices.len();
                out.push_str(&format!(
                    "  %arr_idx_{} = alloca [{} x i32], align 4\n",
                    dest, dims
                ));
                for (i, idx_op) in indices.iter().enumerate() {
                    let (idx_val, idx_ty) = self.format_operand(idx_op, local_types, global_types, temp_types, string_constants);
                    let idx_i32 = if idx_ty != "i32" {
                        let cast_idx = format!("%arr_idx_cast_{}_{}", dest, i);
                        out.push_str(&format!("  {} = sext {} {} to i32\n", cast_idx, idx_ty, idx_val));
                        cast_idx
                    } else {
                        idx_val
                    };
                    out.push_str(&format!(
                        "  %arr_idx_slot_{}_{} = getelementptr inbounds [{} x i32], ptr %arr_idx_{}, i32 0, i32 {}\n",
                        dest, i, dims, dest, i
                    ));
                    out.push_str(&format!(
                        "  store i32 {}, ptr %arr_idx_slot_{}_{}\n",
                        idx_i32, dest, i
                    ));
                }
                out.push_str(&format!(
                    "  %arr_idx_decay_{} = getelementptr inbounds [{} x i32], ptr %arr_idx_{}, i32 0, i32 0\n",
                    dest, dims, dest
                ));
                out.push_str(&format!(
                    "  %t{} = call ptr @vb6_array_element_ptr(ptr {}, i32 {}, ptr %arr_idx_decay_{})\n",
                    dest, arr_val, dims, dest
                ));
                temp_types.insert(*dest, "ptr".to_string());
            }
            IrInstruction::ArrayGetElement {
                dest,
                array,
                indices,
                elem_type,
            } => {
                let (arr_val, _) = self.format_operand(array, local_types, global_types, temp_types, string_constants);
                let dims = indices.len();
                out.push_str(&format!(
                    "  %arr_get_idx_{} = alloca [{} x i32], align 4\n",
                    dest, dims
                ));
                for (i, idx_op) in indices.iter().enumerate() {
                    let (idx_val, idx_ty) = self.format_operand(idx_op, local_types, global_types, temp_types, string_constants);
                    let idx_i32 = if idx_ty != "i32" {
                        let cast_idx = format!("%arr_get_cast_{}_{}", dest, i);
                        out.push_str(&format!("  {} = sext {} {} to i32\n", cast_idx, idx_ty, idx_val));
                        cast_idx
                    } else {
                        idx_val
                    };
                    out.push_str(&format!(
                        "  %arr_get_slot_{}_{} = getelementptr inbounds [{} x i32], ptr %arr_get_idx_{}, i32 0, i32 {}\n",
                        dest, i, dims, dest, i
                    ));
                    out.push_str(&format!(
                        "  store i32 {}, ptr %arr_get_slot_{}_{}\n",
                        idx_i32, dest, i
                    ));
                }
                out.push_str(&format!(
                    "  %arr_get_decay_{} = getelementptr inbounds [{} x i32], ptr %arr_get_idx_{}, i32 0, i32 0\n",
                    dest, dims, dest
                ));
                out.push_str(&format!(
                    "  %arr_elem_ptr_{} = call ptr @vb6_array_element_ptr(ptr {}, i32 {}, ptr %arr_get_decay_{})\n",
                    dest, arr_val, dims, dest
                ));
                if matches!(elem_type, VbType::UserDefined(_)) {
                    out.push_str(&format!(
                        "  %t{} = bitcast ptr %arr_elem_ptr_{} to ptr\n",
                        dest, dest
                    ));
                    temp_types.insert(*dest, "ptr".to_string());
                } else {
                    let llvm_ty = Self::llvm_type(elem_type);
                    out.push_str(&format!(
                        "  %t{} = load {}, ptr %arr_elem_ptr_{}\n",
                        dest, llvm_ty, dest
                    ));
                    temp_types.insert(*dest, llvm_ty.to_string());
                }
            }
            IrInstruction::ArraySetElement {
                array,
                indices,
                value,
                elem_type,
            } => {
                let (arr_val, _) = self.format_operand(array, local_types, global_types, temp_types, string_constants);
                let dims = indices.len();
                let set_id = *branch_counter;
                *branch_counter += 1;
                out.push_str(&format!(
                    "  %arr_set_idx_{} = alloca [{} x i32], align 4\n",
                    set_id, dims
                ));
                for (i, idx_op) in indices.iter().enumerate() {
                    let (idx_val, idx_ty) = self.format_operand(idx_op, local_types, global_types, temp_types, string_constants);
                    let idx_i32 = if idx_ty != "i32" {
                        let cast_idx = format!("%arr_set_cast_{}_{}", set_id, i);
                        out.push_str(&format!("  {} = sext {} {} to i32\n", cast_idx, idx_ty, idx_val));
                        cast_idx
                    } else {
                        idx_val
                    };
                    out.push_str(&format!(
                        "  %arr_set_slot_{}_{} = getelementptr inbounds [{} x i32], ptr %arr_set_idx_{}, i32 0, i32 {}\n",
                        set_id, i, dims, set_id, i
                    ));
                    out.push_str(&format!(
                        "  store i32 {}, ptr %arr_set_slot_{}_{}\n",
                        idx_i32, set_id, i
                    ));
                }
                out.push_str(&format!(
                    "  %arr_set_decay_{} = getelementptr inbounds [{} x i32], ptr %arr_set_idx_{}, i32 0, i32 0\n",
                    set_id, dims, set_id
                ));
                out.push_str(&format!(
                    "  %arr_set_elem_ptr_{} = call ptr @vb6_array_element_ptr(ptr {}, i32 {}, ptr %arr_set_decay_{})\n",
                    set_id, arr_val, dims, set_id
                ));
                if let VbType::UserDefined(ref udt_name) = elem_type {
                    let total_sz = udt_layouts.get(&udt_name.to_ascii_uppercase()).map(|l| l.total_size).unwrap_or(8);
                    let src_ptr = self.format_base_ptr(value, byref_params, out, branch_counter);
                    out.push_str(&format!(
                        "  call void @vb6_memory_copy(ptr %arr_set_elem_ptr_{}, ptr {}, i64 {})\n",
                        set_id, src_ptr, total_sz
                    ));
                } else {
                    let (val_str, val_ty) = self.format_operand(value, local_types, global_types, temp_types, string_constants);
                    let elem_llvm_ty = Self::llvm_type(elem_type);
                    let final_val = if val_ty != elem_llvm_ty {
                        let cast_val = format!("%arr_set_val_cast_{}", set_id);
                        if val_ty == "i32" && elem_llvm_ty == "i64" {
                            out.push_str(&format!("  {} = sext i32 {} to i64\n", cast_val, val_str));
                            cast_val
                        } else if val_ty == "i16" && elem_llvm_ty == "i32" {
                            out.push_str(&format!("  {} = sext i16 {} to i32\n", cast_val, val_str));
                            cast_val
                        } else {
                            val_str
                        }
                    } else {
                        val_str
                    };
                    out.push_str(&format!(
                        "  store {} {}, ptr %arr_set_elem_ptr_{}\n",
                        elem_llvm_ty, final_val, set_id
                    ));
                }
            }
            IrInstruction::ArrayReDim {
                dest,
                array,
                dims,
                bounds,
                elem_type,
                preserve,
            } => {
                let (arr_val, _) = self.format_operand(array, local_types, global_types, temp_types, string_constants);
                let bounds_count = dims * 2;
                out.push_str(&format!(
                    "  %arr_redim_bounds_{} = alloca [{} x i32], align 4\n",
                    dest, bounds_count
                ));
                for (i, (lb, ub)) in bounds.iter().enumerate() {
                    let (lb_val, lb_ty) = self.format_operand(lb, local_types, global_types, temp_types, string_constants);
                    let (ub_val, ub_ty) = self.format_operand(ub, local_types, global_types, temp_types, string_constants);

                    let lb_i32 = if lb_ty != "i32" {
                        let cast_lb = format!("%arr_redim_lb_cast_{}_{}", dest, i);
                        out.push_str(&format!("  {} = sext {} {} to i32\n", cast_lb, lb_ty, lb_val));
                        cast_lb
                    } else {
                        lb_val
                    };

                    let ub_i32 = if ub_ty != "i32" {
                        let cast_ub = format!("%arr_redim_ub_cast_{}_{}", dest, i);
                        out.push_str(&format!("  {} = sext {} {} to i32\n", cast_ub, ub_ty, ub_val));
                        cast_ub
                    } else {
                        ub_val
                    };

                    out.push_str(&format!(
                        "  %arr_redim_lb_ptr_{}_{} = getelementptr inbounds [{} x i32], ptr %arr_redim_bounds_{}, i32 0, i32 {}\n",
                        dest, i, bounds_count, dest, i * 2
                    ));
                    out.push_str(&format!(
                        "  store i32 {}, ptr %arr_redim_lb_ptr_{}_{}\n",
                        lb_i32, dest, i
                    ));

                    out.push_str(&format!(
                        "  %arr_redim_ub_ptr_{}_{} = getelementptr inbounds [{} x i32], ptr %arr_redim_bounds_{}, i32 0, i32 {}\n",
                        dest, i, bounds_count, dest, i * 2 + 1
                    ));
                    out.push_str(&format!(
                        "  store i32 {}, ptr %arr_redim_ub_ptr_{}_{}\n",
                        ub_i32, dest, i
                    ));
                }

                out.push_str(&format!(
                    "  %arr_redim_decay_{} = getelementptr inbounds [{} x i32], ptr %arr_redim_bounds_{}, i32 0, i32 0\n",
                    dest, bounds_count, dest
                ));
                let elem_sz = elem_size_bytes(elem_type, udt_layouts);
                let pres_int = if *preserve { 1 } else { 0 };
                out.push_str(&format!(
                    "  %t{} = call ptr @vb6_array_redim(ptr {}, i32 {}, ptr %arr_redim_decay_{}, i32 {}, i32 {})\n",
                    dest, arr_val, dims, dest, elem_sz, pres_int
                ));
                temp_types.insert(*dest, "ptr".to_string());
            }
            IrInstruction::ArrayBound {
                dest,
                array,
                dim,
                is_ubound,
            } => {
                let (arr_val, _) = self.format_operand(array, local_types, global_types, temp_types, string_constants);
                let (dim_val, dim_ty) = self.format_operand(dim, local_types, global_types, temp_types, string_constants);
                let dim_i32 = if dim_ty != "i32" {
                    let cast_dim = format!("%arr_bound_dim_cast_{}", dest);
                    out.push_str(&format!("  {} = sext {} {} to i32\n", cast_dim, dim_ty, dim_val));
                    cast_dim
                } else {
                    dim_val
                };
                let func_name = if *is_ubound { "vb6_array_ubound" } else { "vb6_array_lbound" };
                out.push_str(&format!(
                    "  %t{} = call i32 @{}(ptr {}, i32 {})\n",
                    dest, func_name, arr_val, dim_i32
                ));
                temp_types.insert(*dest, "i32".to_string());
            }
            IrInstruction::GetMember {
                dest,
                base,
                offset,
                member_type,
            } => {
                let base_ptr = self.format_base_ptr(base, byref_params, out, branch_counter);
                *branch_counter += 1;
                let member_ptr = format!("%mem_ptr_{}", branch_counter);
                out.push_str(&format!(
                    "  {} = getelementptr i8, ptr {}, i64 {}\n",
                    member_ptr, base_ptr, offset
                ));
                match member_type {
                    VbType::UserDefined(_) => {
                        out.push_str(&format!("  %t{} = bitcast ptr {} to ptr\n", dest, member_ptr));
                        temp_types.insert(*dest, "ptr".to_string());
                    }
                    VbType::FixedString(len) => {
                        out.push_str(&format!(
                            "  %t{} = call ptr @vb6_fixed_str_get(ptr {}, i64 {})\n",
                            dest, member_ptr, len
                        ));
                        temp_types.insert(*dest, "ptr".to_string());
                    }
                    VbType::Variant => {
                        out.push_str(&format!(
                            "  %t{} = load [24 x i8], ptr {}\n",
                            dest, member_ptr
                        ));
                        temp_types.insert(*dest, "[24 x i8]".to_string());
                    }
                    _ => {
                        let llvm_ty = Self::llvm_type(member_type);
                        out.push_str(&format!(
                            "  %t{} = load {}, ptr {}\n",
                            dest, llvm_ty, member_ptr
                        ));
                        temp_types.insert(*dest, llvm_ty.to_string());
                    }
                }
            }
            IrInstruction::SetMember {
                base,
                offset,
                value,
                member_type,
            } => {
                let base_ptr = self.format_base_ptr(base, byref_params, out, branch_counter);
                *branch_counter += 1;
                let member_ptr = format!("%mem_ptr_{}", branch_counter);
                out.push_str(&format!(
                    "  {} = getelementptr i8, ptr {}, i64 {}\n",
                    member_ptr, base_ptr, offset
                ));
                let (val_str, val_ty) = self.format_operand(value, local_types, global_types, temp_types, string_constants);
                match member_type {
                    VbType::UserDefined(udt_name) => {
                        let total_sz = udt_layouts.get(&udt_name.to_ascii_uppercase()).map(|l| l.total_size).unwrap_or(8);
                        let src_ptr = self.format_base_ptr(value, byref_params, out, branch_counter);
                        out.push_str(&format!(
                            "  call void @vb6_memory_copy(ptr {}, ptr {}, i64 {})\n",
                            member_ptr, src_ptr, total_sz
                        ));
                    }
                    VbType::FixedString(len) => {
                        out.push_str(&format!(
                            "  call void @vb6_fixed_str_set(ptr {}, i64 {}, ptr {})\n",
                            member_ptr, len, val_str
                        ));
                    }
                    VbType::Variant => {
                        if val_ty == "[24 x i8]" {
                            *branch_counter += 1;
                            let c_id = *branch_counter;
                            out.push_str(&format!("  %var_src_{} = alloca [24 x i8], align 8\n", c_id));
                            out.push_str(&format!("  store [24 x i8] {}, ptr %var_src_{}\n", val_str, c_id));
                            out.push_str(&format!("  call void @vb6_variant_copy(ptr {}, ptr %var_src_{})\n", member_ptr, c_id));
                        } else {
                            out.push_str(&format!("  store {} {}, ptr {}\n", val_ty, val_str, member_ptr));
                        }
                    }
                    _ => {
                        let target_llvm_ty = Self::llvm_type(member_type);
                        let final_val = if val_ty != target_llvm_ty {
                            *branch_counter += 1;
                            let cast_v = format!("%set_mem_cast_{}", branch_counter);
                            if val_ty == "i32" && target_llvm_ty == "i64" {
                                out.push_str(&format!("  {} = sext i32 {} to i64\n", cast_v, val_str));
                                cast_v
                            } else if val_ty == "i16" && target_llvm_ty == "i32" {
                                out.push_str(&format!("  {} = sext i16 {} to i32\n", cast_v, val_str));
                                cast_v
                            } else {
                                val_str
                            }
                        } else {
                            val_str
                        };
                        out.push_str(&format!(
                            "  store {} {}, ptr {}\n",
                            target_llvm_ty, final_val, member_ptr
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    fn format_operand(
        &self,
        op: &Operand,
        local_types: &HashMap<String, (String, usize)>,
        global_types: &HashMap<String, String>,
        temp_types: &HashMap<usize, String>,
        string_constants: &HashMap<String, String>,
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
            Operand::Global(name) => {
                let ty = global_types
                    .get(&name.to_ascii_uppercase())
                    .cloned()
                    .unwrap_or_else(|| "i32".to_string());
                (format!("@\"{}\"", name), ty)
            }
            Operand::Constant(c) => match c {
                IrConstant::Int(v, ty) => (v.to_string(), Self::llvm_type(ty).to_string()),
                IrConstant::Float(v) => (format!("{:.6}", v), "double".to_string()),
                IrConstant::String(s) => {
                    if let Some(global_name) = string_constants.get(s) {
                        (global_name.clone(), "ptr".to_string())
                    } else {
                        ("null".to_string(), "ptr".to_string())
                    }
                }
                IrConstant::Bool(b) => {
                    let val = if *b { "-1" } else { "0" };
                    (val.to_string(), "i16".to_string())
                }
                IrConstant::Date(_) => ("0.0".to_string(), "double".to_string()),
                IrConstant::Empty => {
                    ("zeroinitializer".to_string(), "[24 x i8]".to_string())
                }
                IrConstant::Null | IrConstant::Nothing => {
                    ("null".to_string(), "ptr".to_string())
                }
            },
        }
    }
}

fn block_ends_with_terminator(block: &BasicBlock) -> bool {
    block.instructions.iter().any(|inst| {
        matches!(
            inst,
            IrInstruction::Branch { .. }
                | IrInstruction::BranchIf { .. }
                | IrInstruction::Return { .. }
                | IrInstruction::Resume { .. }
        )
    })
}

fn escape_llvm_string(s: &str) -> (String, usize) {
    let mut out = String::from("c\"");
    let mut len = 0;
    for b in s.bytes() {
        len += 1;
        if b >= 32 && b <= 126 && b != b'\\' && b != b'"' {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\{:02X}", b));
        }
    }
    out.push_str("\\00\"");
    len += 1;
    (out, len)
}

fn elem_size_bytes(vb_type: &VbType, udt_layouts: &HashMap<String, UdtLayout>) -> usize {
    match vb_type {
        VbType::Byte => 1,
        VbType::Integer | VbType::Boolean => 2,
        VbType::Long | VbType::Enum(_) | VbType::Single => 4,
        VbType::LongPtr | VbType::LongLong | VbType::Currency | VbType::Double | VbType::Date => 8,
        VbType::String | VbType::Object | VbType::Array(_) => 8,
        VbType::UserDefined(name) => {
            udt_layouts.get(&name.to_ascii_uppercase()).map(|l| l.total_size).unwrap_or(8)
        }
        VbType::FixedString(len) => *len,
        VbType::Variant => 24,
        _ => 8,
    }
}

fn cast_arg(
    val: &str,
    ty: &str,
    target_ty: &str,
    out: &mut String,
    counter: &mut usize,
) -> (String, String) {
    if ty == target_ty {
        return (val.to_string(), ty.to_string());
    }
    *counter += 1;
    let cast_name = format!("%arg_cast_{}", counter);
    match target_ty {
        "i32" => {
            if ty == "i8" || ty == "i16" {
                out.push_str(&format!("  {} = sext {} {} to i32\n", cast_name, ty, val));
                (cast_name, "i32".to_string())
            } else if ty == "i64" {
                out.push_str(&format!("  {} = trunc i64 {} to i32\n", cast_name, val));
                (cast_name, "i32".to_string())
            } else if ty == "double" || ty == "float" {
                out.push_str(&format!("  {} = fptosi {} {} to i32\n", cast_name, ty, val));
                (cast_name, "i32".to_string())
            } else {
                (val.to_string(), ty.to_string())
            }
        }
        "i64" => {
            if ty == "i8" || ty == "i16" || ty == "i32" {
                out.push_str(&format!("  {} = sext {} {} to i64\n", cast_name, ty, val));
                (cast_name, "i64".to_string())
            } else if ty == "double" || ty == "float" {
                out.push_str(&format!("  {} = fptosi {} {} to i64\n", cast_name, ty, val));
                (cast_name, "i64".to_string())
            } else {
                (val.to_string(), ty.to_string())
            }
        }
        "double" => {
            if ty == "float" {
                out.push_str(&format!("  {} = fpext float {} to double\n", cast_name, val));
                (cast_name, "double".to_string())
            } else if ty == "i8" || ty == "i16" || ty == "i32" || ty == "i64" {
                out.push_str(&format!("  {} = sitofp {} {} to double\n", cast_name, ty, val));
                (cast_name, "double".to_string())
            } else {
                (val.to_string(), ty.to_string())
            }
        }
        "ptr" => {
            if ty == "[24 x i8]" {
                let slot = format!("%var_arg_slot_{}", counter);
                out.push_str(&format!("  {} = alloca [24 x i8], align 8\n", slot));
                out.push_str(&format!("  store [24 x i8] {}, ptr {}\n", val, slot));
                (slot, "ptr".to_string())
            } else {
                (val.to_string(), ty.to_string())
            }
        }
        _ => (val.to_string(), ty.to_string()),
    }
}

