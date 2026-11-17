#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int};

use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::r_bsp::subsector_t;
use crate::r_main::validcount;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);

    static mut numsubsectors: c_int;
    static mut subsectors: *mut subsector_t;
    static mut segs: *mut seg_t;
    static mut numnodes: c_int;
    static mut nodes: *mut node_t;
    static mut numsectors: c_int;
    static mut sectors: *mut sector_t;
    static mut rejectmatrix: *mut u8;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Sight error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные p_sight.c ---
#[unsafe(no_mangle)]
pub static mut sightzstart: i32 = 0;

#[unsafe(no_mangle)]
pub static mut topslope: i32 = 0;

#[unsafe(no_mangle)]
pub static mut bottomslope: i32 = 0;

#[unsafe(no_mangle)]
pub static mut strace: divline_t = divline_t {
    x: 0,
    y: 0,
    dx: 0,
    dy: 0,
};

#[unsafe(no_mangle)]
pub static mut t2x: i32 = 0;

#[unsafe(no_mangle)]
pub static mut t2y: i32 = 0;

#[unsafe(no_mangle)]
pub static mut sightcounts: [c_int; 2] = [0; 2];

// --- P_DivlineSide ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DivlineSide(x: i32, y: i32, node: *mut divline_t) -> c_int {
    unsafe {
        let nx = (*node).x;
        let ny = (*node).y;
        let ndx = (*node).dx;
        let ndy = (*node).dy;

        if ndx == 0 {
            if x == nx {
                return 2;
            }
            if x <= nx {
                return (ndy > 0) as c_int;
            }
            return (ndy < 0) as c_int;
        }

        if ndy == 0 {
            if x == ny {
                return 2;
            }
            if y <= ny {
                return (ndx < 0) as c_int;
            }
            return (ndx > 0) as c_int;
        }

        let dx = x.wrapping_sub(nx);
        let dy = y.wrapping_sub(ny);

        let left = (ndy >> FRACBITS).wrapping_mul(dx >> FRACBITS);
        let right = (dy >> FRACBITS).wrapping_mul(ndx >> FRACBITS);

        if right < left {
            0
        } else if left == right {
            2
        } else {
            1
        }
    }
}

// --- P_InterceptVector2 ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InterceptVector2(v2: *mut divline_t, v1: *mut divline_t) -> i32 {
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

// --- P_CrossSubsector ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CrossSubsector(num: c_int) -> c_int {
    unsafe {
        if num >= numsubsectors || num < 0 {
            doom_error(&format!(
                "P_CrossSubsector: ss {} with numss = {}",
                num, numsubsectors
            ));
        }

        let sub = subsectors.add(num as usize);
        let count = (*sub).numlines as usize;
        let mut seg = segs.add((*sub).firstline as usize);

        let strace_ptr = &raw mut strace;
        let mut divl = divline_t {
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
        };
        let divl_ptr = &raw mut divl;

        for _ in 0..count {
            let line = (*seg).linedef;

            if (*line).validcount == validcount {
                seg = seg.add(1);
                continue;
            }
            (*line).validcount = validcount;

            let v1 = (*line).v1;
            let v2 = (*line).v2;
            let s1 = P_DivlineSide((*v1).x, (*v1).y, strace_ptr);
            let s2 = P_DivlineSide((*v2).x, (*v2).y, strace_ptr);

            if s1 == s2 {
                seg = seg.add(1);
                continue;
            }

            divl.x = (*v1).x;
            divl.y = (*v1).y;
            divl.dx = (*v2).x.wrapping_sub((*v1).x);
            divl.dy = (*v2).y.wrapping_sub((*v1).y);

            let s1_b = P_DivlineSide(strace.x, strace.y, divl_ptr);
            let s2_b = P_DivlineSide(t2x, t2y, divl_ptr);

            if s1_b == s2_b {
                seg = seg.add(1);
                continue;
            }

            if ((*line).flags as u32 & ML_TWOSIDED) == 0 {
                return 0;
            }

            let front = (*seg).frontsector;
            let back = (*seg).backsector;

            if (*front).floorheight == (*back).floorheight
                && (*front).ceilingheight == (*back).ceilingheight
            {
                seg = seg.add(1);
                continue;
            }

            let opentop = if (*front).ceilingheight < (*back).ceilingheight {
                (*front).ceilingheight
            } else {
                (*back).ceilingheight
            };

            let openbottom = if (*front).floorheight > (*back).floorheight {
                (*front).floorheight
            } else {
                (*back).floorheight
            };

            if openbottom >= opentop {
                return 0;
            }

            let frac = P_InterceptVector2(strace_ptr, divl_ptr);

            if (*front).floorheight != (*back).floorheight {
                let slope = fixed_div(openbottom.wrapping_sub(sightzstart), frac);
                if slope > bottomslope {
                    bottomslope = slope;
                }
            }

            if (*front).ceilingheight != (*back).ceilingheight {
                let slope = fixed_div(opentop.wrapping_sub(sightzstart), frac);
                if slope < topslope {
                    topslope = slope;
                }
            }

            if topslope <= bottomslope {
                return 0;
            }

            seg = seg.add(1);
        }

        1
    }
}

