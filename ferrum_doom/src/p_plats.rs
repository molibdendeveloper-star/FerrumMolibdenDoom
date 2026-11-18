#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;

use std::ffi::{CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::Mutex;

use crate::m_random::p_random;
use crate::p_floor::t_move_plane;
use crate::p_spec::{
    P_FindHighestFloorSurrounding, P_FindLowestFloorSurrounding, P_FindNextHighestFloor,
    P_FindSectorFromLineTag,
};
use crate::p_tick::{P_AddThinker, P_RemoveThinker, get_leveltime};
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

const MAX_PLATS_COUNT: usize = MAXPLATS as usize;

const PLAT_UP: u32 = 0;
const PLAT_DOWN: u32 = 1;
const PLAT_WAITING: u32 = 2;
const PLAT_IN_STASIS: u32 = 3;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);

    static mut sectors: *mut sector_t;
    static mut sides: *mut side_t;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Plat error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

static ACTIVE_PLATS: Mutex<[usize; MAX_PLATS_COUNT]> = Mutex::new([0; MAX_PLATS_COUNT]);

pub fn clear_active_plats() {
    if let Ok(mut list) = ACTIVE_PLATS.lock() {
        list.fill(0);
    }
}

pub fn has_active_plat(p: *mut plat_t) -> bool {
    let addr = p as usize;
    if addr == 0 {
        return false;
    }
    ACTIVE_PLATS
        .lock()
        .map(|list| list.contains(&addr))
        .unwrap_or(false)
}

