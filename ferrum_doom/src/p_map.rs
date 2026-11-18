#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_void};
use std::ptr;

use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP};
use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::m_random::p_random;
use crate::p_maputl::{
    P_AproxDistance, P_BlockLinesIterator, P_BlockThingsIterator, P_BoxOnLineSide, P_LineOpening,
    P_PathTraverse, P_PointOnLineSide, P_SetThingPosition, P_UnsetThingPosition, lowfloor,
    openbottom, openrange, opentop, trace,
};
use crate::p_mobj::{P_RemoveMobj, P_SetMobjState, P_SpawnBlood, P_SpawnMobj, P_SpawnPuff};
use crate::p_setup::{bmaporgx, bmaporgy, lines};
use crate::p_sight::{P_CheckSight, bottomslope, topslope};
use crate::p_spec::{P_CrossSpecialLine, P_ShootSpecialLine};
use crate::p_switch::P_UseSpecialLine;
use crate::p_tick::leveltime;
use crate::r_bsp::subsector_t;
use crate::r_main::{R_PointInSubsector, R_PointToAngle2, validcount};
use crate::r_sky::skyflatnum;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};

pub const MAXSPECIALCROSS: usize = 8;
const QUARTER_FINE: usize = FINEANGLES / 4;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn P_DamageMobj(target: *mut mobj_t, inflictor: *mut mobj_t, source: *mut mobj_t, damage: c_int);
    fn P_TouchSpecialThing(special: *mut mobj_t, toucher: *mut mobj_t);

    static mut gamemap: c_int;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Map physics error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные p_map.c ---
#[unsafe(no_mangle)]
pub static mut tmbbox: [i32; 4] = [0; 4];

#[unsafe(no_mangle)]
pub static mut tmthing: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut tmflags: c_int = 0;

#[unsafe(no_mangle)]
pub static mut tmx: i32 = 0;

#[unsafe(no_mangle)]
pub static mut tmy: i32 = 0;

#[unsafe(no_mangle)]
pub static mut floatok: c_int = 0;

#[unsafe(no_mangle)]
pub static mut tmfloorz: i32 = 0;

#[unsafe(no_mangle)]
pub static mut tmceilingz: i32 = 0;

#[unsafe(no_mangle)]
pub static mut tmdropoffz: i32 = 0;

#[unsafe(no_mangle)]
pub static mut ceilingline: *mut line_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut spechit: [*mut line_t; MAXSPECIALCROSS] = [ptr::null_mut(); MAXSPECIALCROSS];

#[unsafe(no_mangle)]
pub static mut numspechit: c_int = 0;

#[unsafe(no_mangle)]
pub static mut bestslidefrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut secondslidefrac: i32 = 0;

#[unsafe(no_mangle)]
pub static mut bestslideline: *mut line_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut secondslideline: *mut line_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut slidemo: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut tmxmove: i32 = 0;

#[unsafe(no_mangle)]
pub static mut tmymove: i32 = 0;

#[unsafe(no_mangle)]
pub static mut linetarget: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut shootthing: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut shootz: i32 = 0;

#[unsafe(no_mangle)]
pub static mut la_damage: c_int = 0;

#[unsafe(no_mangle)]
pub static mut attackrange: i32 = 0;

#[unsafe(no_mangle)]
pub static mut aimslope: i32 = 0;

#[unsafe(no_mangle)]
pub static mut usething: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut bombsource: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut bombspot: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut bombdamage: c_int = 0;

#[unsafe(no_mangle)]
pub static mut crushchange: c_int = 0;

#[unsafe(no_mangle)]
pub static mut nofit: c_int = 0;

// --- PIT_StompThing ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_StompThing(thing: *mut mobj_t) -> c_int {
    unsafe {
        if ((*thing).flags as u32 & MF_SHOOTABLE) == 0 {
            return 1;
        }

        let blockdist = (*thing).radius + (*tmthing).radius;
        if ((*thing).x.wrapping_sub(tmx)).wrapping_abs() >= blockdist
            || ((*thing).y.wrapping_sub(tmy)).wrapping_abs() >= blockdist
        {
            return 1;
        }

        if thing == tmthing {
            return 1;
        }

        if (*tmthing).player.is_null() && gamemap != 30 {
            return 0;
        }

        P_DamageMobj(thing, tmthing, tmthing, 10000);
        1
    }
}

