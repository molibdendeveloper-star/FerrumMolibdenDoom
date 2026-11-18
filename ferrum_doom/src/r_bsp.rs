#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int};
use std::ptr;
use std::sync::Mutex;

use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP};
use crate::r_sky::get_sky_flat_num;
use crate::tables::ANGLETOFINESHIFT;

const MAXSEGS: usize = 32;
const NUM_DRAWSEGS: usize = MAXDRAWSEGS as usize;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct subsector_t {
    pub sector: *mut sector_t,
    pub numlines: i16,
    pub firstline: i16,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct cliprange_t {
    pub first: c_int,
    pub last: c_int,
}

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn R_StoreWallRange(start: c_int, stop: c_int);
    fn R_PointToAngle(x: i32, y: i32) -> u32;
    fn R_PointOnSide(x: i32, y: i32, node: *mut node_t) -> c_int;
    fn R_FindPlane(height: i32, picnum: c_int, lightlevel: c_int) -> *mut visplane_t;
    fn R_AddSprites(sec: *mut sector_t);

    static mut viewx: i32;
    static mut viewy: i32;
    static mut viewz: i32;
    static mut viewangle: u32;
    static mut viewwidth: c_int;
    static mut clipangle: u32;
    static mut rw_angle1: u32;
    static mut viewangletox: [c_int; 4096];

    static mut sscount: c_int;
    static mut subsectors: *mut subsector_t;
    static mut numsubsectors: c_int;
    static mut segs: *mut seg_t;
    static mut nodes: *mut node_t;

    static mut floorplane: *mut visplane_t;
    static mut ceilingplane: *mut visplane_t;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("BSP error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

const EMPTY_DRAWSEG: drawseg_t = drawseg_t {
    curline: ptr::null_mut(),
    x1: 0,
    x2: 0,
    scale1: 0,
    scale2: 0,
    scalestep: 0,
    silhouette: 0,
    bsilheight: 0,
    tsilheight: 0,
    sprtopclip: ptr::null_mut(),
    sprbottomclip: ptr::null_mut(),
    maskedtexturecol: ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub static mut curline: *mut seg_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut sidedef: *mut side_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut linedef: *mut line_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut frontsector: *mut sector_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut backsector: *mut sector_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut drawsegs: [drawseg_t; NUM_DRAWSEGS] = [EMPTY_DRAWSEG; NUM_DRAWSEGS];

#[unsafe(no_mangle)]
pub static mut ds_p: *mut drawseg_t = ptr::null_mut();

struct ClipState {
    segs: [cliprange_t; MAXSEGS],
    len: usize,
}

impl ClipState {
    const fn new() -> Self {
        Self {
            segs: [cliprange_t { first: 0, last: 0 }; MAXSEGS],
            len: 0,
        }
    }

    fn clear(&mut self, width: c_int) {
        self.segs[0] = cliprange_t {
            first: -0x7fffffff,
            last: -1,
        };
        self.segs[1] = cliprange_t {
            first: width,
            last: 0x7fffffff,
        };
        self.len = 2;
    }

    fn clip_solid(&mut self, first: c_int, last: c_int, ranges: &mut Vec<(c_int, c_int)>) {
        let mut start = 0usize;
        while self.segs[start].last < first - 1 {
            start += 1;
        }

        if first < self.segs[start].first {
            if last < self.segs[start].first - 1 {
                ranges.push((first, last));
                for idx in (start + 1..=self.len).rev() {
                    self.segs[idx] = self.segs[idx - 1];
                }
                self.segs[start] = cliprange_t { first, last };
                self.len += 1;
                return;
            }

            ranges.push((first, self.segs[start].first - 1));
            self.segs[start].first = first;
        }

        if last <= self.segs[start].last {
            return;
        }

        let mut next = start;
        while last >= self.segs[next + 1].first - 1 {
            ranges.push((self.segs[next].last + 1, self.segs[next + 1].first - 1));
            next += 1;

            if last <= self.segs[next].last {
                self.segs[start].last = self.segs[next].last;
                if next != start {
                    let shift = next - start;
                    for i in (start + 1)..(self.len - shift) {
                        self.segs[i] = self.segs[i + shift];
                    }
                    self.len -= shift;
                }
                return;
            }
        }

        ranges.push((self.segs[next].last + 1, last));
        self.segs[start].last = last;

        if next != start {
            let shift = next - start;
            for i in (start + 1)..(self.len - shift) {
                self.segs[i] = self.segs[i + shift];
            }
            self.len -= shift;
        }
    }

    fn clip_pass(&self, first: c_int, last: c_int, ranges: &mut Vec<(c_int, c_int)>) {
        let mut start = 0usize;
        while self.segs[start].last < first - 1 {
            start += 1;
        }

        if first < self.segs[start].first {
            if last < self.segs[start].first - 1 {
                ranges.push((first, last));
                return;
            }
            ranges.push((first, self.segs[start].first - 1));
        }

        if last <= self.segs[start].last {
            return;
        }

        while last >= self.segs[start + 1].first - 1 {
            ranges.push((self.segs[start].last + 1, self.segs[start + 1].first - 1));
            start += 1;

            if last <= self.segs[start].last {
                return;
            }
        }

        ranges.push((self.segs[start].last + 1, last));
    }

    fn is_bbox_visible(&self, sx1: c_int, sx2: c_int) -> bool {
        let mut start = 0usize;
        while self.segs[start].last < sx2 {
            start += 1;
        }
        !(sx1 >= self.segs[start].first && sx2 <= self.segs[start].last)
    }
}

static CLIP_SEGS: Mutex<ClipState> = Mutex::new(ClipState::new());

#[unsafe(no_mangle)]
pub extern "C" fn R_ClearDrawSegs() {
    unsafe {
        ds_p = (&raw mut drawsegs) as *mut drawseg_t;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn R_ClipSolidWallSegment(first: c_int, last: c_int) {
    let mut ranges = Vec::with_capacity(8);
    CLIP_SEGS.lock().unwrap().clip_solid(first, last, &mut ranges);
    for (r_start, r_stop) in ranges {
        unsafe {
            R_StoreWallRange(r_start, r_stop);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn R_ClipPassWallSegment(first: c_int, last: c_int) {
    let mut ranges = Vec::with_capacity(8);
    CLIP_SEGS.lock().unwrap().clip_pass(first, last, &mut ranges);
    for (r_start, r_stop) in ranges {
        unsafe {
            R_StoreWallRange(r_start, r_stop);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn R_ClearClipSegs() {
    let w = unsafe { viewwidth };
    CLIP_SEGS.lock().unwrap().clear(w);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_AddLine(line: *mut seg_t) {
    let Some(seg) = (unsafe { line.as_ref() }) else { return };
    unsafe {
        curline = line;
    }

    let (Some(v1), Some(v2)) = (unsafe { seg.v1.as_ref() }, unsafe { seg.v2.as_ref() }) else {
        return;
    };

    let mut angle1 = unsafe { R_PointToAngle(v1.x, v1.y) };
    let mut angle2 = unsafe { R_PointToAngle(v2.x, v2.y) };

    let span = angle1.wrapping_sub(angle2);
    if span >= ANG180 {
        return;
    }

    let (v_ang, c_ang) = unsafe {
        rw_angle1 = angle1;
        (viewangle, clipangle)
    };

    angle1 = angle1.wrapping_sub(v_ang);
    angle2 = angle2.wrapping_sub(v_ang);

    let mut tspan = angle1.wrapping_add(c_ang);
    if tspan > 2 * c_ang {
        tspan = tspan.wrapping_sub(2 * c_ang);
        if tspan >= span {
            return;
        }
        angle1 = c_ang;
    }

    tspan = c_ang.wrapping_sub(angle2);
    if tspan > 2 * c_ang {
        tspan = tspan.wrapping_sub(2 * c_ang);
        if tspan >= span {
            return;
        }
        angle2 = 0u32.wrapping_sub(c_ang);
    }

    angle1 = angle1.wrapping_add(ANG90) >> ANGLETOFINESHIFT;
    angle2 = angle2.wrapping_add(ANG90) >> ANGLETOFINESHIFT;

    let (x1, x2) = unsafe {
        let vtox_ptr = (&raw const viewangletox) as *const c_int;
        (*vtox_ptr.add(angle1 as usize), *vtox_ptr.add(angle2 as usize))
    };

    if x1 == x2 {
        return;
    }

    unsafe {
        backsector = seg.backsector;
    }

    let Some(back) = (unsafe { seg.backsector.as_ref() }) else {
        R_ClipSolidWallSegment(x1, x2 - 1);
        return;
    };

    let Some(front) = (unsafe { frontsector.as_ref() }) else {
        return;
    };

    if back.ceilingheight <= front.floorheight || back.floorheight >= front.ceilingheight {
        R_ClipSolidWallSegment(x1, x2 - 1);
        return;
    }

    if back.ceilingheight != front.ceilingheight || back.floorheight != front.floorheight {
        R_ClipPassWallSegment(x1, x2 - 1);
        return;
    }

    let mid_tex = unsafe { seg.sidedef.as_ref().map(|s| s.midtexture).unwrap_or(0) };
    if back.ceilingpic == front.ceilingpic
        && back.floorpic == front.floorpic
        && back.lightlevel == front.lightlevel
        && mid_tex == 0
    {
        return;
    }

    R_ClipPassWallSegment(x1, x2 - 1);
}

const CHECKCOORD: [usize; 48] = [
    3, 0, 2, 1,
    3, 0, 2, 0,
    3, 1, 2, 0,
    0, 0, 0, 0,
    2, 0, 2, 1,
    0, 0, 0, 0,
    3, 1, 3, 0,
    0, 0, 0, 0,
    2, 0, 3, 1,
    2, 1, 3, 1,
    2, 1, 3, 0,
    0, 0, 0, 0,
];

pub fn r_check_bbox(bspcoord: &[i32; 4]) -> bool {
    let (vx, vy, v_ang, c_ang) = unsafe { (viewx, viewy, viewangle, clipangle) };

    let boxx = if vx <= bspcoord[BOXLEFT] {
        0
    } else if vx < bspcoord[BOXRIGHT] {
        1
    } else {
        2
};
let boxy = if vy >= bspcoord[BOXTOP] {
0
} else if vy > bspcoord[BOXBOTTOM] {
1
} else {
2
};
let boxpos = (boxy << 2) + boxx;
if boxpos == 5 {
return true;
}
let base_idx = boxpos * 4;
let x1 = bspcoord[CHECKCOORD[base_idx]];
let y1 = bspcoord[CHECKCOORD[base_idx + 1]];
let x2 = bspcoord[CHECKCOORD[base_idx + 2]];
let y2 = bspcoord[CHECKCOORD[base_idx + 3]];
let mut angle1 = unsafe { R_PointToAngle(x1, y1) }.wrapping_sub(v_ang);
let mut angle2 = unsafe { R_PointToAngle(x2, y2) }.wrapping_sub(v_ang);
let span = angle1.wrapping_sub(angle2);
if span >= ANG180 {
return true;
}
let mut tspan = angle1.wrapping_add(c_ang);
if tspan > 2 * c_ang {
tspan = tspan.wrapping_sub(2 * c_ang);
if tspan >= span {
return false;
}
angle1 = c_ang;
}
tspan = c_ang.wrapping_sub(angle2);
if tspan > 2 * c_ang {
tspan = tspan.wrapping_sub(2 * c_ang);
if tspan >= span {
return false;
}
angle2 = 0u32.wrapping_sub(c_ang);
}
angle1 = angle1.wrapping_add(ANG90) >> ANGLETOFINESHIFT;
angle2 = angle2.wrapping_add(ANG90) >> ANGLETOFINESHIFT;
let (sx1, mut sx2) = unsafe {
let vtox_ptr = (&raw const viewangletox) as *const c_int;
(*vtox_ptr.add(angle1 as usize), *vtox_ptr.add(angle2 as usize))
};
if sx1 == sx2 {
return false;
}
sx2 -= 1;
CLIP_SEGS.lock().unwrap().is_bbox_visible(sx1, sx2)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_CheckBBox(bspcoord: *mut i32) -> c_int {
let Some(coords) = (unsafe { (bspcoord as *const [i32; 4]).as_ref() }) else {
return 0;
};
r_check_bbox(coords) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_Subsector(num: c_int) {
unsafe {
if num < 0 || num >= numsubsectors {
let total_ss = numsubsectors;
doom_error(&format!("R_Subsector: ss {} with numss = {}", num, total_ss));
}
sscount += 1;
let sub = &*subsectors.add(num as usize);
frontsector = sub.sector;
let front = &*frontsector;
floorplane = if front.floorheight < viewz {
R_FindPlane(
front.floorheight,
front.floorpic as i32,
front.lightlevel as i32,
)
} else {
ptr::null_mut()
};
ceilingplane = if front.ceilingheight > viewz || (front.ceilingpic as i32) == get_sky_flat_num() {
R_FindPlane(
front.ceilingheight,
front.ceilingpic as i32,
front.lightlevel as i32,
)
} else {
ptr::null_mut()
};
R_AddSprites(frontsector);
let sub_segs = std::slice::from_raw_parts_mut(segs.add(sub.firstline as usize), sub.numlines as usize);
for seg in sub_segs {
R_AddLine(seg as *mut seg_t);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_RenderBSPNode(bspnum: c_int) {
unsafe {
if (bspnum & (NF_SUBSECTOR as c_int)) != 0 {
if bspnum == -1 {
R_Subsector(0);
} else {
R_Subsector(bspnum & !(NF_SUBSECTOR as c_int));
}
return;
}
let bsp = nodes.add(bspnum as usize);
let side = R_PointOnSide(viewx, viewy, bsp) as usize;
let children = (*bsp).children;
R_RenderBSPNode(children[side] as c_int);
if r_check_bbox(&(*bsp).bbox[side ^ 1]) {
R_RenderBSPNode(children[side ^ 1] as c_int);
}
}
}
