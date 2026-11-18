#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::c_int;
use std::ptr;

use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP};
use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::p_setup::{
    blocklinks, blockmap, blockmaplump, bmapheight, bmaporgx, bmaporgy, bmapwidth, lines,
};
use crate::r_bsp::subsector_t;
use crate::r_main::{R_PointInSubsector, validcount};

const MAX_INTERCEPTS_COUNT: usize = MAXINTERCEPTS as usize;

const EMPTY_INTERCEPT: intercept_t = intercept_t {
    frac: 0,
    isaline: 0,
    d: intercept_t__bindgen_ty_1 {
        thing: ptr::null_mut(),
    },
};

#[unsafe(no_mangle)]
pub static mut opentop: i32 = 0;

#[unsafe(no_mangle)]
pub static mut openbottom: i32 = 0;

#[unsafe(no_mangle)]
pub static mut openrange: i32 = 0;

#[unsafe(no_mangle)]
pub static mut lowfloor: i32 = 0;

#[unsafe(no_mangle)]
pub static mut intercepts: [intercept_t; MAX_INTERCEPTS_COUNT] =
    [EMPTY_INTERCEPT; MAX_INTERCEPTS_COUNT];

#[unsafe(no_mangle)]
pub static mut intercept_p: *mut intercept_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut trace: divline_t = divline_t {
    x: 0,
    y: 0,
    dx: 0,
    dy: 0,
};

#[unsafe(no_mangle)]
pub static mut earlyout: c_int = 0;

#[unsafe(no_mangle)]
pub static mut ptflags: c_int = 0;

#[inline]
pub fn p_aprox_distance(mut dx: i32, mut dy: i32) -> i32 {
    dx = dx.wrapping_abs();
    dy = dy.wrapping_abs();
    if dx < dy {
        dx.wrapping_add(dy).wrapping_sub(dx >> 1)
    } else {
        dx.wrapping_add(dy).wrapping_sub(dy >> 1)
    }
}

#[inline]
pub fn p_point_on_line_side(x: i32, y: i32, line: &line_t) -> c_int {
    let Some(v1) = (unsafe { line.v1.as_ref() }) else {
        return 0;
    };
    let ldx = line.dx;
    let ldy = line.dy;

    if ldx == 0 {
        if x <= v1.x {
            return (ldy > 0) as c_int;
        }
        return (ldy < 0) as c_int;
    }
    if ldy == 0 {
        if y <= v1.y {
            return (ldx < 0) as c_int;
        }
        return (ldx > 0) as c_int;
    }

    let dx = x.wrapping_sub(v1.x);
    let dy = y.wrapping_sub(v1.y);

    let left = fixed_mul(ldy >> FRACBITS, dx);
    let right = fixed_mul(dy, ldx >> FRACBITS);

    if right < left { 0 } else { 1 }
}

pub fn p_box_on_line_side(tmbox: &[i32; 4], ld: &line_t) -> c_int {
    let Some(v1) = (unsafe { ld.v1.as_ref() }) else {
        return 0;
    };
    let mut p1 = 0;
    let mut p2 = 0;

    match ld.slopetype {
        ST_HORIZONTAL => {
            p1 = (tmbox[BOXTOP] > v1.y) as c_int;
            p2 = (tmbox[BOXBOTTOM] > v1.y) as c_int;
            if ld.dx < 0 {
                p1 ^= 1;
                p2 ^= 1;
            }
        }
        ST_VERTICAL => {
            p1 = (tmbox[BOXRIGHT] < v1.x) as c_int;
            p2 = (tmbox[BOXLEFT] < v1.x) as c_int;
            if ld.dy < 0 {
                p1 ^= 1;
                p2 ^= 1;
            }
        }
        ST_POSITIVE => {
            p1 = p_point_on_line_side(tmbox[BOXLEFT], tmbox[BOXTOP], ld);
            p2 = p_point_on_line_side(tmbox[BOXRIGHT], tmbox[BOXBOTTOM], ld);
        }
        ST_NEGATIVE => {
            p1 = p_point_on_line_side(tmbox[BOXRIGHT], tmbox[BOXTOP], ld);
            p2 = p_point_on_line_side(tmbox[BOXLEFT], tmbox[BOXBOTTOM], ld);
        }
        _ => {}
    }

    if p1 == p2 { p1 } else { -1 }
}

#[inline]
pub fn p_point_on_divline_side(x: i32, y: i32, line: &divline_t) -> c_int {
    let lx = line.x;
    let ly = line.y;
    let ldx = line.dx;
    let ldy = line.dy;

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

    let left = fixed_mul(ldy >> 8, dx >> 8);
    let right = fixed_mul(dy >> 8, ldx >> 8);

    if right < left { 0 } else { 1 }
}

