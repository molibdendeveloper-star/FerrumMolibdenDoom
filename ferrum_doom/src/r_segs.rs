#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_sky::get_sky_flat_num;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int, c_short};
use std::ptr;

use crate::m_fixed::{FRACBITS, fixed_mul};
use crate::r_bsp::{backsector, curline, drawsegs, ds_p, frontsector, linedef, sidedef};
use crate::r_plane::{R_CheckPlane, ceilingclip, ceilingplane, floorclip, floorplane, lastopening};
use crate::tables::{ANGLETOFINESHIFT, finesine, finetangent};
use crate::v_video::column_t;

#[repr(C)]
pub struct line_s {
    pub v1: *mut vertex_t,
    pub v2: *mut vertex_t,
    pub dx: i32,
    pub dy: i32,
    pub flags: i16,
    pub special: i16,
    pub tag: i16,
    pub sidenum: [i16; 2],
    pub bbox: [i32; 4],
    pub slopetype: slopetype_t,
    pub frontsector: *mut sector_t,
    pub backsector: *mut sector_t,
    pub validcount: c_int,
    pub specialdata: *mut std::ffi::c_void,
}

const HEIGHTBITS: i32 = 12;
const HEIGHTUNIT: i32 = 1 << HEIGHTBITS;
const MAXSHORT: c_short = i16::MAX;
const SCR_W: usize = SCREENWIDTH as usize;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn R_GetColumn(tex: c_int, col: c_int) -> *mut u8;
    fn R_DrawMaskedColumn(column: *mut column_t);
    fn R_PointToDist(x: i32, y: i32) -> i32;
    fn R_ScaleFromGlobalAngle(visangle: u32) -> i32;

    static mut viewwidth: c_int;
    static mut viewheight: c_int;
    static mut viewx: i32;
    static mut viewy: i32;
    static mut viewz: i32;
    static mut viewangle: u32;
    static mut centeryfrac: i32;
    static mut extralight: c_int;

    static mut texturetranslation: *mut c_int;
    static mut textureheight: *mut i32;
    static mut scalelight: [[*mut u8; MAXLIGHTSCALE as usize]; LIGHTLEVELS as usize];
    static mut fixedcolormap: *mut u8;
    static mut xtoviewangle: [u32; SCR_W + 1];

    static mut spryscale: i32;
    static mut sprtopscreen: i32;
    static mut mfloorclip: *mut c_short;
    static mut mceilingclip: *mut c_short;
    static mut screenheightarray: [c_short; SCR_W];
    static mut negonearray: [c_short; SCR_W];

    static mut dc_x: c_int;
    static mut dc_yl: c_int;
    static mut dc_yh: c_int;
    static mut dc_iscale: i32;
    static mut dc_texturemid: i32;
    static mut dc_source: *mut u8;
    static mut dc_colormap: *mut u8;
    static mut colfunc: Option<unsafe extern "C" fn()>;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Segs error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

#[unsafe(no_mangle)]
pub static mut segtextured: c_int = 0;

#[unsafe(no_mangle)]
pub static mut markfloor: c_int = 0;

#[unsafe(no_mangle)]
pub static mut markceiling: c_int = 0;

#[unsafe(no_mangle)]
pub static mut maskedtexture: c_int = 0;

#[unsafe(no_mangle)]
pub static mut toptexture: c_int = 0;

#[unsafe(no_mangle)]
pub static mut bottomtexture: c_int = 0;

#[unsafe(no_mangle)]
pub static mut midtexture: c_int = 0;

#[unsafe(no_mangle)]
pub static mut rw_normalangle: u32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_angle1: c_int = 0;

#[unsafe(no_mangle)]
pub static mut rw_x: c_int = 0;

#[unsafe(no_mangle)]
pub static mut rw_stopx: c_int = 0;

