#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, c_char, c_int, c_uchar};

use crate::r_draw::{R_VideoErase, viewheight, viewwidth, viewwindowx, viewwindowy};
use crate::v_video::V_DrawPatchDirect;

const FG: c_int = 0;

unsafe extern "C" {
    static mut automapactive: c_int;
}

#[inline]
pub fn hulib_clear_text_line(t: &mut hu_textline_t) {
    t.len = 0;
    t.l[0] = 0;
    t.needsupdate = 1;
}

pub fn hulib_init_text_line(
    t: &mut hu_textline_t,
    x: c_int,
    y: c_int,
    f: *mut *mut patch_t,
    sc: c_int,
) {
    t.x = x;
    t.y = y;
    t.f = f;
    t.sc = sc;
    hulib_clear_text_line(t);
}

pub fn hulib_add_char_to_text_line(t: &mut hu_textline_t, ch: c_char) -> bool {
    let max_len = HU_MAXLINELENGTH as c_int;
    if t.len >= max_len {
        return false;
    }
    let idx = t.len as usize;
    t.l[idx] = ch;
    t.len += 1;
    t.l[t.len as usize] = 0;
    t.needsupdate = 4;
    true
}

pub fn hulib_del_char_from_text_line(t: &mut hu_textline_t) -> bool {
    if t.len <= 0 {
        return false;
    }
    t.len -= 1;
    t.l[t.len as usize] = 0;
    t.needsupdate = 4;
    true
}

pub fn hulib_draw_text_line(l: &hu_textline_t, drawcursor: bool) {
    let mut x = l.x;
    let len = l.len.max(0) as usize;
    let sc = l.sc;
    let scr_w = SCREENWIDTH as c_int;

    if l.f.is_null() {
        return;
    }

    for &ch_raw in &l.l[..len.min(l.l.len())] {
        let c = (ch_raw as u8).to_ascii_uppercase() as c_int;
        if c != (b' ' as c_int) && c >= sc && c <= (b'_' as c_int) {
            unsafe {
                let patch = *l.f.add((c - sc) as usize);
                if !patch.is_null() {
                    let w = (*patch).width as c_int;
                    if x + w > scr_w {
                        break;
                    }
                    V_DrawPatchDirect(x, l.y, FG, patch as *mut _);
                    x += w;
                    continue;
                }
            }
        }
        x += 4;
        if x >= scr_w {
            break;
        }
    }

    if drawcursor {
        let cursor_idx = ((b'_' as c_int) - sc) as usize;
        unsafe {
            let cursor_patch = *l.f.add(cursor_idx);
            if !cursor_patch.is_null() {
                let cw = (*cursor_patch).width as c_int;
                if x + cw <= scr_w {
                    V_DrawPatchDirect(x, l.y, FG, cursor_patch as *mut _);
                }
            }
        }
    }
}

pub fn hulib_erase_text_line(l: &mut hu_textline_t) {
    let scr_w = SCREENWIDTH as c_int;
    let (is_automap, win_x, win_y, vw, vh) =
        unsafe { (automapactive, viewwindowx, viewwindowy, viewwidth, viewheight) };

    if is_automap == 0 && win_x != 0 && l.needsupdate != 0 && !l.f.is_null() {
        let lh = unsafe {
            let first_patch = *l.f;
            if first_patch.is_null() {
                return;
            }
            ((*first_patch).height as c_int) + 1
        };

        let start_y = l.y;
        let mut yoffset = start_y * scr_w;

        for y in start_y..(start_y + lh) {
            unsafe {
                if y < win_y || y >= win_y + vh {
                    R_VideoErase(yoffset as u32, scr_w);
                } else {
                    R_VideoErase(yoffset as u32, win_x);
                    R_VideoErase((yoffset + win_x + vw) as u32, win_x);
                }
            }
            yoffset += scr_w;
        }
    }

    if l.needsupdate != 0 {
        l.needsupdate -= 1;
    }
}

