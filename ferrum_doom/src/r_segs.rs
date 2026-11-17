#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
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
    static mut skyflatnum: c_int;

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

// --- Глобальные переменные r_segs.c ---
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_RenderMaskedSegRange(ds: *mut drawseg_t, x1: c_int, x2: c_int) {
    unsafe {
        let cl = (*ds).curline as *mut seg_t;
        curline = cl as *mut crate::r_bsp::seg_t;
        let fsec = (*cl).frontsector as *mut sector_t;
        let bsec = (*cl).backsector as *mut sector_t;
        frontsector = fsec as *mut crate::r_bsp::sector_t;
        backsector = bsec as *mut crate::r_bsp::sector_t;

        let sdef = (*cl).sidedef;
        let texnum = *texturetranslation.add((*sdef).midtexture as usize);

        let mut lightnum = (((*fsec).lightlevel as c_int) >> LIGHTSEGSHIFT) + extralight;

        let v1 = (*cl).v1;
        let v2 = (*cl).v2;
        if (*v1).y == (*v2).y {
            lightnum -= 1;
        } else if (*v1).x == (*v2).x {
            lightnum += 1;
        }

        let sl_ptr = (&raw const scalelight) as *const [*mut u8; MAXLIGHTSCALE as usize];
        if lightnum < 0 {
            walllights = (*sl_ptr).as_ptr() as *mut *mut u8;
        } else if lightnum >= LIGHTLEVELS as c_int {
            walllights = (*sl_ptr.add((LIGHTLEVELS as usize) - 1)).as_ptr() as *mut *mut u8;
        } else {
            walllights = (*sl_ptr.add(lightnum as usize)).as_ptr() as *mut *mut u8;
        }

        maskedtexturecol = (*ds).maskedtexturecol;
        rw_scalestep = (*ds).scalestep;
        spryscale = (*ds).scale1 + (x1 - (*ds).x1) * rw_scalestep;
        mfloorclip = (*ds).sprbottomclip;
        mceilingclip = (*ds).sprtopclip;

        let ldef = (*cl).linedef;
        if ((*ldef).flags as u32 & ML_DONTPEGBOTTOM) != 0 {
            dc_texturemid = if (*fsec).floorheight > (*bsec).floorheight {
                (*fsec).floorheight
            } else {
                (*bsec).floorheight
            };
            dc_texturemid = dc_texturemid + *textureheight.add(texnum as usize) - viewz;
        } else {
            dc_texturemid = if (*fsec).ceilingheight < (*bsec).ceilingheight {
                (*fsec).ceilingheight
            } else {
                (*bsec).ceilingheight
            };
            dc_texturemid -= viewz;
        }
        dc_texturemid += (*sdef).rowoffset;

        if !fixedcolormap.is_null() {
            dc_colormap = fixedcolormap;
        }

        for x in x1..=x2 {
            dc_x = x;
            let col_slot = maskedtexturecol.add(dc_x as usize);
            if *col_slot != MAXSHORT {
                if fixedcolormap.is_null() {
                    let mut index = (spryscale as u32) >> LIGHTSCALESHIFT;
                    if index >= MAXLIGHTSCALE {
                        index = MAXLIGHTSCALE - 1;
                    }
                    dc_colormap = *walllights.add(index as usize);
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
        let fclip_ptr = (&raw mut floorclip) as *mut c_short;
        let cclip_ptr = (&raw mut ceilingclip) as *mut c_short;
        let xtv_ptr = (&raw const xtoviewangle) as *const u32;
        let mut texturecolumn: i32 = 0;

        while rw_x < rw_stopx {
            let rx = rw_x as usize;
            let mut yl = (topfrac + HEIGHTUNIT - 1) >> HEIGHTBITS;

            let cclip_val = *cclip_ptr.add(rx) as c_int;
            let fclip_val = *fclip_ptr.add(rx) as c_int;

            if yl < cclip_val + 1 {
                yl = cclip_val + 1;
            }

            if markceiling != 0 {
                let top = cclip_val + 1;
                let mut bottom = yl - 1;

                if bottom >= fclip_val {
                    bottom = fclip_val - 1;
                }

                if top <= bottom {
                    let cpl = ceilingplane as *mut visplane_t;
                    let cpl_top = (&raw mut (*cpl).top) as *mut u8;
                    let cpl_bot = (&raw mut (*cpl).bottom) as *mut u8;
                    *cpl_top.add(rx) = top as u8;
                    *cpl_bot.add(rx) = bottom as u8;
                }
            }

            let mut yh = bottomfrac >> HEIGHTBITS;
            if yh >= fclip_val {
                yh = fclip_val - 1;
            }

            if markfloor != 0 {
                let mut top = yh + 1;
                let bottom = fclip_val - 1;
                if top <= cclip_val {
                    top = cclip_val + 1;
                }
                if top <= bottom {
                    let fpl = floorplane as *mut visplane_t;
                    let fpl_top = (&raw mut (*fpl).top) as *mut u8;
                    let fpl_bot = (&raw mut (*fpl).bottom) as *mut u8;
                    *fpl_top.add(rx) = top as u8;
                    *fpl_bot.add(rx) = bottom as u8;
                }
            }

            if segtextured != 0 {
                let angle = (rw_centerangle.wrapping_add(*xtv_ptr.add(rx)) >> ANGLETOFINESHIFT) as usize;
                let tan_val = if angle >= 4096 { unsafe { *finetangent.as_ptr() } } else { finetangent[angle] };
                texturecolumn = rw_offset - fixed_mul(tan_val, rw_distance);
                texturecolumn >>= FRACBITS;

                let mut index = (rw_scale as u32) >> LIGHTSCALESHIFT;
                if index >= MAXLIGHTSCALE {
                    index = MAXLIGHTSCALE - 1;
                }

                dc_colormap = *walllights.add(index as usize);
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
*cclip_ptr.add(rx) = viewheight as c_short;
*fclip_ptr.add(rx) = -1;
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
*cclip_ptr.add(rx) = mid as c_short;
} else {
*cclip_ptr.add(rx) = (yl - 1) as c_short;
}
} else if markceiling != 0 {
*cclip_ptr.add(rx) = (yl - 1) as c_short;
}
if bottomtexture != 0 {
let mut mid = (pixlow + HEIGHTUNIT - 1) >> HEIGHTBITS;
pixlow += pixlowstep;
let updated_cclip = *cclip_ptr.add(rx) as c_int;
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
*fclip_ptr.add(rx) = mid as c_short;
} else {
*fclip_ptr.add(rx) = (yh + 1) as c_short;
}
} else if markfloor != 0 {
*fclip_ptr.add(rx) = (yh + 1) as c_short;
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
let current_ds = ds_p as *mut drawseg_t;
if current_ds == ds_base.add(MAXDRAWSEGS as usize) {
return;
}
if start >= viewwidth || start > stop {
doom_error(&format!("Bad R_RenderWallRange: {} to {}", start, stop));
}
let cl = curline as *mut seg_t;
let sdef = (*cl).sidedef;
let ldef = (*cl).linedef;
sidedef = sdef as *mut crate::r_bsp::side_t;
linedef = ldef as *mut crate::r_bsp::line_t;
(*ldef).flags |= ML_MAPPED as i16;
rw_normalangle = (*cl).angle.wrapping_add(ANG90);
let mut offsetangle = (rw_normalangle as i32).wrapping_sub(rw_angle1).abs() as u32;
if offsetangle > ANG90 {
offsetangle = ANG90;
}
let distangle = ANG90 - offsetangle;
let v1 = (*cl).v1;
let hyp = R_PointToDist((*v1).x, (*v1).y);
let sineval = finesine[(distangle >> ANGLETOFINESHIFT) as usize];
rw_distance = fixed_mul(hyp, sineval);
rw_x = start;
(*current_ds).x1 = start;
(*current_ds).x2 = stop;
(*current_ds).curline = cl;
rw_stopx = stop + 1;
let xtv_ptr = (&raw const xtoviewangle) as *const u32;
rw_scale = R_ScaleFromGlobalAngle(viewangle.wrapping_add(*xtv_ptr.add(start as usize)));
(*current_ds).scale1 = rw_scale;
if stop > start {
let sc2 = R_ScaleFromGlobalAngle(viewangle.wrapping_add(*xtv_ptr.add(stop as usize)));
(*current_ds).scale2 = sc2;
rw_scalestep = (sc2 - rw_scale) / (stop - start);
(*current_ds).scalestep = rw_scalestep;
} else {
(*current_ds).scale2 = (*current_ds).scale1;
}
let fsec = frontsector as *mut sector_t;
let bsec = backsector as *mut sector_t;
worldtop = (*fsec).ceilingheight - viewz;
worldbottom = (*fsec).floorheight - viewz;
midtexture = 0;
toptexture = 0;
bottomtexture = 0;
maskedtexture = 0;
(*current_ds).maskedtexturecol = ptr::null_mut();
let scr_h_ptr = (&raw mut screenheightarray) as *mut c_short;
let neg_one_ptr = (&raw mut negonearray) as *mut c_short;
if bsec.is_null() {
midtexture = *texturetranslation.add((*sdef).midtexture as usize);
markfloor = 1;
markceiling = 1;
if ((*ldef).flags as u32 & ML_DONTPEGBOTTOM) != 0 {
let vtop = (*fsec).floorheight + *textureheight.add((*sdef).midtexture as usize);
rw_midtexturemid = vtop - viewz;
} else {
rw_midtexturemid = worldtop;
}
rw_midtexturemid += (*sdef).rowoffset;
(*current_ds).silhouette = SIL_BOTH as c_int;
(*current_ds).sprtopclip = scr_h_ptr;
(*current_ds).sprbottomclip = neg_one_ptr;
(*current_ds).bsilheight = i32::MAX;
(*current_ds).tsilheight = i32::MIN;
} else {
(*current_ds).sprtopclip = ptr::null_mut();
(*current_ds).sprbottomclip = ptr::null_mut();
(*current_ds).silhouette = 0;
if (*fsec).floorheight > (*bsec).floorheight {
(*current_ds).silhouette = SIL_BOTTOM as c_int;
(*current_ds).bsilheight = (*fsec).floorheight;
} else if (*bsec).floorheight > viewz {
(*current_ds).silhouette = SIL_BOTTOM as c_int;
(*current_ds).bsilheight = i32::MAX;
}
if (*fsec).ceilingheight < (*bsec).ceilingheight {
(*current_ds).silhouette |= SIL_TOP as c_int;
(*current_ds).tsilheight = (*fsec).ceilingheight;
} else if (*bsec).ceilingheight < viewz {
(*current_ds).silhouette |= SIL_TOP as c_int;
(*current_ds).tsilheight = i32::MIN;
}
if (*bsec).ceilingheight <= (*fsec).floorheight {
(*current_ds).sprbottomclip = neg_one_ptr;
(*current_ds).bsilheight = i32::MAX;
(*current_ds).silhouette |= SIL_BOTTOM as c_int;
}
if (*bsec).floorheight >= (*fsec).ceilingheight {
(*current_ds).sprtopclip = scr_h_ptr;
(*current_ds).tsilheight = i32::MIN;
(*current_ds).silhouette |= SIL_TOP as c_int;
}
worldhigh = (*bsec).ceilingheight - viewz;
worldlow = (*bsec).floorheight - viewz;
if ((*fsec).ceilingpic as c_int) == skyflatnum && ((*bsec).ceilingpic as c_int) == skyflatnum {
worldtop = worldhigh;
}
markfloor = if worldlow != worldbottom
|| (*bsec).floorpic != (*fsec).floorpic
|| (*bsec).lightlevel != (*fsec).lightlevel
{
1
} else {
0
};
markceiling = if worldhigh != worldtop
|| (*bsec).ceilingpic != (*fsec).ceilingpic
|| (*bsec).lightlevel != (*fsec).lightlevel
{
1
} else {
0
};
if (*bsec).ceilingheight <= (*fsec).floorheight || (*bsec).floorheight >= (*fsec).ceilingheight {
markceiling = 1;
markfloor = 1;
}
if worldhigh < worldtop {
toptexture = *texturetranslation.add((*sdef).toptexture as usize);
if ((*ldef).flags as u32 & ML_DONTPEGTOP) != 0 {
rw_toptexturemid = worldtop;
} else {
let vtop = (*bsec).ceilingheight + *textureheight.add((*sdef).toptexture as usize);
rw_toptexturemid = vtop - viewz;
}
}
if worldlow > worldbottom {
bottomtexture = *texturetranslation.add((*sdef).bottomtexture as usize);
if ((*ldef).flags as u32 & ML_DONTPEGBOTTOM) != 0 {
rw_bottomtexturemid = worldtop;
} else {
rw_bottomtexturemid = worldlow;
}
}
rw_toptexturemid = rw_toptexturemid.wrapping_add((*sdef).rowoffset);
rw_bottomtexturemid = rw_bottomtexturemid.wrapping_add((*sdef).rowoffset);
if (*sdef).midtexture != 0 {
maskedtexture = 1;
maskedtexturecol = lastopening.offset(-(rw_x as isize));
(*current_ds).maskedtexturecol = maskedtexturecol;
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
rw_offset += (*sdef).textureoffset + (*cl).offset;
rw_centerangle = ANG90.wrapping_add(viewangle).wrapping_sub(rw_normalangle);
if fixedcolormap.is_null() {
let mut lightnum = (((*fsec).lightlevel as c_int) >> LIGHTSEGSHIFT) + extralight;
let v2 = (*cl).v2;
if (*v1).y == (*v2).y {
lightnum -= 1;
} else if (*v1).x == (*v2).x {
lightnum += 1;
}
let sl_ptr = (&raw const scalelight) as *const [*mut u8; MAXLIGHTSCALE as usize];
if lightnum < 0 {
walllights = (*sl_ptr).as_ptr() as *mut *mut u8;
} else if lightnum >= LIGHTLEVELS as c_int {
walllights = (*sl_ptr.add((LIGHTLEVELS as usize) - 1)).as_ptr() as *mut *mut u8;
} else {
walllights = (*sl_ptr.add(lightnum as usize)).as_ptr() as *mut *mut u8;
}
}
}
if (*fsec).floorheight >= viewz {
markfloor = 0;
}
if (*fsec).ceilingheight <= viewz && ((*fsec).ceilingpic as c_int) != skyflatnum {
markceiling = 0;
}
worldtop >>= 4;
worldbottom >>= 4;
topstep = -fixed_mul(rw_scalestep, worldtop);
topfrac = (centeryfrac >> 4) - fixed_mul(worldtop, rw_scale);
bottomstep = -fixed_mul(rw_scalestep, worldbottom);
bottomfrac = (centeryfrac >> 4) - fixed_mul(worldbottom, rw_scale);
if !bsec.is_null() {
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
ceilingplane = R_CheckPlane(ceilingplane as *mut crate::r_plane::visplane_t, rw_x, rw_stopx - 1)
;
}
if markfloor != 0 {
floorplane = R_CheckPlane(floorplane as *mut crate::r_plane::visplane_t, rw_x, rw_stopx - 1)
;
}
R_RenderSegLoop();
let cclip_ptr = (&raw const ceilingclip) as *const c_short;
let fclip_ptr = (&raw const floorclip) as *const c_short;
let count = (rw_stopx - start) as usize;
if (((*current_ds).silhouette & (SIL_TOP as c_int)) != 0 || maskedtexture != 0)
&& (*current_ds).sprtopclip.is_null()
{
ptr::copy_nonoverlapping(cclip_ptr.add(start as usize), lastopening, count);
(*current_ds).sprtopclip = lastopening.offset(-(start as isize));
lastopening = lastopening.add(count);
}
if (((*current_ds).silhouette & (SIL_BOTTOM as c_int)) != 0 || maskedtexture != 0)
&& (*current_ds).sprbottomclip.is_null()
{
ptr::copy_nonoverlapping(fclip_ptr.add(start as usize), lastopening, count);
(*current_ds).sprbottomclip = lastopening.offset(-(start as isize));
lastopening = lastopening.add(count);
}
if maskedtexture != 0 && ((*current_ds).silhouette & (SIL_TOP as c_int)) == 0 {
(*current_ds).silhouette |= SIL_TOP as c_int;
(*current_ds).tsilheight = i32::MIN;
}
if maskedtexture != 0 && ((*current_ds).silhouette & (SIL_BOTTOM as c_int)) == 0 {
(*current_ds).silhouette |= SIL_BOTTOM as c_int;
(*current_ds).bsilheight = i32::MAX;
}
ds_p = current_ds.add(1) as *mut crate::r_bsp::drawseg_t;
}
}