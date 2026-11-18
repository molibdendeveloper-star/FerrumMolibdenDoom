#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::c_int;
use std::io::{self, Write};
use std::ptr;

use crate::m_bbox::m_add_to_box;
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
use crate::r_sky::r_init_sky_map;
use crate::r_things::{
    R_ClearSprites, R_DrawMasked, pspriteiscale, pspritescale, screenheightarray,
};
use crate::tables::{
    ANGLETOFINESHIFT, FINEANGLES, FINEMASK, finesine, finetangent, slope_div, tantoangle,
};

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

#[inline]
pub fn r_point_on_side(x: i32, y: i32, node: &node_t) -> c_int {
    let ndx = node.dx;
    let ndy = node.dy;
    let nx = node.x;
    let ny = node.y;

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

#[inline]
pub fn r_point_on_seg_side(x: i32, y: i32, line: &seg_t) -> c_int {
    let (Some(v1), Some(v2)) = (unsafe { line.v1.as_ref() }, unsafe { line.v2.as_ref() }) else {
        return 0;
    };
    let lx = v1.x;
    let ly = v1.y;
    let ldx = v2.x.wrapping_sub(lx);
    let ldy = v2.y.wrapping_sub(ly);

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

#[inline]
pub fn r_point_to_angle_delta(mut x: i32, mut y: i32) -> u32 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_AddPointToBox(x: c_int, y: c_int, box_ptr: *mut i32) {
    if let Some(b) = unsafe { (box_ptr as *mut [i32; 4]).as_mut() } {
        m_add_to_box(b, x, y);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointOnSide(x: i32, y: i32, node: *mut node_t) -> c_int {
    match unsafe { node.as_ref() } {
        Some(n) => r_point_on_side(x, y, n),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointOnSegSide(x: i32, y: i32, line: *mut seg_t) -> c_int {
    match unsafe { line.as_ref() } {
        Some(ln) => r_point_on_seg_side(x, y, ln),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointToAngle(x: i32, y: i32) -> u32 {
    let (vx, vy) = unsafe { (viewx, viewy) };
    r_point_to_angle_delta(x.wrapping_sub(vx), y.wrapping_sub(vy))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointToAngle2(x1: i32, y1: i32, x2: i32, y2: i32) -> u32 {
    unsafe {
        viewx = x1;
        viewy = y1;
    }
    r_point_to_angle_delta(x2.wrapping_sub(x1), y2.wrapping_sub(y1))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointToDist(x: i32, y: i32) -> i32 {
    let (vx, vy) = unsafe { (viewx, viewy) };
    let mut dx = x.wrapping_sub(vx).abs();
    let mut dy = y.wrapping_sub(vy).abs();

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

#[unsafe(no_mangle)]
pub extern "C" fn R_InitPointToAngle() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ScaleFromGlobalAngle(visangle: u32) -> i32 {
    let (v_ang, rw_nang, proj, det, rw_dist) =
        unsafe { (viewangle, rw_normalangle, projection, detailshift, rw_distance) };

    let anglea = ANG90.wrapping_add(visangle.wrapping_sub(v_ang));
    let angleb = ANG90.wrapping_add(visangle.wrapping_sub(rw_nang));

    let sinea = finesine[((anglea >> ANGLETOFINESHIFT) as usize) & FINEMASK];
    let sineb = finesine[((angleb >> ANGLETOFINESHIFT) as usize) & FINEMASK];

    let num = fixed_mul(proj, sineb) << det;
    let den = fixed_mul(rw_dist, sinea);
    let frac_u = FRACUNIT as i32;

    if den > (num >> 16) {
        fixed_div(num, den).clamp(256, 64 * frac_u)
    } else {
        64 * frac_u
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn R_InitTables() {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitTextureMapping() {
unsafe {
let frac_u = FRACUNIT as i32;
let focallength = fixed_div(centerxfrac, finetangent[QUARTER_FINE + FIELDOFVIEW / 2]);
let vtox = (&raw mut viewangletox).as_mut().unwrap();
let vw = viewwidth;
for (i, slot) in vtox.iter_mut().enumerate() {
let tan_val = finetangent[i];
*slot = if tan_val > frac_u * 2 {
-1
} else if tan_val < -frac_u * 2 {
vw + 1
} else {
let val = (centerxfrac - fixed_mul(tan_val, focallength) + frac_u - 1) >> FRACBITS;
val.clamp(-1, vw + 1)
};
}
let xtv = (&raw mut xtoviewangle).as_mut().unwrap();
for x in 0..=vw {
let mut i = 0;
while vtox[i] > x {
i += 1;
}
xtv[x as usize] = ((i as u32) << ANGLETOFINESHIFT).wrapping_sub(ANG90);
}
for slot in vtox.iter_mut() {
if *slot == -1 {
*slot = 0;
} else if *slot == vw + 1 {
*slot = vw;
}
}
clipangle = xtv[0];
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitLightTables() {
unsafe {
let frac_u = FRACUNIT as i32;
let n_levels = LIGHTLEVELS as c_int;
let n_cmaps = NUMCOLORMAPS as c_int;
let zl = (&raw mut zlight).as_mut().unwrap();
for (i, row) in zl.iter_mut().enumerate() {
let startmap = ((n_levels - 1 - (i as c_int)) * 2) * n_cmaps / n_levels;
for (j, slot) in row.iter_mut().enumerate() {
let mut scale =
fixed_div((SCR_W as i32) / 2 * frac_u, ((j as c_int) + 1) << LIGHTZSHIFT);
scale >>= LIGHTSCALESHIFT;
let level = (startmap - scale / DISTMAP).clamp(0, n_cmaps - 1);
*slot = colormaps.offset((level * 256) as isize);
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_SetViewSize(blocks: c_int, detail: c_int) {
unsafe {
setsizeneeded = 1;
setblocks = blocks;
setdetail = detail;
}
}
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
basecolfunc = Some(if detailshift == 0 {
R_DrawColumn
} else {
R_DrawColumnLow
});
colfunc = basecolfunc;
fuzzcolfunc = Some(R_DrawFuzzColumn);
transcolfunc = Some(R_DrawTranslatedColumn);
spanfunc = Some(if detailshift == 0 {
R_DrawSpan
} else {
R_DrawSpanLow
});
R_InitBuffer(scaledviewwidth, viewheight);
R_InitTextureMapping();
let frac_u = FRACUNIT as i32;
let vw = viewwidth;
let vh = viewheight;
pspritescale = frac_u * vw / (SCR_W as i32);
pspriteiscale = frac_u * (SCR_W as i32) / vw;
let scr_h_arr = (&raw mut screenheightarray).as_mut().unwrap();
scr_h_arr[..(vw as usize)].fill(vh as i16);
let yslope_arr = (&raw mut yslope).as_mut().unwrap();
for (i, slot) in yslope_arr[..(vh as usize)].iter_mut().enumerate() {
let dy = (((i as c_int) - vh / 2) << FRACBITS) + frac_u / 2;
*slot = fixed_div(((vw << detailshift) / 2) * frac_u, dy.abs());
}
let dscale_arr = (&raw mut distscale).as_mut().unwrap();
let xtv = (&raw const xtoviewangle).as_ref().unwrap();
for i in 0..(vw as usize) {
let idx = (xtv[i] >> ANGLETOFINESHIFT) as usize;
let cosadj = finesine[idx + QUARTER_FINE].abs();
dscale_arr[i] = fixed_div(frac_u, cosadj);
}
let n_levels = LIGHTLEVELS as c_int;
let n_cmaps = NUMCOLORMAPS as c_int;
let sl = (&raw mut scalelight).as_mut().unwrap();
for (i, row) in sl.iter_mut().enumerate() {
let startmap = ((n_levels - 1 - (i as c_int)) * 2) * n_cmaps / n_levels;
for (j, slot) in row.iter_mut().enumerate() {
let level = (startmap - (j as c_int) * (SCR_W as i32) / (vw << detailshift) / DISTMAP)
.clamp(0, n_cmaps - 1);
*slot = colormaps.offset((level * 256) as isize);
}
}
}
}
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
r_init_sky_map();
print!("\nR_InitSkyMap");
R_InitTranslationTables();
print!("\nR_InitTranslationsTables");
let _ = io::stdout().flush();
framecount = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PointInSubsector(x: i32, y: i32) -> *mut subsector_t {
unsafe {
if numnodes == 0 {
return subsectors;
}
let mut nodenum = numnodes - 1;
let nf_sub = NF_SUBSECTOR as c_int;
let nodes_slice = std::slice::from_raw_parts(nodes, numnodes as usize);
while (nodenum & nf_sub) == 0 {
let node = &nodes_slice[nodenum as usize];
let side = r_point_on_side(x, y, node) as usize;
nodenum = node.children[side] as c_int;
}
subsectors.add((nodenum & !nf_sub) as usize)
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_SetupFrame(player: *mut player_t) {
let Some(p) = (unsafe { player.as_ref() }) else { return };
unsafe {
viewplayer = player;
let mo = &*p.mo;
viewx = mo.x;
viewy = mo.y;
viewangle = mo.angle.wrapping_add(viewangleoffset as u32);
extralight = p.extralight;
viewz = p.viewz;
let idx = (viewangle >> ANGLETOFINESHIFT) as usize;
viewsin = finesine[idx];
viewcos = finesine[idx + QUARTER_FINE];
sscount = 0;
if p.fixedcolormap != 0 {
fixedcolormap = colormaps.offset((p.fixedcolormap * 256) as isize);
let sl_fixed = (&raw mut scalelightfixed).as_mut().unwrap();
sl_fixed.fill(fixedcolormap);
walllights = sl_fixed.as_mut_ptr();
} else {
fixedcolormap = ptr::null_mut();
}
framecount += 1;
validcount += 1;
}
}
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
