#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::{get_gamemode, get_language};
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use crate::i_system::{I_GetTime, I_Quit, I_WaitVBL};
use crate::i_video::I_SetPalette;
use std::ffi::{CStr, CString, c_char, c_int, c_short, c_void};
use std::fs::File;
use std::io::Read;
use std::ptr;

use crate::m_argv::m_check_parm as M_CheckParm;
use crate::m_misc::HU_FONTSIZE;
use crate::r_main::R_SetViewSize;
use crate::v_video::V_DrawPatchDirect;
use crate::w_wad::W_CacheLumpName;
use crate::z_zone::PU_CACHE;

const SAVESTRINGSIZE: usize = 24;
const NEWGAME: &[u8] = b"you can't start a new game\nwhile in a network game.\n\npress a key.\0";
const NIGHTMARE: &[u8] = b"are you sure? this skill level\nisn't even remotely fair.\n\npress y or n.\0";
const SWSTRING: &[u8] = b"this is the shareware version of doom.\n\nyou need to order the entire trilogy.\n\npress a key.\0";
const SKULLXOFF: c_int = -32;
const LINEHEIGHT: c_int = 16;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct menuitem_t {
    pub status: c_short,
    pub name: [c_char; 10],
    pub routine: Option<unsafe extern "C" fn(choice: c_int)>,
    pub alphaKey: c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct menu_t {
    pub numitems: c_short,
    pub prevMenu: *mut menu_t,
    pub menuitems: *mut menuitem_t,
    pub routine: Option<unsafe extern "C" fn()>,
    pub x: c_short,
    pub y: c_short,
    pub lastOn: c_short,
}

const fn c_str10(s: &[u8]) -> [c_char; 10] {
    let mut arr = [0 as c_char; 10];
    let mut i = 0;
    while i < s.len() && i < 10 {
        arr[i] = s[i] as c_char;
        i += 1;
    }
    arr
}

const fn c_str26(s: &[u8]) -> [c_char; 26] {
    let mut arr = [0 as c_char; 26];
    let mut i = 0;
    while i < s.len() && i < 26 {
        arr[i] = s[i] as c_char;
        i += 1;
    }
    arr
}

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn S_SetSfxVolume(volume: c_int);
    fn S_SetMusicVolume(volume: c_int);
    fn G_LoadGame(name: *mut c_char);
    fn G_SaveGame(slot: c_int, description: *mut c_char);
    fn G_DeferedInitNew(skill: skill_t, episode: c_int, map: c_int);
    fn G_ScreenShot();
    fn D_StartTitle();
    
    
    
    

    static mut hu_font: [*mut patch_t; HU_FONTSIZE as usize];
    static mut message_dontfuckwithme: c_int;
    static mut chat_on: c_int;
    static mut sendpause: c_int;
    static mut netgame: c_int;
    static mut usergame: c_int;
    static mut gamestate: gamestate_t;
    static mut demoplayback: c_int;
    static mut automapactive: c_int;
    static mut devparm: c_int;
    static mut gametic: c_int;
    static mut consoleplayer: c_int;
    static mut players: [player_t; NUM_PLAYERS];
    static mut snd_SfxVolume: c_int;
    static mut snd_MusicVolume: c_int;
    // endmsg imported safely from crate::dstrings
    static mut usegamma: c_int;
}

// --- Глобальные переменные m_menu.c ---
#[unsafe(no_mangle)]
pub static mut mouseSensitivity: c_int = 5;

#[unsafe(no_mangle)]
pub static mut showMessages: c_int = 1;

#[unsafe(no_mangle)]
pub static mut detailLevel: c_int = 0;

#[unsafe(no_mangle)]
pub static mut screenblocks: c_int = 9;

#[unsafe(no_mangle)]
pub static mut screenSize: c_int = 6;

#[unsafe(no_mangle)]
pub static mut quickSaveSlot: c_int = -1;

#[unsafe(no_mangle)]
pub static mut messageToPrint: c_int = 0;

#[unsafe(no_mangle)]
pub static mut messageString: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut messx: c_int = 0;

#[unsafe(no_mangle)]
pub static mut messy: c_int = 0;

#[unsafe(no_mangle)]
pub static mut messageLastMenuActive: c_int = 0;

#[unsafe(no_mangle)]
pub static mut messageNeedsInput: c_int = 0;

#[unsafe(no_mangle)]
pub static mut messageRoutine: Option<unsafe extern "C" fn(response: c_int)> = None;

#[unsafe(no_mangle)]
pub static mut gammamsg: [[c_char; 26]; 5] = [
    c_str26(GAMMALVL0),
    c_str26(GAMMALVL1),
    c_str26(GAMMALVL2),
    c_str26(GAMMALVL3),
    c_str26(GAMMALVL4),
];

#[unsafe(no_mangle)]
pub static mut saveStringEnter: c_int = 0;

#[unsafe(no_mangle)]
pub static mut saveSlot: c_int = 0;

#[unsafe(no_mangle)]
pub static mut saveCharIndex: c_int = 0;

#[unsafe(no_mangle)]
pub static mut saveOldString: [c_char; SAVESTRINGSIZE] = [0; SAVESTRINGSIZE];

#[unsafe(no_mangle)]
pub static mut inhelpscreens: c_int = 0;

#[unsafe(no_mangle)]
pub static mut menuactive: c_int = 0;

#[unsafe(no_mangle)]
pub static mut savegamestrings: [[c_char; SAVESTRINGSIZE]; 10] = [[0; SAVESTRINGSIZE]; 10];

#[unsafe(no_mangle)]
pub static mut endstring: [c_char; 160] = [0; 160];

#[unsafe(no_mangle)]
pub static mut itemOn: c_short = 0;

#[unsafe(no_mangle)]
pub static mut skullAnimCounter: c_short = 0;

#[unsafe(no_mangle)]
pub static mut whichSkull: c_short = 0;

static SKULL_NAME: [&[u8]; 2] = [b"M_SKULL1\0", b"M_SKULL2\0"];

