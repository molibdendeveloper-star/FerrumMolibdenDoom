#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_sky::{get_sky_flat_num, get_sky_texture, get_sky_texture_mid};
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
    static mut pspriteiscale: i32;

    static mut firstflat: c_int;
    static mut flattranslation: *mut c_int;
    static mut colormaps: *mut u8;
    static mut fixedcolormap: *mut u8;
    static mut zlight: [[*mut u8; MAXLIGHTZ as usize]; LIGHTLEVELS as usize];

    static mut xtoviewangle: [u32; SCR_W + 1];

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

const EMPTY_VISPLANE: visplane_t = visplane_t {
    height: 0,
    picnum: 0,
    lightlevel: 0,
    minx: 0,
    maxx: 0,
    pad1: 0,
    top: [0; 320],
    pad2: 0,
    pad3: 0,
    bottom: [0; 320],
    pad4: 0,
};

#[unsafe(no_mangle)]
pub static mut floorfunc: planefunction_t = None;

#[unsafe(no_mangle)]
pub static mut ceilingfunc: planefunction_t = None;

#[unsafe(no_mangle)]
pub static mut visplanes: [visplane_t; MAXVISPLANES] = [EMPTY_VISPLANE; MAXVISPLANES];

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
pub extern "C" fn R_InitPlanes() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_MapPlane(y: c_int, x1: c_int, x2: c_int) {
    unsafe {
        if x2 < x1 || x1 < 0 || x2 >= viewwidth || (y as u32) > (viewheight as u32) {
            doom_error(&format!("R_MapPlane: {}, {} at {}", x1, x2, y));
        }

        let yu = y as usize;
        let ch = &mut *(&raw mut cachedheight);
        let cd = &mut *(&raw mut cacheddistance);
        let cx = &mut *(&raw mut cachedxstep);
        let cy = &mut *(&raw mut cachedystep);
        let ys = &*(&raw const yslope);

        let distance = if planeheight != ch[yu] {
            ch[yu] = planeheight;
            let dist = fixed_mul(planeheight, ys[yu]);
            cd[yu] = dist;
            ds_xstep = fixed_mul(dist, basexscale);
            cx[yu] = ds_xstep;
            ds_ystep = fixed_mul(dist, baseyscale);
            cy[yu] = ds_ystep;
            dist
        } else {
            ds_xstep = cx[yu];
            ds_ystep = cy[yu];
            cd[yu]
        };

        let ds_arr = &*(&raw const distscale);
        let length = fixed_mul(distance, ds_arr[x1 as usize]);

        let xtv = (&raw const xtoviewangle).as_ref().unwrap();
        let angle = (viewangle.wrapping_add(xtv[x1 as usize]) >> ANGLETOFINESHIFT) as usize;

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
        let vw = viewwidth as usize;
        let vh = viewheight as c_short;
        let fc = &mut *(&raw mut floorclip);
        let cc = &mut *(&raw mut ceilingclip);

        fc[..vw].fill(vh);
        cc[..vw].fill(-1);

        lastvisplane = (&raw mut visplanes) as *mut visplane_t;
        lastopening = (&raw mut openings) as *mut c_short;

        (&mut *(&raw mut cachedheight)).fill(0);

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
        if picnum == get_sky_flat_num() {
            height = 0;
            lightlevel = 0;
        }

        let vp_base = (&raw mut visplanes) as *mut visplane_t;
        let used_count = lastvisplane.offset_from(vp_base) as usize;
        let active_vps = std::slice::from_raw_parts_mut(vp_base, used_count);

        for vp in active_vps {
            if height == vp.height && picnum == vp.picnum && lightlevel == vp.lightlevel {
                return vp as *mut visplane_t;
            }
        }

        if used_count >= MAXVISPLANES {
            doom_error("R_FindPlane: no more visplanes");
        }

        let new_vp = &mut *lastvisplane;
        lastvisplane = lastvisplane.add(1);

        new_vp.height = height;
        new_vp.picnum = picnum;
        new_vp.lightlevel = lightlevel;
        new_vp.minx = SCREENWIDTH as c_int;
        new_vp.maxx = -1;
        new_vp.top.fill(0xff);

        new_vp as *mut visplane_t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_CheckPlane(
    mut pl: *mut visplane_t,
    start: c_int,
    stop: c_int,
) -> *mut visplane_t {
    unsafe {
        let cur = &mut *pl;
        let (intrl, unionl) = if start < cur.minx {
            (cur.minx, start)
        } else {
            (start, cur.minx)
        };

        let (intrh, unionh) = if stop > cur.maxx {
            (cur.maxx, stop)
        } else {
            (stop, cur.maxx)
        };

        let has_overlap = intrl <= intrh && cur.top[(intrl as usize)..=(intrh as usize)]
            .iter()
            .any(|&v| v != 0xff);

        if !has_overlap {
            cur.minx = unionl;
            cur.maxx = unionh;
            return pl;
        }

        let next_vp = &mut *lastvisplane;
        next_vp.height = cur.height;
        next_vp.picnum = cur.picnum;
        next_vp.lightlevel = cur.lightlevel;

        pl = lastvisplane;
        lastvisplane = lastvisplane.add(1);

        next_vp.minx = start;
        next_vp.maxx = stop;
        next_vp.top.fill(0xff);

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
        let ss = &mut *(&raw mut spanstart);

        while t1 < t2 && t1 <= b1 {
            R_MapPlane(t1, ss[t1 as usize], x - 1);
            t1 += 1;
        }
        while b1 > b2 && b1 >= t1 {
            R_MapPlane(b1, ss[b1 as usize], x - 1);
            b1 -= 1;
        }
        while t2 < t1 && t2 <= b2 {
            ss[t2 as usize] = x;
            t2 += 1;
        }
        while b2 > b1 && b2 >= t2 {
            ss[b2 as usize] = x;
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
let vp_count = lastvisplane.offset_from(vp_base).max(0) as usize;
let vps = std::slice::from_raw_parts_mut(vp_base, vp_count);
let xtv = (&raw const xtoviewangle).as_ref().unwrap();
let zlight_arr = (&raw const zlight).as_ref().unwrap();
for vp in vps {
if vp.minx > vp.maxx {
continue;
}
if vp.picnum == get_sky_flat_num() {
dc_iscale = pspriteiscale >> detailshift;
dc_colormap = colormaps;
dc_texturemid = get_sky_texture_mid();
for x in vp.minx..=vp.maxx {
let xu = x as usize;
dc_yl = vp.top[xu] as c_int;
dc_yh = vp.bottom[xu] as c_int;
if dc_yl <= dc_yh {
let angle = (viewangle.wrapping_add(xtv[xu]) >> ANGLETOSKYSHIFT) as c_int;
dc_x = x;
dc_source = R_GetColumn(get_sky_texture(), angle);
if let Some(func) = colfunc {
func();
}
}
}
continue;
}
let flat_idx = firstflat + *flattranslation.add(vp.picnum as usize);
ds_source = W_CacheLumpNum(flat_idx, PU_STATIC) as *mut u8;
planeheight = (vp.height - viewz).abs();
let light = (((vp.lightlevel >> LIGHTSEGSHIFT) + extralight)
.clamp(0, (LIGHTLEVELS as c_int) - 1)) as usize;
planezlight = zlight_arr[light].as_ptr() as *mut *mut u8;
let minx = vp.minx;
let maxx = vp.maxx;
let top_ptr = vp.top.as_mut_ptr();
let bot_ptr = vp.bottom.as_ptr();
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
}
}
}
