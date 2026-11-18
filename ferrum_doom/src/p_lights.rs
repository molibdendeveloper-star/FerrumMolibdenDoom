#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::m_random::p_random;
use crate::p_setup::{numsectors, sectors};
use crate::p_spec::{P_FindMinSurroundingLight, P_FindSectorFromLineTag, getNextSector};
use crate::p_tick::P_AddThinker;
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

// --- T_FireFlicker ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_FireFlicker(flick: *mut fireflicker_t) {
    unsafe {
        (*flick).count -= 1;
        if (*flick).count != 0 {
            return;
        }

        let amount = ((p_random() & 3) * 16) as c_short;
        let sec = (*flick).sector;
        let min_l = (*flick).minlight as c_short;
        let max_l = (*flick).maxlight as c_short;

        if (*sec).lightlevel - amount < min_l {
            (*sec).lightlevel = min_l;
        } else {
            (*sec).lightlevel = max_l - amount;
        }

        (*flick).count = 4;
    }
}

// --- P_SpawnFireFlicker ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnFireFlicker(sector: *mut sector_t) {
    unsafe {
        (*sector).special = 0;

        let flick = Z_Malloc(size_of::<fireflicker_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
            as *mut fireflicker_t;

        P_AddThinker((&raw mut (*flick).thinker) as *mut crate::p_tick::thinker_t);

        let f: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_FireFlicker as unsafe extern "C" fn(*mut fireflicker_t));
        (*flick).thinker.function.acp1 = Some(f);

        (*flick).sector = sector;
        (*flick).maxlight = (*sector).lightlevel as c_int;
        let min_surr = P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            (*sector).lightlevel as c_int,
        );
        (*flick).minlight = min_surr + 16;
        (*flick).count = 4;
    }
}

// --- T_LightFlash ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_LightFlash(flash: *mut lightflash_t) {
    unsafe {
        (*flash).count -= 1;
        if (*flash).count != 0 {
            return;
        }

        let sec = (*flash).sector;
        let max_l = (*flash).maxlight as c_short;
        let min_l = (*flash).minlight as c_short;

        if (*sec).lightlevel == max_l {
            (*sec).lightlevel = min_l;
            (*flash).count = (p_random() & (*flash).mintime) + 1;
        } else {
            (*sec).lightlevel = max_l;
            (*flash).count = (p_random() & (*flash).maxtime) + 1;
        }
    }
}

// --- P_SpawnLightFlash ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnLightFlash(sector: *mut sector_t) {
    unsafe {
        (*sector).special = 0;

        let flash = Z_Malloc(size_of::<lightflash_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
            as *mut lightflash_t;

        P_AddThinker((&raw mut (*flash).thinker) as *mut crate::p_tick::thinker_t);

        let f: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_LightFlash as unsafe extern "C" fn(*mut lightflash_t));
        (*flash).thinker.function.acp1 = Some(f);

        (*flash).sector = sector;
        (*flash).maxlight = (*sector).lightlevel as c_int;
        (*flash).minlight = P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            (*sector).lightlevel as c_int,
        );
        (*flash).maxtime = 64;
        (*flash).mintime = 7;
        (*flash).count = (p_random() & (*flash).maxtime) + 1;
    }
}

// --- T_StrobeFlash ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_StrobeFlash(flash: *mut strobe_t) {
    unsafe {
        (*flash).count -= 1;
        if (*flash).count != 0 {
            return;
        }

        let sec = (*flash).sector;
        let min_l = (*flash).minlight as c_short;
        let max_l = (*flash).maxlight as c_short;

        if (*sec).lightlevel == min_l {
            (*sec).lightlevel = max_l;
            (*flash).count = (*flash).brighttime;
        } else {
            (*sec).lightlevel = min_l;
            (*flash).count = (*flash).darktime;
        }
    }
}

// --- P_SpawnStrobeFlash ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnStrobeFlash(
    sector: *mut sector_t,
    fast_or_slow: c_int,
    in_sync: c_int,
) {
    unsafe {
        let flash = Z_Malloc(size_of::<strobe_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
            as *mut strobe_t;

        P_AddThinker((&raw mut (*flash).thinker) as *mut crate::p_tick::thinker_t);

        (*flash).sector = sector;
        (*flash).darktime = fast_or_slow;
        (*flash).brighttime = STROBEBRIGHT as c_int;

        let f: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_StrobeFlash as unsafe extern "C" fn(*mut strobe_t));
        (*flash).thinker.function.acp1 = Some(f);

        (*flash).maxlight = (*sector).lightlevel as c_int;
        let min_surr = P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            (*sector).lightlevel as c_int,
        );
        (*flash).minlight = if min_surr == (*flash).maxlight {
            0
        } else {
            min_surr
        };

        (*sector).special = 0;

        if in_sync == 0 {
            (*flash).count = (p_random() & 7) + 1;
        } else {
            (*flash).count = 1;
        }
    }
}

