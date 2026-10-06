//! Core runtime lifecycle and process initialization for VB6 applications.

#[no_mangle]
pub extern "C" fn vb6_runtime_init() {
    // Process initialization
}

#[no_mangle]
pub extern "C" fn vb6_runtime_exit(code: i32) -> ! {
    std::process::exit(code);
}
