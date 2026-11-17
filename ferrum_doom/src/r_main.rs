#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_int, c_void};
use std::io::{self, Write};
use std::ptr;

use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP};
use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::r_bsp::{R_ClearClipSegs, R_ClearDrawSegs, R_RenderBSPNode, subsector_t};
use crate::r_data::{R_InitData, colormaps};
use crate::r_draw::{
    R_DrawColumn, R_DrawColumnLow, R_DrawFuzzColumn, R_DrawSpan, R_DrawSpanLow,
    R_DrawTranslatedColumn, R_InitBuffer, R_InitTranslationTables, scaledviewwidth, viewheight,
    viewwidth,
};
use crate::r_plane::{R_ClearPlanes, R_DrawPlanes, R_InitPlanes, distscale, yslope};
use crate::r_segs::{rw_distance, rw_normalangle, walllights};
use crate::r_sky::R_InitSkyMap;
use crate::r_things::{
    R_ClearSprites, R_DrawMasked, pspriteiscale, pspritescale, screenheightarray,
};
use crate::tables::{ANGLETOFINESHIFT, DBITS, FINEANGLES, finesine, finetangent, slope_div, tantoangle};

pub const FIELDOFVIEW: usize = 2048;
pub const DISTMAP: i32 = 2;
const SCR_W: usize = SCREENWIDTH as usize;
const SCR_H: usize = SCREENHEIGHT as usize;
const HALF_FINE: usize = FINEANGLES / 2;
const QUARTER_FINE: usize = FINEANGLES / 4;

unsafe extern "C" {
    fn NetUpdate();

    static mut numnodes: c_int;
    static mut nodes: *mut node_t;
    static mut subsectors: *mut subsector_t;
    static mut detailLevel: c_int;
    static mut screenblocks: c_int;
}

// --- Глобальные переменные r_main.c ---
#[unsafe(no_mangle)]
pub static mut viewangleoffset: c_int = 0;

#[unsafe(no_mangle)]
pub static mut validcount: c_int = 1;

#[unsafe(no_mangle)]
pub static mut fixedcolormap: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut centerx: c_int = 0;

#[unsafe(no_mangle)]
pub static mut centery: c_int = 0;

#[unsafe(no_mangle)]
pub static mut centerxfrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut centeryfrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut projection: i32 = 0;

#[unsafe(no_mangle)]
pub static mut framecount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut sscount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut linecount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut loopcount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut viewx: i32 = 0;

#[unsafe(no_mangle)]
pub static mut viewy: i32 = 0;

#[unsafe(no_mangle)]
pub static mut viewz: i32 = 0;

#[unsafe(no_mangle)]
pub static mut viewangle: u32 = 0;

#[unsafe(no_mangle)]
pub static mut viewcos: i32 = 0;

#[unsafe(no_mangle)]
pub static mut viewsin: i32 = 0;

#[unsafe(no_mangle)]
pub static mut viewplayer: *mut player_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut detailshift: c_int = 0;

#[unsafe(no_mangle)]
pub static mut clipangle: u32 = 0;

#[unsafe(no_mangle)]
pub static mut viewangletox: [c_int; HALF_FINE] = [0; HALF_FINE];

#[unsafe(no_mangle)]
pub static mut xtoviewangle: [u32; SCR_W + 1] = [0; SCR_W + 1];

#[unsafe(no_mangle)]
pub static mut finecosine: *const i32 = unsafe { finesine.as_ptr().add(QUARTER_FINE) };

#[unsafe(no_mangle)]
pub static mut scalelight: [[*mut u8; MAXLIGHTSCALE as usize]; LIGHTLEVELS as usize] =
    [[ptr::null_mut(); MAXLIGHTSCALE as usize]; LIGHTLEVELS as usize];

#[unsafe(no_mangle)]
pub static mut scalelightfixed: [*mut u8; MAXLIGHTSCALE as usize] =
    [ptr::null_mut(); MAXLIGHTSCALE as usize];

