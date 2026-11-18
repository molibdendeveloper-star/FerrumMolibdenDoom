#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{c_int, c_void};
use std::mem::size_of;
use std::ptr;

use crate::p_map::P_ChangeSector;
use crate::p_setup::sectors;
use crate::p_spec::{
    P_FindHighestFloorSurrounding, P_FindLowestCeilingSurrounding, P_FindLowestFloorSurrounding,
    P_FindNextHighestFloor, P_FindSectorFromLineTag, getSector, getSide, twoSided,
};
use crate::p_tick::{P_AddThinker, P_RemoveThinker, leveltime};
use crate::r_data::textureheight;
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
}

// --- T_MovePlane ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_MovePlane(
    sector: *mut sector_t,
    speed: i32,
    dest: i32,
    crush: c_int,
    floor_or_ceiling: c_int,
    direction: c_int,
) -> result_e {
    unsafe {
        let map_sec = sector as *mut crate::p_map::sector_t;

        match floor_or_ceiling {
            0 => {
                // FLOOR
                match direction {
                    -1 => {
                        // DOWN
                        if (*sector).floorheight.wrapping_sub(speed) < dest {
                            let lastpos = (*sector).floorheight;
                            (*sector).floorheight = dest;
                            if P_ChangeSector(map_sec, crush) != 0 {
                                (*sector).floorheight = lastpos;
                                P_ChangeSector(map_sec, crush);
                            }
                            return pastdest;
                        } else {
                            let lastpos = (*sector).floorheight;
                            (*sector).floorheight = (*sector).floorheight.wrapping_sub(speed);
                            if P_ChangeSector(map_sec, crush) != 0 {
                                (*sector).floorheight = lastpos;
                                P_ChangeSector(map_sec, crush);
                                return crushed;
                            }
                        }
                    }
                    1 => {
                        // UP
                        if (*sector).floorheight.wrapping_add(speed) > dest {
                            let lastpos = (*sector).floorheight;
                            (*sector).floorheight = dest;
                            if P_ChangeSector(map_sec, crush) != 0 {
                                (*sector).floorheight = lastpos;
                                P_ChangeSector(map_sec, crush);
                            }
                            return pastdest;
                        } else {
                            let lastpos = (*sector).floorheight;
                            (*sector).floorheight = (*sector).floorheight.wrapping_add(speed);
                            if P_ChangeSector(map_sec, crush) != 0 {
                                if crush != 0 {
                                    return crushed;
                                }
                                (*sector).floorheight = lastpos;
                                P_ChangeSector(map_sec, crush);
                                return crushed;
                            }
                        }
                    }
                    _ => {}
                }
            }

            1 => {
                // CEILING
                match direction {
                    -1 => {
                        // DOWN
                        if (*sector).ceilingheight.wrapping_sub(speed) < dest {
                            let lastpos = (*sector).ceilingheight;
                            (*sector).ceilingheight = dest;
                            if P_ChangeSector(map_sec, crush) != 0 {
                                (*sector).ceilingheight = lastpos;
                                P_ChangeSector(map_sec, crush);
                            }
                            return pastdest;
                        } else {
                            let lastpos = (*sector).ceilingheight;
                            (*sector).ceilingheight = (*sector).ceilingheight.wrapping_sub(speed);
                            if P_ChangeSector(map_sec, crush) != 0 {
                                if crush != 0 {
                                    return crushed;
                                }
                                (*sector).ceilingheight = lastpos;
                                P_ChangeSector(map_sec, crush);
                                return crushed;
                            }
                        }
                    }
                    1 => {
                        // UP
                        if (*sector).ceilingheight.wrapping_add(speed) > dest {
                            let lastpos = (*sector).ceilingheight;
                            (*sector).ceilingheight = dest;
                            if P_ChangeSector(map_sec, crush) != 0 {
                                (*sector).ceilingheight = lastpos;
                                P_ChangeSector(map_sec, crush);
                            }
                            return pastdest;
                        } else {
                            (*sector).ceilingheight = (*sector).ceilingheight.wrapping_add(speed);
                            P_ChangeSector(map_sec, crush);
                        }
                    }
                    _ => {}
                }
            }

            _ => {}
        }

        ok
    }
}