// --- EV_StartLightStrobing ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_StartLightStrobing(line: *mut line_t) {
    unsafe {
        let mut secnum = -1;
        let sectors_base = sectors as *mut sector_t;

        loop {
            secnum = P_FindSectorFromLineTag(line as *mut crate::p_spec::line_t, secnum);
            if secnum < 0 {
                break;
            }

            let sec = sectors_base.add(secnum as usize);
            if !(*sec).specialdata.is_null() {
                continue;
            }

            P_SpawnStrobeFlash(sec, SLOWDARK as c_int, 0);
        }
    }
}

// --- EV_TurnTagLightsOff ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_TurnTagLightsOff(line: *mut line_t) {
    unsafe {
        let sectors_base = sectors as *mut sector_t;
        let tag = (*line).tag;

        for j in 0..(numsectors as usize) {
            let sector = sectors_base.add(j);
            if (*sector).tag == tag {
                let mut min = (*sector).lightlevel;
                let lines_ptr = (*sector).lines;

                for i in 0..((*sector).linecount as usize) {
                    let templine = *lines_ptr.add(i);
                    let tsec = getNextSector(
                        templine as *mut crate::p_spec::line_t,
                        sector as *mut crate::p_spec::sector_t,
                    ) as *mut sector_t;

                    if !tsec.is_null() && (*tsec).lightlevel < min {
                        min = (*tsec).lightlevel;
                    }
                }
                (*sector).lightlevel = min;
            }
        }
    }
}

// --- EV_LightTurnOn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_LightTurnOn(line: *mut line_t, mut bright: c_int) {
    unsafe {
        let sectors_base = sectors as *mut sector_t;
        let tag = (*line).tag;

        for i in 0..(numsectors as usize) {
            let sector = sectors_base.add(i);
            if (*sector).tag == tag {
                if bright == 0 {
                    let lines_ptr = (*sector).lines;
                    for j in 0..((*sector).linecount as usize) {
                        let templine = *lines_ptr.add(j);
                        let temp = getNextSector(
                            templine as *mut crate::p_spec::line_t,
                            sector as *mut crate::p_spec::sector_t,
                        ) as *mut sector_t;

                        if !temp.is_null() && ((*temp).lightlevel as c_int) > bright {
                            bright = (*temp).lightlevel as c_int;
                        }
                    }
                }
                (*sector).lightlevel = bright as c_short;
            }
        }
    }
}

// --- T_Glow ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_Glow(g: *mut glow_t) {
    unsafe {
        let sec = (*g).sector;
        let speed = GLOWSPEED as c_short;
        let min_l = (*g).minlight as c_short;
        let max_l = (*g).maxlight as c_short;

        match (*g).direction {
            -1 => {
                (*sec).lightlevel -= speed;
                if (*sec).lightlevel <= min_l {
                    (*sec).lightlevel += speed;
                    (*g).direction = 1;
                }
            }
            1 => {
                (*sec).lightlevel += speed;
                if (*sec).lightlevel >= max_l {
                    (*sec).lightlevel -= speed;
                    (*g).direction = -1;
                }
            }
            _ => {}
        }
    }
}

// --- P_SpawnGlowingLight ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnGlowingLight(sector: *mut sector_t) {
    unsafe {
        let g = Z_Malloc(size_of::<glow_t>() as c_int, PU_LEVSPEC, ptr::null_mut()) as *mut glow_t;

        P_AddThinker((&raw mut (*g).thinker) as *mut crate::p_tick::thinker_t);

        (*g).sector = sector;
        (*g).minlight = P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            (*sector).lightlevel as c_int,
        );
        (*g).maxlight = (*sector).lightlevel as c_int;

        let f: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_Glow as unsafe extern "C" fn(*mut glow_t));
        (*g).thinker.function.acp1 = Some(f);

        (*g).direction = -1;
        (*sector).special = 0;
    }
}