#[unsafe(no_mangle)]
pub static mut zlight: [[*mut u8; MAXLIGHTZ as usize]; LIGHTLEVELS as usize] =
    [[ptr::null_mut(); MAXLIGHTZ as usize]; LIGHTLEVELS as usize];

#[unsafe(no_mangle)]
pub static mut extralight: c_int = 0;

#[unsafe(no_mangle)]
pub static mut colfunc: Option<unsafe extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub static mut basecolfunc: Option<unsafe extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub static mut fuzzcolfunc: Option<unsafe extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub static mut transcolfunc: Option<unsafe extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub static mut spanfunc: Option<unsafe extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub static mut setsizeneeded: c_int = 0;

#[unsafe(no_mangle)]
pub static mut setblocks: c_int = 0;

#[unsafe(no_mangle)]
pub static mut setdetail: c_int = 0;

// --- R_AddPointToBox ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_AddPointToBox(x: c_int, y: c_int, box_ptr: *mut i32) {
    unsafe {
        if x < *box_ptr.add(BOXLEFT) {
            *box_ptr.add(BOXLEFT) = x;
        }
        if x > *box_ptr.add(BOXRIGHT) {
            *box_ptr.add(BOXRIGHT) = x;
        }
        if y < *box_ptr.add(BOXBOTTOM) {
            *box_ptr.add(BOXBOTTOM) = y;
        }
        if y > *box_ptr.add(BOXTOP) {
            *box_ptr.add(BOXTOP) = y;
        }
    }
}

// --- R_PointOnSide ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointOnSide(x: i32, y: i32, node: *mut node_t) -> c_int {
    unsafe {
        let ndx = (*node).dx;
        let ndy = (*node).dy;
        let nx = (*node).x;
        let ny = (*node).y;

        if ndx == 0 {
            if x <= nx {
                return (ndy > 0) as c_int;
            }
            return (ndy < 0) as c_int;
        }
        if ndy == 0 {
            if y <= ny {
                return (ndx < 0) as c_int;
            }
            return (ndx > 0) as c_int;
        }

        let dx = x.wrapping_sub(nx);
        let dy = y.wrapping_sub(ny);

        if ((ndy ^ ndx ^ dx ^ dy) as u32 & 0x8000_0000) != 0 {
            if ((ndy ^ dx) as u32 & 0x8000_0000) != 0 {
                return 1;
            }
            return 0;
        }

        let left = fixed_mul(ndy >> FRACBITS, dx);
        let right = fixed_mul(dy, ndx >> FRACBITS);

        if right < left { 0 } else { 1 }
    }
}

// --- R_PointOnSegSide ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointOnSegSide(x: i32, y: i32, line: *mut seg_t) -> c_int {
    unsafe {
        let v1 = (*line).v1;
        let v2 = (*line).v2;
        let lx = (*v1).x;
        let ly = (*v1).y;
        let ldx = (*v2).x.wrapping_sub(lx);
        let ldy = (*v2).y.wrapping_sub(ly);

        if ldx == 0 {
            if x <= lx {
                return (ldy > 0) as c_int;
            }
            return (ldy < 0) as c_int;
        }
        if ldy == 0 {
            if y <= ly {
                return (ldx < 0) as c_int;
            }
            return (ldx > 0) as c_int;
        }

        let dx = x.wrapping_sub(lx);
        let dy = y.wrapping_sub(ly);

        if ((ldy ^ ldx ^ dx ^ dy) as u32 & 0x8000_0000) != 0 {
            if ((ldy ^ dx) as u32 & 0x8000_0000) != 0 {
                return 1;
            }
            return 0;
        }

        let left = fixed_mul(ldy >> FRACBITS, dx);
        let right = fixed_mul(dy, ldx >> FRACBITS);

        if right < left { 0 } else { 1 }
    }
}

