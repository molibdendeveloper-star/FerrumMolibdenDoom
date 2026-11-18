#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

use crate::d_items::weaponinfo;

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::i_video::I_SetPalette;
use crate::m_cheat::{cht_CheckCheat, cht_GetParam};
use crate::m_random::m_random;
use crate::p_inter::P_GivePower;
use crate::r_main::R_PointToAngle2;
use crate::s_sound::S_ChangeMusic;
use crate::st_lib::{
    STlib_init, stlib_init_bin_icon, stlib_init_mult_icon, stlib_init_num, stlib_init_percent,
    stlib_update_bin_icon, stlib_update_mult_icon, stlib_update_num, stlib_update_percent,
};
use crate::v_video::{V_CopyRect, V_DrawPatch, screens};
use crate::w_wad::{W_CacheLumpName, W_CacheLumpNum, w_get_num_for_name};
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_ChangeTag2 as Z_ChangeTag, Z_Malloc};

const AM_MSGHEADER: u32 = ((b'a' as u32) << 24) + ((b'm' as u32) << 16);
const AM_MSGENTERED: u32 = AM_MSGHEADER | ((b'e' as u32) << 8);
const AM_MSGEXITED: u32 = AM_MSGHEADER | ((b'x' as u32) << 8);
const CF_NOCLIP: c_int = 1;
const CF_GODMODE: c_int = 2;

const TICRATE: c_int = 35;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const NUM_WEAPONS: usize = NUMWEAPONS as usize;
const NUM_AMMO: usize = NUMAMMO as usize;
const NUM_CARDS: usize = NUMCARDS as usize;

const BG: c_int = 4;
const FG: c_int = 0;

const STARTREDPALS: c_int = 1;
const STARTBONUSPALS: c_int = 9;
const NUMREDPALS: c_int = 8;
const NUMBONUSPALS: c_int = 4;
const RADIATIONPAL: c_int = 13;

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

const EMPTY_NUMBER: st_number_t = st_number_t {
    x: 0,
    y: 0,
    width: 0,
    oldnum: 0,
    num: ptr::null_mut(),
    on: ptr::null_mut(),
    p: ptr::null_mut(),
    data: 0,
};

const EMPTY_PERCENT: st_percent_t = st_percent_t {
    n: EMPTY_NUMBER,
    p: ptr::null_mut(),
};

const EMPTY_MULTICON: st_multicon_t = st_multicon_t {
    x: 0,
    y: 0,
    oldinum: 0,
    inum: ptr::null_mut(),
    on: ptr::null_mut(),
    p: ptr::null_mut(),
    data: 0,
};

const EMPTY_BINICON: st_binicon_t = st_binicon_t {
    x: 0,
    y: 0,
    oldval: 0,
    val: ptr::null_mut(),
    on: ptr::null_mut(),
    p: ptr::null_mut(),
    data: 0,
};

unsafe extern "C" {
    fn G_DeferedInitNew(skill: skill_t, episode: c_int, map: c_int);

    static mut netgame: c_int;
    static mut deathmatch: c_int;
    static mut gameskill: skill_t;
    static mut consoleplayer: c_int;
    static mut players: [player_t; NUM_PLAYERS];
    static mut automapactive: c_int;
}

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

static mut sbar: *mut patch_t = ptr::null_mut();
static mut tallnum: [*mut patch_t; 10] = [ptr::null_mut(); 10];
static mut tallpercent: *mut patch_t = ptr::null_mut();
static mut shortnum: [*mut patch_t; 10] = [ptr::null_mut(); 10];
static mut keys: [*mut patch_t; NUM_CARDS] = [ptr::null_mut(); NUM_CARDS];
static mut faces: [*mut patch_t; ST_NUMFACES] = [ptr::null_mut(); ST_NUMFACES];
static mut faceback: *mut patch_t = ptr::null_mut();
static mut armsbg: *mut patch_t = ptr::null_mut();
static mut arms: [[*mut patch_t; 2]; 6] = [[ptr::null_mut(); 2]; 6];

