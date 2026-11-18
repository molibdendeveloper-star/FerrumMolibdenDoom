#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int, c_short, c_void};
use std::ptr;

use crate::m_fixed::{fixed_div, fixed_mul};
use crate::r_bsp::{drawsegs, ds_p};
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};
use crate::w_wad::W_CacheLumpNum;
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_ChangeTag2};

pub const MAXVISPLANES: usize = 128;
const SCR_W: usize = SCREENWIDTH as usize;
const SCR_H: usize = SCREENHEIGHT as usize;
pub const MAXOPENINGS: usize = SCR_W * 64;

pub type planefunction_t = Option<unsafe extern "C" fn()>;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn R_GetColumn(tex: c_int, col: c_int) -> *mut u8;

    static mut viewwidth: c_int;
    static mut viewheight: c_int;
    static mut viewx: i32;
    static mut viewy: i32;
    static mut viewz: i32;
    static mut viewangle: u32;
    static mut centerxfrac: i32;
    static mut detailshift: c_int;
    static mut extralight: c_int;

    static mut skyflatnum: c_int;
    static mut skytexture: c_int;
    static mut skytexturemid: i32;
    static mut pspriteiscale: i32;

    static mut firstflat: c_int;
    static mut flattranslation: *mut c_int;
    static mut colormaps: *mut u8;
    static mut fixedcolormap: *mut u8;
    static mut zlight: [[*mut u8; MAXLIGHTZ as usize]; LIGHTLEVELS as usize];

    static mut xtoviewangle: [u32; SCR_W + 1];

    // Переменные отрисовки спанов и столбцов (r_draw.c)
    static mut ds_y: c_int;
    static mut ds_x1: c_int;
    static mut ds_x2: c_int;
    static mut ds_xfrac: i32;
    static mut ds_yfrac: i32;
    static mut ds_xstep: i32;
    static mut ds_ystep: i32;
    static mut ds_source: *mut u8;
    static mut ds_colormap: *mut u8;

    static mut dc_x: c_int;
    static mut dc_yl: c_int;
    static mut dc_yh: c_int;
    static mut dc_iscale: i32;
    static mut dc_texturemid: i32;
    static mut dc_source: *mut u8;
    static mut dc_colormap: *mut u8;

    static mut spanfunc: Option<unsafe extern "C" fn()>;
    static mut colfunc: Option<unsafe extern "C" fn()>;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Plane error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные r_plane.c ---
#[unsafe(no_mangle)]
pub static mut floorfunc: planefunction_t = None;

#[unsafe(no_mangle)]
pub static mut ceilingfunc: planefunction_t = None;

#[unsafe(no_mangle)]
pub static mut visplanes: [visplane_t; MAXVISPLANES] =
    [unsafe { std::mem::zeroed() }; MAXVISPLANES];

#[unsafe(no_mangle)]
pub static mut lastvisplane: *mut visplane_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut floorplane: *mut visplane_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut ceilingplane: *mut visplane_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut openings: [c_short; MAXOPENINGS] = [0; MAXOPENINGS];

#[unsafe(no_mangle)]
pub static mut lastopening: *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut floorclip: [c_short; SCR_W] = [0; SCR_W];

#[unsafe(no_mangle)]
pub static mut ceilingclip: [c_short; SCR_W] = [0; SCR_W];

#[unsafe(no_mangle)]
pub static mut spanstart: [c_int; SCR_H] = [0; SCR_H];

#[unsafe(no_mangle)]
pub static mut spanstop: [c_int; SCR_H] = [0; SCR_H];

#[unsafe(no_mangle)]
pub static mut planezlight: *mut *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut planeheight: i32 = 0;

#[unsafe(no_mangle)]
pub static mut yslope: [i32; SCR_H] = [0; SCR_H];

#[unsafe(no_mangle)]
pub static mut distscale: [i32; SCR_W] = [0; SCR_W];

#[unsafe(no_mangle)]
pub static mut basexscale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut baseyscale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut cachedheight: [i32; SCR_H] = [0; SCR_H];

#[unsafe(no_mangle)]
pub static mut cacheddistance: [i32; SCR_H] = [0; SCR_H];

#[unsafe(no_mangle)]
pub static mut cachedxstep: [i32; SCR_H] = [0; SCR_H];

#[unsafe(no_mangle)]
pub static mut cachedystep: [i32; SCR_H] = [0; SCR_H];

