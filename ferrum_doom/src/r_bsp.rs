#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int};
use std::ptr;

use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP};
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
    static mut skyflatnum: c_int;

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

// Глобальные переменные состояния обхода BSP
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
pub static mut drawsegs: [drawseg_t; NUM_DRAWSEGS] = [unsafe { std::mem::zeroed() }; NUM_DRAWSEGS];

#[unsafe(no_mangle)]
pub static mut ds_p: *mut drawseg_t = ptr::null_mut();

static mut newend: *mut cliprange_t = ptr::null_mut();
static mut solidsegs: [cliprange_t; MAXSEGS] = [cliprange_t { first: 0, last: 0 }; MAXSEGS];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ClearDrawSegs() {
    unsafe {
        ds_p = (&raw mut drawsegs) as *mut drawseg_t;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ClipSolidWallSegment(first: c_int, last: c_int) {
    unsafe {
        let mut start = (&raw mut solidsegs) as *mut cliprange_t;
        while (*start).last < first - 1 {
            start = start.add(1);
        }

        if first < (*start).first {
            if last < (*start).first - 1 {
                R_StoreWallRange(first, last);
                let mut next = newend;
                newend = newend.add(1);

                while next != start {
                    *next = *next.sub(1);
                    next = next.sub(1);
                }
                (*next).first = first;
                (*next).last = last;
                return;
            }

            R_StoreWallRange(first, (*start).first - 1);
            (*start).first = first;
        }

        if last <= (*start).last {
            return;
        }

        let mut next = start;
        while last >= (*next.add(1)).first - 1 {
            R_StoreWallRange((*next).last + 1, (*next.add(1)).first - 1);
            next = next.add(1);

            if last <= (*next).last {
                (*start).last = (*next).last;
                if next != start {
                    while next != newend {
                        next = next.add(1);
                        start = start.add(1);
                        *start = *next;
                    }
                    newend = start.add(1);
                }
                return;
            }
        }

        R_StoreWallRange((*next).last + 1, last);
        (*start).last = last;

        if next != start {
            while next != newend {
                next = next.add(1);
                start = start.add(1);
                *start = *next;
            }
            newend = start.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ClipPassWallSegment(first: c_int, last: c_int) {
    unsafe {
        let mut start = (&raw mut solidsegs) as *mut cliprange_t;
        while (*start).last < first - 1 {
            start = start.add(1);
        }

        if first < (*start).first {
            if last < (*start).first - 1 {
                R_StoreWallRange(first, last);
                return;
            }
            R_StoreWallRange(first, (*start).first - 1);
        }

        if last <= (*start).last {
            return;
        }

        while last >= (*start.add(1)).first - 1 {
            R_StoreWallRange((*start).last + 1, (*start.add(1)).first - 1);
            start = start.add(1);

            if last <= (*start).last {
                return;
            }
        }

        R_StoreWallRange((*start).last + 1, last);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ClearClipSegs() {
    unsafe {
        let segs_ptr = (&raw mut solidsegs) as *mut cliprange_t;
        (*segs_ptr).first = -0x7fffffff;
        (*segs_ptr).last = -1;
        (*segs_ptr.add(1)).first = viewwidth;
        (*segs_ptr.add(1)).last = 0x7fffffff;
        newend = segs_ptr.add(2);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_AddLine(line: *mut seg_t) {
    unsafe {
        curline = line;

        let v1 = (*line).v1;
        let v2 = (*line).v2;
        let mut angle1 = R_PointToAngle((*v1).x, (*v1).y);
        let mut angle2 = R_PointToAngle((*v2).x, (*v2).y);

        let span = angle1.wrapping_sub(angle2);

        // Отсечение нелицевых граней (Backface culling)
        if span >= ANG180 {
            return;
        }

        rw_angle1 = angle1;
        angle1 = angle1.wrapping_sub(viewangle);
        angle2 = angle2.wrapping_sub(viewangle);

        let mut tspan = angle1.wrapping_add(clipangle);
        if tspan > 2 * clipangle {
            tspan = tspan.wrapping_sub(2 * clipangle);
            if tspan >= span {
                return;
            }
            angle1 = clipangle;
        }

        tspan = clipangle.wrapping_sub(angle2);
        if tspan > 2 * clipangle {
            tspan = tspan.wrapping_sub(2 * clipangle);
            if tspan >= span {
                return;
            }
            angle2 = 0u32.wrapping_sub(clipangle);
        }

        angle1 = angle1.wrapping_add(ANG90) >> ANGLETOFINESHIFT;
        angle2 = angle2.wrapping_add(ANG90) >> ANGLETOFINESHIFT;

        let vtox_ptr = (&raw const viewangletox) as *const c_int;
        let x1 = *vtox_ptr.add(angle1 as usize);
        let x2 = *vtox_ptr.add(angle2 as usize);

        if x1 == x2 {
            return;
        }

        backsector = (*line).backsector;

        // Односторонняя стена
        if backsector.is_null() {
            R_ClipSolidWallSegment(x1, x2 - 1);
            return;
        }

        // Закрытая дверь
        if (*backsector).ceilingheight <= (*frontsector).floorheight
            || (*backsector).floorheight >= (*frontsector).ceilingheight
        {
            R_ClipSolidWallSegment(x1, x2 - 1);
            return;
        }

        // Окно или перепад высот
        if (*backsector).ceilingheight != (*frontsector).ceilingheight
            || (*backsector).floorheight != (*frontsector).floorheight
        {
            R_ClipPassWallSegment(x1, x2 - 1);
            return;
        }

        // Пустая линия-триггер без текстур
        if (*backsector).ceilingpic == (*frontsector).ceilingpic
            && (*backsector).floorpic == (*frontsector).floorpic
            && (*backsector).lightlevel == (*frontsector).lightlevel
            && (*(*curline).sidedef).midtexture == 0
        {
            return;
        }

        R_ClipPassWallSegment(x1, x2 - 1);
    }
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_CheckBBox(bspcoord: *mut i32) -> c_int {
    unsafe {
        let boxx = if viewx <= *bspcoord.add(BOXLEFT) {
            0
        } else if viewx < *bspcoord.add(BOXRIGHT) {
            1
        } else {
            2
        };

        let boxy = if viewy >= *bspcoord.add(BOXTOP) {
            0
        } else if viewy > *bspcoord.add(BOXBOTTOM) {
            1
        } else {
            2
        };

        let boxpos = (boxy << 2) + boxx;
        if boxpos == 5 {
            return 1;
        }

        let base_idx = boxpos * 4;
        let x1 = *bspcoord.add(CHECKCOORD[base_idx]);
        let y1 = *bspcoord.add(CHECKCOORD[base_idx + 1]);
        let x2 = *bspcoord.add(CHECKCOORD[base_idx + 2]);
        let y2 = *bspcoord.add(CHECKCOORD[base_idx + 3]);

        let mut angle1 = R_PointToAngle(x1, y1).wrapping_sub(viewangle);
        let mut angle2 = R_PointToAngle(x2, y2).wrapping_sub(viewangle);

        let span = angle1.wrapping_sub(angle2);
        if span >= ANG180 {
            return 1;
        }

        let mut tspan = angle1.wrapping_add(clipangle);
        if tspan > 2 * clipangle {
            tspan = tspan.wrapping_sub(2 * clipangle);
            if tspan >= span {
                return 0;
            }
            angle1 = clipangle;
        }

        tspan = clipangle.wrapping_sub(angle2);
        if tspan > 2 * clipangle {
            tspan = tspan.wrapping_sub(2 * clipangle);
            if tspan >= span {
                return 0;
            }
            angle2 = 0u32.wrapping_sub(clipangle);
        }
angle1 = angle1.wrapping_add(ANG90) >> ANGLETOFINESHIFT;
angle2 = angle2.wrapping_add(ANG90) >> ANGLETOFINESHIFT;
let vtox_ptr = (&raw const viewangletox) as *const c_int;
let sx1 = *vtox_ptr.add(angle1 as usize);
let mut sx2 = *vtox_ptr.add(angle2 as usize);
if sx1 == sx2 {
return 0;
}
sx2 -= 1;
let mut start = (&raw const solidsegs) as *const cliprange_t;
while (*start).last < sx2 {
start = start.add(1);
}
if sx1 >= (*start).first && sx2 <= (*start).last {
return 0;
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_Subsector(num: c_int) {
unsafe {
if num < 0 || num >= numsubsectors {
let total_ss = numsubsectors; doom_error(&format!("R_Subsector: ss {} with numss = {}", num, total_ss));
}
sscount += 1;
let sub = subsectors.add(num as usize);
frontsector = (*sub).sector;
let mut count = (*sub).numlines;
let mut line = segs.add((*sub).firstline as usize);
if (*frontsector).floorheight < viewz {
floorplane = R_FindPlane(
(*frontsector).floorheight,
(*frontsector).floorpic as i32,
(*frontsector).lightlevel as i32,
);
} else {
floorplane = ptr::null_mut();
}
if (*frontsector).ceilingheight > viewz || ((*frontsector).ceilingpic as i32) == skyflatnum {
ceilingplane = R_FindPlane(
(*frontsector).ceilingheight,
(*frontsector).ceilingpic as i32,
(*frontsector).lightlevel as i32,
);
} else {
ceilingplane = ptr::null_mut();
}
R_AddSprites(frontsector);
while count > 0 {
R_AddLine(line);
line = line.add(1);
count -= 1;
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
// Рекурсивно рендерим ближнее подпространство
let children_ptr = (&raw const (*bsp).children) as *const u16;
R_RenderBSPNode(*children_ptr.add(side) as c_int);
// Проверяем, видно ли дальнее подпространство через ограничивающий прямоугольник
let bbox_ptr = (&raw mut (*bsp).bbox) as *mut [i32; 4];
if R_CheckBBox((*bbox_ptr.add(side ^ 1)).as_mut_ptr()) != 0 {
R_RenderBSPNode(*children_ptr.add(side ^ 1) as c_int);
}
}
}