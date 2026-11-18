#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{c_char, c_int};
use std::ptr;

use crate::v_video::doom_error;
use crate::v_video::{V_CopyRect, V_DrawPatch};
use crate::w_wad::W_CacheLumpName;
use crate::z_zone::PU_STATIC;

const BG: c_int = 4;
const FG: c_int = 0;

unsafe extern "C" {
    static mut automapactive: c_int;
}

#[unsafe(no_mangle)]
pub static mut sttminus: *mut patch_t = ptr::null_mut();

// --- STlib_init ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_init() {
    unsafe {
        sttminus = W_CacheLumpName(b"STTMINUS\0".as_ptr() as *mut c_char, PU_STATIC) as *mut patch_t;
    }
}

// --- Number widgets ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_initNum(
    n: *mut st_number_t,
    x: c_int,
    y: c_int,
    pl: *mut *mut patch_t,
    num: *mut c_int,
    on: *mut c_int,
    width: c_int,
) {
    unsafe {
        (*n).x = x;
        (*n).y = y;
        (*n).oldnum = 0;
        (*n).width = width;
        (*n).num = num;
        (*n).on = on;
        (*n).p = pl;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_drawNum(n: *mut st_number_t, _refresh: c_int) {
    unsafe {
        let mut numdigits = (*n).width;
        let mut num = *(*n).num;

        let first_patch = *(*n).p;
        let w = (*first_patch).width as c_int;
        let h = (*first_patch).height as c_int;
        let st_y = ST_Y as c_int;

        (*n).oldnum = num;

        let neg = num < 0;
        if neg {
            if numdigits == 2 && num < -9 {
                num = -9;
            } else if numdigits == 3 && num < -99 {
                num = -99;
            }
            num = -num;
        }

        let clear_x = (*n).x - numdigits * w;
        if (*n).y - st_y < 0 {
            doom_error("drawNum: n->y - ST_Y < 0");
        }

        V_CopyRect(
            clear_x,
            (*n).y - st_y,
            BG,
            w * numdigits,
            h,
            clear_x,
            (*n).y,
            FG,
        );

        if num == 1994 {
            return;
        }

        let mut x = (*n).x;

        if num == 0 {
            V_DrawPatch(x - w, (*n).y, FG, first_patch as *mut _);
        }

        while num != 0 && numdigits > 0 {
            numdigits -= 1;
            x -= w;
            let digit_patch = *(*n).p.add((num % 10) as usize);
            V_DrawPatch(x, (*n).y, FG, digit_patch as *mut _);
            num /= 10;
        }

        if neg {
            V_DrawPatch(x - 8, (*n).y, FG, sttminus as *mut _);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updateNum(n: *mut st_number_t, refresh: c_int) {
    unsafe {
        if *(*n).on != 0 {
            STlib_drawNum(n, refresh);
        }
    }
}

// --- Percent widgets ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_initPercent(
    p: *mut st_percent_t,
    x: c_int,
    y: c_int,
    pl: *mut *mut patch_t,
    num: *mut c_int,
    on: *mut c_int,
    percent: *mut patch_t,
) {
    unsafe {
        STlib_initNum(&raw mut (*p).n, x, y, pl, num, on, 3);
        (*p).p = percent;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updatePercent(per: *mut st_percent_t, refresh: c_int) {
    unsafe {
        if refresh != 0 && *(*per).n.on != 0 {
            V_DrawPatch((*per).n.x, (*per).n.y, FG, (*per).p as *mut _);
        }
        STlib_updateNum(&raw mut (*per).n, refresh);
    }
}

// --- Multi-icon widgets ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_initMultIcon(
    i: *mut st_multicon_t,
    x: c_int,
    y: c_int,
    il: *mut *mut patch_t,
    inum: *mut c_int,
    on: *mut c_int,
) {
    unsafe {
        (*i).x = x;
        (*i).y = y;
        (*i).oldinum = -1;
        (*i).inum = inum;
        (*i).on = on;
        (*i).p = il;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updateMultIcon(mi: *mut st_multicon_t, refresh: c_int) {
    unsafe {
        let cur_inum = *(*mi).inum;
        if *(*mi).on != 0 && ((*mi).oldinum != cur_inum || refresh != 0) && cur_inum != -1 {
            let st_y = ST_Y as c_int;
            if (*mi).oldinum != -1 {
                let old_patch = *(*mi).p.add((*mi).oldinum as usize);
                let x = (*mi).x - ((*old_patch).leftoffset as c_int);
                let y = (*mi).y - ((*old_patch).topoffset as c_int);
                let w = (*old_patch).width as c_int;
                let h = (*old_patch).height as c_int;

                if y - st_y < 0 {
                    doom_error("updateMultIcon: y - ST_Y < 0");
                }

                V_CopyRect(x, y - st_y, BG, w, h, x, y, FG);
            }
            let new_patch = *(*mi).p.add(cur_inum as usize);
            V_DrawPatch((*mi).x, (*mi).y, FG, new_patch as *mut _);
            (*mi).oldinum = cur_inum;
        }
    }
}

// --- Binary icon widgets ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_initBinIcon(
    b: *mut st_binicon_t,
    x: c_int,
    y: c_int,
    i: *mut patch_t,
    val: *mut c_int,
    on: *mut c_int,
) {
    unsafe {
        (*b).x = x;
        (*b).y = y;
        (*b).oldval = 0;
        (*b).val = val;
        (*b).on = on;
        (*b).p = i;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updateBinIcon(bi: *mut st_binicon_t, refresh: c_int) {
    unsafe {
        let cur_val = *(*bi).val;
        if *(*bi).on != 0 && ((*bi).oldval != cur_val || refresh != 0) {
            let st_y = ST_Y as c_int;
            let patch = (*bi).p;
            let x = (*bi).x - ((*patch).leftoffset as c_int);
            let y = (*bi).y - ((*patch).topoffset as c_int);
            let w = (*patch).width as c_int;
            let h = (*patch).height as c_int;

            if y - st_y < 0 {
                doom_error("updateBinIcon: y - ST_Y < 0");
            }

            if cur_val != 0 {
                V_DrawPatch((*bi).x, (*bi).y, FG, patch as *mut _);
            } else {
                V_CopyRect(x, y - st_y, BG, w, h, x, y, FG);
            }

            (*bi).oldval = cur_val;
        }
    }
}
