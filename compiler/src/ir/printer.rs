//! Textual formatter and printer for VB6 Intermediate Representation (IR).

use crate::ir::model::*;

pub struct IrPrinter;

impl IrPrinter {
    pub fn print_module(module: &IrModule) -> String {
        let mut out = String::new();
        out.push_str(&format!("module \"{}\"\n\n", module.name));

        for (glob_name, glob_ty) in &module.globals {
            out.push_str(&format!("global @{} : {:?}\n", glob_name, glob_ty));
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
            .map(|(name, ty)| format!("{}: {:?}", name, ty))
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
            IrInstruction::Call { dest, name, args } => {
                let args_str: Vec<String> = args.iter().map(Self::print_operand).collect();
                if let Some(d) = dest {
                    format!("%{} = CALL \"{}\" [{}]", d, name, args_str.join(", "))
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
        }
    }

    pub fn print_operand(op: &Operand) -> String {
        match op {
            Operand::Temp(id) => format!("%{}", id),
            Operand::Local(name) => format!("Local(\"{}\")", name),
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
