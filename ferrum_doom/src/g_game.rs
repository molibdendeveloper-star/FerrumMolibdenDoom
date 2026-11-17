#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CStr, c_char, c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::f_finale::{F_Responder, F_StartFinale, F_Ticker};
use crate::hu_stuff::{HU_Responder, HU_Ticker, HU_dequeueChatChar, player_names};
use crate::info::{mobjinfo, states};
use crate::m_argv::{m_check_parm, myargc, myargv};
use crate::m_fixed::FRACBITS;
const FRAC_UNIT: c_int = 65536;
use crate::m_menu::M_StartControlPanel;
use crate::m_misc::{M_ReadFile, M_WriteFile};
use crate::m_random::{m_clear_random, p_random, rndindex};
use crate::p_map::P_CheckPosition;
use crate::p_mobj::{P_RemoveMobj, P_SpawnMobj};
use crate::p_saveg::{
    P_ArchivePlayers, P_ArchiveSpecials, P_ArchiveThinkers, P_ArchiveWorld, P_UnArchivePlayers,
    P_UnArchiveSpecials, P_UnArchiveThinkers, P_UnArchiveWorld, save_p,
};
use crate::p_setup::{P_SetupLevel, deathmatch_p, deathmatchstarts, playerstarts};

use crate::p_tick::{P_Ticker, leveltime};
use crate::r_data::{R_FlatNumForName, R_TextureNumForName};
use crate::r_draw::R_FillBackScreen;
use crate::r_main::{R_ExecuteSetViewSize, R_PointInSubsector, setsizeneeded};
use crate::r_sky::{skyflatnum, skytexture};
const SKYFLATNAME: &[u8] = b"F_SKY1\0";
use crate::s_sound::{S_PauseSound, S_ResumeSound, S_StartSound};
use crate::st_stuff::{ST_Responder, ST_Ticker};
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};
use crate::v_video::{doom_error, screens};
use crate::w_wad::{W_CacheLumpName, W_CheckNumForName};
use crate::wi_stuff::{WI_Start, WI_Ticker};
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_ChangeTag2 as Z_ChangeTag, Z_CheckHeap, Z_Free, Z_Malloc};

const SAVEGAMESIZE: usize = 0x2c000;
const VERSION: u32 = 109;
const SAVESTRINGSIZE: usize = 24;
const VERSIONSIZE: usize = 16;
const DEMOMARKER: u8 = 0x80;
const BODYQUESIZE: usize = 32;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const NUM_WEAPONS: usize = NUMWEAPONS as usize;
const NUM_AMMO: usize = NUMAMMO as usize;
const BACKUP_TICS: usize = 12;
const TURBOTHRESHOLD: i8 = 0x32;
const SLOWTURNTICS: c_int = 6;
const NUMKEYS: usize = 256;

const GGSAVED_STR: &[u8] = b"game saved.\0";
const SAVEGAMENAME_STR: &str = "doomsav";

unsafe extern "C" {
    fn I_BaseTiccmd() -> *mut ticcmd_t;
    fn I_GetTime() -> c_int;
    fn I_Quit();
    fn D_AdvanceDemo();
    fn D_PageTicker();
    fn AM_Responder(ev: *mut event_t) -> c_int;
    fn AM_Ticker();
    fn AM_Stop();
    fn M_ScreenShot();
    fn P_SpawnPlayer(mthing: *mut mapthing_t);

    static mut wipegamestate: gamestate_t;
    static mut ticdup: c_int;
    static mut maketic: c_int;
    static mut netcmds: [[ticcmd_t; BACKUP_TICS]; NUM_PLAYERS];
    static mut respawnparm: c_int;
    static mut fastparm: c_int;
    static mut nomonsters: c_int;
    static mut singletics: c_int;
    static mut gamemode: GameMode_t;
    static mut automapactive: c_int;
    static mut mouseSensitivity: c_int;
    static mut maxammo: [c_int; NUM_AMMO];
}

// --- Глобальные переменные g_game.c ---
#[unsafe(no_mangle)]
pub static mut gameaction: gameaction_t = ga_nothing;

#[unsafe(no_mangle)]
pub static mut gamestate: gamestate_t = GS_DEMOSCREEN;

#[unsafe(no_mangle)]
pub static mut gameskill: skill_t = sk_medium;

#[unsafe(no_mangle)]
pub static mut respawnmonsters: c_int = 0;

#[unsafe(no_mangle)]
pub static mut gameepisode: c_int = 1;

#[unsafe(no_mangle)]
pub static mut gamemap: c_int = 1;

#[unsafe(no_mangle)]
pub static mut paused: c_int = 0;

#[unsafe(no_mangle)]
pub static mut sendpause: c_int = 0;

#[unsafe(no_mangle)]
pub static mut sendsave: c_int = 0;

#[unsafe(no_mangle)]
pub static mut usergame: c_int = 0;

#[unsafe(no_mangle)]
pub static mut timingdemo: c_int = 0;

#[unsafe(no_mangle)]
pub static mut nodrawers: c_int = 0;

#[unsafe(no_mangle)]
pub static mut noblit: c_int = 0;

#[unsafe(no_mangle)]
pub static mut starttime: c_int = 0;

#[unsafe(no_mangle)]
pub static mut viewactive: c_int = 0;

#[unsafe(no_mangle)]
pub static mut deathmatch: c_int = 0;

#[unsafe(no_mangle)]
pub static mut netgame: c_int = 0;

#[unsafe(no_mangle)]
pub static mut playeringame: [c_int; NUM_PLAYERS] = [0; NUM_PLAYERS];

#[unsafe(no_mangle)]
pub static mut players: [player_t; NUM_PLAYERS] = unsafe { std::mem::zeroed() };

#[unsafe(no_mangle)]
pub static mut consoleplayer: c_int = 0;

#[unsafe(no_mangle)]
pub static mut displayplayer: c_int = 0;

#[unsafe(no_mangle)]
pub static mut gametic: c_int = 0;

#[unsafe(no_mangle)]
pub static mut levelstarttic: c_int = 0;

#[unsafe(no_mangle)]
pub static mut totalkills: c_int = 0;

#[unsafe(no_mangle)]
pub static mut totalitems: c_int = 0;

#[unsafe(no_mangle)]
pub static mut totalsecret: c_int = 0;

#[unsafe(no_mangle)]
pub static mut demoname: [c_char; 32] = [0; 32];

