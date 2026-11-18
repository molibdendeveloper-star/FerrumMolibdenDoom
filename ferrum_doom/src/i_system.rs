#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use crate::d_net::D_QuitNetGame;
use crate::g_game::{G_CheckDemoStatus, demorecording};
use crate::i_sound::{I_InitMusic, I_InitSound, I_ShutdownMusic, I_ShutdownSound};
use crate::i_video::I_ShutdownGraphics;
use crate::m_misc::M_SaveDefaults;

#[unsafe(no_mangle)]
pub static mut mb_used: c_int = 32;

#[unsafe(no_mangle)]
pub static mut emptycmd: ticcmd_t = ticcmd_t {
    forwardmove: 0,
    sidemove: 0,
    angleturn: 0,
    consistancy: 0,
    chatchar: 0,
    buttons: 0,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_Tactile(_on: c_int, _off: c_int, _total: c_int) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_BaseTiccmd() -> *mut ticcmd_t {
    &raw mut emptycmd
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_GetHeapSize() -> c_int {
    32 * 1024 * 1024
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ZoneBase(size: *mut c_int) -> *mut u8 {
    unsafe {
        let sz = 32 * 1024 * 1024;
        *size = sz;
        let heap = Box::leak(vec![0u8; sz as usize].into_boxed_slice());
        heap.as_mut_ptr()
    }
}

static START_TIME: OnceLock<Instant> = OnceLock::new();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_GetTime() -> c_int {
    let start = START_TIME.get_or_init(Instant::now);
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs() as i64;
    let micros = elapsed.subsec_micros() as i64;
    let tic_rate: i64 = 35;
    ((secs * tic_rate) + (micros * tic_rate) / 1_000_000) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_Init() {
    unsafe {
        I_InitSound();
        I_InitMusic();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_Quit() {
    unsafe {
        D_QuitNetGame();
        I_ShutdownSound();
        I_ShutdownMusic();
        M_SaveDefaults();
        I_ShutdownGraphics();
        std::process::exit(0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_WaitVBL(count: c_int) {
    if count > 0 {
        let us = (count as u64) * (1_000_000 / 70);
        thread::sleep(Duration::from_micros(us));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_BeginRead() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_EndRead() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_AllocLow(length: c_int) -> *mut u8 {
    if length <= 0 {
        return ptr::null_mut();
    }
    let buf = Box::leak(vec![0u8; length as usize].into_boxed_slice());
    buf.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_Error_rs(msg: *const c_char) -> ! {
    unsafe {
        let text = if msg.is_null() {
            "Unknown error".into()
        } else {
            CStr::from_ptr(msg).to_string_lossy()
        };
        eprintln!("Error: {}", text);

        if demorecording != 0 {
            G_CheckDemoStatus();
        }

        D_QuitNetGame();
        I_ShutdownGraphics();
        std::process::exit(-1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_Error(fmt: *const c_char, arg: *const c_char) -> ! {
    unsafe {
        let fmt_str = if fmt.is_null() {
            ""
        } else {
            CStr::from_ptr(fmt).to_str().unwrap_or("")
        };
        if fmt_str == "%s" && !arg.is_null() {
            I_Error_rs(arg);
        } else {
            I_Error_rs(fmt);
        }
    }
}
