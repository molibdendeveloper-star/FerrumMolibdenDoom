#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;

use std::ffi::c_int;

use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::r_bsp::subsector_t;
use crate::r_main::validcount;
use crate::v_video::doom_error;

unsafe extern "C" {
    static mut numsubsectors: c_int;
    static mut subsectors: *mut subsector_t;
    static mut segs: *mut seg_t;
    static mut numnodes: c_int;
    static mut nodes: *mut node_t;
    static mut numsectors: c_int;
    static mut sectors: *mut sector_t;
    static mut rejectmatrix: *mut u8;
}

struct SightTrace {
    zstart: i32,
    topslope: i32,
    bottomslope: i32,
    strace: divline_t,
    t2x: i32,
    t2y: i32,
}

#[inline]
pub fn divline_side(x: i32, y: i32, node: &divline_t) -> c_int {
    if node.dx == 0 {
        if x == node.x {
            return 2;
        }
        if x <= node.x {
            return (node.dy > 0) as c_int;
        }
        return (node.dy < 0) as c_int;
    }

    if node.dy == 0 {
        if x == node.y {
            return 2;
        }
        if y <= node.y {
            return (node.dx < 0) as c_int;
        }
        return (node.dx > 0) as c_int;
    }

    let dx = x.wrapping_sub(node.x);
    let dy = y.wrapping_sub(node.y);

    let left = (node.dy >> FRACBITS).wrapping_mul(dx >> FRACBITS);
    let right = (dy >> FRACBITS).wrapping_mul(node.dx >> FRACBITS);

    if right < left {
        0
    } else if left == right {
        2
    } else {
        1
    }
}

#[inline]
pub fn intercept_vector2(v2: &divline_t, v1: &divline_t) -> i32 {
    let den =
        fixed_mul(v1.dy >> 8, v2.dx).wrapping_sub(fixed_mul(v1.dx >> 8, v2.dy));

    if den == 0 {
        return 0;
    }

    let num = fixed_mul(v1.x.wrapping_sub(v2.x) >> 8, v1.dy)
        .wrapping_add(fixed_mul(v2.y.wrapping_sub(v1.y) >> 8, v1.dx));

    fixed_div(num, den)
}

fn cross_subsector(trace: &mut SightTrace, num: c_int, cur_validcount: c_int) -> bool {
    let sub_slice = unsafe {
        if num >= numsubsectors || num < 0 {
            doom_error(&format!(
                "P_CrossSubsector: ss {} with numss = {}",
                num, numsubsectors
            ));
        }
        std::slice::from_raw_parts(subsectors, numsubsectors as usize)
    };

    let sub = &sub_slice[num as usize];
    let first = sub.firstline as usize;
    let count = sub.numlines as usize;

    let seg_slice = unsafe { std::slice::from_raw_parts(segs.add(first), count) };

    for seg in seg_slice {
        let Some(line) = (unsafe { seg.linedef.as_mut() }) else {
            continue;
        };

        if line.validcount == cur_validcount {
            continue;
        }
        line.validcount = cur_validcount;

        let (Some(v1), Some(v2)) = (unsafe { line.v1.as_ref() }, unsafe { line.v2.as_ref() })
        else {
            continue;
        };

        let s1 = divline_side(v1.x, v1.y, &trace.strace);
        let s2 = divline_side(v2.x, v2.y, &trace.strace);
        if s1 == s2 {
            continue;
        }

        let divl = divline_t {
            x: v1.x,
            y: v1.y,
            dx: v2.x.wrapping_sub(v1.x),
            dy: v2.y.wrapping_sub(v1.y),
        };

        let s1_b = divline_side(trace.strace.x, trace.strace.y, &divl);
        let s2_b = divline_side(trace.t2x, trace.t2y, &divl);
        if s1_b == s2_b {
            continue;
        }

        if (line.flags as u32 & ML_TWOSIDED) == 0 {
            return false;
        }

        let (Some(front), Some(back)) =
            (unsafe { seg.frontsector.as_ref() }, unsafe { seg.backsector.as_ref() })
        else {
            continue;
        };

        if front.floorheight == back.floorheight && front.ceilingheight == back.ceilingheight {
            continue;
        }

        let opentop = front.ceilingheight.min(back.ceilingheight);
        let openbottom = front.floorheight.max(back.floorheight);

        if openbottom >= opentop {
            return false;
        }

        let frac = intercept_vector2(&trace.strace, &divl);

        if front.floorheight != back.floorheight {
            let slope = fixed_div(openbottom.wrapping_sub(trace.zstart), frac);
            if slope > trace.bottomslope {
                trace.bottomslope = slope;
            }
        }

        if front.ceilingheight != back.ceilingheight {
            let slope = fixed_div(opentop.wrapping_sub(trace.zstart), frac);
            if slope < trace.topslope {
                trace.topslope = slope;
            }
        }

        if trace.topslope <= trace.bottomslope {
            return false;
        }
    }

    true
}

