#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

use crate::am_map::{AM_Drawer, automapactive};
use crate::d_net::{D_CheckNetGame, NetUpdate, TryRunTics, maketic, netcmds};
use crate::doomstat::{get_gamemode, is_modified_game, set_gamemode, set_language, set_modified_game};
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
use crate::i_sound::{I_SubmitSound, I_UpdateSound};
use crate::i_system::{I_GetTime, I_Init};
use crate::i_video::{
    I_FinishUpdate, I_InitGraphics, I_SetPalette, I_StartFrame, I_StartTic, I_UpdateNoBlit,
};
use crate::m_argv::{args_len, get_arg, get_arg_cptr, m_check_parm, set_args};
use crate::m_menu::{M_Drawer, M_Init, M_Responder, M_Ticker, inhelpscreens, menuactive};
use crate::m_misc::M_LoadDefaults;
use crate::p_setup::P_Init;
use crate::r_draw::{
    R_DrawViewBorder, R_FillBackScreen, scaledviewwidth, viewheight, viewwindowx, viewwindowy,
};
use crate::r_main::{R_ExecuteSetViewSize, R_Init, R_RenderPlayerView, setsizeneeded};
use crate::s_sound::{S_Init, S_StartMusic, S_UpdateSounds, snd_MusicVolume, snd_SfxVolume};
use crate::st_stuff::{ST_Drawer, ST_Init};
use crate::v_video::{V_DrawPatch, V_DrawPatchDirect, V_Init, doom_error};
use crate::w_wad::{W_CacheLumpName, W_InitMultipleFiles, w_check_num_for_name};
use crate::wi_stuff::WI_Drawer;
use crate::z_zone::{PU_CACHE, Z_Init};

