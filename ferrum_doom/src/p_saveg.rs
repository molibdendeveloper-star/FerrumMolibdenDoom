#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_short, c_void};
use std::mem::{size_of, size_of_val};
use std::ptr;

use crate::info::{mobjinfo, states};
use crate::m_fixed::FRACBITS;
use crate::p_mobj::{P_MobjThinker, P_RemoveMobj};
use crate::p_tick::{P_AddThinker, P_InitThinkers, thinkercap};
use crate::r_bsp::subsector_t;
use crate::z_zone::{PU_LEVEL, Z_Free, Z_Malloc};

const NUM_PLAYERS: usize = MAXPLAYERS as usize;

// Маркеры типов объектов в файле сохранения
const TC_END: u8 = 0;
const TC_MOBJ: u8 = 1;

// Маркеры специальных эффектов в файле сохранения
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

    static mut activeceilings: [*mut ceiling_t; MAXCEILINGS as usize];
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Savegame error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальная переменная p_saveg.c ---
#[unsafe(no_mangle)]
pub static mut save_p: *mut u8 = ptr::null_mut();

#[inline]
unsafe fn pad_save_p() {
    unsafe {
        let pad = (4usize.wrapping_sub(save_p as usize & 3)) & 3;
        save_p = save_p.add(pad);
    }
}

// --- P_ArchivePlayers ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ArchivePlayers() {
    unsafe {
        let in_game_ptr = (&raw const playeringame) as *const c_int;
        let players_ptr = (&raw const players) as *const player_t;
        let states_base = (&raw const states) as *const state_t;

        for i in 0..NUM_PLAYERS {
            if *in_game_ptr.add(i) == 0 {
                continue;
            }

            pad_save_p();

            let mut tmp_player = ptr::read_unaligned(players_ptr.add(i));
            for j in 0..(NUMPSPRITES as usize) {
                let st = tmp_player.psprites[j].state;
                if !st.is_null() {
                    let st_idx = (st as *const state_t).offset_from(states_base) as usize;
                    tmp_player.psprites[j].state = st_idx as *mut crate::r_things::state_t;
                }
            }
            ptr::copy_nonoverlapping((&raw const tmp_player) as *const u8, save_p, size_of::<player_t>());
            save_p = save_p.add(size_of::<player_t>());
        }
    }
}

// --- P_UnArchivePlayers ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchivePlayers() {
    unsafe {
        let in_game_ptr = (&raw const playeringame) as *const c_int;
        let players_ptr = (&raw mut players) as *mut player_t;
        let states_base = (&raw mut states) as *mut state_t;

        for i in 0..NUM_PLAYERS {
            if *in_game_ptr.add(i) == 0 {
                continue;
            }

            pad_save_p();

            let p = players_ptr.add(i);
            ptr::copy_nonoverlapping((save_p as *const player_t) as *const u8, (p) as *mut u8, size_of::<player_t>());
            save_p = save_p.add(size_of::<player_t>());

            (*p).mo = ptr::null_mut();
            (*p).message = ptr::null_mut();
            (*p).attacker = ptr::null_mut();

            let pspr_ptr = (&raw mut (*p).psprites) as *mut pspdef_t;
            for j in 0..(NUMPSPRITES as usize) {
                let psp = pspr_ptr.add(j);
                if !(*psp).state.is_null() {
                    let st_idx = (*psp).state as usize;
                    (*psp).state = states_base.add(st_idx);
                }
            }
        }
    }
}

// --- P_ArchiveWorld ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ArchiveWorld() {
    unsafe {
        let mut put = save_p as *mut c_short;

        let mut push_i16 = |val: c_short| {
            ptr::write_unaligned(put, val);
            put = put.add(1);
        };

        // Секторы
        for i in 0..(numsectors as usize) {
            let sec = sectors.add(i);
            push_i16(((*sec).floorheight >> FRACBITS) as c_short);
            push_i16(((*sec).ceilingheight >> FRACBITS) as c_short);
            push_i16((*sec).floorpic);
            push_i16((*sec).ceilingpic);
            push_i16((*sec).lightlevel);
            push_i16((*sec).special);
            push_i16((*sec).tag);
        }

        // Линии и стороны
        for i in 0..(numlines as usize) {
            let li = lines.add(i);
            push_i16((*li).flags);
            push_i16((*li).special);
            push_i16((*li).tag);

            let snum_ptr = (&raw const (*li).sidenum) as *const c_short;
            for j in 0..2 {
                let snum = *snum_ptr.add(j);
                if snum == -1 {
                    continue;
                }
                let si = sides.add(snum as usize);
                push_i16(((*si).textureoffset >> FRACBITS) as c_short);
                push_i16(((*si).rowoffset >> FRACBITS) as c_short);
                push_i16((*si).toptexture);
                push_i16((*si).bottomtexture);
                push_i16((*si).midtexture);
            }
        }

        save_p = put as *mut u8;
    }
}

