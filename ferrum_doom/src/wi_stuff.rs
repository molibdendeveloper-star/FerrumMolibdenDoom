#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::{get_gamemode, get_language};
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

use crate::m_random::m_random;
use crate::s_sound::{S_ChangeMusic, S_StartSound};
use crate::v_video::{V_DrawPatch, V_MarkRect, screens};
use crate::w_wad::W_CacheLumpName;
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_ChangeTag2 as Z_ChangeTag, Z_Free, Z_Malloc};

const NUMEPISODES: usize = 4;
const TICRATE: u32 = 35;
const NUMMAPS: usize = 9;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const FB: c_int = 0;

const WI_TITLEY: c_int = 2;
const WI_SPACINGY: c_int = 33;

const SP_STATSX: c_int = 50;
const SP_STATSY: c_int = 50;
const SP_TIMEX: c_int = 16;
const SP_TIMEY: c_int = (SCREENHEIGHT as c_int) - 32;

const NG_STATSY: c_int = 50;
const NG_SPACINGX: c_int = 64;

const DM_MATRIXX: c_int = 42;
const DM_MATRIXY: c_int = 68;
const DM_SPACINGX: c_int = 40;
const DM_TOTALSX: c_int = 269;
const DM_KILLERSX: c_int = 10;
const DM_KILLERSY: c_int = 100;
const DM_VICTIMSX: c_int = 5;
const DM_VICTIMSY: c_int = 50;

const SHOWNEXTLOCDELAY: c_int = 4;
const mus_inter: c_int = 28;
const mus_dm2int: c_int = 66;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum animenum_t {
    ANIM_ALWAYS = 0,
    ANIM_RANDOM = 1,
    ANIM_LEVEL = 2,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct point_t {
    pub x: c_int,
    pub y: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct anim_t {
    pub a_type: animenum_t,
    pub period: c_int,
    pub nanims: c_int,
    pub loc: point_t,
    pub data1: c_int,
    pub data2: c_int,
    pub p: [*mut patch_t; 3],
    pub nexttic: c_int,
    pub lastdrawn: c_int,
    pub ctr: c_int,
    pub state: c_int,
}

impl anim_t {
    const fn new(
        a_type: animenum_t,
        period: c_int,
        nanims: c_int,
        x: c_int,
        y: c_int,
        data1: c_int,
    ) -> Self {
        Self {
            a_type,
            period,
            nanims,
            loc: point_t { x, y },
            data1,
            data2: 0,
            p: [ptr::null_mut(); 3],
            nexttic: 0,
            lastdrawn: 0,
            ctr: 0,
            state: 0,
        }
    }
}

static LNODES: [[point_t; NUMMAPS]; NUMEPISODES] = [
    [
        point_t { x: 185, y: 164 },
        point_t { x: 148, y: 143 },
        point_t { x: 69, y: 122 },
        point_t { x: 209, y: 102 },
        point_t { x: 116, y: 89 },
        point_t { x: 166, y: 55 },
        point_t { x: 71, y: 56 },
        point_t { x: 135, y: 29 },
        point_t { x: 71, y: 24 },
    ],
    [
        point_t { x: 254, y: 25 },
        point_t { x: 97, y: 50 },
        point_t { x: 188, y: 64 },
        point_t { x: 128, y: 78 },
        point_t { x: 214, y: 92 },
        point_t { x: 133, y: 130 },
        point_t { x: 208, y: 136 },
        point_t { x: 148, y: 140 },
        point_t { x: 235, y: 158 },
    ],
    [
        point_t { x: 156, y: 168 },
        point_t { x: 48, y: 154 },
        point_t { x: 174, y: 95 },
        point_t { x: 265, y: 75 },
        point_t { x: 130, y: 48 },
        point_t { x: 279, y: 23 },
        point_t { x: 198, y: 48 },
        point_t { x: 140, y: 25 },
        point_t { x: 281, y: 136 },
    ],
    [point_t { x: 0, y: 0 }; NUMMAPS],
];

const TIC_3: c_int = (TICRATE as c_int) / 3;
const TIC_4: c_int = (TICRATE as c_int) / 4;

static mut EPSD0_ANIM: [anim_t; 10] = [
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 224, 104, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 184, 160, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 112, 136, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 72, 112, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 88, 96, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 64, 48, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 192, 40, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 136, 16, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 80, 16, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 64, 24, 0),
];

static mut EPSD1_ANIM: [anim_t; 9] = [
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 1),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 2),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 3),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 4),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 5),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 6),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 7),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 3, 192, 144, 8),
    anim_t::new(animenum_t::ANIM_LEVEL, TIC_3, 1, 128, 136, 8),
];

static mut EPSD2_ANIM: [anim_t; 6] = [
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 104, 168, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 40, 136, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 160, 96, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 104, 80, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_3, 3, 120, 32, 0),
    anim_t::new(animenum_t::ANIM_ALWAYS, TIC_4, 3, 40, 0, 0),
];

