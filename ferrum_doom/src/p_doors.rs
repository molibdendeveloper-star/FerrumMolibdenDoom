#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::p_floor::T_MovePlane;
use crate::p_setup::{sectors, sides};
use crate::p_spec::{P_FindLowestCeilingSurrounding, P_FindSectorFromLineTag};
use crate::p_tick::{P_AddThinker, P_RemoveThinker};
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
}

// --- T_VerticalDoor ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_VerticalDoor(door: *mut vldoor_t) {
    unsafe {
        let sec = (*door).sector;
        let sound_org = (&raw mut (*sec).soundorg) as *mut c_void;

        match (*door).direction {
            0 => {
                // WAITING
                (*door).topcountdown -= 1;
                if (*door).topcountdown == 0 {
                    match (*door).type_ {
                        blazeRaise => {
                            (*door).direction = -1;
                            S_StartSound(sound_org, sfx_bdcls as c_int);
                        }
                        normal => {
                            (*door).direction = -1;
                            S_StartSound(sound_org, sfx_dorcls as c_int);
                        }
                        close30ThenOpen => {
                            (*door).direction = 1;
                            S_StartSound(sound_org, sfx_doropn as c_int);
                        }
                        _ => {}
                    }
                }
            }

            2 => {
                // INITIAL WAIT
                (*door).topcountdown -= 1;
                if (*door).topcountdown == 0 {
                    if (*door).type_ == raiseIn5Mins {
                        (*door).direction = 1;
                        (*door).type_ = normal;
                        S_StartSound(sound_org, sfx_doropn as c_int);
                    }
                }
            }

            -1 => {
                // DOWN
                let res = T_MovePlane(
                    sec as *mut crate::p_floor::sector_t,
                    (*door).speed,
                    (*sec).floorheight,
                    0,
                    1,
                    (*door).direction,
                );

                if res == pastdest {
                    match (*door).type_ {
                        blazeRaise | blazeClose => {
                            (*sec).specialdata = ptr::null_mut();
                            P_RemoveThinker(
                                (&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t,
                            );
                            S_StartSound(sound_org, sfx_bdcls as c_int);
                        }
                        normal | close => {
                            (*sec).specialdata = ptr::null_mut();
                            P_RemoveThinker(
                                (&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t,
                            );
                        }
                        close30ThenOpen => {
                            (*door).direction = 0;
                            (*door).topcountdown = 35 * 30;
                        }
                        _ => {}
                    }
                } else if res == crushed {
                    match (*door).type_ {
                        blazeClose | close => {}
                        _ => {
                            (*door).direction = 1;
                            S_StartSound(sound_org, sfx_doropn as c_int);
                        }
                    }
                }
            }

            1 => {
                // UP
                let res = T_MovePlane(
                    sec as *mut crate::p_floor::sector_t,
                    (*door).speed,
                    (*door).topheight,
                    0,
                    1,
                    (*door).direction,
                );

                if res == pastdest {
                    match (*door).type_ {
                        blazeRaise | normal => {
                            (*door).direction = 0;
                            (*door).topcountdown = (*door).topwait;
                        }
                        close30ThenOpen | blazeOpen | open => {
                            (*sec).specialdata = ptr::null_mut();
                            P_RemoveThinker(
                                (&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t,
                            );
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }
}

// --- EV_DoLockedDoor ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoLockedDoor(
    line: *mut line_t,
    door_type: vldoor_e,
    thing: *mut mobj_t,
) -> c_int {
    unsafe {
        let p = (*thing).player as *mut player_t;
        if p.is_null() {
            return 0;
        }

        match (*line).special {
            99 | 133 => {
                if (*p).cards[it_bluecard as usize] == 0 && (*p).cards[it_blueskull as usize] == 0 {
                    (*p).message = PD_BLUEO.as_ptr() as *mut c_char;
                    S_StartSound(ptr::null_mut(), sfx_oof as c_int);
                    return 0;
                }
            }
            134 | 135 => {
                if (*p).cards[it_redcard as usize] == 0 && (*p).cards[it_redskull as usize] == 0 {
                    (*p).message = PD_REDO.as_ptr() as *mut c_char;
                    S_StartSound(ptr::null_mut(), sfx_oof as c_int);
                    return 0;
                }
            }
            136 | 137 => {
                if (*p).cards[it_yellowcard as usize] == 0
                    && (*p).cards[it_yellowskull as usize] == 0
                {
                    (*p).message = PD_YELLOWO.as_ptr() as *mut c_char;
                    S_StartSound(ptr::null_mut(), sfx_oof as c_int);
                    return 0;
                }
            }
            _ => {}
        }

        EV_DoDoor(line, door_type)
    }
}

// --- EV_DoDoor ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoDoor(line: *mut line_t, door_type: vldoor_e) -> c_int {
    unsafe {
        let mut secnum = -1;
        let mut rtn = 0;
        let sectors_base = sectors as *mut sector_t;
        let frac_u = FRACUNIT as i32;
        let door_spd = VDOORSPEED as i32;
        let door_wt = VDOORWAIT as c_int;

        let door_fn: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_VerticalDoor as unsafe extern "C" fn(*mut vldoor_t));

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
            let door = Z_Malloc(size_of::<vldoor_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
                as *mut vldoor_t;
            P_AddThinker((&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t);

            (*sec).specialdata = door as *mut c_void;
            (*door).thinker.function.acp1 = Some(door_fn);
            (*door).sector = sec;
            (*door).type_ = door_type;
            (*door).topwait = door_wt;
            (*door).speed = door_spd;

            let sound_org = (&raw mut (*sec).soundorg) as *mut c_void;
            let spec_sec = sec as *mut crate::p_spec::sector_t;

            match door_type {
                blazeClose => {
                    (*door).topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
                    (*door).direction = -1;
                    (*door).speed = door_spd * 4;
                    S_StartSound(sound_org, sfx_bdcls as c_int);
                }
                close => {
                    (*door).topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
                    (*door).direction = -1;
                    S_StartSound(sound_org, sfx_dorcls as c_int);
                }
                close30ThenOpen => {
                    (*door).topheight = (*sec).ceilingheight;
                    (*door).direction = -1;
                    S_StartSound(sound_org, sfx_dorcls as c_int);
                }
                blazeRaise | blazeOpen => {
                    (*door).direction = 1;
                    (*door).topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
                    (*door).speed = door_spd * 4;
                    if (*door).topheight != (*sec).ceilingheight {
                        S_StartSound(sound_org, sfx_bdopn as c_int);
                    }
                }
                normal | open => {
                    (*door).direction = 1;
                    (*door).topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
                    if (*door).topheight != (*sec).ceilingheight {
                        S_StartSound(sound_org, sfx_doropn as c_int);
                    }
                }
                _ => {}
            }
        }

        rtn
    }
}

// --- EV_VerticalDoor ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_VerticalDoor(line: *mut line_t, thing: *mut mobj_t) {
    unsafe {
        let side = 0usize;
        let player = (*thing).player as *mut player_t;

        match (*line).special {
            26 | 32 => {
                if player.is_null() {
                    return;
                }
                if (*player).cards[it_bluecard as usize] == 0
                    && (*player).cards[it_blueskull as usize] == 0
                {
(*player).message = PD_BLUEK.as_ptr() as *mut c_char;
S_StartSound(ptr::null_mut(), sfx_oof as c_int);
return;
}
}
27 | 34 => {
if player.is_null() {
return;
}
if (*player).cards[it_yellowcard as usize] == 0
&& (*player).cards[it_yellowskull as usize] == 0
{
(*player).message = PD_YELLOWK.as_ptr() as *mut c_char;
S_StartSound(ptr::null_mut(), sfx_oof as c_int);
return;
}
}
28 | 33 => {
if player.is_null() {
return;
}
if (*player).cards[it_redcard as usize] == 0
&& (*player).cards[it_redskull as usize] == 0
{
(*player).message = PD_REDK.as_ptr() as *mut c_char;
S_StartSound(ptr::null_mut(), sfx_oof as c_int);
return;
}
}
_ => {}
}
let snum_ptr = (&raw const (*line).sidenum) as *const c_short;
let back_side_idx = *snum_ptr.add(side ^ 1) as usize;
let sides_base = sides as *mut side_t;
let sec = (*sides_base.add(back_side_idx)).sector;
if !(*sec).specialdata.is_null() {
let door = (*sec).specialdata as *mut vldoor_t;
match (*line).special {
1 | 26 | 27 | 28 | 117 => {
if (*door).direction == -1 {
(*door).direction = 1;
} else {
if (*thing).player.is_null() {
return;
}
(*door).direction = -1;
}
return;
}
_ => {}
}
}
let sound_org = (&raw mut (*sec).soundorg) as *mut c_void;
match (*line).special {
117 | 118 => S_StartSound(sound_org, sfx_bdopn as c_int),
1 | 31 => S_StartSound(sound_org, sfx_doropn as c_int),
_ => S_StartSound(sound_org, sfx_doropn as c_int),
}
let door = Z_Malloc(size_of::<vldoor_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
as *mut vldoor_t;
P_AddThinker((&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t);
(*sec).specialdata = door as *mut c_void;
let door_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_VerticalDoor as unsafe extern "C" fn(*mut vldoor_t));
(*door).thinker.function.acp1 = Some(door_fn);
(*door).sector = sec;
(*door).direction = 1;
let frac_u = FRACUNIT as i32;
let door_spd = VDOORSPEED as i32;
(*door).speed = door_spd;
(*door).topwait = VDOORWAIT as c_int;
match (*line).special {
1 | 26 | 27 | 28 => {
(*door).type_ = normal;
}
31 | 32 | 33 | 34 => {
(*door).type_ = open;
(*line).special = 0;
}
117 => {
(*door).type_ = blazeRaise;
(*door).speed = door_spd * 4;
}
118 => {
(*door).type_ = blazeOpen;
(*line).special = 0;
(*door).speed = door_spd * 4;
}
_ => {}
}
(*door).topheight =
P_FindLowestCeilingSurrounding(sec as *mut crate::p_spec::sector_t) - 4 * frac_u;
}
}
// --- P_SpawnDoorCloseIn30 ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnDoorCloseIn30(sec: *mut sector_t) {
unsafe {
let door = Z_Malloc(size_of::<vldoor_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
as *mut vldoor_t;
P_AddThinker((&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t);
(*sec).specialdata = door as *mut c_void;
(*sec).special = 0;
let door_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_VerticalDoor as unsafe extern "C" fn(*mut vldoor_t));
(*door).thinker.function.acp1 = Some(door_fn);
(*door).sector = sec;
(*door).direction = 0;
(*door).type_ = normal;
(*door).speed = VDOORSPEED as i32;
(*door).topcountdown = 30 * 35;
}
}
// --- P_SpawnDoorRaiseIn5Mins ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnDoorRaiseIn5Mins(sec: *mut sector_t, _secnum: c_int) {
unsafe {
let door = Z_Malloc(size_of::<vldoor_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
as *mut vldoor_t;
P_AddThinker((&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t);
(*sec).specialdata = door as *mut c_void;
(*sec).special = 0;
let door_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_VerticalDoor as unsafe extern "C" fn(*mut vldoor_t));
(*door).thinker.function.acp1 = Some(door_fn);
(*door).sector = sec;
(*door).direction = 2;
(*door).type_ = raiseIn5Mins;
(*door).speed = VDOORSPEED as i32;
let frac_u = FRACUNIT as i32;
(*door).topheight =
P_FindLowestCeilingSurrounding(sec as *mut crate::p_spec::sector_t) - 4 * frac_u;
(*door).topwait = VDOORWAIT as c_int;
(*door).topcountdown = 5 * 60 * 35;
}
}