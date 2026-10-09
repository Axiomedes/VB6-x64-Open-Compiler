//! Data structures and instructions for VB6 Intermediate Representation (IR).

use crate::types::VbType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrGlobal {
    pub name: String,
    pub vb_type: VbType,
    pub is_public: bool,
    pub initial_value: Option<IrConstant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrModule {
    pub name: String,
    pub globals: Vec<IrGlobal>,
    pub functions: Vec<IrFunction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrParam {
    pub name: String,
    pub vb_type: VbType,
    pub is_by_val: bool,
    pub is_optional: bool,
    pub is_param_array: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<IrParam>,
    pub return_type: Option<VbType>,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<IrInstruction>,
}

impl BasicBlock {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            instructions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Operand {
    Temp(usize),
    Local(String),
    Global(String),
    Constant(IrConstant),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IrConstant {
    Int(i64, VbType),
    Float(f64),
    String(String),
    Bool(bool),
    Date(String),
    Null,
    Empty,
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Concat,
    BitAnd,
    BitOr,
    BitXor,
    BitEqv,
    BitImp,
    Exponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrUnaryOp {
    Negate,
    BitNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrCompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Is,
    Like,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IrInstruction {
    /// Allocate local variable on stack: `%dest = ALLOC_LOCAL "X", VbType::Long`
    AllocLocal {
        dest: usize,
        name: String,
        vb_type: VbType,
    },
    /// Load value from variable or address: `%dest = LOAD %src`
    Load {
        dest: usize,
        src: Operand,
    },
    /// Store value to variable or address: `STORE %dest, %src`
    Store {
        dest: Operand,
        src: Operand,
    },
    /// Binary arithmetic/logical operation: `%dest = ADD %left, %right : VbType`
    BinaryOp {
        dest: usize,
        op: IrBinaryOp,
        left: Operand,
        right: Operand,
        result_type: VbType,
    },
    /// Unary operation: `%dest = NEGATE %operand : VbType`
    UnaryOp {
        dest: usize,
        op: IrUnaryOp,
        operand: Operand,
        result_type: VbType,
    },
    /// Comparison operation: `%dest = COMPARE_GT %left, %right`
    Compare {
        dest: usize,
        op: IrCompareOp,
        left: Operand,
        right: Operand,
    },
    /// Unconditional jump: `BRANCH "label"`
    Branch {
        target: String,
    },
    /// Conditional jump: `BRANCH_IF %condition, "true_label", "false_label"`
    BranchIf {
        condition: Operand,
        true_target: String,
        false_target: String,
    },
    /// Subroutine or function call: `%dest = CALL "MyFunc", [%arg0, %arg1] : Type`
    Call {
        dest: Option<usize>,
        name: String,
        args: Vec<Operand>,
        return_type: Option<VbType>,
    },
    /// Return from procedure: `RETURN [%value]`
    Return {
        value: Option<Operand>,
    },
    /// String concatenation: `%dest = STRING_CONCAT %left, %right`
    StringConcat {
        dest: usize,
        left: Operand,
        right: Operand,
    },
    /// Variant type coercion: `%dest = VARIANT_CONVERT %src, target_type`
    VariantConvert {
        dest: usize,
        src: Operand,
        target_type: VbType,
    },
    /// General type coercion: `%dest = COERCE %src, from_type -> to_type`
    Coerce {
        dest: usize,
        src: Operand,
        from_type: VbType,
        to_type: VbType,
    },
    /// On Error GoTo handler registration
    PushErrorHandler {
        target: String,
    },
    /// Disable error handler (On Error GoTo 0)
    PopErrorHandler,
    /// Resume execution from error
    Resume {
        target: Option<String>,
    },
    /// Built-in Debug.Print output
    DebugPrint {
        args: Vec<Operand>,
    },
    /// Array creation: `%dest = ARRAY_CREATE dims, [(lower, upper)...], elem_type`
    ArrayCreate {
        dest: usize,
        dims: usize,
        bounds: Vec<(Operand, Operand)>,
        elem_type: VbType,
    },
    /// Array element pointer: `%dest = ARRAY_ELEM_PTR %array, [indices...], elem_type`
    ArrayElementPtr {
        dest: usize,
        array: Operand,
        indices: Vec<Operand>,
        elem_type: VbType,
    },
    /// Array get element: `%dest = ARRAY_GET_ELEM %array, [indices...], elem_type`
    ArrayGetElement {
        dest: usize,
        array: Operand,
        indices: Vec<Operand>,
        elem_type: VbType,
    },
    /// Array set element: `ARRAY_SET_ELEM %array, [indices...], %value, elem_type`
    ArraySetElement {
        array: Operand,
        indices: Vec<Operand>,
        value: Operand,
        elem_type: VbType,
    },
    /// Array re-dimension: `%dest = ARRAY_REDIM %array, dims, [(lower, upper)...], elem_type, preserve`
    ArrayReDim {
        dest: usize,
        array: Operand,
        dims: usize,
        bounds: Vec<(Operand, Operand)>,
        elem_type: VbType,
        preserve: bool,
    },
    /// Array bound (LBound / UBound): `%dest = ARRAY_BOUND %array, %dim, is_ubound`
    ArrayBound {
        dest: usize,
        array: Operand,
        dim: Operand,
        is_ubound: bool,
    },
}
