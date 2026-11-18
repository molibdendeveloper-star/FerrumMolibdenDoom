#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::m_argv::{get_arg, m_check_parm};
use crate::m_random::p_random;
use crate::p_switch::{clear_buttons, update_buttons};
use crate::p_telept::EV_Teleport;
use crate::p_tick::{P_AddThinker, get_leveltime};
use crate::r_bsp::subsector_t;
use crate::r_data::{
    R_CheckTextureNumForName, R_FlatNumForName, R_TextureNumForName, flattranslation,
    texturetranslation,
};
use crate::w_wad::w_check_num_for_name;
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

pub const MAXANIMS: usize = 32;
pub const MAXLINEANIMS: usize = 64;
const MAX_ADJOINING_SECTORS: usize = 20;
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
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Spec error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

pub static ANIMDEFS: [animdef_t; 22] = [
    make_anim(0, b"NUKAGE3\0\0", b"NUKAGE1\0\0", 8),
    make_anim(0, b"FWATER4\0\0", b"FWATER1\0\0", 8),
    make_anim(0, b"SWATER4\0\0", b"SWATER1\0\0", 8),
    make_anim(0, b"LAVA4\0\0\0\0", b"LAVA1\0\0\0\0", 8),
    make_anim(0, b"BLOOD3\0\0\0", b"BLOOD1\0\0\0", 8),
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
];

static ANIMS: OnceLock<Mutex<Vec<anim_t>>> = OnceLock::new();

fn anims_list() -> &'static Mutex<Vec<anim_t>> {
    ANIMS.get_or_init(|| Mutex::new(Vec::with_capacity(MAXANIMS)))
}

#[unsafe(no_mangle)]
pub static mut levelTimer: c_int = 0;

#[unsafe(no_mangle)]
pub static mut levelTimeCount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut numlinespecials: c_short = 0;

#[unsafe(no_mangle)]
pub static mut linespeciallist: [*mut line_t; MAXLINEANIMS] = [ptr::null_mut(); MAXLINEANIMS];

fn anim_name_str(arr: &[c_char; 9]) -> &str {
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(arr.as_ptr() as *const u8, 9) };
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(8);
    std::str::from_utf8(&bytes[..len]).unwrap_or("?")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InitPicAnims() {
    let mut list = anims_list().lock().unwrap();
    list.clear();

    for def in &ANIMDEFS {
        let start_ptr = def.startname.as_ptr() as *mut c_char;
        let end_ptr = def.endname.as_ptr() as *mut c_char;

        let (picnum, basepic) = if def.istexture != 0 {
            if unsafe { R_CheckTextureNumForName(start_ptr) } == -1 {
                continue;
            }
            unsafe { (R_TextureNumForName(end_ptr), R_TextureNumForName(start_ptr)) }
        } else {
            if w_check_num_for_name(anim_name_str(&def.startname)) == -1 {
                continue;
            }
            unsafe { (R_FlatNumForName(end_ptr), R_FlatNumForName(start_ptr)) }
        };

        let numpics = picnum - basepic + 1;
        if numpics < 2 {
            doom_error(&format!(
                "P_InitPicAnims: bad cycle from {} to {}",
                anim_name_str(&def.startname),
                anim_name_str(&def.endname)
            ));
        }

        list.push(anim_t {
            istexture: def.istexture,
            picnum,
            basepic,
            numpics,
            speed: def.speed,
        });
    }
}

#[inline]
pub fn get_next_sector(line: &line_t, sec: &sector_t) -> Option<&'static mut sector_t> {
    if (line.flags as u32 & ML_TWOSIDED) == 0 {
        return None;
    }
    let sec_ptr = (sec as *const sector_t) as *mut sector_t;
    let target = if (line.frontsector as *mut sector_t) == sec_ptr {
        line.backsector as *mut sector_t
    } else {
        line.frontsector as *mut sector_t
    };
    unsafe { target.as_mut() }
}

#[inline]
unsafe fn sector_lines(sec: &sector_t) -> &'static [*mut line_t] {
    unsafe { std::slice::from_raw_parts(sec.lines, sec.linecount.max(0) as usize) }
}

pub fn p_find_lowest_floor_surrounding(sec: &sector_t) -> i32 {
    let mut floor = sec.floorheight;
    for &ln_ptr in unsafe { sector_lines(sec) } {
        if let Some(ln) = unsafe { ln_ptr.as_ref() } {
            if let Some(other) = get_next_sector(ln, sec) {
                floor = floor.min(other.floorheight);
            }
        }
    }
    floor
}

