#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int, c_uint};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

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
const SCR_H_USIZE: usize = SCREENHEIGHT as usize;
const SCR_PIXELS: usize = SCR_W_USIZE * SCR_H_USIZE;

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

static FUZZPOS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    static mut centery: c_int;
    static mut colormaps: *mut u8;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Draw error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

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

#[inline]
unsafe fn screen_slice(idx: usize) -> &'static mut [u8] {
    unsafe {
        let scr_ptr = (&raw const screens) as *const *mut u8;
        std::slice::from_raw_parts_mut(*scr_ptr.add(idx), SCR_PIXELS)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawColumn() {
    unsafe {
        let count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        let col_ofs = (&raw const columnofs).as_ref().unwrap();
        let start_y = (viewwindowy + dc_yl) as usize;
        let x_pos = col_ofs[dc_x as usize] as usize;

        let scr = screen_slice(0);
        let cmap = std::slice::from_raw_parts(dc_colormap, 256);
        let src = std::slice::from_raw_parts(dc_source, 128);

        let fracstep = dc_iscale;
        let mut frac = dc_texturemid.wrapping_add((dc_yl - centery).wrapping_mul(fracstep));

        for row in 0..=(count as usize) {
            let tex_idx = ((frac >> FRACBITS) & 127) as usize;
            scr[(start_y + row) * SCR_W_USIZE + x_pos] = cmap[src[tex_idx] as usize];
            frac = frac.wrapping_add(fracstep);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawColumnLow() {
    unsafe {
        let count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        dc_x <<= 1;
        let col_ofs = (&raw const columnofs).as_ref().unwrap();
        let start_y = (viewwindowy + dc_yl) as usize;
        let x1 = col_ofs[dc_x as usize] as usize;
        let x2 = col_ofs[(dc_x + 1) as usize] as usize;

        let scr = screen_slice(0);
        let cmap = std::slice::from_raw_parts(dc_colormap, 256);
        let src = std::slice::from_raw_parts(dc_source, 128);

        let fracstep = dc_iscale;
        let mut frac = dc_texturemid.wrapping_add((dc_yl - centery).wrapping_mul(fracstep));

        for row in 0..=(count as usize) {
            let tex_idx = ((frac >> FRACBITS) & 127) as usize;
            let pixel = cmap[src[tex_idx] as usize];
            let base = (start_y + row) * SCR_W_USIZE;
            scr[base + x1] = pixel;
            scr[base + x2] = pixel;
            frac = frac.wrapping_add(fracstep);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawFuzzColumn() {
    unsafe {
        if dc_yl == 0 {
            dc_yl = 1;
        }
        if dc_yh == viewheight - 1 {
            dc_yh = viewheight - 2;
        }

        let count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawFuzzColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        let col_ofs = (&raw const columnofs).as_ref().unwrap();
        let start_y = (viewwindowy + dc_yl) as usize;
        let x_pos = col_ofs[dc_x as usize] as usize;

        let scr = screen_slice(0);
        let cmap_6 = std::slice::from_raw_parts(colormaps.add(6 * 256), 256);
        let mut fpos = FUZZPOS.load(Ordering::Relaxed);

        for row in 0..=(count as usize) {
            let cur_idx = (start_y + row) * SCR_W_USIZE + x_pos;
            let src_idx = (cur_idx as isize + FUZZOFFSET[fpos]).clamp(0, (SCR_PIXELS - 1) as isize) as usize;
            let neighbor = scr[src_idx] as usize;
            scr[cur_idx] = cmap_6[neighbor];

            fpos += 1;
            if fpos == FUZZTABLE {
                fpos = 0;
            }
        }
        FUZZPOS.store(fpos, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawTranslatedColumn() {
    unsafe {
        let count = dc_yh - dc_yl;
        if count < 0 {
            return;
        }

        if (dc_x as u32) >= SCREENWIDTH || dc_yl < 0 || dc_yh >= SCR_H {
            doom_error(&format!("R_DrawColumn: {} to {} at {}", dc_yl, dc_yh, dc_x));
        }

        let col_ofs = (&raw const columnofs).as_ref().unwrap();
        let start_y = (viewwindowy + dc_yl) as usize;
        let x_pos = col_ofs[dc_x as usize] as usize;

        let scr = screen_slice(0);
        let cmap = std::slice::from_raw_parts(dc_colormap, 256);
        let trans = std::slice::from_raw_parts(dc_translation, 256);
        let src = std::slice::from_raw_parts(dc_source, 256);

        let fracstep = dc_iscale;
        let mut frac = dc_texturemid.wrapping_add((dc_yl - centery).wrapping_mul(fracstep));

        for row in 0..=(count as usize) {
            let tex_idx = ((frac >> FRACBITS) & 255) as usize;
            let mapped_color = trans[src[tex_idx] as usize] as usize;
            scr[(start_y + row) * SCR_W_USIZE + x_pos] = cmap[mapped_color];
            frac = frac.wrapping_add(fracstep);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitTranslationTables() {
    unsafe {
        let raw_ptr = Z_Malloc(256 * 3 + 255, PU_STATIC, ptr::null_mut()) as usize;
        let aligned = (raw_ptr + 255) & !255;
        translationtables = aligned as *mut u8;

        let tables = std::slice::from_raw_parts_mut(translationtables, 256 * 3);
        for i in 0..256usize {
            if (0x70..=0x7f).contains(&i) {
                let low = (i & 0xf) as u8;
                tables[i] = 0x60 + low;
                tables[i + 256] = 0x40 + low;
                tables[i + 512] = 0x20 + low;
            } else {
                let b = i as u8;
                tables[i] = b;
                tables[i + 256] = b;
                tables[i + 512] = b;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawSpan() {
    unsafe {
        if ds_x2 < ds_x1 || ds_x1 < 0 || ds_x2 >= SCR_W || (ds_y as u32) > SCREENHEIGHT {
            doom_error(&format!("R_DrawSpan: {} to {} at {}", ds_x1, ds_x2, ds_y));
        }

        let mut xfrac = ds_xfrac;
        let mut yfrac = ds_yfrac;
let col_ofs = (&raw const columnofs).as_ref().unwrap();
let row_y = (viewwindowy + ds_y) as usize;
let start_x = col_ofs[ds_x1 as usize] as usize;
let count = (ds_x2 - ds_x1) as usize;
let scr = screen_slice(0);
let cmap = std::slice::from_raw_parts(ds_colormap, 256);
let src = std::slice::from_raw_parts(ds_source, 64 * 64);
let xstep = ds_xstep;
let ystep = ds_ystep;
let dst_row = &mut scr[row_y * SCR_W_USIZE + start_x..=row_y * SCR_W_USIZE + start_x + count];
for px in dst_row {
let spot = (((yfrac >> (16 - 6)) & (63 * 64)) + ((xfrac >> 16) & 63)) as usize;
*px = cmap[src[spot] as usize];
xfrac = xfrac.wrapping_add(xstep);
yfrac = yfrac.wrapping_add(ystep);
}
}
}
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
let col_ofs = (&raw const columnofs).as_ref().unwrap();
let row_y = (viewwindowy + ds_y) as usize;
let start_x = col_ofs[ds_x1 as usize] as usize;
let count = (ds_x2 - ds_x1) as usize;
let scr = screen_slice(0);
let cmap = std::slice::from_raw_parts(ds_colormap, 256);
let src = std::slice::from_raw_parts(ds_source, 64 * 64);
let xstep = ds_xstep;
let ystep = ds_ystep;
let base = row_y * SCR_W_USIZE + start_x;
let mut i = 0usize;
while i <= count {
let spot = (((yfrac >> (16 - 6)) & (63 * 64)) + ((xfrac >> 16) & 63)) as usize;
let pixel = cmap[src[spot] as usize];
scr[base + i] = pixel;
if i + 1 <= count {
scr[base + i + 1] = pixel;
}
i += 2;
xfrac = xfrac.wrapping_add(xstep);
yfrac = yfrac.wrapping_add(ystep);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitBuffer(width: c_int, height: c_int) {
unsafe {
viewwindowx = (SCR_W - width) >> 1;
let col_arr = (&raw mut columnofs).as_mut().unwrap();
for i in 0..(width as usize) {
col_arr[i] = viewwindowx + (i as c_int);
}
viewwindowy = if width == SCR_W {
0
} else {
(SCR_H - SBARHEIGHT - height) >> 1
};
let scr0 = *(&raw const screens as *const *mut u8);
let ylook_arr = (&raw mut ylookup).as_mut().unwrap();
for i in 0..(height as usize) {
ylook_arr[i] = scr0.add(((i as c_int + viewwindowy) * SCR_W) as usize);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_FillBackScreen() {
unsafe {
if scaledviewwidth == 320 {
return;
}
let name: &[u8] = if get_gamemode() == commercial {
b"GRNROCK\0"
} else {
b"FLOOR7_2\0"
};
let src_ptr = W_CacheLumpName(name.as_ptr() as *const c_char, PU_CACHE) as *const u8;
let src = std::slice::from_raw_parts(src_ptr, 64 * 64);
let dst = screen_slice(1);
let max_y = (SCR_H - SBARHEIGHT) as usize;
for y in 0..max_y {
let row_src = &src[((y & 63) << 6)..((y & 63) << 6) + 64];
let dst_row = &mut dst[y * SCR_W_USIZE..(y + 1) * SCR_W_USIZE];
for chunk in dst_row.chunks_mut(64) {
chunk.copy_from_slice(&row_src[..chunk.len()]);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_VideoErase(ofs: c_uint, count: c_int) {
if count <= 0 {
return;
}
let start = ofs as usize;
let len = count as usize;
let scr0 = unsafe { screen_slice(0) };
let scr1 = unsafe { screen_slice(1) };
scr0[start..start + len].copy_from_slice(&scr1[start..start + len]);
}
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