const MAX_WADFILES: usize = MAXWADFILES as usize;
const MAX_EVENTS: usize = MAXEVENTS as usize;
const BACKUP_TICS: usize = BACKUPTICS as usize;
const VERSION: c_int = 109;
const SAVEGAMENAME_STR: &str = "doomsav";
const mus_intro: c_int = 29;
const mus_dm2ttl: c_int = 66;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_PostEvent(ev: *mut event_t) {
    let Some(ev_ref) = (unsafe { ev.as_ref() }) else { return };
    unsafe {
        let ev_arr = (&raw mut events).as_mut().unwrap();
        ev_arr[eventhead as usize] = *ev_ref;
        eventhead = (eventhead + 1) & ((MAX_EVENTS as c_int) - 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_ProcessEvents() {
    if get_gamemode() == commercial && w_check_num_for_name("map01") < 0 {
        return;
    }

    unsafe {
        let ev_arr = (&raw mut events).as_mut().unwrap();
        while eventtail != eventhead {
            let ev = &mut ev_arr[eventtail as usize];
            if M_Responder((ev as *mut event_t) as *mut _) == 0 {
                G_Responder((ev as *mut event_t) as *mut _);
            }
            eventtail = (eventtail + 1) & ((MAX_EVENTS as c_int) - 1);
        }
    }
}

struct DisplayState {
    view_active: bool,
    menu_active: bool,
    in_help_screens: bool,
    fullscreen: bool,
    old_gamestate: gamestate_t,
    border_draw_count: c_int,
}

impl DisplayState {
    const fn new() -> Self {
        Self {
            view_active: false,
            menu_active: false,
            in_help_screens: false,
            fullscreen: false,
            old_gamestate: -1i32 as gamestate_t,
            border_draw_count: 0,
        }
    }
}

static DISPLAY_STATE: Mutex<DisplayState> = Mutex::new(DisplayState::new());

#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_Display() {
    unsafe {
        if nodrawers != 0 {
            return;
        }

        let mut dst = DISPLAY_STATE.lock().unwrap();
        let mut redrawsbar = false;
        let scr_w = SCREENWIDTH as c_int;
        let scr_h = SCREENHEIGHT as c_int;

        if setsizeneeded != 0 {
            R_ExecuteSetViewSize();
            dst.old_gamestate = -1i32 as gamestate_t;
            dst.border_draw_count = 3;
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
                    if wipe || (viewheight != 200 && dst.fullscreen) {
                        redrawsbar = true;
                    }
                    if dst.in_help_screens && inhelpscreens == 0 {
                        redrawsbar = true;
                    }
                    ST_Drawer((viewheight == 200) as c_int, redrawsbar as c_int);
                    dst.fullscreen = viewheight == 200;
                }
            }
            GS_INTERMISSION => WI_Drawer(),
            GS_FINALE => F_Drawer(),
            GS_DEMOSCREEN => D_PageDrawer(),
            _ => {}
        }

        I_UpdateNoBlit();

        if gamestate == GS_LEVEL && automapactive == 0 && gametic != 0 {
            let players_arr = (&raw mut players).as_mut().unwrap();
            R_RenderPlayerView((&mut players_arr[displayplayer as usize] as *mut player_t) as *mut _);
        }

        if gamestate == GS_LEVEL && gametic != 0 {
            HU_Drawer();
        }

        if gamestate != dst.old_gamestate && gamestate != GS_LEVEL {
            let pal = W_CacheLumpName(b"PLAYPAL\0".as_ptr() as *mut c_char, PU_CACHE) as *mut u8;
            I_SetPalette(pal);
        }

        if gamestate == GS_LEVEL && dst.old_gamestate != GS_LEVEL {
            dst.view_active = false;
            R_FillBackScreen();
        }

        if gamestate == GS_LEVEL && automapactive == 0 && scaledviewwidth != 320 {
            if menuactive != 0 || dst.menu_active || !dst.view_active {
                dst.border_draw_count = 3;
            }
            if dst.border_draw_count != 0 {
                R_DrawViewBorder();
                dst.border_draw_count -= 1;
            }
        }

        dst.menu_active = menuactive != 0;
        dst.view_active = viewactive != 0;
        dst.in_help_screens = inhelpscreens != 0;
        dst.old_gamestate = gamestate;
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
            let done = wipe_ScreenWipe(wipe_Melt, 0, 0, scr_w, scr_h, tics);
            I_UpdateNoBlit();
            M_Drawer();
            I_FinishUpdate();
            if done != 0 {
                break;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_DoomLoop() {
    unsafe {
        if demorecording != 0 {
            G_BeginRecording();
        }

        I_InitGraphics();

        let players_arr = (&raw const players).as_ref().unwrap();
        let netcmds_arr = (&raw mut netcmds).as_mut().unwrap();

        loop {
            I_StartFrame();

            if singletics != 0 {
                I_StartTic();
                D_ProcessEvents();
                let slot = (maketic as usize) % BACKUP_TICS;
let cmd = &mut netcmds_arr[consoleplayer as usize][slot];
G_BuildTiccmd((cmd as *mut ticcmd_t) as *mut _);
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
let mo = players_arr[consoleplayer as usize].mo;
S_UpdateSounds(mo as *mut c_void);
D_Display();
I_UpdateSound();
I_SubmitSound();
}
}
}
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
let players_arr = (&raw mut players).as_mut().unwrap();
players_arr[consoleplayer as usize].playerstate = PST_LIVE as c_int;
advancedemo = 0;
usergame = 0;
paused = 0;
gameaction = ga_nothing;
let seq_mod = if get_gamemode() == retail { 7 } else { 6 };
demosequence = (demosequence + 1) % seq_mod;
match demosequence {
0 => {
pagetic = if get_gamemode() == commercial { 35 * 11 } else { 170 };
gamestate = GS_DEMOSCREEN;
pagename = b"TITLEPIC\0".as_ptr() as *mut c_char;
if get_gamemode() == commercial {
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
if get_gamemode() == commercial {
pagetic = 35 * 11;
pagename = b"TITLEPIC\0".as_ptr() as *mut c_char;
S_StartMusic(mus_dm2ttl);
} else {
pagetic = 200;
pagename = if get_gamemode() == retail {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_AddFile(file: *mut c_char) {
if file.is_null() {
return;
}
let s = unsafe { CStr::from_ptr(file).to_bytes_with_nul() };
let leaked = Box::leak(s.to_vec().into_boxed_slice());
unsafe {
let wf = (&raw mut wadfiles).as_mut().unwrap();
if let Some(slot) = wf.iter_mut().find(|p| p.is_null()) {
*slot = leaked.as_mut_ptr() as *mut c_char;
}
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
let dst = (&raw mut basedefault).as_mut().unwrap();
dst.fill(0);
for (d, &b) in dst.iter_mut().zip(path.as_bytes()).take(1023) {
*d = b as c_char;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn IdentifyVersion() {
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
set_gamemode(mode);
if let Some(l) = lang {
set_language(l);
println!("French version");
}
add_file_str(&full_path);
return;
}
}
println!("Game mode indeterminate.");
set_gamemode(indetermined);
}
#[unsafe(no_mangle)]
pub extern "C" fn FindResponseFile() {
let total = args_len();
for i in 1..total {
let Some(arg) = get_arg(i) else { continue };
if let Some(fname) = arg.strip_prefix('@') {
let content = match fs::read(fname) {
Ok(c) => c,
Err(_) => {
println!("\nNo such response file!");
std::process::exit(1);
}
};
println!("Found response file {}!", fname);
let mut new_args: Vec<String> = Vec::new();
if let Some(prog) = get_arg(0) {
new_args.push(prog);
}
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
new_args.push(String::from_utf8_lossy(&content[start..k]).into_owned());
}
for k_idx in (i + 1)..total {
if let Some(a) = get_arg(k_idx) {
new_args.push(a);
}
}
set_args(new_args);
let updated_len = args_len();
println!("{} command-line args:", updated_len);
for k_idx in 1..updated_len {
if let Some(a) = get_arg(k_idx) {
println!("{}", a);
}
}
break;
}
}
}
#[unsafe(no_mangle)]
pub extern "C" fn D_DoomMain() {
unsafe {
FindResponseFile();
IdentifyVersion();
set_modified_game(false);
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
let banner = match get_gamemode() {
retail => format!(
"                         The Ultimate DOOM Startup v{}.{}                           ",
major, minor
),
shareware => format!(
"                            DOOM Shareware Startup v{}.{}                           ",
major, minor
),
registered => format!(
"                            DOOM Registered Startup v{}.{}                           ",
major, minor
),
commercial => format!(
"                         DOOM 2: Hell on Earth v{}.{}                           ",
major, minor
),
_ => format!(
"                     Public DOOM - v{}.{}                           ",
major, minor
),
};
println!("{}", banner);
let title_dst = (&raw mut title).as_mut().unwrap();
title_dst.fill(0);
for (d, &b) in title_dst.iter_mut().zip(banner.as_bytes()).take(127) {
*d = b as c_char;
}
if devparm != 0 {
print!(
"{}",
CStr::from_bytes_with_nul_unchecked(D_DEVSTR).to_string_lossy()
);
}
if m_check_parm("-cdrom") != 0 {
print!(
"{}",
CStr::from_bytes_with_nul_unchecked(D_CDROM).to_string_lossy()
);
let _ = fs::create_dir_all("c:\\doomdata");
set_basedefault("c:/doomdata/default.cfg");
}
let p_turbo = m_check_parm("-turbo");
if p_turbo != 0 {
let scale = get_arg((p_turbo + 1) as usize)
.and_then(|s| s.parse::<c_int>().ok())
.unwrap_or(200)
.clamp(10, 400);
println!("turbo scale: {}%", scale);
let fwd = (&raw mut forwardmove).as_mut().unwrap();
let side = (&raw mut sidemove).as_mut().unwrap();
fwd[0] = fwd[0] * scale / 100;
            fwd[1] = fwd[1] * scale / 100;
            side[0] = side[0] * scale / 100;
            side[1] = side[1] * scale / 100;
}
let p_file = m_check_parm("-file");
if p_file != 0 {
set_modified_game(true);
let mut idx = (p_file + 1) as usize;
while let Some(arg) = get_arg(idx) {
if arg.starts_with('-') {
break;
}
add_file_str(&arg);
idx += 1;
}
}
let mut p_demo = m_check_parm("-playdemo");
if p_demo == 0 {
p_demo = m_check_parm("-timedemo");
}
if p_demo != 0 {
if let Some(name) = get_arg((p_demo + 1) as usize) {
let lmp = format!("{}.lmp", name);
add_file_str(&lmp);
println!("Playing demo {}.", lmp);
}
}
startskill = sk_medium;
startepisode = 1;
startmap = 1;
autostart = 0;
let p_skill = m_check_parm("-skill");
if p_skill != 0 {
if let Some(s) = get_arg((p_skill + 1) as usize) {
if let Some(&ch) = s.as_bytes().first() {
startskill = ch.wrapping_sub(b'1') as skill_t;
autostart = 1;
}
}
}
let p_ep = m_check_parm("-episode");
if p_ep != 0 {
if let Some(s) = get_arg((p_ep + 1) as usize) {
if let Some(&ch) = s.as_bytes().first() {
startepisode = ch.wrapping_sub(b'0') as c_int;
startmap = 1;
autostart = 1;
}
}
}
let p_warp = m_check_parm("-warp");
if p_warp != 0 {
if get_gamemode() == commercial {
if let Some(s) = get_arg((p_warp + 1) as usize) {
startmap = s.parse::<c_int>().unwrap_or(1);
autostart = 1;
}
} else if let (Some(ep_s), Some(mp_s)) =
(get_arg((p_warp + 1) as usize), get_arg((p_warp + 2) as usize))
{
if let (Some(&ep_ch), Some(&mp_ch)) = (ep_s.as_bytes().first(), mp_s.as_bytes().first()) {
startepisode = ep_ch.wrapping_sub(b'0') as c_int;
startmap = mp_ch.wrapping_sub(b'0') as c_int;
autostart = 1;
}
}
}
println!("V_Init: allocate screens.");
V_Init();
println!("M_LoadDefaults: Load system defaults.");
M_LoadDefaults();
println!("Z_Init: Init zone memory allocation daemon. ");
Z_Init();
println!("W_Init: Init WADfiles.");
W_InitMultipleFiles((&raw mut wadfiles) as *mut *mut c_char);
if is_modified_game() {
if get_gamemode() == shareware {
doom_error("\nYou cannot -file with the shareware version. Register!");
}
if get_gamemode() == registered {
let reg_lumps = [
"e2m1", "e2m2", "e2m3", "e2m4", "e2m5", "e2m6", "e2m7", "e2m8", "e2m9",
"e3m1", "e3m2", "e3m3", "e3m4", "e3m5", "e3m6", "e3m7", "e3m8", "e3m9",
"dphoof", "bfgga0", "heada1", "cybra1", "spida1d1",
];
for lump in reg_lumps {
if w_check_num_for_name(lump) < 0 {
doom_error("\nThis is not the registered version.");
}
}
}
}
match get_gamemode() {
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
if p_stat != 0 {
if let Some(s) = get_arg((p_stat + 1) as usize) {
let addr = s.parse::<usize>().unwrap_or(0);
statcopy = addr as *mut c_void;
println!("External statistics registered.");
}
}
let p_rec = m_check_parm("-record");
if p_rec != 0 && (p_rec as usize) + 1 < args_len() {
G_RecordDemo(get_arg_cptr((p_rec + 1) as usize));
autostart = 1;
}
let p_pdemo = m_check_parm("-playdemo");
if p_pdemo != 0 && (p_pdemo as usize) + 1 < args_len() {
singledemo = 1;
G_DeferedPlayDemo(get_arg_cptr((p_pdemo + 1) as usize));
D_DoomLoop();
}
let p_tdemo = m_check_parm("-timedemo");
if p_tdemo != 0 && (p_tdemo as usize) + 1 < args_len() {
G_TimeDemo(get_arg_cptr((p_tdemo + 1) as usize));
D_DoomLoop();
}
let p_load = m_check_parm("-loadgame");
if p_load != 0 {
if let Some(s) = get_arg((p_load + 1) as usize) {
if let Some(&slot_b) = s.as_bytes().first() {
let slot_ch = slot_b as char;
let save_path = if m_check_parm("-cdrom") != 0 {
format!("c:\\doomdata\\{}{}.dsg\0", SAVEGAMENAME_STR, slot_ch)
} else {
format!("{}{}.dsg\0", SAVEGAMENAME_STR, slot_ch)
};
G_LoadGame(save_path.as_ptr() as *mut c_char);
}
}
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
let players_arr = (&raw mut players).as_mut().unwrap();
players_arr[consoleplayer as usize].message = msg as *mut c_char;
}
}