#[unsafe(no_mangle)]
pub static mut demorecording: c_int = 0;

#[unsafe(no_mangle)]
pub static mut demoplayback: c_int = 0;

#[unsafe(no_mangle)]
pub static mut netdemo: c_int = 0;

#[unsafe(no_mangle)]
pub static mut demobuffer: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut demo_p: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut demoend: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut singledemo: c_int = 0;

#[unsafe(no_mangle)]
pub static mut precache: c_int = 1;

#[unsafe(no_mangle)]
pub static mut wminfo: wbstartstruct_t = unsafe { std::mem::zeroed() };

#[unsafe(no_mangle)]
pub static mut consistancy: [[c_short; BACKUP_TICS]; NUM_PLAYERS] = [[0; BACKUP_TICS]; NUM_PLAYERS];

#[unsafe(no_mangle)]
pub static mut savebuffer: *mut u8 = ptr::null_mut();

// --- Клавиши и мышь ---
#[unsafe(no_mangle)]
pub static mut key_right: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_left: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_up: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_down: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_strafeleft: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_straferight: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_fire: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_use: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_strafe: c_int = 0;
#[unsafe(no_mangle)]
pub static mut key_speed: c_int = 0;

#[unsafe(no_mangle)]
pub static mut mousebfire: c_int = 0;
#[unsafe(no_mangle)]
pub static mut mousebstrafe: c_int = 0;
#[unsafe(no_mangle)]
pub static mut mousebforward: c_int = 0;

#[unsafe(no_mangle)]
pub static mut joybfire: c_int = 0;
#[unsafe(no_mangle)]
pub static mut joybstrafe: c_int = 0;
#[unsafe(no_mangle)]
pub static mut joybuse: c_int = 0;
#[unsafe(no_mangle)]
pub static mut joybspeed: c_int = 0;

#[unsafe(no_mangle)]
pub static mut forwardmove: [c_int; 2] = [0x19, 0x32];

#[unsafe(no_mangle)]
pub static mut sidemove: [c_int; 2] = [0x18, 0x28];

#[unsafe(no_mangle)]
pub static mut angleturn: [c_int; 3] = [640, 1280, 320];

#[unsafe(no_mangle)]
pub static mut gamekeydown: [c_int; NUMKEYS] = [0; NUMKEYS];

#[unsafe(no_mangle)]
pub static mut turnheld: c_int = 0;

#[unsafe(no_mangle)]
pub static mut mousearray: [c_int; 4] = [0; 4];

#[unsafe(no_mangle)]
pub static mut mousebuttons: *mut c_int = unsafe { ((&raw mut mousearray) as *mut c_int).add(1) };

#[unsafe(no_mangle)]
pub static mut mousex: c_int = 0;

#[unsafe(no_mangle)]
pub static mut mousey: c_int = 0;

#[unsafe(no_mangle)]
pub static mut dclicktime: c_int = 0;
#[unsafe(no_mangle)]
pub static mut dclickstate: c_int = 0;
#[unsafe(no_mangle)]
pub static mut dclicks: c_int = 0;
#[unsafe(no_mangle)]
pub static mut dclicktime2: c_int = 0;
#[unsafe(no_mangle)]
pub static mut dclickstate2: c_int = 0;
#[unsafe(no_mangle)]
pub static mut dclicks2: c_int = 0;

#[unsafe(no_mangle)]
pub static mut joyxmove: c_int = 0;
#[unsafe(no_mangle)]
pub static mut joyymove: c_int = 0;

#[unsafe(no_mangle)]
pub static mut joyarray: [c_int; 5] = [0; 5];

#[unsafe(no_mangle)]
pub static mut joybuttons: *mut c_int = unsafe { ((&raw mut joyarray) as *mut c_int).add(1) };

#[unsafe(no_mangle)]
pub static mut savegameslot: c_int = 0;

#[unsafe(no_mangle)]
pub static mut savedescription: [c_char; 32] = [0; 32];

#[unsafe(no_mangle)]
pub static mut bodyque: [*mut mobj_t; BODYQUESIZE] = [ptr::null_mut(); BODYQUESIZE];

#[unsafe(no_mangle)]
pub static mut bodyqueslot: c_int = 0;

#[unsafe(no_mangle)]
pub static mut statcopy: *mut c_void = ptr::null_mut();

// --- G_CmdChecksum ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_CmdChecksum(cmd: *mut ticcmd_t) -> c_int {
    unsafe {
        let mut sum: c_int = 0;
        let ptr = cmd as *const c_int;
        let count = (size_of::<ticcmd_t>() / 4).saturating_sub(1);
        for i in 0..count {
            sum = sum.wrapping_add(*ptr.add(i));
        }
        sum
    }
}