pub fn t_plat_raise(plat: &mut plat_t) {
    let Some(sec) = (unsafe { plat.sector.as_mut() }) else {
        return;
    };
    let sound_org = (&raw mut sec.soundorg) as *mut c_void;
    let plat_ptr = plat as *mut plat_t;

    match plat.status as u32 {
        PLAT_UP => {
            let res = t_move_plane(sec, plat.speed, plat.high, plat.crush, 0, 1);

            if (plat.type_ == raiseAndChange || plat.type_ == raiseToNearestAndChange)
                && (get_leveltime() & 7) == 0
            {
                unsafe {
                    S_StartSound(sound_org, sfx_stnmov as c_int);
                }
            }

            if res == crushed && plat.crush == 0 {
                plat.count = plat.wait;
                plat.status = PLAT_DOWN as _;
                unsafe {
                    S_StartSound(sound_org, sfx_pstart as c_int);
                }
            } else if res == pastdest {
                plat.count = plat.wait;
                plat.status = PLAT_WAITING as _;
                unsafe {
                    S_StartSound(sound_org, sfx_pstop as c_int);
                }

                match plat.type_ {
                    blazeDWUS | downWaitUpStay | raiseAndChange | raiseToNearestAndChange => unsafe {
                        P_RemoveActivePlat(plat_ptr);
                    },
                    _ => {}
                }
            }
        }
        PLAT_DOWN => {
            let res = t_move_plane(sec, plat.speed, plat.low, 0, 0, -1);

            if res == pastdest {
                plat.count = plat.wait;
                plat.status = PLAT_WAITING as _;
                unsafe {
                    S_StartSound(sound_org, sfx_pstop as c_int);
                }
            }
        }
        PLAT_WAITING => {
            plat.count -= 1;
            if plat.count == 0 {
                plat.status = if sec.floorheight == plat.low {
                    PLAT_UP as _
                } else {
                    PLAT_DOWN as _
                };
                unsafe {
                    S_StartSound(sound_org, sfx_pstart as c_int);
                }
            }
        }
        PLAT_IN_STASIS | _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_PlatRaise(plat: *mut plat_t) {
    if let Some(p) = unsafe { plat.as_mut() } {
        t_plat_raise(p);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoPlat(
    line: *mut line_t,
    plat_type: plattype_e,
    amount: c_int,
) -> c_int {
    unsafe {
        let mut secnum = -1;
        let mut rtn = 0;

        if plat_type == perpetualRaise {
            P_ActivateInStasis((*line).tag as c_int);
        }

        let frac_u = FRACUNIT as i32;
        let plat_spd = PLATSPEED as i32;
        let plat_wt = PLATWAIT as c_int;

        loop {
            secnum = P_FindSectorFromLineTag(line as *mut crate::p_spec::line_t, secnum);
            if secnum < 0 {
                break;
            }

            let sec = sectors.add(secnum as usize);
            if !(*sec).specialdata.is_null() {
                continue;
            }

            rtn = 1;
            let plat = Z_Malloc(size_of::<plat_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
                as *mut plat_t;
            P_AddThinker((&raw mut (*plat).thinker) as *mut crate::p_tick::thinker_t);

            (*plat).type_ = plat_type;
            (*plat).sector = sec;
            (*sec).specialdata = plat as *mut c_void;

            let raise_fn: unsafe extern "C" fn(*mut c_void) =
                std::mem::transmute(T_PlatRaise as unsafe extern "C" fn(*mut plat_t));
            (*plat).thinker.function.acp1 = Some(raise_fn);

            (*plat).crush = 0;
            (*plat).tag = (*line).tag as c_int;

            let sound_org = (&raw mut (*sec).soundorg) as *mut c_void;
            let spec_sec = sec as *mut crate::p_spec::sector_t;

            match plat_type {
                raiseToNearestAndChange => {
                    (*plat).speed = plat_spd / 2;
                    let s0 = *(&raw const (*line).sidenum as *const i16) as usize;
                    (*sec).floorpic = (*(*sides.add(s0)).sector).floorpic;
                    (*plat).high = P_FindNextHighestFloor(spec_sec, (*sec).floorheight);
                    (*plat).wait = 0;
                    (*plat).status = PLAT_UP as _;
                    (*sec).special = 0;
                    S_StartSound(sound_org, sfx_stnmov as c_int);
                }
                raiseAndChange => {
                    (*plat).speed = plat_spd / 2;
                    let s0 = *(&raw const (*line).sidenum as *const i16) as usize;
                    (*sec).floorpic = (*(*sides.add(s0)).sector).floorpic;
                    (*plat).high = (*sec).floorheight + amount * frac_u;
                    (*plat).wait = 0;
                    (*plat).status = PLAT_UP as _;
                    S_StartSound(sound_org, sfx_stnmov as c_int);
                }
                downWaitUpStay => {
                    (*plat).speed = plat_spd * 4;
                    let mut low = P_FindLowestFloorSurrounding(spec_sec);
                    if low > (*sec).floorheight {
                        low = (*sec).floorheight;
                    }
                    (*plat).low = low;
                    (*plat).high = (*sec).floorheight;
                    (*plat).wait = 35 * plat_wt;
                    (*plat).status = PLAT_DOWN as _;
                    S_StartSound(sound_org, sfx_pstart as c_int);
                }
                blazeDWUS => {
                    (*plat).speed = plat_spd * 8;
                    let mut low = P_FindLowestFloorSurrounding(spec_sec);
                    if low > (*sec).floorheight {
                        low = (*sec).floorheight;
                    }
                    (*plat).low = low;
                    (*plat).high = (*sec).floorheight;
                    (*plat).wait = 35 * plat_wt;
                    (*plat).status = PLAT_DOWN as _;
                    S_StartSound(sound_org, sfx_pstart as c_int);
                }
                perpetualRaise => {
                    (*plat).speed = plat_spd;
                    let mut low = P_FindLowestFloorSurrounding(spec_sec);
                    if low > (*sec).floorheight {
                        low = (*sec).floorheight;
                    }
                    (*plat).low = low;

                    let mut high = P_FindHighestFloorSurrounding(spec_sec);
                    if high < (*sec).floorheight {
                        high = (*sec).floorheight;
                    }
                    (*plat).high = high;

                    (*plat).wait = 35 * plat_wt;
                    (*plat).status = (p_random() & 1) as _;
                    S_StartSound(sound_org, sfx_pstart as c_int);
                }
                _ => {}
            }

            P_AddActivePlat(plat);
        }

        rtn
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ActivateInStasis(tag: c_int) {
    let raise_fn: unsafe extern "C" fn(*mut c_void) =
        unsafe { std::mem::transmute(T_PlatRaise as unsafe extern "C" fn(*mut plat_t)) };

    let addrs: Vec<usize> = ACTIVE_PLATS
        .lock()
        .map(|list| list.iter().copied().filter(|&a| a != 0).collect())
        .unwrap_or_default();

    for addr in addrs {
        unsafe {
            let p = addr as *mut plat_t;
            if (*p).tag == tag && ((*p).status as u32) == PLAT_IN_STASIS {
                (*p).status = (*p).oldstatus;
                (*p).thinker.function.acp1 = Some(raise_fn);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_StopPlat(line: *mut line_t) {
    let tag = unsafe { (*line).tag as c_int };

    let addrs: Vec<usize> = ACTIVE_PLATS
        .lock()
        .map(|list| list.iter().copied().filter(|&a| a != 0).collect())
        .unwrap_or_default();

    for addr in addrs {
        unsafe {
            let p = addr as *mut plat_t;
            if ((*p).status as u32) != PLAT_IN_STASIS && (*p).tag == tag {
                (*p).oldstatus = (*p).status;
                (*p).status = PLAT_IN_STASIS as _;
                (*p).thinker.function.acv = None;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AddActivePlat(plat: *mut plat_t) {
    let addr = plat as usize;
if let Ok(mut list) = ACTIVE_PLATS.lock() {
for slot in list.iter_mut() {
if *slot == 0 {
*slot = addr;
return;
}
}
}
doom_error("P_AddActivePlat: no more plats!");
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RemoveActivePlat(plat: *mut plat_t) {
let addr = plat as usize;
let removed = if let Ok(mut list) = ACTIVE_PLATS.lock() {
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
if removed && !plat.is_null() {
unsafe {
let sec = (*plat).sector;
if !sec.is_null() {
(*sec).specialdata = ptr::null_mut();
}
P_RemoveThinker((&raw mut (*plat).thinker) as *mut crate::p_tick::thinker_t);
}
return;
}
doom_error("P_RemoveActivePlat: can't find plat!");
}