// --- P_UnArchiveWorld ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchiveWorld() {
    unsafe {
        let mut get = save_p as *const c_short;

        let mut pop_i16 = || -> c_short {
            let val = ptr::read_unaligned(get);
            get = get.add(1);
            val
        };

        // Секторы
        for i in 0..(numsectors as usize) {
            let sec = sectors.add(i);
            (*sec).floorheight = (pop_i16() as i32) << FRACBITS;
            (*sec).ceilingheight = (pop_i16() as i32) << FRACBITS;
            (*sec).floorpic = pop_i16();
            (*sec).ceilingpic = pop_i16();
            (*sec).lightlevel = pop_i16();
            (*sec).special = pop_i16();
            (*sec).tag = pop_i16();
            (*sec).specialdata = ptr::null_mut();
            (*sec).soundtarget = ptr::null_mut();
        }

        // Линии и стороны
        for i in 0..(numlines as usize) {
            let li = lines.add(i);
            (*li).flags = pop_i16();
            (*li).special = pop_i16();
            (*li).tag = pop_i16();

            let snum_ptr = (&raw const (*li).sidenum) as *const c_short;
            for j in 0..2 {
                let snum = *snum_ptr.add(j);
                if snum == -1 {
                    continue;
                }
                let si = sides.add(snum as usize);
                (*si).textureoffset = (pop_i16() as i32) << FRACBITS;
                (*si).rowoffset = (pop_i16() as i32) << FRACBITS;
                (*si).toptexture = pop_i16();
                (*si).bottomtexture = pop_i16();
                (*si).midtexture = pop_i16();
            }
        }

        save_p = get as *mut u8;
    }
}

// --- P_ArchiveThinkers ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ArchiveThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        let mut th = (*cap).next;
        let mobj_thinker_addr = P_MobjThinker as *const ();
        let states_base = (&raw const states) as *const state_t;
        let players_base = (&raw const players) as *const player_t;

        while th != cap {
            let is_mobj = match (*th).function.acp1 {
                Some(func) => (func as *const ()) == mobj_thinker_addr,
                None => false,
            };

            if is_mobj {
                *save_p = TC_MOBJ;
                save_p = save_p.add(1);
                pad_save_p();

                let mut tmp_mobj = ptr::read_unaligned(th as *const mobj_t);
                let st_idx = (tmp_mobj.state as *const state_t).offset_from(states_base) as usize;
                tmp_mobj.state = st_idx as *mut state_t;

                if !tmp_mobj.player.is_null() {
                    let p_idx = (tmp_mobj.player as *const player_t).offset_from(players_base) as usize;
                    tmp_mobj.player = (p_idx + 1) as *mut player_t;
                }
                ptr::copy_nonoverlapping((&raw const tmp_mobj) as *const u8, save_p, size_of::<mobj_t>());
                save_p = save_p.add(size_of::<mobj_t>());
            }

            th = (*th).next;
        }

        *save_p = TC_END;
        save_p = save_p.add(1);
    }
}

