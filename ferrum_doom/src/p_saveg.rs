#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::info::{mobjinfo, states};
use crate::m_fixed::FRACBITS;
use crate::p_mobj::{P_MobjThinker, P_RemoveMobj};
use crate::p_tick::{P_AddThinker, P_InitThinkers, thinkercap};
use crate::r_bsp::subsector_t;
use crate::z_zone::{PU_LEVEL, Z_Free, Z_Malloc};

const NUM_PLAYERS: usize = MAXPLAYERS as usize;

const TC_END: u8 = 0;
const TC_MOBJ: u8 = 1;

const TC_CEILING: u8 = 0;
const TC_DOOR: u8 = 1;
const TC_FLOOR: u8 = 2;
const TC_PLAT: u8 = 3;
const TC_FLASH: u8 = 4;
const TC_STROBE: u8 = 5;
const TC_GLOW: u8 = 6;
const TC_ENDSPECIALS: u8 = 7;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn P_SetThingPosition(thing: *mut mobj_t);

    fn T_MoveCeiling(ceiling: *mut ceiling_t);
    fn T_VerticalDoor(door: *mut vldoor_t);
    fn T_MoveFloor(floor: *mut floormove_t);
    fn T_PlatRaise(plat: *mut plat_t);
    fn T_LightFlash(flash: *mut lightflash_t);
    fn T_StrobeFlash(strobe: *mut strobe_t);
    fn T_Glow(glow: *mut glow_t);

    fn P_AddActiveCeiling(c: *mut ceiling_t);
    fn P_AddActivePlat(p: *mut plat_t);

    static mut playeringame: [c_int; NUM_PLAYERS];
    static mut players: [player_t; NUM_PLAYERS];

    static mut numsectors: c_int;
    static mut sectors: *mut sector_t;
    static mut numlines: c_int;
    static mut lines: *mut line_t;
    static mut sides: *mut side_t;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Savegame error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

#[unsafe(no_mangle)]
pub static mut save_p: *mut u8 = ptr::null_mut();

#[inline]
unsafe fn pad_save_p() {
    unsafe {
        let pad = (4usize.wrapping_sub(save_p as usize & 3)) & 3;
        save_p = save_p.add(pad);
    }
}

#[inline]
unsafe fn push_byte(val: u8) {
    unsafe {
        *save_p = val;
        save_p = save_p.add(1);
    }
}

#[inline]
unsafe fn pop_byte() -> u8 {
    unsafe {
        let val = *save_p;
        save_p = save_p.add(1);
        val
    }
}

#[inline]
unsafe fn write_struct<T>(val: &T) {
    unsafe {
        ptr::copy_nonoverlapping((val as *const T) as *const u8, save_p, size_of::<T>());
        save_p = save_p.add(size_of::<T>());
    }
}

#[inline]
unsafe fn alloc_and_read_struct<T>() -> &'static mut T {
    unsafe {
        let ptr = Z_Malloc(size_of::<T>() as c_int, PU_LEVEL, ptr::null_mut()) as *mut T;
        ptr::copy_nonoverlapping(save_p, ptr as *mut u8, size_of::<T>());
        save_p = save_p.add(size_of::<T>());
        &mut *ptr
    }
}