pub fn p_find_highest_floor_surrounding(sec: &sector_t) -> i32 {
    let mut floor = -500 * (FRACUNIT as i32);
    for &ln_ptr in unsafe { sector_lines(sec) } {
        if let Some(ln) = unsafe { ln_ptr.as_ref() } {
            if let Some(other) = get_next_sector(ln, sec) {
                floor = floor.max(other.floorheight);
            }
        }
    }
    floor
}

pub fn p_find_next_highest_floor(sec: &sector_t, currentheight: c_int) -> i32 {
    let mut heightlist = [0i32; MAX_ADJOINING_SECTORS];
    let mut h = 0usize;

    for &ln_ptr in unsafe { sector_lines(sec) } {
        let Some(ln) = (unsafe { ln_ptr.as_ref() }) else { continue };
        let Some(other) = get_next_sector(ln, sec) else { continue };

        if other.floorheight > currentheight {
            heightlist[h] = other.floorheight;
            h += 1;
            if h >= MAX_ADJOINING_SECTORS {
                eprintln!("Sector with more than 20 adjoining sectors");
                break;
            }
        }
    }

    if h == 0 {
        return currentheight;
    }

    *heightlist[..h].iter().min().unwrap_or(&currentheight)
}

pub fn p_find_lowest_ceiling_surrounding(sec: &sector_t) -> i32 {
    let mut height = i32::MAX;
    for &ln_ptr in unsafe { sector_lines(sec) } {
        if let Some(ln) = unsafe { ln_ptr.as_ref() } {
            if let Some(other) = get_next_sector(ln, sec) {
                height = height.min(other.ceilingheight);
            }
        }
    }
    height
}

pub fn p_find_highest_ceiling_surrounding(sec: &sector_t) -> i32 {
    let mut height = 0i32;
    for &ln_ptr in unsafe { sector_lines(sec) } {
        if let Some(ln) = unsafe { ln_ptr.as_ref() } {
            if let Some(other) = get_next_sector(ln, sec) {
                height = height.max(other.ceilingheight);
            }
        }
    }
    height
}

pub fn p_find_min_surrounding_light(sec: &sector_t, max: c_int) -> c_int {
    let mut min = max;
    for &ln_ptr in unsafe { sector_lines(sec) } {
        if let Some(ln) = unsafe { ln_ptr.as_ref() } {
            if let Some(other) = get_next_sector(ln, sec) {
                min = min.min(other.lightlevel as c_int);
            }
        }
    }
    min
}

