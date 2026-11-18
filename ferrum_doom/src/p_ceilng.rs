#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;

use std::ffi::{c_int, c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::Mutex;

use crate::p_floor::t_move_plane;
use crate::p_setup::sectors;
use crate::p_spec::{P_FindHighestCeilingSurrounding, P_FindSectorFromLineTag};
use crate::p_tick::{P_AddThinker, P_RemoveThinker, get_leveltime};
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

const MAX_CEILINGS_COUNT: usize = MAXCEILINGS as usize;

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
}

static ACTIVE_CEILINGS: Mutex<[usize; MAX_CEILINGS_COUNT]> = Mutex::new([0; MAX_CEILINGS_COUNT]);

pub fn clear_active_ceilings() {
    if let Ok(mut list) = ACTIVE_CEILINGS.lock() {
        list.fill(0);
    }
}

pub fn has_active_ceiling(c: *mut ceiling_t) -> bool {
    let addr = c as usize;
    if addr == 0 {
        return false;
    }
    ACTIVE_CEILINGS
        .lock()
        .map(|list| list.contains(&addr))
        .unwrap_or(false)
}

pub fn t_move_ceiling(ceiling: &mut ceiling_t) {
    let Some(sec) = (unsafe { ceiling.sector.as_mut() }) else {
        return;
    };
    let sound_org = (&raw mut sec.soundorg) as *mut c_void;
    let ceil_spd = CEILSPEED as i32;
    let ceiling_ptr = ceiling as *mut ceiling_t;

    match ceiling.direction {
        0 => {}
        1 => {
            let res = t_move_plane(
                sec,
                ceiling.speed,
                ceiling.topheight,
                0,
                1,
                ceiling.direction,
            );

            if (get_leveltime() & 7) == 0 && ceiling.type_ != silentCrushAndRaise {
                unsafe {
                    S_StartSound(sound_org, sfx_stnmov as c_int);
                }
            }

            if res == pastdest {
                match ceiling.type_ {
                    raiseToHighest => unsafe {
                        P_RemoveActiveCeiling(ceiling_ptr);
                    },
                    silentCrushAndRaise => {
                        unsafe {
                            S_StartSound(sound_org, sfx_pstop as c_int);
                        }
                        ceiling.direction = -1;
                    }
                    fastCrushAndRaise | crushAndRaise => {
                        ceiling.direction = -1;
                    }
                    _ => {}
                }
            }
        }
        -1 => {
            let res = t_move_plane(
                sec,
                ceiling.speed,
                ceiling.bottomheight,
                ceiling.crush,
                1,
                ceiling.direction,
            );

            if (get_leveltime() & 7) == 0 && ceiling.type_ != silentCrushAndRaise {
                unsafe {
                    S_StartSound(sound_org, sfx_stnmov as c_int);
                }
            }

            if res == pastdest {
                match ceiling.type_ {
                    silentCrushAndRaise => {
                        unsafe {
                            S_StartSound(sound_org, sfx_pstop as c_int);
                        }
                        ceiling.speed = ceil_spd;
                        ceiling.direction = 1;
                    }
                    crushAndRaise => {
                        ceiling.speed = ceil_spd;
                        ceiling.direction = 1;
                    }
                    fastCrushAndRaise => {
                        ceiling.direction = 1;
                    }
                    lowerAndCrush | lowerToFloor => unsafe {
                        P_RemoveActiveCeiling(ceiling_ptr);
                    },
                    _ => {}
                }
            } else if res == crushed {
                match ceiling.type_ {
                    silentCrushAndRaise | crushAndRaise | lowerAndCrush => {
                        ceiling.speed = ceil_spd / 8;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_MoveCeiling(ceiling: *mut ceiling_t) {
    if let Some(c) = unsafe { ceiling.as_mut() } {
        t_move_ceiling(c);
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AddActiveCeiling(c: *mut ceiling_t) {
    let addr = c as usize;
    if let Ok(mut list) = ACTIVE_CEILINGS.lock() {
        for slot in list.iter_mut() {
            if *slot == 0 {
                *slot = addr;
                return;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RemoveActiveCeiling(c: *mut ceiling_t) {
    let addr = c as usize;
    let removed = if let Ok(mut list) = ACTIVE_CEILINGS.lock() {
        let mut found = false;
        for slot in list.iter_mut() {
            if *slot == addr {
                *slot = 0;
                found = true;
                break;
            }
        }
        found
    } else {
        false
    };

    if removed && !c.is_null() {
        unsafe {
            let sec = (*c).sector;
            if !sec.is_null() {
                (*sec).specialdata = ptr::null_mut();
            }
            P_RemoveThinker((&raw mut (*c).thinker) as *mut crate::p_tick::thinker_t);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ActivateInStasisCeiling(line: *mut line_t) {
    let tag = unsafe { (*line).tag as c_int };
    let move_fn: unsafe extern "C" fn(*mut c_void) =
        unsafe { std::mem::transmute(T_MoveCeiling as unsafe extern "C" fn(*mut ceiling_t)) };

    let addrs: Vec<usize> = ACTIVE_CEILINGS
        .lock()
        .map(|list| list.iter().copied().filter(|&a| a != 0).collect())
        .unwrap_or_default();

    for addr in addrs {
        unsafe {
            let c = addr as *mut ceiling_t;
            if (*c).tag == tag && (*c).direction == 0 {
                (*c).direction = (*c).olddirection;
                (*c).thinker.function.acp1 = Some(move_fn);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_CeilingCrushStop(line: *mut line_t) -> c_int {
    let mut rtn = 0;
    let tag = unsafe { (*line).tag as c_int };

    let addrs: Vec<usize> = ACTIVE_CEILINGS
        .lock()
        .map(|list| list.iter().copied().filter(|&a| a != 0).collect())
        .unwrap_or_default();

    for addr in addrs {
        unsafe {
            let c = addr as *mut ceiling_t;
            if (*c).tag == tag && (*c).direction != 0 {
                (*c).olddirection = (*c).direction;
                (*c).thinker.function.acv = None;
                (*c).direction = 0;
                rtn = 1;
            }
        }
    }

    rtn
}