unsafe fn archive_sector_thinker<T>(
    tag: u8,
    th: *mut crate::p_tick::thinker_t,
    get_sector: impl FnOnce(&mut T) -> &mut *mut sector_t,
) {
    unsafe {
        push_byte(tag);
        pad_save_p();
        let mut tmp = ptr::read_unaligned(th as *const T);
        let sec_field = get_sector(&mut tmp);
        let sec_idx = (*sec_field).offset_from(sectors) as usize;
        *sec_field = sec_idx as *mut sector_t;
        write_struct(&tmp);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ArchivePlayers() {
    unsafe {
        let in_game = (&raw const playeringame).as_ref().unwrap();
        let players_arr = (&raw const players).as_ref().unwrap();
        let states_base = (&raw const states) as *const state_t;

        for i in 0..NUM_PLAYERS {
            if in_game[i] == 0 {
                continue;
            }
            pad_save_p();

            let mut tmp_player = ptr::read_unaligned(&players_arr[i]);
            for j in 0..(NUMPSPRITES as usize) {
                let st = tmp_player.psprites[j].state;
                if !st.is_null() {
                    let st_idx = (st as *const state_t).offset_from(states_base) as usize;
                    tmp_player.psprites[j].state = st_idx as *mut crate::r_things::state_t;
                }
            }
            write_struct(&tmp_player);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchivePlayers() {
    unsafe {
        let in_game = (&raw const playeringame).as_ref().unwrap();
        let players_arr = (&raw mut players).as_mut().unwrap();
        let states_base = (&raw mut states) as *mut state_t;

        for i in 0..NUM_PLAYERS {
            if in_game[i] == 0 {
                continue;
            }
            pad_save_p();

            let p = &mut players_arr[i];
            ptr::copy_nonoverlapping(save_p, (p as *mut player_t) as *mut u8, size_of::<player_t>());
            save_p = save_p.add(size_of::<player_t>());

            p.mo = ptr::null_mut();
            p.message = ptr::null_mut();
            p.attacker = ptr::null_mut();

            for j in 0..(NUMPSPRITES as usize) {
                let psp = &mut p.psprites[j];
                if !psp.state.is_null() {
                    let st_idx = psp.state as usize;
                    psp.state = states_base.add(st_idx);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ArchiveWorld() {
    unsafe {
        let mut put = save_p as *mut c_short;
        let mut push_i16 = |val: c_short| {
            ptr::write_unaligned(put, val);
            put = put.add(1);
        };

        let secs = std::slice::from_raw_parts(sectors, numsectors as usize);
        for sec in secs {
            push_i16((sec.floorheight >> FRACBITS) as c_short);
            push_i16((sec.ceilingheight >> FRACBITS) as c_short);
            push_i16(sec.floorpic);
            push_i16(sec.ceilingpic);
            push_i16(sec.lightlevel);
            push_i16(sec.special);
            push_i16(sec.tag);
        }

        let lns = std::slice::from_raw_parts(lines, numlines as usize);
        for li in lns {
            push_i16(li.flags);
            push_i16(li.special);
            push_i16(li.tag);

            for &snum in &li.sidenum {
                if snum == -1 {
                    continue;
                }
                let si = &*sides.add(snum as usize);
                push_i16((si.textureoffset >> FRACBITS) as c_short);
                push_i16((si.rowoffset >> FRACBITS) as c_short);
                push_i16(si.toptexture);
                push_i16(si.bottomtexture);
                push_i16(si.midtexture);
            }
        }

        save_p = put as *mut u8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchiveWorld() {
    unsafe {
        let mut get = save_p as *const c_short;
        let mut pop_i16 = || -> c_short {
            let val = ptr::read_unaligned(get);
            get = get.add(1);
            val
        };

        let secs = std::slice::from_raw_parts_mut(sectors, numsectors as usize);
        for sec in secs {
            sec.floorheight = (pop_i16() as i32) << FRACBITS;
            sec.ceilingheight = (pop_i16() as i32) << FRACBITS;
            sec.floorpic = pop_i16();
            sec.ceilingpic = pop_i16();
            sec.lightlevel = pop_i16();
            sec.special = pop_i16();
            sec.tag = pop_i16();
            sec.specialdata = ptr::null_mut();
            sec.soundtarget = ptr::null_mut();
        }

        let lns = std::slice::from_raw_parts_mut(lines, numlines as usize);
        for li in lns {
            li.flags = pop_i16();
            li.special = pop_i16();
            li.tag = pop_i16();

            for &snum in &li.sidenum {
                if snum == -1 {
                    continue;
                }
                let si = &mut *sides.add(snum as usize);
                si.textureoffset = (pop_i16() as i32) << FRACBITS;
                si.rowoffset = (pop_i16() as i32) << FRACBITS;
                si.toptexture = pop_i16();
                si.bottomtexture = pop_i16();
                si.midtexture = pop_i16();
            }
        }

        save_p = get as *mut u8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ArchiveThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        let mut th = (*cap).next;
        let mobj_thinker_addr = P_MobjThinker as *const ();
        let states_base = (&raw const states) as *const state_t;
        let players_base = (&raw const players) as *const player_t;

        while th != cap {
            let is_mobj = (*th)
                .function
                .acp1
                .map(|f| (f as *const ()) == mobj_thinker_addr)
                .unwrap_or(false);

            if is_mobj {
                push_byte(TC_MOBJ);
                pad_save_p();

                let mut tmp_mobj = ptr::read_unaligned(th as *const mobj_t);
                let st_idx = (tmp_mobj.state as *const state_t).offset_from(states_base) as usize;
                tmp_mobj.state = st_idx as *mut state_t;

                if !tmp_mobj.player.is_null() {
                    let p_idx = (tmp_mobj.player as *const player_t).offset_from(players_base) as usize;
                    tmp_mobj.player = (p_idx + 1) as *mut player_t;
                }
                write_struct(&tmp_mobj);
            }

            th = (*th).next;
        }

        push_byte(TC_END);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchiveThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        let mut current = (*cap).next;
        let mobj_thinker_addr = P_MobjThinker as *const ();

        while current != cap {
            let next = (*current).next;
            let is_mobj = (*current)
                .function
                .acp1
                .map(|f| (f as *const ()) == mobj_thinker_addr)
                .unwrap_or(false);

            if is_mobj {
                P_RemoveMobj(current as *mut crate::p_mobj::mobj_t);
            } else {
Z_Free(current as *mut c_void);
}
current = next;
}
P_InitThinkers();
let states_base = (&raw mut states) as *mut state_t;
let players_base = (&raw mut players) as *mut player_t;
let info_base = (&raw mut mobjinfo) as *mut mobjinfo_t;
let thinker_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(P_MobjThinker as unsafe extern "C" fn(*mut crate::p_mobj::mobj_t));
loop {
match pop_byte() {
TC_END => return,
TC_MOBJ => {
pad_save_p();
let mobj: &mut mobj_t = alloc_and_read_struct();
let st_idx = mobj.state as usize;
mobj.state = states_base.add(st_idx);
mobj.target = ptr::null_mut();
if !mobj.player.is_null() {
let p_idx = (mobj.player as usize) - 1;
let p = &mut *players_base.add(p_idx);
mobj.player = p;
p.mo = (mobj as *mut mobj_t) as *mut crate::r_things::mobj_t;
}
P_SetThingPosition(mobj);
mobj.info = info_base.add(mobj.type_ as usize);
let sub = &*(mobj.subsector as *const subsector_t);
let sec = &*sub.sector;
mobj.floorz = sec.floorheight;
mobj.ceilingz = sec.ceilingheight;
mobj.thinker.function.acp1 = Some(thinker_fn);
P_AddThinker((&raw mut mobj.thinker) as *mut crate::p_tick::thinker_t);
}
other => {
doom_error(&format!("Unknown tclass {} in savegame", other));
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ArchiveSpecials() {
unsafe {
let cap = &raw mut thinkercap;
let mut th = (*cap).next;
let fn_ceiling = T_MoveCeiling as *const ();
let fn_door = T_VerticalDoor as *const ();
let fn_floor = T_MoveFloor as *const ();
let fn_plat = T_PlatRaise as *const ();
let fn_flash = T_LightFlash as *const ();
let fn_strobe = T_StrobeFlash as *const ();
let fn_glow = T_Glow as *const ();
while th != cap {
let Some(acp1) = (*th).function.acp1 else {
if crate::p_ceilng::has_active_ceiling(th as *mut crate::p_ceilng::ceiling_t) {
archive_sector_thinker::<ceiling_t>(TC_CEILING, th, |c| &mut c.sector);
}
th = (*th).next;
continue;
};
let fn_addr = acp1 as *const ();
if fn_addr == fn_ceiling {
archive_sector_thinker::<ceiling_t>(TC_CEILING, th, |c| &mut c.sector);
} else if fn_addr == fn_door {
archive_sector_thinker::<vldoor_t>(TC_DOOR, th, |d| &mut d.sector);
} else if fn_addr == fn_floor {
archive_sector_thinker::<floormove_t>(TC_FLOOR, th, |f| &mut f.sector);
} else if fn_addr == fn_plat {
archive_sector_thinker::<plat_t>(TC_PLAT, th, |p| &mut p.sector);
} else if fn_addr == fn_flash {
archive_sector_thinker::<lightflash_t>(TC_FLASH, th, |f| &mut f.sector);
} else if fn_addr == fn_strobe {
archive_sector_thinker::<strobe_t>(TC_STROBE, th, |s| &mut s.sector);
} else if fn_addr == fn_glow {
archive_sector_thinker::<glow_t>(TC_GLOW, th, |g| &mut g.sector);
}
th = (*th).next;
}
push_byte(TC_ENDSPECIALS);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchiveSpecials() {
unsafe {
loop {
match pop_byte() {
TC_ENDSPECIALS => return,
TC_CEILING => {
pad_save_p();
let ceiling: &mut ceiling_t = alloc_and_read_struct();
let sec = &mut *sectors.add(ceiling.sector as usize);
ceiling.sector = sec;
sec.specialdata = (ceiling as *mut ceiling_t) as *mut c_void;
if ceiling.thinker.function.acp1.is_some() {
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_MoveCeiling as unsafe extern "C" fn(*mut ceiling_t));
ceiling.thinker.function.acp1 = Some(f);
}
P_AddThinker((&raw mut ceiling.thinker) as *mut crate::p_tick::thinker_t);
P_AddActiveCeiling(ceiling);
}
TC_DOOR => {
pad_save_p();
let door: &mut vldoor_t = alloc_and_read_struct();
let sec = &mut *sectors.add(door.sector as usize);
door.sector = sec;
sec.specialdata = (door as *mut vldoor_t) as *mut c_void;
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_VerticalDoor as unsafe extern "C" fn(*mut vldoor_t));
door.thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut door.thinker) as *mut crate::p_tick::thinker_t);
}
TC_FLOOR => {
pad_save_p();
let floor: &mut floormove_t = alloc_and_read_struct();
let sec = &mut *sectors.add(floor.sector as usize);
floor.sector = sec;
sec.specialdata = (floor as *mut floormove_t) as *mut c_void;
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_MoveFloor as unsafe extern "C" fn(*mut floormove_t));
floor.thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut floor.thinker) as *mut crate::p_tick::thinker_t);
}
TC_PLAT => {
pad_save_p();
let plat: &mut plat_t = alloc_and_read_struct();
let sec = &mut *sectors.add(plat.sector as usize);
plat.sector = sec;
sec.specialdata = (plat as *mut plat_t) as *mut c_void;
if plat.thinker.function.acp1.is_some() {
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_PlatRaise as unsafe extern "C" fn(*mut plat_t));
plat.thinker.function.acp1 = Some(f);
}
P_AddThinker((&raw mut plat.thinker) as *mut crate::p_tick::thinker_t);
P_AddActivePlat(plat);
}
TC_FLASH => {
pad_save_p();
let flash: &mut lightflash_t = alloc_and_read_struct();
flash.sector = sectors.add(flash.sector as usize);
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_LightFlash as unsafe extern "C" fn(*mut lightflash_t));
flash.thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut flash.thinker) as *mut crate::p_tick::thinker_t);
}
TC_STROBE => {
pad_save_p();
let strobe: &mut strobe_t = alloc_and_read_struct();
strobe.sector = sectors.add(strobe.sector as usize);
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_StrobeFlash as unsafe extern "C" fn(*mut strobe_t));
strobe.thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut strobe.thinker) as *mut crate::p_tick::thinker_t);
}
TC_GLOW => {
pad_save_p();
let glow: &mut glow_t = alloc_and_read_struct();
glow.sector = sectors.add(glow.sector as usize);
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_Glow as unsafe extern "C" fn(*mut glow_t));
glow.thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut glow.thinker) as *mut crate::p_tick::thinker_t);
}
other => {
doom_error(&format!("P_UnarchiveSpecials:Unknown tclass {} in savegame", other));
}
}
}
}
}