unsafe fn get_anims_mut(epsd: usize) -> &'static mut [anim_t] {
    unsafe {
        match epsd {
            0 => (&raw mut EPSD0_ANIM).as_mut().unwrap(),
            1 => (&raw mut EPSD1_ANIM).as_mut().unwrap(),
            2 => (&raw mut EPSD2_ANIM).as_mut().unwrap(),
            _ => &mut [],
        }
    }
}

unsafe extern "C" {
    fn G_WorldDone();
    static mut netgame: c_int;
    static mut deathmatch: c_int;
    static mut playeringame: [c_int; NUM_PLAYERS];
    static mut players: [player_t; NUM_PLAYERS];
}

static mut acceleratestage: c_int = 0;
static mut me: c_int = 0;
static mut state: stateenum_t = NoState;
static mut wbs: *mut wbstartstruct_t = ptr::null_mut();
static mut plrs: *mut wbplayerstruct_t = ptr::null_mut();

static mut cnt: c_int = 0;
static mut bcnt: c_int = 0;
static mut firstrefresh: c_int = 0;

static mut cnt_kills: [c_int; NUM_PLAYERS] = [0; NUM_PLAYERS];
static mut cnt_items: [c_int; NUM_PLAYERS] = [0; NUM_PLAYERS];
static mut cnt_secret: [c_int; NUM_PLAYERS] = [0; NUM_PLAYERS];
static mut cnt_time: c_int = 0;
static mut cnt_par: c_int = 0;
static mut cnt_pause: c_int = 0;
static mut NUMCMAPS: c_int = 0;

static mut bg: *mut patch_t = ptr::null_mut();
static mut yah: [*mut patch_t; 2] = [ptr::null_mut(); 2];
static mut splat: *mut patch_t = ptr::null_mut();
static mut percent: *mut patch_t = ptr::null_mut();
static mut colon: *mut patch_t = ptr::null_mut();
static mut num: [*mut patch_t; 10] = [ptr::null_mut(); 10];
static mut wiminus: *mut patch_t = ptr::null_mut();
static mut finished: *mut patch_t = ptr::null_mut();
static mut entering: *mut patch_t = ptr::null_mut();
static mut sp_secret: *mut patch_t = ptr::null_mut();
static mut kills: *mut patch_t = ptr::null_mut();
static mut secret: *mut patch_t = ptr::null_mut();
static mut items: *mut patch_t = ptr::null_mut();
static mut frags: *mut patch_t = ptr::null_mut();
static mut time_patch: *mut patch_t = ptr::null_mut();
static mut par: *mut patch_t = ptr::null_mut();
static mut sucks: *mut patch_t = ptr::null_mut();
static mut killers: *mut patch_t = ptr::null_mut();
static mut victims: *mut patch_t = ptr::null_mut();
static mut total: *mut patch_t = ptr::null_mut();
static mut star: *mut patch_t = ptr::null_mut();
static mut bstar: *mut patch_t = ptr::null_mut();
static mut p: [*mut patch_t; NUM_PLAYERS] = [ptr::null_mut(); NUM_PLAYERS];
static mut bp: [*mut patch_t; NUM_PLAYERS] = [ptr::null_mut(); NUM_PLAYERS];
static mut lnames: *mut *mut patch_t = ptr::null_mut();

