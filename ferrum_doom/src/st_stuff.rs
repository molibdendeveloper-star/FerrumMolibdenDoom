#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

use crate::m_cheat::{cht_CheckCheat, cht_GetParam};
use crate::m_random::m_random;
use crate::p_inter::P_GivePower;
const AM_MSGHEADER: u32 = ((b'a' as u32) << 24) + ((b'm' as u32) << 16);
const AM_MSGENTERED: u32 = AM_MSGHEADER | ((b'e' as u32) << 8);
const AM_MSGEXITED: u32 = AM_MSGHEADER | ((b'x' as u32) << 8);
const CF_NOCLIP: c_int = 1;
const CF_GODMODE: c_int = 2;
use crate::r_main::R_PointToAngle2;
use crate::v_video::{V_CopyRect, V_DrawPatch, screens};
use crate::w_wad::{W_CacheLumpName, W_CacheLumpNum, W_GetNumForName};
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_ChangeTag2 as Z_ChangeTag, Z_Malloc};

const TICRATE: c_int = 35;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const NUM_WEAPONS: usize = NUMWEAPONS as usize;
const NUM_AMMO: usize = NUMAMMO as usize;
const NUM_CARDS: usize = NUMCARDS as usize;
const NUM_POWERS: usize = NUMPOWERS as usize;

const BG: c_int = 4;
const FG: c_int = 0;

// Палитры
const STARTREDPALS: c_int = 1;
const STARTBONUSPALS: c_int = 9;
const NUMREDPALS: c_int = 8;
const NUMBONUSPALS: c_int = 4;
const RADIATIONPAL: c_int = 13;

// Лицо морпеха
const ST_NUMPAINFACES: usize = 5;
const ST_NUMSTRAIGHTFACES: usize = 3;
const ST_NUMTURNFACES: usize = 2;
const ST_NUMSPECIALFACES: usize = 3;
const ST_FACESTRIDE: usize = ST_NUMSTRAIGHTFACES + ST_NUMTURNFACES + ST_NUMSPECIALFACES;
const ST_NUMEXTRAFACES: usize = 2;
const ST_NUMFACES: usize = ST_FACESTRIDE * ST_NUMPAINFACES + ST_NUMEXTRAFACES;

const ST_TURNOFFSET: c_int = ST_NUMSTRAIGHTFACES as c_int;
const ST_OUCHOFFSET: c_int = ST_TURNOFFSET + (ST_NUMTURNFACES as c_int);
const ST_EVILGRINOFFSET: c_int = ST_OUCHOFFSET + 1;
const ST_RAMPAGEOFFSET: c_int = ST_EVILGRINOFFSET + 1;
const ST_GODFACE: c_int = (ST_NUMPAINFACES * ST_FACESTRIDE) as c_int;
const ST_DEADFACE: c_int = ST_GODFACE + 1;

const ST_FACESX: c_int = 143;
const ST_FACESY: c_int = 168;
const ST_EVILGRINCOUNT: c_int = 2 * TICRATE;
const ST_STRAIGHTFACECOUNT: c_int = TICRATE / 2;
const ST_TURNCOUNT: c_int = 1 * TICRATE;
const ST_RAMPAGEDELAY: c_int = 2 * TICRATE;
const ST_MUCHPAIN: c_int = 20;

// Координаты виджетов
const ST_X: c_int = 0;
const ST_FX: c_int = 143;

const ST_AMMOWIDTH: c_int = 3;
const ST_AMMOX: c_int = 44;
const ST_AMMOY: c_int = 171;

const ST_HEALTHWIDTH: c_int = 3;
const ST_HEALTHX: c_int = 90;
const ST_HEALTHY: c_int = 171;

const ST_ARMSX: c_int = 111;
const ST_ARMSY: c_int = 172;
const ST_ARMSBGX: c_int = 104;
const ST_ARMSBGY: c_int = 168;
const ST_ARMSXSPACE: c_int = 12;
const ST_ARMSYSPACE: c_int = 10;

const ST_FRAGSX: c_int = 138;
const ST_FRAGSY: c_int = 171;
const ST_FRAGSWIDTH: c_int = 2;

const ST_ARMORWIDTH: c_int = 3;
const ST_ARMORX: c_int = 221;
const ST_ARMORY: c_int = 171;

const ST_KEY0X: c_int = 239;
const ST_KEY0Y: c_int = 171;
const ST_KEY1X: c_int = 239;
const ST_KEY1Y: c_int = 181;
const ST_KEY2X: c_int = 239;
const ST_KEY2Y: c_int = 191;

const ST_AMMO0WIDTH: c_int = 3;
const ST_AMMO0X: c_int = 288;
const ST_AMMO0Y: c_int = 173;
const ST_AMMO1WIDTH: c_int = 3;
const ST_AMMO1X: c_int = 288;
const ST_AMMO1Y: c_int = 179;
const ST_AMMO2WIDTH: c_int = 3;
const ST_AMMO2X: c_int = 288;
const ST_AMMO2Y: c_int = 191;
const ST_AMMO3WIDTH: c_int = 3;
const ST_AMMO3X: c_int = 288;
const ST_AMMO3Y: c_int = 185;

const ST_MAXAMMO0WIDTH: c_int = 3;
const ST_MAXAMMO0X: c_int = 314;
const ST_MAXAMMO0Y: c_int = 173;
const ST_MAXAMMO1WIDTH: c_int = 3;
const ST_MAXAMMO1X: c_int = 314;
const ST_MAXAMMO1Y: c_int = 179;
const ST_MAXAMMO2WIDTH: c_int = 3;
const ST_MAXAMMO2X: c_int = 314;
const ST_MAXAMMO2Y: c_int = 191;
const ST_MAXAMMO3WIDTH: c_int = 3;
const ST_MAXAMMO3X: c_int = 314;
const ST_MAXAMMO3Y: c_int = 185;

