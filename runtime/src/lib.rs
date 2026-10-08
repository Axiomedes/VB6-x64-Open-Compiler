//! VB6 Open Runtime x64 library.
//! Clean-room implementation reproducing observable VB6 runtime behavior.

pub mod arrays;
pub mod com;
pub mod controls;
pub mod core;
pub mod dates;
pub mod errors;
pub mod events;
pub mod forms;
pub mod io;
pub mod math;
pub mod strings;
pub mod variants;

pub use core::*;

/// Returns the embedded C source code for the standalone static runtime.
pub fn get_runtime_c_source() -> &'static str {
    include_str!("../c/vb6_runtime.c")
}