// --- P_UnArchiveThinkers ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchiveThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        let mut current = (*cap).next;
        let mobj_thinker_addr = P_MobjThinker as *const ();

        while current != cap {
            let next = (*current).next;
            let is_mobj = match (*current).function.acp1 {
                Some(func) => (func as *const ()) == mobj_thinker_addr,
                None => false,
            };

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
loop {
let tclass = *save_p;
save_p = save_p.add(1);
match tclass {
TC_END => return,
TC_MOBJ => {
pad_save_p();
let mobj = Z_Malloc(size_of::<mobj_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut mobj_t;
ptr::copy_nonoverlapping((save_p as *const mobj_t) as *const u8, (mobj) as *mut u8, size_of::<mobj_t>());
save_p = save_p.add(size_of::<mobj_t>());
let st_idx = (*mobj).state as usize;
(*mobj).state = states_base.add(st_idx);
(*mobj).target = ptr::null_mut();
if !(*mobj).player.is_null() {
let p_idx = ((*mobj).player as usize) - 1;
let p = players_base.add(p_idx);
(*mobj).player = p;
(*p).mo = mobj as *mut crate::r_things::mobj_t;
}
P_SetThingPosition(mobj);
(*mobj).info = info_base.add((*mobj).type_ as usize);
let sub = (*mobj).subsector as *mut subsector_t;
let sec = (*sub).sector;
(*mobj).floorz = (*sec).floorheight;
(*mobj).ceilingz = (*sec).ceilingheight;
let thinker_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(P_MobjThinker as unsafe extern "C" fn(*mut crate::p_mobj::mobj_t));
(*mobj).thinker.function.acp1 = Some(thinker_fn);
P_AddThinker((&raw mut (*mobj).thinker) as *mut crate::p_tick::thinker_t);
}
other => {
doom_error(&format!("Unknown tclass {} in savegame", other));
}
}
}
}
}
// --- P_ArchiveSpecials ---
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
let ceil_arr = (&raw const activeceilings) as *const *mut ceiling_t;
while th != cap {
let acp1_opt = (*th).function.acp1;
if acp1_opt.is_none() {
let mut found = false;
for i in 0..(MAXCEILINGS as usize) {
if *ceil_arr.add(i) == (th as *mut ceiling_t) {
found = true;
break;
}
}
if found {
*save_p = TC_CEILING;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const ceiling_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<ceiling_t>());
                    save_p = save_p.add(size_of::<ceiling_t>());
}
th = (*th).next;
continue;
}
let fn_addr = acp1_opt.unwrap() as *const ();
if fn_addr == fn_ceiling {
*save_p = TC_CEILING;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const ceiling_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<ceiling_t>());
                    save_p = save_p.add(size_of::<ceiling_t>());
} else if fn_addr == fn_door {
*save_p = TC_DOOR;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const vldoor_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<vldoor_t>());
                    save_p = save_p.add(size_of::<vldoor_t>());
} else if fn_addr == fn_floor {
*save_p = TC_FLOOR;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const floormove_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<floormove_t>());
                    save_p = save_p.add(size_of::<floormove_t>());
} else if fn_addr == fn_plat {
*save_p = TC_PLAT;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const plat_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<plat_t>());
                    save_p = save_p.add(size_of::<plat_t>());
} else if fn_addr == fn_flash {
*save_p = TC_FLASH;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const lightflash_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<lightflash_t>());
                    save_p = save_p.add(size_of::<lightflash_t>());
} else if fn_addr == fn_strobe {
*save_p = TC_STROBE;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const strobe_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<strobe_t>());
                    save_p = save_p.add(size_of::<strobe_t>());
} else if fn_addr == fn_glow {
*save_p = TC_GLOW;
save_p = save_p.add(1);
pad_save_p();
let mut tmp = ptr::read_unaligned(th as *const glow_t);
                    let sec_idx = tmp.sector.offset_from(sectors) as usize;
                    tmp.sector = sec_idx as *mut sector_t;
                    ptr::copy_nonoverlapping((&raw const tmp) as *const u8, save_p, size_of::<glow_t>());
                    save_p = save_p.add(size_of::<glow_t>());
}
th = (*th).next;
}
*save_p = TC_ENDSPECIALS;
save_p = save_p.add(1);
}
}
// --- P_UnArchiveSpecials ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UnArchiveSpecials() {
unsafe {
loop {
let tclass = *save_p;
save_p = save_p.add(1);
match tclass {
TC_ENDSPECIALS => return,
TC_CEILING => {
pad_save_p();
let ceiling = Z_Malloc(size_of::<ceiling_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut ceiling_t;
ptr::copy_nonoverlapping((save_p as *const ceiling_t) as *const u8, (ceiling) as *mut u8, size_of::<ceiling_t>());
save_p = save_p.add(size_of::<ceiling_t>());
let sec = sectors.add((*ceiling).sector as usize);
(*ceiling).sector = sec;
(*sec).specialdata = ceiling as *mut c_void;
if (*ceiling).thinker.function.acp1.is_some() {
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_MoveCeiling as unsafe extern "C" fn(*mut ceiling_t));
(*ceiling).thinker.function.acp1 = Some(f);
}
P_AddThinker((&raw mut (*ceiling).thinker) as *mut crate::p_tick::thinker_t);
P_AddActiveCeiling(ceiling);
}
TC_DOOR => {
pad_save_p();
let door = Z_Malloc(size_of::<vldoor_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut vldoor_t;
ptr::copy_nonoverlapping((save_p as *const vldoor_t) as *const u8, (door) as *mut u8, size_of::<vldoor_t>());
save_p = save_p.add(size_of::<vldoor_t>());
let sec = sectors.add((*door).sector as usize);
(*door).sector = sec;
(*sec).specialdata = door as *mut c_void;
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_VerticalDoor as unsafe extern "C" fn(*mut vldoor_t));
(*door).thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut (*door).thinker) as *mut crate::p_tick::thinker_t);
}
TC_FLOOR => {
pad_save_p();
let floor = Z_Malloc(size_of::<floormove_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut floormove_t;
ptr::copy_nonoverlapping((save_p as *const floormove_t) as *const u8, (floor) as *mut u8, size_of::<floormove_t>());
save_p = save_p.add(size_of::<floormove_t>());
let sec = sectors.add((*floor).sector as usize);
(*floor).sector = sec;
(*sec).specialdata = floor as *mut c_void;
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_MoveFloor as unsafe extern "C" fn(*mut floormove_t));
(*floor).thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut (*floor).thinker) as *mut crate::p_tick::thinker_t);
}
TC_PLAT => {
pad_save_p();
let plat = Z_Malloc(size_of::<plat_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut plat_t;
ptr::copy_nonoverlapping((save_p as *const plat_t) as *const u8, (plat) as *mut u8, size_of::<plat_t>());
save_p = save_p.add(size_of::<plat_t>());
let sec = sectors.add((*plat).sector as usize);
(*plat).sector = sec;
(*sec).specialdata = plat as *mut c_void;
if (*plat).thinker.function.acp1.is_some() {
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_PlatRaise as unsafe extern "C" fn(*mut plat_t));
(*plat).thinker.function.acp1 = Some(f);
}
P_AddThinker((&raw mut (*plat).thinker) as *mut crate::p_tick::thinker_t);
P_AddActivePlat(plat);
}
TC_FLASH => {
pad_save_p();
let flash = Z_Malloc(size_of::<lightflash_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut lightflash_t;
ptr::copy_nonoverlapping((save_p as *const lightflash_t) as *const u8, (flash) as *mut u8, size_of::<lightflash_t>());
save_p = save_p.add(size_of::<lightflash_t>());
(*flash).sector = sectors.add((*flash).sector as usize);
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_LightFlash as unsafe extern "C" fn(*mut lightflash_t));
(*flash).thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut (*flash).thinker) as *mut crate::p_tick::thinker_t);
}
TC_STROBE => {
pad_save_p();
let strobe = Z_Malloc(size_of::<strobe_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut strobe_t;
ptr::copy_nonoverlapping((save_p as *const strobe_t) as *const u8, (strobe) as *mut u8, size_of::<strobe_t>());
save_p = save_p.add(size_of::<strobe_t>());
(*strobe).sector = sectors.add((*strobe).sector as usize);
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_StrobeFlash as unsafe extern "C" fn(*mut strobe_t));
(*strobe).thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut (*strobe).thinker) as *mut crate::p_tick::thinker_t);
}
TC_GLOW => {
pad_save_p();
let glow = Z_Malloc(size_of::<glow_t>() as c_int, PU_LEVEL, ptr::null_mut())
as *mut glow_t;
ptr::copy_nonoverlapping((save_p as *const glow_t) as *const u8, (glow) as *mut u8, size_of::<glow_t>());
save_p = save_p.add(size_of::<glow_t>());
(*glow).sector = sectors.add((*glow).sector as usize);
let f: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_Glow as unsafe extern "C" fn(*mut glow_t));
(*glow).thinker.function.acp1 = Some(f);
P_AddThinker((&raw mut (*glow).thinker) as *mut crate::p_tick::thinker_t);
}
other => {
doom_error(&format!("P_UnarchiveSpecials:Unknown tclass {} in savegame", other));
}
}
}
}
}