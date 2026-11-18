#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::g_game::{
    consoleplayer, deathmatch, gameepisode, gamemap, netgame, playeringame, players, singledemo,
    viewactive,
};
use crate::m_cheat::cht_CheckCheat;
use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::p_setup::{bmaporgx, bmaporgy, lines, numlines, numsectors, numvertexes, sectors, vertexes};
use crate::st_stuff::ST_Responder;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};
use crate::v_video::{V_DrawPatch, V_MarkRect, screens};
use crate::w_wad::W_CacheLumpName;
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_ChangeTag2 as Z_ChangeTag};

const FRAC_UNIT: c_int = 65536;
const MAX_INT: c_int = 0x7fff_ffff;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const QUARTER_FINE: usize = FINEANGLES / 4;

const REDS: c_int = 256 - 5 * 16;
const REDRANGE: c_int = 16;
const GREENS: c_int = 7 * 16;
const GREENRANGE: c_int = 16;
const GRAYS: c_int = 6 * 16;
const GRAYSRANGE: c_int = 16;
const BROWNS: c_int = 4 * 16;
const BROWNRANGE: c_int = 16;
const YELLOWS: c_int = 256 - 32 + 7;
const YELLOWRANGE: c_int = 1;
const BLACK: c_int = 0;
const WHITE: c_int = 256 - 47;

const BACKGROUND: c_int = BLACK;
const WALLCOLORS: c_int = REDS;
const WALLRANGE: c_int = REDRANGE;
const TSWALLCOLORS: c_int = GRAYS;
const FDWALLCOLORS: c_int = BROWNS;
const CDWALLCOLORS: c_int = YELLOWS;
const THINGCOLORS: c_int = GREENS;
const THINGRANGE: c_int = GREENRANGE;
const SECRETWALLCOLORS: c_int = WALLCOLORS;
const GRIDCOLORS: c_int = GRAYS + GRAYSRANGE / 2;
const XHAIRCOLORS: c_int = GRAYS;

const FB: c_int = 0;

const AM_PANDOWNKEY: c_int = KEY_DOWNARROW as c_int;
const AM_PANUPKEY: c_int = KEY_UPARROW as c_int;
const AM_PANRIGHTKEY: c_int = KEY_RIGHTARROW as c_int;
const AM_PANLEFTKEY: c_int = KEY_LEFTARROW as c_int;
const AM_ZOOMINKEY: c_int = b'=' as c_int;
const AM_ZOOMOUTKEY: c_int = b'-' as c_int;
const AM_STARTKEY: c_int = KEY_TAB as c_int;
const AM_ENDKEY: c_int = KEY_TAB as c_int;
const AM_GOBIGKEY: c_int = b'0' as c_int;
const AM_FOLLOWKEY: c_int = b'f' as c_int;
const AM_GRIDKEY: c_int = b'g' as c_int;
const AM_MARKKEY: c_int = b'm' as c_int;
const AM_CLEARMARKKEY: c_int = b'c' as c_int;

const AM_NUMMARKPOINTS: usize = 10;
const INITSCALEMTOF: c_int = (0.2 * (FRAC_UNIT as f64)) as c_int;
const F_PANINC: c_int = 4;
const M_ZOOMIN: c_int = (1.02 * (FRAC_UNIT as f64)) as c_int;
const M_ZOOMOUT: c_int = ((FRAC_UNIT as f64) / 1.02) as c_int;
const LINE_NEVERSEE: i16 = 128;