#[unsafe(no_mangle)]
pub extern "C" fn R_InitPlanes() {
    // Doh!
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_MapPlane(y: c_int, x1: c_int, x2: c_int) {
    unsafe {
        if x2 < x1 || x1 < 0 || x2 >= viewwidth || (y as u32) > (viewheight as u32) {
            doom_error(&format!("R_MapPlane: {}, {} at {}", x1, x2, y));
        }

        let yu = y as usize;
        let ch_ptr = (&raw mut cachedheight) as *mut i32;
        let cd_ptr = (&raw mut cacheddistance) as *mut i32;
        let cx_ptr = (&raw mut cachedxstep) as *mut i32;
        let cy_ptr = (&raw mut cachedystep) as *mut i32;
        let ys_ptr = (&raw const yslope) as *const i32;

        let distance = if planeheight != *ch_ptr.add(yu) {
            *ch_ptr.add(yu) = planeheight;
            let dist = fixed_mul(planeheight, *ys_ptr.add(yu));
            *cd_ptr.add(yu) = dist;
            ds_xstep = fixed_mul(dist, basexscale);
            *cx_ptr.add(yu) = ds_xstep;
            ds_ystep = fixed_mul(dist, baseyscale);
            *cy_ptr.add(yu) = ds_ystep;
            dist
        } else {
            ds_xstep = *cx_ptr.add(yu);
            ds_ystep = *cy_ptr.add(yu);
            *cd_ptr.add(yu)
        };

        let ds_ptr = (&raw const distscale) as *const i32;
        let length = fixed_mul(distance, *ds_ptr.add(x1 as usize));

        let xtv_ptr = (&raw const xtoviewangle) as *const u32;
        let angle = (viewangle.wrapping_add(*xtv_ptr.add(x1 as usize)) >> ANGLETOFINESHIFT) as usize;

        // finecosine[angle] == finesine[angle + FINEANGLES / 4]
        let finecos = finesine[angle + (FINEANGLES / 4)];
        let finesin = finesine[angle];

        ds_xfrac = viewx.wrapping_add(fixed_mul(finecos, length));
        ds_yfrac = (-viewy).wrapping_sub(fixed_mul(finesin, length));

        if !fixedcolormap.is_null() {
            ds_colormap = fixedcolormap;
        } else {
            let mut index = (distance as u32) >> LIGHTZSHIFT;
            if index >= MAXLIGHTZ {
                index = MAXLIGHTZ - 1;
            }
            ds_colormap = *planezlight.add(index as usize);
        }

        ds_y = y;
        ds_x1 = x1;
        ds_x2 = x2;

        if let Some(func) = spanfunc {
            func();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ClearPlanes() {
    unsafe {
        let fc_ptr = (&raw mut floorclip) as *mut c_short;
        let cc_ptr = (&raw mut ceilingclip) as *mut c_short;

        for i in 0..(viewwidth as usize) {
            *fc_ptr.add(i) = viewheight as c_short;
            *cc_ptr.add(i) = -1;
        }

        lastvisplane = (&raw mut visplanes) as *mut visplane_t;
        lastopening = (&raw mut openings) as *mut c_short;

        ptr::write_bytes((&raw mut cachedheight) as *mut u8, 0, SCR_H * std::mem::size_of::<i32>());

        let angle = (viewangle.wrapping_sub(ANG90) >> ANGLETOFINESHIFT) as usize;
        let finecos = finesine[angle + (FINEANGLES / 4)];
        let finesin = finesine[angle];

        basexscale = fixed_div(finecos, centerxfrac);
        baseyscale = -fixed_div(finesin, centerxfrac);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_FindPlane(
    mut height: i32,
    picnum: c_int,
    mut lightlevel: c_int,
) -> *mut visplane_t {
    unsafe {
        if picnum == skyflatnum {
            height = 0;
            lightlevel = 0;
        }

        let vp_base = (&raw mut visplanes) as *mut visplane_t;
        let mut check = vp_base;

        while check < lastvisplane {
            if height == (*check).height
                && picnum == (*check).picnum
                && lightlevel == (*check).lightlevel
            {
                return check;
            }
            check = check.add(1);
        }

        if lastvisplane.offset_from(vp_base) as usize == MAXVISPLANES {
            doom_error("R_FindPlane: no more visplanes");
        }

        lastvisplane = lastvisplane.add(1);

        (*check).height = height;
        (*check).picnum = picnum;
        (*check).lightlevel = lightlevel;
        (*check).minx = SCREENWIDTH as c_int;
        (*check).maxx = -1;

        let top_ptr = (&raw mut (*check).top) as *mut u8;
        ptr::write_bytes(top_ptr, 0xff, std::mem::size_of_val(&(*check).top));

        check
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_CheckPlane(
    mut pl: *mut visplane_t,
    start: c_int,
    stop: c_int,
) -> *mut visplane_t {
    unsafe {
        let (intrl, unionl) = if start < (*pl).minx {
            ((*pl).minx, start)
        } else {
            (start, (*pl).minx)
        };

        let (intrh, unionh) = if stop > (*pl).maxx {
            ((*pl).maxx, stop)
        } else {
            (stop, (*pl).maxx)
        };

        let top_ptr = (&raw const (*pl).top) as *const u8;
        let mut x = intrl;
        while x <= intrh {
            if *top_ptr.add(x as usize) != 0xff {
                break;
            }
            x += 1;
        }

        if x > intrh {
            (*pl).minx = unionl;
            (*pl).maxx = unionh;
            return pl;
        }

        (*lastvisplane).height = (*pl).height;
        (*lastvisplane).picnum = (*pl).picnum;
        (*lastvisplane).lightlevel = (*pl).lightlevel;

        pl = lastvisplane;
        lastvisplane = lastvisplane.add(1);

        (*pl).minx = start;
        (*pl).maxx = stop;

        let new_top_ptr = (&raw mut (*pl).top) as *mut u8;
        ptr::write_bytes(new_top_ptr, 0xff, std::mem::size_of_val(&(*pl).top));

        pl
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_MakeSpans(
    x: c_int,
    mut t1: c_int,
    mut b1: c_int,
    mut t2: c_int,
    mut b2: c_int,
) {
    unsafe {
        let ss_ptr = (&raw mut spanstart) as *mut c_int;

        while t1 < t2 && t1 <= b1 {
R_MapPlane(t1, *ss_ptr.add(t1 as usize), x - 1);
t1 += 1;
}
while b1 > b2 && b1 >= t1 {
R_MapPlane(b1, *ss_ptr.add(b1 as usize), x - 1);
b1 -= 1;
}
while t2 < t1 && t2 <= b2 {
*ss_ptr.add(t2 as usize) = x;
t2 += 1;
}
while b2 > b1 && b2 >= t2 {
*ss_ptr.add(b2 as usize) = x;
b2 -= 1;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawPlanes() {
unsafe {
let ds_base = (&raw const drawsegs) as *const crate::r_bsp::drawseg_t;
let vp_base = (&raw mut visplanes) as *mut visplane_t;
let op_base = (&raw const openings) as *const c_short;
if ds_p.offset_from(ds_base) > MAXDRAWSEGS as isize {
doom_error(&format!(
"R_DrawPlanes: drawsegs overflow ({})",
ds_p.offset_from(ds_base)
));
}
if lastvisplane.offset_from(vp_base) > MAXVISPLANES as isize {
doom_error(&format!(
"R_DrawPlanes: visplane overflow ({})",
lastvisplane.offset_from(vp_base)
));
}
if lastopening.offset_from(op_base) > MAXOPENINGS as isize {
doom_error(&format!(
"R_DrawPlanes: opening overflow ({})",
lastopening.offset_from(op_base)
));
}
let mut pl = vp_base;
while pl < lastvisplane {
if (*pl).minx > (*pl).maxx {
pl = pl.add(1);
continue;
}
let top_ptr = (&raw mut (*pl).top) as *mut u8;
let bot_ptr = (&raw const (*pl).bottom) as *const u8;
// Отрисовка неба
if (*pl).picnum == skyflatnum {
dc_iscale = pspriteiscale >> detailshift;
dc_colormap = colormaps;
dc_texturemid = skytexturemid;
let xtv_ptr = (&raw const xtoviewangle) as *const u32;
for x in (*pl).minx..=(*pl).maxx {
let xu = x as usize;
dc_yl = *top_ptr.add(xu) as c_int;
dc_yh = *bot_ptr.add(xu) as c_int;
if dc_yl <= dc_yh {
let angle = (viewangle.wrapping_add(*xtv_ptr.add(xu)) >> ANGLETOSKYSHIFT) as c_int;
dc_x = x;
dc_source = R_GetColumn(skytexture, angle);
if let Some(func) = colfunc {
func();
}
}
}
pl = pl.add(1);
continue;
}
// Отрисовка обычного пола или потолка
let flat_idx = firstflat + *flattranslation.add((*pl).picnum as usize);
ds_source = W_CacheLumpNum(flat_idx, PU_STATIC) as *mut u8;
planeheight = ((*pl).height - viewz).abs();
let mut light = ((*pl).lightlevel >> LIGHTSEGSHIFT) + extralight;
if light >= LIGHTLEVELS as c_int {
light = (LIGHTLEVELS as c_int) - 1;
}
if light < 0 {
light = 0;
}
let zlight_ptr = (&raw const zlight) as *const [*mut u8; MAXLIGHTZ as usize];
planezlight = (*zlight_ptr.add(light as usize)).as_ptr() as *mut *mut u8;
let minx = (*pl).minx;
let maxx = (*pl).maxx;
*top_ptr.offset((maxx + 1) as isize) = 0xff;
*top_ptr.offset((minx - 1) as isize) = 0xff;
let stop = maxx + 1;
for x in minx..=stop {
let prev = (x - 1) as isize;
let curr = x as isize;
R_MakeSpans(
x,
*top_ptr.offset(prev) as c_int,
*bot_ptr.offset(prev) as c_int,
*top_ptr.offset(curr) as c_int,
*bot_ptr.offset(curr) as c_int,
);
}
Z_ChangeTag2(ds_source as *mut c_void, PU_CACHE);
pl = pl.add(1);
}
}
}