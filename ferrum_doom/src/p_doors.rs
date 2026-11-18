#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

use crate::p_floor::t_move_plane;
use crate::p_setup::{sectors, sides};
use crate::p_spec::{P_FindLowestCeilingSurrounding, P_FindSectorFromLineTag};
use crate::p_tick::{P_AddThinker, P_RemoveThinker};
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
}

#[inline]
fn play_sound(origin: *mut c_void, sound_id: u32) {
    unsafe {
        S_StartSound(origin, sound_id as c_int);
    }
}

unsafe fn alloc_door(sec: &mut sector_t) -> &'static mut vldoor_t {
    unsafe {
        let door = Z_Malloc(size_of::<vldoor_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
            as *mut vldoor_t;
        P_AddThinker((&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t);
        sec.specialdata = door as *mut c_void;

        let door_fn: unsafe extern "C" fn(*mut c_void) =
            std::mem::transmute(T_VerticalDoor as unsafe extern "C" fn(*mut vldoor_t));
        (*door).thinker.function.acp1 = Some(door_fn);
        (*door).sector = sec as *mut sector_t;
        &mut *door
    }
}

/// Безопасная логика движения вертикальной двери.
pub fn t_vertical_door(door: &mut vldoor_t) {
    let Some(sec) = (unsafe { door.sector.as_mut() }) else {
        return;
    };
    let sound_org = (&raw mut sec.soundorg) as *mut c_void;

    match door.direction {
        0 => {
            door.topcountdown -= 1;
            if door.topcountdown == 0 {
                match door.type_ {
                    blazeRaise => {
                        door.direction = -1;
                        play_sound(sound_org, sfx_bdcls);
                    }
                    normal => {
                        door.direction = -1;
                        play_sound(sound_org, sfx_dorcls);
                    }
                    close30ThenOpen => {
                        door.direction = 1;
                        play_sound(sound_org, sfx_doropn);
                    }
                    _ => {}
                }
            }
        }
        2 => {
            door.topcountdown -= 1;
            if door.topcountdown == 0 && door.type_ == raiseIn5Mins {
                door.direction = 1;
                door.type_ = normal;
                play_sound(sound_org, sfx_doropn);
            }
        }
        -1 => {
            let res = t_move_plane(sec, door.speed, sec.floorheight, 0, 1, door.direction);

            if res == pastdest {
                match door.type_ {
                    blazeRaise | blazeClose => {
                        sec.specialdata = ptr::null_mut();
                        unsafe {
                            P_RemoveThinker((&raw mut door.thinker) as *mut crate::p_tick::thinker_t);
                        }
                        play_sound(sound_org, sfx_bdcls);
                    }
                    normal | close => {
                        sec.specialdata = ptr::null_mut();
                        unsafe {
                            P_RemoveThinker((&raw mut door.thinker) as *mut crate::p_tick::thinker_t);
                        }
                    }
                    close30ThenOpen => {
                        door.direction = 0;
                        door.topcountdown = 35 * 30;
                    }
                    _ => {}
                }
            } else if res == crushed {
                match door.type_ {
                    blazeClose | close => {}
                    _ => {
                        door.direction = 1;
                        play_sound(sound_org, sfx_doropn);
                    }
                }
            }
        }
        1 => {
            let res = t_move_plane(sec, door.speed, door.topheight, 0, 1, door.direction);

            if res == pastdest {
                match door.type_ {
                    blazeRaise | normal => {
                        door.direction = 0;
                        door.topcountdown = door.topwait;
                    }
                    close30ThenOpen | blazeOpen | open => {
                        sec.specialdata = ptr::null_mut();
                        unsafe {
                            P_RemoveThinker((&raw mut door.thinker) as *mut crate::p_tick::thinker_t);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

pub fn ev_do_locked_door(line: &mut line_t, door_type: vldoor_e, thing: &mut mobj_t) -> c_int {
    let Some(p) = (unsafe { (thing.player as *mut player_t).as_mut() }) else {
        return 0;
    };

    match line.special {
        99 | 133 => {
            if p.cards[it_bluecard as usize] == 0 && p.cards[it_blueskull as usize] == 0 {
                p.message = PD_BLUEO.as_ptr() as *mut c_char;
                play_sound(ptr::null_mut(), sfx_oof);
                return 0;
            }
        }
        134 | 135 => {
            if p.cards[it_redcard as usize] == 0 && p.cards[it_redskull as usize] == 0 {
                p.message = PD_REDO.as_ptr() as *mut c_char;
                play_sound(ptr::null_mut(), sfx_oof);
                return 0;
            }
        }
        136 | 137 => {
            if p.cards[it_yellowcard as usize] == 0 && p.cards[it_yellowskull as usize] == 0 {
                p.message = PD_YELLOWO.as_ptr() as *mut c_char;
                play_sound(ptr::null_mut(), sfx_oof);
                return 0;
            }
        }
        _ => {}
    }

    unsafe { EV_DoDoor(line as *mut line_t, door_type) }
}

pub fn ev_vertical_door(line: &mut line_t, thing: &mut mobj_t) {
    let player = unsafe { (thing.player as *mut player_t).as_mut() };

    match line.special {
        26 | 32 => {
            let Some(p) = player else { return };
            if p.cards[it_bluecard as usize] == 0 && p.cards[it_blueskull as usize] == 0 {
                p.message = PD_BLUEK.as_ptr() as *mut c_char;
                play_sound(ptr::null_mut(), sfx_oof);
                return;
            }
        }
        27 | 34 => {
            let Some(p) = player else { return };
            if p.cards[it_yellowcard as usize] == 0 && p.cards[it_yellowskull as usize] == 0 {
                p.message = PD_YELLOWK.as_ptr() as *mut c_char;
                play_sound(ptr::null_mut(), sfx_oof);
                return;
            }
        }
        28 | 33 => {
            let Some(p) = player else { return };
            if p.cards[it_redcard as usize] == 0 && p.cards[it_redskull as usize] == 0 {
                p.message = PD_REDK.as_ptr() as *mut c_char;
                play_sound(ptr::null_mut(), sfx_oof);
                return;
            }
        }
        _ => {}
    }

    let back_side_idx = line.sidenum[1] as usize;
    let Some(sec) = (unsafe { (*sides.add(back_side_idx)).sector.as_mut() }) else {
        return;
    };

    if !sec.specialdata.is_null() {
        let door = unsafe { &mut *(sec.specialdata as *mut vldoor_t) };
        match line.special {
            1 | 26 | 27 | 28 | 117 => {
                if door.direction == -1 {
                    door.direction = 1;
                } else {
                    if thing.player.is_null() {
                        return;
                    }
                    door.direction = -1;
                }
                return;
            }
            _ => {}
        }
    }

    let sound_org = (&raw mut sec.soundorg) as *mut c_void;
    match line.special {
        117 | 118 => play_sound(sound_org, sfx_bdopn),
        1 | 31 => play_sound(sound_org, sfx_doropn),
        _ => play_sound(sound_org, sfx_doropn),
    }

    let door = unsafe { alloc_door(sec) };
    door.direction = 1;
    let frac_u = FRACUNIT as i32;
    let door_spd = VDOORSPEED as i32;
    door.speed = door_spd;
    door.topwait = VDOORWAIT as c_int;

    match line.special {
        1 | 26 | 27 | 28 => {
            door.type_ = normal;
        }
        31 | 32 | 33 | 34 => {
            door.type_ = open;
            line.special = 0;
        }
        117 => {
            door.type_ = blazeRaise;
            door.speed = door_spd * 4;
        }
        118 => {
            door.type_ = blazeOpen;
            line.special = 0;
            door.speed = door_spd * 4;
        }
        _ => {}
    }

    door.topheight =
        unsafe { P_FindLowestCeilingSurrounding(door.sector as *mut crate::p_spec::sector_t) }
            - 4 * frac_u;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_VerticalDoor(door: *mut vldoor_t) {
    if let Some(d) = unsafe { door.as_mut() } {
        t_vertical_door(d);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoLockedDoor(
    line: *mut line_t,
    door_type: vldoor_e,
    thing: *mut mobj_t,
) -> c_int {
    let (Some(ln), Some(th)) = (unsafe { line.as_mut() }, unsafe { thing.as_mut() }) else {
        return 0;
    };
    ev_do_locked_door(ln, door_type, th)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoDoor(line: *mut line_t, door_type: vldoor_e) -> c_int {
    unsafe {
        let mut secnum = -1;
        let mut rtn = 0;
        let sectors_base = sectors as *mut sector_t;
        let frac_u = FRACUNIT as i32;
        let door_spd = VDOORSPEED as i32;
        let door_wt = VDOORWAIT as c_int;

        loop {
            secnum = P_FindSectorFromLineTag(line as *mut crate::p_spec::line_t, secnum);
            if secnum < 0 {
                break;
            }

            let sec = &mut *sectors_base.add(secnum as usize);
            if !sec.specialdata.is_null() {
                continue;
            }

            rtn = 1;
            let door = alloc_door(sec);
            door.type_ = door_type;
door.topwait = door_wt;
door.speed = door_spd;
let sound_org = (&raw mut sec.soundorg) as *mut c_void;
let spec_sec = (sec as *mut sector_t) as *mut crate::p_spec::sector_t;
match door_type {
blazeClose => {
door.topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
door.direction = -1;
door.speed = door_spd * 4;
play_sound(sound_org, sfx_bdcls);
}
close => {
door.topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
door.direction = -1;
play_sound(sound_org, sfx_dorcls);
}
close30ThenOpen => {
door.topheight = sec.ceilingheight;
door.direction = -1;
play_sound(sound_org, sfx_dorcls);
}
blazeRaise | blazeOpen => {
door.direction = 1;
door.topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
door.speed = door_spd * 4;
if door.topheight != sec.ceilingheight {
play_sound(sound_org, sfx_bdopn);
}
}
normal | open => {
door.direction = 1;
door.topheight = P_FindLowestCeilingSurrounding(spec_sec) - 4 * frac_u;
if door.topheight != sec.ceilingheight {
play_sound(sound_org, sfx_doropn);
}
}
_ => {}
}
}
rtn
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_VerticalDoor(line: *mut line_t, thing: *mut mobj_t) {
if let (Some(ln), Some(th)) = (unsafe { line.as_mut() }, unsafe { thing.as_mut() }) {
ev_vertical_door(ln, th);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnDoorCloseIn30(sec: *mut sector_t) {
if let Some(s) = unsafe { sec.as_mut() } {
s.special = 0;
let door = unsafe { alloc_door(s) };
door.direction = 0;
door.type_ = normal;
door.speed = VDOORSPEED as i32;
door.topcountdown = 30 * 35;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnDoorRaiseIn5Mins(sec: *mut sector_t, secnum: c_int) {
if let Some(s) = unsafe { sec.as_mut() } {
s.special = 0;
let door = unsafe { alloc_door(s) };
door.direction = 2;
door.type_ = raiseIn5Mins;
door.speed = VDOORSPEED as i32;
door.topheight =
unsafe { P_FindLowestCeilingSurrounding(sec as *mut crate::p_spec::sector_t) }
- 4 * (FRACUNIT as i32);
door.topwait = VDOORWAIT as c_int;
door.topcountdown = 5 * 60 * 35;
}
}
