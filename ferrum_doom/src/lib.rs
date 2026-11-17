use std::ffi::{CStr, c_char, c_int, c_ulong, c_ushort};

pub mod m_argv;
pub mod m_bbox;
pub mod m_fixed;
pub mod m_random;
pub mod m_swap;
pub mod tables;
pub mod z_zone;
pub mod w_wad;
pub mod m_misc;
pub mod info;
pub mod sounds;
pub mod v_video;
pub mod r_bsp;
pub mod r_plane;
pub mod r_segs;
pub mod r_sky;
pub mod r_draw;
pub mod r_things;
pub mod r_data;
pub mod r_main;
pub mod p_tick;
pub mod p_mobj;
pub mod p_user;
pub mod p_telept;
pub mod p_switch;
pub mod p_spec;
pub mod p_saveg;
pub mod p_sight;
pub mod p_setup;
pub mod p_pspr;
pub mod p_plats;
pub mod p_maputl;
pub mod p_map;
pub mod p_lights;

// --- m_fixed ---
#[unsafe(no_mangle)]
pub extern "C" fn FixedMul(a: i32, b: i32) -> i32 {
    m_fixed::fixed_mul(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn FixedDiv(a: i32, b: i32) -> i32 {
    m_fixed::fixed_div(a, b)
}

// --- m_random ---
#[unsafe(no_mangle)]
pub extern "C" fn P_Random() -> c_int {
    m_random::p_random()
}

#[unsafe(no_mangle)]
pub extern "C" fn M_Random() -> c_int {
    m_random::m_random()
}

#[unsafe(no_mangle)]
pub extern "C" fn M_ClearRandom() {
    m_random::m_clear_random();
}

// --- m_argv ---
#[unsafe(no_mangle)]
pub extern "C" fn M_CheckParm(check: *const c_char) -> c_int {
    if check.is_null() {
        return 0;
    }

    let c_str = unsafe { CStr::from_ptr(check) };
    match c_str.to_str() {
        Ok(s) => m_argv::m_check_parm(s),
        Err(_) => 0,
    }
}

// --- m_bbox ---
#[unsafe(no_mangle)]
pub extern "C" fn M_ClearBox(box_ptr: *mut i32) {
    if box_ptr.is_null() {
        return;
    }
    let box_arr = unsafe { &mut *(box_ptr as *mut [i32; 4]) };
    m_bbox::m_clear_box(box_arr);
}

#[unsafe(no_mangle)]
pub extern "C" fn M_AddToBox(box_ptr: *mut i32, x: i32, y: i32) {
    if box_ptr.is_null() {
        return;
    }
    let box_arr = unsafe { &mut *(box_ptr as *mut [i32; 4]) };
    m_bbox::m_add_to_box(box_arr, x, y);
}

// --- m_swap ---
#[unsafe(no_mangle)]
pub extern "C" fn SwapSHORT(x: c_ushort) -> c_ushort {
    m_swap::swap_short(x)
}

#[unsafe(no_mangle)]
pub extern "C" fn SwapLONG(x: c_ulong) -> c_ulong {
    m_swap::swap_long(x as u32) as c_ulong
}