// --- R_PointToAngle ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointToAngle(mut x: i32, mut y: i32) -> u32 {
    unsafe {
        x = x.wrapping_sub(viewx);
        y = y.wrapping_sub(viewy);

        if x == 0 && y == 0 {
            return 0;
        }

        if x >= 0 {
            if y >= 0 {
                if x > y {
                    tantoangle[slope_div(y as u32, x as u32) as usize]
                } else {
                    ANG90
                        .wrapping_sub(1)
                        .wrapping_sub(tantoangle[slope_div(x as u32, y as u32) as usize])
                }
            } else {
                y = y.wrapping_neg();
                if x > y {
                    0u32.wrapping_sub(tantoangle[slope_div(y as u32, x as u32) as usize])
                } else {
                    ANG270.wrapping_add(tantoangle[slope_div(x as u32, y as u32) as usize])
                }
            }
        } else {
            x = x.wrapping_neg();
            if y >= 0 {
                if x > y {
                    ANG180
                        .wrapping_sub(1)
                        .wrapping_sub(tantoangle[slope_div(y as u32, x as u32) as usize])
                } else {
                    ANG90.wrapping_add(tantoangle[slope_div(x as u32, y as u32) as usize])
                }
            } else {
                y = y.wrapping_neg();
                if x > y {
                    ANG180.wrapping_add(tantoangle[slope_div(y as u32, x as u32) as usize])
                } else {
                    ANG270
                        .wrapping_sub(1)
                        .wrapping_sub(tantoangle[slope_div(x as u32, y as u32) as usize])
                }
            }
        }
    }
}

// --- R_PointToAngle2 ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointToAngle2(x1: i32, y1: i32, x2: i32, y2: i32) -> u32 {
    unsafe {
        viewx = x1;
        viewy = y1;
        R_PointToAngle(x2, y2)
    }
}

