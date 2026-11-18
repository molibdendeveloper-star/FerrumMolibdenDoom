#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{c_char, c_int, c_uchar};

use crate::r_draw::{R_VideoErase, viewheight, viewwidth, viewwindowx, viewwindowy};
use crate::v_video::V_DrawPatchDirect;

const FG: c_int = 0;

unsafe extern "C" {
    static mut automapactive: c_int;
}

// --- HUlib_init ---
#[unsafe(no_mangle)]
pub extern "C" fn HUlib_init() {}

// --- HUlib_clearTextLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_clearTextLine(t: *mut hu_textline_t) {
    unsafe {
        (*t).len = 0;
        (*t).l[0] = 0;
        (*t).needsupdate = 1;
    }
}

// --- HUlib_initTextLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_initTextLine(
    t: *mut hu_textline_t,
    x: c_int,
    y: c_int,
    f: *mut *mut patch_t,
    sc: c_int,
) {
    unsafe {
        (*t).x = x;
        (*t).y = y;
        (*t).f = f;
        (*t).sc = sc;
        HUlib_clearTextLine(t);
    }
}

// --- HUlib_addCharToTextLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addCharToTextLine(t: *mut hu_textline_t, ch: c_char) -> c_int {
    unsafe {
        let max_len = HU_MAXLINELENGTH as c_int;
        if (*t).len == max_len {
            0
        } else {
            let idx = (*t).len as usize;
            (*t).l[idx] = ch;
            (*t).len += 1;
            (*t).l[(*t).len as usize] = 0;
            (*t).needsupdate = 4;
            1
        }
    }
}

// --- HUlib_delCharFromTextLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_delCharFromTextLine(t: *mut hu_textline_t) -> c_int {
    unsafe {
        if (*t).len == 0 {
            0
        } else {
            (*t).len -= 1;
            (*t).l[(*t).len as usize] = 0;
            (*t).needsupdate = 4;
            1
        }
    }
}

// --- HUlib_drawTextLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_drawTextLine(l: *mut hu_textline_t, drawcursor: c_int) {
    unsafe {
        let mut x = (*l).x;
        let len = (*l).len as usize;
        let sc = (*l).sc;
        let font = (*l).f;
        let scr_w = SCREENWIDTH as c_int;

        for i in 0..len {
            let c = ((*l).l[i] as u8).to_ascii_uppercase() as c_int;
            if c != (b' ' as c_int) && c >= sc && c <= (b'_' as c_int) {
                let patch = *font.add((c - sc) as usize);
                let w = (*patch).width as c_int;
                if x + w > scr_w {
                    break;
                }
                V_DrawPatchDirect(x, (*l).y, FG, patch as *mut _);
                x += w;
            } else {
                x += 4;
                if x >= scr_w {
                    break;
                }
            }
        }

        if drawcursor != 0 {
            let cursor_idx = ((b'_' as c_int) - sc) as usize;
            let cursor_patch = *font.add(cursor_idx);
            let cw = (*cursor_patch).width as c_int;
            if x + cw <= scr_w {
                V_DrawPatchDirect(x, (*l).y, FG, cursor_patch as *mut _);
            }
        }
    }
}

// --- HUlib_eraseTextLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseTextLine(l: *mut hu_textline_t) {
    unsafe {
        let scr_w = SCREENWIDTH as c_int;
        if automapactive == 0 && viewwindowx != 0 && (*l).needsupdate != 0 {
            let first_patch = *(*l).f;
            let lh = ((*first_patch).height as c_int) + 1;
            let start_y = (*l).y;
            let mut yoffset = start_y * scr_w;

            for y in start_y..(start_y + lh) {
                if y < viewwindowy || y >= viewwindowy + viewheight {
                    R_VideoErase(yoffset as u32, scr_w);
                } else {
                    R_VideoErase(yoffset as u32, viewwindowx);
                    R_VideoErase((yoffset + viewwindowx + viewwidth) as u32, viewwindowx);
                }
                yoffset += scr_w;
            }
        }

        if (*l).needsupdate != 0 {
            (*l).needsupdate -= 1;
        }
    }
}

// --- HUlib_initSText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_initSText(
    s: *mut hu_stext_t,
    x: c_int,
    y: c_int,
    h: c_int,
    font: *mut *mut patch_t,
    startchar: c_int,
    on: *mut c_int,
) {
    unsafe {
        (*s).h = h;
        (*s).on = on;
        (*s).laston = 1;
        (*s).cl = 0;

        let first_patch = *font;
        let line_step = ((*first_patch).height as c_int) + 1;

        for i in 0..(h as usize) {
            HUlib_initTextLine(
                &raw mut (*s).l[i],
                x,
                y - (i as c_int) * line_step,
                font,
                startchar,
            );
        }
    }
}

