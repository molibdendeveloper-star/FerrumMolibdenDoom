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

pub fn stlib_init_num(
    n: &mut st_number_t,
    x: c_int,
    y: c_int,
    pl: *mut *mut patch_t,
    num: *mut c_int,
    on: *mut c_int,
    width: c_int,
) {
    n.x = x;
    n.y = y;
    n.oldnum = 0;
    n.width = width;
    n.num = num;
    n.on = on;
    n.p = pl;
}

pub fn stlib_draw_num(n: &mut st_number_t, _refresh: bool) {
    if n.num.is_null() || n.p.is_null() {
        return;
    }

    let mut numdigits = n.width;
    let mut num = unsafe { *n.num };
    let patches = unsafe { std::slice::from_raw_parts(n.p, 10) };

    let Some(first_patch) = (unsafe { patches[0].as_ref() }) else { return };
    let w = first_patch.width as c_int;
    let h = first_patch.height as c_int;
    let st_y = ST_Y as c_int;

    n.oldnum = num;

    let neg = num < 0;
    if neg {
        if numdigits == 2 && num < -9 {
            num = -9;
        } else if numdigits == 3 && num < -99 {
            num = -99;
        }
        num = -num;
    }

    let clear_x = n.x - numdigits * w;
    if n.y - st_y < 0 {
        doom_error("drawNum: n->y - ST_Y < 0");
    }

    unsafe {
        V_CopyRect(clear_x, n.y - st_y, BG, w * numdigits, h, clear_x, n.y, FG);
    }

    if num == 1994 {
        return;
    }

    let mut x = n.x;
    if num == 0 {
        unsafe {
            V_DrawPatch(x - w, n.y, FG, patches[0] as *mut _);
        }
    }

    while num != 0 && numdigits > 0 {
        numdigits -= 1;
        x -= w;
        let digit_patch = patches[(num % 10) as usize];
        unsafe {
            V_DrawPatch(x, n.y, FG, digit_patch as *mut _);
        }
        num /= 10;
    }

    if neg {
        unsafe {
            V_DrawPatch(x - 8, n.y, FG, sttminus as *mut _);
        }
    }
}

pub fn stlib_update_num(n: &mut st_number_t, refresh: bool) {
    let is_on = unsafe { !n.on.is_null() && *n.on != 0 };
    if is_on {
        stlib_draw_num(n, refresh);
    }
}

pub fn stlib_init_percent(
    p: &mut st_percent_t,
    x: c_int,
    y: c_int,
    pl: *mut *mut patch_t,
    num: *mut c_int,
    on: *mut c_int,
    percent: *mut patch_t,
) {
    stlib_init_num(&mut p.n, x, y, pl, num, on, 3);
    p.p = percent;
}

pub fn stlib_update_percent(per: &mut st_percent_t, refresh: bool) {
    let is_on = unsafe { !per.n.on.is_null() && *per.n.on != 0 };
    if refresh && is_on {
        unsafe {
            V_DrawPatch(per.n.x, per.n.y, FG, per.p as *mut _);
        }
    }
    stlib_update_num(&mut per.n, refresh);
}

pub fn stlib_init_mult_icon(
    i: &mut st_multicon_t,
    x: c_int,
    y: c_int,
    il: *mut *mut patch_t,
    inum: *mut c_int,
    on: *mut c_int,
) {
    i.x = x;
    i.y = y;
    i.oldinum = -1;
    i.inum = inum;
    i.on = on;
    i.p = il;
}

pub fn stlib_update_mult_icon(mi: &mut st_multicon_t, refresh: bool) {
    if mi.inum.is_null() || mi.on.is_null() || mi.p.is_null() {
        return;
    }

    let (is_on, cur_inum) = unsafe { (*mi.on != 0, *mi.inum) };
    if is_on && (mi.oldinum != cur_inum || refresh) && cur_inum != -1 {
        let st_y = ST_Y as c_int;
        if mi.oldinum != -1 {
            let old_patch = unsafe { &*(*mi.p.add(mi.oldinum as usize)) };
            let x = mi.x - (old_patch.leftoffset as c_int);
            let y = mi.y - (old_patch.topoffset as c_int);
            let w = old_patch.width as c_int;
            let h = old_patch.height as c_int;

            if y - st_y < 0 {
                doom_error("updateMultIcon: y - ST_Y < 0");
            }

            unsafe {
                V_CopyRect(x, y - st_y, BG, w, h, x, y, FG);
            }
        }

        let new_patch = unsafe { *mi.p.add(cur_inum as usize) };
        unsafe {
            V_DrawPatch(mi.x, mi.y, FG, new_patch as *mut _);
        }
        mi.oldinum = cur_inum;
    }
}

