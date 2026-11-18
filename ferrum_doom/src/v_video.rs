#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

pub use crate::bindings::*;

use std::ffi::{CString, c_char, c_int};
use std::ptr;

use crate::m_bbox::m_add_to_box;

const SCR_W: usize = SCREENWIDTH as usize;
const SCR_H: usize = SCREENHEIGHT as usize;
const SCR_SIZE: usize = SCR_W * SCR_H;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
}

pub fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Video error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

#[repr(C)]
pub struct column_t {
    pub topdelta: u8,
    pub length: u8,
}

#[unsafe(no_mangle)]
pub static mut screens: [*mut u8; 5] = [ptr::null_mut(); 5];

#[unsafe(no_mangle)]
pub static mut dirtybox: [c_int; 4] = [0; 4];

#[unsafe(no_mangle)]
pub static mut usegamma: c_int = 0;

#[unsafe(no_mangle)]
pub static gammatable: [[u8; 256]; 5] = [
    [
        1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,
        17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,
        33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,
        49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,
        65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,
        81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,96,
        97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,
        113,114,115,116,117,118,119,120,121,122,123,124,125,126,127,128,
        128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,
        144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159,
        160,161,162,163,164,165,166,167,168,169,170,171,172,173,174,175,
        176,177,178,179,180,181,182,183,184,185,186,187,188,189,190,191,
        192,193,194,195,196,197,198,199,200,201,202,203,204,205,206,207,
        208,209,210,211,212,213,214,215,216,217,218,219,220,221,222,223,
        224,225,226,227,228,229,230,231,232,233,234,235,236,237,238,239,
        240,241,242,243,244,245,246,247,248,249,250,251,252,253,254,255
    ],
    [
        2,4,5,7,8,10,11,12,14,15,16,18,19,20,21,23,24,25,26,27,29,30,31,
        32,33,34,36,37,38,39,40,41,42,44,45,46,47,48,49,50,51,52,54,55,
        56,57,58,59,60,61,62,63,64,65,66,67,69,70,71,72,73,74,75,76,77,
        78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,96,97,98,
        99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,114,
        115,116,117,118,119,120,121,122,123,124,125,126,127,128,129,129,
        130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,145,
        146,147,148,148,149,150,151,152,153,154,155,156,157,158,159,160,
        161,162,163,163,164,165,166,167,168,169,170,171,172,173,174,175,
        175,176,177,178,179,180,181,182,183,184,185,186,186,187,188,189,
        190,191,192,193,194,195,196,196,197,198,199,200,201,202,203,204,
        205,205,206,207,208,209,210,211,212,213,214,214,215,216,217,218,
        219,220,221,222,222,223,224,225,226,227,228,229,230,230,231,232,
        233,234,235,236,237,237,238,239,240,241,242,243,244,245,245,246,
        247,248,249,250,251,252,252,253,254,255
    ],
    [
        4,7,9,11,13,15,17,19,21,22,24,26,27,29,30,32,33,35,36,38,39,40,42,
        43,45,46,47,48,50,51,52,54,55,56,57,59,60,61,62,63,65,66,67,68,69,
        70,72,73,74,75,76,77,78,79,80,82,83,84,85,86,87,88,89,90,91,92,93,
        94,95,96,97,98,100,101,102,103,104,105,106,107,108,109,110,111,112,
        113,114,114,115,116,117,118,119,120,121,122,123,124,125,126,127,128,
        129,130,131,132,133,133,134,135,136,137,138,139,140,141,142,143,144,
        144,145,146,147,148,149,150,151,152,153,153,154,155,156,157,158,159,
        160,160,161,162,163,164,165,166,166,167,168,169,170,171,172,172,173,
        174,175,176,177,178,178,179,180,181,182,183,183,184,185,186,187,188,
        188,189,190,191,192,193,193,194,195,196,197,197,198,199,200,201,201,
        202,203,204,205,206,206,207,208,209,210,210,211,212,213,213,214,215,
        216,217,217,218,219,220,221,221,222,223,224,224,225,226,227,228,228,
        229,230,231,231,232,233,234,235,235,236,237,238,238,239,240,241,241,
        242,243,244,244,245,246,247,247,248,249,250,251,251,252,253,254,254,
        255
    ],
    [
        8,12,16,19,22,24,27,29,31,34,36,38,40,41,43,45,47,49,50,52,53,55,
        57,58,60,61,63,64,65,67,68,70,71,72,74,75,76,77,79,80,81,82,84,85,
        86,87,88,90,91,92,93,94,95,96,98,99,100,101,102,103,104,105,106,107,
        108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,
        125,126,127,128,129,130,131,132,133,134,135,135,136,137,138,139,140,
        141,142,143,143,144,145,146,147,148,149,150,150,151,152,153,154,155,
        155,156,157,158,159,160,160,161,162,163,164,165,165,166,167,168,169,
        169,170,171,172,173,173,174,175,176,176,177,178,179,180,180,181,182,
        183,183,184,185,186,186,187,188,189,189,190,191,192,192,193,194,195,
        195,196,197,197,198,199,200,200,201,202,202,203,204,205,205,206,207,
        207,208,209,210,210,211,212,212,213,214,214,215,216,216,217,218,219,
        219,220,221,221,222,223,223,224,225,225,226,227,227,228,229,229,230,
        231,231,232,233,233,234,235,235,236,237,237,238,238,239,240,240,241,
        242,242,243,244,244,245,246,246,247,247,248,249,249,250,251,251,252,
        253,253,254,254,255
    ],
    [
        16,23,28,32,36,39,42,45,48,50,53,55,57,60,62,64,66,68,69,71,73,75,76,
        78,80,81,83,84,86,87,89,90,92,93,94,96,97,98,100,101,102,103,105,106,
        107,108,109,110,112,113,114,115,116,117,118,119,120,121,122,123,124,
        125,126,128,128,129,130,131,132,133,134,135,136,137,138,139,140,141,
        142,143,143,144,145,146,147,148,149,150,150,151,152,153,154,155,155,
        156,157,158,159,159,160,161,162,163,163,164,165,166,166,167,168,169,
        169,170,171,172,172,173,174,175,175,176,177,177,178,179,180,180,181,
        182,182,183,184,184,185,186,187,187,188,189,189,190,191,191,192,193,
        193,194,195,195,196,196,197,198,198,199,200,200,201,202,202,203,203,
        204,205,205,206,207,207,208,208,209,210,210,211,211,212,213,213,214,
        214,215,216,216,217,217,218,219,219,220,220,221,221,222,223,223,224,
        224,225,225,226,227,227,228,228,229,229,230,230,231,232,232,233,233,
        234,234,235,235,236,236,237,237,238,239,239,240,240,241,241,242,242,
        243,243,244,244,245,245,246,246,247,247,248,248,249,249,250,250,251,
        251,252,252,253,254,254,255,255
    ]
];