#[unsafe(no_mangle)]
pub static mut currentMenu: *mut menu_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut epi: c_int = 0;

// --- Определения меню ---
static mut MainMenu: [menuitem_t; 6] = [
    menuitem_t { status: 1, name: c_str10(b"M_NGAME"), routine: Some(M_NewGame), alphaKey: b'n' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_OPTION"), routine: Some(M_Options), alphaKey: b'o' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_LOADG"), routine: Some(M_LoadGame), alphaKey: b'l' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_SAVEG"), routine: Some(M_SaveGame), alphaKey: b's' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_RDTHIS"), routine: Some(M_ReadThis), alphaKey: b'r' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_QUITG"), routine: Some(M_QuitDOOM), alphaKey: b'q' as c_char },
];

static mut MainDef: menu_t = menu_t {
    numitems: 6,
    prevMenu: ptr::null_mut(),
    menuitems: unsafe { (&raw mut MainMenu) as *mut menuitem_t },
    routine: Some(M_DrawMainMenu),
    x: 97,
    y: 64,
    lastOn: 0,
};

static mut EpisodeMenu: [menuitem_t; 4] = [
    menuitem_t { status: 1, name: c_str10(b"M_EPI1"), routine: Some(M_Episode), alphaKey: b'k' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_EPI2"), routine: Some(M_Episode), alphaKey: b't' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_EPI3"), routine: Some(M_Episode), alphaKey: b'i' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_EPI4"), routine: Some(M_Episode), alphaKey: b't' as c_char },
];

static mut EpiDef: menu_t = menu_t {
    numitems: 4,
    prevMenu: unsafe { &raw mut MainDef },
    menuitems: unsafe { (&raw mut EpisodeMenu) as *mut menuitem_t },
    routine: Some(M_DrawEpisode),
    x: 48,
    y: 63,
    lastOn: 0,
};

static mut NewGameMenu: [menuitem_t; 5] = [
    menuitem_t { status: 1, name: c_str10(b"M_JKILL"), routine: Some(M_ChooseSkill), alphaKey: b'i' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_ROUGH"), routine: Some(M_ChooseSkill), alphaKey: b'h' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_HURT"), routine: Some(M_ChooseSkill), alphaKey: b'h' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_ULTRA"), routine: Some(M_ChooseSkill), alphaKey: b'u' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_NMARE"), routine: Some(M_ChooseSkill), alphaKey: b'n' as c_char },
];

static mut NewDef: menu_t = menu_t {
    numitems: 5,
    prevMenu: unsafe { &raw mut EpiDef },
    menuitems: unsafe { (&raw mut NewGameMenu) as *mut menuitem_t },
    routine: Some(M_DrawNewGame),
    x: 48,
    y: 63,
    lastOn: 2,
};

