//! VB6 Intermediate Representation (IR).
//! Decoupled from LLVM, providing an explicit, typed three-address code representation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IrOpcode {
    Load,
    Store,
    Add,
    Sub,
    Mul,
    Div,
    Call,
    Return,
    Branch,
    Compare,
    Alloc,
    StringConcat,
    VariantConvert,
    ObjectInvoke,
    PropertyGet,
    PropertyLet,
    PropertySet,
    EventHandler,
    ErrorHandler,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrInstruction {
    pub opcode: IrOpcode,
    pub operands: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrModule {
    pub name: String,
    pub instructions: Vec<IrInstruction>,
}