fn cross_bsp_node(trace: &mut SightTrace, bspnum: c_int, cur_validcount: c_int) -> bool {
    let nf_sub = NF_SUBSECTOR as c_int;
    if (bspnum & nf_sub) != 0 {
        let ss_num = if bspnum == -1 { 0 } else { bspnum & !nf_sub };
        return cross_subsector(trace, ss_num, cur_validcount);
    }

    let bsp = unsafe { &*nodes.add(bspnum as usize) };
    let div_bsp = divline_t {
        x: bsp.x,
        y: bsp.y,
        dx: bsp.dx,
        dy: bsp.dy,
    };

    let mut side = divline_side(trace.strace.x, trace.strace.y, &div_bsp);
    if side == 2 {
        side = 0;
    }

    let side_idx = side as usize;
    if !cross_bsp_node(trace, bsp.children[side_idx] as c_int, cur_validcount) {
        return false;
    }

    if side == divline_side(trace.t2x, trace.t2y, &div_bsp) {
        return true;
    }

    cross_bsp_node(trace, bsp.children[side_idx ^ 1] as c_int, cur_validcount)
}

pub fn p_check_sight(t1: &mobj_t, t2: &mobj_t) -> bool {
    let (s1, s2, n_sec, n_nodes, cur_vc) = unsafe {
        let sub1 = &*(t1.subsector as *const subsector_t);
        let sub2 = &*(t2.subsector as *const subsector_t);
        let sec1 = sub1.sector as *const sector_t;
        let sec2 = sub2.sector as *const sector_t;

        let s1 = sec1.offset_from(sectors) as c_int;
        let s2 = sec2.offset_from(sectors) as c_int;
        let pnum = s1 * numsectors + s2;
        let bytenum = (pnum >> 3) as usize;
        let bitnum = 1u8 << (pnum & 7);

        let reject = std::slice::from_raw_parts(rejectmatrix, bytenum + 1);
        if (reject[bytenum] & bitnum) != 0 {
            return false;
        }

        validcount += 1;
        (s1, s2, numsectors, numnodes, validcount)
    };
    let _ = (s1, s2, n_sec);

    let zstart = t1.z + t1.height - (t1.height >> 2);
    let mut trace = SightTrace {
        zstart,
        topslope: (t2.z + t2.height) - zstart,
        bottomslope: t2.z - zstart,
        strace: divline_t {
            x: t1.x,
            y: t1.y,
            dx: t2.x.wrapping_sub(t1.x),
            dy: t2.y.wrapping_sub(t1.y),
        },
        t2x: t2.x,
        t2y: t2.y,
    };

    cross_bsp_node(&mut trace, n_nodes - 1, cur_vc)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DivlineSide(x: i32, y: i32, node: *mut divline_t) -> c_int {
    match unsafe { node.as_ref() } {
        Some(n) => divline_side(x, y, n),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InterceptVector2(v2: *mut divline_t, v1: *mut divline_t) -> i32 {
    match (unsafe { v2.as_ref() }, unsafe { v1.as_ref() }) {
        (Some(a), Some(b)) => intercept_vector2(a, b),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckSight(t1: *mut mobj_t, t2: *mut mobj_t) -> c_int {
    match (unsafe { t1.as_ref() }, unsafe { t2.as_ref() }) {
        (Some(a), Some(b)) => p_check_sight(a, b) as c_int,
        _ => 0,
    }
}
