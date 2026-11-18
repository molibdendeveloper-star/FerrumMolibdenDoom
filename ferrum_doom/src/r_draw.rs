#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::ptr;

use crate::m_fixed::FRACBITS;
use crate::v_video::{V_DrawPatch, V_MarkRect, screens};
use crate::w_wad::W_CacheLumpName;
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_Malloc};

pub const MAXWIDTH: usize = 1120;
pub const MAXHEIGHT: usize = 832;
pub const SBARHEIGHT: c_int = 32;

const SCR_W: c_int = SCREENWIDTH as c_int;
const SCR_H: c_int = SCREENHEIGHT as c_int;
const SCR_W_USIZE: usize = SCREENWIDTH as usize;

const FUZZTABLE: usize = 50;
const FUZZOFF: isize = SCR_W as isize;

static FUZZOFFSET: [isize; FUZZTABLE] = [
    FUZZOFF, -FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF,
    FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF,
    FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF,
    FUZZOFF, -FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF,
    FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, -FUZZOFF, FUZZOFF,
    FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF,
    FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF, FUZZOFF, -FUZZOFF, FUZZOFF,
];

static mut FUZZPOS: usize = 0;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    static mut centery: c_int;
    static mut colormaps: *mut u8;
    static mut gamemode: GameMode_t;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Draw error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные r_draw.c ---
#[unsafe(no_mangle)]
pub static mut viewimage: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut viewwidth: c_int = 0;

#[unsafe(no_mangle)]
pub static mut scaledviewwidth: c_int = 0;

#[unsafe(no_mangle)]
pub static mut viewheight: c_int = 0;

#[unsafe(no_mangle)]
pub static mut viewwindowx: c_int = 0;

#[unsafe(no_mangle)]
pub static mut viewwindowy: c_int = 0;

#[unsafe(no_mangle)]
pub static mut ylookup: [*mut u8; MAXHEIGHT] = [ptr::null_mut(); MAXHEIGHT];

#[unsafe(no_mangle)]
pub static mut columnofs: [c_int; MAXWIDTH] = [0; MAXWIDTH];

#[unsafe(no_mangle)]
pub static mut translations: [[u8; 256]; 3] = [[0; 256]; 3];

#[unsafe(no_mangle)]
pub static mut dc_colormap: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut dc_x: c_int = 0;

#[unsafe(no_mangle)]
pub static mut dc_yl: c_int = 0;

#[unsafe(no_mangle)]
pub static mut dc_yh: c_int = 0;

#[unsafe(no_mangle)]
pub static mut dc_iscale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut dc_texturemid: i32 = 0;

#[unsafe(no_mangle)]
pub static mut dc_source: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut dccount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut dc_translation: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut translationtables: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut ds_y: c_int = 0;

#[unsafe(no_mangle)]
pub static mut ds_x1: c_int = 0;

#[unsafe(no_mangle)]
pub static mut ds_x2: c_int = 0;

#[unsafe(no_mangle)]
pub static mut ds_colormap: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut ds_xfrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut ds_yfrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut ds_xstep: i32 = 0;

#[unsafe(no_mangle)]
pub static mut ds_ystep: i32 = 0;

#[unsafe(no_mangle)]
pub static mut ds_source: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut dscount: c_int = 0;

// --- R_DrawColumn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawColumn() {
    unsafe {
        let mut count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        let ylook_ptr = (&raw const ylookup) as *const *mut u8;
        let col_ptr = (&raw const columnofs) as *const c_int;
        let mut dest = (*ylook_ptr.add(dc_yl as usize)).offset(*col_ptr.add(dc_x as usize) as isize);

        let fracstep = dc_iscale;
        let mut frac = dc_texturemid.wrapping_add((dc_yl - centery).wrapping_mul(fracstep));

        let cmap = dc_colormap;
        let src = dc_source;

        loop {
            let tex_idx = ((frac >> FRACBITS) & 127) as usize;
            let color_idx = *src.add(tex_idx) as usize;
            *dest = *cmap.add(color_idx);

            dest = dest.add(SCR_W_USIZE);
            frac = frac.wrapping_add(fracstep);

            if count == 0 {
                break;
            }
            count -= 1;
        }
    }
}

// --- R_DrawColumnLow ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawColumnLow() {
    unsafe {
        let mut count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        dc_x <<= 1;

        let ylook_ptr = (&raw const ylookup) as *const *mut u8;
        let col_ptr = (&raw const columnofs) as *const c_int;
        let base_row = *ylook_ptr.add(dc_yl as usize);
        let mut dest = base_row.offset(*col_ptr.add(dc_x as usize) as isize);
        let mut dest2 = base_row.offset(*col_ptr.add((dc_x + 1) as usize) as isize);

        let fracstep = dc_iscale;
        let mut frac = dc_texturemid.wrapping_add((dc_yl - centery).wrapping_mul(fracstep));

        let cmap = dc_colormap;
        let src = dc_source;

        loop {
            let tex_idx = ((frac >> FRACBITS) & 127) as usize;
            let color_idx = *src.add(tex_idx) as usize;
            let pixel = *cmap.add(color_idx);
            *dest = pixel;
            *dest2 = pixel;

            dest = dest.add(SCR_W_USIZE);
            dest2 = dest2.add(SCR_W_USIZE);
            frac = frac.wrapping_add(fracstep);

            if count == 0 {
                break;
            }
            count -= 1;
        }
    }
}