const mus_e1m1: c_int = 1;
const mus_runnin: c_int = 33;

unsafe extern "C" {
    fn S_ChangeMusic(musicnum: c_int, looping: c_int);
    fn G_DeferedInitNew(skill: skill_t, episode: c_int, map: c_int);
    fn I_SetPalette(palette: *mut u8);

    fn STlib_init();
    fn STlib_initNum(
        n: *mut st_number_t,
        x: c_int,
        y: c_int,
        pl: *mut *mut patch_t,
        num: *mut c_int,
        on: *mut c_int,
        width: c_int,
    );
    fn STlib_updateNum(n: *mut st_number_t, refresh: c_int);
    fn STlib_initPercent(
        p: *mut st_percent_t,
        x: c_int,
        y: c_int,
        pl: *mut *mut patch_t,
        num: *mut c_int,
        on: *mut c_int,
        percent: *mut patch_t,
    );
    fn STlib_updatePercent(per: *mut st_percent_t, refresh: c_int);
    fn STlib_initMultIcon(
        mi: *mut st_multicon_t,
        x: c_int,
        y: c_int,
        il: *mut *mut patch_t,
        inum: *mut c_int,
        on: *mut c_int,
    );
    fn STlib_updateMultIcon(mi: *mut st_multicon_t, refresh: c_int);
    fn STlib_initBinIcon(
        b: *mut st_binicon_t,
        x: c_int,
        y: c_int,
        i: *mut patch_t,
        val: *mut c_int,
        on: *mut c_int,
    );
    fn STlib_updateBinIcon(bi: *mut st_binicon_t, refresh: c_int);

    static mut netgame: c_int;
    static mut deathmatch: c_int;
    static mut gamemode: GameMode_t;
    static mut gameskill: skill_t;
    static mut consoleplayer: c_int;
    static mut players: [player_t; NUM_PLAYERS];
    static mut automapactive: c_int;
    static mut weaponinfo: [weaponinfo_t; NUM_WEAPONS];
}

// --- Состояние статус-бара ---
static mut plyr: *mut player_t = ptr::null_mut();
static mut st_firsttime: c_int = 0;
static mut veryfirsttime: c_int = 1;
static mut lu_palette: c_int = 0;
static mut st_clock: c_uint = 0;
static mut st_msgcounter: c_int = 0;
static mut st_chatstate: st_chatstateenum_t = StartChatState;
static mut st_gamestate: st_stateenum_t = FirstPersonState;
static mut st_statusbaron: c_int = 0;
static mut st_chat: c_int = 0;
static mut st_oldchat: c_int = 0;
static mut st_cursoron: c_int = 0;
static mut st_notdeathmatch: c_int = 0;
static mut st_armson: c_int = 0;
static mut st_fragson: c_int = 0;

// --- Патчи ---
static mut sbar: *mut patch_t = ptr::null_mut();
static mut tallnum: [*mut patch_t; 10] = [ptr::null_mut(); 10];
static mut tallpercent: *mut patch_t = ptr::null_mut();
static mut shortnum: [*mut patch_t; 10] = [ptr::null_mut(); 10];
static mut keys: [*mut patch_t; NUM_CARDS] = [ptr::null_mut(); NUM_CARDS];
static mut faces: [*mut patch_t; ST_NUMFACES] = [ptr::null_mut(); ST_NUMFACES];
static mut faceback: *mut patch_t = ptr::null_mut();
static mut armsbg: *mut patch_t = ptr::null_mut();
static mut arms: [[*mut patch_t; 2]; 6] = [[ptr::null_mut(); 2]; 6];

// --- Виджеты ---
static mut w_ready: st_number_t = unsafe { std::mem::zeroed() };
static mut w_frags: st_number_t = unsafe { std::mem::zeroed() };
static mut w_health: st_percent_t = unsafe { std::mem::zeroed() };
static mut w_armsbg: st_binicon_t = unsafe { std::mem::zeroed() };
static mut w_arms: [st_multicon_t; 6] = unsafe { std::mem::zeroed() };
static mut w_faces: st_multicon_t = unsafe { std::mem::zeroed() };
static mut w_keyboxes: [st_multicon_t; 3] = unsafe { std::mem::zeroed() };
static mut w_armor: st_percent_t = unsafe { std::mem::zeroed() };
static mut w_ammo: [st_number_t; 4] = unsafe { std::mem::zeroed() };
static mut w_maxammo: [st_number_t; 4] = unsafe { std::mem::zeroed() };

static mut st_fragscount: c_int = 0;
static mut st_oldhealth: c_int = -1;
static mut oldweaponsowned: [c_int; NUM_WEAPONS] = [0; NUM_WEAPONS];
static mut st_facecount: c_int = 0;
static mut st_faceindex: c_int = 0;
static mut keyboxes: [c_int; 3] = [0; 3];
static mut st_randomnumber: c_int = 0;