// --- P_TeleportMove ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TeleportMove(thing: *mut mobj_t, x: i32, y: i32) -> c_int {
    unsafe {
        tmthing = thing;
        tmflags = (*thing).flags;
        tmx = x;
        tmy = y;

        let rad = (*tmthing).radius;
        let bbox_ptr = (&raw mut tmbbox) as *mut i32;
        (*bbox_ptr.add(BOXTOP)) = y.wrapping_add(rad);
        (*bbox_ptr.add(BOXBOTTOM)) = y.wrapping_sub(rad);
        (*bbox_ptr.add(BOXRIGHT)) = x.wrapping_add(rad);
        (*bbox_ptr.add(BOXLEFT)) = x.wrapping_sub(rad);

        let newsubsec = R_PointInSubsector(x, y);
        ceilingline = ptr::null_mut();

        let sec = (*newsubsec).sector;
        tmfloorz = (*sec).floorheight;
        tmdropoffz = tmfloorz;
        tmceilingz = (*sec).ceilingheight;

        validcount += 1;
        numspechit = 0;

        let max_rad = MAXRADIUS as i32;
        let shift = MAPBLOCKSHIFT as u32;
        let xl = ((*bbox_ptr.add(BOXLEFT)).wrapping_sub(bmaporgx).wrapping_sub(max_rad)) >> shift;
        let xh = ((*bbox_ptr.add(BOXRIGHT)).wrapping_sub(bmaporgx).wrapping_add(max_rad)) >> shift;
        let yl = ((*bbox_ptr.add(BOXBOTTOM)).wrapping_sub(bmaporgy).wrapping_sub(max_rad)) >> shift;
        let yh = ((*bbox_ptr.add(BOXTOP)).wrapping_sub(bmaporgy).wrapping_add(max_rad)) >> shift;

        for bx in xl..=xh {
            for by in yl..=yh {
                if P_BlockThingsIterator(bx, by, Some(std::mem::transmute(PIT_StompThing as unsafe extern "C" fn(*mut _) -> c_int))) == 0 {
                    return 0;
                }
            }
        }

        P_UnsetThingPosition(thing as *mut _);

        (*thing).floorz = tmfloorz;
        (*thing).ceilingz = tmceilingz;
        (*thing).x = x;
        (*thing).y = y;

        P_SetThingPosition(thing as *mut _);

        1
    }
}

// --- PIT_CheckLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_CheckLine(ld: *mut line_t) -> c_int {
    unsafe {
        let bbox_ptr = (&raw mut tmbbox) as *mut i32;
        let ld_bbox = (&raw const (*ld).bbox) as *const i32;

        if (*bbox_ptr.add(BOXRIGHT)) <= *ld_bbox.add(BOXLEFT)
            || (*bbox_ptr.add(BOXLEFT)) >= *ld_bbox.add(BOXRIGHT)
            || (*bbox_ptr.add(BOXTOP)) <= *ld_bbox.add(BOXBOTTOM)
            || (*bbox_ptr.add(BOXBOTTOM)) >= *ld_bbox.add(BOXTOP)
        {
            return 1;
        }

        if P_BoxOnLineSide(bbox_ptr, ld as *mut _) != -1 {
            return 1;
        }

        if (*ld).backsector.is_null() {
            return 0;
        }

        if ((*tmthing).flags as u32 & MF_MISSILE) == 0 {
            if ((*ld).flags as u32 & ML_BLOCKING) != 0 {
                return 0;
            }
            if (*tmthing).player.is_null() && ((*ld).flags as u32 & ML_BLOCKMONSTERS) != 0 {
                return 0;
            }
        }

        P_LineOpening(ld as *mut _);

        if opentop < tmceilingz {
            tmceilingz = opentop;
            ceilingline = ld;
        }

        if openbottom > tmfloorz {
            tmfloorz = openbottom;
        }

        if lowfloor < tmdropoffz {
            tmdropoffz = lowfloor;
        }

        if (*ld).special != 0 && (numspechit as usize) < MAXSPECIALCROSS {
            let spec_ptr = (&raw mut spechit) as *mut *mut line_t;
            *spec_ptr.add(numspechit as usize) = ld;
            numspechit += 1;
        }

        1
    }
}

