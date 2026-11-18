#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{c_int, c_short};
use std::ptr;

use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP};
use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::p_setup::{
    blocklinks, blockmap, blockmaplump, bmapheight, bmaporgx, bmaporgy, bmapwidth, lines,
};
use crate::r_bsp::subsector_t;
use crate::r_main::{R_PointInSubsector, validcount};

const MAX_INTERCEPTS_COUNT: usize = MAXINTERCEPTS as usize;

// --- Глобальные переменные p_maputl.c ---
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
    [unsafe { std::mem::zeroed() }; MAX_INTERCEPTS_COUNT];

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

// --- P_AproxDistance ---
#[unsafe(no_mangle)]
pub extern "C" fn P_AproxDistance(mut dx: i32, mut dy: i32) -> i32 {
    dx = dx.wrapping_abs();
    dy = dy.wrapping_abs();
    if dx < dy {
        dx.wrapping_add(dy).wrapping_sub(dx >> 1)
    } else {
        dx.wrapping_add(dy).wrapping_sub(dy >> 1)
    }
}

// --- P_PointOnLineSide ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PointOnLineSide(x: i32, y: i32, line: *mut line_t) -> c_int {
    unsafe {
        let v1 = (*line).v1;
        let ldx = (*line).dx;
        let ldy = (*line).dy;

        if ldx == 0 {
            if x <= (*v1).x {
                return (ldy > 0) as c_int;
            }
            return (ldy < 0) as c_int;
        }
        if ldy == 0 {
            if y <= (*v1).y {
                return (ldx < 0) as c_int;
            }
            return (ldx > 0) as c_int;
        }

        let dx = x.wrapping_sub((*v1).x);
        let dy = y.wrapping_sub((*v1).y);

        let left = fixed_mul(ldy >> FRACBITS, dx);
        let right = fixed_mul(dy, ldx >> FRACBITS);

        if right < left { 0 } else { 1 }
    }
}

// --- P_BoxOnLineSide ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BoxOnLineSide(tmbox: *mut i32, ld: *mut line_t) -> c_int {
    unsafe {
        let v1 = (*ld).v1;
        let mut p1 = 0;
        let mut p2 = 0;

        match (*ld).slopetype {
            ST_HORIZONTAL => {
                p1 = (*tmbox.add(BOXTOP) > (*v1).y) as c_int;
                p2 = (*tmbox.add(BOXBOTTOM) > (*v1).y) as c_int;
                if (*ld).dx < 0 {
                    p1 ^= 1;
                    p2 ^= 1;
                }
            }
            ST_VERTICAL => {
                p1 = (*tmbox.add(BOXRIGHT) < (*v1).x) as c_int;
                p2 = (*tmbox.add(BOXLEFT) < (*v1).x) as c_int;
                if (*ld).dy < 0 {
                    p1 ^= 1;
                    p2 ^= 1;
                }
            }
            ST_POSITIVE => {
                p1 = P_PointOnLineSide(*tmbox.add(BOXLEFT), *tmbox.add(BOXTOP), ld);
                p2 = P_PointOnLineSide(*tmbox.add(BOXRIGHT), *tmbox.add(BOXBOTTOM), ld);
            }
            ST_NEGATIVE => {
                p1 = P_PointOnLineSide(*tmbox.add(BOXRIGHT), *tmbox.add(BOXTOP), ld);
                p2 = P_PointOnLineSide(*tmbox.add(BOXLEFT), *tmbox.add(BOXBOTTOM), ld);
            }
            _ => {}
        }

        if p1 == p2 { p1 } else { -1 }
    }
}

// --- P_PointOnDivlineSide ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PointOnDivlineSide(x: i32, y: i32, line: *mut divline_t) -> c_int {
    unsafe {
        let lx = (*line).x;
        let ly = (*line).y;
        let ldx = (*line).dx;
        let ldy = (*line).dy;

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
}

// --- P_MakeDivline ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MakeDivline(li: *mut line_t, dl: *mut divline_t) {
    unsafe {
        let v1 = (*li).v1;
        (*dl).x = (*v1).x;
        (*dl).y = (*v1).y;
        (*dl).dx = (*li).dx;
        (*dl).dy = (*li).dy;
    }
}