pub fn stlib_init_bin_icon(
    b: &mut st_binicon_t,
    x: c_int,
    y: c_int,
    i: *mut patch_t,
    val: *mut c_int,
    on: *mut c_int,
) {
    b.x = x;
    b.y = y;
    b.oldval = 0;
    b.val = val;
    b.on = on;
    b.p = i;
}

pub fn stlib_update_bin_icon(bi: &mut st_binicon_t, refresh: bool) {
    if bi.val.is_null() || bi.on.is_null() || bi.p.is_null() {
        return;
    }

    let (is_on, cur_val) = unsafe { (*bi.on != 0, *bi.val) };
    if is_on && (bi.oldval != cur_val || refresh) {
        let st_y = ST_Y as c_int;
        let patch = unsafe { &*bi.p };
        let x = bi.x - (patch.leftoffset as c_int);
        let y = bi.y - (patch.topoffset as c_int);
        let w = patch.width as c_int;
        let h = patch.height as c_int;

        if y - st_y < 0 {
            doom_error("updateBinIcon: y - ST_Y < 0");
        }

        unsafe {
            if cur_val != 0 {
                V_DrawPatch(bi.x, bi.y, FG, bi.p as *mut _);
            } else {
                V_CopyRect(x, y - st_y, BG, w, h, x, y, FG);
            }
        }

        bi.oldval = cur_val;
    }
}

// --- Совместимые FFI-обёртки ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_init() {
    unsafe {
        sttminus = W_CacheLumpName(b"STTMINUS\0".as_ptr() as *mut c_char, PU_STATIC) as *mut patch_t;
    }
}

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
    if let Some(num_w) = unsafe { n.as_mut() } {
        stlib_init_num(num_w, x, y, pl, num, on, width);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_drawNum(n: *mut st_number_t, refresh: c_int) {
    if let Some(num_w) = unsafe { n.as_mut() } {
        stlib_draw_num(num_w, refresh != 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updateNum(n: *mut st_number_t, refresh: c_int) {
    if let Some(num_w) = unsafe { n.as_mut() } {
        stlib_update_num(num_w, refresh != 0);
    }
}

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
    if let Some(per_w) = unsafe { p.as_mut() } {
        stlib_init_percent(per_w, x, y, pl, num, on, percent);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updatePercent(per: *mut st_percent_t, refresh: c_int) {
    if let Some(per_w) = unsafe { per.as_mut() } {
        stlib_update_percent(per_w, refresh != 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_initMultIcon(
    i: *mut st_multicon_t,
    x: c_int,
    y: c_int,
    il: *mut *mut patch_t,
    inum: *mut c_int,
    on: *mut c_int,
) {
    if let Some(mi) = unsafe { i.as_mut() } {
        stlib_init_mult_icon(mi, x, y, il, inum, on);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updateMultIcon(mi: *mut st_multicon_t, refresh: c_int) {
    if let Some(item) = unsafe { mi.as_mut() } {
        stlib_update_mult_icon(item, refresh != 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_initBinIcon(
    b: *mut st_binicon_t,
    x: c_int,
    y: c_int,
    i: *mut patch_t,
    val: *mut c_int,
    on: *mut c_int,
) {
    if let Some(bi) = unsafe { b.as_mut() } {
        stlib_init_bin_icon(bi, x, y, i, val, on);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn STlib_updateBinIcon(bi: *mut st_binicon_t, refresh: c_int) {
    if let Some(item) = unsafe { bi.as_mut() } {
        stlib_update_bin_icon(item, refresh != 0);
    }
}