// --- HUlib_addLineToSText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addLineToSText(s: *mut hu_stext_t) {
    unsafe {
        (*s).cl += 1;
        if (*s).cl == (*s).h {
            (*s).cl = 0;
        }
        let cl = (*s).cl as usize;
        HUlib_clearTextLine(&raw mut (*s).l[cl]);

        for i in 0..((*s).h as usize) {
            (*s).l[i].needsupdate = 4;
        }
    }
}

// --- HUlib_addMessageToSText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addMessageToSText(
    s: *mut hu_stext_t,
    mut prefix: *mut c_char,
    mut msg: *mut c_char,
) {
    unsafe {
        HUlib_addLineToSText(s);
        let cl = (*s).cl as usize;
        let line_ptr = &raw mut (*s).l[cl];

        if !prefix.is_null() {
            while *prefix != 0 {
                HUlib_addCharToTextLine(line_ptr, *prefix);
                prefix = prefix.add(1);
            }
        }

        if !msg.is_null() {
            while *msg != 0 {
                HUlib_addCharToTextLine(line_ptr, *msg);
                msg = msg.add(1);
            }
        }
    }
}

// --- HUlib_drawSText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_drawSText(s: *mut hu_stext_t) {
    unsafe {
        if *(*s).on == 0 {
            return;
        }

        let h = (*s).h;
        for i in 0..h {
            let mut idx = (*s).cl - i;
            if idx < 0 {
                idx += h;
            }
            HUlib_drawTextLine(&raw mut (*s).l[idx as usize], 0);
        }
    }
}

// --- HUlib_eraseSText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseSText(s: *mut hu_stext_t) {
    unsafe {
        let on_val = *(*s).on;
        for i in 0..((*s).h as usize) {
            if (*s).laston != 0 && on_val == 0 {
                (*s).l[i].needsupdate = 4;
            }
            HUlib_eraseTextLine(&raw mut (*s).l[i]);
        }
        (*s).laston = on_val;
    }
}

// --- HUlib_initIText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_initIText(
    it: *mut hu_itext_t,
    x: c_int,
    y: c_int,
    font: *mut *mut patch_t,
    startchar: c_int,
    on: *mut c_int,
) {
    unsafe {
        (*it).lm = 0;
        (*it).on = on;
        (*it).laston = 1;
        HUlib_initTextLine(&raw mut (*it).l, x, y, font, startchar);
    }
}

// --- HUlib_delCharFromIText / erase / reset / prefix ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_delCharFromIText(it: *mut hu_itext_t) {
    unsafe {
        if (*it).l.len != (*it).lm {
            HUlib_delCharFromTextLine(&raw mut (*it).l);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseLineFromIText(it: *mut hu_itext_t) {
    unsafe {
        while (*it).lm != (*it).l.len {
            HUlib_delCharFromTextLine(&raw mut (*it).l);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_resetIText(it: *mut hu_itext_t) {
    unsafe {
        (*it).lm = 0;
        HUlib_clearTextLine(&raw mut (*it).l);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addPrefixToIText(it: *mut hu_itext_t, mut str: *mut c_char) {
    unsafe {
        while *str != 0 {
            HUlib_addCharToTextLine(&raw mut (*it).l, *str);
            str = str.add(1);
        }
        (*it).lm = (*it).l.len;
    }
}

// --- HUlib_keyInIText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_keyInIText(it: *mut hu_itext_t, ch: c_uchar) -> c_int {
    unsafe {
        if (b' '..=b'_').contains(&ch) {
            HUlib_addCharToTextLine(&raw mut (*it).l, ch as c_char);
        } else if (ch as u32) == KEY_BACKSPACE {
            HUlib_delCharFromIText(it);
        } else if (ch as u32) != KEY_ENTER {
            return 0;
        }
        1
    }
}

// --- HUlib_drawIText / HUlib_eraseIText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_drawIText(it: *mut hu_itext_t) {
    unsafe {
        if *(*it).on == 0 {
            return;
        }
        HUlib_drawTextLine(&raw mut (*it).l, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseIText(it: *mut hu_itext_t) {
    unsafe {
        let on_val = *(*it).on;
        if (*it).laston != 0 && on_val == 0 {
            (*it).l.needsupdate = 4;
        }
        HUlib_eraseTextLine(&raw mut (*it).l);
        (*it).laston = on_val;
    }
}
