#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::path::Path;
use std::ptr;

use crate::am_map::{AM_Drawer, automapactive};
use crate::d_net::{D_CheckNetGame, NetUpdate, TryRunTics, maketic, netcmds};
use crate::doomstat::{gamemode, language, modifiedgame};
use crate::f_finale::F_Drawer;
use crate::f_wipe::{wipe_EndScreen, wipe_ScreenWipe, wipe_StartScreen};
const wipe_Melt: c_int = 1;
use crate::g_game::{
    G_BeginRecording, G_BuildTiccmd, G_DeferedPlayDemo, G_InitNew, G_LoadGame, G_RecordDemo,
    G_Responder, G_Ticker, G_TimeDemo, consoleplayer, deathmatch, demorecording, displayplayer,
    forwardmove, gameaction, gamestate, gametic, netgame, nodrawers, paused, players, sidemove,
    singledemo, statcopy, usergame, viewactive,
};
use crate::hu_stuff::{HU_Drawer, HU_Erase, HU_Init};
use crate::m_argv::{m_check_parm, myargc, myargv};
use crate::m_menu::{M_Drawer, M_Init, M_Responder, M_Ticker, inhelpscreens, menuactive};
use crate::m_misc::M_LoadDefaults;
use crate::p_setup::P_Init;
use crate::r_draw::{R_DrawViewBorder, R_FillBackScreen, scaledviewwidth, viewheight, viewwindowx, viewwindowy};
use crate::r_main::{R_ExecuteSetViewSize, R_Init, R_RenderPlayerView, setsizeneeded};
use crate::s_sound::{
    S_Init, S_StartMusic, S_UpdateSounds, snd_MusicVolume, snd_SfxVolume,
};
use crate::st_stuff::{ST_Drawer, ST_Init};
use crate::v_video::{V_DrawPatch, V_DrawPatchDirect, V_Init, doom_error};
use crate::w_wad::{W_CacheLumpName, W_CheckNumForName, W_InitMultipleFiles};
use crate::wi_stuff::WI_Drawer;
use crate::z_zone::{PU_CACHE, Z_Init};

const MAX_WADFILES: usize = MAXWADFILES as usize;
const MAX_EVENTS: usize = MAXEVENTS as usize;
const BACKUP_TICS: usize = BACKUPTICS as usize;
const VERSION: c_int = 109;
const SAVEGAMENAME_STR: &str = "doomsav";
const mus_intro: c_int = 29;
const mus_dm2ttl: c_int = 65;

unsafe extern "C" {
    fn I_Init();
    fn I_InitGraphics();
    fn I_StartFrame();
    fn I_StartTic();
    fn I_UpdateNoBlit();
    fn I_FinishUpdate();
    fn I_SetPalette(palette: *mut u8);
    fn I_GetTime() -> c_int;
    fn I_UpdateSound();
    fn I_SubmitSound();
}

// --- Глобальные переменные d_main.c ---
#[unsafe(no_mangle)]
pub static mut wadfiles: [*mut c_char; MAX_WADFILES] = [ptr::null_mut(); MAX_WADFILES];

#[unsafe(no_mangle)]
pub static mut devparm: c_int = 0;

#[unsafe(no_mangle)]
pub static mut nomonsters: c_int = 0;

#[unsafe(no_mangle)]
pub static mut respawnparm: c_int = 0;

#[unsafe(no_mangle)]
pub static mut fastparm: c_int = 0;

#[unsafe(no_mangle)]
pub static mut drone: c_int = 0;

#[unsafe(no_mangle)]
pub static mut singletics: c_int = 0;

#[unsafe(no_mangle)]
pub static mut startskill: skill_t = sk_medium;

#[unsafe(no_mangle)]
pub static mut startepisode: c_int = 1;

#[unsafe(no_mangle)]
pub static mut startmap: c_int = 1;

#[unsafe(no_mangle)]
pub static mut autostart: c_int = 0;

#[unsafe(no_mangle)]
pub static mut debugfile: *mut c_void = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut advancedemo: c_int = 0;

#[unsafe(no_mangle)]
pub static mut wadfile: [c_char; 1024] = [0; 1024];

#[unsafe(no_mangle)]
pub static mut mapdir: [c_char; 1024] = [0; 1024];

#[unsafe(no_mangle)]
pub static mut basedefault: [c_char; 1024] = [0; 1024];

#[unsafe(no_mangle)]
pub static mut events: [event_t; MAX_EVENTS] = [event_t {
    type_: ev_keydown,
    data1: 0,
    data2: 0,
    data3: 0,
}; MAX_EVENTS];