static mut w_ready: st_number_t = EMPTY_NUMBER;
static mut w_frags: st_number_t = EMPTY_NUMBER;
static mut w_health: st_percent_t = EMPTY_PERCENT;
static mut w_armsbg: st_binicon_t = EMPTY_BINICON;
static mut w_arms: [st_multicon_t; 6] = [EMPTY_MULTICON; 6];
static mut w_faces: st_multicon_t = EMPTY_MULTICON;
static mut w_keyboxes: [st_multicon_t; 3] = [EMPTY_MULTICON; 3];
static mut w_armor: st_percent_t = EMPTY_PERCENT;
static mut w_ammo: [st_number_t; 4] = [EMPTY_NUMBER; 4];
static mut w_maxammo: [st_number_t; 4] = [EMPTY_NUMBER; 4];

static mut st_fragscount: c_int = 0;
static mut st_oldhealth: c_int = -1;
static mut oldweaponsowned: [c_int; NUM_WEAPONS] = [0; NUM_WEAPONS];
static mut st_facecount: c_int = 0;
static mut st_faceindex: c_int = 0;
static mut keyboxes: [c_int; 3] = [0; 3];
static mut st_randomnumber: c_int = 0;

#[unsafe(no_mangle)]
pub static mut cheat_mus_seq: [u8; 9] = [0xb2, 0x26, 0xb6, 0xae, 0xea, 1, 0, 0, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_choppers_seq: [u8; 11] =
    [0xb2, 0x26, 0xe2, 0x32, 0xf6, 0x2a, 0x2a, 0xa6, 0x6a, 0xea, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_god_seq: [u8; 6] = [0xb2, 0x26, 0x26, 0xaa, 0x26, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_ammo_seq: [u8; 6] = [0xb2, 0x26, 0xf2, 0x66, 0xa2, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_ammonokey_seq: [u8; 5] = [0xb2, 0x26, 0x66, 0xa2, 0xff];

#[unsafe(no_mangle)]
pub static mut cheat_noclip_seq: [u8; 11] =
    [0xb2, 0x26, 0xea, 0x2a, 0xb2, 0xea, 0x2a, 0xf6, 0x2a, 0x26, 0xff];

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
static mut MYPOS_BUF: [c_char; 52] = [0; 52];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Responder(ev: *mut event_t) -> c_int {
let Some(ev_ref) = (unsafe { ev.as_ref() }) else { return 0 };
unsafe {
let ev_type = ev_ref.type_;
let data1 = ev_ref.data1 as u32;
if ev_type == ev_keyup && (data1 & 0xffff_0000) == AM_MSGHEADER {
if data1 == AM_MSGENTERED {
st_gamestate = AutomapState;
st_firsttime = 1;
} else if data1 == AM_MSGEXITED {
st_gamestate = FirstPersonState;
}
} else if ev_type == ev_keydown {
let key = ev_ref.data1 as c_char;
let p = &mut *plyr;
if netgame == 0 {
if cht_CheckCheat((&raw mut cheat_god) as *mut _, key) != 0 {
p.cheats ^= CF_GODMODE;
if (p.cheats & CF_GODMODE) != 0 {
if !p.mo.is_null() {
(*p.mo).health = 100;
}
p.health = 100;
p.message = STSTR_DQDON.as_ptr() as *mut c_char;
} else {
p.message = STSTR_DQDOFF.as_ptr() as *mut c_char;
}
} else if cht_CheckCheat((&raw mut cheat_ammonokey) as *mut _, key) != 0 {
p.armorpoints = 200;
p.armortype = 2;
p.weaponowned.fill(1);
p.ammo = p.maxammo;
p.message = STSTR_FAADDED.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_ammo) as *mut _, key) != 0 {
p.armorpoints = 200;
p.armortype = 2;
p.weaponowned.fill(1);
p.ammo = p.maxammo;
p.cards.fill(1);
p.message = STSTR_KFAADDED.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_mus) as *mut _, key) != 0 {
let mut buf = [0 as c_char; 3];
p.message = STSTR_MUS.as_ptr() as *mut c_char;
cht_GetParam((&raw mut cheat_mus) as *mut _, buf.as_mut_ptr());
let d0 = (buf[0] as u8).wrapping_sub(b'0') as c_int;
let d1 = (buf[1] as u8).wrapping_sub(b'0') as c_int;
if get_gamemode() == commercial {
let musnum = mus_runnin + d0 * 10 + d1 - 1;
if d0 * 10 + d1 > 35 {
p.message = STSTR_NOMUS.as_ptr() as *mut c_char;
} else {
S_ChangeMusic(musnum, 1);
}
} else {
let e0 = (buf[0] as u8).wrapping_sub(b'1') as c_int;
let e1 = (buf[1] as u8).wrapping_sub(b'1') as c_int;
let musnum = mus_e1m1 + e0 * 9 + e1;
if e0 * 9 + e1 > 31 {
p.message = STSTR_NOMUS.as_ptr() as *mut c_char;
} else {
S_ChangeMusic(musnum, 1);
}
}
} else if cht_CheckCheat((&raw mut cheat_noclip) as *mut _, key) != 0
|| cht_CheckCheat((&raw mut cheat_commercial_noclip) as *mut _, key) != 0
{
p.cheats ^= CF_NOCLIP;
p.message = if (p.cheats & CF_NOCLIP) != 0 {
STSTR_NCON.as_ptr() as *mut c_char
} else {
STSTR_NCOFF.as_ptr() as *mut c_char
};
}
let pwr_arr = (&raw mut cheat_powerup).as_mut().unwrap();
for i in 0..6usize {
if cht_CheckCheat(&mut pwr_arr[i] as *mut _ as *mut _, key) != 0 {
if p.powers[i] == 0 {
P_GivePower(plyr as *mut _, i as c_int);
} else if i != (pw_strength as usize) {
p.powers[i] = 1;
} else {
p.powers[i] = 0;
}
p.message = STSTR_BEHOLDX.as_ptr() as *mut c_char;
}
}
if cht_CheckCheat(&mut pwr_arr[6] as *mut _ as *mut _, key) != 0 {
p.message = STSTR_BEHOLD.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_choppers) as *mut _, key) != 0 {
p.weaponowned[wp_chainsaw as usize] = 1;
p.powers[pw_invulnerability as usize] = 1;
p.message = STSTR_CHOPPERS.as_ptr() as *mut c_char;
} else if cht_CheckCheat((&raw mut cheat_mypos) as *mut _, key) != 0 {
let players_arr = (&raw const players).as_ref().unwrap();
let mo = &*players_arr[consoleplayer as usize].mo;
let s = format!("ang=0x{:x};x,y=(0x{:x},0x{:x})\0", mo.angle, mo.x, mo.y);
let dst = (&raw mut MYPOS_BUF).as_mut().unwrap();
dst.fill(0);
for (d, &b) in dst.iter_mut().zip(s.as_bytes()).take(51) {
*d = b as c_char;
}
p.message = dst.as_mut_ptr();
}
}
if cht_CheckCheat((&raw mut cheat_clev) as *mut _, key) != 0 {
let mut buf = [0 as c_char; 3];
cht_GetParam((&raw mut cheat_clev) as *mut _, buf.as_mut_ptr());
let (epsd, map) = if get_gamemode() == commercial {
(
0,
((buf[0] as u8).wrapping_sub(b'0') as c_int) * 10
+ ((buf[1] as u8).wrapping_sub(b'0') as c_int),
)
} else {
(
(buf[0] as u8).wrapping_sub(b'0') as c_int,
(buf[1] as u8).wrapping_sub(b'0') as c_int,
)
};
if epsd < 1 || map < 1 {
return 0;
}
if get_gamemode() == retail && (epsd > 4 || map > 9) {
return 0;
}
if get_gamemode() == registered && (epsd > 3 || map > 9) {
return 0;
}
if get_gamemode() == shareware && (epsd > 1 || map > 9) {
return 0;
}
if get_gamemode() == commercial && (epsd > 1 || map > 34) {
return 0;
}
p.message = STSTR_CLEV.as_ptr() as *mut c_char;
G_DeferedInitNew(gameskill, epsd, map);
}
}
0
}
}
static LAST_CALC: AtomicI32 = AtomicI32::new(0);
static FACE_OLDHEALTH: AtomicI32 = AtomicI32::new(-1);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_calcPainOffset() -> c_int {
let health = unsafe { (*plyr).health.min(100) };
if health != FACE_OLDHEALTH.load(Ordering::Relaxed) {
let val = (ST_FACESTRIDE as c_int) * (((100 - health) * (ST_NUMPAINFACES as c_int)) / 101);
LAST_CALC.store(val, Ordering::Relaxed);
FACE_OLDHEALTH.store(health, Ordering::Relaxed);
}
LAST_CALC.load(Ordering::Relaxed)
}
static LAST_ATTACK_DOWN: AtomicI32 = AtomicI32::new(-1);
static FACE_PRIORITY: AtomicI32 = AtomicI32::new(0);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_updateFaceWidget() {
unsafe {
let p = &*plyr;
let mut prio = FACE_PRIORITY.load(Ordering::Relaxed);
if prio < 10 && p.health == 0 {
prio = 9;
st_faceindex = ST_DEADFACE;
st_facecount = 1;
}
if prio < 9 && p.bonuscount != 0 {
let mut doevilgrin = false;
let old_w = (&raw mut oldweaponsowned).as_mut().unwrap();
for i in 0..NUM_WEAPONS {
if old_w[i] != p.weaponowned[i] {
doevilgrin = true;
old_w[i] = p.weaponowned[i];
}
}
if doevilgrin {
prio = 8;
st_facecount = ST_EVILGRINCOUNT;
st_faceindex = ST_calcPainOffset() + ST_EVILGRINOFFSET;
}
}
if prio < 8 && p.damagecount != 0 && !p.attacker.is_null() && p.attacker != p.mo {
prio = 7;
if p.health - st_oldhealth > ST_MUCHPAIN {
st_facecount = ST_TURNCOUNT;
st_faceindex = ST_calcPainOffset() + ST_OUCHOFFSET;
} else {
let mo = &*p.mo;
let att = &*p.attacker;
let badguyangle = R_PointToAngle2(mo.x, mo.y, att.x, att.y);
let (diffang, turn_right) = if badguyangle > mo.angle {
let d = badguyangle.wrapping_sub(mo.angle);
(d, d > ANG180)
} else {
let d = mo.angle.wrapping_sub(badguyangle);
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
if prio < 7 && p.damagecount != 0 {
if p.health - st_oldhealth > ST_MUCHPAIN {
prio = 7;
st_facecount = ST_TURNCOUNT;
st_faceindex = ST_calcPainOffset() + ST_OUCHOFFSET;
} else {
prio = 6;
st_facecount = ST_TURNCOUNT;
st_faceindex = ST_calcPainOffset() + ST_RAMPAGEOFFSET;
}
}
if prio < 6 {
if p.attackdown != 0 {
let lad = LAST_ATTACK_DOWN.load(Ordering::Relaxed);
if lad == -1 {
LAST_ATTACK_DOWN.store(ST_RAMPAGEDELAY, Ordering::Relaxed);
} else if lad - 1 == 0 {
prio = 5;
st_faceindex = ST_calcPainOffset() + ST_RAMPAGEOFFSET;
st_facecount = 1;
LAST_ATTACK_DOWN.store(1, Ordering::Relaxed);
} else {
LAST_ATTACK_DOWN.store(lad - 1, Ordering::Relaxed);
}
} else {
LAST_ATTACK_DOWN.store(-1, Ordering::Relaxed);
}
}
if prio < 5
&& ((p.cheats & CF_GODMODE) != 0 || p.powers[pw_invulnerability as usize] != 0)
{
prio = 4;
st_faceindex = ST_GODFACE;
st_facecount = 1;
}
if st_facecount == 0 {
st_faceindex = ST_calcPainOffset() + (st_randomnumber % 3);
st_facecount = ST_STRAIGHTFACECOUNT;
prio = 0;
}
FACE_PRIORITY.store(prio, Ordering::Relaxed);
st_facecount -= 1;
}
}
static mut LARGEAMMO: c_int = 1994;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_updateWidgets() {
unsafe {
let p = &mut *plyr;
let ready = p.readyweapon as usize;
let ammo_type = weaponinfo[ready].ammo;
w_ready.num = if ammo_type == am_noammo {
&raw mut LARGEAMMO
} else {
&mut p.ammo[ammo_type as usize] as *mut c_int
};
w_ready.data = p.readyweapon;
let kb = (&raw mut keyboxes).as_mut().unwrap();
for i in 0..3usize {
kb[i] = if p.cards[i] != 0 { i as c_int } else { -1 };
if p.cards[i + 3] != 0 {
kb[i] = (i + 3) as c_int;
}
}
ST_updateFaceWidget();
st_notdeathmatch = (deathmatch == 0) as c_int;
st_armson = (st_statusbaron != 0 && deathmatch == 0) as c_int;
st_fragson = (deathmatch != 0 && st_statusbaron != 0) as c_int;
st_fragscount = 0;
for i in 0..NUM_PLAYERS {
if (i as c_int) != consoleplayer {
st_fragscount += p.frags[i];
} else {
st_fragscount -= p.frags[i];
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
static ST_PALETTE: AtomicI32 = AtomicI32::new(0);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_doPaletteStuff() {
unsafe {
let p = &*plyr;
let mut cnt = p.damagecount;
if p.powers[pw_strength as usize] != 0 {
let bzc = 12 - (p.powers[pw_strength as usize] >> 6);
cnt = cnt.max(bzc);
}
let palette = if cnt != 0 {
((cnt + 7) >> 3).min(NUMREDPALS - 1) + STARTREDPALS
} else if p.bonuscount != 0 {
((p.bonuscount + 7) >> 3).min(NUMBONUSPALS - 1) + STARTBONUSPALS
} else if p.powers[pw_ironfeet as usize] > 4 * 32
|| (p.powers[pw_ironfeet as usize] & 8) != 0
{
RADIATIONPAL
} else {
0
};
if palette != ST_PALETTE.load(Ordering::Relaxed) {
ST_PALETTE.store(palette, Ordering::Relaxed);
let base = W_CacheLumpNum(lu_palette, PU_CACHE) as *mut u8;
I_SetPalette(base.add((palette as usize) * 768));
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_drawWidgets(refresh: c_int) {
let ref_bool = refresh != 0;
unsafe {
st_armson = (st_statusbaron != 0 && deathmatch == 0) as c_int;
st_fragson = (deathmatch != 0 && st_statusbaron != 0) as c_int;
stlib_update_num((&raw mut w_ready).as_mut().unwrap(), ref_bool);
let ammo_w = (&raw mut w_ammo).as_mut().unwrap();
let maxammo_w = (&raw mut w_maxammo).as_mut().unwrap();
for i in 0..4usize {
stlib_update_num(&mut ammo_w[i], ref_bool);
stlib_update_num(&mut maxammo_w[i], ref_bool);
}
stlib_update_percent((&raw mut w_health).as_mut().unwrap(), ref_bool);
stlib_update_percent((&raw mut w_armor).as_mut().unwrap(), ref_bool);
stlib_update_bin_icon((&raw mut w_armsbg).as_mut().unwrap(), ref_bool);
let arms_w = (&raw mut w_arms).as_mut().unwrap();
for w in arms_w.iter_mut() {
stlib_update_mult_icon(w, ref_bool);
}
stlib_update_mult_icon((&raw mut w_faces).as_mut().unwrap(), ref_bool);
let kb_w = (&raw mut w_keyboxes).as_mut().unwrap();
for w in kb_w.iter_mut() {
stlib_update_mult_icon(w, ref_bool);
}
stlib_update_num((&raw mut w_frags).as_mut().unwrap(), ref_bool);
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
unsafe fn cache_static(name: &[u8]) -> *mut patch_t {
unsafe { W_CacheLumpName(name.as_ptr() as *mut c_char, PU_STATIC) as *mut patch_t }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_loadGraphics() {
unsafe {
let tall = (&raw mut tallnum).as_mut().unwrap();
let short = (&raw mut shortnum).as_mut().unwrap();
for i in 0..10usize {
let t_name = format!("STTNUM{}\0", i);
tall[i] = cache_static(t_name.as_bytes());
let s_name = format!("STYSNUM{}\0", i);
short[i] = cache_static(s_name.as_bytes());
}
tallpercent = cache_static(b"STTPRCNT\0");
let keys_arr = (&raw mut keys).as_mut().unwrap();
for (i, slot) in keys_arr.iter_mut().enumerate() {
let k_name = format!("STKEYS{}\0", i);
*slot = cache_static(k_name.as_bytes());
}
armsbg = cache_static(b"STARMS\0");
let arms_arr = (&raw mut arms).as_mut().unwrap();
for i in 0..6usize {
let g_name = format!("STGNUM{}\0", i + 2);
arms_arr[i][0] = cache_static(g_name.as_bytes());
arms_arr[i][1] = short[i + 2];
}
let fb_name = format!("STFB{}\0", consoleplayer);
faceback = cache_static(fb_name.as_bytes());
sbar = cache_static(b"STBAR\0");
let faces_arr = (&raw mut faces).as_mut().unwrap();
let mut facenum = 0usize;
for i in 0..ST_NUMPAINFACES {
for j in 0..ST_NUMSTRAIGHTFACES {
let name = format!("STFST{}{}\0", i, j);
faces_arr[facenum] = cache_static(name.as_bytes());
facenum += 1;
}
faces_arr[facenum] = cache_static(format!("STFTR{}0\0", i).as_bytes());
facenum += 1;
faces_arr[facenum] = cache_static(format!("STFTL{}0\0", i).as_bytes());
facenum += 1;
faces_arr[facenum] = cache_static(format!("STFOUCH{}\0", i).as_bytes());
facenum += 1;
faces_arr[facenum] = cache_static(format!("STFEVL{}\0", i).as_bytes());
facenum += 1;
faces_arr[facenum] = cache_static(format!("STFKILL{}\0", i).as_bytes());
facenum += 1;
}
faces_arr[facenum] = cache_static(b"STFGOD0\0");
facenum += 1;
faces_arr[facenum] = cache_static(b"STFDEAD0\0");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_loadData() {
unsafe {
lu_palette = w_get_num_for_name("PLAYPAL");
ST_loadGraphics();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_unloadGraphics() {
unsafe {
let tall = (&raw const tallnum).as_ref().unwrap();
let short = (&raw const shortnum).as_ref().unwrap();
for i in 0..10usize {
Z_ChangeTag(tall[i] as *mut c_void, PU_CACHE);
Z_ChangeTag(short[i] as *mut c_void, PU_CACHE);
}
Z_ChangeTag(tallpercent as *mut c_void, PU_CACHE);
Z_ChangeTag(armsbg as *mut c_void, PU_CACHE);
let arms_arr = (&raw const arms).as_ref().unwrap();
for slot in arms_arr {
Z_ChangeTag(slot[0] as *mut c_void, PU_CACHE);
}
let keys_arr = (&raw const keys).as_ref().unwrap();
for &slot in keys_arr {
Z_ChangeTag(slot as *mut c_void, PU_CACHE);
}
Z_ChangeTag(sbar as *mut c_void, PU_CACHE);
Z_ChangeTag(faceback as *mut c_void, PU_CACHE);
let faces_arr = (&raw const faces).as_ref().unwrap();
for &slot in faces_arr {
Z_ChangeTag(slot as *mut c_void, PU_CACHE);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_unloadData() {
unsafe {
ST_unloadGraphics();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_initData() {
unsafe {
st_firsttime = 1;
let players_arr = (&raw mut players).as_mut().unwrap();
plyr = &mut players_arr[consoleplayer as usize];
st_clock = 0;
st_chatstate = StartChatState;
st_gamestate = FirstPersonState;
st_statusbaron = 1;
st_chat = 0;
st_oldchat = 0;
st_cursoron = 0;
st_faceindex = 0;
ST_PALETTE.store(-1, Ordering::Relaxed);
st_oldhealth = -1;
let old_w = (&raw mut oldweaponsowned).as_mut().unwrap();
old_w.copy_from_slice(&(*plyr).weaponowned);
(&raw mut keyboxes).as_mut().unwrap().fill(-1);
STlib_init();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_createWidgets() {
unsafe {
let p = &mut *plyr;
let tall_ptr = (&raw mut tallnum) as *mut *mut patch_t;
let short_ptr = (&raw mut shortnum) as *mut *mut patch_t;
let ready = p.readyweapon as usize;
let ammo_type = weaponinfo[ready].ammo;
let ready_num_ptr = if ammo_type == am_noammo {
&raw mut LARGEAMMO
} else {
&mut p.ammo[ammo_type as usize] as *mut c_int
};
let w_r = (&raw mut w_ready).as_mut().unwrap();
stlib_init_num(
w_r,
ST_AMMOX,
ST_AMMOY,
tall_ptr,
ready_num_ptr,
&raw mut st_statusbaron,
ST_AMMOWIDTH,
);
w_r.data = p.readyweapon;
stlib_init_percent(
(&raw mut w_health).as_mut().unwrap(),
ST_HEALTHX,
ST_HEALTHY,
tall_ptr,
&mut p.health as *mut c_int,
&raw mut st_statusbaron,
tallpercent,
);
stlib_init_bin_icon(
(&raw mut w_armsbg).as_mut().unwrap(),
ST_ARMSBGX,
ST_ARMSBGY,
armsbg,
&raw mut st_notdeathmatch,
&raw mut st_statusbaron,
);
let arms_w = (&raw mut w_arms).as_mut().unwrap();
let arms_p = (&raw mut arms).as_mut().unwrap();
for i in 0..6usize {
stlib_init_mult_icon(
&mut arms_w[i],
ST_ARMSX + ((i as c_int) % 3) * ST_ARMSXSPACE,
ST_ARMSY + ((i as c_int) / 3) * ST_ARMSYSPACE,
arms_p[i].as_mut_ptr(),
&mut p.weaponowned[i + 1] as *mut c_int,
&raw mut st_armson,
);
}
stlib_init_num(
(&raw mut w_frags).as_mut().unwrap(),
ST_FRAGSX,
ST_FRAGSY,
tall_ptr,
&raw mut st_fragscount,
&raw mut st_fragson,
ST_FRAGSWIDTH,
);
stlib_init_mult_icon(
(&raw mut w_faces).as_mut().unwrap(),
ST_FACESX,
ST_FACESY,
(&raw mut faces) as *mut *mut patch_t,
&raw mut st_faceindex,
&raw mut st_statusbaron,
);
stlib_init_percent(
(&raw mut w_armor).as_mut().unwrap(),
ST_ARMORX,
ST_ARMORY,
tall_ptr,
&mut p.armorpoints as *mut c_int,
&raw mut st_statusbaron,
tallpercent,
);
let keys_ptr = (&raw mut keys) as *mut *mut patch_t;
let kb_w = (&raw mut w_keyboxes).as_mut().unwrap();
let kb = (&raw mut keyboxes).as_mut().unwrap();
stlib_init_mult_icon(&mut kb_w[0], ST_KEY0X, ST_KEY0Y, keys_ptr, &mut kb[0] as *mut c_int, &raw mut st_statusbaron);
stlib_init_mult_icon(&mut kb_w[1], ST_KEY1X, ST_KEY1Y, keys_ptr, &mut kb[1] as *mut c_int, &raw mut st_statusbaron);
stlib_init_mult_icon(&mut kb_w[2], ST_KEY2X, ST_KEY2Y, keys_ptr, &mut kb[2] as *mut c_int, &raw mut st_statusbaron);
let ammo_w = (&raw mut w_ammo).as_mut().unwrap();
stlib_init_num(&mut ammo_w[0], ST_AMMO0X, ST_AMMO0Y, short_ptr, &mut p.ammo[0] as *mut c_int, &raw mut st_statusbaron, ST_AMMO0WIDTH);
stlib_init_num(&mut ammo_w[1], ST_AMMO1X, ST_AMMO1Y, short_ptr, &mut p.ammo[1] as *mut c_int, &raw mut st_statusbaron, ST_AMMO1WIDTH);
stlib_init_num(&mut ammo_w[2], ST_AMMO2X, ST_AMMO2Y, short_ptr, &mut p.ammo[2] as *mut c_int, &raw mut st_statusbaron, ST_AMMO2WIDTH);
stlib_init_num(&mut ammo_w[3], ST_AMMO3X, ST_AMMO3Y, short_ptr, &mut p.ammo[3] as *mut c_int, &raw mut st_statusbaron, ST_AMMO3WIDTH);
let maxammo_w = (&raw mut w_maxammo).as_mut().unwrap();
stlib_init_num(&mut maxammo_w[0], ST_MAXAMMO0X, ST_MAXAMMO0Y, short_ptr, &mut p.maxammo[0] as *mut c_int, &raw mut st_statusbaron, ST_MAXAMMO0WIDTH);
stlib_init_num(&mut maxammo_w[1], ST_MAXAMMO1X, ST_MAXAMMO1Y, short_ptr, &mut p.maxammo[1] as *mut c_int, &raw mut st_statusbaron, ST_MAXAMMO1WIDTH);
stlib_init_num(&mut maxammo_w[2], ST_MAXAMMO2X, ST_MAXAMMO2Y, short_ptr, &mut p.maxammo[2] as *mut c_int, &raw mut st_statusbaron, ST_MAXAMMO2WIDTH);
stlib_init_num(&mut maxammo_w[3], ST_MAXAMMO3X, ST_MAXAMMO3Y, short_ptr, &mut p.maxammo[3] as *mut c_int, &raw mut st_statusbaron, ST_MAXAMMO3WIDTH);
}
}
static ST_STOPPED: AtomicI32 = AtomicI32::new(1);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Start() {
unsafe {
if ST_STOPPED.load(Ordering::Relaxed) == 0 {
ST_Stop();
}
ST_initData();
ST_createWidgets();
ST_STOPPED.store(0, Ordering::Relaxed);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ST_Stop() {
if ST_STOPPED.load(Ordering::Relaxed) != 0 {
return;
}
let pal = W_CacheLumpNum(unsafe { lu_palette }, PU_CACHE) as *mut u8;
I_SetPalette(pal);
ST_STOPPED.store(1, Ordering::Relaxed);
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