// --- PIT_CheckThing ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_CheckThing(thing: *mut mobj_t) -> c_int {
    unsafe {
        if ((*thing).flags as u32 & (MF_SOLID | MF_SPECIAL | MF_SHOOTABLE)) == 0 {
            return 1;
        }

        let blockdist = (*thing).radius + (*tmthing).radius;
        if ((*thing).x.wrapping_sub(tmx)).wrapping_abs() >= blockdist
            || ((*thing).y.wrapping_sub(tmy)).wrapping_abs() >= blockdist
        {
            return 1;
        }

        if thing == tmthing {
            return 1;
        }

        if ((*tmthing).flags as u32 & MF_SKULLFLY) != 0 {
            let info = (*tmthing).info;
            let damage = ((p_random() % 8) + 1) * (*info).damage;
            P_DamageMobj(thing, tmthing, tmthing, damage);

            (*tmthing).flags &= !(MF_SKULLFLY as c_int);
            (*tmthing).momx = 0;
            (*tmthing).momy = 0;
            (*tmthing).momz = 0;

            P_SetMobjState(tmthing as *mut _, (*info).spawnstate as _);
            return 0;
        }

        if ((*tmthing).flags as u32 & MF_MISSILE) != 0 {
            if (*tmthing).z > (*thing).z + (*thing).height {
                return 1;
            }
            if (*tmthing).z + (*tmthing).height < (*thing).z {
                return 1;
            }

            let target = (*tmthing).target;
            if !target.is_null() {
                let same_species = (*target).type_ == (*thing).type_
                    || ((*target).type_ == MT_KNIGHT && (*thing).type_ == MT_BRUISER)
                    || ((*target).type_ == MT_BRUISER && (*thing).type_ == MT_KNIGHT);

                if same_species {
                    if thing == target {
                        return 1;
                    }
                    if (*thing).type_ != MT_PLAYER {
                        return 0;
                    }
                }
            }

            if ((*thing).flags as u32 & MF_SHOOTABLE) == 0 {
return (((*thing).flags as u32 & MF_SOLID) == 0) as c_int;
}
let info = (*tmthing).info;
let damage = ((p_random() % 8) + 1) * (*info).damage;
P_DamageMobj(thing, tmthing, target, damage);
return 0;
}
if ((*thing).flags as u32 & MF_SPECIAL) != 0 {
let solid = ((*thing).flags as u32 & MF_SOLID) != 0;
if (tmflags as u32 & MF_PICKUP) != 0 {
P_TouchSpecialThing(thing, tmthing);
}
return (!solid) as c_int;
}
(((*thing).flags as u32 & MF_SOLID) == 0) as c_int
}
}
// --- P_CheckPosition ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckPosition(thing: *mut mobj_t, x: i32, y: i32) -> c_int {
unsafe {
tmthing = thing;
tmflags = (*thing).flags;
tmx = x;
tmy = y;
let rad = (*tmthing).radius;
let bbox_ptr = (&raw mut tmbbox) as *mut i32;
(*bbox_ptr.add(BOXTOP)) = y.wrapping_add(rad);
(*bbox_ptr.add(BOXBOTTOM)) = y.wrapping_sub(rad);
(*bbox_ptr.add(BOXRIGHT)) = x.wrapping_add(rad);
(*bbox_ptr.add(BOXLEFT)) = x.wrapping_sub(rad);
let newsubsec = R_PointInSubsector(x, y);
ceilingline = ptr::null_mut();
let sec = (*newsubsec).sector;
tmfloorz = (*sec).floorheight;
tmdropoffz = tmfloorz;
tmceilingz = (*sec).ceilingheight;
validcount += 1;
numspechit = 0;
if (tmflags as u32 & MF_NOCLIP) != 0 {
return 1;
}
let max_rad = MAXRADIUS as i32;
let shift = MAPBLOCKSHIFT as u32;
let mut xl = ((*bbox_ptr.add(BOXLEFT)).wrapping_sub(bmaporgx).wrapping_sub(max_rad)) >> shift;
let mut xh = ((*bbox_ptr.add(BOXRIGHT)).wrapping_sub(bmaporgx).wrapping_add(max_rad)) >> shift;
let mut yl = ((*bbox_ptr.add(BOXBOTTOM)).wrapping_sub(bmaporgy).wrapping_sub(max_rad)) >> shift;
let mut yh = ((*bbox_ptr.add(BOXTOP)).wrapping_sub(bmaporgy).wrapping_add(max_rad)) >> shift;
for bx in xl..=xh {
for by in yl..=yh {
if P_BlockThingsIterator(bx, by, Some(std::mem::transmute(PIT_CheckThing as unsafe extern "C" fn(*mut _) -> c_int))) == 0 {
return 0;
}
}
}
xl = ((*bbox_ptr.add(BOXLEFT)).wrapping_sub(bmaporgx)) >> shift;
xh = ((*bbox_ptr.add(BOXRIGHT)).wrapping_sub(bmaporgx)) >> shift;
yl = ((*bbox_ptr.add(BOXBOTTOM)).wrapping_sub(bmaporgy)) >> shift;
yh = ((*bbox_ptr.add(BOXTOP)).wrapping_sub(bmaporgy)) >> shift;
for bx in xl..=xh {
for by in yl..=yh {
if P_BlockLinesIterator(bx, by, Some(std::mem::transmute(PIT_CheckLine as unsafe extern "C" fn(*mut _) -> c_int))) == 0 {
return 0;
}
}
}
1
}
}
// --- P_TryMove ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TryMove(thing: *mut mobj_t, x: i32, y: i32) -> c_int {
unsafe {
floatok = 0;
if P_CheckPosition(thing, x, y) == 0 {
return 0;
}
let flags = (*thing).flags as u32;
let frac_u = FRACUNIT as i32;
if (flags & MF_NOCLIP) == 0 {
if tmceilingz - tmfloorz < (*thing).height {
return 0;
}
floatok = 1;
if (flags & MF_TELEPORT) == 0 && tmceilingz - (*thing).z < (*thing).height {
return 0;
}
if (flags & MF_TELEPORT) == 0 && tmfloorz - (*thing).z > 24 * frac_u {
return 0;
}
if (flags & (MF_DROPOFF | MF_FLOAT)) == 0 && tmfloorz - tmdropoffz > 24 * frac_u {
return 0;
}
}
P_UnsetThingPosition(thing as *mut _);
let oldx = (*thing).x;
let oldy = (*thing).y;
(*thing).floorz = tmfloorz;
(*thing).ceilingz = tmceilingz;
(*thing).x = x;
(*thing).y = y;
P_SetThingPosition(thing as *mut _);
if (flags & (MF_TELEPORT | MF_NOCLIP)) == 0 {
let spec_ptr = (&raw const spechit) as *const *mut line_t;
let lines_base = lines as *mut line_t;
while numspechit > 0 {
numspechit -= 1;
let ld = *spec_ptr.add(numspechit as usize);
let side = P_PointOnLineSide((*thing).x, (*thing).y, ld as *mut _);
let oldside = P_PointOnLineSide(oldx, oldy, ld as *mut _);
if side != oldside && (*ld).special != 0 {
let line_idx = ld.offset_from(lines_base) as c_int;
P_CrossSpecialLine(line_idx, oldside, thing as *mut _);
}
}
}
1
}
}
// --- P_ThingHeightClip ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ThingHeightClip(thing: *mut mobj_t) -> c_int {
unsafe {
let onfloor = (*thing).z == (*thing).floorz;
P_CheckPosition(thing, (*thing).x, (*thing).y);
(*thing).floorz = tmfloorz;
(*thing).ceilingz = tmceilingz;
if onfloor {
(*thing).z = (*thing).floorz;
} else if (*thing).z + (*thing).height > (*thing).ceilingz {
(*thing).z = (*thing).ceilingz - (*thing).height;
}
(((*thing).ceilingz - (*thing).floorz) >= (*thing).height) as c_int
}
}
// --- P_HitSlideLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_HitSlideLine(ld: *mut line_t) {
unsafe {
if (*ld).slopetype == ST_HORIZONTAL {
tmymove = 0;
return;
}
if (*ld).slopetype == ST_VERTICAL {
tmxmove = 0;
return;
}
let side = P_PointOnLineSide((*slidemo).x, (*slidemo).y, ld as *mut _);
let mut lineangle = R_PointToAngle2(0, 0, (*ld).dx, (*ld).dy);
if side == 1 {
lineangle = lineangle.wrapping_add(ANG180);
}
let moveangle = R_PointToAngle2(0, 0, tmxmove, tmymove);
let mut deltaangle = moveangle.wrapping_sub(lineangle);
if deltaangle > ANG180 {
deltaangle = deltaangle.wrapping_add(ANG180);
}
let line_idx = (lineangle >> ANGLETOFINESHIFT) as usize;
let delta_idx = (deltaangle >> ANGLETOFINESHIFT) as usize;
let movelen = P_AproxDistance(tmxmove, tmymove);
let newlen = fixed_mul(movelen, finesine[delta_idx + QUARTER_FINE]);
tmxmove = fixed_mul(newlen, finesine[line_idx + QUARTER_FINE]);
tmymove = fixed_mul(newlen, finesine[line_idx]);
}
}
// --- PTR_SlideTraverse ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_SlideTraverse(in_ptr: *mut intercept_t) -> c_int {
unsafe {
if (*in_ptr).isaline == 0 {
doom_error("PTR_SlideTraverse: not a line?");
}
let li = (*in_ptr).d.line as *mut line_t;
let mut is_blocking = false;
if ((*li).flags as u32 & ML_TWOSIDED) == 0 {
if P_PointOnLineSide((*slidemo).x, (*slidemo).y, li as *mut _) != 0 {
return 1;
}
is_blocking = true;
} else {
P_LineOpening(li as *mut _);
let frac_u = FRACUNIT as i32;
if openrange < (*slidemo).height
|| opentop - (*slidemo).z < (*slidemo).height
|| openbottom - (*slidemo).z > 24 * frac_u
{
is_blocking = true;
}
}
if is_blocking {
if (*in_ptr).frac < bestslidefrac {
secondslidefrac = bestslidefrac;
secondslideline = bestslideline;
bestslidefrac = (*in_ptr).frac;
bestslideline = li;
}
return 0;
}
1
}
}
// --- P_SlideMove ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SlideMove(mo: *mut mobj_t) {
unsafe {
slidemo = mo;
let mut hitcount = 0;
let frac_u = FRACUNIT as i32;
loop {
hitcount += 1;
if hitcount == 3 {
if P_TryMove(mo, (*mo).x, (*mo).y.wrapping_add((*mo).momy)) == 0 {
P_TryMove(mo, (*mo).x.wrapping_add((*mo).momx), (*mo).y);
}
return;
}
let rad = (*mo).radius;
let (leadx, trailx) = if (*mo).momx > 0 {
((*mo).x.wrapping_add(rad), (*mo).x.wrapping_sub(rad))
} else {
((*mo).x.wrapping_sub(rad), (*mo).x.wrapping_add(rad))
};
let (leady, traily) = if (*mo).momy > 0 {
((*mo).y.wrapping_add(rad), (*mo).y.wrapping_sub(rad))
} else {
((*mo).y.wrapping_sub(rad), (*mo).y.wrapping_add(rad))
};
bestslidefrac = frac_u + 1;
let pt_lines = PT_ADDLINES as c_int;
P_PathTraverse(
leadx,
leady,
leadx.wrapping_add((*mo).momx),
leady.wrapping_add((*mo).momy),
pt_lines,
Some(std::mem::transmute(PTR_SlideTraverse as unsafe extern "C" fn(*mut _) -> c_int)),
);
P_PathTraverse(
trailx,
leady,
trailx.wrapping_add((*mo).momx),
leady.wrapping_add((*mo).momy),
pt_lines,
Some(std::mem::transmute(PTR_SlideTraverse as unsafe extern "C" fn(*mut _) -> c_int)),
);
P_PathTraverse(
leadx,
traily,
leadx.wrapping_add((*mo).momx),
traily.wrapping_add((*mo).momy),
pt_lines,
Some(std::mem::transmute(PTR_SlideTraverse as unsafe extern "C" fn(*mut _) -> c_int)),
);
if bestslidefrac == frac_u + 1 {
if P_TryMove(mo, (*mo).x, (*mo).y.wrapping_add((*mo).momy)) == 0 {
P_TryMove(mo, (*mo).x.wrapping_add((*mo).momx), (*mo).y);
}
return;
}
bestslidefrac -= 0x800;
if bestslidefrac > 0 {
let newx = fixed_mul((*mo).momx, bestslidefrac);
let newy = fixed_mul((*mo).momy, bestslidefrac);
if P_TryMove(mo, (*mo).x.wrapping_add(newx), (*mo).y.wrapping_add(newy)) == 0 {
if P_TryMove(mo, (*mo).x, (*mo).y.wrapping_add((*mo).momy)) == 0 {
P_TryMove(mo, (*mo).x.wrapping_add((*mo).momx), (*mo).y);
}
return;
}
}
bestslidefrac = frac_u - (bestslidefrac + 0x800);
if bestslidefrac > frac_u {
bestslidefrac = frac_u;
}
if bestslidefrac <= 0 {
return;
}
tmxmove = fixed_mul((*mo).momx, bestslidefrac);
tmymove = fixed_mul((*mo).momy, bestslidefrac);
P_HitSlideLine(bestslideline);
(*mo).momx = tmxmove;
(*mo).momy = tmymove;
if P_TryMove(mo, (*mo).x.wrapping_add(tmxmove), (*mo).y.wrapping_add(tmymove)) != 0 {
return;
}
}
}
}
// --- PTR_AimTraverse ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_AimTraverse(in_ptr: *mut intercept_t) -> c_int {
unsafe {
if (*in_ptr).isaline != 0 {
let li = (*in_ptr).d.line as *mut line_t;
if ((*li).flags as u32 & ML_TWOSIDED) == 0 {
return 0;
}
P_LineOpening(li as *mut _);
if openbottom >= opentop {
return 0;
}
let dist = fixed_mul(attackrange, (*in_ptr).frac);
let front = (*li).frontsector;
let back = (*li).backsector;
if (*front).floorheight != (*back).floorheight {
let slope = fixed_div(openbottom - shootz, dist);
if slope > bottomslope {
bottomslope = slope;
}
}
if (*front).ceilingheight != (*back).ceilingheight {
let slope = fixed_div(opentop - shootz, dist);
if slope < topslope {
topslope = slope;
}
}
if topslope <= bottomslope {
return 0;
}
return 1;
}
let th = (*in_ptr).d.thing as *mut mobj_t;
if th == shootthing {
return 1;
}
if ((*th).flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
let dist = fixed_mul(attackrange, (*in_ptr).frac);
let mut thingtopslope = fixed_div((*th).z + (*th).height - shootz, dist);
if thingtopslope < bottomslope {
return 1;
}
let mut thingbottomslope = fixed_div((*th).z - shootz, dist);
if thingbottomslope > topslope {
return 1;
}
if thingtopslope > topslope {
thingtopslope = topslope;
}
if thingbottomslope < bottomslope {
thingbottomslope = bottomslope;
}
aimslope = (thingtopslope + thingbottomslope) / 2;
linetarget = th;
0
}
}
// --- PTR_ShootTraverse ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_ShootTraverse(in_ptr: *mut intercept_t) -> c_int {
unsafe {
let frac_u = FRACUNIT as i32;
if (*in_ptr).isaline != 0 {
let li = (*in_ptr).d.line as *mut line_t;
if (*li).special != 0 {
P_ShootSpecialLine(shootthing as *mut _, li as *mut _);
}
let mut hit_line = false;
if ((*li).flags as u32 & ML_TWOSIDED) == 0 {
hit_line = true;
} else {
P_LineOpening(li as *mut _);
let dist = fixed_mul(attackrange, (*in_ptr).frac);
let front = (*li).frontsector;
let back = (*li).backsector;
if (*front).floorheight != (*back).floorheight {
let slope = fixed_div(openbottom - shootz, dist);
if slope > aimslope {
hit_line = true;
}
}
if !hit_line && (*front).ceilingheight != (*back).ceilingheight {
let slope = fixed_div(opentop - shootz, dist);
if slope < aimslope {
hit_line = true;
}
}
}
if !hit_line {
return 1;
}
let frac = (*in_ptr).frac - fixed_div(4 * frac_u, attackrange);
let x = trace.x.wrapping_add(fixed_mul(trace.dx, frac));
let y = trace.y.wrapping_add(fixed_mul(trace.dy, frac));
let z = shootz.wrapping_add(fixed_mul(aimslope, fixed_mul(frac, attackrange)));
let front = (*li).frontsector;
if ((*front).ceilingpic as c_int) == skyflatnum {
if z > (*front).ceilingheight {
return 0;
}
let back = (*li).backsector;
if !back.is_null() && ((*back).ceilingpic as c_int) == skyflatnum {
return 0;
}
}
P_SpawnPuff(x, y, z);
return 0;
}
let th = (*in_ptr).d.thing as *mut mobj_t;
if th == shootthing {
return 1;
}
if ((*th).flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
let dist = fixed_mul(attackrange, (*in_ptr).frac);
let thingtopslope = fixed_div((*th).z + (*th).height - shootz, dist);
if thingtopslope < aimslope {
return 1;
}
let thingbottomslope = fixed_div((*th).z - shootz, dist);
if thingbottomslope > aimslope {
return 1;
}
let frac = (*in_ptr).frac - fixed_div(10 * frac_u, attackrange);
let x = trace.x.wrapping_add(fixed_mul(trace.dx, frac));
let y = trace.y.wrapping_add(fixed_mul(trace.dy, frac));
let z = shootz.wrapping_add(fixed_mul(aimslope, fixed_mul(frac, attackrange)));
if ((*th).flags as u32 & MF_NOBLOOD) != 0 {
P_SpawnPuff(x, y, z);
} else {
P_SpawnBlood(x, y, z, la_damage);
}
if la_damage != 0 {
P_DamageMobj(th, shootthing, shootthing, la_damage);
}
0
}
}
// --- P_AimLineAttack ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AimLineAttack(t1: *mut mobj_t, mut angle: u32, distance: i32) -> i32 {
unsafe {
angle >>= ANGLETOFINESHIFT;
let idx = angle as usize;
shootthing = t1;
let dist_int = distance >> FRACBITS;
let x2 = (*t1).x.wrapping_add(dist_int.wrapping_mul(finesine[idx + QUARTER_FINE]));
let y2 = (*t1).y.wrapping_add(dist_int.wrapping_mul(finesine[idx]));
let frac_u = FRACUNIT as i32;
shootz = (*t1).z + ((*t1).height >> 1) + 8 * frac_u;
topslope = 100 * frac_u / 160;
bottomslope = -100 * frac_u / 160;
attackrange = distance;
linetarget = ptr::null_mut();
let flags = (PT_ADDLINES | PT_ADDTHINGS) as c_int;
P_PathTraverse((*t1).x, (*t1).y, x2, y2, flags, Some(std::mem::transmute(PTR_AimTraverse as unsafe extern "C" fn(*mut _) -> c_int)));
if !linetarget.is_null() {
aimslope
} else {
0
}
}
}
// --- P_LineAttack ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LineAttack(
t1: *mut mobj_t,
mut angle: u32,
distance: i32,
slope: i32,
damage: c_int,
) {
unsafe {
angle >>= ANGLETOFINESHIFT;
let idx = angle as usize;
shootthing = t1;
la_damage = damage;
let dist_int = distance >> FRACBITS;
let x2 = (*t1).x.wrapping_add(dist_int.wrapping_mul(finesine[idx + QUARTER_FINE]));
let y2 = (*t1).y.wrapping_add(dist_int.wrapping_mul(finesine[idx]));
let frac_u = FRACUNIT as i32;
shootz = (*t1).z + ((*t1).height >> 1) + 8 * frac_u;
attackrange = distance;
aimslope = slope;
let flags = (PT_ADDLINES | PT_ADDTHINGS) as c_int;
P_PathTraverse((*t1).x, (*t1).y, x2, y2, flags, Some(std::mem::transmute(PTR_ShootTraverse as unsafe extern "C" fn(*mut _) -> c_int)));
}
}
// --- PTR_UseTraverse ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_UseTraverse(in_ptr: *mut intercept_t) -> c_int {
unsafe {
let li = (*in_ptr).d.line as *mut line_t;
if (*li).special == 0 {
P_LineOpening(li as *mut _);
if openrange <= 0 {
S_StartSound(usething as *mut c_void, sfx_noway as c_int);
return 0;
}
return 1;
}
let side = if P_PointOnLineSide((*usething).x, (*usething).y, li as *mut _) == 1 {
1
} else {
0
};
P_UseSpecialLine(usething as *mut _, li as *mut _, side);
0
}
}
// --- P_UseLines ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UseLines(player: *mut player_t) {
unsafe {
let mo = (*player).mo as *mut mobj_t;
usething = mo;
let angle = ((*mo).angle >> ANGLETOFINESHIFT) as usize;
let x1 = (*mo).x;
let y1 = (*mo).y;
let range_int = (USERANGE as i32) >> FRACBITS;
let x2 = x1.wrapping_add(range_int.wrapping_mul(finesine[angle + QUARTER_FINE]));
let y2 = y1.wrapping_add(range_int.wrapping_mul(finesine[angle]));
P_PathTraverse(
x1,
y1,
x2,
y2,
PT_ADDLINES as c_int,
Some(std::mem::transmute(PTR_UseTraverse as unsafe extern "C" fn(*mut _) -> c_int)),
);
}
}
// --- PIT_RadiusAttack ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_RadiusAttack(thing: *mut mobj_t) -> c_int {
unsafe {
if ((*thing).flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
if (*thing).type_ == MT_CYBORG || (*thing).type_ == MT_SPIDER {
return 1;
}
let dx = ((*thing).x.wrapping_sub((*bombspot).x)).wrapping_abs();
let dy = ((*thing).y.wrapping_sub((*bombspot).y)).wrapping_abs();
let mut dist = if dx > dy { dx } else { dy };
dist = (dist.wrapping_sub((*thing).radius)) >> FRACBITS;
if dist < 0 {
dist = 0;
}
if dist >= bombdamage {
return 1;
}
if P_CheckSight(thing as *mut _, bombspot as *mut _) != 0 {
P_DamageMobj(thing, bombspot, bombsource, bombdamage - dist);
}
1
}
}
// --- P_RadiusAttack ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RadiusAttack(spot: *mut mobj_t, source: *mut mobj_t, damage: c_int) {
unsafe {
let max_rad = MAXRADIUS as i32;
let dist = (damage + max_rad) << FRACBITS;
let shift = MAPBLOCKSHIFT as u32;
let yh = ((*spot).y.wrapping_add(dist).wrapping_sub(bmaporgy)) >> shift;
let yl = ((*spot).y.wrapping_sub(dist).wrapping_sub(bmaporgy)) >> shift;
let xh = ((*spot).x.wrapping_add(dist).wrapping_sub(bmaporgx)) >> shift;
let xl = ((*spot).x.wrapping_sub(dist).wrapping_sub(bmaporgx)) >> shift;
bombspot = spot;
bombsource = source;
bombdamage = damage;
for y in yl..=yh {
for x in xl..=xh {
P_BlockThingsIterator(x, y, Some(std::mem::transmute(PIT_RadiusAttack as unsafe extern "C" fn(*mut _) -> c_int)));
}
}
}
}
// --- PIT_ChangeSector ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_ChangeSector(thing: *mut mobj_t) -> c_int {
unsafe {
if P_ThingHeightClip(thing) != 0 {
return 1;
}
if (*thing).health <= 0 {
P_SetMobjState(thing as *mut _, S_GIBS);
(*thing).flags &= !(MF_SOLID as c_int);
(*thing).height = 0;
(*thing).radius = 0;
return 1;
}
if ((*thing).flags as u32 & MF_DROPPED) != 0 {
P_RemoveMobj(thing as *mut _);
return 1;
}
if ((*thing).flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
nofit = 1;
if crushchange != 0 && (leveltime & 3) == 0 {
P_DamageMobj(thing, ptr::null_mut(), ptr::null_mut(), 10);
let mo = P_SpawnMobj(
(*thing).x,
(*thing).y,
(*thing).z + (*thing).height / 2,
MT_BLOOD,
);
(*mo).momx = (p_random() - p_random()) << 12;
(*mo).momy = (p_random() - p_random()) << 12;
}
1
}
}
// --- P_ChangeSector ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ChangeSector(sector: *mut sector_t, crunch: c_int) -> c_int {
unsafe {
nofit = 0;
crushchange = crunch;
let bbox_ptr = (&raw const (*sector).blockbox) as *const c_int;
let xl = (*bbox_ptr.add(BOXLEFT));
let xh = (*bbox_ptr.add(BOXRIGHT));
let yl = (*bbox_ptr.add(BOXBOTTOM));
let yh = (*bbox_ptr.add(BOXTOP));
for x in xl..=xh {
for y in yl..=yh {
P_BlockThingsIterator(x, y, Some(std::mem::transmute(PIT_ChangeSector as unsafe extern "C" fn(*mut _) -> c_int)));
}
}
nofit
}
}