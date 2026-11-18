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
use crate::p_sight::P_CheckSight;
use crate::p_spec::{P_CrossSpecialLine, P_ShootSpecialLine};
use crate::p_switch::P_UseSpecialLine;
use crate::p_tick::get_leveltime;
use crate::r_bsp::subsector_t;
use crate::r_main::{R_PointInSubsector, R_PointToAngle2, validcount};
use crate::r_sky::get_sky_flat_num;
use crate::s_sound::S_StartSound;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};

static mut topslope: i32 = 0;
static mut bottomslope: i32 = 0;

pub const MAXSPECIALCROSS: usize = 8;
const QUARTER_FINE: usize = FINEANGLES / 4;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_StompThing(thing: *mut mobj_t) -> c_int {
    let Some(th) = (unsafe { thing.as_ref() }) else { return 1 };
    unsafe {
        if (th.flags as u32 & MF_SHOOTABLE) == 0 {
            return 1;
        }

        let tm = &*tmthing;
        let blockdist = th.radius + tm.radius;
        if (th.x.wrapping_sub(tmx)).wrapping_abs() >= blockdist
            || (th.y.wrapping_sub(tmy)).wrapping_abs() >= blockdist
        {
            return 1;
        }

        if thing == tmthing {
            return 1;
        }

        if tm.player.is_null() && gamemap != 30 {
            return 0;
        }

        P_DamageMobj(thing, tmthing, tmthing, 10000);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TeleportMove(thing: *mut mobj_t, x: i32, y: i32) -> c_int {
    let Some(th) = (unsafe { thing.as_mut() }) else { return 0 };
    unsafe {
        tmthing = thing;
        tmflags = th.flags;
        tmx = x;
        tmy = y;

        let rad = th.radius;
        let bbox = (&raw mut tmbbox).as_mut().unwrap();
        bbox[BOXTOP] = y.wrapping_add(rad);
        bbox[BOXBOTTOM] = y.wrapping_sub(rad);
        bbox[BOXRIGHT] = x.wrapping_add(rad);
        bbox[BOXLEFT] = x.wrapping_sub(rad);

        let newsubsec = R_PointInSubsector(x, y);
        ceilingline = ptr::null_mut();

        let sec = &*(*newsubsec).sector;
        tmfloorz = sec.floorheight;
        tmdropoffz = tmfloorz;
        tmceilingz = sec.ceilingheight;

        validcount += 1;
        numspechit = 0;

        let max_rad = MAXRADIUS as i32;
        let shift = MAPBLOCKSHIFT as u32;
        let xl = (bbox[BOXLEFT].wrapping_sub(bmaporgx).wrapping_sub(max_rad)) >> shift;
        let xh = (bbox[BOXRIGHT].wrapping_sub(bmaporgx).wrapping_add(max_rad)) >> shift;
        let yl = (bbox[BOXBOTTOM].wrapping_sub(bmaporgy).wrapping_sub(max_rad)) >> shift;
        let yh = (bbox[BOXTOP].wrapping_sub(bmaporgy).wrapping_add(max_rad)) >> shift;

        for bx in xl..=xh {
            for by in yl..=yh {
                if P_BlockThingsIterator(bx, by, Some(PIT_StompThing)) == 0 {
                    return 0;
                }
            }
        }

        P_UnsetThingPosition(thing as *mut _);

        th.floorz = tmfloorz;
        th.ceilingz = tmceilingz;
        th.x = x;
        th.y = y;

        P_SetThingPosition(thing as *mut _);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_CheckLine(ld: *mut line_t) -> c_int {
    let Some(line) = (unsafe { ld.as_ref() }) else { return 1 };
    unsafe {
        let bbox = (&raw mut tmbbox).as_mut().unwrap();

        if bbox[BOXRIGHT] <= line.bbox[BOXLEFT]
            || bbox[BOXLEFT] >= line.bbox[BOXRIGHT]
            || bbox[BOXTOP] <= line.bbox[BOXBOTTOM]
            || bbox[BOXBOTTOM] >= line.bbox[BOXTOP]
        {
            return 1;
        }

        if P_BoxOnLineSide(bbox.as_mut_ptr(), ld as *mut _) != -1 {
            return 1;
        }

        if line.backsector.is_null() {
            return 0;
        }

        let tm = &*tmthing;
        if (tm.flags as u32 & MF_MISSILE) == 0 {
            if (line.flags as u32 & ML_BLOCKING) != 0 {
                return 0;
            }
            if tm.player.is_null() && (line.flags as u32 & ML_BLOCKMONSTERS) != 0 {
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

        if line.special != 0 && (numspechit as usize) < MAXSPECIALCROSS {
            let spec_arr = (&raw mut spechit).as_mut().unwrap();
            spec_arr[numspechit as usize] = ld;
            numspechit += 1;
        }

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_CheckThing(thing: *mut mobj_t) -> c_int {
    let Some(th) = (unsafe { thing.as_ref() }) else { return 1 };
    unsafe {
        if (th.flags as u32 & (MF_SOLID | MF_SPECIAL | MF_SHOOTABLE)) == 0 {
            return 1;
        }

        let tm = &mut *tmthing;
        let blockdist = th.radius + tm.radius;
        if (th.x.wrapping_sub(tmx)).wrapping_abs() >= blockdist
            || (th.y.wrapping_sub(tmy)).wrapping_abs() >= blockdist
        {
            return 1;
        }

        if thing == tmthing {
            return 1;
        }

        if (tm.flags as u32 & MF_SKULLFLY) != 0 {
            let info = &*tm.info;
            let damage = ((p_random() % 8) + 1) * info.damage;
            P_DamageMobj(thing, tmthing, tmthing, damage);

            tm.flags &= !(MF_SKULLFLY as c_int);
            tm.momx = 0;
            tm.momy = 0;
            tm.momz = 0;

            P_SetMobjState(tmthing as *mut _, info.spawnstate as _);
            return 0;
        }

        if (tm.flags as u32 & MF_MISSILE) != 0 {
            if tm.z > th.z + th.height {
                return 1;
            }
            if tm.z + tm.height < th.z {
                return 1;
            }

            let target = tm.target;
            if let Some(tgt) = target.as_ref() {
                let same_species = tgt.type_ == th.type_
                    || (tgt.type_ == MT_KNIGHT && th.type_ == MT_BRUISER)
                    || (tgt.type_ == MT_BRUISER && th.type_ == MT_KNIGHT);

                if same_species {
                    if thing == target {
                        return 1;
                    }
                    if th.type_ != MT_PLAYER {
                        return 0;
                    }
                }
            }

            if (th.flags as u32 & MF_SHOOTABLE) == 0 {
                return ((th.flags as u32 & MF_SOLID) == 0) as c_int;
            }

            let info = &*tm.info;
            let damage = ((p_random() % 8) + 1) * info.damage;
            P_DamageMobj(thing, tmthing, target, damage);
return 0;
}
if (th.flags as u32 & MF_SPECIAL) != 0 {
let solid = (th.flags as u32 & MF_SOLID) != 0;
if (tmflags as u32 & MF_PICKUP) != 0 {
P_TouchSpecialThing(thing, tmthing);
}
return (!solid) as c_int;
}
((th.flags as u32 & MF_SOLID) == 0) as c_int
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckPosition(thing: *mut mobj_t, x: i32, y: i32) -> c_int {
let Some(th) = (unsafe { thing.as_ref() }) else { return 0 };
unsafe {
tmthing = thing;
tmflags = th.flags;
tmx = x;
tmy = y;
let rad = th.radius;
let bbox = (&raw mut tmbbox).as_mut().unwrap();
bbox[BOXTOP] = y.wrapping_add(rad);
bbox[BOXBOTTOM] = y.wrapping_sub(rad);
bbox[BOXRIGHT] = x.wrapping_add(rad);
bbox[BOXLEFT] = x.wrapping_sub(rad);
let newsubsec = R_PointInSubsector(x, y);
ceilingline = ptr::null_mut();
let sec = &*(*newsubsec).sector;
tmfloorz = sec.floorheight;
tmdropoffz = tmfloorz;
tmceilingz = sec.ceilingheight;
validcount += 1;
numspechit = 0;
if (tmflags as u32 & MF_NOCLIP) != 0 {
return 1;
}
let max_rad = MAXRADIUS as i32;
let shift = MAPBLOCKSHIFT as u32;
let mut xl = (bbox[BOXLEFT].wrapping_sub(bmaporgx).wrapping_sub(max_rad)) >> shift;
let mut xh = (bbox[BOXRIGHT].wrapping_sub(bmaporgx).wrapping_add(max_rad)) >> shift;
let mut yl = (bbox[BOXBOTTOM].wrapping_sub(bmaporgy).wrapping_sub(max_rad)) >> shift;
let mut yh = (bbox[BOXTOP].wrapping_sub(bmaporgy).wrapping_add(max_rad)) >> shift;
for bx in xl..=xh {
for by in yl..=yh {
if P_BlockThingsIterator(bx, by, Some(PIT_CheckThing)) == 0 {
return 0;
}
}
}
xl = (bbox[BOXLEFT].wrapping_sub(bmaporgx)) >> shift;
xh = (bbox[BOXRIGHT].wrapping_sub(bmaporgx)) >> shift;
yl = (bbox[BOXBOTTOM].wrapping_sub(bmaporgy)) >> shift;
yh = (bbox[BOXTOP].wrapping_sub(bmaporgy)) >> shift;
for bx in xl..=xh {
for by in yl..=yh {
if P_BlockLinesIterator(bx, by, Some(PIT_CheckLine)) == 0 {
return 0;
}
}
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TryMove(thing: *mut mobj_t, x: i32, y: i32) -> c_int {
let Some(th) = (unsafe { thing.as_mut() }) else { return 0 };
unsafe {
floatok = 0;
if P_CheckPosition(thing, x, y) == 0 {
return 0;
}
let flags = th.flags as u32;
let frac_u = FRACUNIT as i32;
if (flags & MF_NOCLIP) == 0 {
if tmceilingz - tmfloorz < th.height {
return 0;
}
floatok = 1;
if (flags & MF_TELEPORT) == 0 && tmceilingz - th.z < th.height {
return 0;
}
if (flags & MF_TELEPORT) == 0 && tmfloorz - th.z > 24 * frac_u {
return 0;
}
if (flags & (MF_DROPOFF | MF_FLOAT)) == 0 && tmfloorz - tmdropoffz > 24 * frac_u {
return 0;
}
}
P_UnsetThingPosition(thing as *mut _);
let oldx = th.x;
let oldy = th.y;
th.floorz = tmfloorz;
th.ceilingz = tmceilingz;
th.x = x;
th.y = y;
P_SetThingPosition(thing as *mut _);
if (flags & (MF_TELEPORT | MF_NOCLIP)) == 0 {
let spec_arr = (&raw const spechit).as_ref().unwrap();
let lines_base = lines as *mut line_t;
while numspechit > 0 {
numspechit -= 1;
let ld = spec_arr[numspechit as usize];
let side = P_PointOnLineSide(th.x, th.y, ld as *mut _);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ThingHeightClip(thing: *mut mobj_t) -> c_int {
let Some(th) = (unsafe { thing.as_mut() }) else { return 0 };
unsafe {
let onfloor = th.z == th.floorz;
P_CheckPosition(thing, th.x, th.y);
th.floorz = tmfloorz;
th.ceilingz = tmceilingz;
if onfloor {
th.z = th.floorz;
} else if th.z + th.height > th.ceilingz {
th.z = th.ceilingz - th.height;
}
((th.ceilingz - th.floorz) >= th.height) as c_int
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_HitSlideLine(ld: *mut line_t) {
let Some(line) = (unsafe { ld.as_ref() }) else { return };
unsafe {
if line.slopetype == ST_HORIZONTAL {
tmymove = 0;
return;
}
if line.slopetype == ST_VERTICAL {
tmxmove = 0;
return;
}
let s_mo = &*slidemo;
let side = P_PointOnLineSide(s_mo.x, s_mo.y, ld as *mut _);
let mut lineangle = R_PointToAngle2(0, 0, line.dx, line.dy);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_SlideTraverse(in_ptr: *mut intercept_t) -> c_int {
let Some(inc) = (unsafe { in_ptr.as_ref() }) else { return 1 };
unsafe {
if inc.isaline == 0 {
doom_error("PTR_SlideTraverse: not a line?");
}
let li = inc.d.line as *mut line_t;
let line = &*li;
let s_mo = &*slidemo;
let mut is_blocking = false;
if (line.flags as u32 & ML_TWOSIDED) == 0 {
if P_PointOnLineSide(s_mo.x, s_mo.y, li as *mut _) != 0 {
return 1;
}
is_blocking = true;
} else {
P_LineOpening(li as *mut _);
let frac_u = FRACUNIT as i32;
if openrange < s_mo.height
|| opentop - s_mo.z < s_mo.height
|| openbottom - s_mo.z > 24 * frac_u
{
is_blocking = true;
}
}
if is_blocking {
if inc.frac < bestslidefrac {
secondslidefrac = bestslidefrac;
secondslideline = bestslideline;
bestslidefrac = inc.frac;
bestslideline = li;
}
return 0;
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SlideMove(mo: *mut mobj_t) {
let Some(m) = (unsafe { mo.as_mut() }) else { return };
unsafe {
slidemo = mo;
let mut hitcount = 0;
let frac_u = FRACUNIT as i32;
loop {
hitcount += 1;
if hitcount == 3 {
if P_TryMove(mo, m.x, m.y.wrapping_add(m.momy)) == 0 {
P_TryMove(mo, m.x.wrapping_add(m.momx), m.y);
}
return;
}
let rad = m.radius;
let (leadx, trailx) = if m.momx > 0 {
(m.x.wrapping_add(rad), m.x.wrapping_sub(rad))
} else {
(m.x.wrapping_sub(rad), m.x.wrapping_add(rad))
};
let (leady, traily) = if m.momy > 0 {
(m.y.wrapping_add(rad), m.y.wrapping_sub(rad))
} else {
(m.y.wrapping_sub(rad), m.y.wrapping_add(rad))
};
bestslidefrac = frac_u + 1;
let pt_lines = PT_ADDLINES as c_int;
P_PathTraverse(
leadx,
leady,
leadx.wrapping_add(m.momx),
leady.wrapping_add(m.momy),
pt_lines,
Some(PTR_SlideTraverse),
);
P_PathTraverse(
trailx,
leady,
trailx.wrapping_add(m.momx),
leady.wrapping_add(m.momy),
pt_lines,
Some(PTR_SlideTraverse),
);
P_PathTraverse(
leadx,
traily,
leadx.wrapping_add(m.momx),
traily.wrapping_add(m.momy),
pt_lines,
Some(PTR_SlideTraverse),
);
if bestslidefrac == frac_u + 1 {
if P_TryMove(mo, m.x, m.y.wrapping_add(m.momy)) == 0 {
P_TryMove(mo, m.x.wrapping_add(m.momx), m.y);
}
return;
}
bestslidefrac -= 0x800;
if bestslidefrac > 0 {
let newx = fixed_mul(m.momx, bestslidefrac);
let newy = fixed_mul(m.momy, bestslidefrac);
if P_TryMove(mo, m.x.wrapping_add(newx), m.y.wrapping_add(newy)) == 0 {
if P_TryMove(mo, m.x, m.y.wrapping_add(m.momy)) == 0 {
P_TryMove(mo, m.x.wrapping_add(m.momx), m.y);
}
return;
}
}
bestslidefrac = (frac_u - (bestslidefrac + 0x800)).min(frac_u);
if bestslidefrac <= 0 {
return;
}
tmxmove = fixed_mul(m.momx, bestslidefrac);
tmymove = fixed_mul(m.momy, bestslidefrac);
P_HitSlideLine(bestslideline);
m.momx = tmxmove;
m.momy = tmymove;
if P_TryMove(mo, m.x.wrapping_add(tmxmove), m.y.wrapping_add(tmymove)) != 0 {
return;
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_AimTraverse(in_ptr: *mut intercept_t) -> c_int {
let Some(inc) = (unsafe { in_ptr.as_ref() }) else { return 1 };
unsafe {
if inc.isaline != 0 {
let li = inc.d.line as *mut line_t;
let line = &*li;
if (line.flags as u32 & ML_TWOSIDED) == 0 {
return 0;
}
P_LineOpening(li as *mut _);
if openbottom >= opentop {
return 0;
}
let dist = fixed_mul(attackrange, inc.frac);
let front = &*line.frontsector;
let back = &*line.backsector;
if front.floorheight != back.floorheight {
let slope = fixed_div(openbottom - shootz, dist);
if slope > bottomslope {
bottomslope = slope;
}
}
if front.ceilingheight != back.ceilingheight {
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
let th_ptr = inc.d.thing as *mut mobj_t;
if th_ptr == shootthing {
return 1;
}
let th = &*th_ptr;
if (th.flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
let dist = fixed_mul(attackrange, inc.frac);
let mut thingtopslope = fixed_div(th.z + th.height - shootz, dist);
if thingtopslope < bottomslope {
return 1;
}
let mut thingbottomslope = fixed_div(th.z - shootz, dist);
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
linetarget = th_ptr;
0
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_ShootTraverse(in_ptr: *mut intercept_t) -> c_int {
let Some(inc) = (unsafe { in_ptr.as_ref() }) else { return 1 };
unsafe {
let frac_u = FRACUNIT as i32;
if inc.isaline != 0 {
let li = inc.d.line as *mut line_t;
let line = &*li;
if line.special != 0 {
P_ShootSpecialLine(shootthing as *mut _, li as *mut _);
}
let mut hit_line = false;
if (line.flags as u32 & ML_TWOSIDED) == 0 {
hit_line = true;
} else {
P_LineOpening(li as *mut _);
let dist = fixed_mul(attackrange, inc.frac);
let front = &*line.frontsector;
let back = &*line.backsector;
if front.floorheight != back.floorheight {
let slope = fixed_div(openbottom - shootz, dist);
if slope > aimslope {
hit_line = true;
}
}
if !hit_line && front.ceilingheight != back.ceilingheight {
let slope = fixed_div(opentop - shootz, dist);
if slope < aimslope {
hit_line = true;
}
}
}
if !hit_line {
return 1;
}
let frac = inc.frac - fixed_div(4 * frac_u, attackrange);
let x = trace.x.wrapping_add(fixed_mul(trace.dx, frac));
let y = trace.y.wrapping_add(fixed_mul(trace.dy, frac));
let z = shootz.wrapping_add(fixed_mul(aimslope, fixed_mul(frac, attackrange)));
let front = &*line.frontsector;
if (front.ceilingpic as c_int) == get_sky_flat_num() {
if z > front.ceilingheight {
return 0;
}
if let Some(back) = line.backsector.as_ref() {
if (back.ceilingpic as c_int) == get_sky_flat_num() {
return 0;
}
}
}
P_SpawnPuff(x, y, z);
return 0;
}
let th_ptr = inc.d.thing as *mut mobj_t;
if th_ptr == shootthing {
return 1;
}
let th = &*th_ptr;
if (th.flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
let dist = fixed_mul(attackrange, inc.frac);
let thingtopslope = fixed_div(th.z + th.height - shootz, dist);
if thingtopslope < aimslope {
return 1;
}
let thingbottomslope = fixed_div(th.z - shootz, dist);
if thingbottomslope > aimslope {
return 1;
}
let frac = inc.frac - fixed_div(10 * frac_u, attackrange);
let x = trace.x.wrapping_add(fixed_mul(trace.dx, frac));
let y = trace.y.wrapping_add(fixed_mul(trace.dy, frac));
let z = shootz.wrapping_add(fixed_mul(aimslope, fixed_mul(frac, attackrange)));
if (th.flags as u32 & MF_NOBLOOD) != 0 {
P_SpawnPuff(x, y, z);
} else {
P_SpawnBlood(x, y, z, la_damage);
}
if la_damage != 0 {
P_DamageMobj(th_ptr, shootthing, shootthing, la_damage);
}
0
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AimLineAttack(t1: *mut mobj_t, mut angle: u32, distance: i32) -> i32 {
let Some(src) = (unsafe { t1.as_ref() }) else { return 0 };
unsafe {
angle >>= ANGLETOFINESHIFT;
let idx = angle as usize;
shootthing = t1;
let dist_int = distance >> FRACBITS;
let x2 = src.x.wrapping_add(dist_int.wrapping_mul(finesine[idx + QUARTER_FINE]));
let y2 = src.y.wrapping_add(dist_int.wrapping_mul(finesine[idx]));
let frac_u = FRACUNIT as i32;
shootz = src.z + (src.height >> 1) + 8 * frac_u;
topslope = 100 * frac_u / 160;
bottomslope = -100 * frac_u / 160;
attackrange = distance;
linetarget = ptr::null_mut();
let flags = (PT_ADDLINES | PT_ADDTHINGS) as c_int;
P_PathTraverse(src.x, src.y, x2, y2, flags, Some(PTR_AimTraverse));
if !linetarget.is_null() { aimslope } else { 0 }
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LineAttack(
t1: *mut mobj_t,
mut angle: u32,
distance: i32,
slope: i32,
damage: c_int,
) {
let Some(src) = (unsafe { t1.as_ref() }) else { return };
unsafe {
angle >>= ANGLETOFINESHIFT;
let idx = angle as usize;
shootthing = t1;
la_damage = damage;
let dist_int = distance >> FRACBITS;
let x2 = src.x.wrapping_add(dist_int.wrapping_mul(finesine[idx + QUARTER_FINE]));
let y2 = src.y.wrapping_add(dist_int.wrapping_mul(finesine[idx]));
let frac_u = FRACUNIT as i32;
shootz = src.z + (src.height >> 1) + 8 * frac_u;
attackrange = distance;
aimslope = slope;
let flags = (PT_ADDLINES | PT_ADDTHINGS) as c_int;
P_PathTraverse(src.x, src.y, x2, y2, flags, Some(PTR_ShootTraverse));
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PTR_UseTraverse(in_ptr: *mut intercept_t) -> c_int {
let Some(inc) = (unsafe { in_ptr.as_ref() }) else { return 1 };
unsafe {
let li = inc.d.line as *mut line_t;
if (*li).special == 0 {
P_LineOpening(li as *mut _);
if openrange <= 0 {
S_StartSound(usething as *mut c_void, sfx_noway as c_int);
return 0;
}
return 1;
}
let u_mo = &*usething;
let side = (P_PointOnLineSide(u_mo.x, u_mo.y, li as *mut _) == 1) as c_int;
P_UseSpecialLine(usething as *mut _, li as *mut _, side);
0
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UseLines(player: *mut player_t) {
let Some(p) = (unsafe { player.as_ref() }) else { return };
unsafe {
let mo_ptr = p.mo as *mut mobj_t;
let mo = &*mo_ptr;
usething = mo_ptr;
let angle = (mo.angle >> ANGLETOFINESHIFT) as usize;
let x1 = mo.x;
let y1 = mo.y;
let range_int = (USERANGE as i32) >> FRACBITS;
let x2 = x1.wrapping_add(range_int.wrapping_mul(finesine[angle + QUARTER_FINE]));
let y2 = y1.wrapping_add(range_int.wrapping_mul(finesine[angle]));
P_PathTraverse(x1, y1, x2, y2, PT_ADDLINES as c_int, Some(PTR_UseTraverse));
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_RadiusAttack(thing: *mut mobj_t) -> c_int {
let Some(th) = (unsafe { thing.as_ref() }) else { return 1 };
unsafe {
if (th.flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
if th.type_ == MT_CYBORG || th.type_ == MT_SPIDER {
return 1;
}
let spot = &*bombspot;
let dx = (th.x.wrapping_sub(spot.x)).wrapping_abs();
let dy = (th.y.wrapping_sub(spot.y)).wrapping_abs();
let dist = ((dx.max(dy).wrapping_sub(th.radius)) >> FRACBITS).max(0);
if dist >= bombdamage {
return 1;
}
if P_CheckSight(thing as *mut _, bombspot as *mut _) != 0 {
P_DamageMobj(thing, bombspot, bombsource, bombdamage - dist);
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RadiusAttack(spot: *mut mobj_t, source: *mut mobj_t, damage: c_int) {
let Some(sp) = (unsafe { spot.as_ref() }) else { return };
unsafe {
let max_rad = MAXRADIUS as i32;
let dist = (damage + max_rad) << FRACBITS;
let shift = MAPBLOCKSHIFT as u32;
let yh = (sp.y.wrapping_add(dist).wrapping_sub(bmaporgy)) >> shift;
let yl = (sp.y.wrapping_sub(dist).wrapping_sub(bmaporgy)) >> shift;
let xh = (sp.x.wrapping_add(dist).wrapping_sub(bmaporgx)) >> shift;
let xl = (sp.x.wrapping_sub(dist).wrapping_sub(bmaporgx)) >> shift;
bombspot = spot;
bombsource = source;
bombdamage = damage;
for y in yl..=yh {
for x in xl..=xh {
P_BlockThingsIterator(x, y, Some(PIT_RadiusAttack));
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_ChangeSector(thing: *mut mobj_t) -> c_int {
let Some(th) = (unsafe { thing.as_mut() }) else { return 1 };
unsafe {
if P_ThingHeightClip(thing) != 0 {
return 1;
}
if th.health <= 0 {
P_SetMobjState(thing as *mut _, S_GIBS);
th.flags &= !(MF_SOLID as c_int);
th.height = 0;
th.radius = 0;
return 1;
}
if (th.flags as u32 & MF_DROPPED) != 0 {
P_RemoveMobj(thing as *mut _);
return 1;
}
if (th.flags as u32 & MF_SHOOTABLE) == 0 {
return 1;
}
nofit = 1;
if crushchange != 0 && (get_leveltime() & 3) == 0 {
P_DamageMobj(thing, ptr::null_mut(), ptr::null_mut(), 10);
let mo = P_SpawnMobj(th.x, th.y, th.z + th.height / 2, MT_BLOOD);
(*mo).momx = (p_random() - p_random()) << 12;
(*mo).momy = (p_random() - p_random()) << 12;
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ChangeSector(sector: *mut sector_t, crunch: c_int) -> c_int {
let Some(sec) = (unsafe { sector.as_ref() }) else { return 0 };
unsafe {
nofit = 0;
crushchange = crunch;
let xl = sec.blockbox[BOXLEFT];
let xh = sec.blockbox[BOXRIGHT];
let yl = sec.blockbox[BOXBOTTOM];
let yh = sec.blockbox[BOXTOP];
for x in xl..=xh {
for y in yl..=yh {
P_BlockThingsIterator(x, y, Some(PIT_ChangeSector));
}
}
nofit
}
}