#[unsafe(no_mangle)]
pub static mut eventhead: c_int = 0;

#[unsafe(no_mangle)]
pub static mut eventtail: c_int = 0;

#[unsafe(no_mangle)]
pub static mut wipegamestate: gamestate_t = GS_DEMOSCREEN;

#[unsafe(no_mangle)]
pub static mut demosequence: c_int = 0;

#[unsafe(no_mangle)]
pub static mut pagetic: c_int = 0;

#[unsafe(no_mangle)]
pub static mut pagename: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut title: [c_char; 128] = [0; 128];

// --- События ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_PostEvent(ev: *mut event_t) {
    unsafe {
        let ev_base = (&raw mut events) as *mut event_t;
        ptr::copy_nonoverlapping(ev, ev_base.add(eventhead as usize), 1);
        eventhead = (eventhead + 1) & ((MAX_EVENTS as c_int) - 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_ProcessEvents() {
    unsafe {
        if gamemode == commercial && W_CheckNumForName(b"map01\0".as_ptr() as *mut c_char) < 0 {
            return;
        }

        let ev_base = (&raw mut events) as *mut event_t;
        while eventtail != eventhead {
            let ev = ev_base.add(eventtail as usize);
            if M_Responder(ev as *mut _) == 0 {
                G_Responder(ev as *mut _);
            }
            eventtail = (eventtail + 1) & ((MAX_EVENTS as c_int) - 1);
        }
    }
}

// --- Отрисовка кадра и wipe ---
static mut VIEW_ACTIVE_STATE: c_int = 0;
static mut MENU_ACTIVE_STATE: c_int = 0;
static mut IN_HELP_SCREENS_STATE: c_int = 0;
static mut FULLSCREEN_STATE: c_int = 0;
static mut OLD_GAMESTATE: gamestate_t = -1i32 as gamestate_t;
static mut BORDER_DRAW_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_Display() {
    unsafe {
        if nodrawers != 0 {
            return;
        }

        let mut redrawsbar = false;
        let scr_w = SCREENWIDTH as c_int;
        let scr_h = SCREENHEIGHT as c_int;

        if setsizeneeded != 0 {
            R_ExecuteSetViewSize();
            OLD_GAMESTATE = -1i32 as gamestate_t;
            BORDER_DRAW_COUNT = 3;
        }

        let wipe = if gamestate != wipegamestate {
            wipe_StartScreen(0, 0, scr_w, scr_h);
            true
        } else {
            false
        };

        if gamestate == GS_LEVEL && gametic != 0 {
            HU_Erase();
        }

        match gamestate {
            GS_LEVEL => {
                if gametic != 0 {
                    if automapactive != 0 {
                        AM_Drawer();
                    }
                    if wipe || (viewheight != 200 && FULLSCREEN_STATE != 0) {
                        redrawsbar = true;
                    }
                    if IN_HELP_SCREENS_STATE != 0 && inhelpscreens == 0 {
                        redrawsbar = true;
                    }
                    ST_Drawer((viewheight == 200) as c_int, redrawsbar as c_int);
                    FULLSCREEN_STATE = (viewheight == 200) as c_int;
                }
            }
            GS_INTERMISSION => WI_Drawer(),
            GS_FINALE => F_Drawer(),
            GS_DEMOSCREEN => D_PageDrawer(),
            _ => {}
        }

        I_UpdateNoBlit();

        if gamestate == GS_LEVEL && automapactive == 0 && gametic != 0 {
            let players_base = (&raw mut players) as *mut player_t;
            R_RenderPlayerView(players_base.add(displayplayer as usize) as *mut _);
        }

        if gamestate == GS_LEVEL && gametic != 0 {
            HU_Drawer();
        }

        if gamestate != OLD_GAMESTATE && gamestate != GS_LEVEL {
            let pal = W_CacheLumpName(b"PLAYPAL\0".as_ptr() as *mut c_char, PU_CACHE) as *mut u8;
            I_SetPalette(pal);
        }

        if gamestate == GS_LEVEL && OLD_GAMESTATE != GS_LEVEL {
            VIEW_ACTIVE_STATE = 0;
            R_FillBackScreen();
        }

        if gamestate == GS_LEVEL && automapactive == 0 && scaledviewwidth != 320 {
            if menuactive != 0 || MENU_ACTIVE_STATE != 0 || VIEW_ACTIVE_STATE == 0 {
                BORDER_DRAW_COUNT = 3;
            }
            if BORDER_DRAW_COUNT != 0 {
                R_DrawViewBorder();
                BORDER_DRAW_COUNT -= 1;
            }
        }

        MENU_ACTIVE_STATE = menuactive;
        VIEW_ACTIVE_STATE = viewactive;
        IN_HELP_SCREENS_STATE = inhelpscreens;
        OLD_GAMESTATE = gamestate;
        wipegamestate = gamestate;

        if paused != 0 {
            let y = if automapactive != 0 { 4 } else { viewwindowy + 4 };
            let pause_patch =
                W_CacheLumpName(b"M_PAUSE\0".as_ptr() as *mut c_char, PU_CACHE) as *mut _;
            V_DrawPatchDirect(viewwindowx + (scaledviewwidth - 68) / 2, y, 0, pause_patch);
        }

        M_Drawer();
        NetUpdate();

        if !wipe {
            I_FinishUpdate();
            return;
        }

        wipe_EndScreen(0, 0, scr_w, scr_h);
        let mut wipestart = I_GetTime() - 1;

        loop {
            let mut tics;
            let mut nowtime;
            loop {
                nowtime = I_GetTime();
                tics = nowtime - wipestart;
                if tics != 0 {
                    break;
                }
            }
            wipestart = nowtime;
            let done = wipe_ScreenWipe(wipe_Melt as c_int, 0, 0, scr_w, scr_h, tics);
            I_UpdateNoBlit();
            M_Drawer();
            I_FinishUpdate();
            if done != 0 {
                break;
            }
        }
    }
}

// --- Главный цикл игры ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_DoomLoop() {
    unsafe {
        if demorecording != 0 {
            G_BeginRecording();
        }

        I_InitGraphics();

        let players_base = (&raw const players) as *const player_t;
        let netcmds_base = (&raw mut netcmds) as *mut [ticcmd_t; BACKUP_TICS];

        loop {
            I_StartFrame();

            if singletics != 0 {
                I_StartTic();
                D_ProcessEvents();
                let slot = (maketic as usize) % BACKUP_TICS;
                let cmd = &raw mut (*netcmds_base.add(consoleplayer as usize))[slot];
                G_BuildTiccmd(cmd as *mut _);
                if advancedemo != 0 {
                    D_DoAdvanceDemo();
                }
                M_Ticker();
                G_Ticker();
                gametic += 1;
maketic += 1;
} else {
TryRunTics();
}
let mo = (*players_base.add(consoleplayer as usize)).mo;
S_UpdateSounds(mo as *mut c_void);
D_Display();
I_UpdateSound();
I_SubmitSound();
}
}
}
// --- Демонстрационная петля ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_PageTicker() {
unsafe {
pagetic -= 1;
if pagetic < 0 {
D_AdvanceDemo();
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_PageDrawer() {
unsafe {
if !pagename.is_null() {
let p = W_CacheLumpName(pagename, PU_CACHE) as *mut _;
V_DrawPatch(0, 0, 0, p);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_AdvanceDemo() {
unsafe {
advancedemo = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_DoAdvanceDemo() {
unsafe {
let players_base = (&raw mut players) as *mut player_t;
(*players_base.add(consoleplayer as usize)).playerstate = PST_LIVE as c_int;
advancedemo = 0;
usergame = 0;
paused = 0;
gameaction = ga_nothing;
let seq_mod = if gamemode == retail { 7 } else { 6 };
demosequence = (demosequence + 1) % seq_mod;
match demosequence {
0 => {
pagetic = if gamemode == commercial { 35 * 11 } else { 170 };
gamestate = GS_DEMOSCREEN;
pagename = b"TITLEPIC\0".as_ptr() as *mut c_char;
if gamemode == commercial {
S_StartMusic(mus_dm2ttl);
} else {
S_StartMusic(mus_intro);
}
}
1 => G_DeferedPlayDemo(b"demo1\0".as_ptr() as *mut c_char),
2 => {
pagetic = 200;
gamestate = GS_DEMOSCREEN;
pagename = b"CREDIT\0".as_ptr() as *mut c_char;
}
3 => G_DeferedPlayDemo(b"demo2\0".as_ptr() as *mut c_char),
4 => {
gamestate = GS_DEMOSCREEN;
if gamemode == commercial {
pagetic = 35 * 11;
pagename = b"TITLEPIC\0".as_ptr() as *mut c_char;
S_StartMusic(mus_dm2ttl);
} else {
pagetic = 200;
pagename = if gamemode == retail {
b"CREDIT\0".as_ptr() as *mut c_char
} else {
b"HELP2\0".as_ptr() as *mut c_char
};
}
}
5 => G_DeferedPlayDemo(b"demo3\0".as_ptr() as *mut c_char),
6 => G_DeferedPlayDemo(b"demo4\0".as_ptr() as *mut c_char),
_ => {}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_StartTitle() {
unsafe {
gameaction = ga_nothing;
demosequence = -1;
D_AdvanceDemo();
}
}
// --- Поиск WAD и разбор параметров командной строки ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_AddFile(file: *mut c_char) {
unsafe {
let wf_base = (&raw mut wadfiles) as *mut *mut c_char;
let mut idx = 0usize;
while idx < MAX_WADFILES && !(*wf_base.add(idx)).is_null() {
idx += 1;
}
if idx >= MAX_WADFILES {
return;
}
let s = CStr::from_ptr(file).to_bytes_with_nul();
let leaked = Box::leak(s.to_vec().into_boxed_slice());
*wf_base.add(idx) = leaked.as_mut_ptr() as *mut c_char;
}
}
fn add_file_str(path: &str) {
if let Ok(cstr) = CString::new(path) {
unsafe {
D_AddFile(cstr.as_ptr() as *mut c_char);
}
}
}
fn set_basedefault(path: &str) {
unsafe {
let dst = (&raw mut basedefault) as *mut c_char;
ptr::write_bytes(dst, 0, 1024);
let bytes = path.as_bytes();
let len = bytes.len().min(1023);
ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, dst, len);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn IdentifyVersion() {
unsafe {
let doomwaddir = env::var("DOOMWADDIR").unwrap_or_else(|_| ".".to_string());
let home = env::var("HOME").unwrap_or_else(|_| {
doom_error("Please set $HOME to your home directory");
});
set_basedefault(&format!("{}/.doomrc", home));
let candidates: [(&str, GameMode_t, Option<Language_t>); 7] = [
("doom2f.wad", commercial, Some(french)),
("doom2.wad", commercial, None),
("plutonia.wad", commercial, None),
("tnt.wad", commercial, None),
("doomu.wad", retail, None),
("doom.wad", registered, None),
("doom1.wad", shareware, None),
];
for (wad_name, mode, lang) in candidates {
let full_path = format!("{}/{}", doomwaddir, wad_name);
if Path::new(&full_path).exists() {
gamemode = mode;
if let Some(l) = lang {
language = l;
println!("French version");
}
add_file_str(&full_path);
return;
}
}
println!("Game mode indeterminate.");
gamemode = indetermined;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FindResponseFile() {
unsafe {
for i in 1..(myargc as usize) {
let arg_ptr = *myargv.add(i);
let arg_bytes = CStr::from_ptr(arg_ptr).to_bytes();
if arg_bytes.first() == Some(&b'@') {
let fname = String::from_utf8_lossy(&arg_bytes[1..]).into_owned();
let content = match fs::read(&fname) {
Ok(c) => c,
Err(_) => {
println!("\nNo such response file!");
std::process::exit(1);
}
};
println!("Found response file {}!", fname);
let mut new_args: Vec<*mut c_char> = Vec::new();
new_args.push(*myargv.add(0));
let mut k = 0usize;
while k < content.len() {
while k < content.len() && (content[k] <= b' ' || content[k] > b'z') {
k += 1;
}
if k >= content.len() {
break;
}
let start = k;
while k < content.len() && content[k] > b' ' && content[k] <= b'z' {
k += 1;
}
let mut token = content[start..k].to_vec();
token.push(0);
let leaked = Box::leak(token.into_boxed_slice());
new_args.push(leaked.as_mut_ptr() as *mut c_char);
}
for k_idx in (i + 1)..(myargc as usize) {
new_args.push(*myargv.add(k_idx));
}
myargc = new_args.len() as c_int;
let leaked_argv = Box::leak(new_args.into_boxed_slice());
myargv = leaked_argv.as_mut_ptr();
println!("{} command-line args:", myargc);
for k_idx in 1..(myargc as usize) {
let s = CStr::from_ptr(*myargv.add(k_idx)).to_string_lossy();
println!("{}", s);
}
break;
}
}
}
}
// --- Главная точка входа движка D_DoomMain ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_DoomMain() {
unsafe {
FindResponseFile();
IdentifyVersion();
modifiedgame = 0;
nomonsters = (m_check_parm("-nomonsters") != 0) as c_int;
respawnparm = (m_check_parm("-respawn") != 0) as c_int;
fastparm = (m_check_parm("-fast") != 0) as c_int;
devparm = (m_check_parm("-devparm") != 0) as c_int;
if m_check_parm("-altdeath") != 0 {
deathmatch = 2;
} else if m_check_parm("-deathmatch") != 0 {
deathmatch = 1;
}
let major = VERSION / 100;
let minor = VERSION % 100;
let banner = match gamemode {
retail => format!("                         The Ultimate DOOM Startup v{}.{}                           ", major, minor),
shareware => format!("                            DOOM Shareware Startup v{}.{}                           ", major, minor),
registered => format!("                            DOOM Registered Startup v{}.{}                           ", major, minor),
commercial => format!("                         DOOM 2: Hell on Earth v{}.{}                           ", major, minor),
_ => format!("                     Public DOOM - v{}.{}                           ", major, minor),
};
println!("{}", banner);
let title_dst = (&raw mut title) as *mut c_char;
let b_bytes = banner.as_bytes();
let b_len = b_bytes.len().min(127);
ptr::copy_nonoverlapping(b_bytes.as_ptr() as *const c_char, title_dst, b_len);
*title_dst.add(b_len) = 0;
if devparm != 0 {
print!("{}", CStr::from_bytes_with_nul_unchecked(D_DEVSTR).to_string_lossy());
}
if m_check_parm("-cdrom") != 0 {
print!("{}", CStr::from_bytes_with_nul_unchecked(D_CDROM).to_string_lossy());
let _ = fs::create_dir_all("c:\\doomdata");
set_basedefault("c:/doomdata/default.cfg");
}
let p_turbo = m_check_parm("-turbo");
if p_turbo != 0 {
let mut scale: c_int = 200;
if p_turbo < myargc - 1 {
let s = CStr::from_ptr(*myargv.add((p_turbo + 1) as usize)).to_string_lossy();
scale = s.parse::<c_int>().unwrap_or(200);
}
scale = scale.clamp(10, 400);
println!("turbo scale: {}%", scale);
let fwd_ptr = (&raw mut forwardmove) as *mut c_int;
let side_ptr = (&raw mut sidemove) as *mut c_int;
*fwd_ptr.add(0) = *fwd_ptr.add(0) * scale / 100;
*fwd_ptr.add(1) = *fwd_ptr.add(1) * scale / 100;
*side_ptr.add(0) = *side_ptr.add(0) * scale / 100;
*side_ptr.add(1) = *side_ptr.add(1) * scale / 100;
}
let p_file = m_check_parm("-file");
if p_file != 0 {
modifiedgame = 1;
let mut idx = (p_file + 1) as usize;
while idx < (myargc as usize) {
let arg = *myargv.add(idx);
if *arg == (b'-' as c_char) {
break;
}
D_AddFile(arg);
idx += 1;
}
}
let mut p_demo = m_check_parm("-playdemo");
if p_demo == 0 {
p_demo = m_check_parm("-timedemo");
}
if p_demo != 0 && p_demo < myargc - 1 {
let name = CStr::from_ptr(*myargv.add((p_demo + 1) as usize)).to_string_lossy();
let lmp = format!("{}.lmp", name);
add_file_str(&lmp);
println!("Playing demo {}.", lmp);
}
startskill = sk_medium;
startepisode = 1;
startmap = 1;
autostart = 0;
let p_skill = m_check_parm("-skill");
if p_skill != 0 && p_skill < myargc - 1 {
let ch = *(*myargv.add((p_skill + 1) as usize)) as u8;
startskill = ch.wrapping_sub(b'1') as skill_t;
autostart = 1;
}
let p_ep = m_check_parm("-episode");
if p_ep != 0 && p_ep < myargc - 1 {
let ch = *(*myargv.add((p_ep + 1) as usize)) as u8;
startepisode = ch.wrapping_sub(b'0') as c_int;
startmap = 1;
autostart = 1;
}
let p_warp = m_check_parm("-warp");
if p_warp != 0 && p_warp < myargc - 1 {
if gamemode == commercial {
let s = CStr::from_ptr(*myargv.add((p_warp + 1) as usize)).to_string_lossy();
startmap = s.parse::<c_int>().unwrap_or(1);
} else if p_warp < myargc - 2 {
let ep_ch = *(*myargv.add((p_warp + 1) as usize)) as u8;
let mp_ch = *(*myargv.add((p_warp + 2) as usize)) as u8;
startepisode = ep_ch.wrapping_sub(b'0') as c_int;
startmap = mp_ch.wrapping_sub(b'0') as c_int;
}
autostart = 1;
}
println!("V_Init: allocate screens.");
V_Init();
println!("M_LoadDefaults: Load system defaults.");
M_LoadDefaults();
println!("Z_Init: Init zone memory allocation daemon. ");
Z_Init();
println!("W_Init: Init WADfiles.");
let wf_base = (&raw mut wadfiles) as *mut *mut c_char;
W_InitMultipleFiles(wf_base);
if modifiedgame != 0 {
if gamemode == shareware {
doom_error("\nYou cannot -file with the shareware version. Register!");
}
if gamemode == registered {
let reg_lumps: [&[u8]; 23] = [
b"e2m1\0", b"e2m2\0", b"e2m3\0", b"e2m4\0", b"e2m5\0", b"e2m6\0", b"e2m7\0", b"e2m8\0", b"e2m9\0",
b"e3m1\0", b"e3m2\0", b"e3m3\0", b"e3m4\0", b"e3m5\0", b"e3m6\0", b"e3m7\0", b"e3m8\0", b"e3m9\0",
b"dphoof\0", b"bfgga0\0", b"heada1\0", b"cybra1\0", b"spida1d1\0",
];
for lump in reg_lumps {
if W_CheckNumForName(lump.as_ptr() as *mut c_char) < 0 {
doom_error("\nThis is not the registered version.");
}
}
}
}
match gamemode {
shareware | indetermined => {
println!("===========================================================================");
println!("                                Shareware!");
println!("===========================================================================");
}
registered | retail | commercial => {
println!("===========================================================================");
println!("                 Commercial product - do not distribute!");
println!("         Please report software piracy to the SPA: 1-800-388-PIR8");
println!("===========================================================================");
}
_ => {}
}
println!("M_Init: Init miscellaneous info.");
M_Init();
println!("R_Init: Init DOOM refresh daemon - ");
R_Init();
println!("\nP_Init: Init Playloop state.");
P_Init();
println!("I_Init: Setting up machine state.");
I_Init();
println!("D_CheckNetGame: Checking network game status.");
D_CheckNetGame();
println!("S_Init: Setting up sound.");
S_Init(snd_SfxVolume, snd_MusicVolume);
println!("HU_Init: Setting up heads up display.");
HU_Init();
println!("ST_Init: Init status bar.");
ST_Init();
let p_stat = m_check_parm("-statcopy");
if p_stat != 0 && p_stat < myargc - 1 {
let s = CStr::from_ptr(*myargv.add((p_stat + 1) as usize)).to_string_lossy();
let addr = s.parse::<usize>().unwrap_or(0);
statcopy = addr as *mut c_void;
println!("External statistics registered.");
}
let p_rec = m_check_parm("-record");
if p_rec != 0 && p_rec < myargc - 1 {
G_RecordDemo(*myargv.add((p_rec + 1) as usize));
autostart = 1;
}
let p_pdemo = m_check_parm("-playdemo");
if p_pdemo != 0 && p_pdemo < myargc - 1 {
singledemo = 1;
G_DeferedPlayDemo(*myargv.add((p_pdemo + 1) as usize));
D_DoomLoop();
}
let p_tdemo = m_check_parm("-timedemo");
if p_tdemo != 0 && p_tdemo < myargc - 1 {
G_TimeDemo(*myargv.add((p_tdemo + 1) as usize));
D_DoomLoop();
}
let p_load = m_check_parm("-loadgame");
if p_load != 0 && p_load < myargc - 1 {
let slot_ch = *(*myargv.add((p_load + 1) as usize)) as u8 as char;
let save_path = if m_check_parm("-cdrom") != 0 {
format!("c:\\doomdata\\{}{}.dsg\0", SAVEGAMENAME_STR, slot_ch)
} else {
format!("{}{}.dsg\0", SAVEGAMENAME_STR, slot_ch)
};
G_LoadGame(save_path.as_ptr() as *mut c_char);
}
if gameaction != ga_loadgame {
if autostart != 0 || netgame != 0 {
G_InitNew(startskill, startepisode, startmap);
} else {
D_StartTitle();
}
}
D_DoomLoop();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_SetConsoleMessage(msg: *const c_char) {
unsafe {
let players_base = (&raw mut players) as *mut player_t;
(*players_base.add(consoleplayer as usize)).message = msg as *mut c_char;
}
}