pub fn hulib_init_stext(
    s: &mut hu_stext_t,
    x: c_int,
    y: c_int,
    h: c_int,
    font: *mut *mut patch_t,
    startchar: c_int,
    on: *mut c_int,
) {
    s.h = h;
    s.on = on;
    s.laston = 1;
    s.cl = 0;

    let line_step = unsafe {
        if font.is_null() || (*font).is_null() {
            8
        } else {
            ((**font).height as c_int) + 1
        }
    };

    for i in 0..(h.max(0) as usize).min(s.l.len()) {
        hulib_init_text_line(
            &mut s.l[i],
            x,
            y - (i as c_int) * line_step,
            font,
            startchar,
        );
    }
}

pub fn hulib_add_line_to_stext(s: &mut hu_stext_t) {
    s.cl += 1;
    if s.cl == s.h {
        s.cl = 0;
    }
    let cl = s.cl.max(0) as usize;
    if cl < s.l.len() {
        hulib_clear_text_line(&mut s.l[cl]);
    }

    let count = (s.h.max(0) as usize).min(s.l.len());
    for line in &mut s.l[..count] {
        line.needsupdate = 4;
    }
}

pub fn hulib_add_message_to_stext(s: &mut hu_stext_t, prefix: Option<&[u8]>, msg: Option<&[u8]>) {
    hulib_add_line_to_stext(s);
    let cl = s.cl.max(0) as usize;
    let Some(line) = s.l.get_mut(cl) else { return };

    if let Some(p_bytes) = prefix {
        for &b in p_bytes {
            if b == 0 {
                break;
            }
            hulib_add_char_to_text_line(line, b as c_char);
        }
    }

    if let Some(m_bytes) = msg {
        for &b in m_bytes {
            if b == 0 {
                break;
            }
            hulib_add_char_to_text_line(line, b as c_char);
        }
    }
}

pub fn hulib_draw_stext(s: &hu_stext_t) {
    let is_on = unsafe { !s.on.is_null() && *s.on != 0 };
    if !is_on {
        return;
    }

    let h = s.h;
    for i in 0..h {
        let mut idx = s.cl - i;
        if idx < 0 {
            idx += h;
        }
        if let Some(line) = s.l.get(idx as usize) {
            hulib_draw_text_line(line, false);
        }
    }
}

pub fn hulib_erase_stext(s: &mut hu_stext_t) {
    let on_val = unsafe { if s.on.is_null() { 0 } else { *s.on } };
    let count = (s.h.max(0) as usize).min(s.l.len());
    for line in &mut s.l[..count] {
        if s.laston != 0 && on_val == 0 {
            line.needsupdate = 4;
        }
        hulib_erase_text_line(line);
    }
    s.laston = on_val;
}

pub fn hulib_init_itext(
    it: &mut hu_itext_t,
    x: c_int,
    y: c_int,
    font: *mut *mut patch_t,
    startchar: c_int,
    on: *mut c_int,
) {
    it.lm = 0;
    it.on = on;
    it.laston = 1;
    hulib_init_text_line(&mut it.l, x, y, font, startchar);
}

pub fn hulib_del_char_from_itext(it: &mut hu_itext_t) {
    if it.l.len != it.lm {
        hulib_del_char_from_text_line(&mut it.l);
    }
}

pub fn hulib_erase_line_from_itext(it: &mut hu_itext_t) {
    while it.lm != it.l.len {
        hulib_del_char_from_text_line(&mut it.l);
    }
}

pub fn hulib_reset_itext(it: &mut hu_itext_t) {
    it.lm = 0;
    hulib_clear_text_line(&mut it.l);
}

pub fn hulib_add_prefix_to_itext(it: &mut hu_itext_t, bytes: &[u8]) {
    for &b in bytes {
        if b == 0 {
            break;
        }
        hulib_add_char_to_text_line(&mut it.l, b as c_char);
    }
    it.lm = it.l.len;
}

pub fn hulib_key_in_itext(it: &mut hu_itext_t, ch: c_uchar) -> bool {
    if (b' '..=b'_').contains(&ch) {
        hulib_add_char_to_text_line(&mut it.l, ch as c_char);
    } else if (ch as u32) == KEY_BACKSPACE {
        hulib_del_char_from_itext(it);
    } else if (ch as u32) != KEY_ENTER {
        return false;
    }
    true
}

pub fn hulib_draw_itext(it: &hu_itext_t) {
    let is_on = unsafe { !it.on.is_null() && *it.on != 0 };
    if !is_on {
        return;
    }
    hulib_draw_text_line(&it.l, true);
}