#[inline]
pub fn p_make_divline(li: &line_t) -> divline_t {
    let (vx, vy) = unsafe {
        li.v1.as_ref().map(|v| (v.x, v.y)).unwrap_or((0, 0))
    };
    divline_t {
        x: vx,
        y: vy,
        dx: li.dx,
        dy: li.dy,
    }
}

#[inline]
pub fn p_intercept_vector(v2: &divline_t, v1: &divline_t) -> i32 {
    let den =
        fixed_mul(v1.dy >> 8, v2.dx).wrapping_sub(fixed_mul(v1.dx >> 8, v2.dy));

    if den == 0 {
        return 0;
    }

    let num = fixed_mul(v1.x.wrapping_sub(v2.x) >> 8, v1.dy)
        .wrapping_add(fixed_mul(v2.y.wrapping_sub(v1.y) >> 8, v1.dx));

    fixed_div(num, den)
}

#[unsafe(no_mangle)]
pub extern "C" fn P_AproxDistance(dx: i32, dy: i32) -> i32 {
    p_aprox_distance(dx, dy)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PointOnLineSide(x: i32, y: i32, line: *mut line_t) -> c_int {
    match unsafe { line.as_ref() } {
        Some(ln) => p_point_on_line_side(x, y, ln),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BoxOnLineSide(tmbox: *mut i32, ld: *mut line_t) -> c_int {
    match (unsafe { (tmbox as *const [i32; 4]).as_ref() }, unsafe { ld.as_ref() }) {
        (Some(b), Some(l)) => p_box_on_line_side(b, l),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PointOnDivlineSide(x: i32, y: i32, line: *mut divline_t) -> c_int {
    match unsafe { line.as_ref() } {
        Some(ln) => p_point_on_divline_side(x, y, ln),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MakeDivline(li: *mut line_t, dl: *mut divline_t) {
    if let (Some(l), Some(d)) = (unsafe { li.as_ref() }, unsafe { dl.as_mut() }) {
        *d = p_make_divline(l);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InterceptVector(v2: *mut divline_t, v1: *mut divline_t) -> i32 {
    match (unsafe { v2.as_ref() }, unsafe { v1.as_ref() }) {
        (Some(a), Some(b)) => p_intercept_vector(a, b),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LineOpening(linedef: *mut line_t) {
    let Some(ld) = (unsafe { linedef.as_ref() }) else { return };
    unsafe {
        if ld.sidenum[1] == -1 {
            openrange = 0;
            return;
        }

        let front = &*ld.frontsector;
        let back = &*ld.backsector;

        opentop = front.ceilingheight.min(back.ceilingheight);

        if front.floorheight > back.floorheight {
            openbottom = front.floorheight;
            lowfloor = back.floorheight;
        } else {
            openbottom = back.floorheight;
            lowfloor = front.floorheight;
        }

        openrange = opentop.wrapping_sub(openbottom);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnsetThingPosition(thing: *mut mobj_t) {
    let Some(th) = (unsafe { thing.as_mut() }) else { return };
    unsafe {
        if (th.flags as u32 & MF_NOSECTOR) == 0 {
            if !th.snext.is_null() {
                (*th.snext).sprev = th.sprev;
            }
            if !th.sprev.is_null() {
                (*th.sprev).snext = th.snext;
            } else {
                let sub = &*(th.subsector as *const subsector_t);
                let sec = &mut *sub.sector;
                sec.thinglist = th.snext as *mut _;
            }
        }

        if (th.flags as u32 & MF_NOBLOCKMAP) == 0 {
            if !th.bnext.is_null() {
                (*th.bnext).bprev = th.bprev;
            }
            if !th.bprev.is_null() {
                (*th.bprev).bnext = th.bnext;
            } else {
                let shift = MAPBLOCKSHIFT as u32;
                let blockx = (th.x.wrapping_sub(bmaporgx)) >> shift;
                let blocky = (th.y.wrapping_sub(bmaporgy)) >> shift;

                if blockx >= 0 && blockx < bmapwidth && blocky >= 0 && blocky < bmapheight {
                    let idx = (blocky * bmapwidth + blockx) as usize;
                    *blocklinks.add(idx) = th.bnext as *mut _;
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetThingPosition(thing: *mut mobj_t) {
    let Some(th) = (unsafe { thing.as_mut() }) else { return };
    unsafe {
        let ss = R_PointInSubsector(th.x, th.y);
        th.subsector = ss as *mut _;

        if (th.flags as u32 & MF_NOSECTOR) == 0 {
            let sec = &mut *(*ss).sector;
            th.sprev = ptr::null_mut();
            th.snext = sec.thinglist as *mut _;

            if !sec.thinglist.is_null() {
                (*(sec.thinglist as *mut mobj_t)).sprev = thing;
            }

            sec.thinglist = thing as *mut _;
        }

        if (th.flags as u32 & MF_NOBLOCKMAP) == 0 {
            let shift = MAPBLOCKSHIFT as u32;
            let blockx = (th.x.wrapping_sub(bmaporgx)) >> shift;
            let blocky = (th.y.wrapping_sub(bmaporgy)) >> shift;

            if blockx >= 0 && blockx < bmapwidth && blocky >= 0 && blocky < bmapheight {
                let idx = (blocky * bmapwidth + blockx) as usize;
                let link = blocklinks.add(idx) as *mut *mut mobj_t;

                th.bprev = ptr::null_mut();
                th.bnext = *link;
                if !(*link).is_null() {
                    (*(*link)).bprev = thing;
                }
                *link = thing;
            } else {
                th.bnext = ptr::null_mut();
                th.bprev = ptr::null_mut();
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BlockLinesIterator(
    x: c_int,
    y: c_int,
    func: Option<unsafe extern "C" fn(*mut line_t) -> c_int>,
) -> c_int {
    unsafe {
        if x < 0 || y < 0 || x >= bmapwidth || y >= bmapheight {
            return 1;
}
let offset_idx = (y * bmapwidth + x) as usize;
let offset = *blockmap.add(offset_idx) as isize;
let mut list = blockmaplump.offset(offset);
let lines_base = lines as *mut line_t;
while *list != -1 {
let ld = &mut *lines_base.add(*list as usize);
if ld.validcount == validcount {
list = list.add(1);
continue;
}
ld.validcount = validcount;
if let Some(f) = func {
if f(ld as *mut line_t) == 0 {
return 0;
}
}
list = list.add(1);
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BlockThingsIterator(
x: c_int,
y: c_int,
func: Option<unsafe extern "C" fn(*mut mobj_t) -> c_int>,
) -> c_int {
unsafe {
if x < 0 || y < 0 || x >= bmapwidth || y >= bmapheight {
return 1;
}
let idx = (y * bmapwidth + x) as usize;
let mut mobj = *blocklinks.add(idx) as *mut mobj_t;
while !mobj.is_null() {
if let Some(f) = func {
if f(mobj) == 0 {
return 0;
}
}
mobj = (*mobj).bnext;
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_AddLineIntercepts(ld: *mut line_t) -> c_int {
let Some(line) = (unsafe { ld.as_ref() }) else { return 1 };
let tr = unsafe { *(&raw const trace) };
let frac_u = FRACUNIT as i32;
let limit = frac_u * 16;
let (s1, s2) = if tr.dx > limit || tr.dy > limit || tr.dx < -limit || tr.dy < -limit {
let (Some(v1), Some(v2)) = (unsafe { line.v1.as_ref() }, unsafe { line.v2.as_ref() })
else {
return 1;
};
(
p_point_on_divline_side(v1.x, v1.y, &tr),
p_point_on_divline_side(v2.x, v2.y, &tr),
)
} else {
(
p_point_on_line_side(tr.x, tr.y, line),
p_point_on_line_side(tr.x.wrapping_add(tr.dx), tr.y.wrapping_add(tr.dy), line),
)
};
if s1 == s2 {
return 1;
}
let dl = p_make_divline(line);
let frac = p_intercept_vector(&tr, &dl);
if frac < 0 {
return 1;
}
unsafe {
if earlyout != 0 && frac < frac_u && line.backsector.is_null() {
return 0;
}
let slot = &mut *intercept_p;
slot.frac = frac;
slot.isaline = 1;
slot.d.line = ld;
intercept_p = intercept_p.add(1);
}
1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_AddThingIntercepts(thing: *mut mobj_t) -> c_int {
let Some(th) = (unsafe { thing.as_ref() }) else { return 1 };
let tr = unsafe { *(&raw const trace) };
let tracepositive = (tr.dx ^ tr.dy) > 0;
let tx = th.x;
let ty = th.y;
let rad = th.radius;
let (x1, y1, x2, y2) = if tracepositive {
(
tx.wrapping_sub(rad),
ty.wrapping_add(rad),
tx.wrapping_add(rad),
ty.wrapping_sub(rad),
)
} else {
(
tx.wrapping_sub(rad),
ty.wrapping_sub(rad),
tx.wrapping_add(rad),
ty.wrapping_add(rad),
)
};
let s1 = p_point_on_divline_side(x1, y1, &tr);
let s2 = p_point_on_divline_side(x2, y2, &tr);
if s1 == s2 {
return 1;
}
let dl = divline_t {
x: x1,
y: y1,
dx: x2.wrapping_sub(x1),
dy: y2.wrapping_sub(y1),
};
let frac = p_intercept_vector(&tr, &dl);
if frac < 0 {
return 1;
}
unsafe {
let slot = &mut *intercept_p;
slot.frac = frac;
slot.isaline = 0;
slot.d.thing = thing;
intercept_p = intercept_p.add(1);
}
1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TraverseIntercepts(func: traverser_t, maxfrac: i32) -> c_int {
unsafe {
let base = (&raw mut intercepts) as *mut intercept_t;
let len = intercept_p.offset_from(base).max(0) as usize;
let list = std::slice::from_raw_parts_mut(base, len);
for _ in 0..len {
let mut dist = i32::MAX;
let mut best_idx = None;
for (idx, item) in list.iter().enumerate() {
if item.frac < dist {
dist = item.frac;
best_idx = Some(idx);
}
}
let Some(idx) = best_idx else { break };
if dist > maxfrac {
return 1;
}
if let Some(f) = func {
if f(&mut list[idx] as *mut intercept_t) == 0 {
return 0;
}
}
list[idx].frac = i32::MAX;
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PathTraverse(
mut x1: i32,
mut y1: i32,
mut x2: i32,
mut y2: i32,
flags: c_int,
trav: traverser_t,
) -> c_int {
unsafe {
let frac_u = FRACUNIT as i32;
let blk_size = MAPBLOCKSIZE as i32;
let blk_shift = MAPBLOCKSHIFT as u32;
let btofrac = MAPBTOFRAC as u32;
earlyout = flags & (PT_EARLYOUT as c_int);
validcount += 1;
intercept_p = (&raw mut intercepts) as *mut intercept_t;
if ((x1.wrapping_sub(bmaporgx)) & (blk_size - 1)) == 0 {
x1 = x1.wrapping_add(frac_u);
}
if ((y1.wrapping_sub(bmaporgy)) & (blk_size - 1)) == 0 {
y1 = y1.wrapping_add(frac_u);
}
trace.x = x1;
trace.y = y1;
trace.dx = x2.wrapping_sub(x1);
trace.dy = y2.wrapping_sub(y1);
x1 = x1.wrapping_sub(bmaporgx);
y1 = y1.wrapping_sub(bmaporgy);
let xt1 = x1 >> blk_shift;
let yt1 = y1 >> blk_shift;
x2 = x2.wrapping_sub(bmaporgx);
y2 = y2.wrapping_sub(bmaporgy);
let xt2 = x2 >> blk_shift;
let yt2 = y2 >> blk_shift;
let (mapxstep, partial_y, ystep) = if xt2 > xt1 {
(
1,
frac_u - ((x1 >> btofrac) & (frac_u - 1)),
fixed_div(y2.wrapping_sub(y1), x2.wrapping_sub(x1).wrapping_abs()),
)
} else if xt2 < xt1 {
(
-1,
(x1 >> btofrac) & (frac_u - 1),
fixed_div(y2.wrapping_sub(y1), x2.wrapping_sub(x1).wrapping_abs()),
)
} else {
(0, frac_u, 256 * frac_u)
};
let mut yintercept = (y1 >> btofrac).wrapping_add(fixed_mul(partial_y, ystep));
let (mapystep, partial_x, xstep) = if yt2 > yt1 {
(
1,
frac_u - ((y1 >> btofrac) & (frac_u - 1)),
fixed_div(x2.wrapping_sub(x1), y2.wrapping_sub(y1).wrapping_abs()),
)
} else if yt2 < yt1 {
(
-1,
(y1 >> btofrac) & (frac_u - 1),
fixed_div(x2.wrapping_sub(x1), y2.wrapping_sub(y1).wrapping_abs()),
)
} else {
(0, frac_u, 256 * frac_u)
};
let mut xintercept = (x1 >> btofrac).wrapping_add(fixed_mul(partial_x, xstep));
let mut mapx = xt1;
let mut mapy = yt1;
for _ in 0..64 {
if (flags & (PT_ADDLINES as c_int)) != 0 {
if P_BlockLinesIterator(mapx, mapy, Some(PIT_AddLineIntercepts)) == 0 {
return 0;
}
}
if (flags & (PT_ADDTHINGS as c_int)) != 0 {
if P_BlockThingsIterator(mapx, mapy, Some(PIT_AddThingIntercepts)) == 0 {
return 0;
}
}
if mapx == xt2 && mapy == yt2 {
break;
}
if (yintercept >> FRACBITS) == mapy {
yintercept = yintercept.wrapping_add(ystep);
mapx += mapxstep;
} else if (xintercept >> FRACBITS) == mapx {
xintercept = xintercept.wrapping_add(xstep);
mapy += mapystep;
}
}
P_TraverseIntercepts(trav, frac_u)
}
}