// --- T_MoveFloor ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_MoveFloor(floor: *mut floormove_t) {
    unsafe {
        let sec = (*floor).sector;
        let res = T_MovePlane(
            sec,
            (*floor).speed,
            (*floor).floordestheight,
            (*floor).crush,
            0,
            (*floor).direction,
        );

        let sound_org = (&raw mut (*sec).soundorg) as *mut c_void;
        if (leveltime & 7) == 0 {
            S_StartSound(sound_org, sfx_stnmov as c_int);
        }

        if res == pastdest {
            (*sec).specialdata = ptr::null_mut();

            if (*floor).direction == 1 {
                if (*floor).type_ == donutRaise {
                    (*sec).special = (*floor).newspecial as i16;
                    (*sec).floorpic = (*floor).texture;
                }
            } else if (*floor).direction == -1 {
                if (*floor).type_ == lowerAndChange {
                    (*sec).special = (*floor).newspecial as i16;
                    (*sec).floorpic = (*floor).texture;
                }
            }

            P_RemoveThinker((&raw mut (*floor).thinker) as *mut crate::p_tick::thinker_t);
            S_StartSound(sound_org, sfx_pstop as c_int);
        }
    }
}

// --- EV_DoFloor ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoFloor(line: *mut line_t, floortype: floor_e) -> c_int {
    unsafe {
        let mut secnum = -1;
        let mut rtn = 0;
        let sectors_base = sectors as *mut sector_t;
        let frac_u = FRACUNIT as i32;
        let flr_spd = FLOORSPEED as i32;

        loop {
            secnum = P_FindSectorFromLineTag(line as *mut crate::p_spec::line_t, secnum);
            if secnum < 0 {
                break;
            }

            let mut sec = sectors_base.add(secnum as usize);
            if !(*sec).specialdata.is_null() {
                continue;
            }

            rtn = 1;
            let floor = Z_Malloc(size_of::<floormove_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
                as *mut floormove_t;
            P_AddThinker((&raw mut (*floor).thinker) as *mut crate::p_tick::thinker_t);

            (*sec).specialdata = floor as *mut c_void;
            let move_fn: unsafe extern "C" fn(*mut c_void) =
                std::mem::transmute(T_MoveFloor as unsafe extern "C" fn(*mut floormove_t));
            (*floor).thinker.function.acp1 = Some(move_fn);

            (*floor).type_ = floortype;
            (*floor).crush = 0;

            let spec_sec = sec as *mut crate::p_spec::sector_t;

            match floortype {
                lowerFloor => {
                    (*floor).direction = -1;
                    (*floor).sector = sec;
                    (*floor).speed = flr_spd;
                    (*floor).floordestheight = P_FindHighestFloorSurrounding(spec_sec);
                }
                lowerFloorToLowest => {
                    (*floor).direction = -1;
                    (*floor).sector = sec;
                    (*floor).speed = flr_spd;
                    (*floor).floordestheight = P_FindLowestFloorSurrounding(spec_sec);
                }
                turboLower => {
                    (*floor).direction = -1;
                    (*floor).sector = sec;
                    (*floor).speed = flr_spd * 4;
                    let mut dest = P_FindHighestFloorSurrounding(spec_sec);
                    if dest != (*sec).floorheight {
                        dest += 8 * frac_u;
                    }
                    (*floor).floordestheight = dest;
                }
                raiseFloorCrush | raiseFloor => {
                    if floortype == raiseFloorCrush {
                        (*floor).crush = 1;
                    }
                    (*floor).direction = 1;
                    (*floor).sector = sec;
                    (*floor).speed = flr_spd;
                    let mut dest = P_FindLowestCeilingSurrounding(spec_sec);
                    if dest > (*sec).ceilingheight {
                        dest = (*sec).ceilingheight;
                    }
                    if floortype == raiseFloorCrush {
                        dest -= 8 * frac_u;
                    }
                    (*floor).floordestheight = dest;
                }
                raiseFloorTurbo => {
(*floor).direction = 1;
(*floor).sector = sec;
(*floor).speed = flr_spd * 4;
(*floor).floordestheight = P_FindNextHighestFloor(spec_sec, (*sec).floorheight);
}
raiseFloorToNearest => {
(*floor).direction = 1;
(*floor).sector = sec;
(*floor).speed = flr_spd;
(*floor).floordestheight = P_FindNextHighestFloor(spec_sec, (*sec).floorheight);
}
raiseFloor24 => {
(*floor).direction = 1;
(*floor).sector = sec;
(*floor).speed = flr_spd;
(*floor).floordestheight = (*sec).floorheight + 24 * frac_u;
}
raiseFloor512 => {
(*floor).direction = 1;
(*floor).sector = sec;
(*floor).speed = flr_spd;
(*floor).floordestheight = (*sec).floorheight + 512 * frac_u;
}
raiseFloor24AndChange => {
(*floor).direction = 1;
(*floor).sector = sec;
(*floor).speed = flr_spd;
(*floor).floordestheight = (*sec).floorheight + 24 * frac_u;
let fsec = (*line).frontsector;
(*sec).floorpic = (*fsec).floorpic;
(*sec).special = (*fsec).special;
}
raiseToTexture => {
let mut minsize = i32::MAX;
(*floor).direction = 1;
(*floor).sector = sec;
(*floor).speed = flr_spd;
for i in 0..((*sec).linecount as c_int) {
if twoSided(secnum, i) != 0 {
let side0 = getSide(secnum, i, 0);
if (*side0).bottomtexture >= 0 {
let h = *textureheight.add((*side0).bottomtexture as usize);
if h < minsize {
minsize = h;
}
}
let side1 = getSide(secnum, i, 1);
if (*side1).bottomtexture >= 0 {
let h = *textureheight.add((*side1).bottomtexture as usize);
if h < minsize {
minsize = h;
}
}
}
}
(*floor).floordestheight = (*sec).floorheight.wrapping_add(minsize);
}
lowerAndChange => {
(*floor).direction = -1;
(*floor).sector = sec;
(*floor).speed = flr_spd;
(*floor).floordestheight = P_FindLowestFloorSurrounding(spec_sec);
(*floor).texture = (*sec).floorpic;
for i in 0..((*sec).linecount as c_int) {
if twoSided(secnum, i) != 0 {
let s0 = getSide(secnum, i, 0);
let s0_sec = (*s0).sector as *mut sector_t;
let next_sec = if (s0_sec.offset_from(sectors_base) as c_int) == secnum {
getSector(secnum, i, 1) as *mut sector_t
} else {
getSector(secnum, i, 0) as *mut sector_t
};
sec = next_sec;
if (*sec).floorheight == (*floor).floordestheight {
(*floor).texture = (*sec).floorpic;
(*floor).newspecial = (*sec).special as i32;
break;
}
}
}
}
_ => {}
}
}
rtn
}
}
// --- EV_BuildStairs ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_BuildStairs(line: *mut line_t, stair_type: stair_e) -> c_int {
unsafe {
let mut secnum = -1;
let mut rtn = 0;
let sectors_base = sectors as *mut sector_t;
let frac_u = FRACUNIT as i32;
let flr_spd = FLOORSPEED as i32;
loop {
secnum = P_FindSectorFromLineTag(line as *mut crate::p_spec::line_t, secnum);
if secnum < 0 {
break;
}
let mut sec = sectors_base.add(secnum as usize);
if !(*sec).specialdata.is_null() {
continue;
}
rtn = 1;
let mut floor = Z_Malloc(size_of::<floormove_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
as *mut floormove_t;
P_AddThinker((&raw mut (*floor).thinker) as *mut crate::p_tick::thinker_t);
(*sec).specialdata = floor as *mut c_void;
let move_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_MoveFloor as unsafe extern "C" fn(*mut floormove_t));
(*floor).thinker.function.acp1 = Some(move_fn);
(*floor).direction = 1;
(*floor).sector = sec;
let (speed, stairsize) = match stair_type {
build8 => (flr_spd / 4, 8 * frac_u),
turbo16 => (flr_spd * 4, 16 * frac_u),
_ => (0, 0),
};
(*floor).speed = speed;
let mut height = (*sec).floorheight + stairsize;
(*floor).floordestheight = height;
let texture = (*sec).floorpic;
loop {
let mut found_next = false;
let lines_ptr = (*sec).lines;
for i in 0..((*sec).linecount as usize) {
let ln = *lines_ptr.add(i);
if ((*ln).flags as u32 & ML_TWOSIDED) == 0 {
continue;
}
let fsec = (*ln).frontsector as *mut sector_t;
if (fsec.offset_from(sectors_base) as c_int) != secnum {
continue;
}
let tsec = (*ln).backsector as *mut sector_t;
let newsecnum = tsec.offset_from(sectors_base) as c_int;
if (*tsec).floorpic != texture {
continue;
}
height += stairsize;
if !(*tsec).specialdata.is_null() {
continue;
}
sec = tsec;
secnum = newsecnum;
floor = Z_Malloc(size_of::<floormove_t>() as c_int, PU_LEVSPEC, ptr::null_mut())
as *mut floormove_t;
P_AddThinker((&raw mut (*floor).thinker) as *mut crate::p_tick::thinker_t);
(*sec).specialdata = floor as *mut c_void;
(*floor).thinker.function.acp1 = Some(move_fn);
(*floor).direction = 1;
(*floor).sector = sec;
(*floor).speed = speed;
(*floor).floordestheight = height;
found_next = true;
break;
}
if !found_next {
break;
}
}
}
rtn
}
}