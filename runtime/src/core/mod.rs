//! Core runtime lifecycle and process initialization for VB6 applications.

#[link(name = "user32")]
#[link(name = "ole32")]
#[link(name = "kernel32")]
extern "system" {
    fn MessageBoxA(
        hwnd: *mut std::ffi::c_void,
        text: *const u8,
        caption: *const u8,
        utype: u32,
    ) -> i32;

    fn CoInitializeEx(pv_reserved: *mut std::ffi::c_void, dw_co_init: u32) -> i32;
    fn CoUninitialize();
    fn OutputDebugStringA(lp_output_string: *const u8);
}

#[no_mangle]
pub extern "C" fn vb6_init() {
    unsafe {
        // COINIT_APARTMENTTHREADED = 0x2
        CoInitializeEx(std::ptr::null_mut(), 0x2);
    }
}

#[no_mangle]
pub extern "C" fn vb6_shutdown() {
    unsafe {
        CoUninitialize();
    }
}

#[no_mangle]
pub extern "C" fn vb6_msgbox(prompt: *const u8, title: *const u8, buttons: u32) -> i32 {
    let default_title = b"VB6 Application\0".as_ptr();
    let default_prompt = b"\0".as_ptr();

    let actual_title = if title.is_null() {
        default_title
    } else {
        title
    };

    let actual_prompt = if prompt.is_null() {
        default_prompt
    } else {
        prompt
    };

    unsafe {
        MessageBoxA(std::ptr::null_mut(), actual_prompt, actual_title, buttons)
    }
}

#[no_mangle]
pub extern "C" fn vb6_debug_print(msg: *const u8) {
    if !msg.is_null() {
        unsafe {
            OutputDebugStringA(msg);
            let c_str = std::ffi::CStr::from_ptr(msg as *const i8);
            if let Ok(str_slice) = c_str.to_str() {
                println!("{}", str_slice);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn vb6_runtime_exit(code: i32) -> ! {
    std::process::exit(code);
}
