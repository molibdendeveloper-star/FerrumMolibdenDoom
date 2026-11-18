#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CStr, CString, c_char, c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::m_argv::myargv;
use crate::m_random::p_random;
use crate::p_switch::{button_t, buttonlist};
use crate::p_telept::EV_Teleport;
use crate::p_tick::{P_AddThinker, leveltime};
use crate::r_bsp::subsector_t;
use crate::r_data::{
    R_CheckTextureNumForName, R_FlatNumForName, R_TextureNumForName, flattranslation,
    texturetranslation,
};
use crate::w_wad::W_CheckNumForName;
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

pub const MAXANIMS: usize = 32;
pub const MAXLINEANIMS: usize = 64;
const MAX_ADJOINING_SECTORS: usize = 20;
const BWHERE_TOP: u32 = 0;
const BWHERE_MIDDLE: u32 = 1;
const BWHERE_BOTTOM: u32 = 2;
const CF_GODMODE_FLAG: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct anim_t {
    pub istexture: c_int,
    pub picnum: c_int,
    pub basepic: c_int,
    pub numpics: c_int,
    pub speed: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct animdef_t {
    pub istexture: c_int,
    pub endname: [c_char; 9],
    pub startname: [c_char; 9],
    pub speed: c_int,
}

const fn make_anim(istex: c_int, end: &[u8; 9], start: &[u8; 9], spd: c_int) -> animdef_t {
    let mut e = [0 as c_char; 9];
    let mut s = [0 as c_char; 9];
    let mut i = 0;
    while i < 9 {
        e[i] = end[i] as c_char;
        s[i] = start[i] as c_char;
        i += 1;
    }
    animdef_t {
        istexture: istex,
        endname: e,
        startname: s,
        speed: spd,
    }
}

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn M_CheckParm(check: *const c_char) -> c_int;
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn G_ExitLevel();
    fn G_SecretExitLevel();

    fn EV_DoDoor(line: *mut line_t, door_type: vldoor_e) -> c_int;
    fn EV_DoFloor(line: *mut line_t, floortype: floor_e) -> c_int;
    fn EV_DoCeiling(line: *mut line_t, ceil_type: ceiling_e) -> c_int;
    fn EV_BuildStairs(line: *mut line_t, stair_type: stair_e) -> c_int;
    fn EV_DoPlat(line: *mut line_t, plat_type: plattype_e, amount: c_int) -> c_int;
    fn EV_StopPlat(line: *mut line_t);
    fn EV_CeilingCrushStop(line: *mut line_t) -> c_int;
    fn EV_LightTurnOn(line: *mut line_t, bright: c_int);
    fn EV_StartLightStrobing(line: *mut line_t);
    fn EV_TurnTagLightsOff(line: *mut line_t);

    fn P_ChangeSwitchTexture(line: *mut line_t, use_again: c_int);
    fn P_DamageMobj(target: *mut mobj_t, inflictor: *mut mobj_t, source: *mut mobj_t, damage: c_int);
    fn T_MoveFloor(floor: *mut floormove_t);

    fn P_SpawnLightFlash(sector: *mut sector_t);
    fn P_SpawnStrobeFlash(sector: *mut sector_t, fast_or_slow: c_int, in_sync: c_int);
    fn P_SpawnGlowingLight(sector: *mut sector_t);
    fn P_SpawnDoorCloseIn30(sector: *mut sector_t);
    fn P_SpawnDoorRaiseIn5Mins(sector: *mut sector_t, secnum: c_int);
    fn P_SpawnFireFlicker(sector: *mut sector_t);

    static mut sectors: *mut sector_t;
    static mut numsectors: c_int;
    static mut sides: *mut side_t;
    static mut lines: *mut line_t;
    static mut numlines: c_int;
    static mut deathmatch: c_int;
    static mut totalsecret: c_int;
    static mut activeceilings: [*mut ceiling_t; MAXCEILINGS as usize];
    static mut activeplats: [*mut plat_t; MAXPLATS as usize];
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Spec error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные p_spec.c ---
#[unsafe(no_mangle)]
pub static mut animdefs: [animdef_t; 23] = [
    make_anim(0, b"NUKAGE3\0\0", b"NUKAGE1\0\0", 8),
    make_anim(0, b"FWATER4\0\0", b"FWATER1\0\0", 8),
    make_anim(0, b"SWATER4\0\0", b"SWATER1\0\0", 8),
    make_anim(0, b"LAVA4\0\0\0\0", b"LAVA1\0\0\0\0", 8),
    make_anim(0, b"BLOOD3\0\0\0", b"BLOOD1\0\0\0", 8),
    // DOOM II flat animations
    make_anim(0, b"RROCK08\0\0", b"RROCK05\0\0", 8),
    make_anim(0, b"SLIME04\0\0", b"SLIME01\0\0", 8),
    make_anim(0, b"SLIME08\0\0", b"SLIME05\0\0", 8),
    make_anim(0, b"SLIME12\0\0", b"SLIME09\0\0", 8),
    make_anim(1, b"BLODGR4\0\0", b"BLODGR1\0\0", 8),
    make_anim(1, b"SLADRIP3\0", b"SLADRIP1\0", 8),
    make_anim(1, b"BLODRIP4\0", b"BLODRIP1\0", 8),
    make_anim(1, b"FIREWALL\0", b"FIREWALA\0", 8),
    make_anim(1, b"GSTFONT3\0", b"GSTFONT1\0", 8),
    make_anim(1, b"FIRELAVA\0", b"FIRELAV3\0", 8),
    make_anim(1, b"FIREMAG3\0", b"FIREMAG1\0", 8),
    make_anim(1, b"FIREBLU2\0", b"FIREBLU1\0", 8),
    make_anim(1, b"ROCKRED3\0", b"ROCKRED1\0", 8),
    make_anim(1, b"BFALL4\0\0\0", b"BFALL1\0\0\0", 8),
    make_anim(1, b"SFALL4\0\0\0", b"SFALL1\0\0\0", 8),
    make_anim(1, b"WFALL4\0\0\0", b"WFALL1\0\0\0", 8),
    make_anim(1, b"DBRAIN4\0\0", b"DBRAIN1\0\0", 8),
    // Терминатор списка (-1)
    make_anim(-1, b"\0\0\0\0\0\0\0\0\0", b"\0\0\0\0\0\0\0\0\0", 0),
];

#[unsafe(no_mangle)]
pub static mut anims: [anim_t; MAXANIMS] = [anim_t {
    istexture: 0,
    picnum: 0,
    basepic: 0,
    numpics: 0,
    speed: 0,
}; MAXANIMS];

#[unsafe(no_mangle)]
pub static mut lastanim: *mut anim_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut levelTimer: c_int = 0;

#[unsafe(no_mangle)]
pub static mut levelTimeCount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut numlinespecials: c_short = 0;

#[unsafe(no_mangle)]
pub static mut linespeciallist: [*mut line_t; MAXLINEANIMS] = [ptr::null_mut(); MAXLINEANIMS];

// --- P_InitPicAnims ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InitPicAnims() {
    unsafe {
        let defs_ptr = (&raw const animdefs) as *const animdef_t;
        lastanim = (&raw mut anims) as *mut anim_t;

        let mut i = 0usize;
        while (*defs_ptr.add(i)).istexture != -1 {
            let def = &*defs_ptr.add(i);
            let start_ptr = def.startname.as_ptr() as *mut c_char;
            let end_ptr = def.endname.as_ptr() as *mut c_char;

            if def.istexture != 0 {
                if R_CheckTextureNumForName(start_ptr) == -1 {
                    i += 1;
                    continue;
                }
                (*lastanim).picnum = R_TextureNumForName(end_ptr);
                (*lastanim).basepic = R_TextureNumForName(start_ptr);
            } else {
                if W_CheckNumForName(start_ptr) == -1 {
                    i += 1;
                    continue;
                }
                (*lastanim).picnum = R_FlatNumForName(end_ptr);
                (*lastanim).basepic = R_FlatNumForName(start_ptr);
            }

            (*lastanim).istexture = def.istexture;
            (*lastanim).numpics = (*lastanim).picnum - (*lastanim).basepic + 1;

            if (*lastanim).numpics < 2 {
                let s_str = CStr::from_ptr(start_ptr).to_str().unwrap_or("?");
                let e_str = CStr::from_ptr(end_ptr).to_str().unwrap_or("?");
                doom_error(&format!("P_InitPicAnims: bad cycle from {} to {}", s_str, e_str));
            }

            (*lastanim).speed = def.speed;
            lastanim = lastanim.add(1);
            i += 1;
        }
    }
}