// --- Совместимые FFI-обёртки ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getSide(
current_sector: c_int,
line: c_int,
side: c_int,
) -> *mut side_t {
unsafe {
        let sec = sectors.add(current_sector as usize).as_ref().unwrap();
        let ln = (*sec.lines.add(line as usize)).as_ref().unwrap();
let snum = ln.sidenum[side as usize];
sides.add(snum as usize)
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getSector(
current_sector: c_int,
line: c_int,
side: c_int,
) -> *mut sector_t {
unsafe { (*getSide(current_sector, line, side)).sector }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn twoSided(sector: c_int, line: c_int) -> c_int {
unsafe {
        let sec = sectors.add(sector as usize).as_ref().unwrap();
        let ln = (*sec.lines.add(line as usize)).as_ref().unwrap();
((ln.flags as u32) & ML_TWOSIDED) as c_int
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getNextSector(line: *mut line_t, sec: *mut sector_t) -> *mut sector_t {
match (unsafe { line.as_ref() }, unsafe { sec.as_ref() }) {
(Some(ln), Some(s)) => get_next_sector(ln, s)
.map(|r| r as *mut sector_t)
.unwrap_or(ptr::null_mut()),
_ => ptr::null_mut(),
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindLowestFloorSurrounding(sec: *mut sector_t) -> i32 {
match unsafe { sec.as_ref() } {
Some(s) => p_find_lowest_floor_surrounding(s),
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindHighestFloorSurrounding(sec: *mut sector_t) -> i32 {
match unsafe { sec.as_ref() } {
Some(s) => p_find_highest_floor_surrounding(s),
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindNextHighestFloor(sec: *mut sector_t, currentheight: c_int) -> i32 {
match unsafe { sec.as_ref() } {
Some(s) => p_find_next_highest_floor(s, currentheight),
None => currentheight,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindLowestCeilingSurrounding(sec: *mut sector_t) -> i32 {
match unsafe { sec.as_ref() } {
Some(s) => p_find_lowest_ceiling_surrounding(s),
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindHighestCeilingSurrounding(sec: *mut sector_t) -> i32 {
match unsafe { sec.as_ref() } {
Some(s) => p_find_highest_ceiling_surrounding(s),
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindSectorFromLineTag(line: *mut line_t, start: c_int) -> c_int {
let Some(ln) = (unsafe { line.as_ref() }) else { return -1 };
let tag = ln.tag;
let secs = unsafe { std::slice::from_raw_parts(sectors, numsectors.max(0) as usize) };
let begin = (start + 1).max(0) as usize;
for (offset, sec) in secs.get(begin..).unwrap_or(&[]).iter().enumerate() {
if sec.tag == tag {
return (begin + offset) as c_int;
}
}
-1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FindMinSurroundingLight(sector: *mut sector_t, max: c_int) -> c_int {
match unsafe { sector.as_ref() } {
Some(s) => p_find_min_surrounding_light(s, max),
None => max,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CrossSpecialLine(linenum: c_int, side: c_int, thing: *mut mobj_t) {
unsafe {
let line = lines.add(linenum as usize);
let ln = &mut *line;
let th = &*thing;
if th.player.is_null() {
match th.type_ {
MT_ROCKET | MT_PLASMA | MT_BFG | MT_TROOPSHOT | MT_HEADSHOT | MT_BRUISERSHOT => {
return;
}
_ => {}
}
if !matches!(ln.special, 39 | 97 | 125 | 126 | 4 | 10 | 88) {
return;
}
}
match ln.special {
2 => { EV_DoDoor(line, open); ln.special = 0; }
3 => { EV_DoDoor(line, close); ln.special = 0; }
4 => { EV_DoDoor(line, normal); ln.special = 0; }
5 => { EV_DoFloor(line, raiseFloor); ln.special = 0; }
6 => { EV_DoCeiling(line, fastCrushAndRaise); ln.special = 0; }
8 => { EV_BuildStairs(line, build8); ln.special = 0; }
10 => { EV_DoPlat(line, downWaitUpStay, 0); ln.special = 0; }
12 => { EV_LightTurnOn(line, 0); ln.special = 0; }
13 => { EV_LightTurnOn(line, 255); ln.special = 0; }
16 => { EV_DoDoor(line, close30ThenOpen); ln.special = 0; }
17 => { EV_StartLightStrobing(line); ln.special = 0; }
19 => { EV_DoFloor(line, lowerFloor); ln.special = 0; }
22 => { EV_DoPlat(line, raiseToNearestAndChange, 0); ln.special = 0; }
25 => { EV_DoCeiling(line, crushAndRaise); ln.special = 0; }
30 => { EV_DoFloor(line, raiseToTexture); ln.special = 0; }
35 => { EV_LightTurnOn(line, 35); ln.special = 0; }
36 => { EV_DoFloor(line, turboLower); ln.special = 0; }
37 => { EV_DoFloor(line, lowerAndChange); ln.special = 0; }
38 => { EV_DoFloor(line, lowerFloorToLowest); ln.special = 0; }
39 => {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
ln.special = 0;
}
40 => {
EV_DoCeiling(line, raiseToHighest);
EV_DoFloor(line, lowerFloorToLowest);
ln.special = 0;
}
44 => { EV_DoCeiling(line, lowerAndCrush); ln.special = 0; }
52 => { G_ExitLevel(); }
53 => { EV_DoPlat(line, perpetualRaise, 0); ln.special = 0; }
54 => { EV_StopPlat(line); ln.special = 0; }
56 => { EV_DoFloor(line, raiseFloorCrush); ln.special = 0; }
57 => { EV_CeilingCrushStop(line); ln.special = 0; }
58 => { EV_DoFloor(line, raiseFloor24); ln.special = 0; }
59 => { EV_DoFloor(line, raiseFloor24AndChange); ln.special = 0; }
104 => { EV_TurnTagLightsOff(line); ln.special = 0; }
108 => { EV_DoDoor(line, blazeRaise); ln.special = 0; }
109 => { EV_DoDoor(line, blazeOpen); ln.special = 0; }
100 => { EV_BuildStairs(line, turbo16); ln.special = 0; }
110 => { EV_DoDoor(line, blazeClose); ln.special = 0; }
119 => { EV_DoFloor(line, raiseFloorToNearest); ln.special = 0; }
121 => { EV_DoPlat(line, blazeDWUS, 0); ln.special = 0; }
124 => { G_SecretExitLevel(); }
125 => {
if th.player.is_null() {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
ln.special = 0;
}
}
130 => { EV_DoFloor(line, raiseFloorTurbo); ln.special = 0; }
141 => { EV_DoCeiling(line, silentCrushAndRaise); ln.special = 0; }
// RETRIGGERS
72 => { EV_DoCeiling(line, lowerAndCrush); }
73 => { EV_DoCeiling(line, crushAndRaise); }
74 => { EV_CeilingCrushStop(line); }
75 => { EV_DoDoor(line, close); }
76 => { EV_DoDoor(line, close30ThenOpen); }
77 => { EV_DoCeiling(line, fastCrushAndRaise); }
79 => { EV_LightTurnOn(line, 35); }
80 => { EV_LightTurnOn(line, 0); }
81 => { EV_LightTurnOn(line, 255); }
82 => { EV_DoFloor(line, lowerFloorToLowest); }
83 => { EV_DoFloor(line, lowerFloor); }
84 => { EV_DoFloor(line, lowerAndChange); }
86 => { EV_DoDoor(line, open); }
87 => { EV_DoPlat(line, perpetualRaise, 0); }
88 => { EV_DoPlat(line, downWaitUpStay, 0); }
89 => { EV_StopPlat(line); }
90 => { EV_DoDoor(line, normal); }
91 => { EV_DoFloor(line, raiseFloor); }
92 => { EV_DoFloor(line, raiseFloor24); }
93 => { EV_DoFloor(line, raiseFloor24AndChange); }
94 => { EV_DoFloor(line, raiseFloorCrush); }
95 => { EV_DoPlat(line, raiseToNearestAndChange, 0); }
96 => { EV_DoFloor(line, raiseToTexture); }
97 => {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
}
98 => { EV_DoFloor(line, turboLower); }
105 => { EV_DoDoor(line, blazeRaise); }
106 => { EV_DoDoor(line, blazeOpen); }
107 => { EV_DoDoor(line, blazeClose); }
120 => { EV_DoPlat(line, blazeDWUS, 0); }
126 => {
if th.player.is_null() {
EV_Teleport(line as *mut crate::p_telept::line_t, side, thing as *mut crate::p_telept::mobj_t);
}
}
128 => { EV_DoFloor(line, raiseFloorToNearest); }
129 => { EV_DoFloor(line, raiseFloorTurbo); }
_ => {}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ShootSpecialLine(thing: *mut mobj_t, line: *mut line_t) {
let (Some(th), Some(ln)) = (unsafe { thing.as_ref() }, unsafe { line.as_ref() }) else {
return;
};
if th.player.is_null() && ln.special != 46 {
return;
}
unsafe {
match ln.special {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PlayerInSpecialSector(player: *mut player_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
unsafe {
let mo = p.mo as *mut mobj_t;
let sub = ((*mo).subsector as *const subsector_t).as_ref().unwrap();
let sector = sub.sector.as_mut().unwrap();
if (*mo).z != sector.floorheight {
return;
}
let tics = get_leveltime();
match sector.special {
5 => {
if p.powers[pw_ironfeet as usize] == 0 && (tics & 0x1f) == 0 {
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 10);
}
}
7 => {
if p.powers[pw_ironfeet as usize] == 0 && (tics & 0x1f) == 0 {
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 5);
}
}
16 | 4 => {
if (p.powers[pw_ironfeet as usize] == 0 || p_random() < 5) && (tics & 0x1f) == 0 {
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 20);
}
}
9 => {
p.secretcount += 1;
sector.special = 0;
}
11 => {
p.cheats &= !(CF_GODMODE_FLAG as c_int);
if (tics & 0x1f) == 0 {
P_DamageMobj(mo, ptr::null_mut(), ptr::null_mut(), 20);
}
if p.health <= 10 {
G_ExitLevel();
}
}
other => {
doom_error(&format!("P_PlayerInSpecialSector: unknown special {}", other));
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UpdateSpecials() {
unsafe {
if levelTimer != 0 {
levelTimeCount -= 1;
if levelTimeCount == 0 {
G_ExitLevel();
}
}
let tics = get_leveltime();
let list = anims_list().lock().unwrap();
for anim in list.iter() {
for i in anim.basepic..(anim.basepic + anim.numpics) {
let pic = anim.basepic + ((tics / anim.speed + i) % anim.numpics);
if anim.istexture != 0 {
*texturetranslation.add(i as usize) = pic;
} else {
*flattranslation.add(i as usize) = pic;
}
}
}
let frac_u = FRACUNIT as i32;
let spec_list = (&raw const linespeciallist).as_ref().unwrap();
for &line_ptr in &spec_list[..(numlinespecials.max(0) as usize)] {
let ln = &*line_ptr;
if ln.special == 48 {
let s0 = ln.sidenum[0] as usize;
(*sides.add(s0)).textureoffset += frac_u;
}
}
update_buttons();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_DoDonut(line: *mut line_t) -> c_int {
unsafe {
let mut secnum = -1;
let mut rtn = 0;
let move_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(T_MoveFloor as unsafe extern "C" fn(*mut floormove_t));
loop {
secnum = P_FindSectorFromLineTag(line, secnum);
if secnum < 0 {
break;
}
let s1 = &mut *sectors.add(secnum as usize);
if !s1.specialdata.is_null() {
continue;
}
rtn = 1;
let first_line = &*(*s1.lines);
let Some(s2) = get_next_sector(first_line, s1) else { continue };
for &ln_ptr in sector_lines(s2) {
let ln = &*ln_ptr;
if ((ln.flags as u32) & ML_TWOSIDED) == 0
|| (ln.backsector as *mut sector_t) == (s1 as *mut sector_t)
{
continue;
}
let s3 = &*(ln.backsector as *const sector_t);
let floor1 = &mut *(Z_Malloc(
size_of::<floormove_t>() as c_int,
PU_LEVSPEC,
ptr::null_mut(),
) as *mut floormove_t);
P_AddThinker((&raw mut floor1.thinker) as *mut crate::p_tick::thinker_t);
s2.specialdata = (floor1 as *mut floormove_t) as *mut c_void;
floor1.thinker.function.acp1 = Some(move_fn);
floor1.type_ = donutRaise;
floor1.crush = 0;
floor1.direction = 1;
floor1.sector = s2;
floor1.speed = (FLOORSPEED as i32) / 2;
floor1.texture = s3.floorpic;
floor1.newspecial = 0;
floor1.floordestheight = s3.floorheight;
let floor2 = &mut *(Z_Malloc(
size_of::<floormove_t>() as c_int,
PU_LEVSPEC,
ptr::null_mut(),
) as *mut floormove_t);
P_AddThinker((&raw mut floor2.thinker) as *mut crate::p_tick::thinker_t);
s1.specialdata = (floor2 as *mut floormove_t) as *mut c_void;
floor2.thinker.function.acp1 = Some(move_fn);
floor2.type_ = lowerFloor;
floor2.crush = 0;
floor2.direction = -1;
floor2.sector = s1;
floor2.speed = (FLOORSPEED as i32) / 2;
floor2.floordestheight = s3.floorheight;
break;
}
}
rtn
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnSpecials() {
unsafe {
let mut _episode = 1;
if w_check_num_for_name("texture2") >= 0 {
_episode = 2;
}
levelTimer = 0;
let i_avg = m_check_parm("-avg");
if i_avg != 0 && deathmatch != 0 {
levelTimer = 1;
levelTimeCount = 20 * 60 * 35;
}
let i_timer = m_check_parm("-timer");
if i_timer != 0 && deathmatch != 0 {
let mins = get_arg((i_timer + 1) as usize)
.and_then(|s| s.trim().parse::<c_int>().ok())
.unwrap_or(0);
levelTimer = 1;
levelTimeCount = mins * 60 * 35;
}
let secs = std::slice::from_raw_parts_mut(sectors, numsectors.max(0) as usize);
for (i, sector) in secs.iter_mut().enumerate() {
if sector.special == 0 {
continue;
}
let sec_ptr = sector as *mut sector_t;
match sector.special {
1 => P_SpawnLightFlash(sec_ptr),
2 => P_SpawnStrobeFlash(sec_ptr, FASTDARK as c_int, 0),
3 => P_SpawnStrobeFlash(sec_ptr, SLOWDARK as c_int, 0),
4 => {
P_SpawnStrobeFlash(sec_ptr, FASTDARK as c_int, 0);
sector.special = 4;
}
8 => P_SpawnGlowingLight(sec_ptr),
9 => totalsecret += 1,
10 => P_SpawnDoorCloseIn30(sec_ptr),
12 => P_SpawnStrobeFlash(sec_ptr, SLOWDARK as c_int, 1),
13 => P_SpawnStrobeFlash(sec_ptr, FASTDARK as c_int, 1),
14 => P_SpawnDoorRaiseIn5Mins(sec_ptr, i as c_int),
17 => P_SpawnFireFlicker(sec_ptr),
_ => {}
}
}
numlinespecials = 0;
let spec_list = (&raw mut linespeciallist).as_mut().unwrap();
let lns = std::slice::from_raw_parts_mut(lines, numlines.max(0) as usize);
for ln in lns {
if ln.special == 48 {
spec_list[numlinespecials as usize] = ln;
numlinespecials += 1;
}
}
crate::p_ceilng::clear_active_ceilings();
crate::p_plats::clear_active_plats();
clear_buttons();
}
}