// --- R_DrawFuzzColumn (невидимость спектров) ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawFuzzColumn() {
    unsafe {
        if dc_yl == 0 {
            dc_yl = 1;
        }
        if dc_yh == viewheight - 1 {
            dc_yh = viewheight - 2;
        }

        let mut count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawFuzzColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        let ylook_ptr = (&raw const ylookup) as *const *mut u8;
        let col_ptr = (&raw const columnofs) as *const c_int;
        let mut dest = (*ylook_ptr.add(dc_yl as usize)).offset(*col_ptr.add(dc_x as usize) as isize);

        let cmap_6 = colormaps.add(6 * 256);

        loop {
            let offset = FUZZOFFSET[FUZZPOS];
            let neighbor = *dest.offset(offset) as usize;
            *dest = *cmap_6.add(neighbor);

            FUZZPOS += 1;
            if FUZZPOS == FUZZTABLE {
                FUZZPOS = 0;
            }

            dest = dest.add(SCR_W_USIZE);

            if count == 0 {
                break;
            }
            count -= 1;
        }
    }
}

// --- R_DrawTranslatedColumn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawTranslatedColumn() {
    unsafe {
        let mut count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        let ylook_ptr = (&raw const ylookup) as *const *mut u8;
        let col_ptr = (&raw const columnofs) as *const c_int;
        let mut dest = (*ylook_ptr.add(dc_yl as usize)).offset(*col_ptr.add(dc_x as usize) as isize);

        let fracstep = dc_iscale;
        let mut frac = dc_texturemid.wrapping_add((dc_yl - centery).wrapping_mul(fracstep));

        let cmap = dc_colormap;
        let trans = dc_translation;
        let src = dc_source;

        loop {
            let tex_idx = (frac >> FRACBITS) as isize;
            let raw_color = *src.offset(tex_idx) as usize;
            let mapped_color = *trans.add(raw_color) as usize;
            *dest = *cmap.add(mapped_color);

            dest = dest.add(SCR_W_USIZE);
            frac = frac.wrapping_add(fracstep);

            if count == 0 {
                break;
            }
            count -= 1;
        }
    }
}

// --- R_InitTranslationTables ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitTranslationTables() {
    unsafe {
        let raw_ptr = Z_Malloc(256 * 3 + 255, PU_STATIC, ptr::null_mut()) as usize;
        let aligned = (raw_ptr + 255) & !255;
        translationtables = aligned as *mut u8;

        for i in 0..256usize {
            if (0x70..=0x7f).contains(&i) {
                let low = (i & 0xf) as u8;
                *translationtables.add(i) = 0x60 + low;
                *translationtables.add(i + 256) = 0x40 + low;
                *translationtables.add(i + 512) = 0x20 + low;
            } else {
                let b = i as u8;
                *translationtables.add(i) = b;
                *translationtables.add(i + 256) = b;
                *translationtables.add(i + 512) = b;
            }
        }
    }
}