#[unsafe(no_mangle)]
pub static mut rw_centerangle: u32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_offset: i32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_distance: i32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_scale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_scalestep: i32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_midtexturemid: i32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_toptexturemid: i32 = 0;

#[unsafe(no_mangle)]
pub static mut rw_bottomtexturemid: i32 = 0;

#[unsafe(no_mangle)]
pub static mut worldtop: c_int = 0;

#[unsafe(no_mangle)]
pub static mut worldbottom: c_int = 0;

#[unsafe(no_mangle)]
pub static mut worldhigh: c_int = 0;

#[unsafe(no_mangle)]
pub static mut worldlow: c_int = 0;

#[unsafe(no_mangle)]
pub static mut pixhigh: i32 = 0;

#[unsafe(no_mangle)]
pub static mut pixlow: i32 = 0;

#[unsafe(no_mangle)]
pub static mut pixhighstep: i32 = 0;

#[unsafe(no_mangle)]
pub static mut pixlowstep: i32 = 0;

#[unsafe(no_mangle)]
pub static mut topfrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut topstep: i32 = 0;

#[unsafe(no_mangle)]
pub static mut bottomfrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut bottomstep: i32 = 0;

#[unsafe(no_mangle)]
pub static mut walllights: *mut *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut maskedtexturecol: *mut c_short = ptr::null_mut();