const AM_MSGHEADER: c_int = (((b'a' as u32) << 24) + ((b'm' as u32) << 16)) as c_int;
const AM_MSGENTERED: c_int = AM_MSGHEADER | ((b'e' as c_int) << 8);
const AM_MSGEXITED: c_int = AM_MSGHEADER | ((b'x' as c_int) << 8);

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct fpoint_t {
    pub x: c_int,
    pub y: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct fline_t {
    pub a: fpoint_t,
    pub b: fpoint_t,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct mpoint_t {
    pub x: c_int,
    pub y: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct mline_t {
    pub a: mpoint_t,
    pub b: mpoint_t,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct islope_t {
    pub slp: c_int,
    pub islp: c_int,
}

const PR: c_int = (8 * (PLAYERRADIUS as c_int)) / 7;

static PLAYER_ARROW: [mline_t; 7] = [
    mline_t { a: mpoint_t { x: -PR + PR / 8, y: 0 }, b: mpoint_t { x: PR, y: 0 } },
    mline_t { a: mpoint_t { x: PR, y: 0 }, b: mpoint_t { x: PR - PR / 2, y: PR / 4 } },
    mline_t { a: mpoint_t { x: PR, y: 0 }, b: mpoint_t { x: PR - PR / 2, y: -PR / 4 } },
    mline_t { a: mpoint_t { x: -PR + PR / 8, y: 0 }, b: mpoint_t { x: -PR - PR / 8, y: PR / 4 } },
    mline_t { a: mpoint_t { x: -PR + PR / 8, y: 0 }, b: mpoint_t { x: -PR - PR / 8, y: -PR / 4 } },
    mline_t { a: mpoint_t { x: -PR + 3 * PR / 8, y: 0 }, b: mpoint_t { x: -PR + PR / 8, y: PR / 4 } },
    mline_t { a: mpoint_t { x: -PR + 3 * PR / 8, y: 0 }, b: mpoint_t { x: -PR + PR / 8, y: -PR / 4 } },
];

static CHEAT_PLAYER_ARROW: [mline_t; 16] = [
    mline_t { a: mpoint_t { x: -PR + PR / 8, y: 0 }, b: mpoint_t { x: PR, y: 0 } },
    mline_t { a: mpoint_t { x: PR, y: 0 }, b: mpoint_t { x: PR - PR / 2, y: PR / 6 } },
    mline_t { a: mpoint_t { x: PR, y: 0 }, b: mpoint_t { x: PR - PR / 2, y: -PR / 6 } },
    mline_t { a: mpoint_t { x: -PR + PR / 8, y: 0 }, b: mpoint_t { x: -PR - PR / 8, y: PR / 6 } },
    mline_t { a: mpoint_t { x: -PR + PR / 8, y: 0 }, b: mpoint_t { x: -PR - PR / 8, y: -PR / 6 } },
    mline_t { a: mpoint_t { x: -PR + 3 * PR / 8, y: 0 }, b: mpoint_t { x: -PR + PR / 8, y: PR / 6 } },
    mline_t { a: mpoint_t { x: -PR + 3 * PR / 8, y: 0 }, b: mpoint_t { x: -PR + PR / 8, y: -PR / 6 } },
    mline_t { a: mpoint_t { x: -PR / 2, y: 0 }, b: mpoint_t { x: -PR / 2, y: -PR / 6 } },
    mline_t { a: mpoint_t { x: -PR / 2, y: -PR / 6 }, b: mpoint_t { x: -PR / 2 + PR / 6, y: -PR / 6 } },
    mline_t { a: mpoint_t { x: -PR / 2 + PR / 6, y: -PR / 6 }, b: mpoint_t { x: -PR / 2 + PR / 6, y: PR / 4 } },
    mline_t { a: mpoint_t { x: -PR / 6, y: 0 }, b: mpoint_t { x: -PR / 6, y: -PR / 6 } },
    mline_t { a: mpoint_t { x: -PR / 6, y: -PR / 6 }, b: mpoint_t { x: 0, y: -PR / 6 } },
    mline_t { a: mpoint_t { x: 0, y: -PR / 6 }, b: mpoint_t { x: 0, y: PR / 4 } },
    mline_t { a: mpoint_t { x: PR / 6, y: PR / 4 }, b: mpoint_t { x: PR / 6, y: -PR / 7 } },
    mline_t { a: mpoint_t { x: PR / 6, y: -PR / 7 }, b: mpoint_t { x: PR / 6 + PR / 32, y: -PR / 7 - PR / 32 } },
    mline_t { a: mpoint_t { x: PR / 6 + PR / 32, y: -PR / 7 - PR / 32 }, b: mpoint_t { x: PR / 6 + PR / 10, y: -PR / 7 } },
];

const UR: f64 = FRAC_UNIT as f64;
static THINTRIANGLE_GUY: [mline_t; 3] = [
    mline_t {
        a: mpoint_t { x: (-0.5 * UR) as c_int, y: (-0.7 * UR) as c_int },
        b: mpoint_t { x: FRAC_UNIT, y: 0 },
    },
    mline_t {
        a: mpoint_t { x: FRAC_UNIT, y: 0 },
        b: mpoint_t { x: (-0.5 * UR) as c_int, y: (0.7 * UR) as c_int },
    },
    mline_t {
        a: mpoint_t { x: (-0.5 * UR) as c_int, y: (0.7 * UR) as c_int },
        b: mpoint_t { x: (-0.5 * UR) as c_int, y: (-0.7 * UR) as c_int },
    },
];

#[unsafe(no_mangle)]
pub static mut automapactive: c_int = 0;

static mut cheating: c_int = 0;
static mut grid: c_int = 0;
static mut leveljuststarted: c_int = 1;

static mut finit_width: c_int = SCREENWIDTH as c_int;
static mut finit_height: c_int = (SCREENHEIGHT as c_int) - 32;

static mut f_x: c_int = 0;
static mut f_y: c_int = 0;
static mut f_w: c_int = 0;
static mut f_h: c_int = 0;

static mut lightlev: c_int = 0;
static mut fb: *mut u8 = ptr::null_mut();
static mut amclock: c_int = 0;

static mut m_paninc: mpoint_t = mpoint_t { x: 0, y: 0 };
static mut mtof_zoommul: c_int = FRAC_UNIT;
static mut ftom_zoommul: c_int = FRAC_UNIT;

static mut m_x: c_int = 0;
static mut m_y: c_int = 0;
static mut m_x2: c_int = 0;
static mut m_y2: c_int = 0;
static mut m_w: c_int = 0;
static mut m_h: c_int = 0;

static mut min_x: c_int = 0;
static mut min_y: c_int = 0;
static mut max_x: c_int = 0;
static mut max_y: c_int = 0;
static mut max_w: c_int = 0;
static mut max_h: c_int = 0;
static mut min_w: c_int = 0;
static mut min_h: c_int = 0;

static mut min_scale_mtof: c_int = 0;
static mut max_scale_mtof: c_int = 0;

static mut old_m_w: c_int = 0;
static mut old_m_h: c_int = 0;
static mut old_m_x: c_int = 0;
static mut old_m_y: c_int = 0;

static mut f_oldloc: mpoint_t = mpoint_t { x: 0, y: 0 };
static mut scale_mtof: c_int = INITSCALEMTOF;
static mut scale_ftom: c_int = 0;

static mut plr: *mut player_t = ptr::null_mut();
static mut marknums: [*mut patch_t; 10] = [ptr::null_mut(); 10];
static mut markpoints: [mpoint_t; AM_NUMMARKPOINTS] = [mpoint_t { x: -1, y: 0 }; AM_NUMMARKPOINTS];
static mut markpointnum: usize = 0;
static mut followplayer: c_int = 1;

static mut cheat_amap_seq: [u8; 5] = [0xb2, 0x26, 0x26, 0x2e, 0xff];
static mut cheat_amap: cheatseq_t = cheatseq_t {
    sequence: unsafe { (&raw mut cheat_amap_seq) as *mut u8 },
    p: ptr::null_mut(),
};

static mut stopped: c_int = 1;

#[inline]
fn ftom(x: c_int) -> c_int {
    fixed_mul(x << 16, unsafe { scale_ftom })
}

#[inline]
fn mtof(x: c_int) -> c_int {
    fixed_mul(x, unsafe { scale_mtof }) >> 16
}

#[inline]
fn cxmtof(x: c_int) -> c_int {
    unsafe { f_x + mtof(x - m_x) }
}

#[inline]
fn cymtof(y: c_int) -> c_int {
    unsafe { f_y + (f_h - mtof(y - m_y)) }
}

#[inline]
unsafe fn fb_slice() -> &'static mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(fb, (f_w * f_h).max(0) as usize) }
}

#[inline]
pub fn am_get_islope(ml: &mline_t) -> islope_t {
    let dy = ml.a.y - ml.b.y;
    let dx = ml.b.x - ml.a.x;
    islope_t {
        islp: if dy == 0 {
            if dx < 0 { -MAX_INT } else { MAX_INT }
        } else {
            fixed_div(dx, dy)
        },
        slp: if dx == 0 {
            if dy < 0 { -MAX_INT } else { MAX_INT }
        } else {
            fixed_div(dy, dx)
        },
    }
}

#[inline]
pub fn am_rotate_pt(x: c_int, y: c_int, a: u32) -> (c_int, c_int) {
    let idx = (a >> ANGLETOFINESHIFT) as usize;
    let sin_v = finesine[idx];
    let cos_v = finesine[idx + QUARTER_FINE];
    let nx = fixed_mul(x, cos_v) - fixed_mul(y, sin_v);
    let ny = fixed_mul(x, sin_v) + fixed_mul(y, cos_v);
    (nx, ny)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_getIslope(ml: *mut mline_t, is: *mut islope_t) {
    if let (Some(m), Some(out)) = (unsafe { ml.as_ref() }, unsafe { is.as_mut() }) {
        *out = am_get_islope(m);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_activateNewScale() {
    unsafe {
        m_x += m_w / 2;
        m_y += m_h / 2;
        m_w = ftom(f_w);
        m_h = ftom(f_h);
m_x -= m_w / 2;
m_y -= m_h / 2;
m_x2 = m_x + m_w;
m_y2 = m_y + m_h;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_saveScaleAndLoc() {
unsafe {
old_m_x = m_x;
old_m_y = m_y;
old_m_w = m_w;
old_m_h = m_h;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_restoreScaleAndLoc() {
unsafe {
m_w = old_m_w;
m_h = old_m_h;
if followplayer == 0 {
m_x = old_m_x;
m_y = old_m_y;
} else {
let mo = &*(*plr).mo;
m_x = mo.x - m_w / 2;
m_y = mo.y - m_h / 2;
}
m_x2 = m_x + m_w;
m_y2 = m_y + m_h;
scale_mtof = fixed_div(f_w << FRACBITS, m_w);
scale_ftom = fixed_div(FRAC_UNIT, scale_mtof);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_addMark() {
unsafe {
let mp = (&raw mut markpoints).as_mut().unwrap();
mp[markpointnum] = mpoint_t {
x: m_x + m_w / 2,
y: m_y + m_h / 2,
};
markpointnum = (markpointnum + 1) % AM_NUMMARKPOINTS;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_findMinMaxBoundaries() {
unsafe {
min_x = MAX_INT;
min_y = MAX_INT;
max_x = -MAX_INT;
max_y = -MAX_INT;
let verts = std::slice::from_raw_parts(vertexes, numvertexes.max(0) as usize);
for v in verts {
min_x = min_x.min(v.x);
max_x = max_x.max(v.x);
min_y = min_y.min(v.y);
max_y = max_y.max(v.y);
}
max_w = max_x - min_x;
max_h = max_y - min_y;
min_w = 2 * (PLAYERRADIUS as c_int);
min_h = 2 * (PLAYERRADIUS as c_int);
let a = fixed_div(f_w << FRACBITS, max_w);
let b = fixed_div(f_h << FRACBITS, max_h);
min_scale_mtof = a.min(b);
max_scale_mtof = fixed_div(f_h << FRACBITS, 2 * (PLAYERRADIUS as c_int));
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_changeWindowLoc() {
unsafe {
if m_paninc.x != 0 || m_paninc.y != 0 {
followplayer = 0;
f_oldloc.x = MAX_INT;
}
m_x = (m_x + m_paninc.x).clamp(min_x - m_w / 2, max_x - m_w / 2);
m_y = (m_y + m_paninc.y).clamp(min_y - m_h / 2, max_y - m_h / 2);
m_x2 = m_x + m_w;
m_y2 = m_y + m_h;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_initVariables() {
unsafe {
let mut st_notify = event_t {
type_: ev_keyup,
data1: AM_MSGENTERED,
data2: 0,
data3: 0,
};
automapactive = 1;
fb = *(&raw const screens as *const *mut u8);
f_oldloc.x = MAX_INT;
amclock = 0;
lightlev = 0;
m_paninc = mpoint_t { x: 0, y: 0 };
ftom_zoommul = FRAC_UNIT;
mtof_zoommul = FRAC_UNIT;
m_w = ftom(f_w);
m_h = ftom(f_h);
let in_game = (&raw const playeringame).as_ref().unwrap();
let mut pnum = consoleplayer as usize;
if in_game[pnum] == 0 {
pnum = in_game.iter().position(|&v| v != 0).unwrap_or(0);
}
let players_arr = (&raw mut players).as_mut().unwrap();
plr = &mut players_arr[pnum];
let mo = &*(*plr).mo;
m_x = mo.x - m_w / 2;
m_y = mo.y - m_h / 2;
AM_changeWindowLoc();
old_m_x = m_x;
old_m_y = m_y;
old_m_w = m_w;
old_m_h = m_h;
ST_Responder((&raw mut st_notify) as *mut _);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_loadPics() {
unsafe {
let mn = (&raw mut marknums).as_mut().unwrap();
for (i, slot) in mn.iter_mut().enumerate() {
let name = format!("AMMNUM{}\0", i);
*slot = W_CacheLumpName(name.as_ptr() as *mut c_char, PU_STATIC) as *mut patch_t;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_unloadPics() {
unsafe {
let mn = (&raw const marknums).as_ref().unwrap();
for &slot in mn {
Z_ChangeTag(slot as *mut c_void, PU_CACHE);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_clearMarks() {
unsafe {
let mp = (&raw mut markpoints).as_mut().unwrap();
for pt in mp.iter_mut() {
pt.x = -1;
}
markpointnum = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_LevelInit() {
unsafe {
leveljuststarted = 0;
f_x = 0;
f_y = 0;
f_w = finit_width;
f_h = finit_height;
AM_clearMarks();
AM_findMinMaxBoundaries();
scale_mtof = fixed_div(min_scale_mtof, (0.7 * (FRAC_UNIT as f64)) as c_int);
if scale_mtof > max_scale_mtof {
scale_mtof = min_scale_mtof;
}
scale_ftom = fixed_div(FRAC_UNIT, scale_mtof);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_Stop() {
unsafe {
let mut st_notify = event_t {
type_: ev_keyup,
data1: AM_MSGEXITED,
data2: 0,
data3: 0,
};
AM_unloadPics();
automapactive = 0;
ST_Responder((&raw mut st_notify) as *mut _);
stopped = 1;
}
}
static LAST_LEVEL: AtomicI32 = AtomicI32::new(-1);
static LAST_EPISODE: AtomicI32 = AtomicI32::new(-1);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_Start() {
unsafe {
if stopped == 0 {
AM_Stop();
}
stopped = 0;
let cur_map = gamemap;
let cur_ep = gameepisode;
if LAST_LEVEL.load(Ordering::Relaxed) != cur_map
|| LAST_EPISODE.load(Ordering::Relaxed) != cur_ep
{
AM_LevelInit();
LAST_LEVEL.store(cur_map, Ordering::Relaxed);
LAST_EPISODE.store(cur_ep, Ordering::Relaxed);
}
AM_initVariables();
AM_loadPics();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_minOutWindowScale() {
unsafe {
scale_mtof = min_scale_mtof;
scale_ftom = fixed_div(FRAC_UNIT, scale_mtof);
AM_activateNewScale();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_maxOutWindowScale() {
unsafe {
scale_mtof = max_scale_mtof;
scale_ftom = fixed_div(FRAC_UNIT, scale_mtof);
AM_activateNewScale();
}
}
static BIG_STATE: AtomicI32 = AtomicI32::new(0);
static mut MARK_BUFFER: [c_char; 32] = [0; 32];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_Responder(ev: *mut event_t) -> c_int {
let Some(ev_ref) = (unsafe { ev.as_ref() }) else { return 0 };
unsafe {
let mut rc: c_int = 0;
let ev_type = ev_ref.type_;
let data1 = ev_ref.data1;
if automapactive == 0 {
if ev_type == ev_keydown && data1 == AM_STARTKEY {
AM_Start();
viewactive = 0;
rc = 1;
}
} else if ev_type == ev_keydown {
rc = 1;
match data1 {
AM_PANRIGHTKEY => {
if followplayer == 0 {
m_paninc.x = ftom(F_PANINC);
} else {
rc = 0;
}
}
AM_PANLEFTKEY => {
if followplayer == 0 {
m_paninc.x = -ftom(F_PANINC);
} else {
rc = 0;
}
}
AM_PANUPKEY => {
if followplayer == 0 {
m_paninc.y = ftom(F_PANINC);
} else {
rc = 0;
}
}
AM_PANDOWNKEY => {
if followplayer == 0 {
m_paninc.y = -ftom(F_PANINC);
} else {
rc = 0;
}
}
AM_ZOOMOUTKEY => {
mtof_zoommul = M_ZOOMOUT;
ftom_zoommul = M_ZOOMIN;
}
AM_ZOOMINKEY => {
mtof_zoommul = M_ZOOMIN;
ftom_zoommul = M_ZOOMOUT;
}
AM_ENDKEY => {
BIG_STATE.store(0, Ordering::Relaxed);
viewactive = 1;
AM_Stop();
}
AM_GOBIGKEY => {
let next_big = (BIG_STATE.load(Ordering::Relaxed) == 0) as c_int;
BIG_STATE.store(next_big, Ordering::Relaxed);
if next_big != 0 {
AM_saveScaleAndLoc();
AM_minOutWindowScale();
} else {
AM_restoreScaleAndLoc();
}
}
AM_FOLLOWKEY => {
followplayer = (followplayer == 0) as c_int;
f_oldloc.x = MAX_INT;
(*plr).message = if followplayer != 0 {
AMSTR_FOLLOWON.as_ptr() as *mut c_char
} else {
AMSTR_FOLLOWOFF.as_ptr() as *mut c_char
};
}
AM_GRIDKEY => {
grid = (grid == 0) as c_int;
(*plr).message = if grid != 0 {
AMSTR_GRIDON.as_ptr() as *mut c_char
} else {
AMSTR_GRIDOFF.as_ptr() as *mut c_char
};
}
AM_MARKKEY => {
let s = format!("Marked Spot {}\0", markpointnum);
let dst = (&raw mut MARK_BUFFER).as_mut().unwrap();
dst.fill(0);
for (d, &b) in dst.iter_mut().zip(s.as_bytes()).take(31) {
*d = b as c_char;
}
(*plr).message = dst.as_mut_ptr();
AM_addMark();
}
AM_CLEARMARKKEY => {
AM_clearMarks();
(*plr).message = AMSTR_MARKSCLEARED.as_ptr() as *mut c_char;
}
_ => {
rc = 0;
}
}
if deathmatch == 0 && cht_CheckCheat((&raw mut cheat_amap) as *mut _, data1 as c_char) != 0
{
rc = 0;
cheating = (cheating + 1) % 3;
}
} else if ev_type == ev_keyup {
rc = 0;
match data1 {
AM_PANRIGHTKEY | AM_PANLEFTKEY => {
if followplayer == 0 {
m_paninc.x = 0;
}
}
AM_PANUPKEY | AM_PANDOWNKEY => {
if followplayer == 0 {
m_paninc.y = 0;
}
}
AM_ZOOMOUTKEY | AM_ZOOMINKEY => {
mtof_zoommul = FRAC_UNIT;
ftom_zoommul = FRAC_UNIT;
}
_ => {}
}
}
rc
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_changeWindowScale() {
unsafe {
scale_mtof = fixed_mul(scale_mtof, mtof_zoommul);
scale_ftom = fixed_div(FRAC_UNIT, scale_mtof);
if scale_mtof < min_scale_mtof {
AM_minOutWindowScale();
} else if scale_mtof > max_scale_mtof {
AM_maxOutWindowScale();
} else {
AM_activateNewScale();
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_doFollowPlayer() {
unsafe {
let mo = &*(*plr).mo;
if f_oldloc.x != mo.x || f_oldloc.y != mo.y {
m_x = ftom(mtof(mo.x)) - m_w / 2;
m_y = ftom(mtof(mo.y)) - m_h / 2;
m_x2 = m_x + m_w;
m_y2 = m_y + m_h;
f_oldloc = mpoint_t { x: mo.x, y: mo.y };
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_Ticker() {
unsafe {
if automapactive == 0 {
return;
}
amclock += 1;
if followplayer != 0 {
AM_doFollowPlayer();
}
if ftom_zoommul != FRAC_UNIT {
AM_changeWindowScale();
}
if m_paninc.x != 0 || m_paninc.y != 0 {
AM_changeWindowLoc();
}
}
}
pub fn am_clip_mline(ml: &mline_t) -> Option<fline_t> {
const LEFT: c_int = 1;
const RIGHT: c_int = 2;
const BOTTOM: c_int = 4;
const TOP: c_int = 8;
let (mx1, my1, mx2, my2, fw, fh) = unsafe { (m_x, m_y, m_x2, m_y2, f_w, f_h) };
let mut outcode1 = if ml.a.y > my2 {
TOP
} else if ml.a.y < my1 {
BOTTOM
} else {
0
};
let mut outcode2 = if ml.b.y > my2 {
TOP
} else if ml.b.y < my1 {
BOTTOM
} else {
0
};
if (outcode1 & outcode2) != 0 {
return None;
}
if ml.a.x < mx1 {
outcode1 |= LEFT;
} else if ml.a.x > mx2 {
outcode1 |= RIGHT;
}
if ml.b.x < mx1 {
outcode2 |= LEFT;
} else if ml.b.x > mx2 {
outcode2 |= RIGHT;
}
if (outcode1 & outcode2) != 0 {
return None;
}
let mut fl = fline_t {
a: fpoint_t {
x: cxmtof(ml.a.x),
y: cymtof(ml.a.y),
},
b: fpoint_t {
x: cxmtof(ml.b.x),
y: cymtof(ml.b.y),
},
};
let do_outcode = |mx: c_int, my: c_int| -> c_int {
let mut oc = 0;
if my < 0 {
oc |= TOP;
} else if my >= fh {
oc |= BOTTOM;
}
if mx < 0 {
oc |= LEFT;
} else if mx >= fw {
oc |= RIGHT;
}
oc
};
outcode1 = do_outcode(fl.a.x, fl.a.y);
outcode2 = do_outcode(fl.b.x, fl.b.y);
if (outcode1 & outcode2) != 0 {
return None;
}
let mut tmp = fpoint_t::default();
while (outcode1 | outcode2) != 0 {
let outside = if outcode1 != 0 { outcode1 } else { outcode2 };
if (outside & TOP) != 0 {
let dy = fl.a.y - fl.b.y;
let dx = fl.b.x - fl.a.x;
tmp.x = fl.a.x + (dx * fl.a.y) / dy;
tmp.y = 0;
} else if (outside & BOTTOM) != 0 {
let dy = fl.a.y - fl.b.y;
let dx = fl.b.x - fl.a.x;
tmp.x = fl.a.x + (dx * (fl.a.y - fh)) / dy;
tmp.y = fh - 1;
} else if (outside & RIGHT) != 0 {
let dy = fl.b.y - fl.a.y;
let dx = fl.b.x - fl.a.x;
tmp.y = fl.a.y + (dy * (fw - 1 - fl.a.x)) / dx;
tmp.x = fw - 1;
} else if (outside & LEFT) != 0 {
let dy = fl.b.y - fl.a.y;
let dx = fl.b.x - fl.a.x;
tmp.y = fl.a.y + (dy * (-fl.a.x)) / dx;
tmp.x = 0;
}
if outside == outcode1 {
fl.a = tmp;
outcode1 = do_outcode(fl.a.x, fl.a.y);
} else {
fl.b = tmp;
outcode2 = do_outcode(fl.b.x, fl.b.y);
}
if (outcode1 & outcode2) != 0 {
return None;
}
}
Some(fl)
}
pub fn am_draw_fline(fl: &fline_t, color: c_int) {
let (fw, fh) = unsafe { (f_w, f_h) };
if fl.a.x < 0
|| fl.a.x >= fw
|| fl.a.y < 0
|| fl.a.y >= fh
|| fl.b.x < 0
|| fl.b.x >= fw
|| fl.b.y < 0
|| fl.b.y >= fh
{
return;
}
let dx = fl.b.x - fl.a.x;
let ax = 2 * dx.abs();
let sx = if dx < 0 { -1 } else { 1 };
let dy = fl.b.y - fl.a.y;
let ay = 2 * dy.abs();
let sy = if dy < 0 { -1 } else { 1 };
let mut x = fl.a.x;
let mut y = fl.a.y;
let c = color as u8;
let w = fw as usize;
let buf = unsafe { fb_slice() };
if ax > ay {
let mut d = ay - ax / 2;
loop {
buf[(y as usize) * w + (x as usize)] = c;
if x == fl.b.x {
return;
}
if d >= 0 {
y += sy;
d -= ax;
}
x += sx;
d += ay;
}
} else {
let mut d = ax - ay / 2;
loop {
buf[(y as usize) * w + (x as usize)] = c;
if y == fl.b.y {
return;
}
if d >= 0 {
x += sx;
d -= ay;
}
y += sy;
d += ax;
}
}
}
#[inline]
pub fn am_draw_mline(ml: &mline_t, color: c_int) {
if let Some(fl) = am_clip_mline(ml) {
am_draw_fline(&fl, color);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_clearFB(color: c_int) {
unsafe {
fb_slice().fill(color as u8);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_clipMline(ml: *mut mline_t, fl: *mut fline_t) -> c_int {
let (Some(m), Some(out)) = (unsafe { ml.as_ref() }, unsafe { fl.as_mut() }) else {
return 0;
};
match am_clip_mline(m) {
Some(clipped) => {
*out = clipped;
1
}
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawFline(fl: *mut fline_t, color: c_int) {
if let Some(line) = unsafe { fl.as_ref() } {
am_draw_fline(line, color);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawMline(ml: *mut mline_t, color: c_int) {
if let Some(line) = unsafe { ml.as_ref() } {
am_draw_mline(line, color);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawGrid(color: c_int) {
unsafe {
let step = (MAPBLOCKUNITS as c_int) << FRACBITS;
let mut ml = mline_t::default();
let mut start = m_x;
let rem_x = (start - bmaporgx) % step;
if rem_x != 0 {
start += step - rem_x;
}
let end_x = m_x + m_w;
ml.a.y = m_y;
ml.b.y = m_y + m_h;
let mut x = start;
while x < end_x {
ml.a.x = x;
ml.b.x = x;
am_draw_mline(&ml, color);
x += step;
}
start = m_y;
let rem_y = (start - bmaporgy) % step;
if rem_y != 0 {
start += step - rem_y;
}
let end_y = m_y + m_h;
ml.a.x = m_x;
ml.b.x = m_x + m_w;
let mut y = start;
while y < end_y {
ml.a.y = y;
ml.b.y = y;
am_draw_mline(&ml, color);
y += step;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawWalls() {
unsafe {
let lns = std::slice::from_raw_parts(lines, numlines.max(0) as usize);
let has_allmap = (*plr).powers[pw_allmap as usize] != 0;
for line in lns {
let v1 = &*line.v1;
let v2 = &*line.v2;
let ml = mline_t {
a: mpoint_t { x: v1.x, y: v1.y },
b: mpoint_t { x: v2.x, y: v2.y },
};
if cheating != 0 || (line.flags & (ML_MAPPED as i16)) != 0 {
if (line.flags & LINE_NEVERSEE) != 0 && cheating == 0 {
continue;
}
if line.backsector.is_null() {
am_draw_mline(&ml, WALLCOLORS + lightlev);
} else if line.special == 39 {
am_draw_mline(&ml, WALLCOLORS + WALLRANGE / 2);
} else if (line.flags & (ML_SECRET as i16)) != 0 {
let c = if cheating != 0 {
SECRETWALLCOLORS
} else {
WALLCOLORS
};
am_draw_mline(&ml, c + lightlev);
} else if (*line.backsector).floorheight != (*line.frontsector).floorheight {
am_draw_mline(&ml, FDWALLCOLORS + lightlev);
} else if (*line.backsector).ceilingheight != (*line.frontsector).ceilingheight {
am_draw_mline(&ml, CDWALLCOLORS + lightlev);
} else if cheating != 0 {
am_draw_mline(&ml, TSWALLCOLORS + lightlev);
}
} else if has_allmap && (line.flags & LINE_NEVERSEE) == 0 {
am_draw_mline(&ml, GRAYS + 3);
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_rotate(x: *mut c_int, y: *mut c_int, a: u32) {
if let (Some(xr), Some(yr)) = (unsafe { x.as_mut() }, unsafe { y.as_mut() }) {
let (nx, ny) = am_rotate_pt(*xr, *yr, a);
*xr = nx;
*yr = ny;
}
}
pub fn am_draw_line_character(
shape: &[mline_t],
scale: c_int,
angle: u32,
color: c_int,
x: c_int,
y: c_int,
) {
for seg in shape {
let mut ax = seg.a.x;
let mut ay = seg.a.y;
if scale != 0 {
ax = fixed_mul(scale, ax);
ay = fixed_mul(scale, ay);
}
if angle != 0 {
(ax, ay) = am_rotate_pt(ax, ay, angle);
}
let mut bx = seg.b.x;
let mut by = seg.b.y;
if scale != 0 {
bx = fixed_mul(scale, bx);
by = fixed_mul(scale, by);
}
if angle != 0 {
(bx, by) = am_rotate_pt(bx, by, angle);
}
let ml = mline_t {
a: mpoint_t { x: ax + x, y: ay + y },
b: mpoint_t { x: bx + x, y: by + y },
};
am_draw_mline(&ml, color);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawLineCharacter(
lineguy: *const mline_t,
lineguylines: c_int,
scale: c_int,
angle: u32,
color: c_int,
x: c_int,
y: c_int,
) {
if lineguy.is_null() || lineguylines <= 0 {
return;
}
let slice = unsafe { std::slice::from_raw_parts(lineguy, lineguylines as usize) };
am_draw_line_character(slice, scale, angle, color, x, y);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawPlayers() {
unsafe {
let their_colors: [c_int; 4] = [GREENS, GRAYS, BROWNS, REDS];
if netgame == 0 {
let mo = &*(*plr).mo;
let shape: &[mline_t] = if cheating != 0 {
&CHEAT_PLAYER_ARROW
} else {
&PLAYER_ARROW
};
am_draw_line_character(shape, 0, mo.angle, WHITE, mo.x, mo.y);
return;
}
let players_arr = (&raw mut players).as_mut().unwrap();
let in_game = (&raw const playeringame).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
let p = &mut players_arr[i];
if deathmatch != 0 && singledemo == 0 && (p as *mut player_t) != plr {
continue;
}
if in_game[i] == 0 {
continue;
}
let color = if p.powers[pw_invisibility as usize] != 0 {
246
} else {
their_colors[i]
};
let mo = &*p.mo;
am_draw_line_character(&PLAYER_ARROW, 0, mo.angle, color, mo.x, mo.y);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawThings(colors: c_int, _colorrange: c_int) {
unsafe {
let secs = std::slice::from_raw_parts(sectors, numsectors.max(0) as usize);
for sec in secs {
let mut t = sec.thinglist;
while !t.is_null() {
let th = &*t;
am_draw_line_character(
&THINTRIANGLE_GUY,
16 << FRACBITS,
th.angle,
colors + lightlev,
th.x,
th.y,
);
t = th.snext;
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawMarks() {
unsafe {
let mp_arr = (&raw const markpoints).as_ref().unwrap();
let mn_arr = (&raw const marknums).as_ref().unwrap();
for (mp, &patch_ptr) in mp_arr.iter().zip(mn_arr.iter()) {
if mp.x != -1 {
let fx = cxmtof(mp.x);
let fy = cymtof(mp.y);
if fx >= f_x && fx <= f_w - 5 && fy >= f_y && fy <= f_h - 6 {
V_DrawPatch(fx, fy, FB, patch_ptr as *mut _);
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_drawCrosshair(color: c_int) {
unsafe {
let idx = ((f_w * (f_h + 1)) / 2) as usize;
fb_slice()[idx] = color as u8;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AM_Drawer() {
unsafe {
if automapactive == 0 {
return;
}
AM_clearFB(BACKGROUND);
if grid != 0 {
AM_drawGrid(GRIDCOLORS);
}
AM_drawWalls();
AM_drawPlayers();
if cheating == 2 {
AM_drawThings(THINGCOLORS, THINGRANGE);
}
AM_drawCrosshair(XHAIRCOLORS);
AM_drawMarks();
V_MarkRect(f_x, f_y, f_w, f_h);
}
}