// --- R_DrawSpan ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawSpan() {
    unsafe {
        if ds_x2 < ds_x1 || ds_x1 < 0 || ds_x2 >= SCR_W || (ds_y as u32) > SCREENHEIGHT {
            doom_error(&format!("R_DrawSpan: {} to {} at {}", ds_x1, ds_x2, ds_y));
        }

        let mut xfrac = ds_xfrac;
        let mut yfrac = ds_yfrac;

        let ylook_ptr = (&raw const ylookup) as *const *mut u8;
let col_ptr = (&raw const columnofs) as *const c_int;
let mut dest = (*ylook_ptr.add(ds_y as usize)).offset(*col_ptr.add(ds_x1 as usize) as isize);
let mut count = ds_x2 - ds_x1;
let cmap = ds_colormap;
let src = ds_source;
let xstep = ds_xstep;
let ystep = ds_ystep;
loop {
let spot = (((yfrac >> (16 - 6)) & (63 * 64)) + ((xfrac >> 16) & 63)) as usize;
let color_idx = *src.add(spot) as usize;
*dest = *cmap.add(color_idx);
dest = dest.add(1);
xfrac = xfrac.wrapping_add(xstep);
yfrac = yfrac.wrapping_add(ystep);
if count == 0 {
break;
}
count -= 1;
}
}
}
// --- R_DrawSpanLow ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawSpanLow() {
unsafe {
if ds_x2 < ds_x1 || ds_x1 < 0 || ds_x2 >= SCR_W || (ds_y as u32) > SCREENHEIGHT {
doom_error(&format!("R_DrawSpan: {} to {} at {}", ds_x1, ds_x2, ds_y));
}
let mut xfrac = ds_xfrac;
let mut yfrac = ds_yfrac;
ds_x1 <<= 1;
ds_x2 <<= 1;
let ylook_ptr = (&raw const ylookup) as *const *mut u8;
let col_ptr = (&raw const columnofs) as *const c_int;
let mut dest = (*ylook_ptr.add(ds_y as usize)).offset(*col_ptr.add(ds_x1 as usize) as isize);
let mut count = ds_x2 - ds_x1;
let cmap = ds_colormap;
let src = ds_source;
let xstep = ds_xstep;
let ystep = ds_ystep;
loop {
let spot = (((yfrac >> (16 - 6)) & (63 * 64)) + ((xfrac >> 16) & 63)) as usize;
let color_idx = *src.add(spot) as usize;
let pixel = *cmap.add(color_idx);
*dest = pixel;
dest = dest.add(1);
*dest = pixel;
dest = dest.add(1);
xfrac = xfrac.wrapping_add(xstep);
yfrac = yfrac.wrapping_add(ystep);
if count == 0 {
break;
}
count -= 1;
}
}
}
// --- R_InitBuffer ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitBuffer(width: c_int, height: c_int) {
unsafe {
viewwindowx = (SCR_W - width) >> 1;
let col_ptr = (&raw mut columnofs) as *mut c_int;
for i in 0..width {
*col_ptr.add(i as usize) = viewwindowx + i;
}
if width == SCR_W {
viewwindowy = 0;
} else {
viewwindowy = (SCR_H - SBARHEIGHT - height) >> 1;
}
let scr0 = *(&raw const screens as *const *mut u8);
let ylook_ptr = (&raw mut ylookup) as *mut *mut u8;
for i in 0..height {
*ylook_ptr.add(i as usize) = scr0.offset(((i + viewwindowy) * SCR_W) as isize);
}
}
}
// --- R_FillBackScreen ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_FillBackScreen() {
unsafe {
if scaledviewwidth == 320 {
return;
}
let name: &[u8] = if gamemode == commercial {
b"GRNROCK\0"
} else {
b"FLOOR7_2\0"
};
let src = W_CacheLumpName(name.as_ptr() as *const c_char, PU_CACHE) as *const u8;
let scr_ptr = &raw const screens as *const *mut u8;
let mut dest = *scr_ptr.add(1);
let max_y = SCR_H - SBARHEIGHT;
for y in 0..max_y {
let row_src = src.add(((y & 63) << 6) as usize);
for _ in 0..(SCR_W / 64) {
ptr::copy_nonoverlapping(row_src, dest, 64);
dest = dest.add(64);
}
let rem = (SCR_W & 63) as usize;
if rem != 0 {
ptr::copy_nonoverlapping(row_src, dest, rem);
dest = dest.add(rem);
}
}
let draw_border = |lump: &[u8], x: c_int, y: c_int| {
let p = W_CacheLumpName(lump.as_ptr() as *const c_char, PU_CACHE) as *mut patch_t;
V_DrawPatch(x, y, 1, p as *mut _);
};
let mut x = 0;
while x < scaledviewwidth {
draw_border(b"brdr_t\0", viewwindowx + x, viewwindowy - 8);
draw_border(b"brdr_b\0", viewwindowx + x, viewwindowy + viewheight);
x += 8;
}
let mut y = 0;
while y < viewheight {
draw_border(b"brdr_l\0", viewwindowx - 8, viewwindowy + y);
draw_border(b"brdr_r\0", viewwindowx + scaledviewwidth, viewwindowy + y);
y += 8;
}
draw_border(b"brdr_tl\0", viewwindowx - 8, viewwindowy - 8);
draw_border(b"brdr_tr\0", viewwindowx + scaledviewwidth, viewwindowy - 8);
draw_border(b"brdr_bl\0", viewwindowx - 8, viewwindowy + viewheight);
draw_border(b"brdr_br\0", viewwindowx + scaledviewwidth, viewwindowy + viewheight);
}
}
// --- R_VideoErase ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_VideoErase(ofs: c_uint, count: c_int) {
unsafe {
if count <= 0 {
return;
}
let scr_ptr = &raw const screens as *const *mut u8;
let scr0 = (*scr_ptr).add(ofs as usize);
let scr1 = (*scr_ptr.add(1)).add(ofs as usize);
ptr::copy_nonoverlapping(scr1, scr0, count as usize);
}
}
// --- R_DrawViewBorder ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawViewBorder() {
unsafe {
if scaledviewwidth == SCR_W {
return;
}
let top = ((SCR_H - SBARHEIGHT) - viewheight) / 2;
let side = (SCR_W - scaledviewwidth) / 2;
R_VideoErase(0, top * SCR_W + side);
let mut ofs = (viewheight + top) * SCR_W - side;
R_VideoErase(ofs as c_uint, top * SCR_W + side);
ofs = top * SCR_W + SCR_W - side;
let double_side = side << 1;
for _ in 1..viewheight {
R_VideoErase(ofs as c_uint, double_side);
ofs += SCR_W;
}
V_MarkRect(0, 0, SCR_W, SCR_H - SBARHEIGHT);
}
}