// --- P_InterceptVector ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InterceptVector(v2: *mut divline_t, v1: *mut divline_t) -> i32 {
    unsafe {
        let den = fixed_mul((*v1).dy >> 8, (*v2).dx)
            .wrapping_sub(fixed_mul((*v1).dx >> 8, (*v2).dy));

        if den == 0 {
            return 0;
        }

        let num = fixed_mul((*v1).x.wrapping_sub((*v2).x) >> 8, (*v1).dy)
            .wrapping_add(fixed_mul((*v2).y.wrapping_sub((*v1).y) >> 8, (*v1).dx));

        fixed_div(num, den)
    }
}

// --- P_LineOpening ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LineOpening(linedef: *mut line_t) {
    unsafe {
        let s1 = *(&raw const (*linedef).sidenum as *const c_short).add(1);
        if s1 == -1 {
            openrange = 0;
            return;
        }

        let front = (*linedef).frontsector;
        let back = (*linedef).backsector;

        opentop = if (*front).ceilingheight < (*back).ceilingheight {
            (*front).ceilingheight
        } else {
            (*back).ceilingheight
        };

        if (*front).floorheight > (*back).floorheight {
            openbottom = (*front).floorheight;
            lowfloor = (*back).floorheight;
        } else {
            openbottom = (*back).floorheight;
            lowfloor = (*front).floorheight;
        }

        openrange = opentop.wrapping_sub(openbottom);
    }
}

// --- P_UnsetThingPosition ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnsetThingPosition(thing: *mut mobj_t) {
    unsafe {
        if ((*thing).flags as u32 & MF_NOSECTOR) == 0 {
            if !(*thing).snext.is_null() {
                (*(*thing).snext).sprev = (*thing).sprev;
            }
            if !(*thing).sprev.is_null() {
                (*(*thing).sprev).snext = (*thing).snext;
            } else {
                let sub = (*thing).subsector as *mut subsector_t;
                let sec = (*sub).sector;
                (*sec).thinglist = (*thing).snext as *mut _;
            }
        }

        if ((*thing).flags as u32 & MF_NOBLOCKMAP) == 0 {
            if !(*thing).bnext.is_null() {
                (*(*thing).bnext).bprev = (*thing).bprev;
            }
            if !(*thing).bprev.is_null() {
                (*(*thing).bprev).bnext = (*thing).bnext;
            } else {
                let shift = MAPBLOCKSHIFT as u32;
                let blockx = ((*thing).x.wrapping_sub(bmaporgx)) >> shift;
                let blocky = ((*thing).y.wrapping_sub(bmaporgy)) >> shift;

                if blockx >= 0 && blockx < bmapwidth && blocky >= 0 && blocky < bmapheight {
                    let idx = (blocky * bmapwidth + blockx) as usize;
                    *blocklinks.add(idx) = (*thing).bnext as *mut _;
                }
            }
        }
    }
}

// --- P_SetThingPosition ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetThingPosition(thing: *mut mobj_t) {
    unsafe {
        let ss = R_PointInSubsector((*thing).x, (*thing).y);
        (*thing).subsector = ss as *mut _;

        if ((*thing).flags as u32 & MF_NOSECTOR) == 0 {
            let sec = (*ss).sector;
            (*thing).sprev = ptr::null_mut();
            (*thing).snext = (*sec).thinglist as *mut _;

            if !(*sec).thinglist.is_null() {
                (*((*sec).thinglist as *mut mobj_t)).sprev = thing;
            }

            (*sec).thinglist = thing as *mut _;
        }

        if ((*thing).flags as u32 & MF_NOBLOCKMAP) == 0 {
            let shift = MAPBLOCKSHIFT as u32;
            let blockx = ((*thing).x.wrapping_sub(bmaporgx)) >> shift;
            let blocky = ((*thing).y.wrapping_sub(bmaporgy)) >> shift;

            if blockx >= 0 && blockx < bmapwidth && blocky >= 0 && blocky < bmapheight {
                let idx = (blocky * bmapwidth + blockx) as usize;
                let link = blocklinks.add(idx) as *mut *mut mobj_t;

                (*thing).bprev = ptr::null_mut();
                (*thing).bnext = *link;
                if !(*link).is_null() {
                    (*(*link)).bprev = thing;
                }
                *link = thing;
            } else {
                (*thing).bnext = ptr::null_mut();
                (*thing).bprev = ptr::null_mut();
            }
        }
    }
}