static mut OptionsMenu: [menuitem_t; 8] = [
    menuitem_t { status: 1, name: c_str10(b"M_ENDGAM"), routine: Some(M_EndGame), alphaKey: b'e' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_MESSG"), routine: Some(M_ChangeMessages), alphaKey: b'm' as c_char },
    menuitem_t { status: 1, name: c_str10(b"M_DETAIL"), routine: Some(M_ChangeDetail), alphaKey: b'g' as c_char },
    menuitem_t { status: 2, name: c_str10(b"M_SCRNSZ"), routine: Some(M_SizeDisplay), alphaKey: b's' as c_char },
    menuitem_t { status: -1, name: [0; 10], routine: None, alphaKey: 0 },
    menuitem_t { status: 2, name: c_str10(b"M_MSENS"), routine: Some(M_ChangeSensitivity), alphaKey: b'm' as c_char },
    menuitem_t { status: -1, name: [0; 10], routine: None, alphaKey: 0 },
    menuitem_t { status: 1, name: c_str10(b"M_SVOL"), routine: Some(M_Sound), alphaKey: b's' as c_char },
];

static mut OptionsDef: menu_t = menu_t {
    numitems: 8,
    prevMenu: unsafe { &raw mut MainDef },
    menuitems: unsafe { (&raw mut OptionsMenu) as *mut menuitem_t },
    routine: Some(M_DrawOptions),
    x: 60,
    y: 37,
    lastOn: 0,
};

static mut ReadMenu1: [menuitem_t; 1] = [
    menuitem_t { status: 1, name: [0; 10], routine: Some(M_ReadThis2), alphaKey: 0 },
];

static mut ReadDef1: menu_t = menu_t {
    numitems: 1,
    prevMenu: unsafe { &raw mut MainDef },
    menuitems: unsafe { (&raw mut ReadMenu1) as *mut menuitem_t },
    routine: Some(M_DrawReadThis1),
    x: 280,
    y: 185,
    lastOn: 0,
};

static mut ReadMenu2: [menuitem_t; 1] = [
    menuitem_t { status: 1, name: [0; 10], routine: Some(M_FinishReadThis), alphaKey: 0 },
];

static mut ReadDef2: menu_t = menu_t {
    numitems: 1,
    prevMenu: unsafe { &raw mut ReadDef1 },
    menuitems: unsafe { (&raw mut ReadMenu2) as *mut menuitem_t },
    routine: Some(M_DrawReadThis2),
    x: 330,
    y: 175,
    lastOn: 0,
};

static mut SoundMenu: [menuitem_t; 4] = [
    menuitem_t { status: 2, name: c_str10(b"M_SFXVOL"), routine: Some(M_SfxVol), alphaKey: b's' as c_char },
    menuitem_t { status: -1, name: [0; 10], routine: None, alphaKey: 0 },
    menuitem_t { status: 2, name: c_str10(b"M_MUSVOL"), routine: Some(M_MusicVol), alphaKey: b'm' as c_char },
    menuitem_t { status: -1, name: [0; 10], routine: None, alphaKey: 0 },
];

static mut SoundDef: menu_t = menu_t {
    numitems: 4,
    prevMenu: unsafe { &raw mut OptionsDef },
    menuitems: unsafe { (&raw mut SoundMenu) as *mut menuitem_t },
    routine: Some(M_DrawSound),
    x: 80,
    y: 64,
    lastOn: 0,
};

static mut LoadMenu: [menuitem_t; 6] = [
menuitem_t { status: 1, name: [0; 10], routine: Some(M_LoadSelect), alphaKey: b'1' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_LoadSelect), alphaKey: b'2' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_LoadSelect), alphaKey: b'3' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_LoadSelect), alphaKey: b'4' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_LoadSelect), alphaKey: b'5' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_LoadSelect), alphaKey: b'6' as c_char },
];
static mut LoadDef: menu_t = menu_t {
numitems: 6,
prevMenu: unsafe { &raw mut MainDef },
menuitems: unsafe { (&raw mut LoadMenu) as *mut menuitem_t },
routine: Some(M_DrawLoad),
x: 80,
y: 54,
lastOn: 0,
};
static mut SaveMenu: [menuitem_t; 6] = [
menuitem_t { status: 1, name: [0; 10], routine: Some(M_SaveSelect), alphaKey: b'1' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_SaveSelect), alphaKey: b'2' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_SaveSelect), alphaKey: b'3' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_SaveSelect), alphaKey: b'4' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_SaveSelect), alphaKey: b'5' as c_char },
menuitem_t { status: 1, name: [0; 10], routine: Some(M_SaveSelect), alphaKey: b'6' as c_char },
];
static mut SaveDef: menu_t = menu_t {
numitems: 6,
prevMenu: unsafe { &raw mut MainDef },
menuitems: unsafe { (&raw mut SaveMenu) as *mut menuitem_t },
routine: Some(M_DrawSave),
x: 80,
y: 54,
lastOn: 0,
};
unsafe fn draw_lump(x: c_int, y: c_int, name: &[u8]) {
unsafe {
let patch = W_CacheLumpName(name.as_ptr() as *mut c_char, PU_CACHE) as *mut crate::v_video::patch_t;
V_DrawPatchDirect(x, y, 0, patch);
}
}
unsafe fn write_c_buf(dst: *mut c_char, max_len: usize, src: &[u8]) {
unsafe {
ptr::write_bytes(dst, 0, max_len);
let copy_len = src.len().min(max_len.saturating_sub(1));
ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst, copy_len);
}
}
// --- M_ReadSaveStrings ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ReadSaveStrings() {
unsafe {
let base_name = CStr::from_ptr(SAVEGAMENAME.as_ptr() as *const c_char)
.to_str()
.unwrap_or("doomsav");
let is_cdrom = M_CheckParm("-cdrom") != 0;
let load_menu_ptr = (&raw mut LoadMenu) as *mut menuitem_t;
let strings_ptr = (&raw mut savegamestrings) as *mut [c_char; SAVESTRINGSIZE];
for i in 0..6 {
let filename = if is_cdrom {
format!(r"c:\doomdata\{}{}.dsg", base_name, i)
} else {
format!("{}{}.dsg", base_name, i)
};
let slot_buf = (*strings_ptr.add(i)).as_mut_ptr();
match File::open(&filename) {
Ok(mut f) => {
let mut raw = [0u8; SAVESTRINGSIZE];
let _ = f.read(&mut raw);
ptr::copy_nonoverlapping(raw.as_ptr() as *const c_char, slot_buf, SAVESTRINGSIZE);
(*load_menu_ptr.add(i)).status = 1;
}
Err(_) => {
write_c_buf(slot_buf, SAVESTRINGSIZE, EMPTYSTRING);
(*load_menu_ptr.add(i)).status = 0;
}
}
}
}
}
// --- M_DrawLoad & Border ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawLoad() {
unsafe {
draw_lump(72, 28, b"M_LOADG\0");
let lx = LoadDef.x as c_int;
let ly = LoadDef.y as c_int;
let strings_ptr = (&raw mut savegamestrings) as *mut [c_char; SAVESTRINGSIZE];
for i in 0..6 {
let y = ly + LINEHEIGHT * (i as c_int);
M_DrawSaveLoadBorder(lx, y);
M_WriteText(lx, y, (*strings_ptr.add(i)).as_mut_ptr());
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawSaveLoadBorder(mut x: c_int, y: c_int) {
unsafe {
draw_lump(x - 8, y + 7, b"M_LSLEFT\0");
for _ in 0..24 {
draw_lump(x, y + 7, b"M_LSCNTR\0");
x += 8;
}
draw_lump(x, y + 7, b"M_LSRGHT\0");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_LoadSelect(choice: c_int) {
unsafe {
let base_name = CStr::from_ptr(SAVEGAMENAME.as_ptr() as *const c_char)
.to_str()
.unwrap_or("doomsav");
let is_cdrom = M_CheckParm("-cdrom") != 0;
let filename = if is_cdrom {
format!(r"c:\doomdata\{}{}.dsg", base_name, choice)
} else {
format!("{}{}.dsg", base_name, choice)
};
let mut buf = [0 as c_char; 256];
write_c_buf(buf.as_mut_ptr(), 256, filename.as_bytes());
G_LoadGame(buf.as_mut_ptr());
M_ClearMenus();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_LoadGame(_choice: c_int) {
unsafe {
if netgame != 0 {
M_StartMessage(LOADNET.as_ptr() as *mut c_char, ptr::null_mut(), 0);
return;
}
M_SetupNextMenu(&raw mut LoadDef);
M_ReadSaveStrings();
}
}
// --- M_SaveGame ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawSave() {
unsafe {
draw_lump(72, 28, b"M_SAVEG\0");
let lx = LoadDef.x as c_int;
let ly = LoadDef.y as c_int;
let strings_ptr = (&raw mut savegamestrings) as *mut [c_char; SAVESTRINGSIZE];
for i in 0..6 {
let y = ly + LINEHEIGHT * (i as c_int);
M_DrawSaveLoadBorder(lx, y);
M_WriteText(lx, y, (*strings_ptr.add(i)).as_mut_ptr());
}
if saveStringEnter != 0 {
let slot_ptr = (*strings_ptr.add(saveSlot as usize)).as_mut_ptr();
let w = M_StringWidth(slot_ptr);
M_WriteText(
lx + w,
ly + LINEHEIGHT * saveSlot,
b"_\0".as_ptr() as *mut c_char,
);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DoSave(slot: c_int) {
unsafe {
let strings_ptr = (&raw mut savegamestrings) as *mut [c_char; SAVESTRINGSIZE];
G_SaveGame(slot, (*strings_ptr.add(slot as usize)).as_mut_ptr());
M_ClearMenus();
if quickSaveSlot == -2 {
quickSaveSlot = slot;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_SaveSelect(choice: c_int) {
unsafe {
saveStringEnter = 1;
saveSlot = choice;
let strings_ptr = (&raw mut savegamestrings) as *mut [c_char; SAVESTRINGSIZE];
let slot_ptr = (*strings_ptr.add(choice as usize)).as_mut_ptr();
let old_ptr = (&raw mut saveOldString) as *mut c_char;
ptr::copy_nonoverlapping(slot_ptr, old_ptr, SAVESTRINGSIZE);
let empty_c = CStr::from_ptr(EMPTYSTRING.as_ptr() as *const c_char);
let cur_c = CStr::from_ptr(slot_ptr);
if cur_c == empty_c {
*slot_ptr = 0;
}
saveCharIndex = CStr::from_ptr(slot_ptr).to_bytes().len() as c_int;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_SaveGame(_choice: c_int) {
unsafe {
if usergame == 0 {
M_StartMessage(SAVEDEAD.as_ptr() as *mut c_char, ptr::null_mut(), 0);
return;
}
if gamestate != GS_LEVEL {
return;
}
M_SetupNextMenu(&raw mut SaveDef);
M_ReadSaveStrings();
}
}
// --- QuickSave / QuickLoad ---
static mut tempstring: [c_char; 80] = [0; 80];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_QuickSaveResponse(ch: c_int) {
unsafe {
if ch == (b'y' as c_int) {
M_DoSave(quickSaveSlot);
S_StartSound(ptr::null_mut(), sfx_swtchx as c_int);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_QuickSave() {
unsafe {
if usergame == 0 {
S_StartSound(ptr::null_mut(), sfx_oof as c_int);
return;
}
if gamestate != GS_LEVEL {
return;
}
if quickSaveSlot < 0 {
M_StartControlPanel();
M_ReadSaveStrings();
M_SetupNextMenu(&raw mut SaveDef);
quickSaveSlot = -2;
return;
}
let strings_ptr = (&raw const savegamestrings) as *const [c_char; SAVESTRINGSIZE];
let slot_name = CStr::from_ptr((*strings_ptr.add(quickSaveSlot as usize)).as_ptr())
.to_str()
.unwrap_or("");
let prompt_fmt = CStr::from_ptr(QSPROMPT.as_ptr() as *const c_char)
.to_str()
.unwrap_or("quicksave over '%s'?\n\npress y or n.");
let msg = prompt_fmt.replacen("%s", slot_name, 1);
let tmp_ptr = (&raw mut tempstring) as *mut c_char;
write_c_buf(tmp_ptr, 80, msg.as_bytes());
M_StartMessage(tmp_ptr, M_QuickSaveResponse as *mut c_void, 1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_QuickLoadResponse(ch: c_int) {
unsafe {
if ch == (b'y' as c_int) {
M_LoadSelect(quickSaveSlot);
S_StartSound(ptr::null_mut(), sfx_swtchx as c_int);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_QuickLoad() {
unsafe {
if netgame != 0 {
M_StartMessage(QLOADNET.as_ptr() as *mut c_char, ptr::null_mut(), 0);
return;
}
if quickSaveSlot < 0 {
M_StartMessage(QSAVESPOT.as_ptr() as *mut c_char, ptr::null_mut(), 0);
return;
}
let strings_ptr = (&raw const savegamestrings) as *const [c_char; SAVESTRINGSIZE];
let slot_name = CStr::from_ptr((*strings_ptr.add(quickSaveSlot as usize)).as_ptr())
.to_str()
.unwrap_or("");
let prompt_fmt = CStr::from_ptr(QLPROMPT.as_ptr() as *const c_char)
.to_str()
.unwrap_or("do you want to quickload the game named\n\n'%s'?\n\npress y or n.");
let msg = prompt_fmt.replacen("%s", slot_name, 1);
let tmp_ptr = (&raw mut tempstring) as *mut c_char;
write_c_buf(tmp_ptr, 80, msg.as_bytes());
M_StartMessage(tmp_ptr, M_QuickLoadResponse as *mut c_void, 1);
}
}
// --- Read This! ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawReadThis1() {
unsafe {
inhelpscreens = 1;
match get_gamemode() {
commercial => draw_lump(0, 0, b"HELP\0"),
shareware | registered | retail => draw_lump(0, 0, b"HELP1\0"),
_ => {}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawReadThis2() {
unsafe {
inhelpscreens = 1;
match get_gamemode() {
retail | commercial => draw_lump(0, 0, b"CREDIT\0"),
shareware | registered => draw_lump(0, 0, b"HELP2\0"),
_ => {}
}
}
}
// --- Sound Menu ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawSound() {
unsafe {
draw_lump(60, 38, b"M_SVOL\0");
let sx = SoundDef.x as c_int;
let sy = SoundDef.y as c_int;
M_DrawThermo(sx, sy + LINEHEIGHT * 1, 16, snd_SfxVolume);
M_DrawThermo(sx, sy + LINEHEIGHT * 3, 16, snd_MusicVolume);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_Sound(_choice: c_int) {
unsafe {
M_SetupNextMenu(&raw mut SoundDef);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_SfxVol(choice: c_int) {
unsafe {
if choice == 0 {
if snd_SfxVolume > 0 {
snd_SfxVolume -= 1;
}
} else if choice == 1 && snd_SfxVolume < 15 {
snd_SfxVolume += 1;
}
S_SetSfxVolume(snd_SfxVolume);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_MusicVol(choice: c_int) {
unsafe {
if choice == 0 {
if snd_MusicVolume > 0 {
snd_MusicVolume -= 1;
}
} else if choice == 1 && snd_MusicVolume < 15 {
snd_MusicVolume += 1;
}
S_SetMusicVolume(snd_MusicVolume);
}
}
// --- Main / New Game / Episode ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawMainMenu() {
unsafe {
draw_lump(94, 2, b"M_DOOM\0");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawNewGame() {
unsafe {
draw_lump(96, 14, b"M_NEWG\0");
draw_lump(54, 38, b"M_SKILL\0");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_NewGame(_choice: c_int) {
unsafe {
if netgame != 0 && demoplayback == 0 {
M_StartMessage(NEWGAME.as_ptr() as *mut c_char, ptr::null_mut(), 0);
return;
}
if get_gamemode() == commercial {
M_SetupNextMenu(&raw mut NewDef);
} else {
M_SetupNextMenu(&raw mut EpiDef);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawEpisode() {
unsafe {
draw_lump(54, 38, b"M_EPISOD\0");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_VerifyNightmare(ch: c_int) {
unsafe {
if ch != (b'y' as c_int) {
return;
}
G_DeferedInitNew(sk_nightmare, epi + 1, 1);
M_ClearMenus();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ChooseSkill(choice: c_int) {
unsafe {
if choice == (sk_nightmare as c_int) {
M_StartMessage(
NIGHTMARE.as_ptr() as *mut c_char,
M_VerifyNightmare as *mut c_void,
1,
);
return;
}
G_DeferedInitNew(choice as skill_t, epi + 1, 1);
M_ClearMenus();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_Episode(mut choice: c_int) {
unsafe {
if get_gamemode() == shareware && choice != 0 {
M_StartMessage(SWSTRING.as_ptr() as *mut c_char, ptr::null_mut(), 0);
M_SetupNextMenu(&raw mut ReadDef1);
return;
}
if get_gamemode() == registered && choice > 2 {
eprintln!("M_Episode: 4th episode requires UltimateDOOM");
choice = 0;
}
epi = choice;
M_SetupNextMenu(&raw mut NewDef);
}
}
// --- Options ---
static DETAIL_NAMES: [&[u8]; 2] = [b"M_GDHIGH\0", b"M_GDLOW\0"];
static MSG_NAMES: [&[u8]; 2] = [b"M_MSGOFF\0", b"M_MSGON\0"];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawOptions() {
unsafe {
draw_lump(108, 15, b"M_OPTTTL\0");
let ox = OptionsDef.x as c_int;
let oy = OptionsDef.y as c_int;
draw_lump(ox + 175, oy + LINEHEIGHT * 2, DETAIL_NAMES[detailLevel as usize]);
draw_lump(ox + 120, oy + LINEHEIGHT * 1, MSG_NAMES[showMessages as usize]);
M_DrawThermo(ox, oy + LINEHEIGHT * 6, 10, mouseSensitivity);
M_DrawThermo(ox, oy + LINEHEIGHT * 4, 9, screenSize);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_Options(_choice: c_int) {
unsafe {
M_SetupNextMenu(&raw mut OptionsDef);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ChangeMessages(_choice: c_int) {
unsafe {
showMessages = 1 - showMessages;
let players_base = (&raw mut players) as *mut player_t;
let p = players_base.add(consoleplayer as usize);
if showMessages == 0 {
(*p).message = MSGOFF.as_ptr() as *mut c_char;
} else {
(*p).message = MSGON.as_ptr() as *mut c_char;
}
message_dontfuckwithme = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_EndGameResponse(ch: c_int) {
unsafe {
if ch != (b'y' as c_int) {
return;
}
(*currentMenu).lastOn = itemOn;
M_ClearMenus();
D_StartTitle();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_EndGame(_choice: c_int) {
unsafe {
if usergame == 0 {
S_StartSound(ptr::null_mut(), sfx_oof as c_int);
return;
}
if netgame != 0 {
M_StartMessage(NETEND.as_ptr() as *mut c_char, ptr::null_mut(), 0);
return;
}
M_StartMessage(
ENDGAME.as_ptr() as *mut c_char,
M_EndGameResponse as *mut c_void,
1,
);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ReadThis(_choice: c_int) {
unsafe {
M_SetupNextMenu(&raw mut ReadDef1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ReadThis2(_choice: c_int) {
unsafe {
M_SetupNextMenu(&raw mut ReadDef2);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_FinishReadThis(_choice: c_int) {
unsafe {
M_SetupNextMenu(&raw mut MainDef);
}
}
// --- Quit ---
#[unsafe(no_mangle)]
pub static mut quitsounds: [c_int; 8] = [
sfx_pldeth as c_int,
sfx_dmpain as c_int,
sfx_popain as c_int,
sfx_slop as c_int,
sfx_telept as c_int,
sfx_posit1 as c_int,
sfx_posit3 as c_int,
sfx_sgtatk as c_int,
];
#[unsafe(no_mangle)]
pub static mut quitsounds2: [c_int; 8] = [
sfx_vilact as c_int,
sfx_getpow as c_int,
sfx_boscub as c_int,
sfx_slop as c_int,
sfx_skeswg as c_int,
sfx_kntdth as c_int,
sfx_bspact as c_int,
sfx_sgtatk as c_int,
];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_QuitResponse(ch: c_int) {
unsafe {
if ch != (b'y' as c_int) {
return;
}
if netgame == 0 {
let idx = ((gametic >> 2) & 7) as usize;
let snd = if get_gamemode() == commercial {
*(&raw const quitsounds2 as *const c_int).add(idx)
} else {
*(&raw const quitsounds as *const c_int).add(idx)
};
S_StartSound(ptr::null_mut(), snd);
I_WaitVBL(105);
}
I_Quit();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_QuitDOOM(_choice: c_int) {
unsafe {
let idx = if get_language() != english {
0usize
} else {
((gametic as usize) % ((NUM_QUITMESSAGES as usize) - 2)) + 1
};
let raw_msg = crate::dstrings::get_quit_message(idx);
let dosy_str = CStr::from_ptr(DOSY.as_ptr() as *const c_char)
.to_str()
.unwrap_or("(press y to quit)");
let full_msg = format!("{}\n\n{}", raw_msg, dosy_str);
let end_ptr = (&raw mut endstring) as *mut c_char;
write_c_buf(end_ptr, 160, full_msg.as_bytes());
M_StartMessage(end_ptr, M_QuitResponse as *mut c_void, 1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ChangeSensitivity(choice: c_int) {
unsafe {
if choice == 0 {
if mouseSensitivity > 0 {
mouseSensitivity -= 1;
}
} else if choice == 1 && mouseSensitivity < 9 {
mouseSensitivity += 1;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ChangeDetail(_choice: c_int) {
unsafe {
detailLevel = 1 - detailLevel;
eprintln!("M_ChangeDetail: low detail mode n.a.");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_SizeDisplay(choice: c_int) {
unsafe {
if choice == 0 {
if screenSize > 0 {
screenblocks -= 1;
screenSize -= 1;
}
} else if choice == 1 && screenSize < 8 {
screenblocks += 1;
screenSize += 1;
}
R_SetViewSize(screenblocks, detailLevel);
}
}
// --- Widgets & Text ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawThermo(x: c_int, y: c_int, therm_width: c_int, therm_dot: c_int) {
unsafe {
let mut xx = x;
draw_lump(xx, y, b"M_THERML\0");
xx += 8;
for _ in 0..therm_width {
draw_lump(xx, y, b"M_THERMM\0");
xx += 8;
}
draw_lump(xx, y, b"M_THERMR\0");
draw_lump((x + 8) + therm_dot * 8, y, b"M_THERMO\0");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawEmptyCell(menu: *mut menu_t, item: c_int) {
unsafe {
draw_lump(
((*menu).x as c_int) - 10,
((*menu).y as c_int) + item * LINEHEIGHT - 1,
b"M_CELL1\0",
);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawSelCell(menu: *mut menu_t, item: c_int) {
unsafe {
draw_lump(
((*menu).x as c_int) - 10,
((*menu).y as c_int) + item * LINEHEIGHT - 1,
b"M_CELL2\0",
);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_StartMessage(string: *mut c_char, routine: *mut c_void, input: c_int) {
unsafe {
messageLastMenuActive = menuactive;
messageToPrint = 1;
messageString = string;
messageRoutine = if routine.is_null() {
None
} else {
Some(std::mem::transmute(routine))
};
messageNeedsInput = input;
menuactive = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_StopMessage() {
unsafe {
menuactive = messageLastMenuActive;
messageToPrint = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_StringWidth(string: *mut c_char) -> c_int {
unsafe {
let mut w = 0;
let font_base = (&raw const hu_font) as *const *mut patch_t;
let f_start = HU_FONTSTART as c_int;
let f_size = HU_FONTSIZE as c_int;
for &b in CStr::from_ptr(string).to_bytes() {
let c = (b.to_ascii_uppercase() as c_int) - f_start;
if c < 0 || c >= f_size {
w += 4;
} else {
let p = *font_base.add(c as usize);
w += (*p).width as c_int;
}
}
w
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_StringHeight(string: *mut c_char) -> c_int {
unsafe {
let font_base = (&raw const hu_font) as *const *mut patch_t;
let height = (**font_base).height as c_int;
let mut h = height;
for &b in CStr::from_ptr(string).to_bytes() {
if b == b'\n' {
h += height;
}
}
h
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_WriteText(x: c_int, y: c_int, string: *mut c_char) {
unsafe {
let mut cx = x;
let mut cy = y;
let font_base = (&raw const hu_font) as *const *mut patch_t;
let f_start = HU_FONTSTART as c_int;
let f_size = HU_FONTSIZE as c_int;
let scr_w = SCREENWIDTH as c_int;
for &b in CStr::from_ptr(string).to_bytes() {
if b == b'\n' {
cx = x;
cy += 12;
continue;
}
let c = (b.to_ascii_uppercase() as c_int) - f_start;
if c < 0 || c >= f_size {
cx += 4;
continue;
}
let p = *font_base.add(c as usize);
let w = (*p).width as c_int;
if cx + w > scr_w {
break;
}
V_DrawPatchDirect(cx, cy, 0, p as *mut crate::v_video::patch_t);
cx += w;
}
}
}
// --- M_Responder ---
static mut JOY_WAIT: c_int = 0;
static mut MOUSE_WAIT: c_int = 0;
static mut MOUSE_Y: c_int = 0;
static mut LAST_Y: c_int = 0;
static mut MOUSE_X: c_int = 0;
static mut LAST_X: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_Responder(ev: *mut event_t) -> c_int {
unsafe {
let mut ch: c_int = -1;
let ev_type = (*ev).type_;
if ev_type == ev_joystick && JOY_WAIT < I_GetTime() {
if (*ev).data3 == -1 {
ch = KEY_UPARROW as c_int;
JOY_WAIT = I_GetTime() + 5;
} else if (*ev).data3 == 1 {
ch = KEY_DOWNARROW as c_int;
JOY_WAIT = I_GetTime() + 5;
}
if (*ev).data2 == -1 {
ch = KEY_LEFTARROW as c_int;
JOY_WAIT = I_GetTime() + 2;
} else if (*ev).data2 == 1 {
ch = KEY_RIGHTARROW as c_int;
JOY_WAIT = I_GetTime() + 2;
}
if ((*ev).data1 & 1) != 0 {
ch = KEY_ENTER as c_int;
JOY_WAIT = I_GetTime() + 5;
}
if ((*ev).data1 & 2) != 0 {
ch = KEY_BACKSPACE as c_int;
JOY_WAIT = I_GetTime() + 5;
}
} else if ev_type == ev_mouse && MOUSE_WAIT < I_GetTime() {
MOUSE_Y += (*ev).data3;
if MOUSE_Y < LAST_Y - 30 {
ch = KEY_DOWNARROW as c_int;
MOUSE_WAIT = I_GetTime() + 5;
LAST_Y -= 30;
MOUSE_Y = LAST_Y;
} else if MOUSE_Y > LAST_Y + 30 {
ch = KEY_UPARROW as c_int;
MOUSE_WAIT = I_GetTime() + 5;
LAST_Y += 30;
MOUSE_Y = LAST_Y;
}
MOUSE_X += (*ev).data2;
if MOUSE_X < LAST_X - 30 {
ch = KEY_LEFTARROW as c_int;
MOUSE_WAIT = I_GetTime() + 5;
LAST_X -= 30;
MOUSE_X = LAST_X;
} else if MOUSE_X > LAST_X + 30 {
ch = KEY_RIGHTARROW as c_int;
MOUSE_WAIT = I_GetTime() + 5;
LAST_X += 30;
MOUSE_X = LAST_X;
}
if ((*ev).data1 & 1) != 0 {
ch = KEY_ENTER as c_int;
MOUSE_WAIT = I_GetTime() + 15;
}
if ((*ev).data1 & 2) != 0 {
ch = KEY_BACKSPACE as c_int;
MOUSE_WAIT = I_GetTime() + 15;
}
} else if ev_type == ev_keydown {
ch = (*ev).data1;
}
if ch == -1 {
return 0;
}
// Ввод названия сохранения
if saveStringEnter != 0 {
let strings_ptr = (&raw mut savegamestrings) as *mut [c_char; SAVESTRINGSIZE];
let slot_ptr = (*strings_ptr.add(saveSlot as usize)).as_mut_ptr();
if ch == (KEY_BACKSPACE as c_int) {
if saveCharIndex > 0 {
saveCharIndex -= 1;
*slot_ptr.add(saveCharIndex as usize) = 0;
}
} else if ch == (KEY_ESCAPE as c_int) {
saveStringEnter = 0;
let old_ptr = (&raw const saveOldString) as *const c_char;
ptr::copy_nonoverlapping(old_ptr, slot_ptr, SAVESTRINGSIZE);
} else if ch == (KEY_ENTER as c_int) {
saveStringEnter = 0;
if *slot_ptr != 0 {
M_DoSave(saveSlot);
}
} else if ch >= 0 && ch <= 255 {
let up_ch = (ch as u8).to_ascii_uppercase() as c_int;
let f_start = HU_FONTSTART as c_int;
let f_size = HU_FONTSIZE as c_int;
if up_ch != 32 && (up_ch - f_start < 0 || up_ch - f_start >= f_size) {
return 1;
}
if (32..=127).contains(&up_ch)
&& (saveCharIndex as usize) < SAVESTRINGSIZE - 1
&& M_StringWidth(slot_ptr) < ((SAVESTRINGSIZE - 2) as c_int) * 8
{
*slot_ptr.add(saveCharIndex as usize) = up_ch as c_char;
saveCharIndex += 1;
*slot_ptr.add(saveCharIndex as usize) = 0;
}
}
return 1;
}
// Модальные сообщения
if messageToPrint != 0 {
if messageNeedsInput != 0
&& !(ch == (b' ' as c_int)
|| ch == (b'n' as c_int)
|| ch == (b'y' as c_int)
|| ch == (KEY_ESCAPE as c_int))
{
return 0;
}
menuactive = messageLastMenuActive;
messageToPrint = 0;
if let Some(routine) = messageRoutine {
routine(ch);
}
menuactive = 0;
S_StartSound(ptr::null_mut(), sfx_swtchx as c_int);
return 1;
}
if devparm != 0 && ch == (KEY_F1 as c_int) {
G_ScreenShot();
return 1;
}
// F-клавиши
if menuactive == 0 {
let k = ch as u32;
if k == KEY_MINUS {
if automapactive != 0 || chat_on != 0 {
return 0;
}
M_SizeDisplay(0);
S_StartSound(ptr::null_mut(), sfx_stnmov as c_int);
return 1;
} else if k == KEY_EQUALS {
if automapactive != 0 || chat_on != 0 {
return 0;
}
M_SizeDisplay(1);
S_StartSound(ptr::null_mut(), sfx_stnmov as c_int);
return 1;
} else if k == KEY_F1 {
M_StartControlPanel();
currentMenu = if get_gamemode() == retail {
&raw mut ReadDef2
} else {
&raw mut ReadDef1
};
itemOn = 0;
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
return 1;
} else if k == KEY_F2 {
M_StartControlPanel();
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
M_SaveGame(0);
return 1;
} else if k == KEY_F3 {
M_StartControlPanel();
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
M_LoadGame(0);
return 1;
} else if k == KEY_F4 {
M_StartControlPanel();
currentMenu = &raw mut SoundDef;
itemOn = 0;
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
return 1;
} else if k == KEY_F5 {
M_ChangeDetail(0);
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
return 1;
} else if k == KEY_F6 {
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
M_QuickSave();
return 1;
} else if k == KEY_F7 {
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
M_EndGame(0);
return 1;
} else if k == KEY_F8 {
M_ChangeMessages(0);
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
return 1;
} else if k == KEY_F9 {
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
M_QuickLoad();
return 1;
} else if k == KEY_F10 {
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
M_QuitDOOM(0);
return 1;
} else if k == KEY_F11 {
usegamma += 1;
if usegamma > 4 {
usegamma = 0;
}
let players_base = (&raw mut players) as *mut player_t;
let g_ptr = (&raw mut gammamsg) as *mut [c_char; 26];
(*players_base.add(consoleplayer as usize)).message =
(*g_ptr.add(usegamma as usize)).as_mut_ptr();
I_SetPalette(W_CacheLumpName(b"PLAYPAL\0".as_ptr() as *mut c_char, PU_CACHE) as *mut u8);
return 1;
}
}
if menuactive == 0 {
if (ch as u32) == KEY_ESCAPE {
M_StartControlPanel();
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
return 1;
}
return 0;
}
// Навигация внутри меню
let k = ch as u32;
let items = (*currentMenu).menuitems;
let num_items = (*currentMenu).numitems;
if k == KEY_DOWNARROW {
loop {
if itemOn + 1 > num_items - 1 {
itemOn = 0;
} else {
itemOn += 1;
}
S_StartSound(ptr::null_mut(), sfx_pstop as c_int);
if (*items.add(itemOn as usize)).status != -1 {
break;
}
}
return 1;
} else if k == KEY_UPARROW {
loop {
if itemOn == 0 {
itemOn = num_items - 1;
} else {
itemOn -= 1;
}
S_StartSound(ptr::null_mut(), sfx_pstop as c_int);
if (*items.add(itemOn as usize)).status != -1 {
break;
}
}
return 1;
} else if k == KEY_LEFTARROW {
let cur = &*items.add(itemOn as usize);
if let Some(routine) = cur.routine {
if cur.status == 2 {
S_StartSound(ptr::null_mut(), sfx_stnmov as c_int);
routine(0);
}
}
return 1;
} else if k == KEY_RIGHTARROW {
let cur = &*items.add(itemOn as usize);
if let Some(routine) = cur.routine {
if cur.status == 2 {
S_StartSound(ptr::null_mut(), sfx_stnmov as c_int);
routine(1);
}
}
return 1;
} else if k == KEY_ENTER {
let cur = &*items.add(itemOn as usize);
if let Some(routine) = cur.routine {
if cur.status != 0 {
(*currentMenu).lastOn = itemOn;
if cur.status == 2 {
routine(1);
S_StartSound(ptr::null_mut(), sfx_stnmov as c_int);
} else {
routine(itemOn as c_int);
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
}
}
}
return 1;
} else if k == KEY_ESCAPE {
(*currentMenu).lastOn = itemOn;
M_ClearMenus();
S_StartSound(ptr::null_mut(), sfx_swtchx as c_int);
return 1;
} else if k == KEY_BACKSPACE {
(*currentMenu).lastOn = itemOn;
if !(*currentMenu).prevMenu.is_null() {
currentMenu = (*currentMenu).prevMenu;
itemOn = (*currentMenu).lastOn;
S_StartSound(ptr::null_mut(), sfx_swtchn as c_int);
}
return 1;
} else {
for i in (itemOn + 1)..num_items {
if ((*items.add(i as usize)).alphaKey as c_int) == ch {
itemOn = i;
S_StartSound(ptr::null_mut(), sfx_pstop as c_int);
return 1;
}
}
for i in 0..=itemOn {
if ((*items.add(i as usize)).alphaKey as c_int) == ch {
itemOn = i;
S_StartSound(ptr::null_mut(), sfx_pstop as c_int);
return 1;
}
}
}
0
}
}
// --- M_StartControlPanel ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_StartControlPanel() {
unsafe {
if menuactive != 0 {
return;
}
menuactive = 1;
currentMenu = &raw mut MainDef;
itemOn = (*currentMenu).lastOn;
}
}
// --- M_Drawer ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_Drawer() {
unsafe {
inhelpscreens = 0;
if messageToPrint != 0 {
let full_str = CStr::from_ptr(messageString).to_str().unwrap_or("");
let mut y = 100 - M_StringHeight(messageString) / 2;
let font_base = (&raw const hu_font) as *const *mut patch_t;
let line_h = (**font_base).height as c_int;
for line in full_str.split('\n') {
let mut line_buf = [0 as c_char; 80];
write_c_buf(line_buf.as_mut_ptr(), 80, line.as_bytes());
let x = 160 - M_StringWidth(line_buf.as_mut_ptr()) / 2;
M_WriteText(x, y, line_buf.as_mut_ptr());
y += line_h;
}
return;
}
if menuactive == 0 {
return;
}
if let Some(routine) = (*currentMenu).routine {
routine();
}
let x = (*currentMenu).x as c_int;
let mut y = (*currentMenu).y as c_int;
let max = (*currentMenu).numitems as usize;
let items = (*currentMenu).menuitems;
for i in 0..max {
let item = &*items.add(i);
if item.name[0] != 0 {
let patch = W_CacheLumpName(item.name.as_ptr() as *mut c_char, PU_CACHE)
as *mut crate::v_video::patch_t;
V_DrawPatchDirect(x, y, 0, patch);
}
y += LINEHEIGHT;
}
let skull_lump = SKULL_NAME[whichSkull as usize];
draw_lump(
x + SKULLXOFF,
((*currentMenu).y as c_int) - 5 + (itemOn as c_int) * LINEHEIGHT,
skull_lump,
);
}
}
// --- M_ClearMenus / M_SetupNextMenu / M_Ticker / M_Init ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ClearMenus() {
unsafe {
menuactive = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_SetupNextMenu(menudef: *mut menu_t) {
unsafe {
currentMenu = menudef;
itemOn = (*currentMenu).lastOn;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_Ticker() {
unsafe {
skullAnimCounter -= 1;
if skullAnimCounter <= 0 {
whichSkull ^= 1;
skullAnimCounter = 8;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_Init() {
unsafe {
currentMenu = &raw mut MainDef;
menuactive = 0;
itemOn = (*currentMenu).lastOn;
whichSkull = 0;
skullAnimCounter = 10;
screenSize = screenblocks - 3;
messageToPrint = 0;
messageString = ptr::null_mut();
messageLastMenuActive = menuactive;
quickSaveSlot = -1;
match get_gamemode() {
commercial => {
MainMenu[4] = MainMenu[5];
MainDef.numitems -= 1;
MainDef.y += 8;
NewDef.prevMenu = &raw mut MainDef;
ReadDef1.routine = Some(M_DrawReadThis1);
ReadDef1.x = 330;
ReadDef1.y = 165;
ReadMenu1[0].routine = Some(M_FinishReadThis);
}
shareware | registered => {
EpiDef.numitems -= 1;
}
_ => {}
}
}
}