pub fn hulib_erase_itext(it: &mut hu_itext_t) {
    let on_val = unsafe { if it.on.is_null() { 0 } else { *it.on } };
    if it.laston != 0 && on_val == 0 {
        it.l.needsupdate = 4;
    }
    hulib_erase_text_line(&mut it.l);
    it.laston = on_val;
}

// --- Совместимые обёртки для вызывающих модулей ---
#[unsafe(no_mangle)]
pub extern "C" fn HUlib_init() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_clearTextLine(t: *mut hu_textline_t) {
    if let Some(line) = unsafe { t.as_mut() } {
        hulib_clear_text_line(line);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_initTextLine(
    t: *mut hu_textline_t,
    x: c_int,
    y: c_int,
    f: *mut *mut patch_t,
    sc: c_int,
) {
    if let Some(line) = unsafe { t.as_mut() } {
        hulib_init_text_line(line, x, y, f, sc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addCharToTextLine(t: *mut hu_textline_t, ch: c_char) -> c_int {
    match unsafe { t.as_mut() } {
        Some(line) => hulib_add_char_to_text_line(line, ch) as c_int,
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_delCharFromTextLine(t: *mut hu_textline_t) -> c_int {
    match unsafe { t.as_mut() } {
        Some(line) => hulib_del_char_from_text_line(line) as c_int,
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_drawTextLine(l: *mut hu_textline_t, drawcursor: c_int) {
    if let Some(line) = unsafe { l.as_ref() } {
        hulib_draw_text_line(line, drawcursor != 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseTextLine(l: *mut hu_textline_t) {
    if let Some(line) = unsafe { l.as_mut() } {
        hulib_erase_text_line(line);
    }
}

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
    if let Some(st) = unsafe { s.as_mut() } {
        hulib_init_stext(st, x, y, h, font, startchar, on);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addLineToSText(s: *mut hu_stext_t) {
    if let Some(st) = unsafe { s.as_mut() } {
        hulib_add_line_to_stext(st);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addMessageToSText(
    s: *mut hu_stext_t,
    prefix: *mut c_char,
    msg: *mut c_char,
) {
    let Some(st) = (unsafe { s.as_mut() }) else { return };
    let p_bytes = if prefix.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(prefix).to_bytes() })
    };
let m_bytes = if msg.is_null() {
None
} else {
Some(unsafe { CStr::from_ptr(msg).to_bytes() })
};
hulib_add_message_to_stext(st, p_bytes, m_bytes);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_drawSText(s: *mut hu_stext_t) {
if let Some(st) = unsafe { s.as_ref() } {
hulib_draw_stext(st);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseSText(s: *mut hu_stext_t) {
if let Some(st) = unsafe { s.as_mut() } {
hulib_erase_stext(st);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_initIText(
it: *mut hu_itext_t,
x: c_int,
y: c_int,
font: *mut *mut patch_t,
startchar: c_int,
on: *mut c_int,
) {
if let Some(item) = unsafe { it.as_mut() } {
hulib_init_itext(item, x, y, font, startchar, on);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_delCharFromIText(it: *mut hu_itext_t) {
if let Some(item) = unsafe { it.as_mut() } {
hulib_del_char_from_itext(item);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseLineFromIText(it: *mut hu_itext_t) {
if let Some(item) = unsafe { it.as_mut() } {
hulib_erase_line_from_itext(item);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_resetIText(it: *mut hu_itext_t) {
if let Some(item) = unsafe { it.as_mut() } {
hulib_reset_itext(item);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_addPrefixToIText(it: *mut hu_itext_t, str: *mut c_char) {
if let Some(item) = unsafe { it.as_mut() } {
if !str.is_null() {
let bytes = unsafe { CStr::from_ptr(str).to_bytes() };
hulib_add_prefix_to_itext(item, bytes);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_keyInIText(it: *mut hu_itext_t, ch: c_uchar) -> c_int {
match unsafe { it.as_mut() } {
Some(item) => hulib_key_in_itext(item, ch) as c_int,
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_drawIText(it: *mut hu_itext_t) {
if let Some(item) = unsafe { it.as_ref() } {
hulib_draw_itext(item);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUlib_eraseIText(it: *mut hu_itext_t) {
if let Some(item) = unsafe { it.as_mut() } {
hulib_erase_itext(item);
}
}