// --- P_BlockLinesIterator ---
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
            let ld = lines_base.add(*list as usize);
if (*ld).validcount == validcount {
list = list.add(1);
continue;
}
(*ld).validcount = validcount;
if let Some(f) = func {
if f(ld) == 0 {
return 0;
}
}
list = list.add(1);
}
1
}
}
// --- P_BlockThingsIterator ---
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
// --- PIT_AddLineIntercepts ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_AddLineIntercepts(ld: *mut line_t) -> c_int {
unsafe {
let frac_u = FRACUNIT as i32;
let limit = frac_u * 16;
let trace_ptr = &raw mut trace;
let (s1, s2) =
if trace.dx > limit || trace.dy > limit || trace.dx < -limit || trace.dy < -limit {
let v1 = (*ld).v1;
let v2 = (*ld).v2;
(
P_PointOnDivlineSide((*v1).x, (*v1).y, trace_ptr),
P_PointOnDivlineSide((*v2).x, (*v2).y, trace_ptr),
)
} else {
(
P_PointOnLineSide(trace.x, trace.y, ld),
P_PointOnLineSide(
trace.x.wrapping_add(trace.dx),
trace.y.wrapping_add(trace.dy),
ld,
),
)
};
if s1 == s2 {
return 1;
}
let mut dl = divline_t {
x: 0,
y: 0,
dx: 0,
dy: 0,
};
let dl_ptr = &raw mut dl;
P_MakeDivline(ld, dl_ptr);
let frac = P_InterceptVector(trace_ptr, dl_ptr);
if frac < 0 {
return 1;
}
if earlyout != 0 && frac < frac_u && (*ld).backsector.is_null() {
return 0;
}
(*intercept_p).frac = frac;
(*intercept_p).isaline = 1;
(*intercept_p).d.line = ld;
intercept_p = intercept_p.add(1);
1
}
}
// --- PIT_AddThingIntercepts ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_AddThingIntercepts(thing: *mut mobj_t) -> c_int {
unsafe {
let tracepositive = (trace.dx ^ trace.dy) > 0;
let tx = (*thing).x;
let ty = (*thing).y;
let rad = (*thing).radius;
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
let trace_ptr = &raw mut trace;
let s1 = P_PointOnDivlineSide(x1, y1, trace_ptr);
let s2 = P_PointOnDivlineSide(x2, y2, trace_ptr);
if s1 == s2 {
return 1;
}
let mut dl = divline_t {
x: x1,
y: y1,
dx: x2.wrapping_sub(x1),
dy: y2.wrapping_sub(y1),
};
let frac = P_InterceptVector(trace_ptr, &raw mut dl);
if frac < 0 {
return 1;
}
(*intercept_p).frac = frac;
(*intercept_p).isaline = 0;
(*intercept_p).d.thing = thing;
intercept_p = intercept_p.add(1);
1
}
}
// --- P_TraverseIntercepts ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TraverseIntercepts(func: traverser_t, maxfrac: i32) -> c_int {
unsafe {
let base = (&raw mut intercepts) as *mut intercept_t;
let mut count = intercept_p.offset_from(base);
let mut in_ptr: *mut intercept_t = ptr::null_mut();
while count > 0 {
count -= 1;
let mut dist = i32::MAX;
let mut scan = base;
while scan < intercept_p {
if (*scan).frac < dist {
dist = (*scan).frac;
in_ptr = scan;
}
scan = scan.add(1);
}
if dist > maxfrac {
return 1;
}
if let Some(f) = func {
if f(in_ptr) == 0 {
return 0;
}
}
(*in_ptr).frac = i32::MAX;
}
1
}
}
// --- P_PathTraverse ---
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