// --- P_CrossBSPNode ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CrossBSPNode(bspnum: c_int) -> c_int {
    unsafe {
        let nf_sub = NF_SUBSECTOR as c_int;
        if (bspnum & nf_sub) != 0 {
            if bspnum == -1 {
                return P_CrossSubsector(0);
            } else {
                return P_CrossSubsector(bspnum & !nf_sub);
            }
        }

        let bsp = nodes.add(bspnum as usize);
        let div_bsp = bsp as *mut divline_t;

        let mut side = P_DivlineSide(strace.x, strace.y, div_bsp);
        if side == 2 {
            side = 0;
        }

        let children_ptr = (&raw const (*bsp).children) as *const u16;
        let child_start = *children_ptr.add(side as usize) as c_int;
        if P_CrossBSPNode(child_start) == 0 {
            return 0;
        }

        if side == P_DivlineSide(t2x, t2y, div_bsp) {
            return 1;
        }

        let child_end = *children_ptr.add((side ^ 1) as usize) as c_int;
        P_CrossBSPNode(child_end)
    }
}

// --- P_CheckSight ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckSight(t1: *mut mobj_t, t2: *mut mobj_t) -> c_int {
    unsafe {
        let sub1 = (*t1).subsector as *mut subsector_t;
        let sub2 = (*t2).subsector as *mut subsector_t;

        let sec1 = (*sub1).sector as *mut sector_t;
        let sec2 = (*sub2).sector as *mut sector_t;

        let s1 = sec1.offset_from(sectors) as c_int;
        let s2 = sec2.offset_from(sectors) as c_int;

        let pnum = s1 * numsectors + s2;
        let bytenum = (pnum >> 3) as usize;
        let bitnum = 1u8 << (pnum & 7);

        let counts_ptr = (&raw mut sightcounts) as *mut c_int;
        if (*rejectmatrix.add(bytenum) & bitnum) != 0 {
            *counts_ptr += 1;
            return 0;
        }

        *counts_ptr.add(1) += 1;
        validcount += 1;

        sightzstart = (*t1).z + (*t1).height - ((*t1).height >> 2);
        topslope = ((*t2).z + (*t2).height) - sightzstart;
        bottomslope = (*t2).z - sightzstart;

        strace.x = (*t1).x;
        strace.y = (*t1).y;
        t2x = (*t2).x;
        t2y = (*t2).y;
        strace.dx = (*t2).x.wrapping_sub((*t1).x);
        strace.dy = (*t2).y.wrapping_sub((*t1).y);

        P_CrossBSPNode(numnodes - 1)
    }
}
