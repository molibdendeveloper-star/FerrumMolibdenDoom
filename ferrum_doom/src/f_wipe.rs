#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

use std::ffi::{c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::m_random::m_random;
use crate::v_video::{V_DrawBlock, V_MarkRect, screens};
use crate::z_zone::{PU_STATIC, Z_Free, Z_Malloc};

unsafe extern "C" {
    fn I_ReadScreen(scr: *mut u8);
}

static mut GO: c_int = 0;
static mut WIPE_SCR_START: *mut u8 = ptr::null_mut();
static mut WIPE_SCR_END: *mut u8 = ptr::null_mut();
static mut WIPE_SCR: *mut u8 = ptr::null_mut();
static mut WIPE_Y: *mut c_int = ptr::null_mut();

// --- wipe_shittyColMajorXform ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_shittyColMajorXform(array: *mut c_short, width: c_int, height: c_int) {
    unsafe {
        let w = width as usize;
        let h = height as usize;
        let total = w * h;
        let dest = Z_Malloc((total * 2) as c_int, PU_STATIC, ptr::null_mut()) as *mut c_short;

        for y in 0..h {
            for x in 0..w {
                *dest.add(x * h + y) = *array.add(y * w + x);
            }
        }

        ptr::copy_nonoverlapping(dest, array, total);
        Z_Free(dest as *mut c_void);
    }
}

// --- ColorXForm ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_initColorXForm(width: c_int, height: c_int, _ticks: c_int) -> c_int {
    unsafe {
        ptr::copy_nonoverlapping(WIPE_SCR_START, WIPE_SCR, (width * height) as usize);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_doColorXForm(width: c_int, height: c_int, ticks: c_int) -> c_int {
    unsafe {
        let mut changed = false;
        let mut w = WIPE_SCR;
        let mut e = WIPE_SCR_END;
        let end = WIPE_SCR.add((width * height) as usize);

        while w != end {
            let wv = *w as c_int;
            let ev = *e as c_int;
            if wv != ev {
                if wv > ev {
                    let mut newval = wv - ticks;
                    if newval < ev {
                        newval = ev;
                    }
                    *w = newval as u8;
                    changed = true;
                } else {
                    let mut newval = wv + ticks;
                    if newval > ev {
                        newval = ev;
                    }
                    *w = newval as u8;
                    changed = true;
                }
            }
            w = w.add(1);
            e = e.add(1);
        }

        (!changed) as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wipe_exitColorXForm(_width: c_int, _height: c_int, _ticks: c_int) -> c_int {
    0
}

// --- Melt ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_initMelt(width: c_int, height: c_int, _ticks: c_int) -> c_int {
    unsafe {
        let w = width as usize;
        let h = height as usize;
        ptr::copy_nonoverlapping(WIPE_SCR_START, WIPE_SCR, w * h);

        wipe_shittyColMajorXform(WIPE_SCR_START as *mut c_short, width / 2, height);
        wipe_shittyColMajorXform(WIPE_SCR_END as *mut c_short, width / 2, height);

        WIPE_Y = Z_Malloc((w * size_of::<c_int>()) as c_int, PU_STATIC, ptr::null_mut())
            as *mut c_int;

        *WIPE_Y = -(m_random() % 16);
        for i in 1..w {
            let r = (m_random() % 3) - 1;
            let mut val = *WIPE_Y.add(i - 1) + r;
            if val > 0 {
                val = 0;
            } else if val == -16 {
                val = -15;
            }
            *WIPE_Y.add(i) = val;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_doMelt(width: c_int, height: c_int, mut ticks: c_int) -> c_int {
    unsafe {
        let mut done: c_int = 1;
        let half_w = (width / 2) as usize;
        let h = height as usize;

        let scr_start = WIPE_SCR_START as *const c_short;
        let scr_end = WIPE_SCR_END as *const c_short;
        let scr = WIPE_SCR as *mut c_short;

        while ticks > 0 {
            ticks -= 1;
            for i in 0..half_w {
                let yi = *WIPE_Y.add(i);
                if yi < 0 {
                    *WIPE_Y.add(i) = yi + 1;
                    done = 0;
                } else if yi < height {
                    let mut dy = if yi < 16 { yi + 1 } else { 8 };
                    if yi + dy >= height {
                        dy = height - yi;
                    }

                    let yi_u = yi as usize;
                    let mut s = scr_end.add(i * h + yi_u);
                    let mut d = scr.add(yi_u * half_w + i);
                    let mut idx = 0usize;
                    for _ in 0..dy {
                        *d.add(idx) = *s;
                        s = s.add(1);
                        idx += half_w;
                    }

                    let new_yi = yi + dy;
                    *WIPE_Y.add(i) = new_yi;

                    let new_yi_u = new_yi as usize;
                    s = scr_start.add(i * h);
                    d = scr.add(new_yi_u * half_w + i);
                    idx = 0;
                    for _ in 0..(height - new_yi) {
                        *d.add(idx) = *s;
                        s = s.add(1);
                        idx += half_w;
                    }

                    done = 0;
                }
            }
        }

        done
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_exitMelt(_width: c_int, _height: c_int, _ticks: c_int) -> c_int {
    unsafe {
        Z_Free(WIPE_Y as *mut c_void);
        0
    }
}

// --- Public ScreenWipe API ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_StartScreen(
    _x: c_int,
    _y: c_int,
    _width: c_int,
    _height: c_int,
) -> c_int {
    unsafe {
        let scr_ptr = (&raw mut screens) as *mut *mut u8;
        WIPE_SCR_START = *scr_ptr.add(2);
        I_ReadScreen(WIPE_SCR_START);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_EndScreen(x: c_int, y: c_int, width: c_int, height: c_int) -> c_int {
    unsafe {
        let scr_ptr = (&raw mut screens) as *mut *mut u8;
        WIPE_SCR_END = *scr_ptr.add(3);
        I_ReadScreen(WIPE_SCR_END);
        V_DrawBlock(x, y, 0, width, height, WIPE_SCR_START);
        0
    }
}

type WipeFn = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

static WIPES: [WipeFn; 6] = [
    wipe_initColorXForm,
    wipe_doColorXForm,
    wipe_exitColorXForm,
    wipe_initMelt,
    wipe_doMelt,
    wipe_exitMelt,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_ScreenWipe(
    wipeno: c_int,
    _x: c_int,
    _y: c_int,
    width: c_int,
    height: c_int,
    ticks: c_int,
) -> c_int {
    unsafe {
        let base = (wipeno * 3) as usize;
        if GO == 0 {
            GO = 1;
            let scr_ptr = (&raw mut screens) as *mut *mut u8;
            WIPE_SCR = *scr_ptr;
            (WIPES[base])(width, height, ticks);
        }

        V_MarkRect(0, 0, width, height);
        let rc = (WIPES[base + 1])(width, height, ticks);

        if rc != 0 {
            GO = 0;
            (WIPES[base + 2])(width, height, ticks);
        }

        (GO == 0) as c_int
    }
}