// --- R_PointToDist ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointToDist(x: i32, y: i32) -> i32 {
    unsafe {
        let mut dx = x.wrapping_sub(viewx).abs();
        let mut dy = y.wrapping_sub(viewy).abs();

        if dy > dx {
            std::mem::swap(&mut dx, &mut dy);
        }

        if dx == 0 {
            return 0;
        }

        let slope_idx = slope_div(dy as u32, dx as u32);
        let angle = (tantoangle[slope_idx as usize].wrapping_add(ANG90) >> ANGLETOFINESHIFT) as usize;
        fixed_div(dx, finesine[angle])
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn R_InitPointToAngle() {}

// --- R_ScaleFromGlobalAngle ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ScaleFromGlobalAngle(visangle: u32) -> i32 {
    unsafe {
        let anglea = ANG90.wrapping_add(visangle.wrapping_sub(viewangle));
        let angleb = ANG90.wrapping_add(visangle.wrapping_sub(rw_normalangle));

        let sinea = finesine[((anglea >> ANGLETOFINESHIFT) as usize) & crate::tables::FINEMASK];
        let sineb = finesine[((angleb >> ANGLETOFINESHIFT) as usize) & crate::tables::FINEMASK];

let num = fixed_mul(projection, sineb) << detailshift;
let den = fixed_mul(rw_distance, sinea);
let frac_u = FRACUNIT as i32;
if den > (num >> 16) {
let mut scale = fixed_div(num, den);
if scale > 64 * frac_u {
scale = 64 * frac_u;
} else if scale < 256 {
scale = 256;
}
scale
} else {
64 * frac_u
}
}
}
#[unsafe(no_mangle)]
pub extern "C" fn R_InitTables() {}
// --- R_InitTextureMapping ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitTextureMapping() {
unsafe {
let frac_u = FRACUNIT as i32;
let focallength = fixed_div(centerxfrac, finetangent[QUARTER_FINE + FIELDOFVIEW / 2]);
let vtox_ptr = (&raw mut viewangletox) as *mut c_int;
for i in 0..HALF_FINE {
let tan_val = finetangent[i];
let t = if tan_val > frac_u * 2 {
-1
} else if tan_val < -frac_u * 2 {
viewwidth + 1
} else {
let mut val = fixed_mul(tan_val, focallength);
val = (centerxfrac - val + frac_u - 1) >> FRACBITS;
if val < -1 {
-1
} else if val > viewwidth + 1 {
viewwidth + 1
} else {
val
}
};
*vtox_ptr.add(i) = t;
}
let xtv_ptr = (&raw mut xtoviewangle) as *mut u32;
for x in 0..=viewwidth {
let mut i = 0;
while *vtox_ptr.add(i) > x {
i += 1;
}
*xtv_ptr.add(x as usize) = ((i as u32) << ANGLETOFINESHIFT).wrapping_sub(ANG90);
}
for i in 0..HALF_FINE {
let cur = *vtox_ptr.add(i);
if cur == -1 {
*vtox_ptr.add(i) = 0;
} else if cur == viewwidth + 1 {
*vtox_ptr.add(i) = viewwidth;
}
}
clipangle = *xtv_ptr;
}
}
// --- R_InitLightTables ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitLightTables() {
unsafe {
let frac_u = FRACUNIT as i32;
let n_levels = LIGHTLEVELS as c_int;
let n_lightz = MAXLIGHTZ as c_int;
let n_cmaps = NUMCOLORMAPS as c_int;
let zl_ptr = (&raw mut zlight) as *mut [*mut u8; MAXLIGHTZ as usize];
for i in 0..n_levels {
let startmap = ((n_levels - 1 - i) * 2) * n_cmaps / n_levels;
let row_ptr = (*zl_ptr.add(i as usize)).as_mut_ptr();
for j in 0..n_lightz {
let mut scale = fixed_div((SCR_W as i32) / 2 * frac_u, (j + 1) << LIGHTZSHIFT);
scale >>= LIGHTSCALESHIFT;
let mut level = startmap - scale / DISTMAP;
if level < 0 {
level = 0;
}
if level >= n_cmaps {
level = n_cmaps - 1;
}
*row_ptr.add(j as usize) = colormaps.offset((level * 256) as isize);
}
}
}
}
// --- R_SetViewSize ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_SetViewSize(blocks: c_int, detail: c_int) {
unsafe {
setsizeneeded = 1;
setblocks = blocks;
setdetail = detail;
}
}
// --- R_ExecuteSetViewSize ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ExecuteSetViewSize() {
unsafe {
setsizeneeded = 0;
if setblocks == 11 {
scaledviewwidth = SCR_W as c_int;
viewheight = SCR_H as c_int;
} else {
scaledviewwidth = setblocks * 32;
viewheight = (setblocks * 168 / 10) & !7;
}
detailshift = setdetail;
viewwidth = scaledviewwidth >> detailshift;
centery = viewheight / 2;
centerx = viewwidth / 2;
centerxfrac = centerx << FRACBITS;
centeryfrac = centery << FRACBITS;
projection = centerxfrac;
if detailshift == 0 {
basecolfunc = Some(R_DrawColumn);
colfunc = basecolfunc;
fuzzcolfunc = Some(R_DrawFuzzColumn);
transcolfunc = Some(R_DrawTranslatedColumn);
spanfunc = Some(R_DrawSpan);
} else {
basecolfunc = Some(R_DrawColumnLow);
colfunc = basecolfunc;
fuzzcolfunc = Some(R_DrawFuzzColumn);
transcolfunc = Some(R_DrawTranslatedColumn);
spanfunc = Some(R_DrawSpanLow);
}
R_InitBuffer(scaledviewwidth, viewheight);
R_InitTextureMapping();
let frac_u = FRACUNIT as i32;
pspritescale = frac_u * viewwidth / (SCR_W as i32);
pspriteiscale = frac_u * (SCR_W as i32) / viewwidth;
let scr_h_arr = (&raw mut screenheightarray) as *mut i16;
for i in 0..(viewwidth as usize) {
*scr_h_arr.add(i) = viewheight as i16;
}
let yslope_ptr = (&raw mut yslope) as *mut i32;
for i in 0..viewheight {
let mut dy = ((i - viewheight / 2) << FRACBITS) + frac_u / 2;
dy = dy.abs();
*yslope_ptr.add(i as usize) = fixed_div(((viewwidth << detailshift) / 2) * frac_u, dy);
}
let dscale_ptr = (&raw mut distscale) as *mut i32;
let xtv_ptr = (&raw const xtoviewangle) as *const u32;
for i in 0..(viewwidth as usize) {
let idx = (*xtv_ptr.add(i) >> ANGLETOFINESHIFT) as usize;
let cosadj = finesine[idx + QUARTER_FINE].abs();
*dscale_ptr.add(i) = fixed_div(frac_u, cosadj);
}
let n_levels = LIGHTLEVELS as c_int;
let n_lscale = MAXLIGHTSCALE as c_int;
let n_cmaps = NUMCOLORMAPS as c_int;
let sl_ptr = (&raw mut scalelight) as *mut [*mut u8; MAXLIGHTSCALE as usize];
for i in 0..n_levels {
let startmap = ((n_levels - 1 - i) * 2) * n_cmaps / n_levels;
let row_ptr = (*sl_ptr.add(i as usize)).as_mut_ptr();
for j in 0..n_lscale {
let mut level = startmap - j * (SCR_W as i32) / (viewwidth << detailshift) / DISTMAP;
if level < 0 {
level = 0;
}
if level >= n_cmaps {
level = n_cmaps - 1;
}
*row_ptr.add(j as usize) = colormaps.offset((level * 256) as isize);
}
}
}
}
// --- R_Init ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_Init() {
unsafe {
R_InitData();
print!("\nR_InitData");
R_InitPointToAngle();
print!("\nR_InitPointToAngle");
R_InitTables();
print!("\nR_InitTables");
R_SetViewSize(screenblocks, detailLevel);
R_InitPlanes();
print!("\nR_InitPlanes");
R_InitLightTables();
print!("\nR_InitLightTables");
R_InitSkyMap();
print!("\nR_InitSkyMap");
R_InitTranslationTables();
print!("\nR_InitTranslationsTables");
let _ = io::stdout().flush();
framecount = 0;
}
}
// --- R_PointInSubsector ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointInSubsector(x: i32, y: i32) -> *mut subsector_t {
unsafe {
if numnodes == 0 {
return subsectors;
}
let mut nodenum = numnodes - 1;
let nf_sub = NF_SUBSECTOR as c_int;
while (nodenum & nf_sub) == 0 {
let node = nodes.add(nodenum as usize);
let side = R_PointOnSide(x, y, node) as usize;
let children_ptr = (&raw const (*node).children) as *const u16;
nodenum = *children_ptr.add(side) as c_int;
}
subsectors.add((nodenum & !nf_sub) as usize)
}
}
// --- R_SetupFrame ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_SetupFrame(player: *mut player_t) {
unsafe {
viewplayer = player;
let mo = (*player).mo;
viewx = (*mo).x;
viewy = (*mo).y;
viewangle = (*mo).angle.wrapping_add(viewangleoffset as u32);
extralight = (*player).extralight;
viewz = (*player).viewz;
let idx = (viewangle >> ANGLETOFINESHIFT) as usize;
viewsin = finesine[idx];
viewcos = finesine[idx + QUARTER_FINE];
sscount = 0;
if (*player).fixedcolormap != 0 {
fixedcolormap = colormaps.offset(((*player).fixedcolormap * 256) as isize);
let sl_fixed_ptr = (&raw mut scalelightfixed) as *mut *mut u8;
walllights = sl_fixed_ptr;
for i in 0..(MAXLIGHTSCALE as usize) {
*sl_fixed_ptr.add(i) = fixedcolormap;
}
} else {
fixedcolormap = ptr::null_mut();
}
framecount += 1;
validcount += 1;
}
}
// --- R_RenderPlayerView ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_RenderPlayerView(player: *mut player_t) {
unsafe {
R_SetupFrame(player);
R_ClearClipSegs();
R_ClearDrawSegs();
R_ClearPlanes();
R_ClearSprites();
NetUpdate();
R_RenderBSPNode(numnodes - 1);
NetUpdate();
R_DrawPlanes();
NetUpdate();
R_DrawMasked();
NetUpdate();
}
}