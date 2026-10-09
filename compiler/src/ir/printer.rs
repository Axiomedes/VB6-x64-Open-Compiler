//! Textual formatter and printer for VB6 Intermediate Representation (IR).

use crate::ir::model::*;

pub struct IrPrinter;

impl IrPrinter {
    pub fn print_module(module: &IrModule) -> String {
        let mut out = String::new();
        out.push_str(&format!("module \"{}\"\n\n", module.name));

        for glob in &module.globals {
            let vis = if glob.is_public { "public" } else { "private" };
            out.push_str(&format!("global @{} : {:?} ({})\n", glob.name, glob.vb_type, vis));
        }
        if !module.globals.is_empty() {
            out.push('\n');
        }

        for func in &module.functions {
            out.push_str(&Self::print_function(func));
            out.push('\n');
        }

        out
    }

    pub fn print_function(func: &IrFunction) -> String {
        let mut out = String::new();
        let ret_str = if let Some(ref ret) = func.return_type {
            format!(" -> {:?}", ret)
        } else {
            String::new()
        };

        let params_str: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                let mut prefix = String::new();
                if p.is_optional {
                    prefix.push_str("Optional ");
                }
                if p.is_by_val {
                    prefix.push_str("ByVal ");
                } else {
                    prefix.push_str("ByRef ");
                }
                if p.is_param_array {
                    prefix.push_str("ParamArray ");
                }
                format!("{}{}: {:?}", prefix, p.name, p.vb_type)
            })
            .collect();

        out.push_str(&format!(
            "function {}({}){}\n",
            func.name,
            params_str.join(", "),
            ret_str
        ));

        for block in &func.blocks {
            out.push_str(&format!("{}:\n", block.label));
            for inst in &block.instructions {
                out.push_str(&format!("  {}\n", Self::print_instruction(inst)));
            }
        }

        out
    }

    pub fn print_instruction(inst: &IrInstruction) -> String {
        match inst {
            IrInstruction::AllocLocal { dest, name, vb_type } => {
                format!("%{} = ALLOC_LOCAL \"{}\" : {:?}", dest, name, vb_type)
            }
            IrInstruction::Load { dest, src } => {
                format!("%{} = LOAD {}", dest, Self::print_operand(src))
            }
            IrInstruction::Store { dest, src } => {
                format!("STORE {}, {}", Self::print_operand(dest), Self::print_operand(src))
            }
            IrInstruction::BinaryOp { dest, op, left, right, result_type } => {
                format!(
                    "%{} = {:?} {}, {} : {:?}",
                    dest,
                    op,
                    Self::print_operand(left),
                    Self::print_operand(right),
                    result_type
                )
            }
            IrInstruction::UnaryOp { dest, op, operand, result_type } => {
                format!(
                    "%{} = {:?} {} : {:?}",
                    dest,
                    op,
                    Self::print_operand(operand),
                    result_type
                )
            }
            IrInstruction::Compare { dest, op, left, right } => {
                format!(
                    "%{} = COMPARE_{:?} {}, {}",
                    dest,
                    op,
                    Self::print_operand(left),
                    Self::print_operand(right)
                )
            }
            IrInstruction::Branch { target } => {
                format!("BRANCH \"{}\"", target)
            }
            IrInstruction::BranchIf { condition, true_target, false_target } => {
                format!(
                    "BRANCH_IF {}, \"{}\", \"{}\"",
                    Self::print_operand(condition),
                    true_target,
                    false_target
                )
            }
            IrInstruction::Call { dest, name, args, return_type } => {
                let args_str: Vec<String> = args.iter().map(Self::print_operand).collect();
                let ret_str = return_type.as_ref().map(|t| format!(" : {:?}", t)).unwrap_or_default();
                if let Some(d) = dest {
                    format!("%{} = CALL \"{}\" [{}]{}", d, name, args_str.join(", "), ret_str)
                } else {
                    format!("CALL \"{}\" [{}]", name, args_str.join(", "))
                }
            }
            IrInstruction::Return { value } => {
                if let Some(v) = value {
                    format!("RETURN {}", Self::print_operand(v))
                } else {
                    "RETURN".to_string()
                }
            }
            IrInstruction::StringConcat { dest, left, right } => {
                format!(
                    "%{} = STRING_CONCAT {}, {}",
                    dest,
                    Self::print_operand(left),
                    Self::print_operand(right)
                )
            }
            IrInstruction::VariantConvert { dest, src, target_type } => {
                format!(
                    "%{} = VARIANT_CONVERT {}, {:?}",
                    dest,
                    Self::print_operand(src),
                    target_type
                )
            }
            IrInstruction::Coerce {
                dest,
                src,
                from_type,
                to_type,
            } => format!(
                "%{} = COERCE {}, {:?} -> {:?}",
                dest,
                Self::print_operand(src),
                from_type,
                to_type
            ),
            IrInstruction::PushErrorHandler { target } => {
                format!("PUSH_ERROR_HANDLER \"{}\"", target)
            }
            IrInstruction::PopErrorHandler => "POP_ERROR_HANDLER".to_string(),
            IrInstruction::Resume { target } => {
                if let Some(t) = target {
                    format!("RESUME \"{}\"", t)
                } else {
                    "RESUME".to_string()
                }
            }
            IrInstruction::DebugPrint { args } => {
                let args_str: Vec<String> = args.iter().map(Self::print_operand).collect();
                format!("DEBUG_PRINT [{}]", args_str.join(", "))
            }
            IrInstruction::ArrayCreate { dest, dims, bounds, elem_type } => {
                let bounds_str: Vec<String> = bounds
                    .iter()
                    .map(|(lb, ub)| format!("({} To {})", Self::print_operand(lb), Self::print_operand(ub)))
                    .collect();
                format!(
                    "%{} = ARRAY_CREATE dims={}, bounds=[{}], elem_type={:?}",
                    dest, dims, bounds_str.join(", "), elem_type
                )
            }
            IrInstruction::ArrayElementPtr { dest, array, indices, elem_type } => {
                let idx_str: Vec<String> = indices.iter().map(Self::print_operand).collect();
                format!(
                    "%{} = ARRAY_ELEM_PTR {}, indices=[{}], elem_type={:?}",
                    dest, Self::print_operand(array), idx_str.join(", "), elem_type
                )
            }
            IrInstruction::ArrayGetElement { dest, array, indices, elem_type } => {
                let idx_str: Vec<String> = indices.iter().map(Self::print_operand).collect();
                format!(
                    "%{} = ARRAY_GET_ELEM {}, indices=[{}], elem_type={:?}",
                    dest, Self::print_operand(array), idx_str.join(", "), elem_type
                )
            }
            IrInstruction::ArraySetElement { array, indices, value, elem_type } => {
                let idx_str: Vec<String> = indices.iter().map(Self::print_operand).collect();
                format!(
                    "ARRAY_SET_ELEM {}, indices=[{}], val={}, elem_type={:?}",
                    Self::print_operand(array), idx_str.join(", "), Self::print_operand(value), elem_type
                )
            }
            IrInstruction::ArrayReDim { dest, array, dims, bounds, elem_type, preserve } => {
                let bounds_str: Vec<String> = bounds
                    .iter()
                    .map(|(lb, ub)| format!("({} To {})", Self::print_operand(lb), Self::print_operand(ub)))
                    .collect();
                format!(
                    "%{} = ARRAY_REDIM {}, dims={}, bounds=[{}], elem_type={:?}, preserve={}",
                    dest, Self::print_operand(array), dims, bounds_str.join(", "), elem_type, preserve
                )
            }
            IrInstruction::ArrayBound { dest, array, dim, is_ubound } => {
                let name = if *is_ubound { "UBOUND" } else { "LBOUND" };
                format!(
                    "%{} = {} {}, dim={}",
                    dest, name, Self::print_operand(array), Self::print_operand(dim)
                )
            }
        }
    }

    pub fn print_operand(op: &Operand) -> String {
        match op {
            Operand::Temp(id) => format!("%{}", id),
            Operand::Local(name) => format!("Local(\"{}\")", name),
            Operand::Global(name) => format!("Global(\"{}\")", name),
            Operand::Constant(c) => match c {
                IrConstant::Int(v, ty) => format!("ConstInt({}, {:?})", v, ty),
                IrConstant::Float(v) => format!("ConstFloat({})", v),
                IrConstant::String(s) => format!("ConstString(\"{}\")", s),
                IrConstant::Bool(b) => format!("ConstBool({})", b),
                IrConstant::Date(d) => format!("ConstDate(#{}#)", d),
                IrConstant::Null => "Null".to_string(),
                IrConstant::Empty => "Empty".to_string(),
                IrConstant::Nothing => "Nothing".to_string(),
            },
        }
    }
}