// --- G_BuildTiccmd ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_BuildTiccmd(cmd: *mut ticcmd_t) {
    unsafe {
        let base = I_BaseTiccmd();
        ptr::copy_nonoverlapping(base, cmd, 1);

        (*cmd).consistancy =
            consistancy[consoleplayer as usize][(maketic as usize) % BACKUP_TICS];

        let kd = (&raw const gamekeydown) as *const c_int;
        let mb = mousebuttons;
        let jb = joybuttons;

        let strafe = *kd.add(key_strafe as usize) != 0
            || *mb.offset(mousebstrafe as isize) != 0
            || *jb.offset(joybstrafe as isize) != 0;

        let speed: usize =
            (*kd.add(key_speed as usize) == 0 || *jb.offset(joybspeed as isize) != 0) as usize;

        let mut forward: c_int = 0;
        let mut side: c_int = 0;

        if joyxmove != 0
            || *kd.add(key_right as usize) != 0
            || *kd.add(key_left as usize) != 0
        {
            turnheld += ticdup;
        } else {
            turnheld = 0;
        }

        let tspeed: usize = if turnheld < SLOWTURNTICS { 2 } else { speed };

        let fwd_spd = forwardmove[speed];
        let side_spd = sidemove[speed];
        let turn_spd = angleturn[tspeed] as c_short;
        let maxplmove = forwardmove[1];
if strafe {
if *kd.add(key_right as usize) != 0 {
side += side_spd;
}
if *kd.add(key_left as usize) != 0 {
side -= side_spd;
}
if joyxmove > 0 {
side += side_spd;
}
if joyxmove < 0 {
side -= side_spd;
}
} else {
if *kd.add(key_right as usize) != 0 {
(*cmd).angleturn = (*cmd).angleturn.wrapping_sub(turn_spd);
}
if *kd.add(key_left as usize) != 0 {
(*cmd).angleturn = (*cmd).angleturn.wrapping_add(turn_spd);
}
if joyxmove > 0 {
(*cmd).angleturn = (*cmd).angleturn.wrapping_sub(turn_spd);
}
if joyxmove < 0 {
(*cmd).angleturn = (*cmd).angleturn.wrapping_add(turn_spd);
}
}
if *kd.add(key_up as usize) != 0 {
forward += fwd_spd;
}
if *kd.add(key_down as usize) != 0 {
forward -= fwd_spd;
}
if joyymove < 0 {
forward += fwd_spd;
}
if joyymove > 0 {
forward -= fwd_spd;
}
if *kd.add(key_strafeleft as usize) != 0 {
side -= side_spd;
}
if *kd.add(key_straferight as usize) != 0 {
side += side_spd;
}
(*cmd).chatchar = HU_dequeueChatChar() as u8;
if *kd.add(key_fire as usize) != 0
|| *mb.offset(mousebfire as isize) != 0
|| *jb.offset(joybfire as isize) != 0
{
(*cmd).buttons |= BT_ATTACK as u8;
}
if *kd.add(key_use as usize) != 0
|| *kd.add(b' ' as usize) != 0
|| *jb.offset(joybuse as isize) != 0
{
(*cmd).buttons |= BT_USE as u8;
dclicks = 0;
}
for i in 0..((NUM_WEAPONS - 1) as usize) {
if *kd.add((b'1' as usize) + i) != 0 {
(*cmd).buttons |= BT_CHANGE as u8;
(*cmd).buttons |= (i << BT_WEAPONSHIFT) as u8;
break;
}
}
let mb_fwd = *mb.offset(mousebforward as isize);
if mb_fwd != 0 {
forward += fwd_spd;
}
if mb_fwd != dclickstate && dclicktime > 1 {
dclickstate = mb_fwd;
if dclickstate != 0 {
dclicks += 1;
}
if dclicks == 2 {
(*cmd).buttons |= BT_USE as u8;
dclicks = 0;
} else {
dclicktime = 0;
}
} else {
dclicktime += ticdup;
if dclicktime > 20 {
dclicks = 0;
dclickstate = 0;
}
}
let bstrafe = (*mb.offset(mousebstrafe as isize) != 0
|| *jb.offset(joybstrafe as isize) != 0) as c_int;
if bstrafe != dclickstate2 && dclicktime2 > 1 {
dclickstate2 = bstrafe;
if dclickstate2 != 0 {
dclicks2 += 1;
}
if dclicks2 == 2 {
(*cmd).buttons |= BT_USE as u8;
dclicks2 = 0;
} else {
dclicktime2 = 0;
}
} else {
dclicktime2 += ticdup;
if dclicktime2 > 20 {
dclicks2 = 0;
dclickstate2 = 0;
}
}
if strafe {
side += mousex * 2;
} else {
(*cmd).angleturn = (*cmd).angleturn.wrapping_sub((mousex * 0x8) as c_short);
}
mousex = 0;
mousey = 0;
forward = forward.clamp(-maxplmove, maxplmove);
side = side.clamp(-maxplmove, maxplmove);
(*cmd).forwardmove = (*cmd).forwardmove.wrapping_add(forward as i8);
(*cmd).sidemove = (*cmd).sidemove.wrapping_add(side as i8);
if sendpause != 0 {
sendpause = 0;
(*cmd).buttons = (BT_SPECIAL | BTS_PAUSE) as u8;
}
if sendsave != 0 {
sendsave = 0;
(*cmd).buttons =
(BT_SPECIAL | BTS_SAVEGAME | ((savegameslot as u32) << BTS_SAVESHIFT)) as u8;
}
}
}
// --- G_DoLoadLevel ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoLoadLevel() {
unsafe {
skyflatnum = R_FlatNumForName(SKYFLATNAME.as_ptr() as *mut c_char);
if gamemode == commercial {
skytexture = R_TextureNumForName(b"SKY3\0".as_ptr() as *mut c_char);
if gamemap < 12 {
skytexture = R_TextureNumForName(b"SKY1\0".as_ptr() as *mut c_char);
} else if gamemap < 21 {
skytexture = R_TextureNumForName(b"SKY2\0".as_ptr() as *mut c_char);
}
}
levelstarttic = gametic;
if wipegamestate == GS_LEVEL {
wipegamestate = -1i32 as gamestate_t;
}
gamestate = GS_LEVEL;
let in_game_ptr = (&raw const playeringame) as *const c_int;
let players_base = (&raw mut players) as *mut player_t;
for i in 0..NUM_PLAYERS {
let p = players_base.add(i);
if *in_game_ptr.add(i) != 0 && (*p).playerstate == (PST_DEAD as c_int) {
(*p).playerstate = (PST_REBORN as c_int);
}
ptr::write_bytes((&raw mut (*p).frags) as *mut c_int, 0, NUM_PLAYERS);
}
P_SetupLevel(gameepisode, gamemap, 0, gameskill);
displayplayer = consoleplayer;
starttime = I_GetTime();
gameaction = ga_nothing;
Z_CheckHeap();
ptr::write_bytes((&raw mut gamekeydown) as *mut c_int, 0, NUMKEYS);
joyxmove = 0;
joyymove = 0;
mousex = 0;
mousey = 0;
sendpause = 0;
sendsave = 0;
paused = 0;
ptr::write_bytes(mousebuttons, 0, 3);
ptr::write_bytes(joybuttons, 0, 4);
}
}
// --- G_Responder ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_Responder(ev: *mut event_t) -> c_int {
unsafe {
let ev_type = (*ev).type_;
let data1 = (*ev).data1;
if gamestate == GS_LEVEL
&& ev_type == ev_keydown
&& (data1 as u32) == KEY_F12
&& (singledemo != 0 || deathmatch == 0)
{
let in_game_ptr = (&raw const playeringame) as *const c_int;
loop {
displayplayer += 1;
if displayplayer == (MAXPLAYERS as c_int) {
displayplayer = 0;
}
if *in_game_ptr.add(displayplayer as usize) != 0 || displayplayer == consoleplayer {
break;
}
}
return 1;
}
if gameaction == ga_nothing
&& singledemo == 0
&& (demoplayback != 0 || gamestate == GS_DEMOSCREEN)
{
if ev_type == ev_keydown
|| (ev_type == ev_mouse && data1 != 0)
|| (ev_type == ev_joystick && data1 != 0)
{
M_StartControlPanel();
return 1;
}
return 0;
}
if gamestate == GS_LEVEL {
if HU_Responder(ev as *mut _) != 0 {
return 1;
}
if ST_Responder(ev as *mut _) != 0 {
return 1;
}
if AM_Responder(ev) != 0 {
return 1;
}
}
if gamestate == GS_FINALE && F_Responder(ev as *mut _) != 0 {
return 1;
}
match ev_type {
ev_keydown => {
if (data1 as u32) == KEY_PAUSE {
sendpause = 1;
return 1;
}
if (data1 as usize) < NUMKEYS {
let kd = (&raw mut gamekeydown) as *mut c_int;
*kd.add(data1 as usize) = 1;
}
1
}
ev_keyup => {
if (data1 as usize) < NUMKEYS {
let kd = (&raw mut gamekeydown) as *mut c_int;
*kd.add(data1 as usize) = 0;
}
0
}
ev_mouse => {
*mousebuttons.offset(0) = data1 & 1;
*mousebuttons.offset(1) = data1 & 2;
*mousebuttons.offset(2) = data1 & 4;
mousex = (*ev).data2 * (mouseSensitivity + 5) / 10;
mousey = (*ev).data3 * (mouseSensitivity + 5) / 10;
1
}
ev_joystick => {
*joybuttons.offset(0) = data1 & 1;
*joybuttons.offset(1) = data1 & 2;
*joybuttons.offset(2) = data1 & 4;
*joybuttons.offset(3) = data1 & 8;
joyxmove = (*ev).data2;
joyymove = (*ev).data3;
1
}
_ => 0,
}
}
}
// --- G_Ticker ---
static mut TURBOMESSAGE: [c_char; 80] = [0; 80];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_Ticker() {
unsafe {
let in_game_ptr = (&raw const playeringame) as *const c_int;
let players_base = (&raw mut players) as *mut player_t;
for i in 0..NUM_PLAYERS {
if *in_game_ptr.add(i) != 0 && (*players_base.add(i)).playerstate == (PST_REBORN as c_int) {
G_DoReborn(i as c_int);
}
}
while gameaction != ga_nothing {
match gameaction {
ga_loadlevel => G_DoLoadLevel(),
ga_newgame => G_DoNewGame(),
ga_loadgame => G_DoLoadGame(),
ga_savegame => G_DoSaveGame(),
ga_playdemo => G_DoPlayDemo(),
ga_completed => G_DoCompleted(),
ga_victory => F_StartFinale(),
ga_worlddone => G_DoWorldDone(),
ga_screenshot => {
M_ScreenShot();
gameaction = ga_nothing;
}
_ => {}
}
}
let buf = ((gametic / ticdup) as usize) % BACKUP_TICS;
let netcmds_base = (&raw const netcmds) as *const [ticcmd_t; BACKUP_TICS];
for i in 0..NUM_PLAYERS {
if *in_game_ptr.add(i) != 0 {
let p = players_base.add(i);
let cmd = (&raw mut (*p).cmd) as *mut ticcmd_t;
let src_cmd = &raw const (*netcmds_base.add(i))[buf];
ptr::copy_nonoverlapping(src_cmd, cmd, 1);
if demoplayback != 0 {
G_ReadDemoTiccmd(cmd);
}
if demorecording != 0 {
G_WriteDemoTiccmd(cmd);
}
if (*cmd).forwardmove > TURBOTHRESHOLD
&& (gametic & 31) == 0
&& ((gametic >> 5) & 3) == (i as c_int)
{
let pnames_ptr = (&raw const player_names) as *const *mut c_char;
let pname = CStr::from_ptr(*pnames_ptr.add(i)).to_str().unwrap_or("Player");
let msg = format!("{} is turbo!\0", pname);
let dst = (&raw mut TURBOMESSAGE) as *mut c_char;
let len = msg.len().min(79);
ptr::copy_nonoverlapping(msg.as_ptr() as *const c_char, dst, len);
(*players_base.add(consoleplayer as usize)).message = dst;
}
if netgame != 0 && netdemo == 0 && (gametic % ticdup) == 0 {
if gametic > (BACKUPTICS as c_int)
&& consistancy[i][buf] != (*cmd).consistancy
{
doom_error(&format!(
"consistency failure ({} should be {})",
(*cmd).consistancy, consistancy[i][buf]
));
}
if !(*p).mo.is_null() {
consistancy[i][buf] = (*(*p).mo).x as c_short;
} else {
consistancy[i][buf] = rndindex as c_short;
}
}
}
}
for i in 0..NUM_PLAYERS {
if *in_game_ptr.add(i) != 0 {
let btns = (*((&raw const (*players_base.add(i)).cmd) as *const ticcmd_t)).buttons as u32;
if (btns & BT_SPECIAL) != 0 {
match btns & BT_SPECIALMASK {
BTS_PAUSE => {
paused ^= 1;
if paused != 0 {
S_PauseSound();
} else {
S_ResumeSound();
}
}
BTS_SAVEGAME => {
let desc_ptr = (&raw mut savedescription) as *mut c_char;
if *desc_ptr == 0 {
let net_str = b"NET GAME\0";
ptr::copy_nonoverlapping(
net_str.as_ptr() as *const c_char,
desc_ptr,
net_str.len(),
);
}
savegameslot = ((btns & BTS_SAVEMASK) >> BTS_SAVESHIFT) as c_int;
gameaction = ga_savegame;
}
_ => {}
}
}
}
}
match gamestate {
GS_LEVEL => {
P_Ticker();
ST_Ticker();
AM_Ticker();
HU_Ticker();
}
GS_INTERMISSION => WI_Ticker(),
GS_FINALE => F_Ticker(),
GS_DEMOSCREEN => D_PageTicker(),
_ => {}
}
}
}
// --- Player Init / Finish / Reborn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_InitPlayer(player: c_int) {
unsafe {
G_PlayerReborn(player);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_PlayerFinishLevel(player: c_int) {
unsafe {
let players_base = (&raw mut players) as *mut player_t;
let p = players_base.add(player as usize);
ptr::write_bytes((&raw mut (*p).powers) as *mut c_int, 0, NUMPOWERS as usize);
ptr::write_bytes((&raw mut (*p).cards) as *mut c_int, 0, NUMCARDS as usize);
(*(*p).mo).flags &= !(MF_SHADOW as c_int);
(*p).extralight = 0;
(*p).fixedcolormap = 0;
(*p).damagecount = 0;
(*p).bonuscount = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_PlayerReborn(player: c_int) {
unsafe {
let players_base = (&raw mut players) as *mut player_t;
let p = players_base.add(player as usize);
let mut frags = [0 as c_int; NUM_PLAYERS];
ptr::copy_nonoverlapping((&raw const (*p).frags) as *const c_int, frags.as_mut_ptr(), NUM_PLAYERS);
let killcount = (*p).killcount;
let itemcount = (*p).itemcount;
let secretcount = (*p).secretcount;
ptr::write_bytes(p, 0, 1);
ptr::copy_nonoverlapping(frags.as_ptr(), (&raw mut (*p).frags) as *mut c_int, NUM_PLAYERS);
(*p).killcount = killcount;
(*p).itemcount = itemcount;
(*p).secretcount = secretcount;
(*p).usedown = 1;
(*p).attackdown = 1;
(*p).playerstate = (PST_LIVE as c_int);
(*p).health = MAXHEALTH as c_int;
(*p).readyweapon = wp_pistol as c_int;
(*p).pendingweapon = wp_pistol as c_int;
(*p).weaponowned[wp_fist as usize] = 1;
(*p).weaponowned[wp_pistol as usize] = 1;
(*p).ammo[am_clip as usize] = 50;
let max_ptr = (&raw const maxammo) as *const c_int;
for i in 0..NUM_AMMO {
(*p).maxammo[i] = *max_ptr.add(i);
}
}
}
// --- Spawn & Deathmatch ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_CheckSpot(playernum: c_int, mthing: *mut mapthing_t) -> c_int {
unsafe {
let players_base = (&raw const players) as *const player_t;
let p_cur = players_base.add(playernum as usize);
if (*p_cur).mo.is_null() {
for i in 0..(playernum as usize) {
let mo_i = (*players_base.add(i)).mo;
if (*mo_i).x == (((*mthing).x as c_int) << FRACBITS)
&& (*mo_i).y == (((*mthing).y as c_int) << FRACBITS)
{
return 0;
}
}
return 1;
}
let x = ((*mthing).x as c_int) << FRACBITS;
let y = ((*mthing).y as c_int) << FRACBITS;
if P_CheckPosition((*p_cur).mo as *mut _, x, y) == 0 {
return 0;
}
let bq_ptr = (&raw mut bodyque) as *mut *mut mobj_t;
if bodyqueslot >= (BODYQUESIZE as c_int) {
P_RemoveMobj(*bq_ptr.add((bodyqueslot as usize) % BODYQUESIZE) as *mut _);
}
*bq_ptr.add((bodyqueslot as usize) % BODYQUESIZE) = (*p_cur).mo as *mut mobj_t;
bodyqueslot += 1;
let ss = R_PointInSubsector(x, y);
let an = (ANG45.wrapping_mul((((*mthing).angle as c_int) / 45) as u32)) >> ANGLETOFINESHIFT;
let cos_v = *finesine.as_ptr().add((an as usize) + ((FINEANGLES as usize) / 4));
let sin_v = *finesine.as_ptr().add(an as usize);
let mo = P_SpawnMobj(
x + 20 * cos_v,
y + 20 * sin_v,
(*(*ss).sector).floorheight,
MT_TFOG,
);
if (*players_base.add(consoleplayer as usize)).viewz != 1 {
S_StartSound(mo as *mut c_void, sfx_telept as c_int);
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DeathMatchSpawnPlayer(playernum: c_int) {
unsafe {
let starts = (&raw mut deathmatchstarts) as *mut mapthing_t;
let selections = deathmatch_p.offset_from(starts as *const _) as c_int;
if selections < 4 {
doom_error(&format!("Only {} deathmatch spots, 4 required", selections));
}
for _ in 0..20 {
let i = (p_random() % selections) as usize;
let spot = starts.add(i);
if G_CheckSpot(playernum, spot) != 0 {
(*spot).type_ = (playernum + 1) as c_short;
P_SpawnPlayer(spot);
return;
}
}
let pstarts = (&raw mut playerstarts) as *mut mapthing_t;
P_SpawnPlayer(pstarts.add(playernum as usize));
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoReborn(playernum: c_int) {
unsafe {
if netgame == 0 {
gameaction = ga_loadlevel;
} else {
let players_base = (&raw mut players) as *mut player_t;
(*(*players_base.add(playernum as usize)).mo).player = ptr::null_mut();
if deathmatch != 0 {
G_DeathMatchSpawnPlayer(playernum);
return;
}
let pstarts = (&raw mut playerstarts) as *mut mapthing_t;
if G_CheckSpot(playernum, pstarts.add(playernum as usize)) != 0 {
P_SpawnPlayer(pstarts.add(playernum as usize));
return;
}
for i in 0..NUM_PLAYERS {
let spot = pstarts.add(i);
if G_CheckSpot(playernum, spot) != 0 {
(*spot).type_ = (playernum + 1) as c_short;
P_SpawnPlayer(spot);
(*spot).type_ = (i + 1) as c_short;
return;
}
}
P_SpawnPlayer(pstarts.add(playernum as usize));
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_ScreenShot() {
unsafe {
gameaction = ga_screenshot;
}
}
// --- Par Times & Level Completion ---
#[unsafe(no_mangle)]
pub static mut pars: [[c_int; 10]; 4] = [
    [0; 10],
    [0, 30, 75, 120, 90, 165, 180, 180, 30, 165],
    [0, 90, 90, 90, 120, 90, 360, 240, 30, 170],
    [0, 90, 45, 90, 150, 90, 90, 165, 30, 135],
];

#[unsafe(no_mangle)]
pub static mut cpars: [c_int; 32] = [
30, 90, 120, 120, 90, 150, 120, 120, 270, 90,
210, 150, 150, 150, 210, 150, 420, 150, 210, 150,
240, 150, 180, 150, 150, 300, 330, 420, 300, 180,
120, 30,
];
#[unsafe(no_mangle)]
pub static mut secretexit: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_ExitLevel() {
unsafe {
secretexit = 0;
gameaction = ga_completed;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_SecretExitLevel() {
unsafe {
if gamemode == commercial && W_CheckNumForName(b"map31\0".as_ptr() as *mut c_char) < 0 {
secretexit = 0;
} else {
secretexit = 1;
}
gameaction = ga_completed;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoCompleted() {
unsafe {
gameaction = ga_nothing;
let in_game_ptr = (&raw const playeringame) as *const c_int;
let players_base = (&raw mut players) as *mut player_t;
for i in 0..NUM_PLAYERS {
if *in_game_ptr.add(i) != 0 {
G_PlayerFinishLevel(i as c_int);
}
}
if automapactive != 0 {
AM_Stop();
}
if gamemode != commercial {
match gamemap {
8 => {
gameaction = ga_victory;
return;
}
9 => {
for i in 0..NUM_PLAYERS {
(*players_base.add(i)).didsecret = 1;
}
}
_ => {}
}
}
wminfo.didsecret = (*players_base.add(consoleplayer as usize)).didsecret;
wminfo.epsd = gameepisode - 1;
wminfo.last = gamemap - 1;
if gamemode == commercial {
if secretexit != 0 {
match gamemap {
15 => wminfo.next = 30,
31 => wminfo.next = 31,
_ => {}
}
} else {
match gamemap {
31 | 32 => wminfo.next = 15,
_ => wminfo.next = gamemap,
}
}
} else if secretexit != 0 {
wminfo.next = 8;
} else if gamemap == 9 {
wminfo.next = match gameepisode {
1 => 3,
2 => 5,
3 => 6,
4 => 2,
_ => gamemap,
};
} else {
wminfo.next = gamemap;
}
wminfo.maxkills = totalkills;
wminfo.maxitems = totalitems;
wminfo.maxsecret = totalsecret;
wminfo.maxfrags = 0;
if gamemode == commercial {
wminfo.partime = 35 * cpars[(gamemap - 1).clamp(0, 31) as usize];
} else {
let ep = gameepisode.clamp(0, 3) as usize;
let mp = gamemap.clamp(0, 9) as usize;
wminfo.partime = 35 * pars[ep][mp];
}
wminfo.pnum = consoleplayer;
for i in 0..NUM_PLAYERS {
let pl = &*players_base.add(i);
wminfo.plyr[i].in_ = *in_game_ptr.add(i);
wminfo.plyr[i].skills = pl.killcount;
wminfo.plyr[i].sitems = pl.itemcount;
wminfo.plyr[i].ssecret = pl.secretcount;
wminfo.plyr[i].stime = leveltime;
ptr::copy_nonoverlapping(
(&raw const pl.frags) as *const c_int,
(&raw mut wminfo.plyr[i].frags) as *mut c_int,
NUM_PLAYERS,
);
}
gamestate = GS_INTERMISSION;
viewactive = 0;
automapactive = 0;
if !statcopy.is_null() {
ptr::copy_nonoverlapping(&raw const wminfo, statcopy as *mut wbstartstruct_t, 1);
}
WI_Start((&raw mut wminfo) as *mut _);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_WorldDone() {
unsafe {
gameaction = ga_worlddone;
let players_base = (&raw mut players) as *mut player_t;
if secretexit != 0 {
(*players_base.add(consoleplayer as usize)).didsecret = 1;
}
if gamemode == commercial {
match gamemap {
15 | 31 => {
if secretexit != 0 {
F_StartFinale();
}
}
6 | 11 | 20 | 30 => {
F_StartFinale();
}
_ => {}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoWorldDone() {
unsafe {
gamestate = GS_LEVEL;
gamemap = wminfo.next + 1;
G_DoLoadLevel();
gameaction = ga_nothing;
viewactive = 1;
}
}
// --- Save / Load Game ---
#[unsafe(no_mangle)]
pub static mut savename: [c_char; 256] = [0; 256];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_LoadGame(name: *mut c_char) {
unsafe {
let src = CStr::from_ptr(name).to_bytes_with_nul();
let dst = (&raw mut savename) as *mut c_char;
let len = src.len().min(256);
ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst, len);
gameaction = ga_loadgame;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoLoadGame() {
unsafe {
gameaction = ga_nothing;
let sn_ptr = (&raw mut savename) as *mut c_char;
M_ReadFile(sn_ptr, &raw mut savebuffer);
save_p = savebuffer.add(SAVESTRINGSIZE);
let vcheck = format!("version {}\0", VERSION);
let save_v = CStr::from_ptr(save_p as *const c_char);
if save_v.to_bytes_with_nul() != vcheck.as_bytes() {
return;
}
save_p = save_p.add(VERSIONSIZE);
gameskill = (*save_p) as skill_t;
save_p = save_p.add(1);
gameepisode = (*save_p) as c_int;
save_p = save_p.add(1);
gamemap = (*save_p) as c_int;
save_p = save_p.add(1);
let in_game_ptr = (&raw mut playeringame) as *mut c_int;
for i in 0..NUM_PLAYERS {
*in_game_ptr.add(i) = (*save_p) as c_int;
save_p = save_p.add(1);
}
G_InitNew(gameskill, gameepisode, gamemap);
let a = (*save_p) as c_int;
save_p = save_p.add(1);
let b = (*save_p) as c_int;
save_p = save_p.add(1);
let c = (*save_p) as c_int;
save_p = save_p.add(1);
leveltime = (a << 16) + (b << 8) + c;
P_UnArchivePlayers();
P_UnArchiveWorld();
P_UnArchiveThinkers();
P_UnArchiveSpecials();
if *save_p != 0x1d {
doom_error("Bad savegame");
}
Z_Free(savebuffer as *mut c_void);
if setsizeneeded != 0 {
R_ExecuteSetViewSize();
}
R_FillBackScreen();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_SaveGame(slot: c_int, description: *mut c_char) {
unsafe {
savegameslot = slot;
let src = CStr::from_ptr(description).to_bytes_with_nul();
let dst = (&raw mut savedescription) as *mut c_char;
let len = src.len().min(32);
ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst, len);
sendsave = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoSaveGame() {
unsafe {
let name = if m_check_parm("-cdrom") != 0 {
format!("c:\\doomdata\\{}{}.dsg\0", SAVEGAMENAME_STR, savegameslot)
} else {
format!("{}{}.dsg\0", SAVEGAMENAME_STR, savegameslot)
};
let scr_ptr = (&raw const screens) as *const *mut u8;
savebuffer = (*scr_ptr.add(1)).add(0x4000);
save_p = savebuffer;
let desc_ptr = (&raw mut savedescription) as *mut c_char;
ptr::copy_nonoverlapping(desc_ptr as *const u8, save_p, SAVESTRINGSIZE);
save_p = save_p.add(SAVESTRINGSIZE);
let mut name2 = [0u8; VERSIONSIZE];
let v_str = format!("version {}", VERSION);
let v_bytes = v_str.as_bytes();
let v_len = v_bytes.len().min(VERSIONSIZE - 1);
ptr::copy_nonoverlapping(v_bytes.as_ptr(), name2.as_mut_ptr(), v_len);
ptr::copy_nonoverlapping(name2.as_ptr(), save_p, VERSIONSIZE);
save_p = save_p.add(VERSIONSIZE);
*save_p = gameskill as u8;
save_p = save_p.add(1);
*save_p = gameepisode as u8;
save_p = save_p.add(1);
*save_p = gamemap as u8;
save_p = save_p.add(1);
let in_game_ptr = (&raw const playeringame) as *const c_int;
for i in 0..NUM_PLAYERS {
*save_p = (*in_game_ptr.add(i)) as u8;
save_p = save_p.add(1);
}
*save_p = (leveltime >> 16) as u8;
save_p = save_p.add(1);
*save_p = (leveltime >> 8) as u8;
save_p = save_p.add(1);
*save_p = leveltime as u8;
save_p = save_p.add(1);
P_ArchivePlayers();
P_ArchiveWorld();
P_ArchiveThinkers();
P_ArchiveSpecials();
*save_p = 0x1d;
save_p = save_p.add(1);
let length = save_p.offset_from(savebuffer) as usize;
if length > SAVEGAMESIZE {
doom_error("Savegame buffer overrun");
}
M_WriteFile(name.as_ptr() as *mut c_char, savebuffer as *mut c_void, length as c_int);
gameaction = ga_nothing;
*desc_ptr = 0;
let players_base = (&raw mut players) as *mut player_t;
(*players_base.add(consoleplayer as usize)).message = GGSAVED_STR.as_ptr() as *mut c_char;
R_FillBackScreen();
}
}
// --- G_InitNew ---
#[unsafe(no_mangle)]
pub static mut d_skill: skill_t = sk_medium;
#[unsafe(no_mangle)]
pub static mut d_episode: c_int = 1;
#[unsafe(no_mangle)]
pub static mut d_map: c_int = 1;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DeferedInitNew(skill: skill_t, episode: c_int, map: c_int) {
unsafe {
d_skill = skill;
d_episode = episode;
d_map = map;
gameaction = ga_newgame;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoNewGame() {
unsafe {
demoplayback = 0;
netdemo = 0;
netgame = 0;
deathmatch = 0;
let in_game_ptr = (&raw mut playeringame) as *mut c_int;
*in_game_ptr.add(1) = 0;
*in_game_ptr.add(2) = 0;
*in_game_ptr.add(3) = 0;
respawnparm = 0;
fastparm = 0;
nomonsters = 0;
consoleplayer = 0;
G_InitNew(d_skill, d_episode, d_map);
gameaction = ga_nothing;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_InitNew(mut skill: skill_t, mut episode: c_int, mut map: c_int) {
unsafe {
if paused != 0 {
paused = 0;
S_ResumeSound();
}
if skill > sk_nightmare {
skill = sk_nightmare;
}
if episode < 1 {
episode = 1;
}
if gamemode == retail {
if episode > 4 {
episode = 4;
}
} else if gamemode == shareware {
if episode > 1 {
episode = 1;
}
} else if episode > 3 {
episode = 3;
}
if map < 1 {
map = 1;
}
if map > 9 && gamemode != commercial {
map = 9;
}
m_clear_random();
respawnmonsters = (skill == sk_nightmare || respawnparm != 0) as c_int;
let states_base = (&raw mut states) as *mut state_t;
let info_base = (&raw mut mobjinfo) as *mut mobjinfo_t;
if fastparm != 0 || (skill == sk_nightmare && gameskill != sk_nightmare) {
for i in (S_SARG_RUN1 as usize)..=(S_SARG_PAIN2 as usize) {
(*states_base.add(i)).tics >>= 1;
}
(*info_base.add(MT_BRUISERSHOT as usize)).speed = 20 * FRAC_UNIT;
(*info_base.add(MT_HEADSHOT as usize)).speed = 20 * FRAC_UNIT;
(*info_base.add(MT_TROOPSHOT as usize)).speed = 20 * FRAC_UNIT;
} else if skill != sk_nightmare && gameskill == sk_nightmare {
for i in (S_SARG_RUN1 as usize)..=(S_SARG_PAIN2 as usize) {
(*states_base.add(i)).tics <<= 1;
}
(*info_base.add(MT_BRUISERSHOT as usize)).speed = 15 * FRAC_UNIT;
(*info_base.add(MT_HEADSHOT as usize)).speed = 10 * FRAC_UNIT;
(*info_base.add(MT_TROOPSHOT as usize)).speed = 10 * FRAC_UNIT;
}
let players_base = (&raw mut players) as *mut player_t;
for i in 0..NUM_PLAYERS {
(*players_base.add(i)).playerstate = (PST_REBORN as c_int);
}
usergame = 1;
paused = 0;
demoplayback = 0;
automapactive = 0;
viewactive = 1;
gameepisode = episode;
gamemap = map;
gameskill = skill;
if gamemode == commercial {
skytexture = R_TextureNumForName(b"SKY3\0".as_ptr() as *mut c_char);
if gamemap < 12 {
skytexture = R_TextureNumForName(b"SKY1\0".as_ptr() as *mut c_char);
} else if gamemap < 21 {
skytexture = R_TextureNumForName(b"SKY2\0".as_ptr() as *mut c_char);
}
} else {
let sky_lump = match episode {
1 => b"SKY1\0".as_ptr(),
2 => b"SKY2\0".as_ptr(),
3 => b"SKY3\0".as_ptr(),
4 => b"SKY4\0".as_ptr(),
_ => b"SKY1\0".as_ptr(),
};
skytexture = R_TextureNumForName(sky_lump as *mut c_char);
}
G_DoLoadLevel();
}
}
// --- Demo Recording & Playback ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_ReadDemoTiccmd(cmd: *mut ticcmd_t) {
unsafe {
if *demo_p == DEMOMARKER {
G_CheckDemoStatus();
return;
}
(*cmd).forwardmove = (*demo_p) as i8;
demo_p = demo_p.add(1);
(*cmd).sidemove = (*demo_p) as i8;
demo_p = demo_p.add(1);
(*cmd).angleturn = ((*demo_p) as c_short) << 8;
demo_p = demo_p.add(1);
(*cmd).buttons = *demo_p;
demo_p = demo_p.add(1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_WriteDemoTiccmd(cmd: *mut ticcmd_t) {
unsafe {
let kd = (&raw const gamekeydown) as *const c_int;
if *kd.add(b'q' as usize) != 0 {
G_CheckDemoStatus();
}
*demo_p = (*cmd).forwardmove as u8;
demo_p = demo_p.add(1);
*demo_p = (*cmd).sidemove as u8;
demo_p = demo_p.add(1);
*demo_p = (((*cmd).angleturn + 128) >> 8) as u8;
demo_p = demo_p.add(1);
*demo_p = (*cmd).buttons;
demo_p = demo_p.add(1);
demo_p = demo_p.sub(4);
if demo_p > demoend.sub(16) {
G_CheckDemoStatus();
return;
}
G_ReadDemoTiccmd(cmd);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_RecordDemo(name: *mut c_char) {
unsafe {
usergame = 0;
let src = CStr::from_ptr(name).to_str().unwrap_or("demo");
let full = format!("{}.lmp\0", src);
let dst = (&raw mut demoname) as *mut c_char;
let len = full.len().min(32);
ptr::copy_nonoverlapping(full.as_ptr() as *const c_char, dst, len);
let mut maxsize: c_int = 0x20000;
let i = m_check_parm("-maxdemo");
if i != 0 && i < myargc - 1 {
let arg_ptr = *myargv.offset((i + 1) as isize);
let s = CStr::from_ptr(arg_ptr).to_str().unwrap_or("128");
maxsize = s.parse::<c_int>().unwrap_or(128) * 1024;
}
demobuffer = Z_Malloc(maxsize, PU_STATIC, ptr::null_mut()) as *mut u8;
demoend = demobuffer.add(maxsize as usize);
demorecording = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_BeginRecording() {
unsafe {
demo_p = demobuffer;
*demo_p = VERSION as u8;
demo_p = demo_p.add(1);
*demo_p = gameskill as u8;
demo_p = demo_p.add(1);
*demo_p = gameepisode as u8;
demo_p = demo_p.add(1);
*demo_p = gamemap as u8;
demo_p = demo_p.add(1);
*demo_p = deathmatch as u8;
demo_p = demo_p.add(1);
*demo_p = respawnparm as u8;
demo_p = demo_p.add(1);
*demo_p = fastparm as u8;
demo_p = demo_p.add(1);
*demo_p = nomonsters as u8;
demo_p = demo_p.add(1);
*demo_p = consoleplayer as u8;
demo_p = demo_p.add(1);
let in_game_ptr = (&raw const playeringame) as *const c_int;
for i in 0..NUM_PLAYERS {
*demo_p = (*in_game_ptr.add(i)) as u8;
demo_p = demo_p.add(1);
}
}
}
#[unsafe(no_mangle)]
pub static mut defdemoname: *mut c_char = ptr::null_mut();
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DeferedPlayDemo(name: *mut c_char) {
unsafe {
defdemoname = name;
gameaction = ga_playdemo;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_DoPlayDemo() {
unsafe {
gameaction = ga_nothing;
demobuffer = W_CacheLumpName(defdemoname, PU_STATIC) as *mut u8;
demo_p = demobuffer;
let ver = *demo_p;
demo_p = demo_p.add(1);
if (ver as u32) != VERSION {
eprintln!("Demo is from a different game version!");
gameaction = ga_nothing;
return;
}
let skill = (*demo_p) as skill_t;
demo_p = demo_p.add(1);
let episode = (*demo_p) as c_int;
demo_p = demo_p.add(1);
let map = (*demo_p) as c_int;
demo_p = demo_p.add(1);
deathmatch = (*demo_p) as c_int;
demo_p = demo_p.add(1);
respawnparm = (*demo_p) as c_int;
demo_p = demo_p.add(1);
fastparm = (*demo_p) as c_int;
demo_p = demo_p.add(1);
nomonsters = (*demo_p) as c_int;
demo_p = demo_p.add(1);
consoleplayer = (*demo_p) as c_int;
demo_p = demo_p.add(1);
let in_game_ptr = (&raw mut playeringame) as *mut c_int;
for i in 0..NUM_PLAYERS {
*in_game_ptr.add(i) = (*demo_p) as c_int;
demo_p = demo_p.add(1);
}
if *in_game_ptr.add(1) != 0 {
netgame = 1;
netdemo = 1;
}
precache = 0;
G_InitNew(skill, episode, map);
precache = 1;
usergame = 0;
demoplayback = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_TimeDemo(name: *mut c_char) {
unsafe {
nodrawers = m_check_parm("-nodraw");
noblit = m_check_parm("-noblit");
timingdemo = 1;
singletics = 1;
defdemoname = name;
gameaction = ga_playdemo;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn G_CheckDemoStatus() -> c_int {
unsafe {
if timingdemo != 0 {
let endtime = I_GetTime();
doom_error(&format!(
"timed {} gametics in {} realtics",
gametic,
endtime - starttime
));
}
if demoplayback != 0 {
if singledemo != 0 {
I_Quit();
}
Z_ChangeTag(demobuffer as *mut c_void, PU_CACHE);
demoplayback = 0;
netdemo = 0;
netgame = 0;
deathmatch = 0;
let in_game_ptr = (&raw mut playeringame) as *mut c_int;
*in_game_ptr.add(1) = 0;
*in_game_ptr.add(2) = 0;
*in_game_ptr.add(3) = 0;
respawnparm = 0;
fastparm = 0;
nomonsters = 0;
consoleplayer = 0;
D_AdvanceDemo();
return 1;
}
if demorecording != 0 {
*demo_p = DEMOMARKER;
demo_p = demo_p.add(1);
let len = demo_p.offset_from(demobuffer) as c_int;
let dn_ptr = (&raw mut demoname) as *mut c_char;
M_WriteFile(dn_ptr, demobuffer as *mut c_void, len);
Z_Free(demobuffer as *mut c_void);
demorecording = 0;
let dn_str = CStr::from_ptr(dn_ptr).to_str().unwrap_or("");
doom_error(&format!("Demo {} recorded", dn_str));
}
0
}
}