unsafe fn cache_static(name: &[u8]) -> *mut patch_t {
    unsafe { W_CacheLumpName(name.as_ptr() as *mut c_char, PU_STATIC) as *mut patch_t }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_slamBackground() {
    unsafe {
        let scr_ptr = (&raw const screens) as *const *mut u8;
        let len = (SCREENWIDTH as usize) * (SCREENHEIGHT as usize);
        let s0 = std::slice::from_raw_parts_mut(*scr_ptr, len);
        let s1 = std::slice::from_raw_parts(*scr_ptr.add(1), len);
        s0.copy_from_slice(s1);
        V_MarkRect(0, 0, SCREENWIDTH as c_int, SCREENHEIGHT as c_int);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn WI_Responder(_ev: *mut event_t) -> c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawLF() {
    unsafe {
        let max_maps = if get_gamemode() == commercial { NUMCMAPS } else { NUMMAPS as c_int };
        if wbs.is_null() || lnames.is_null() || (*wbs).last < 0 || (*wbs).last >= max_maps { return; }
        let mut y = WI_TITLEY;
        let last_patch = &**lnames.add((*wbs).last as usize);
        let scr_w = SCREENWIDTH as c_int;

        V_DrawPatch(
            (scr_w - (last_patch.width as c_int)) / 2,
            y,
            FB,
            last_patch as *const _ as *mut _,
        );
        y += (5 * (last_patch.height as c_int)) / 4;
        V_DrawPatch(
            (scr_w - ((*finished).width as c_int)) / 2,
            y,
            FB,
            finished as *mut _,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawEL() {
    unsafe {
        let max_maps = if get_gamemode() == commercial { NUMCMAPS } else { NUMMAPS as c_int };
        if wbs.is_null() || lnames.is_null() || (*wbs).next < 0 || (*wbs).next >= max_maps { return; }
        let mut y = WI_TITLEY;
        let scr_w = SCREENWIDTH as c_int;

        V_DrawPatch(
            (scr_w - ((*entering).width as c_int)) / 2,
            y,
            FB,
            entering as *mut _,
        );
        let next_patch = &**lnames.add((*wbs).next as usize);
        y += (5 * (next_patch.height as c_int)) / 4;
        V_DrawPatch(
            (scr_w - (next_patch.width as c_int)) / 2,
            y,
            FB,
            next_patch as *const _ as *mut _,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawOnLnode(n: c_int, c: *mut *mut patch_t) {
    unsafe { if wbs.is_null() { return; } }
    unsafe {
        let epsd = (*wbs).epsd as usize;
        let node = LNODES[epsd][n as usize];
        let scr_w = SCREENWIDTH as c_int;
        let scr_h = SCREENHEIGHT as c_int;
let candidates = std::slice::from_raw_parts(c, 2);
for &patch_ptr in candidates {
if patch_ptr.is_null() {
continue;
}
let patch = &*patch_ptr;
let left = node.x - (patch.leftoffset as c_int);
let top = node.y - (patch.topoffset as c_int);
let right = left + (patch.width as c_int);
let bottom = top + (patch.height as c_int);
if left >= 0 && right < scr_w && top >= 0 && bottom < scr_h {
V_DrawPatch(node.x, node.y, FB, patch_ptr as *mut _);
return;
}
}
println!("Could not place patch on level {}", n + 1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_initAnimatedBack() {
unsafe {
if get_gamemode() == commercial || (*wbs).epsd > 2 {
return;
}
for a in get_anims_mut((*wbs).epsd as usize) {
a.ctr = -1;
match a.a_type {
animenum_t::ANIM_ALWAYS => {
a.nexttic = bcnt + 1 + (m_random() % a.period);
}
animenum_t::ANIM_RANDOM => {
a.nexttic = bcnt + 1 + a.data2 + (m_random() % a.data1);
}
animenum_t::ANIM_LEVEL => {
a.nexttic = bcnt + 1;
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_updateAnimatedBack() {
unsafe {
if get_gamemode() == commercial || (*wbs).epsd > 2 {
return;
}
let next_lvl = (*wbs).next;
for (i, a) in get_anims_mut((*wbs).epsd as usize).iter_mut().enumerate() {
if bcnt == a.nexttic {
match a.a_type {
animenum_t::ANIM_ALWAYS => {
a.ctr += 1;
if a.ctr >= a.nanims {
a.ctr = 0;
}
a.nexttic = bcnt + a.period;
}
animenum_t::ANIM_RANDOM => {
a.ctr += 1;
if a.ctr == a.nanims {
a.ctr = -1;
a.nexttic = bcnt + a.data2 + (m_random() % a.data1);
} else {
a.nexttic = bcnt + a.period;
}
}
animenum_t::ANIM_LEVEL => {
if !(state == StatCount && i == 7) && next_lvl == a.data1 {
a.ctr += 1;
if a.ctr == a.nanims {
a.ctr -= 1;
}
a.nexttic = bcnt + a.period;
}
}
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawAnimatedBack() {
    unsafe { if wbs.is_null() { return; } }
unsafe {
if get_gamemode() == commercial || (*wbs).epsd > 2 {
return;
}
for a in get_anims_mut((*wbs).epsd as usize) {
if a.ctr >= 0 {
V_DrawPatch(a.loc.x, a.loc.y, FB, a.p[a.ctr as usize] as *mut _);
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawNum(
mut x: c_int,
y: c_int,
mut n: c_int,
mut digits: c_int,
) -> c_int {
unsafe {
let num_arr = (&raw const num).as_ref().unwrap();
let fontwidth = (*num_arr[0]).width as c_int;
if digits < 0 {
if n == 0 {
digits = 1;
} else {
digits = 0;
let mut temp = n;
while temp != 0 {
temp /= 10;
digits += 1;
}
}
}
let neg = n < 0;
if neg {
n = -n;
}
if n == 1994 {
return 0;
}
while digits > 0 {
digits -= 1;
x -= fontwidth;
V_DrawPatch(x, y, FB, num_arr[(n % 10) as usize] as *mut _);
n /= 10;
}
if neg {
x -= 8;
V_DrawPatch(x, y, FB, wiminus as *mut _);
}
x
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawPercent(x: c_int, y: c_int, p_val: c_int) {
if p_val < 0 {
return;
}
unsafe {
V_DrawPatch(x, y, FB, percent as *mut _);
WI_drawNum(x, y, p_val, -1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawTime(mut x: c_int, y: c_int, t: c_int) {
if t < 0 {
return;
}
unsafe {
if t <= 61 * 59 {
let mut div = 1;
loop {
let n = (t / div) % 60;
x = WI_drawNum(x, y, n, 2) - ((*colon).width as c_int);
div *= 60;
if div == 60 || (t / div) != 0 {
V_DrawPatch(x, y, FB, colon as *mut _);
}
if (t / div) == 0 {
break;
}
}
} else {
V_DrawPatch(x - ((*sucks).width as c_int), y, FB, sucks as *mut _);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_End() {
unsafe {
WI_unloadData();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_initNoState() {
unsafe {
state = NoState;
acceleratestage = 0;
cnt = 10;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_updateNoState() {
unsafe {
WI_updateAnimatedBack();
cnt -= 1;
if cnt == 0 {
WI_End();
G_WorldDone();
}
}
}
static mut snl_pointeron: bool = false;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_initShowNextLoc() {
unsafe {
state = ShowNextLoc;
acceleratestage = 0;
cnt = SHOWNEXTLOCDELAY * (TICRATE as c_int);
WI_initAnimatedBack();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_updateShowNextLoc() {
unsafe {
WI_updateAnimatedBack();
cnt -= 1;
if cnt == 0 || acceleratestage != 0 {
WI_initNoState();
} else {
snl_pointeron = (cnt & 31) < 20;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawShowNextLoc() {
    unsafe { if wbs.is_null() { return; } }
unsafe {
WI_slamBackground();
WI_drawAnimatedBack();
if get_gamemode() != commercial {
if (*wbs).epsd > 2 {
WI_drawEL();
return;
}
let last = if (*wbs).last == 8 {
(*wbs).next - 1
} else {
(*wbs).last
};
for i in 0..=last {
WI_drawOnLnode(i, &raw mut splat);
}
if (*wbs).didsecret != 0 {
WI_drawOnLnode(8, &raw mut splat);
}
if snl_pointeron {
WI_drawOnLnode((*wbs).next, (&raw mut yah) as *mut *mut patch_t);
}
}
if get_gamemode() != commercial || (*wbs).next != 30 {
WI_drawEL();
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawNoState() {
unsafe {
snl_pointeron = true;
WI_drawShowNextLoc();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_fragSum(playernum: c_int) -> c_int {
unsafe {
let in_game = (&raw const playeringame).as_ref().unwrap();
let p_entry = &*plrs.add(playernum as usize);
let mut frags_total = 0;
for i in 0..NUM_PLAYERS {
if in_game[i] != 0 && (i as c_int) != playernum {
frags_total += p_entry.frags[i];
}
}
frags_total - p_entry.frags[playernum as usize]
}
}
static mut dm_state: c_int = 0;
static mut dm_frags: [[c_int; NUM_PLAYERS]; NUM_PLAYERS] = [[0; NUM_PLAYERS]; NUM_PLAYERS];
static mut dm_totals: [c_int; NUM_PLAYERS] = [0; NUM_PLAYERS];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_initDeathmatchStats() {
unsafe {
state = StatCount;
acceleratestage = 0;
dm_state = 1;
cnt_pause = TICRATE as c_int;
let in_game = (&raw const playeringame).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
if in_game[i] != 0 {
dm_frags[i].fill(0);
dm_totals[i] = 0;
}
}
WI_initAnimatedBack();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_updateDeathmatchStats() {
unsafe {
WI_updateAnimatedBack();
let in_game = (&raw const playeringame).as_ref().unwrap();
let plrs_slice = std::slice::from_raw_parts(plrs, NUM_PLAYERS);
if acceleratestage != 0 && dm_state != 4 {
acceleratestage = 0;
for i in 0..NUM_PLAYERS {
if in_game[i] != 0 {
for j in 0..NUM_PLAYERS {
if in_game[j] != 0 {
dm_frags[i][j] = plrs_slice[i].frags[j];
}
}
dm_totals[i] = WI_fragSum(i as c_int);
}
}
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
dm_state = 4;
}
if dm_state == 2 {
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let mut stillticking = false;
for i in 0..NUM_PLAYERS {
if in_game[i] != 0 {
for j in 0..NUM_PLAYERS {
if in_game[j] != 0 && dm_frags[i][j] != plrs_slice[i].frags[j] {
if plrs_slice[i].frags[j] < 0 {
dm_frags[i][j] -= 1;
} else {
dm_frags[i][j] += 1;
}
dm_frags[i][j] = dm_frags[i][j].clamp(-99, 99);
stillticking = true;
}
}
dm_totals[i] = WI_fragSum(i as c_int).clamp(-99, 99);
}
}
if !stillticking {
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
dm_state += 1;
}
} else if dm_state == 4 {
if acceleratestage != 0 {
S_StartSound(ptr::null_mut(), sfx_slop as c_int);
if get_gamemode() == commercial {
WI_initNoState();
} else {
WI_initShowNextLoc();
}
}
} else if (dm_state & 1) != 0 {
cnt_pause -= 1;
if cnt_pause == 0 {
dm_state += 1;
cnt_pause = TICRATE as c_int;
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawDeathmatchStats() {
unsafe {
WI_slamBackground();
WI_drawAnimatedBack();
WI_drawLF();
V_DrawPatch(
DM_TOTALSX - ((*total).width as c_int) / 2,
DM_MATRIXY - WI_SPACINGY + 10,
FB,
total as *mut _,
);
V_DrawPatch(DM_KILLERSX, DM_KILLERSY, FB, killers as *mut _);
V_DrawPatch(DM_VICTIMSX, DM_VICTIMSY, FB, victims as *mut _);
let mut x = DM_MATRIXX + DM_SPACINGX;
let mut y = DM_MATRIXY;
let in_game = (&raw const playeringame).as_ref().unwrap();
let p_arr = (&raw const p).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
if in_game[i] != 0 {
let pi = p_arr[i];
let half_w = ((*pi).width as c_int) / 2;
V_DrawPatch(x - half_w, DM_MATRIXY - WI_SPACINGY, FB, pi as *mut _);
V_DrawPatch(DM_MATRIXX - half_w, y, FB, pi as *mut _);
if (i as c_int) == me {
V_DrawPatch(x - half_w, DM_MATRIXY - WI_SPACINGY, FB, bstar as *mut _);
V_DrawPatch(DM_MATRIXX - half_w, y, FB, star as *mut _);
}
}
x += DM_SPACINGX;
y += WI_SPACINGY;
}
y = DM_MATRIXY + 10;
let num_arr = (&raw const num).as_ref().unwrap();
let w = (*num_arr[0]).width as c_int;
for i in 0..NUM_PLAYERS {
x = DM_MATRIXX + DM_SPACINGX;
if in_game[i] != 0 {
for j in 0..NUM_PLAYERS {
if in_game[j] != 0 {
WI_drawNum(x + w, y, dm_frags[i][j], 2);
}
x += DM_SPACINGX;
}
WI_drawNum(DM_TOTALSX + w, y, dm_totals[i], 2);
}
y += WI_SPACINGY;
}
}
}
static mut cnt_frags: [c_int; NUM_PLAYERS] = [0; NUM_PLAYERS];
static mut dofrags: c_int = 0;
static mut ng_state: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_initNetgameStats() {
unsafe {
state = StatCount;
acceleratestage = 0;
ng_state = 1;
cnt_pause = TICRATE as c_int;
dofrags = 0;
let in_game = (&raw const playeringame).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
cnt_kills[i] = 0;
cnt_items[i] = 0;
cnt_secret[i] = 0;
cnt_frags[i] = 0;
dofrags += WI_fragSum(i as c_int);
}
dofrags = (dofrags != 0) as c_int;
WI_initAnimatedBack();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_updateNetgameStats() {
unsafe {
WI_updateAnimatedBack();
let in_game = (&raw const playeringame).as_ref().unwrap();
let plrs_slice = std::slice::from_raw_parts(plrs, NUM_PLAYERS);
if acceleratestage != 0 && ng_state != 10 {
acceleratestage = 0;
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
let pl = &plrs_slice[i];
cnt_kills[i] = (pl.skills * 100) / (*wbs).maxkills;
cnt_items[i] = (pl.sitems * 100) / (*wbs).maxitems;
cnt_secret[i] = (pl.ssecret * 100) / (*wbs).maxsecret;
if dofrags != 0 {
cnt_frags[i] = WI_fragSum(i as c_int);
}
}
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
ng_state = 10;
}
if ng_state == 2 {
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let mut stillticking = false;
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
cnt_kills[i] += 2;
let max_k = (plrs_slice[i].skills * 100) / (*wbs).maxkills;
if cnt_kills[i] >= max_k {
cnt_kills[i] = max_k;
} else {
stillticking = true;
}
}
if !stillticking {
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
ng_state += 1;
}
} else if ng_state == 4 {
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let mut stillticking = false;
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
cnt_items[i] += 2;
let max_i = (plrs_slice[i].sitems * 100) / (*wbs).maxitems;
if cnt_items[i] >= max_i {
cnt_items[i] = max_i;
} else {
stillticking = true;
}
}
if !stillticking {
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
ng_state += 1;
}
} else if ng_state == 6 {
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let mut stillticking = false;
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
cnt_secret[i] += 2;
let max_s = (plrs_slice[i].ssecret * 100) / (*wbs).maxsecret;
if cnt_secret[i] >= max_s {
cnt_secret[i] = max_s;
} else {
stillticking = true;
}
}
if !stillticking {
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
ng_state += 1 + 2 * ((dofrags == 0) as c_int);
}
} else if ng_state == 8 {
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let mut stillticking = false;
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
cnt_frags[i] += 1;
let fsum = WI_fragSum(i as c_int);
if cnt_frags[i] >= fsum {
cnt_frags[i] = fsum;
} else {
stillticking = true;
}
}
if !stillticking {
S_StartSound(ptr::null_mut(), sfx_pldeth as c_int);
ng_state += 1;
}
} else if ng_state == 10 {
if acceleratestage != 0 {
S_StartSound(ptr::null_mut(), sfx_sgcock as c_int);
if get_gamemode() == commercial {
WI_initNoState();
} else {
WI_initShowNextLoc();
}
}
} else if (ng_state & 1) != 0 {
cnt_pause -= 1;
if cnt_pause == 0 {
ng_state += 1;
cnt_pause = TICRATE as c_int;
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawNetgameStats() {
unsafe {
let pwidth = (*percent).width as c_int;
let ng_statsx = 32 + ((*star).width as c_int) / 2 + 32 * ((dofrags == 0) as c_int);
WI_slamBackground();
WI_drawAnimatedBack();
WI_drawLF();
V_DrawPatch(
ng_statsx + NG_SPACINGX - ((*kills).width as c_int),
NG_STATSY,
FB,
kills as *mut _,
);
V_DrawPatch(
ng_statsx + 2 * NG_SPACINGX - ((*items).width as c_int),
NG_STATSY,
FB,
items as *mut _,
);
V_DrawPatch(
ng_statsx + 3 * NG_SPACINGX - ((*secret).width as c_int),
NG_STATSY,
FB,
secret as *mut _,
);
if dofrags != 0 {
V_DrawPatch(
ng_statsx + 4 * NG_SPACINGX - ((*frags).width as c_int),
NG_STATSY,
FB,
frags as *mut _,
);
}
let mut y = NG_STATSY + ((*kills).height as c_int);
let in_game = (&raw const playeringame).as_ref().unwrap();
let p_arr = (&raw const p).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
let mut x = ng_statsx;
let pi = p_arr[i];
V_DrawPatch(x - ((*pi).width as c_int), y, FB, pi as *mut _);
if (i as c_int) == me {
V_DrawPatch(x - ((*pi).width as c_int), y, FB, star as *mut _);
}
x += NG_SPACINGX;
WI_drawPercent(x - pwidth, y + 10, cnt_kills[i]);
x += NG_SPACINGX;
WI_drawPercent(x - pwidth, y + 10, cnt_items[i]);
x += NG_SPACINGX;
WI_drawPercent(x - pwidth, y + 10, cnt_secret[i]);
x += NG_SPACINGX;
if dofrags != 0 {
WI_drawNum(x, y + 10, cnt_frags[i], -1);
}
y += WI_SPACINGY;
}
}
}
static mut sp_state: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_initStats() {
unsafe {
state = StatCount;
acceleratestage = 0;
sp_state = 1;
cnt_kills[0] = -1;
cnt_items[0] = -1;
cnt_secret[0] = -1;
cnt_time = -1;
cnt_par = -1;
cnt_pause = TICRATE as c_int;
WI_initAnimatedBack();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_updateStats() {
unsafe {
WI_updateAnimatedBack();
let tic = TICRATE as c_int;
let me_pl = &*plrs.add(me as usize);
if acceleratestage != 0 && sp_state != 10 {
acceleratestage = 0;
cnt_kills[0] = (me_pl.skills * 100) / (*wbs).maxkills;
cnt_items[0] = (me_pl.sitems * 100) / (*wbs).maxitems;
cnt_secret[0] = (me_pl.ssecret * 100) / (*wbs).maxsecret;
cnt_time = me_pl.stime / tic;
cnt_par = (*wbs).partime / tic;
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
sp_state = 10;
}
if sp_state == 2 {
cnt_kills[0] += 2;
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let max_k = (me_pl.skills * 100) / (*wbs).maxkills;
if cnt_kills[0] >= max_k {
cnt_kills[0] = max_k;
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
sp_state += 1;
}
} else if sp_state == 4 {
cnt_items[0] += 2;
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let max_i = (me_pl.sitems * 100) / (*wbs).maxitems;
if cnt_items[0] >= max_i {
cnt_items[0] = max_i;
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
sp_state += 1;
}
} else if sp_state == 6 {
cnt_secret[0] += 2;
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
let max_s = (me_pl.ssecret * 100) / (*wbs).maxsecret;
if cnt_secret[0] >= max_s {
cnt_secret[0] = max_s;
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
sp_state += 1;
}
} else if sp_state == 8 {
if (bcnt & 3) == 0 {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
cnt_time += 3;
let max_t = me_pl.stime / tic;
if cnt_time >= max_t {
cnt_time = max_t;
}
cnt_par += 3;
let max_p = (*wbs).partime / tic;
if cnt_par >= max_p {
cnt_par = max_p;
if cnt_time >= max_t {
S_StartSound(ptr::null_mut(), sfx_barexp as c_int);
sp_state += 1;
}
}
} else if sp_state == 10 {
if acceleratestage != 0 {
S_StartSound(ptr::null_mut(), sfx_sgcock as c_int);
if get_gamemode() == commercial {
WI_initNoState();
} else {
WI_initShowNextLoc();
}
}
} else if (sp_state & 1) != 0 {
cnt_pause -= 1;
if cnt_pause == 0 {
sp_state += 1;
cnt_pause = TICRATE as c_int;
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_drawStats() {
unsafe {
let num_arr = (&raw const num).as_ref().unwrap();
let lh = (3 * ((*num_arr[0]).height as c_int)) / 2;
let scr_w = SCREENWIDTH as c_int;
WI_slamBackground();
WI_drawAnimatedBack();
WI_drawLF();
V_DrawPatch(SP_STATSX, SP_STATSY, FB, kills as *mut _);
WI_drawPercent(scr_w - SP_STATSX, SP_STATSY, cnt_kills[0]);
V_DrawPatch(SP_STATSX, SP_STATSY + lh, FB, items as *mut _);
WI_drawPercent(scr_w - SP_STATSX, SP_STATSY + lh, cnt_items[0]);
V_DrawPatch(SP_STATSX, SP_STATSY + 2 * lh, FB, sp_secret as *mut _);
WI_drawPercent(scr_w - SP_STATSX, SP_STATSY + 2 * lh, cnt_secret[0]);
V_DrawPatch(SP_TIMEX, SP_TIMEY, FB, time_patch as *mut _);
WI_drawTime(scr_w / 2 - SP_TIMEX, SP_TIMEY, cnt_time);
if (*wbs).epsd < 3 {
V_DrawPatch(scr_w / 2 + SP_TIMEX, SP_TIMEY, FB, par as *mut _);
WI_drawTime(scr_w - SP_TIMEX, SP_TIMEY, cnt_par);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_checkForAccelerate() {
unsafe {
let in_game = (&raw const playeringame).as_ref().unwrap();
let players_arr = (&raw mut players).as_mut().unwrap();
let bt_atk = BT_ATTACK as u8;
let bt_use = BT_USE as u8;
for i in 0..NUM_PLAYERS {
if in_game[i] != 0 {
let player = &mut players_arr[i];
let buttons = player.cmd[7];
if (buttons & bt_atk) != 0 {
if player.attackdown == 0 {
acceleratestage = 1;
}
player.attackdown = 1;
} else {
player.attackdown = 0;
}
if (buttons & bt_use) != 0 {
if player.usedown == 0 {
acceleratestage = 1;
}
player.usedown = 1;
} else {
player.usedown = 0;
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_Ticker() {
unsafe {
bcnt += 1;
if bcnt == 1 {
if get_gamemode() == commercial {
S_ChangeMusic(mus_dm2int, 1);
} else {
S_ChangeMusic(mus_inter, 1);
}
}
WI_checkForAccelerate();
match state {
StatCount => {
if deathmatch != 0 {
WI_updateDeathmatchStats();
} else if netgame != 0 {
WI_updateNetgameStats();
} else {
WI_updateStats();
}
}
ShowNextLoc => WI_updateShowNextLoc(),
NoState => WI_updateNoState(),
_ => {}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_loadData() {
unsafe {
let bg_name =
if get_gamemode() == commercial || (get_gamemode() == retail && (*wbs).epsd == 3) {
"INTERPIC\0".to_string()
} else {
format!("WIMAP{}\0", (*wbs).epsd)
};
bg = W_CacheLumpName(bg_name.as_ptr() as *mut c_char, PU_CACHE) as *mut patch_t;
V_DrawPatch(0, 0, 1, bg as *mut _);
if get_gamemode() == commercial {
NUMCMAPS = 32;
let count = NUMCMAPS as usize;
lnames = Z_Malloc(
(size_of::<*mut patch_t>() * count) as c_int,
PU_STATIC,
ptr::null_mut(),
) as *mut *mut patch_t;
let lnames_slice = std::slice::from_raw_parts_mut(lnames, count);
for (i, slot) in lnames_slice.iter_mut().enumerate() {
let name = format!("CWILV{:02}\0", i);
*slot = cache_static(name.as_bytes());
}
} else {
lnames = Z_Malloc(
(size_of::<*mut patch_t>() * NUMMAPS) as c_int,
PU_STATIC,
ptr::null_mut(),
) as *mut *mut patch_t;
let lnames_slice = std::slice::from_raw_parts_mut(lnames, NUMMAPS);
for (i, slot) in lnames_slice.iter_mut().enumerate() {
let name = format!("WILV{}{}\0", (*wbs).epsd, i);
*slot = cache_static(name.as_bytes());
}
yah[0] = cache_static(b"WIURH0\0");
yah[1] = cache_static(b"WIURH1\0");
splat = cache_static(b"WISPLAT\0");
if (*wbs).epsd < 3 {
let epsd = (*wbs).epsd as usize;
let anims = get_anims_mut(epsd);
for j in 0..anims.len() {
let nanims = anims[j].nanims as usize;
for i in 0..nanims {
if epsd != 1 || j != 8 {
let name = format!("WIA{}{:02}{:02}\0", epsd, j, i);
anims[j].p[i] = cache_static(name.as_bytes());
} else {
anims[j].p[i] = anims[4].p[i];
}
}
}
}
}
wiminus = cache_static(b"WIMINUS\0");
let num_arr = (&raw mut num).as_mut().unwrap();
for (i, slot) in num_arr.iter_mut().enumerate() {
let name = format!("WINUM{}\0", i);
*slot = cache_static(name.as_bytes());
}
percent = cache_static(b"WIPCNT\0");
finished = cache_static(b"WIF\0");
entering = cache_static(b"WIENTER\0");
kills = cache_static(b"WIOSTK\0");
secret = cache_static(b"WIOSTS\0");
sp_secret = cache_static(b"WISCRT2\0");
if get_language() == french && netgame != 0 && deathmatch == 0 {
items = cache_static(b"WIOBJ\0");
} else {
items = cache_static(b"WIOSTI\0");
}
frags = cache_static(b"WIFRGS\0");
colon = cache_static(b"WICOLON\0");
time_patch = cache_static(b"WITIME\0");
sucks = cache_static(b"WISUCKS\0");
par = cache_static(b"WIPAR\0");
killers = cache_static(b"WIKILRS\0");
victims = cache_static(b"WIVCTMS\0");
total = cache_static(b"WIMSTT\0");
star = cache_static(b"STFST01\0");
bstar = cache_static(b"STFDEAD0\0");
let p_arr = (&raw mut p).as_mut().unwrap();
let bp_arr = (&raw mut bp).as_mut().unwrap();
for i in 0..NUM_PLAYERS {
let p_name = format!("STPB{}\0", i);
p_arr[i] = cache_static(p_name.as_bytes());
let bp_name = format!("WIBP{}\0", i + 1);
bp_arr[i] = cache_static(bp_name.as_bytes());
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_unloadData() {
unsafe {
Z_ChangeTag(wiminus as *mut c_void, PU_CACHE);
let num_arr = (&raw const num).as_ref().unwrap();
for &slot in num_arr {
Z_ChangeTag(slot as *mut c_void, PU_CACHE);
}
if get_gamemode() == commercial {
let lnames_slice = std::slice::from_raw_parts(lnames, NUMCMAPS as usize);
for &slot in lnames_slice {
Z_ChangeTag(slot as *mut c_void, PU_CACHE);
}
} else {
Z_ChangeTag(yah[0] as *mut c_void, PU_CACHE);
Z_ChangeTag(yah[1] as *mut c_void, PU_CACHE);
Z_ChangeTag(splat as *mut c_void, PU_CACHE);
let lnames_slice = std::slice::from_raw_parts(lnames, NUMMAPS);
for &slot in lnames_slice {
Z_ChangeTag(slot as *mut c_void, PU_CACHE);
}
if (*wbs).epsd < 3 {
let epsd = (*wbs).epsd as usize;
for (j, a) in get_anims_mut(epsd).iter().enumerate() {
if epsd != 1 || j != 8 {
for &patch_ptr in &a.p[..(a.nanims as usize)] {
Z_ChangeTag(patch_ptr as *mut c_void, PU_CACHE);
}
}
}
}
}
Z_Free(lnames as *mut c_void);
        lnames = ptr::null_mut();
        wbs = ptr::null_mut();
Z_ChangeTag(percent as *mut c_void, PU_CACHE);
Z_ChangeTag(colon as *mut c_void, PU_CACHE);
Z_ChangeTag(finished as *mut c_void, PU_CACHE);
Z_ChangeTag(entering as *mut c_void, PU_CACHE);
Z_ChangeTag(kills as *mut c_void, PU_CACHE);
Z_ChangeTag(secret as *mut c_void, PU_CACHE);
Z_ChangeTag(sp_secret as *mut c_void, PU_CACHE);
Z_ChangeTag(items as *mut c_void, PU_CACHE);
Z_ChangeTag(frags as *mut c_void, PU_CACHE);
Z_ChangeTag(time_patch as *mut c_void, PU_CACHE);
Z_ChangeTag(sucks as *mut c_void, PU_CACHE);
Z_ChangeTag(par as *mut c_void, PU_CACHE);
Z_ChangeTag(victims as *mut c_void, PU_CACHE);
Z_ChangeTag(killers as *mut c_void, PU_CACHE);
Z_ChangeTag(total as *mut c_void, PU_CACHE);
let p_arr = (&raw const p).as_ref().unwrap();
let bp_arr = (&raw const bp).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
Z_ChangeTag(p_arr[i] as *mut c_void, PU_CACHE);
Z_ChangeTag(bp_arr[i] as *mut c_void, PU_CACHE);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_Drawer() {
    unsafe { if wbs.is_null() { return; } }
unsafe {
match state {
StatCount => {
if deathmatch != 0 {
WI_drawDeathmatchStats();
} else if netgame != 0 {
WI_drawNetgameStats();
} else {
WI_drawStats();
}
}
ShowNextLoc => WI_drawShowNextLoc(),
NoState => WI_drawNoState(),
_ => {}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_initVariables(wbstartstruct: *mut wbstartstruct_t) {
unsafe {
wbs = wbstartstruct;
acceleratestage = 0;
cnt = 0;
bcnt = 0;
firstrefresh = 1;
me = (*wbs).pnum;
plrs = (&raw mut (*wbs).plyr) as *mut wbplayerstruct_t;
if (*wbs).maxkills == 0 {
(*wbs).maxkills = 1;
}
if (*wbs).maxitems == 0 {
(*wbs).maxitems = 1;
}
if (*wbs).maxsecret == 0 {
(*wbs).maxsecret = 1;
}
if get_gamemode() != retail && (*wbs).epsd > 2 {
(*wbs).epsd -= 3;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WI_Start(wbstartstruct: *mut wbstartstruct_t) {
unsafe {
WI_initVariables(wbstartstruct);
WI_loadData();
if deathmatch != 0 {
WI_initDeathmatchStats();
} else if netgame != 0 {
WI_initNetgameStats();
} else {
WI_initStats();
}
}
}
