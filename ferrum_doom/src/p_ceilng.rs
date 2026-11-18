#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{c_int, c_void};
use std::mem::size_of;
use std::ptr;

use crate::p_floor::T_MovePlane;
use crate::p_setup::sectors;
use crate::p_spec::{P_FindHighestCeilingSurrounding, P_FindSectorFromLineTag};
use crate::p_tick::{P_AddThinker, P_RemoveThinker, leveltime};
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

const MAX_CEILINGS_COUNT: usize = MAXCEILINGS as usize;

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
}

// --- Глобальные переменные p_ceilng.c ---
#[unsafe(no_mangle)]
pub static mut activeceilings: [*mut ceiling_t; MAX_CEILINGS_COUNT] =
    [ptr::null_mut(); MAX_CEILINGS_COUNT];

// --- T_MoveCeiling ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_MoveCeiling(ceiling: *mut ceiling_t) {
    unsafe {
        let sec = (*ceiling).sector;
        let sound_org = (&raw mut (*sec).soundorg) as *mut c_void;
        let ceil_spd = CEILSPEED as i32;

        match (*ceiling).direction {
            0 => {
                // IN STASIS
            }

            1 => {
                // UP
                let res = T_MovePlane(
                    sec as *mut crate::p_floor::sector_t,
                    (*ceiling).speed,
                    (*ceiling).topheight,
                    0,
                    1,
                    (*ceiling).direction,
                );

                if (leveltime & 7) == 0 && (*ceiling).type_ != silentCrushAndRaise {
                    S_StartSound(sound_org, sfx_stnmov as c_int);
                }

                if res == pastdest {
                    match (*ceiling).type_ {
                        raiseToHighest => {
                            P_RemoveActiveCeiling(ceiling);
                        }
                        silentCrushAndRaise => {
                            S_StartSound(sound_org, sfx_pstop as c_int);
                            (*ceiling).direction = -1;
                        }
                        fastCrushAndRaise | crushAndRaise => {
                            (*ceiling).direction = -1;
                        }
                        _ => {}
                    }
                }
            }

            -1 => {
                // DOWN
                let res = T_MovePlane(
                    sec as *mut crate::p_floor::sector_t,
                    (*ceiling).speed,
                    (*ceiling).bottomheight,
                    (*ceiling).crush,
                    1,
                    (*ceiling).direction,
                );

                if (leveltime & 7) == 0 && (*ceiling).type_ != silentCrushAndRaise {
                    S_StartSound(sound_org, sfx_stnmov as c_int);
                }

                if res == pastdest {
                    match (*ceiling).type_ {
                        silentCrushAndRaise => {
                            S_StartSound(sound_org, sfx_pstop as c_int);
                            (*ceiling).speed = ceil_spd;
                            (*ceiling).direction = 1;
                        }
                        crushAndRaise => {
                            (*ceiling).speed = ceil_spd;
                            (*ceiling).direction = 1;
                        }
                        fastCrushAndRaise => {
                            (*ceiling).direction = 1;
                        }
                        lowerAndCrush | lowerToFloor => {
                            P_RemoveActiveCeiling(ceiling);
                        }
                        _ => {}
                    }
                } else if res == crushed {
                    match (*ceiling).type_ {
                        silentCrushAndRaise | crushAndRaise | lowerAndCrush => {
                            (*ceiling).speed = ceil_spd / 8;
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }
}

// --- EV_DoCeiling ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoCeiling(line: *mut line_t, ceil_type: ceiling_e) -> c_int {
    unsafe {
        let mut secnum = -1;
        let mut rtn = 0;

        match ceil_type {
            fastCrushAndRaise | silentCrushAndRaise | crushAndRaise => {
                P_ActivateInStasisCeiling(line);
            }
            _ => {}
        }

        let sectors_base = sectors as *mut sector_t;
        let frac_u = FRACUNIT as i32;
        let ceil_spd = CEILSPEED as i32;
        let move_fn: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_MoveCeiling as unsafe extern "C" fn(*mut ceiling_t));

        loop {
            secnum = P_FindSectorFromLineTag(line as *mut crate::p_spec::line_t, secnum);
            if secnum < 0 {
                break;
            }

            let sec = sectors_base.add(secnum as usize);
            if !(*sec).specialdata.is_null() {
                continue;
            }

            rtn = 1;
            let ceiling = Z_Malloc(size_of::<ceiling_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
                as *mut ceiling_t;
            P_AddThinker((&raw mut (*ceiling).thinker) as *mut crate::p_tick::thinker_t);

            (*sec).specialdata = ceiling as *mut c_void;
            (*ceiling).thinker.function.acp1 = Some(move_fn);
            (*ceiling).sector = sec;
            (*ceiling).crush = 0;

            match ceil_type {
                fastCrushAndRaise => {
                    (*ceiling).crush = 1;
                    (*ceiling).topheight = (*sec).ceilingheight;
                    (*ceiling).bottomheight = (*sec).floorheight + 8 * frac_u;
                    (*ceiling).direction = -1;
                    (*ceiling).speed = ceil_spd * 2;
                }
                silentCrushAndRaise | crushAndRaise => {
                    (*ceiling).crush = 1;
                    (*ceiling).topheight = (*sec).ceilingheight;
                    (*ceiling).bottomheight = (*sec).floorheight + 8 * frac_u;
                    (*ceiling).direction = -1;
                    (*ceiling).speed = ceil_spd;
                }
                lowerAndCrush | lowerToFloor => {
                    let mut bottom = (*sec).floorheight;
                    if ceil_type != lowerToFloor {
                        bottom += 8 * frac_u;
                    }
                    (*ceiling).bottomheight = bottom;
                    (*ceiling).direction = -1;
                    (*ceiling).speed = ceil_spd;
                }
                raiseToHighest => {
                    (*ceiling).topheight =
                        P_FindHighestCeilingSurrounding(sec as *mut crate::p_spec::sector_t);
                    (*ceiling).direction = 1;
                    (*ceiling).speed = ceil_spd;
                }
                _ => {}
            }

            (*ceiling).tag = (*sec).tag as c_int;
            (*ceiling).type_ = ceil_type;
            P_AddActiveCeiling(ceiling);
        }

        rtn
    }
}

// --- P_AddActiveCeiling ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AddActiveCeiling(c: *mut ceiling_t) {
    unsafe {
        let ceil_ptr = (&raw mut activeceilings) as *mut *mut ceiling_t;
        for i in 0..MAX_CEILINGS_COUNT {
            let slot = ceil_ptr.add(i);
            if (*slot).is_null() {
                *slot = c;
                return;
            }
        }
    }
}

// --- P_RemoveActiveCeiling ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RemoveActiveCeiling(c: *mut ceiling_t) {
    unsafe {
        let ceil_ptr = (&raw mut activeceilings) as *mut *mut ceiling_t;
        for i in 0..MAX_CEILINGS_COUNT {
            let slot = ceil_ptr.add(i);
            if *slot == c {
                let sec = (**slot).sector;
                (*sec).specialdata = ptr::null_mut();
                P_RemoveThinker((&raw mut (**slot).thinker) as *mut crate::p_tick::thinker_t);
                *slot = ptr::null_mut();
                break;
            }
        }
    }
}

// --- P_ActivateInStasisCeiling ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ActivateInStasisCeiling(line: *mut line_t) {
    unsafe {
        let ceil_ptr = (&raw mut activeceilings) as *mut *mut ceiling_t;
        let tag = (*line).tag as c_int;
        let move_fn: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_MoveCeiling as unsafe extern "C" fn(*mut ceiling_t));

        for i in 0..MAX_CEILINGS_COUNT {
            let c = *ceil_ptr.add(i);
            if !c.is_null() && (*c).tag == tag && (*c).direction == 0 {
                (*c).direction = (*c).olddirection;
                (*c).thinker.function.acp1 = Some(move_fn);
            }
        }
    }
}

// --- EV_CeilingCrushStop ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_CeilingCrushStop(line: *mut line_t) -> c_int {
    unsafe {
        let mut rtn = 0;
        let ceil_ptr = (&raw mut activeceilings) as *mut *mut ceiling_t;
        let tag = (*line).tag as c_int;

        for i in 0..MAX_CEILINGS_COUNT {
            let c = *ceil_ptr.add(i);
            if !c.is_null() && (*c).tag == tag && (*c).direction != 0 {
                (*c).olddirection = (*c).direction;
                (*c).thinker.function.acv = None;
                (*c).direction = 0;
                rtn = 1;
            }
        }

        rtn
    }
}
