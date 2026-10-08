//! VB6 Intermediate Representation (IR).
//! Decoupled from LLVM, providing an explicit, typed three-address code representation.

pub mod lowering;
pub mod model;
pub mod printer;

pub use lowering::AstToIrLowering;
pub use model::*;
pub use printer::IrPrinter;

use crate::ast::SourceFile;

/// Compile a parsed SourceFile into a typed VB6 IrModule.
pub fn compile_to_ir(source: &SourceFile, module_name: &str) -> IrModule {
    let mut lowering = AstToIrLowering::new();
    lowering.lower_source_file(source, module_name)
}
