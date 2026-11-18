#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

use std::ffi::{c_char, c_int};

use crate::d_main::D_DoomMain;
use crate::m_argv::{myargc, myargv};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn doom_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe {
        myargc = argc;
        myargv = argv;
        D_DoomMain();
    }
    0
}