// --- Утилиты секторов и линий ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getSide(
    current_sector: c_int,
    line: c_int,
    side: c_int,
) -> *mut side_t {
    unsafe {
        let sec = sectors.add(current_sector as usize);
        let ln = *(*sec).lines.add(line as usize);
        let snum = *(&raw const (*ln).sidenum as *const i16).add(side as usize);
        sides.add(snum as usize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getSector(
    current_sector: c_int,
    line: c_int,
    side: c_int,
) -> *mut sector_t {
    unsafe {
        let s = getSide(current_sector, line, side);
        (*s).sector
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn twoSided(sector: c_int, line: c_int) -> c_int {
    unsafe {
        let sec = sectors.add(sector as usize);
        let ln = *(*sec).lines.add(line as usize);
        ((*ln).flags as u32 & ML_TWOSIDED) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getNextSector(line: *mut line_t, sec: *mut sector_t) -> *mut sector_t {
    unsafe {
        if ((*line).flags as u32 & ML_TWOSIDED) == 0 {
            return ptr::null_mut();
        }
        if ((*line).frontsector as *mut sector_t) == sec {
            (*line).backsector as *mut sector_t
        } else {
            (*line).frontsector as *mut sector_t
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindLowestFloorSurrounding(sec: *mut sector_t) -> i32 {
    unsafe {
        let mut floor = (*sec).floorheight;
        for i in 0..((*sec).linecount as usize) {
            let check = *(*sec).lines.add(i);
            let other = getNextSector(check, sec);
            if !other.is_null() && (*other).floorheight < floor {
                floor = (*other).floorheight;
            }
        }
        floor
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindHighestFloorSurrounding(sec: *mut sector_t) -> i32 {
    unsafe {
        let mut floor = -500 * (FRACUNIT as i32);
        for i in 0..((*sec).linecount as usize) {
            let check = *(*sec).lines.add(i);
            let other = getNextSector(check, sec);
            if !other.is_null() && (*other).floorheight > floor {
                floor = (*other).floorheight;
            }
        }
        floor
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindNextHighestFloor(
    sec: *mut sector_t,
    currentheight: c_int,
) -> i32 {
    unsafe {
        let mut h = 0usize;
        let mut heightlist = [0i32; MAX_ADJOINING_SECTORS];

        for i in 0..((*sec).linecount as usize) {
            let check = *(*sec).lines.add(i);
            let other = getNextSector(check, sec);
            if other.is_null() {
                continue;
            }
if (*other).floorheight > currentheight {
heightlist[h] = (*other).floorheight;
h += 1;
}
if h >= MAX_ADJOINING_SECTORS {
eprintln!("Sector with more than 20 adjoining sectors");
break;
}
}
if h == 0 {
return currentheight;
}
let mut min = *heightlist.as_ptr();
for i in 1..h {
if heightlist[i] < min {
min = heightlist[i];
}
}
min
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindLowestCeilingSurrounding(sec: *mut sector_t) -> i32 {
unsafe {
let mut height = i32::MAX;
for i in 0..((*sec).linecount as usize) {
let check = *(*sec).lines.add(i);
let other = getNextSector(check, sec);
if !other.is_null() && (*other).ceilingheight < height {
height = (*other).ceilingheight;
}
}
height
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindHighestCeilingSurrounding(sec: *mut sector_t) -> i32 {
unsafe {
let mut height = 0i32;
for i in 0..((*sec).linecount as usize) {
let check = *(*sec).lines.add(i);
let other = getNextSector(check, sec);
if !other.is_null() && (*other).ceilingheight > height {
height = (*other).ceilingheight;
}
}
height
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindSectorFromLineTag(line: *mut line_t, start: c_int) -> c_int {
unsafe {
let tag = (*line).tag;
for i in (start + 1)..numsectors {
if (*sectors.add(i as usize)).tag == tag {
return i;
}
}
-1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindMinSurroundingLight(sector: *mut sector_t, max: c_int) -> c_int {
unsafe {
let mut min = max;
for i in 0..((*sector).linecount as usize) {
let line = *(*sector).lines.add(i);
let check = getNextSector(line, sector);
if !check.is_null() && ((*check).lightlevel as c_int) < min {
min = (*check).lightlevel as c_int;
}
}
min
}
}
// --- P_CrossSpecialLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CrossSpecialLine(linenum: c_int, side: c_int, thing: *mut mobj_t) {
unsafe {
let line = lines.add(linenum as usize);
if (*thing).player.is_null() {
match (*thing).type_ {
MT_ROCKET | MT_PLASMA | MT_BFG | MT_TROOPSHOT | MT_HEADSHOT | MT_BRUISERSHOT => {
return;
}
_ => {}
}
let is_ok = matches!((*line).special, 39 | 97 | 125 | 126 | 4 | 10 | 88);
if !is_ok {
return;
}
}
match (*line).special {
// TRIGGERS
2 => {
EV_DoDoor(line, open);
(*line).special = 0;
}
3 => {
EV_DoDoor(line, close);
(*line).special = 0;
}
4 => {
EV_DoDoor(line, normal);
(*line).special = 0;
}
5 => {
EV_DoFloor(line, raiseFloor);
(*line).special = 0;
}
6 => {
EV_DoCeiling(line, fastCrushAndRaise);
(*line).special = 0;
}
8 => {
EV_BuildStairs(line, build8);
(*line).special = 0;
}
10 => {
EV_DoPlat(line, downWaitUpStay, 0);
(*line).special = 0;
}
12 => {
EV_LightTurnOn(line, 0);
(*line).special = 0;
}
13 => {
EV_LightTurnOn(line, 255);
(*line).special = 0;
}
16 => {
EV_DoDoor(line, close30ThenOpen);
(*line).special = 0;
}
17 => {
EV_StartLightStrobing(line);
(*line).special = 0;
}
19 => {
EV_DoFloor(line, lowerFloor);
(*line).special = 0;
}
22 => {
EV_DoPlat(line, raiseToNearestAndChange, 0);
(*line).special = 0;
}
25 => {
EV_DoCeiling(line, crushAndRaise);
(*line).special = 0;
}
30 => {
EV_DoFloor(line, raiseToTexture);
(*line).special = 0;
}
35 => {
EV_LightTurnOn(line, 35);
(*line).special = 0;
}
36 => {
EV_DoFloor(line, turboLower);
(*line).special = 0;
}
37 => {
EV_DoFloor(line, lowerAndChange);
(*line).special = 0;
}
38 => {
EV_DoFloor(line, lowerFloorToLowest);
(*line).special = 0;
}
39 => {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
(*line).special = 0;
}
40 => {
EV_DoCeiling(line, raiseToHighest);
EV_DoFloor(line, lowerFloorToLowest);
(*line).special = 0;
}
44 => {
EV_DoCeiling(line, lowerAndCrush);
(*line).special = 0;
}
52 => {
G_ExitLevel();
}
53 => {
EV_DoPlat(line, perpetualRaise, 0);
(*line).special = 0;
}
54 => {
EV_StopPlat(line);
(*line).special = 0;
}
56 => {
EV_DoFloor(line, raiseFloorCrush);
(*line).special = 0;
}
57 => {
EV_CeilingCrushStop(line);
(*line).special = 0;
}
58 => {
EV_DoFloor(line, raiseFloor24);
(*line).special = 0;
}
59 => {
EV_DoFloor(line, raiseFloor24AndChange);
(*line).special = 0;
}
104 => {
EV_TurnTagLightsOff(line);
(*line).special = 0;
}
108 => {
EV_DoDoor(line, blazeRaise);
(*line).special = 0;
}
109 => {
EV_DoDoor(line, blazeOpen);
(*line).special = 0;
}
100 => {
EV_BuildStairs(line, turbo16);
(*line).special = 0;
}
110 => {
EV_DoDoor(line, blazeClose);
(*line).special = 0;
}
119 => {
EV_DoFloor(line, raiseFloorToNearest);
(*line).special = 0;
}
121 => {
EV_DoPlat(line, blazeDWUS, 0);
(*line).special = 0;
}
124 => {
G_SecretExitLevel();
}
125 => {
if (*thing).player.is_null() {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
(*line).special = 0;
}
}
130 => {
EV_DoFloor(line, raiseFloorTurbo);
(*line).special = 0;
}
141 => {
EV_DoCeiling(line, silentCrushAndRaise);
(*line).special = 0;
}
// RETRIGGERS
72 => {
EV_DoCeiling(line, lowerAndCrush);
}
73 => {
EV_DoCeiling(line, crushAndRaise);
}
74 => {
EV_CeilingCrushStop(line);
}
75 => {
EV_DoDoor(line, close);
}
76 => {
EV_DoDoor(line, close30ThenOpen);
}
77 => {
EV_DoCeiling(line, fastCrushAndRaise);
}
79 => {
EV_LightTurnOn(line, 35);
}
80 => {
EV_LightTurnOn(line, 0);
}
81 => {
EV_LightTurnOn(line, 255);
}
82 => {
EV_DoFloor(line, lowerFloorToLowest);
}
83 => {
EV_DoFloor(line, lowerFloor);
}
84 => {
EV_DoFloor(line, lowerAndChange);
}
86 => {
EV_DoDoor(line, open);
}
87 => {
EV_DoPlat(line, perpetualRaise, 0);
}
88 => {
EV_DoPlat(line, downWaitUpStay, 0);
}
89 => {
EV_StopPlat(line);
}
90 => {
EV_DoDoor(line, normal);
}
91 => {
EV_DoFloor(line, raiseFloor);
}
92 => {
EV_DoFloor(line, raiseFloor24);
}
93 => {
EV_DoFloor(line, raiseFloor24AndChange);
}
94 => {
EV_DoFloor(line, raiseFloorCrush);
}
95 => {
EV_DoPlat(line, raiseToNearestAndChange, 0);
}
96 => {
EV_DoFloor(line, raiseToTexture);
}
97 => {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
}
98 => {
EV_DoFloor(line, turboLower);
}
105 => {
EV_DoDoor(line, blazeRaise);
}
106 => {
EV_DoDoor(line, blazeOpen);
}
107 => {
EV_DoDoor(line, blazeClose);
}
120 => {
EV_DoPlat(line, blazeDWUS, 0);
}
126 => {
if (*thing).player.is_null() {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
}
}
128 => {
EV_DoFloor(line, raiseFloorToNearest);
}
129 => {
EV_DoFloor(line, raiseFloorTurbo);
}
_ => {}
}
}
}
// --- P_ShootSpecialLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ShootSpecialLine(thing: *mut mobj_t, line: *mut line_t) {
unsafe {
if (*thing).player.is_null() && (*line).special != 46 {
return;
}
match (*line).special {
24 => {
EV_DoFloor(line, raiseFloor);
P_ChangeSwitchTexture(line, 0);
}
46 => {
EV_DoDoor(line, open);
P_ChangeSwitchTexture(line, 1);
}
47 => {
EV_DoPlat(line, raiseToNearestAndChange, 0);
P_ChangeSwitchTexture(line, 0);
}
_ => {}
}
}
}
// --- P_PlayerInSpecialSector ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PlayerInSpecialSector(player: *mut player_t) {
unsafe {
let mo = (*player).mo as *mut mobj_t;
let sub = (*mo).subsector as *mut subsector_t;
let sector = (*sub).sector;
if (*mo).z != (*sector).floorheight {
return;
}
match (*sector).special {
5 => {
if (*player).powers[pw_ironfeet as usize] == 0 && (leveltime & 0x1f) == 0 {
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 10);
}
}
7 => {
if (*player).powers[pw_ironfeet as usize] == 0 && (leveltime & 0x1f) == 0 {
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 5);
}
}
16 | 4 => {
if ((*player).powers[pw_ironfeet as usize] == 0 || p_random() < 5)
&& (leveltime & 0x1f) == 0
{
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 20);
}
}
9 => {
(*player).secretcount += 1;
(*sector).special = 0;
}
11 => {
(*player).cheats &= !(CF_GODMODE_FLAG as c_int);
if (leveltime & 0x1f) == 0 {
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 20);
}
if (*player).health <= 10 {
G_ExitLevel();
}
}
other => {
doom_error(&format!("P_PlayerInSpecialSector: unknown special {}", other));
}
}
}
}
// --- P_UpdateSpecials ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UpdateSpecials() {
unsafe {
if levelTimer != 0 {
levelTimeCount -= 1;
if levelTimeCount == 0 {
G_ExitLevel();
}
}
let mut anim = (&raw mut anims) as *mut anim_t;
while anim < lastanim {
let base = (*anim).basepic;
let count = (*anim).numpics;
let speed = (*anim).speed;
for i in base..(base + count) {
let pic = base + ((leveltime / speed + i) % count);
if (*anim).istexture != 0 {
*texturetranslation.add(i as usize) = pic;
} else {
*flattranslation.add(i as usize) = pic;
}
}
anim = anim.add(1);
}
let frac_u = FRACUNIT as i32;
let spec_list = (&raw const linespeciallist) as *const *mut line_t;
for i in 0..(numlinespecials as usize) {
let line = *spec_list.add(i);
if (*line).special == 48 {
let s0 = *(&raw const (*line).sidenum as *const i16) as usize;
(*sides.add(s0)).textureoffset += frac_u;
}
}
let btn_list = (&raw mut buttonlist) as *mut button_t;
for i in 0..(MAXBUTTONS as usize) {
let btn = btn_list.add(i);
if (*btn).btimer != 0 {
(*btn).btimer -= 1;
if (*btn).btimer == 0 {
let s0 = *(&raw const (*(*btn).line).sidenum as *const i16) as usize;
let side_ptr = sides.add(s0);
let tex = (*btn).btexture as i16;
match (*btn).where_ {
BWHERE_TOP => (*side_ptr).toptexture = tex,
BWHERE_MIDDLE => (*side_ptr).midtexture = tex,
BWHERE_BOTTOM => (*side_ptr).bottomtexture = tex,
_ => {}
}
S_StartSound((&raw mut (*btn).soundorg) as *mut c_void, sfx_swtchn as c_int);
ptr::write_bytes(btn as *mut u8, 0, size_of::<button_t>());
}
}
}
}
}
// --- EV_DoDonut ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoDonut(line: *mut line_t) -> c_int {
unsafe {
let mut secnum = -1;
let mut rtn = 0;
loop {
secnum = P_FindSectorFromLineTag(line, secnum);
if secnum < 0 {
break;
}
let s1 = sectors.add(secnum as usize);
if !(*s1).specialdata.is_null() {
continue;
}
rtn = 1;
let first_line = *(*s1).lines;
let s2 = getNextSector(first_line, s1);
for i in 0..((*s2).linecount as usize) {
let ln = *(*s2).lines.add(i);
if (((*ln).flags as u32) & ML_TWOSIDED) == 0 || ((*ln).backsector as *mut sector_t) == s1 {
continue;
}
let s3 = (*ln).backsector as *mut sector_t;
// Подъем слизи
let floor1 = Z_Malloc(size_of::<floormove_t>() as c_int, PU_LEVSPEC as c_int, ptr::null_mut())
as *mut floormove_t;
P_AddThinker((&raw mut (*floor1).thinker) as *mut crate::p_tick::thinker_t);
(*s2).specialdata = floor1 as *mut c_void;
let move_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_MoveFloor as unsafe extern "C" fn(*mut floormove_t));
(*floor1).thinker.function.acp1 = Some(move_fn);
(*floor1).type_ = donutRaise;
(*floor1).crush = 0;
(*floor1).direction = 1;
(*floor1).sector = s2;
(*floor1).speed = (FLOORSPEED as i32) / 2;
(*floor1).texture = (*s3).floorpic;
(*floor1).newspecial = 0;
(*floor1).floordestheight = (*s3).floorheight;
// Опускание центрального кольца
let floor2 = Z_Malloc(size_of::<floormove_t>() as c_int, PU_LEVSPEC as c_int, ptr::null_mut())
as *mut floormove_t;
P_AddThinker((&raw mut (*floor2).thinker) as *mut crate::p_tick::thinker_t);
(*s1).specialdata = floor2 as *mut c_void;
(*floor2).thinker.function.acp1 = Some(move_fn);
(*floor2).type_ = lowerFloor;
(*floor2).crush = 0;
(*floor2).direction = -1;
(*floor2).sector = s1;
(*floor2).speed = (FLOORSPEED as i32) / 2;
(*floor2).floordestheight = (*s3).floorheight;
break;
}
}
rtn
}
}
// --- P_SpawnSpecials ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnSpecials() {
unsafe {
let mut _episode = 1;
if W_CheckNumForName(b"texture2\0".as_ptr() as *const c_char) >= 0 {
_episode = 2;
}
levelTimer = 0;
let i_avg = M_CheckParm(b"-avg\0".as_ptr() as *const c_char);
if i_avg != 0 && deathmatch != 0 {
levelTimer = 1;
levelTimeCount = 20 * 60 * 35;
}
let i_timer = M_CheckParm(b"-timer\0".as_ptr() as *const c_char);
if i_timer != 0 && deathmatch != 0 {
let arg_ptr = *myargv.add((i_timer + 1) as usize);
let mins = CStr::from_ptr(arg_ptr)
.to_str()
.ok()
.and_then(|s| s.trim().parse::<c_int>().ok())
.unwrap_or(0);
levelTimer = 1;
levelTimeCount = mins * 60 * 35;
}
for i in 0..numsectors {
let sector = sectors.add(i as usize);
if (*sector).special == 0 {
continue;
}
match (*sector).special {
1 => P_SpawnLightFlash(sector),
2 => P_SpawnStrobeFlash(sector, FASTDARK as c_int, 0),
3 => P_SpawnStrobeFlash(sector, SLOWDARK as c_int, 0),
4 => {
P_SpawnStrobeFlash(sector, FASTDARK as c_int, 0);
(*sector).special = 4;
}
8 => P_SpawnGlowingLight(sector),
9 => totalsecret += 1,
10 => P_SpawnDoorCloseIn30(sector),
12 => P_SpawnStrobeFlash(sector, SLOWDARK as c_int, 1),
13 => P_SpawnStrobeFlash(sector, FASTDARK as c_int, 1),
14 => P_SpawnDoorRaiseIn5Mins(sector, i),
17 => P_SpawnFireFlicker(sector),
_ => {}
}
}
numlinespecials = 0;
let spec_list = (&raw mut linespeciallist) as *mut *mut line_t;
for i in 0..(numlines as usize) {
let ln = lines.add(i);
if (*ln).special == 48 {
*spec_list.add(numlinespecials as usize) = ln;
numlinespecials += 1;
}
}
let ceil_ptr = (&raw mut activeceilings) as *mut *mut ceiling_t;
for i in 0..(MAXCEILINGS as usize) {
*ceil_ptr.add(i) = ptr::null_mut();
}
let plat_ptr = (&raw mut activeplats) as *mut *mut plat_t;
for i in 0..(MAXPLATS as usize) {
*plat_ptr.add(i) = ptr::null_mut();
}
let btn_list = (&raw mut buttonlist) as *mut button_t;
for i in 0..(MAXBUTTONS as usize) {
ptr::write_bytes(btn_list.add(i) as *mut u8, 0, size_of::<button_t>());
}
}
}