#[inline]
unsafe fn select_wall_lights(fsec: &sector_t, v1: &vertex_t, v2: &vertex_t) -> *mut *mut u8 {
    unsafe {
        let mut lightnum = ((fsec.lightlevel as c_int) >> LIGHTSEGSHIFT) + extralight;
        if v1.y == v2.y {
            lightnum -= 1;
        } else if v1.x == v2.x {
            lightnum += 1;
        }
        let clamped = lightnum.clamp(0, (LIGHTLEVELS as c_int) - 1) as usize;
        let sl = (&raw const scalelight).as_ref().unwrap();
        sl[clamped].as_ptr() as *mut *mut u8
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_RenderMaskedSegRange(ds: *mut drawseg_t, x1: c_int, x2: c_int) {
    let Some(ds_ref) = (unsafe { ds.as_mut() }) else { return };
    unsafe {
        let cl = ds_ref.curline as *mut seg_t;
        curline = cl as *mut crate::r_bsp::seg_t;
        let fsec = &*( (*cl).frontsector as *const sector_t );
        let bsec = &*( (*cl).backsector as *const sector_t );
        frontsector = (*cl).frontsector as *mut crate::r_bsp::sector_t;
        backsector = (*cl).backsector as *mut crate::r_bsp::sector_t;

        let sdef = &*(*cl).sidedef;
        let texnum = *texturetranslation.add(sdef.midtexture as usize);

        walllights = select_wall_lights(fsec, &*(*cl).v1, &*(*cl).v2);

        maskedtexturecol = ds_ref.maskedtexturecol;
        rw_scalestep = ds_ref.scalestep;
        spryscale = ds_ref.scale1 + (x1 - ds_ref.x1) * rw_scalestep;
        mfloorclip = ds_ref.sprbottomclip;
        mceilingclip = ds_ref.sprtopclip;

        let ldef = &*(*cl).linedef;
        if (ldef.flags as u32 & ML_DONTPEGBOTTOM) != 0 {
            let base_h = fsec.floorheight.max(bsec.floorheight);
            dc_texturemid = base_h + *textureheight.add(texnum as usize) - viewz;
        } else {
            let base_h = fsec.ceilingheight.min(bsec.ceilingheight);
            dc_texturemid = base_h - viewz;
        }
        dc_texturemid += sdef.rowoffset;

        if !fixedcolormap.is_null() {
            dc_colormap = fixedcolormap;
        }

        for x in x1..=x2 {
            dc_x = x;
            let col_slot = maskedtexturecol.add(dc_x as usize);
            if *col_slot != MAXSHORT {
                if fixedcolormap.is_null() {
                    let index = ((spryscale as u32) >> LIGHTSCALESHIFT).min(MAXLIGHTSCALE - 1) as usize;
                    dc_colormap = *walllights.add(index);
                }

                sprtopscreen = centeryfrac - fixed_mul(dc_texturemid, spryscale);
                dc_iscale = (0xffffffffu32 / (spryscale as u32)) as i32;

                let col_data = R_GetColumn(texnum, *col_slot as c_int);
                let col = col_data.sub(3) as *mut column_t;
                R_DrawMaskedColumn(col);
                *col_slot = MAXSHORT;
            }
            spryscale += rw_scalestep;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_RenderSegLoop() {
    unsafe {
        let fclip = (&raw mut floorclip).as_mut().unwrap();
        let cclip = (&raw mut ceilingclip).as_mut().unwrap();
        let xtv = (&raw const xtoviewangle).as_ref().unwrap();
        let mut texturecolumn: i32 = 0;

        while rw_x < rw_stopx {
            let rx = rw_x as usize;
            let mut yl = (topfrac + HEIGHTUNIT - 1) >> HEIGHTBITS;

            let cclip_val = cclip[rx] as c_int;
            let fclip_val = fclip[rx] as c_int;

            if yl < cclip_val + 1 {
                yl = cclip_val + 1;
            }

            if markceiling != 0 {
                let top = cclip_val + 1;
                let bottom = (yl - 1).min(fclip_val - 1);

                if top <= bottom {
                    let cpl = &mut *(ceilingplane as *mut visplane_t);
                    cpl.top[rx] = top as u8;
                    cpl.bottom[rx] = bottom as u8;
                }
            }

            let yh = (bottomfrac >> HEIGHTBITS).min(fclip_val - 1);

            if markfloor != 0 {
                let top = (yh + 1).max(cclip_val + 1);
                let bottom = fclip_val - 1;
                if top <= bottom {
                    let fpl = &mut *(floorplane as *mut visplane_t);
                    fpl.top[rx] = top as u8;
                    fpl.bottom[rx] = bottom as u8;
                }
            }

            if segtextured != 0 {
                let angle = (rw_centerangle.wrapping_add(xtv[rx]) >> ANGLETOFINESHIFT) as usize;
                let tan_val = if angle >= 4096 { finetangent[0] } else { finetangent[angle] };
                texturecolumn = (rw_offset - fixed_mul(tan_val, rw_distance)) >> FRACBITS;

                let index = ((rw_scale as u32) >> LIGHTSCALESHIFT).min(MAXLIGHTSCALE - 1) as usize;

                dc_colormap = *walllights.add(index);
                dc_x = rw_x;
                dc_iscale = (0xffffffffu32 / (rw_scale as u32)) as i32;
            }

            if midtexture != 0 {
                dc_yl = yl;
                dc_yh = yh;
                dc_texturemid = rw_midtexturemid;
                dc_source = R_GetColumn(midtexture, texturecolumn);
                if let Some(func) = colfunc {
                    func();
                }
                cclip[rx] = viewheight as c_short;
                fclip[rx] = -1;
            } else {
if toptexture != 0 {
let mut mid = pixhigh >> HEIGHTBITS;
pixhigh += pixhighstep;
if mid >= fclip_val {
mid = fclip_val - 1;
}
if mid >= yl {
dc_yl = yl;
dc_yh = mid;
dc_texturemid = rw_toptexturemid;
dc_source = R_GetColumn(toptexture, texturecolumn);
if let Some(func) = colfunc {
func();
}
cclip[rx] = mid as c_short;
} else {
cclip[rx] = (yl - 1) as c_short;
}
} else if markceiling != 0 {
cclip[rx] = (yl - 1) as c_short;
}
if bottomtexture != 0 {
let mut mid = (pixlow + HEIGHTUNIT - 1) >> HEIGHTBITS;
pixlow += pixlowstep;
let updated_cclip = cclip[rx] as c_int;
if mid <= updated_cclip {
mid = updated_cclip + 1;
}
if mid <= yh {
dc_yl = mid;
dc_yh = yh;
dc_texturemid = rw_bottomtexturemid;
dc_source = R_GetColumn(bottomtexture, texturecolumn);
if let Some(func) = colfunc {
func();
}
fclip[rx] = mid as c_short;
} else {
fclip[rx] = (yh + 1) as c_short;
}
} else if markfloor != 0 {
fclip[rx] = (yh + 1) as c_short;
}
if maskedtexture != 0 {
*maskedtexturecol.add(rx) = texturecolumn as c_short;
}
}
rw_scale += rw_scalestep;
topfrac += topstep;
bottomfrac += bottomstep;
rw_x += 1;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_StoreWallRange(start: c_int, stop: c_int) {
unsafe {
let ds_base = (&raw mut drawsegs) as *mut drawseg_t;
let current_ds_ptr = ds_p as *mut drawseg_t;
if current_ds_ptr == ds_base.add(MAXDRAWSEGS as usize) {
return;
}
if start >= viewwidth || start > stop {
doom_error(&format!("Bad R_RenderWallRange: {} to {}", start, stop));
}
let current_ds = current_ds_ptr.as_mut().unwrap();
let cl = (curline as *const seg_t).as_ref().unwrap();
let sdef = &*cl.sidedef;
let ldef = &mut *cl.linedef;
sidedef = cl.sidedef as *mut crate::r_bsp::side_t;
linedef = cl.linedef as *mut crate::r_bsp::line_t;
ldef.flags |= ML_MAPPED as i16;
rw_normalangle = cl.angle.wrapping_add(ANG90);
let mut offsetangle = (rw_normalangle as i32).wrapping_sub(rw_angle1).abs() as u32;
if offsetangle > ANG90 {
offsetangle = ANG90;
}
let distangle = ANG90 - offsetangle;
let v1 = &*cl.v1;
let hyp = R_PointToDist(v1.x, v1.y);
let sineval = finesine[(distangle >> ANGLETOFINESHIFT) as usize];
rw_distance = fixed_mul(hyp, sineval);
rw_x = start;
current_ds.x1 = start;
current_ds.x2 = stop;
current_ds.curline = curline as *mut seg_t;
rw_stopx = stop + 1;
let xtv = (&raw const xtoviewangle).as_ref().unwrap();
rw_scale = R_ScaleFromGlobalAngle(viewangle.wrapping_add(xtv[start as usize]));
current_ds.scale1 = rw_scale;
if stop > start {
let sc2 = R_ScaleFromGlobalAngle(viewangle.wrapping_add(xtv[stop as usize]));
current_ds.scale2 = sc2;
rw_scalestep = (sc2 - rw_scale) / (stop - start);
current_ds.scalestep = rw_scalestep;
} else {
current_ds.scale2 = current_ds.scale1;
}
let fsec = &*(frontsector as *const sector_t);
let bsec_ptr = backsector as *const sector_t;
worldtop = fsec.ceilingheight - viewz;
worldbottom = fsec.floorheight - viewz;
midtexture = 0;
toptexture = 0;
bottomtexture = 0;
maskedtexture = 0;
current_ds.maskedtexturecol = ptr::null_mut();
let scr_h_ptr = (&raw mut screenheightarray) as *mut c_short;
let neg_one_ptr = (&raw mut negonearray) as *mut c_short;
if bsec_ptr.is_null() {
midtexture = *texturetranslation.add(sdef.midtexture as usize);
markfloor = 1;
markceiling = 1;
if (ldef.flags as u32 & ML_DONTPEGBOTTOM) != 0 {
let vtop = fsec.floorheight + *textureheight.add(sdef.midtexture as usize);
rw_midtexturemid = vtop - viewz;
} else {
rw_midtexturemid = worldtop;
}
rw_midtexturemid += sdef.rowoffset;
current_ds.silhouette = SIL_BOTH as c_int;
current_ds.sprtopclip = scr_h_ptr;
current_ds.sprbottomclip = neg_one_ptr;
current_ds.bsilheight = i32::MAX;
current_ds.tsilheight = i32::MIN;
} else {
let bsec = &*bsec_ptr;
current_ds.sprtopclip = ptr::null_mut();
current_ds.sprbottomclip = ptr::null_mut();
current_ds.silhouette = 0;
if fsec.floorheight > bsec.floorheight {
current_ds.silhouette = SIL_BOTTOM as c_int;
current_ds.bsilheight = fsec.floorheight;
} else if bsec.floorheight > viewz {
current_ds.silhouette = SIL_BOTTOM as c_int;
current_ds.bsilheight = i32::MAX;
}
if fsec.ceilingheight < bsec.ceilingheight {
current_ds.silhouette |= SIL_TOP as c_int;
current_ds.tsilheight = fsec.ceilingheight;
} else if bsec.ceilingheight < viewz {
current_ds.silhouette |= SIL_TOP as c_int;
current_ds.tsilheight = i32::MIN;
}
if bsec.ceilingheight <= fsec.floorheight {
current_ds.sprbottomclip = neg_one_ptr;
current_ds.bsilheight = i32::MAX;
current_ds.silhouette |= SIL_BOTTOM as c_int;
}
if bsec.floorheight >= fsec.ceilingheight {
current_ds.sprtopclip = scr_h_ptr;
current_ds.tsilheight = i32::MIN;
current_ds.silhouette |= SIL_TOP as c_int;
}
worldhigh = bsec.ceilingheight - viewz;
worldlow = bsec.floorheight - viewz;
if (fsec.ceilingpic as c_int) == get_sky_flat_num()
&& (bsec.ceilingpic as c_int) == get_sky_flat_num()
{
worldtop = worldhigh;
}
markfloor = if worldlow != worldbottom
|| bsec.floorpic != fsec.floorpic
|| bsec.lightlevel != fsec.lightlevel
{
1
} else {
0
};
markceiling = if worldhigh != worldtop
|| bsec.ceilingpic != fsec.ceilingpic
|| bsec.lightlevel != fsec.lightlevel
{
1
} else {
0
};
if bsec.ceilingheight <= fsec.floorheight || bsec.floorheight >= fsec.ceilingheight {
markceiling = 1;
markfloor = 1;
}
if worldhigh < worldtop {
toptexture = *texturetranslation.add(sdef.toptexture as usize);
if (ldef.flags as u32 & ML_DONTPEGTOP) != 0 {
rw_toptexturemid = worldtop;
} else {
let vtop = bsec.ceilingheight + *textureheight.add(sdef.toptexture as usize);
rw_toptexturemid = vtop - viewz;
}
}
if worldlow > worldbottom {
bottomtexture = *texturetranslation.add(sdef.bottomtexture as usize);
if (ldef.flags as u32 & ML_DONTPEGBOTTOM) != 0 {
rw_bottomtexturemid = worldtop;
} else {
rw_bottomtexturemid = worldlow;
}
}
rw_toptexturemid = rw_toptexturemid.wrapping_add(sdef.rowoffset);
rw_bottomtexturemid = rw_bottomtexturemid.wrapping_add(sdef.rowoffset);
if sdef.midtexture != 0 {
maskedtexture = 1;
maskedtexturecol = lastopening.offset(-(rw_x as isize));
current_ds.maskedtexturecol = maskedtexturecol;
lastopening = lastopening.offset((rw_stopx - rw_x) as isize);
}
}
segtextured = midtexture | toptexture | bottomtexture | maskedtexture;
if segtextured != 0 {
let mut off_ang = rw_normalangle.wrapping_sub(rw_angle1 as u32);
if off_ang > ANG180 {
off_ang = 0u32.wrapping_sub(off_ang);
}
if off_ang > ANG90 {
off_ang = ANG90;
}
let sin_v = finesine[(off_ang >> ANGLETOFINESHIFT) as usize];
rw_offset = fixed_mul(hyp, sin_v);
if rw_normalangle.wrapping_sub(rw_angle1 as u32) < ANG180 {
rw_offset = -rw_offset;
}
rw_offset += sdef.textureoffset + cl.offset;
rw_centerangle = ANG90.wrapping_add(viewangle).wrapping_sub(rw_normalangle);
if fixedcolormap.is_null() {
walllights = select_wall_lights(fsec, v1, &*cl.v2);
}
}
if fsec.floorheight >= viewz {
markfloor = 0;
}
if fsec.ceilingheight <= viewz && (fsec.ceilingpic as c_int) != get_sky_flat_num() {
markceiling = 0;
}
worldtop >>= 4;
worldbottom >>= 4;
topstep = -fixed_mul(rw_scalestep, worldtop);
topfrac = (centeryfrac >> 4) - fixed_mul(worldtop, rw_scale);
bottomstep = -fixed_mul(rw_scalestep, worldbottom);
bottomfrac = (centeryfrac >> 4) - fixed_mul(worldbottom, rw_scale);
if !bsec_ptr.is_null() {
worldhigh >>= 4;
worldlow >>= 4;
if worldhigh < worldtop {
pixhigh = (centeryfrac >> 4) - fixed_mul(worldhigh, rw_scale);
pixhighstep = -fixed_mul(rw_scalestep, worldhigh);
}
if worldlow > worldbottom {
pixlow = (centeryfrac >> 4) - fixed_mul(worldlow, rw_scale);
pixlowstep = -fixed_mul(rw_scalestep, worldlow);
}
}
if markceiling != 0 {
ceilingplane = R_CheckPlane(ceilingplane as *mut crate::r_plane::visplane_t, rw_x, rw_stopx - 1);
}
if markfloor != 0 {
floorplane = R_CheckPlane(floorplane as *mut crate::r_plane::visplane_t, rw_x, rw_stopx - 1);
}
R_RenderSegLoop();
let cclip = (&raw const ceilingclip).as_ref().unwrap();
let fclip = (&raw const floorclip).as_ref().unwrap();
let start_u = start as usize;
let count = (rw_stopx - start) as usize;
if ((current_ds.silhouette & (SIL_TOP as c_int)) != 0 || maskedtexture != 0)
&& current_ds.sprtopclip.is_null()
{
let dst = std::slice::from_raw_parts_mut(lastopening, count);
dst.copy_from_slice(&cclip[start_u..start_u + count]);
current_ds.sprtopclip = lastopening.offset(-(start as isize));
lastopening = lastopening.add(count);
}
if ((current_ds.silhouette & (SIL_BOTTOM as c_int)) != 0 || maskedtexture != 0)
&& current_ds.sprbottomclip.is_null()
{
let dst = std::slice::from_raw_parts_mut(lastopening, count);
dst.copy_from_slice(&fclip[start_u..start_u + count]);
current_ds.sprbottomclip = lastopening.offset(-(start as isize));
lastopening = lastopening.add(count);
}
if maskedtexture != 0 && (current_ds.silhouette & (SIL_TOP as c_int)) == 0 {
current_ds.silhouette |= SIL_TOP as c_int;
current_ds.tsilheight = i32::MIN;
}
if maskedtexture != 0 && (current_ds.silhouette & (SIL_BOTTOM as c_int)) == 0 {
current_ds.silhouette |= SIL_BOTTOM as c_int;
current_ds.bsilheight = i32::MAX;
}
ds_p = current_ds_ptr.add(1) as *mut crate::r_bsp::drawseg_t;
}
}