// --- Последовательности чит-кодов ---
#[unsafe(no_mangle)]
pub static mut cheat_mus_seq: [u8; 9] = [0xb2, 0x26, 0xb6, 0xae, 0xea, 1, 0, 0, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_choppers_seq: [u8; 11] = [0xb2, 0x26, 0xe2, 0x32, 0xf6, 0x2a, 0x2a, 0xa6, 0x6a, 0xea, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_god_seq: [u8; 6] = [0xb2, 0x26, 0x26, 0xaa, 0x26, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_ammo_seq: [u8; 6] = [0xb2, 0x26, 0xf2, 0x66, 0xa2, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_ammonokey_seq: [u8; 5] = [0xb2, 0x26, 0x66, 0xa2, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_noclip_seq: [u8; 11] = [0xb2, 0x26, 0xea, 0x2a, 0xb2, 0xea, 0x2a, 0xf6, 0x2a, 0x26, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_commercial_noclip_seq: [u8; 7] = [0xb2, 0x26, 0xe2, 0x36, 0xb2, 0x2a, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_powerup_seq: [[u8; 10]; 7] = [
    [0xb2, 0x26, 0x62, 0xa6, 0x32, 0xf6, 0x36, 0x26, 0x6e, 0xff],
    [0xb2, 0x26, 0x62, 0xa6, 0x32, 0xf6, 0x36, 0x26, 0xea, 0xff],
    [0xb2, 0x26, 0x62, 0xa6, 0x32, 0xf6, 0x36, 0x26, 0xb2, 0xff],
    [0xb2, 0x26, 0x62, 0xa6, 0x32, 0xf6, 0x36, 0x26, 0x6a, 0xff],
    [0xb2, 0x26, 0x62, 0xa6, 0x32, 0xf6, 0x36, 0x26, 0xa2, 0xff],
    [0xb2, 0x26, 0x62, 0xa6, 0x32, 0xf6, 0x36, 0x26, 0x36, 0xff],
    [0xb2, 0x26, 0x62, 0xa6, 0x32, 0xf6, 0x36, 0x26, 0xff, 0],
];

#[unsafe(no_mangle)]
pub static mut cheat_clev_seq: [u8; 10] = [0xb2, 0x26, 0xe2, 0x36, 0xa6, 0x6e, 1, 0, 0, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_mypos_seq: [u8; 8] = [0xb2, 0x26, 0xb6, 0xba, 0x2a, 0xf6, 0xea, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_mus: cheatseq_t = cheatseq_t {
    sequence: unsafe { (&raw mut cheat_mus_seq) as *mut u8 },
    p: ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub static mut cheat_god: cheatseq_t = cheatseq_t {
    sequence: unsafe { (&raw mut cheat_god_seq) as *mut u8 },
    p: ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub static mut cheat_ammo: cheatseq_t = cheatseq_t {
    sequence: unsafe { (&raw mut cheat_ammo_seq) as *mut u8 },
    p: ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub static mut cheat_ammonokey: cheatseq_t = cheatseq_t {
    sequence: unsafe { (&raw mut cheat_ammonokey_seq) as *mut u8 },
    p: ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub static mut cheat_noclip: cheatseq_t = cheatseq_t {
    sequence: unsafe { (&raw mut cheat_noclip_seq) as *mut u8 },
    p: ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub static mut cheat_commercial_noclip: cheatseq_t = cheatseq_t {
sequence: unsafe { (&raw mut cheat_commercial_noclip_seq) as *mut u8 },
p: ptr::null_mut(),
};
#[unsafe(no_mangle)]
pub static mut cheat_powerup: [cheatseq_t; 7] = [
cheatseq_t { sequence: unsafe { (&raw mut cheat_powerup_seq) as *mut u8 }, p: ptr::null_mut() },
cheatseq_t { sequence: unsafe { (&raw mut cheat_powerup_seq) as *mut u8 }, p: ptr::null_mut() },
cheatseq_t { sequence: unsafe { (&raw mut cheat_powerup_seq) as *mut u8 }, p: ptr::null_mut() },
cheatseq_t { sequence: unsafe { (&raw mut cheat_powerup_seq) as *mut u8 }, p: ptr::null_mut() },
cheatseq_t { sequence: unsafe { (&raw mut cheat_powerup_seq) as *mut u8 }, p: ptr::null_mut() },
cheatseq_t { sequence: unsafe { (&raw mut cheat_powerup_seq) as *mut u8 }, p: ptr::null_mut() },
cheatseq_t { sequence: unsafe { (&raw mut cheat_powerup_seq) as *mut u8 }, p: ptr::null_mut() },
];
#[unsafe(no_mangle)]
pub static mut cheat_choppers: cheatseq_t = cheatseq_t {
sequence: unsafe { (&raw mut cheat_choppers_seq) as *mut u8 },
p: ptr::null_mut(),
};
#[unsafe(no_mangle)]
pub static mut cheat_clev: cheatseq_t = cheatseq_t {
sequence: unsafe { (&raw mut cheat_clev_seq) as *mut u8 },
p: ptr::null_mut(),
};
#[unsafe(no_mangle)]
pub static mut cheat_mypos: cheatseq_t = cheatseq_t {
sequence: unsafe { (&raw mut cheat_mypos_seq) as *mut u8 },
p: ptr::null_mut(),
};
// --- ST_refreshBackground ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_refreshBackground() {
unsafe {
if st_statusbaron != 0 {
V_DrawPatch(ST_X, 0, BG, sbar as *mut _);
if netgame != 0 {
V_DrawPatch(ST_FX, 0, BG, faceback as *mut _);
}
V_CopyRect(
ST_X,
0,
BG,
ST_WIDTH as c_int,
ST_HEIGHT as c_int,
ST_X,
ST_Y as c_int,
FG,
);
}
}
}
// --- ST_Responder (Читы и автокарта) ---
static mut MYPOS_BUF: [c_char; 52] = [0; 52];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Responder(ev: *mut event_t) -> c_int {
unsafe {
let ev_type = (*ev).type_;
let data1 = (*ev).data1 as u32;
if ev_type == ev_keyup && (data1 & 0xffff_0000) == AM_MSGHEADER {
if data1 == AM_MSGENTERED {
st_gamestate = AutomapState;
st_firsttime = 1;
} else if data1 == AM_MSGEXITED {
st_gamestate = FirstPersonState;
}
} else if ev_type == ev_keydown {
let key = (*ev).data1 as c_char;
if netgame == 0 {
if cht_CheckCheat((&raw mut cheat_god) as *mut _, key) != 0 {
(*plyr).cheats ^= CF_GODMODE as c_int;
if ((*plyr).cheats & (CF_GODMODE as c_int)) != 0 {
if !(*plyr).mo.is_null() {
(*(*plyr).mo).health = 100;
}
(*plyr).health = 100;
(*plyr).message = STSTR_DQDON.as_ptr() as *mut c_char;
} else {
(*plyr).message = STSTR_DQDOFF.as_ptr() as *mut c_char;
}
} else if cht_CheckCheat((&raw mut cheat_ammonokey) as *mut _, key) != 0 {
(*plyr).armorpoints = 200;
(*plyr).armortype = 2;
for i in 0..NUM_WEAPONS {
(*plyr).weaponowned[i] = 1;
}
for i in 0..NUM_AMMO {
(*plyr).ammo[i] = (*plyr).maxammo[i];
}
(*plyr).message = STSTR_FAADDED.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_ammo) as *mut _, key) != 0 {
(*plyr).armorpoints = 200;
(*plyr).armortype = 2;
for i in 0..NUM_WEAPONS {
(*plyr).weaponowned[i] = 1;
}
for i in 0..NUM_AMMO {
(*plyr).ammo[i] = (*plyr).maxammo[i];
}
for i in 0..NUM_CARDS {
(*plyr).cards[i] = 1;
}
(*plyr).message = STSTR_KFAADDED.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_mus) as *mut _, key) != 0 {
let mut buf = [0 as c_char; 3];
(*plyr).message = STSTR_MUS.as_ptr() as *mut c_char;
cht_GetParam((&raw mut cheat_mus) as *mut _, buf.as_mut_ptr());
let d0 = (buf[0] as u8).wrapping_sub(b'0') as c_int;
let d1 = (buf[1] as u8).wrapping_sub(b'0') as c_int;
if gamemode == commercial {
let musnum = mus_runnin + d0 * 10 + d1 - 1;
if d0 * 10 + d1 > 35 {
(*plyr).message = STSTR_NOMUS.as_ptr() as *mut c_char;
} else {
S_ChangeMusic(musnum, 1);
}
} else {
let e0 = (buf[0] as u8).wrapping_sub(b'1') as c_int;
let e1 = (buf[1] as u8).wrapping_sub(b'1') as c_int;
let musnum = mus_e1m1 + e0 * 9 + e1;
if e0 * 9 + e1 > 31 {
(*plyr).message = STSTR_NOMUS.as_ptr() as *mut c_char;
} else {
S_ChangeMusic(musnum, 1);
}
}
} else if cht_CheckCheat((&raw mut cheat_noclip) as *mut _, key) != 0
|| cht_CheckCheat((&raw mut cheat_commercial_noclip) as *mut _, key) != 0
{
(*plyr).cheats ^= CF_NOCLIP as c_int;
if ((*plyr).cheats & (CF_NOCLIP as c_int)) != 0 {
(*plyr).message = STSTR_NCON.as_ptr() as *mut c_char;
} else {
(*plyr).message = STSTR_NCOFF.as_ptr() as *mut c_char;
}
}
let pwr_ptr = (&raw mut cheat_powerup) as *mut cheatseq_t;
for i in 0..6usize {
if cht_CheckCheat(pwr_ptr.add(i) as *mut _, key) != 0 {
if (*plyr).powers[i] == 0 {
P_GivePower(plyr as *mut _, i as c_int);
} else if i != (pw_strength as usize) {
(*plyr).powers[i] = 1;
} else {
(*plyr).powers[i] = 0;
}
(*plyr).message = STSTR_BEHOLDX.as_ptr() as *mut c_char;
}
}
if cht_CheckCheat(pwr_ptr.add(6) as *mut _, key) != 0 {
(*plyr).message = STSTR_BEHOLD.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_choppers) as *mut _, key) != 0 {
(*plyr).weaponowned[wp_chainsaw as usize] = 1;
(*plyr).powers[pw_invulnerability as usize] = 1;
(*plyr).message = STSTR_CHOPPERS.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_mypos) as *mut _, key) != 0 {
let players_base = (&raw const players) as *const player_t;
let mo = (*players_base.add(consoleplayer as usize)).mo;
let s = format!(
"ang=0x{:x};x,y=(0x{:x},0x{:x})\0",
(*mo).angle, (*mo).x, (*mo).y
);
let dst = (&raw mut MYPOS_BUF) as *mut c_char;
ptr::write_bytes(dst, 0, 52);
let len = s.len().min(51);
ptr::copy_nonoverlapping(s.as_ptr() as *const c_char, dst, len);
(*plyr).message = dst;
}
}
if cht_CheckCheat((&raw mut cheat_clev) as *mut _, key) != 0 {
let mut buf = [0 as c_char; 3];
cht_GetParam((&raw mut cheat_clev) as *mut _, buf.as_mut_ptr());
let (epsd, map) = if gamemode == commercial {
(
0,
((buf[0] as u8).wrapping_sub(b'0') as c_int) * 10
+ ((buf[1] as u8).wrapping_sub(b'0') as c_int),
)
} else {
(
(*buf.as_ptr() as u8).wrapping_sub(b'0') as c_int,
(*buf.as_ptr().add(1) as u8).wrapping_sub(b'0') as c_int,
)
};
if epsd < 1 || map < 1 {
return 0;
}
if gamemode == retail && (epsd > 4 || map > 9) {
return 0;
}
if gamemode == registered && (epsd > 3 || map > 9) {
return 0;
}
if gamemode == shareware && (epsd > 1 || map > 9) {
return 0;
}
if gamemode == commercial && (epsd > 1 || map > 34) {
return 0;
}
(*plyr).message = STSTR_CLEV.as_ptr() as *mut c_char;
G_DeferedInitNew(gameskill, epsd, map);
}
}
0
}
}
// --- Лицо морпеха ---
static mut LAST_CALC: c_int = 0;
static mut FACE_OLDHEALTH: c_int = -1;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_calcPainOffset() -> c_int {
unsafe {
let health = if (*plyr).health > 100 { 100 } else { (*plyr).health };
if health != FACE_OLDHEALTH {
LAST_CALC = (ST_FACESTRIDE as c_int) * (((100 - health) * (ST_NUMPAINFACES as c_int)) / 101);
FACE_OLDHEALTH = health;
}
LAST_CALC
}
}
static mut LAST_ATTACK_DOWN: c_int = -1;
static mut FACE_PRIORITY: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_updateFaceWidget() {
unsafe {
if FACE_PRIORITY < 10 && (*plyr).health == 0 {
FACE_PRIORITY = 9;
st_faceindex = ST_DEADFACE;
st_facecount = 1;
}
if FACE_PRIORITY < 9 && (*plyr).bonuscount != 0 {
let mut doevilgrin = false;
let old_w_ptr = (&raw mut oldweaponsowned) as *mut c_int;
for i in 0..NUM_WEAPONS {
if *old_w_ptr.add(i) != (*plyr).weaponowned[i] {
doevilgrin = true;
*old_w_ptr.add(i) = (*plyr).weaponowned[i];
}
}
if doevilgrin {
FACE_PRIORITY = 8;
st_facecount = ST_EVILGRINCOUNT;
st_faceindex = ST_calcPainOffset() + ST_EVILGRINOFFSET;
}
}
if FACE_PRIORITY < 8 {
if (*plyr).damagecount != 0 && !(*plyr).attacker.is_null() && (*plyr).attacker != (*plyr).mo {
FACE_PRIORITY = 7;
if (*plyr).health - st_oldhealth > ST_MUCHPAIN {
st_facecount = ST_TURNCOUNT;
st_faceindex = ST_calcPainOffset() + ST_OUCHOFFSET;
} else {
let mo = (*plyr).mo;
let att = (*plyr).attacker;
let badguyangle = R_PointToAngle2((*mo).x, (*mo).y, (*att).x, (*att).y);
let (diffang, turn_right) = if badguyangle > (*mo).angle {
let d = badguyangle.wrapping_sub((*mo).angle);
(d, d > ANG180)
} else {
let d = (*mo).angle.wrapping_sub(badguyangle);
(d, d <= ANG180)
};
st_facecount = ST_TURNCOUNT;
st_faceindex = ST_calcPainOffset();
if diffang < ANG45 {
st_faceindex += ST_RAMPAGEOFFSET;
} else if turn_right {
st_faceindex += ST_TURNOFFSET;
} else {
st_faceindex += ST_TURNOFFSET + 1;
}
}
}
}
if FACE_PRIORITY < 7 && (*plyr).damagecount != 0 {
if (*plyr).health - st_oldhealth > ST_MUCHPAIN {
FACE_PRIORITY = 7;
st_facecount = ST_TURNCOUNT;
st_faceindex = ST_calcPainOffset() + ST_OUCHOFFSET;
} else {
FACE_PRIORITY = 6;
st_facecount = ST_TURNCOUNT;
st_faceindex = ST_calcPainOffset() + ST_RAMPAGEOFFSET;
}
}
if FACE_PRIORITY < 6 {
if (*plyr).attackdown != 0 {
if LAST_ATTACK_DOWN == -1 {
LAST_ATTACK_DOWN = ST_RAMPAGEDELAY;
} else {
LAST_ATTACK_DOWN -= 1;
if LAST_ATTACK_DOWN == 0 {
FACE_PRIORITY = 5;
st_faceindex = ST_calcPainOffset() + ST_RAMPAGEOFFSET;
st_facecount = 1;
LAST_ATTACK_DOWN = 1;
}
}
} else {
LAST_ATTACK_DOWN = -1;
}
}
if FACE_PRIORITY < 5 {
if ((*plyr).cheats & (CF_GODMODE as c_int)) != 0
|| (*plyr).powers[pw_invulnerability as usize] != 0
{
FACE_PRIORITY = 4;
st_faceindex = ST_GODFACE;
st_facecount = 1;
}
}
if st_facecount == 0 {
st_faceindex = ST_calcPainOffset() + (st_randomnumber % 3);
st_facecount = ST_STRAIGHTFACECOUNT;
FACE_PRIORITY = 0;
}
st_facecount -= 1;
}
}
// --- Обновление виджетов и тик ---
static mut LARGEAMMO: c_int = 1994;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_updateWidgets() {
unsafe {
let w_info_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let ready = (*plyr).readyweapon as usize;
let ammo_type = (*w_info_ptr.add(ready)).ammo;
if ammo_type == am_noammo {
w_ready.num = &raw mut LARGEAMMO;
} else {
w_ready.num = (&raw mut (*plyr).ammo) as *mut c_int;
w_ready.num = w_ready.num.add(ammo_type as usize);
}
w_ready.data = (*plyr).readyweapon as c_int;
let kb_ptr = (&raw mut keyboxes) as *mut c_int;
for i in 0..3usize {
*kb_ptr.add(i) = if (*plyr).cards[i] != 0 { i as c_int } else { -1 };
if (*plyr).cards[i + 3] != 0 {
*kb_ptr.add(i) = (i + 3) as c_int;
}
}
ST_updateFaceWidget();
st_notdeathmatch = (deathmatch == 0) as c_int;
st_armson = (st_statusbaron != 0 && deathmatch == 0) as c_int;
st_fragson = (deathmatch != 0 && st_statusbaron != 0) as c_int;
st_fragscount = 0;
for i in 0..NUM_PLAYERS {
if (i as c_int) != consoleplayer {
st_fragscount += (*plyr).frags[i];
} else {
st_fragscount -= (*plyr).frags[i];
}
}
st_msgcounter -= 1;
if st_msgcounter == 0 {
st_chat = st_oldchat;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Ticker() {
unsafe {
st_clock = st_clock.wrapping_add(1);
st_randomnumber = m_random();
ST_updateWidgets();
st_oldhealth = (*plyr).health;
}
}
// --- Эффекты палитры (боль, бонусы, радиация) ---
static mut st_palette: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_doPaletteStuff() {
unsafe {
let mut cnt = (*plyr).damagecount;
if (*plyr).powers[pw_strength as usize] != 0 {
let bzc = 12 - ((*plyr).powers[pw_strength as usize] >> 6);
if bzc > cnt {
cnt = bzc;
}
}
let palette = if cnt != 0 {
let mut p = (cnt + 7) >> 3;
if p >= NUMREDPALS {
p = NUMREDPALS - 1;
}
p + STARTREDPALS
} else if (*plyr).bonuscount != 0 {
let mut p = ((*plyr).bonuscount + 7) >> 3;
if p >= NUMBONUSPALS {
p = NUMBONUSPALS - 1;
}
p + STARTBONUSPALS
} else if (*plyr).powers[pw_ironfeet as usize] > 4 * 32
|| ((*plyr).powers[pw_ironfeet as usize] & 8) != 0
{
RADIATIONPAL
} else {
0
};
if palette != st_palette {
st_palette = palette;
let base = W_CacheLumpNum(lu_palette, PU_CACHE) as *mut u8;
let pal = base.add((palette as usize) * 768);
I_SetPalette(pal);
}
}
}
// --- Отрисовка виджетов и статус-бара ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_drawWidgets(refresh: c_int) {
unsafe {
st_armson = (st_statusbaron != 0 && deathmatch == 0) as c_int;
st_fragson = (deathmatch != 0 && st_statusbaron != 0) as c_int;
STlib_updateNum(&raw mut w_ready, refresh);
let ammo_w_ptr = (&raw mut w_ammo) as *mut st_number_t;
let maxammo_w_ptr = (&raw mut w_maxammo) as *mut st_number_t;
for i in 0..4usize {
STlib_updateNum(ammo_w_ptr.add(i), refresh);
STlib_updateNum(maxammo_w_ptr.add(i), refresh);
}
STlib_updatePercent(&raw mut w_health, refresh);
STlib_updatePercent(&raw mut w_armor, refresh);
STlib_updateBinIcon(&raw mut w_armsbg, refresh);
let arms_w_ptr = (&raw mut w_arms) as *mut st_multicon_t;
for i in 0..6usize {
STlib_updateMultIcon(arms_w_ptr.add(i), refresh);
}
STlib_updateMultIcon(&raw mut w_faces, refresh);
let kb_w_ptr = (&raw mut w_keyboxes) as *mut st_multicon_t;
for i in 0..3usize {
STlib_updateMultIcon(kb_w_ptr.add(i), refresh);
}
STlib_updateNum(&raw mut w_frags, refresh);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_doRefresh() {
unsafe {
st_firsttime = 0;
ST_refreshBackground();
ST_drawWidgets(1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_diffDraw() {
unsafe {
ST_drawWidgets(0);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Drawer(fullscreen: c_int, refresh: c_int) {
unsafe {
st_statusbaron = (fullscreen == 0 || automapactive != 0) as c_int;
st_firsttime = (st_firsttime != 0 || refresh != 0) as c_int;
ST_doPaletteStuff();
if st_firsttime != 0 {
ST_doRefresh();
} else {
ST_diffDraw();
}
}
}
// --- Загрузка и выгрузка графики ---
unsafe fn cache_static(name: &[u8]) -> *mut patch_t {
unsafe { W_CacheLumpName(name.as_ptr() as *mut c_char, PU_STATIC) as *mut patch_t }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_loadGraphics() {
unsafe {
let tall_ptr = (&raw mut tallnum) as *mut *mut patch_t;
let short_ptr = (&raw mut shortnum) as *mut *mut patch_t;
for i in 0..10usize {
let t_name = format!("STTNUM{}\0", i);
*tall_ptr.add(i) = cache_static(t_name.as_bytes());
let s_name = format!("STYSNUM{}\0", i);
*short_ptr.add(i) = cache_static(s_name.as_bytes());
}
tallpercent = cache_static(b"STTPRCNT\0");
let keys_ptr = (&raw mut keys) as *mut *mut patch_t;
for i in 0..NUM_CARDS {
let k_name = format!("STKEYS{}\0", i);
*keys_ptr.add(i) = cache_static(k_name.as_bytes());
}
armsbg = cache_static(b"STARMS\0");
let arms_ptr = (&raw mut arms) as *mut [*mut patch_t; 2];
for i in 0..6usize {
let g_name = format!("STGNUM{}\0", i + 2);
(*arms_ptr.add(i))[0] = cache_static(g_name.as_bytes());
(*arms_ptr.add(i))[1] = *short_ptr.add(i + 2);
}
let fb_name = format!("STFB{}\0", consoleplayer);
faceback = cache_static(fb_name.as_bytes());
sbar = cache_static(b"STBAR\0");
let faces_ptr = (&raw mut faces) as *mut *mut patch_t;
let mut facenum = 0usize;
for i in 0..ST_NUMPAINFACES {
for j in 0..ST_NUMSTRAIGHTFACES {
let name = format!("STFST{}{}\0", i, j);
*faces_ptr.add(facenum) = cache_static(name.as_bytes());
facenum += 1;
}
let r_name = format!("STFTR{}0\0", i);
*faces_ptr.add(facenum) = cache_static(r_name.as_bytes());
facenum += 1;
let l_name = format!("STFTL{}0\0", i);
*faces_ptr.add(facenum) = cache_static(l_name.as_bytes());
facenum += 1;
let o_name = format!("STFOUCH{}\0", i);
*faces_ptr.add(facenum) = cache_static(o_name.as_bytes());
facenum += 1;
let e_name = format!("STFEVL{}\0", i);
*faces_ptr.add(facenum) = cache_static(e_name.as_bytes());
facenum += 1;
let k_name = format!("STFKILL{}\0", i);
*faces_ptr.add(facenum) = cache_static(k_name.as_bytes());
facenum += 1;
}
*faces_ptr.add(facenum) = cache_static(b"STFGOD0\0");
facenum += 1;
*faces_ptr.add(facenum) = cache_static(b"STFDEAD0\0");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_loadData() {
unsafe {
lu_palette = W_GetNumForName(b"PLAYPAL\0".as_ptr() as *mut c_char);
ST_loadGraphics();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_unloadGraphics() {
unsafe {
let tall_ptr = (&raw const tallnum) as *const *mut patch_t;
let short_ptr = (&raw const shortnum) as *const *mut patch_t;
for i in 0..10usize {
Z_ChangeTag((*tall_ptr.add(i)) as *mut c_void, PU_CACHE);
Z_ChangeTag((*short_ptr.add(i)) as *mut c_void, PU_CACHE);
}
Z_ChangeTag(tallpercent as *mut c_void, PU_CACHE);
Z_ChangeTag(armsbg as *mut c_void, PU_CACHE);
let arms_ptr = (&raw const arms) as *const [*mut patch_t; 2];
for i in 0..6usize {
Z_ChangeTag((*arms_ptr.add(i))[0] as *mut c_void, PU_CACHE);
}
let keys_ptr = (&raw const keys) as *const *mut patch_t;
for i in 0..NUM_CARDS {
Z_ChangeTag((*keys_ptr.add(i)) as *mut c_void, PU_CACHE);
}
Z_ChangeTag(sbar as *mut c_void, PU_CACHE);
Z_ChangeTag(faceback as *mut c_void, PU_CACHE);
let faces_ptr = (&raw const faces) as *const *mut patch_t;
for i in 0..ST_NUMFACES {
Z_ChangeTag((*faces_ptr.add(i)) as *mut c_void, PU_CACHE);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_unloadData() {
unsafe {
ST_unloadGraphics();
}
}
// --- Инициализация и запуск статус-бара ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_initData() {
unsafe {
st_firsttime = 1;
let players_base = (&raw mut players) as *mut player_t;
plyr = players_base.add(consoleplayer as usize);
st_clock = 0;
st_chatstate = StartChatState;
st_gamestate = FirstPersonState;
st_statusbaron = 1;
st_chat = 0;
st_oldchat = 0;
st_cursoron = 0;
st_faceindex = 0;
st_palette = -1;
st_oldhealth = -1;
let old_w_ptr = (&raw mut oldweaponsowned) as *mut c_int;
for i in 0..NUM_WEAPONS {
*old_w_ptr.add(i) = (*plyr).weaponowned[i];
}
let kb_ptr = (&raw mut keyboxes) as *mut c_int;
for i in 0..3usize {
*kb_ptr.add(i) = -1;
}
STlib_init();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_createWidgets() {
unsafe {
let tall_ptr = (&raw mut tallnum) as *mut *mut patch_t;
let short_ptr = (&raw mut shortnum) as *mut *mut patch_t;
let w_info_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let ready = (*plyr).readyweapon as usize;
let ammo_type = (*w_info_ptr.add(ready)).ammo;
let ready_num_ptr = if ammo_type == am_noammo {
&raw mut LARGEAMMO
} else {
((&raw mut (*plyr).ammo) as *mut c_int).add(ammo_type as usize)
};
STlib_initNum(
&raw mut w_ready,
ST_AMMOX,
ST_AMMOY,
tall_ptr,
ready_num_ptr,
&raw mut st_statusbaron,
ST_AMMOWIDTH,
);
w_ready.data = (*plyr).readyweapon as c_int;
STlib_initPercent(
&raw mut w_health,
ST_HEALTHX,
ST_HEALTHY,
tall_ptr,
&raw mut (*plyr).health,
&raw mut st_statusbaron,
tallpercent,
);
STlib_initBinIcon(
&raw mut w_armsbg,
ST_ARMSBGX,
ST_ARMSBGY,
armsbg,
&raw mut st_notdeathmatch,
&raw mut st_statusbaron,
);
let arms_w_ptr = (&raw mut w_arms) as *mut st_multicon_t;
let arms_p_ptr = (&raw mut arms) as *mut [*mut patch_t; 2];
let wowned_base = (&raw mut (*plyr).weaponowned) as *mut c_int;
for i in 0..6usize {
STlib_initMultIcon(
arms_w_ptr.add(i),
ST_ARMSX + ((i as c_int) % 3) * ST_ARMSXSPACE,
ST_ARMSY + ((i as c_int) / 3) * ST_ARMSYSPACE,
(*arms_p_ptr.add(i)).as_mut_ptr(),
wowned_base.add(i + 1),
&raw mut st_armson,
);
}
STlib_initNum(
&raw mut w_frags,
ST_FRAGSX,
ST_FRAGSY,
tall_ptr,
&raw mut st_fragscount,
&raw mut st_fragson,
ST_FRAGSWIDTH,
);
let faces_ptr = (&raw mut faces) as *mut *mut patch_t;
STlib_initMultIcon(
&raw mut w_faces,
ST_FACESX,
ST_FACESY,
faces_ptr,
&raw mut st_faceindex,
&raw mut st_statusbaron,
);
STlib_initPercent(
&raw mut w_armor,
ST_ARMORX,
ST_ARMORY,
tall_ptr,
&raw mut (*plyr).armorpoints,
&raw mut st_statusbaron,
tallpercent,
);
let keys_ptr = (&raw mut keys) as *mut *mut patch_t;
let kb_w_ptr = (&raw mut w_keyboxes) as *mut st_multicon_t;
let kb_ptr = (&raw mut keyboxes) as *mut c_int;
STlib_initMultIcon(kb_w_ptr.add(0), ST_KEY0X, ST_KEY0Y, keys_ptr, kb_ptr.add(0), &raw mut st_statusbaron);
STlib_initMultIcon(kb_w_ptr.add(1), ST_KEY1X, ST_KEY1Y, keys_ptr, kb_ptr.add(1), &raw mut st_statusbaron);
STlib_initMultIcon(kb_w_ptr.add(2), ST_KEY2X, ST_KEY2Y, keys_ptr, kb_ptr.add(2), &raw mut st_statusbaron);
let ammo_w_ptr = (&raw mut w_ammo) as *mut st_number_t;
let ammo_base = (&raw mut (*plyr).ammo) as *mut c_int;
STlib_initNum(ammo_w_ptr.add(0), ST_AMMO0X, ST_AMMO0Y, short_ptr, ammo_base.add(0), &raw mut st_statusbaron, ST_AMMO0WIDTH);
STlib_initNum(ammo_w_ptr.add(1), ST_AMMO1X, ST_AMMO1Y, short_ptr, ammo_base.add(1), &raw mut st_statusbaron, ST_AMMO1WIDTH);
STlib_initNum(ammo_w_ptr.add(2), ST_AMMO2X, ST_AMMO2Y, short_ptr, ammo_base.add(2), &raw mut st_statusbaron, ST_AMMO2WIDTH);
STlib_initNum(ammo_w_ptr.add(3), ST_AMMO3X, ST_AMMO3Y, short_ptr, ammo_base.add(3), &raw mut st_statusbaron, ST_AMMO3WIDTH);
let maxammo_w_ptr = (&raw mut w_maxammo) as *mut st_number_t;
let maxammo_base = (&raw mut (*plyr).maxammo) as *mut c_int;
STlib_initNum(maxammo_w_ptr.add(0), ST_MAXAMMO0X, ST_MAXAMMO0Y, short_ptr, maxammo_base.add(0), &raw mut st_statusbaron, ST_MAXAMMO0WIDTH);
STlib_initNum(maxammo_w_ptr.add(1), ST_MAXAMMO1X, ST_MAXAMMO1Y, short_ptr, maxammo_base.add(1), &raw mut st_statusbaron, ST_MAXAMMO1WIDTH);
STlib_initNum(maxammo_w_ptr.add(2), ST_MAXAMMO2X, ST_MAXAMMO2Y, short_ptr, maxammo_base.add(2), &raw mut st_statusbaron, ST_MAXAMMO2WIDTH);
STlib_initNum(maxammo_w_ptr.add(3), ST_MAXAMMO3X, ST_MAXAMMO3Y, short_ptr, maxammo_base.add(3), &raw mut st_statusbaron, ST_MAXAMMO3WIDTH);
}
}
static mut st_stopped: c_int = 1;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Start() {
unsafe {
if st_stopped == 0 {
ST_Stop();
}
ST_initData();
ST_createWidgets();
st_stopped = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Stop() {
unsafe {
if st_stopped != 0 {
return;
}
let pal = W_CacheLumpNum(lu_palette, PU_CACHE) as *mut u8;
I_SetPalette(pal);
st_stopped = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Init() {
unsafe {
veryfirsttime = 0;
ST_loadData();
let scr_ptr = (&raw mut screens) as *mut *mut u8;
*scr_ptr.add(4) = Z_Malloc(
(ST_WIDTH as c_int) * (ST_HEIGHT as c_int),
PU_STATIC,
ptr::null_mut(),
) as *mut u8;
}
}