unsafe fn get_screen_slice(scrn: c_int) -> &'static mut [u8] {
    unsafe {
        let scr_ptr = (&raw const screens) as *const *mut u8;
        let ptr = *scr_ptr.add(scrn as usize);
        std::slice::from_raw_parts_mut(ptr, SCR_SIZE)
    }
}

pub fn v_mark_rect(x: c_int, y: c_int, width: c_int, height: c_int) {
    unsafe {
        let box_arr = &mut *(&raw mut dirtybox);
        m_add_to_box(box_arr, x, y);
        m_add_to_box(box_arr, x + width - 1, y + height - 1);
    }
}

fn copy_rect_safe(
    srcx: usize,
    srcy: usize,
    src: &[u8],
    width: usize,
    height: usize,
    destx: usize,
    desty: usize,
    dest: &mut [u8],
) {
    for row in 0..height {
        let s_ofs = (srcy + row) * SCR_W + srcx;
        let d_ofs = (desty + row) * SCR_W + destx;
        dest[d_ofs..d_ofs + width].copy_from_slice(&src[s_ofs..s_ofs + width]);
    }
}

fn draw_patch_safe(
    mut x: c_int,
    mut y: c_int,
    scrn: c_int,
    patch: &patch_t,
    flipped: bool,
) {
    let w = patch.width as c_int;
    let h = patch.height as c_int;
    y -= patch.topoffset as c_int;
    x -= patch.leftoffset as c_int;

    if (scrn as u32) > 4 {
        return;
    }

    let scr_w = SCREENWIDTH as c_int;
    let scr_h = SCREENHEIGHT as c_int;

    let mark_x = x.max(0);
    let mark_y = y.max(0);
    let mark_w = (x + w).min(scr_w) - mark_x;
    let mark_h = (y + h).min(scr_h) - mark_y;
    if mark_w <= 0 || mark_h <= 0 {
        return;
    }

    if scrn == 0 {
        v_mark_rect(mark_x, mark_y, mark_w, mark_h);
    }

    let dst = unsafe { get_screen_slice(scrn) };
    let patch_base = (patch as *const patch_t) as *const u8;
    let col_ofs = unsafe { std::slice::from_raw_parts(patch.columnofs.as_ptr(), w.max(0) as usize) };

    let start_col = (-x).max(0);
    let end_col = (scr_w - x).min(w);

    for col in start_col..end_col {
        let cur_x = (x + col) as usize;
        let lookup_col = if flipped { w - 1 - col } else { col } as usize;
        let mut ofs = col_ofs[lookup_col] as usize;

        loop {
            let topdelta = unsafe { *patch_base.add(ofs) };
            if topdelta == 0xff {
                break;
            }
            let length = unsafe { *patch_base.add(ofs + 1) } as c_int;
            let src_col = unsafe { std::slice::from_raw_parts(patch_base.add(ofs + 3), length.max(0) as usize) };

            let mut dest_y = y + (topdelta as c_int);
            let mut count = length;
            let mut src_skip = 0usize;

            if dest_y < 0 {
                let skip = -dest_y;
                if skip >= count {
                    ofs += (length as usize) + 4;
                    continue;
                }
                src_skip = skip as usize;
                count -= skip;
                dest_y = 0;
            }

            if dest_y + count > scr_h {
                count = scr_h - dest_y;
            }

            if count > 0 {
                let dy = dest_y as usize;
                let cnt = count as usize;
                for row in 0..cnt {
                    dst[(dy + row) * SCR_W + cur_x] = src_col[src_skip + row];
                }
            }

            ofs += (length as usize) + 4;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_MarkRect(x: c_int, y: c_int, width: c_int, height: c_int) {
    v_mark_rect(x, y, width, height);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_CopyRect(
    srcx: c_int,
    srcy: c_int,
srcscrn: c_int,
width: c_int,
height: c_int,
destx: c_int,
desty: c_int,
destscrn: c_int,
) {
if srcx < 0
|| srcx + width > SCREENWIDTH as c_int
|| srcy < 0
|| srcy + height > SCREENHEIGHT as c_int
|| destx < 0
|| destx + width > SCREENWIDTH as c_int
|| desty < 0
|| desty + height > SCREENHEIGHT as c_int
|| (srcscrn as u32) > 4
|| (destscrn as u32) > 4
{
doom_error("Bad V_CopyRect");
}
v_mark_rect(destx, desty, width, height);
let src = unsafe { get_screen_slice(srcscrn) };
let dst = unsafe { get_screen_slice(destscrn) };
copy_rect_safe(
srcx as usize,
srcy as usize,
src,
width as usize,
height as usize,
destx as usize,
desty as usize,
dst,
);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_DrawPatch(x: c_int, y: c_int, scrn: c_int, patch: *mut patch_t) {
if let Some(p) = unsafe { patch.as_ref() } {
draw_patch_safe(x, y, scrn, p, false);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_DrawPatchFlipped(x: c_int, y: c_int, scrn: c_int, patch: *mut patch_t) {
if let Some(p) = unsafe { patch.as_ref() } {
draw_patch_safe(x, y, scrn, p, true);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_DrawPatchDirect(x: c_int, y: c_int, scrn: c_int, patch: *mut patch_t) {
if let Some(p) = unsafe { patch.as_ref() } {
draw_patch_safe(x, y, scrn, p, false);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_DrawBlock(
x: c_int,
y: c_int,
scrn: c_int,
width: c_int,
height: c_int,
src: *mut u8,
) {
if x < 0
|| x + width > SCREENWIDTH as c_int
|| y < 0
|| y + height > SCREENHEIGHT as c_int
|| (scrn as u32) > 4
|| src.is_null()
{
doom_error("Bad V_DrawBlock");
}
v_mark_rect(x, y, width, height);
let w = width as usize;
let h = height as usize;
let ux = x as usize;
let uy = y as usize;
let src_slice = unsafe { std::slice::from_raw_parts(src, w * h) };
let dst = unsafe { get_screen_slice(scrn) };
for row in 0..h {
let s_ofs = row * w;
let d_ofs = (uy + row) * SCR_W + ux;
dst[d_ofs..d_ofs + w].copy_from_slice(&src_slice[s_ofs..s_ofs + w]);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_GetBlock(
x: c_int,
y: c_int,
scrn: c_int,
width: c_int,
height: c_int,
dest: *mut u8,
) {
if x < 0
|| x + width > SCREENWIDTH as c_int
|| y < 0
|| y + height > SCREENHEIGHT as c_int
|| (scrn as u32) > 4
|| dest.is_null()
{
doom_error("Bad V_GetBlock");
}
let w = width as usize;
let h = height as usize;
let ux = x as usize;
let uy = y as usize;
let src = unsafe { get_screen_slice(scrn) };
let dst_slice = unsafe { std::slice::from_raw_parts_mut(dest, w * h) };
for row in 0..h {
let s_ofs = (uy + row) * SCR_W + ux;
let d_ofs = row * w;
dst_slice[d_ofs..d_ofs + w].copy_from_slice(&src[s_ofs..s_ofs + w]);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn V_Init() {
let base = Box::leak(vec![0u8; SCR_SIZE * 4].into_boxed_slice());
unsafe {
let scr_ptr = (&raw mut screens) as *mut *mut u8;
for i in 0..4 {
*scr_ptr.add(i) = base.as_mut_ptr().add(i * SCR_SIZE);
}
}
}
