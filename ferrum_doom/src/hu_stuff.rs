#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::{get_gamemode, get_language};
use crate::m_misc::HU_FONTSIZE;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::collections::VecDeque;
use std::ffi::{CStr, c_char, c_int, c_uchar, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::w_wad::W_CacheLumpName;
use crate::z_zone::PU_STATIC;

const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const FONT_SIZE: usize = (HU_FONTSIZE as usize) - 1;
const QUEUESIZE: usize = 128;

const EMPTY_TEXTLINE: hu_textline_t = hu_textline_t {
    x: 0,
    y: 0,
    f: ptr::null_mut(),
    sc: 0,
    l: [0; 81],
    len: 0,
    needsupdate: 0,
};

const EMPTY_ITEXT: hu_itext_t = hu_itext_t {
    l: EMPTY_TEXTLINE,
    lm: 0,
    on: ptr::null_mut(),
    laston: 0,
};

const EMPTY_STEXT: hu_stext_t = hu_stext_t {
    l: [EMPTY_TEXTLINE; 4],
    h: 0,
    cl: 0,
    on: ptr::null_mut(),
    laston: 0,
};

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn HUlib_initSText(
        s: *mut hu_stext_t,
        x: c_int,
        y: c_int,
        h: c_int,
        font: *mut *mut patch_t,
        startchar: c_int,
        on: *mut c_int,
    );
    fn HUlib_initTextLine(
        t: *mut hu_textline_t,
        x: c_int,
        y: c_int,
        f: *mut *mut patch_t,
        sc: c_int,
    );
    fn HUlib_addCharToTextLine(t: *mut hu_textline_t, ch: c_char) -> c_int;
    fn HUlib_initIText(
        it: *mut hu_itext_t,
        x: c_int,
        y: c_int,
        font: *mut *mut patch_t,
        startchar: c_int,
        on: *mut c_int,
    );
    fn HUlib_drawSText(s: *mut hu_stext_t);
    fn HUlib_drawIText(it: *mut hu_itext_t);
    fn HUlib_drawTextLine(l: *mut hu_textline_t, drawcursor: c_int);
    fn HUlib_eraseSText(s: *mut hu_stext_t);
    fn HUlib_eraseIText(it: *mut hu_itext_t);
    fn HUlib_eraseTextLine(l: *mut hu_textline_t);
    fn HUlib_addMessageToSText(s: *mut hu_stext_t, prefix: *mut c_char, msg: *mut c_char);
    fn HUlib_keyInIText(it: *mut hu_itext_t, ch: c_uchar) -> c_int;
    fn HUlib_resetIText(it: *mut hu_itext_t);

    static mut consoleplayer: c_int;
    static mut playeringame: [c_int; NUM_PLAYERS];
    static mut players: [player_t; NUM_PLAYERS];
    static mut gameepisode: c_int;
    static mut gamemap: c_int;
    static mut showMessages: c_int;
    static mut automapactive: c_int;
    static mut netgame: c_int;
}

#[unsafe(no_mangle)]
pub static mut chat_macros: [*mut c_char; 10] = [ptr::null_mut(); 10];

#[unsafe(no_mangle)]
pub static mut player_names: [*mut c_char; 4] = [ptr::null_mut(); 4];

#[unsafe(no_mangle)]
pub static mut chat_char: c_char = 0;

static mut plr: *mut player_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut hu_font: [*mut patch_t; FONT_SIZE] = [ptr::null_mut(); FONT_SIZE];

static mut w_title: hu_textline_t = EMPTY_TEXTLINE;

#[unsafe(no_mangle)]
pub static mut chat_on: c_int = 0;

static mut w_chat: hu_itext_t = EMPTY_ITEXT;
static mut always_off: c_int = 0;
static mut chat_dest: [c_char; NUM_PLAYERS] = [0; NUM_PLAYERS];
static mut w_inputbuffer: [hu_itext_t; NUM_PLAYERS] = [EMPTY_ITEXT; NUM_PLAYERS];

static mut message_on: c_int = 0;

#[unsafe(no_mangle)]
pub static mut message_dontfuckwithme: c_int = 0;

static mut message_nottobefuckedwith: c_int = 0;
static mut w_message: hu_stext_t = EMPTY_STEXT;
static mut message_counter: c_int = 0;
static mut headsupactive: c_int = 0;

#[unsafe(no_mangle)]
pub static mut mapnames: [*mut c_char; 45] = [ptr::null_mut(); 45];

#[unsafe(no_mangle)]
pub static mut mapnames2: [*mut c_char; 32] = [ptr::null_mut(); 32];

#[unsafe(no_mangle)]
pub static mut mapnamesp: [*mut c_char; 32] = [ptr::null_mut(); 32];

#[unsafe(no_mangle)]
pub static mut mapnamest: [*mut c_char; 32] = [ptr::null_mut(); 32];

#[unsafe(no_mangle)]
pub static mut shiftxform: *const c_char = ptr::null();

pub static french_shiftxform: [c_char; 128] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, b' ' as c_char, b'!' as c_char, b'"' as c_char, b'#' as c_char,
    b'$' as c_char, b'%' as c_char, b'&' as c_char, b'"' as c_char, b'(' as c_char, b')' as c_char,
    b'*' as c_char, b'+' as c_char, b'?' as c_char, b'_' as c_char, b'>' as c_char, b'?' as c_char,
    b'0' as c_char, b'1' as c_char, b'2' as c_char, b'3' as c_char, b'4' as c_char, b'5' as c_char,
    b'6' as c_char, b'7' as c_char, b'8' as c_char, b'9' as c_char, b'/' as c_char, b'.' as c_char,
    b'<' as c_char, b'+' as c_char, b'>' as c_char, b'?' as c_char, b'@' as c_char, b'A' as c_char,
    b'B' as c_char, b'C' as c_char, b'D' as c_char, b'E' as c_char, b'F' as c_char, b'G' as c_char,
    b'H' as c_char, b'I' as c_char, b'J' as c_char, b'K' as c_char, b'L' as c_char, b'M' as c_char,
    b'N' as c_char, b'O' as c_char, b'P' as c_char, b'Q' as c_char, b'R' as c_char, b'S' as c_char,
    b'T' as c_char, b'U' as c_char, b'V' as c_char, b'W' as c_char, b'X' as c_char, b'Y' as c_char,
    b'Z' as c_char, b'[' as c_char, b'!' as c_char, b']' as c_char, b'"' as c_char, b'_' as c_char,
    b'\'' as c_char, b'A' as c_char, b'B' as c_char, b'C' as c_char, b'D' as c_char,
    b'E' as c_char, b'F' as c_char, b'G' as c_char, b'H' as c_char, b'I' as c_char, b'J' as c_char,
    b'K' as c_char, b'L' as c_char, b'M' as c_char, b'N' as c_char, b'O' as c_char, b'P' as c_char,
    b'Q' as c_char, b'R' as c_char, b'S' as c_char, b'T' as c_char, b'U' as c_char, b'V' as c_char,
    b'W' as c_char, b'X' as c_char, b'Y' as c_char, b'Z' as c_char, b'{' as c_char, b'|' as c_char,
    b'}' as c_char, b'~' as c_char, 127,
];

pub static english_shiftxform: [c_char; 128] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, b' ' as c_char, b'!' as c_char, b'"' as c_char, b'#' as c_char,
    b'$' as c_char, b'%' as c_char, b'&' as c_char, b'"' as c_char, b'(' as c_char, b')' as c_char,
    b'*' as c_char, b'+' as c_char, b'<' as c_char, b'_' as c_char, b'>' as c_char, b'?' as c_char,
    b')' as c_char, b'!' as c_char, b'@' as c_char, b'#' as c_char, b'$' as c_char, b'%' as c_char,
    b'^' as c_char, b'&' as c_char, b'*' as c_char, b'(' as c_char, b':' as c_char, b':' as c_char,
    b'<' as c_char, b'+' as c_char, b'>' as c_char, b'?' as c_char, b'@' as c_char, b'A' as c_char,
    b'B' as c_char, b'C' as c_char, b'D' as c_char, b'E' as c_char, b'F' as c_char, b'G' as c_char,
    b'H' as c_char, b'I' as c_char, b'J' as c_char, b'K' as c_char, b'L' as c_char, b'M' as c_char,
    b'N' as c_char, b'O' as c_char, b'P' as c_char, b'Q' as c_char, b'R' as c_char, b'S' as c_char,
    b'T' as c_char, b'U' as c_char, b'V' as c_char, b'W' as c_char, b'X' as c_char, b'Y' as c_char,
    b'Z' as c_char, b'[' as c_char, b'!' as c_char, b']' as c_char, b'"' as c_char, b'_' as c_char,
    b'\'' as c_char, b'A' as c_char, b'B' as c_char, b'C' as c_char, b'D' as c_char,
    b'E' as c_char, b'F' as c_char, b'G' as c_char, b'H' as c_char, b'I' as c_char, b'J' as c_char,
    b'K' as c_char, b'L' as c_char, b'M' as c_char, b'N' as c_char, b'O' as c_char, b'P' as c_char,
    b'Q' as c_char, b'R' as c_char, b'S' as c_char, b'T' as c_char, b'U' as c_char, b'V' as c_char,
    b'W' as c_char, b'X' as c_char, b'Y' as c_char, b'Z' as c_char, b'{' as c_char, b'|' as c_char,
    b'}' as c_char, b'~' as c_char, 127,
];

pub static frenchKeyMap: [c_char; 128] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, b' ' as c_char, b'!' as c_char, b'"' as c_char, b'#' as c_char,
    b'$' as c_char, b'%' as c_char, b'&' as c_char, b'%' as c_char, b'(' as c_char, b')' as c_char,
    b'*' as c_char, b'+' as c_char, b';' as c_char, b'-' as c_char, b':' as c_char, b'!' as c_char,
    b'0' as c_char, b'1' as c_char, b'2' as c_char, b'3' as c_char, b'4' as c_char, b'5' as c_char,
    b'6' as c_char, b'7' as c_char, b'8' as c_char, b'9' as c_char, b':' as c_char, b'M' as c_char,
    b'<' as c_char, b'=' as c_char, b'>' as c_char, b'?' as c_char, b'@' as c_char, b'Q' as c_char,
    b'B' as c_char, b'C' as c_char, b'D' as c_char, b'E' as c_char, b'F' as c_char, b'G' as c_char,
    b'H' as c_char, b'I' as c_char, b'J' as c_char, b'K' as c_char, b'L' as c_char, b',' as c_char,
    b'N' as c_char, b'O' as c_char, b'P' as c_char, b'A' as c_char, b'R' as c_char, b'S' as c_char,
    b'T' as c_char, b'U' as c_char, b'V' as c_char, b'Z' as c_char, b'X' as c_char, b'Y' as c_char,
    b'W' as c_char, b'^' as c_char, b'\\' as c_char, b'$' as c_char, b'^' as c_char, b'_' as c_char,
    b'@' as c_char, b'Q' as c_char, b'B' as c_char, b'C' as c_char, b'D' as c_char, b'E' as c_char,
    b'F' as c_char, b'G' as c_char, b'H' as c_char, b'I' as c_char, b'J' as c_char, b'K' as c_char,
    b'L' as c_char, b',' as c_char, b'N' as c_char, b'O' as c_char, b'P' as c_char, b'A' as c_char,
    b'R' as c_char, b'S' as c_char, b'T' as c_char, b'U' as c_char, b'V' as c_char, b'Z' as c_char,
    b'X' as c_char, b'Y' as c_char, b'W' as c_char, b'^' as c_char, b'\\' as c_char, b'$' as c_char,
    b'^' as c_char, 127,
];

#[inline]
pub fn foreign_translation(ch: c_uchar) -> c_char {
    if ch < 128 {
        frenchKeyMap[ch as usize]
    } else {
        ch as c_char
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ForeignTranslation(ch: c_uchar) -> c_char {
    foreign_translation(ch)
}

unsafe fn init_string_tables() {
    unsafe {
        let cm = (&raw mut chat_macros).as_mut().unwrap();
        let macros: [&[u8]; 10] = [
            HUSTR_CHATMACRO0, HUSTR_CHATMACRO1, HUSTR_CHATMACRO2, HUSTR_CHATMACRO3, HUSTR_CHATMACRO4,
HUSTR_CHATMACRO5, HUSTR_CHATMACRO6, HUSTR_CHATMACRO7, HUSTR_CHATMACRO8, HUSTR_CHATMACRO9,
];
for (i, m) in macros.iter().enumerate() {
cm[i] = m.as_ptr() as *mut c_char;
}
let pn = (&raw mut player_names).as_mut().unwrap();
pn[0] = HUSTR_PLRGREEN.as_ptr() as *mut c_char;
pn[1] = HUSTR_PLRINDIGO.as_ptr() as *mut c_char;
pn[2] = HUSTR_PLRBROWN.as_ptr() as *mut c_char;
pn[3] = HUSTR_PLRRED.as_ptr() as *mut c_char;
let m1 = (&raw mut mapnames).as_mut().unwrap();
let doom1_names: [&[u8]; 45] = [
HUSTR_E1M1, HUSTR_E1M2, HUSTR_E1M3, HUSTR_E1M4, HUSTR_E1M5, HUSTR_E1M6, HUSTR_E1M7, HUSTR_E1M8, HUSTR_E1M9,
HUSTR_E2M1, HUSTR_E2M2, HUSTR_E2M3, HUSTR_E2M4, HUSTR_E2M5, HUSTR_E2M6, HUSTR_E2M7, HUSTR_E2M8, HUSTR_E2M9,
HUSTR_E3M1, HUSTR_E3M2, HUSTR_E3M3, HUSTR_E3M4, HUSTR_E3M5, HUSTR_E3M6, HUSTR_E3M7, HUSTR_E3M8, HUSTR_E3M9,
HUSTR_E4M1, HUSTR_E4M2, HUSTR_E4M3, HUSTR_E4M4, HUSTR_E4M5, HUSTR_E4M6, HUSTR_E4M7, HUSTR_E4M8, HUSTR_E4M9,
b"NEWLEVEL\0", b"NEWLEVEL\0", b"NEWLEVEL\0", b"NEWLEVEL\0", b"NEWLEVEL\0",
b"NEWLEVEL\0", b"NEWLEVEL\0", b"NEWLEVEL\0", b"NEWLEVEL\0",
];
for (i, n) in doom1_names.iter().enumerate() {
m1[i] = n.as_ptr() as *mut c_char;
}
let m2 = (&raw mut mapnames2).as_mut().unwrap();
let doom2_names: [&[u8]; 32] = [
HUSTR_1, HUSTR_2, HUSTR_3, HUSTR_4, HUSTR_5, HUSTR_6, HUSTR_7, HUSTR_8,
HUSTR_9, HUSTR_10, HUSTR_11, HUSTR_12, HUSTR_13, HUSTR_14, HUSTR_15, HUSTR_16,
HUSTR_17, HUSTR_18, HUSTR_19, HUSTR_20, HUSTR_21, HUSTR_22, HUSTR_23, HUSTR_24,
HUSTR_25, HUSTR_26, HUSTR_27, HUSTR_28, HUSTR_29, HUSTR_30, HUSTR_31, HUSTR_32,
];
for (i, n) in doom2_names.iter().enumerate() {
m2[i] = n.as_ptr() as *mut c_char;
}
let mp = (&raw mut mapnamesp).as_mut().unwrap();
let plut_names: [&[u8]; 32] = [
PHUSTR_1, PHUSTR_2, PHUSTR_3, PHUSTR_4, PHUSTR_5, PHUSTR_6, PHUSTR_7, PHUSTR_8,
PHUSTR_9, PHUSTR_10, PHUSTR_11, PHUSTR_12, PHUSTR_13, PHUSTR_14, PHUSTR_15, PHUSTR_16,
PHUSTR_17, PHUSTR_18, PHUSTR_19, PHUSTR_20, PHUSTR_21, PHUSTR_22, PHUSTR_23, PHUSTR_24,
PHUSTR_25, PHUSTR_26, PHUSTR_27, PHUSTR_28, PHUSTR_29, PHUSTR_30, PHUSTR_31, PHUSTR_32,
];
for (i, n) in plut_names.iter().enumerate() {
mp[i] = n.as_ptr() as *mut c_char;
}
let mt = (&raw mut mapnamest).as_mut().unwrap();
let tnt_names: [&[u8]; 32] = [
THUSTR_1, THUSTR_2, THUSTR_3, THUSTR_4, THUSTR_5, THUSTR_6, THUSTR_7, THUSTR_8,
THUSTR_9, THUSTR_10, THUSTR_11, THUSTR_12, THUSTR_13, THUSTR_14, THUSTR_15, THUSTR_16,
THUSTR_17, THUSTR_18, THUSTR_19, THUSTR_20, THUSTR_21, THUSTR_22, THUSTR_23, THUSTR_24,
THUSTR_25, THUSTR_26, THUSTR_27, THUSTR_28, THUSTR_29, THUSTR_30, THUSTR_31, THUSTR_32,
];
for (i, n) in tnt_names.iter().enumerate() {
mt[i] = n.as_ptr() as *mut c_char;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_Init() {
unsafe {
init_string_tables();
shiftxform = if get_language() == french {
french_shiftxform.as_ptr()
} else {
english_shiftxform.as_ptr()
};
let mut j = HU_FONTSTART as c_int;
let font = (&raw mut hu_font).as_mut().unwrap();
for slot in font.iter_mut() {
let name = format!("STCFN{:03}\0", j);
j += 1;
*slot = W_CacheLumpName(name.as_ptr() as *mut c_char, PU_STATIC) as *mut patch_t;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_Stop() {
unsafe {
headsupactive = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_Start() {
unsafe {
if headsupactive != 0 {
HU_Stop();
}
let players_arr = (&raw mut players).as_mut().unwrap();
plr = &mut players_arr[consoleplayer as usize];
message_on = 0;
message_dontfuckwithme = 0;
message_nottobefuckedwith = 0;
chat_on = 0;
let font_ptr = (&raw mut hu_font) as *mut *mut patch_t;
let f_start = HU_FONTSTART as c_int;
let font_h = (**font_ptr).height as c_int;
HUlib_initSText(
&raw mut w_message,
HU_MSGX as c_int,
HU_MSGY as c_int,
HU_MSGHEIGHT as c_int,
font_ptr,
f_start,
&raw mut message_on,
);
let title_y = 167 - font_h;
HUlib_initTextLine(&raw mut w_title, 0, title_y, font_ptr, f_start);
let mut s: *const c_char = match get_gamemode() {
shareware | registered | retail => {
let idx = ((gameepisode - 1) * 9 + gamemap - 1).max(0) as usize;
(&raw const mapnames).as_ref().unwrap()[idx]
}
_ => {
let idx = (gamemap - 1).max(0) as usize;
(&raw const mapnames2).as_ref().unwrap()[idx]
}
};
while !s.is_null() && *s != 0 {
HUlib_addCharToTextLine(&raw mut w_title, *s);
s = s.add(1);
}
let input_y = (HU_MSGY as c_int) + (HU_MSGHEIGHT as c_int) * (font_h + 1);
HUlib_initIText(
&raw mut w_chat,
HU_MSGX as c_int,
input_y,
font_ptr,
f_start,
&raw mut chat_on,
);
let ibuf = (&raw mut w_inputbuffer).as_mut().unwrap();
for item in ibuf.iter_mut() {
HUlib_initIText(item, 0, 0, ptr::null_mut(), 0, &raw mut always_off);
}
headsupactive = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_Drawer() {
unsafe {
HUlib_drawSText(&raw mut w_message);
HUlib_drawIText(&raw mut w_chat);
if automapactive != 0 {
HUlib_drawTextLine(&raw mut w_title, 0);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_Erase() {
unsafe {
HUlib_eraseSText(&raw mut w_message);
HUlib_eraseIText(&raw mut w_chat);
HUlib_eraseTextLine(&raw mut w_title);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_Ticker() {
unsafe {
if message_counter != 0 {
message_counter -= 1;
if message_counter == 0 {
message_on = 0;
message_nottobefuckedwith = 0;
}
}
if showMessages != 0 || message_dontfuckwithme != 0 {
if (!(*plr).message.is_null() && message_nottobefuckedwith == 0)
|| (!(*plr).message.is_null() && message_dontfuckwithme != 0)
{
HUlib_addMessageToSText(&raw mut w_message, ptr::null_mut(), (*plr).message);
(*plr).message = ptr::null_mut();
message_on = 1;
message_counter = HU_MSGTIMEOUT as c_int;
message_nottobefuckedwith = message_dontfuckwithme;
message_dontfuckwithme = 0;
}
}
if netgame != 0 {
let in_game = (&raw const playeringame).as_ref().unwrap();
let players_arr = (&raw mut players).as_mut().unwrap();
let dest = (&raw mut chat_dest).as_mut().unwrap();
let ibuf = (&raw mut w_inputbuffer).as_mut().unwrap();
let pnames = (&raw const player_names).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
if in_game[i] == 0 {
continue;
}
let mut c = players_arr[i].cmd[6];
if (i as c_int) != consoleplayer && c != 0 {
if c <= (HU_BROADCAST as u8) {
dest[i] = c as c_char;
} else {
if (b'a'..=b'z').contains(&c) {
c = *shiftxform.add(c as usize) as u8;
}
let rc = HUlib_keyInIText(&mut ibuf[i], c);
if rc != 0 && c == (KEY_ENTER as u8) {
if ibuf[i].l.len != 0
&& (dest[i] == ((consoleplayer + 1) as c_char)
|| dest[i] == (HU_BROADCAST as c_char))
{
HUlib_addMessageToSText(
&raw mut w_message,
pnames[i],
ibuf[i].l.l.as_ptr() as *mut c_char,
);
message_nottobefuckedwith = 1;
message_on = 1;
message_counter = HU_MSGTIMEOUT as c_int;
let snd = if get_gamemode() == commercial {
sfx_radio as c_int
} else {
sfx_tink as c_int
};
S_StartSound(ptr::null_mut(), snd);
}
HUlib_resetIText(&mut ibuf[i]);
}
}
players_arr[i].cmd[6] = 0;
}
}
}
}
}
static CHAT_QUEUE: OnceLock<Mutex<VecDeque<c_char>>> = OnceLock::new();
fn chat_queue() -> &'static Mutex<VecDeque<c_char>> {
CHAT_QUEUE.get_or_init(|| Mutex::new(VecDeque::with_capacity(QUEUESIZE)))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_queueChatChar(c: c_char) {
let mut q = chat_queue().lock().unwrap();
if q.len() >= QUEUESIZE - 1 {
unsafe {
(*plr).message = HUSTR_MSGU.as_ptr() as *mut c_char;
}
} else {
q.push_back(c);
}
}
#[unsafe(no_mangle)]
pub extern "C" fn HU_dequeueChatChar() -> c_char {
chat_queue().lock().unwrap().pop_front().unwrap_or(0)
}
static mut lastmessage: [c_char; (HU_MAXLINELENGTH as usize) + 1] =
[0; (HU_MAXLINELENGTH as usize) + 1];
static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static ALT_DOWN: AtomicBool = AtomicBool::new(false);
static NUM_NOBRAINERS: AtomicI32 = AtomicI32::new(0);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HU_Responder(ev: *mut event_t) -> c_int {
let Some(ev_ref) = (unsafe { ev.as_ref() }) else {
return 0;
};
unsafe {
let mut eatkey: c_int = 0;
let in_game = (&raw const playeringame).as_ref().unwrap();
let numplayers: c_int = in_game.iter().sum();
let data1 = ev_ref.data1 as u32;
let ev_type = ev_ref.type_;
if data1 == KEY_RSHIFT {
SHIFT_DOWN.store(ev_type == ev_keydown, Ordering::Relaxed);
return 0;
} else if data1 == KEY_RALT || data1 == KEY_LALT {
ALT_DOWN.store(ev_type == ev_keydown, Ordering::Relaxed);
return 0;
}
if ev_type != ev_keydown {
return 0;
}
let dest_keys: [u32; 4] = [
HUSTR_KEYGREEN as u32,
HUSTR_KEYINDIGO as u32,
HUSTR_KEYBROWN as u32,
HUSTR_KEYRED as u32,
];
if chat_on == 0 {
if data1 == HU_MSGREFRESH {
message_on = 1;
message_counter = HU_MSGTIMEOUT as c_int;
eatkey = 1;
} else if netgame != 0 && data1 == (b't' as u32) {
chat_on = 1;
eatkey = 1;
HUlib_resetIText(&raw mut w_chat);
HU_queueChatChar(HU_BROADCAST as c_char);
} else if netgame != 0 && numplayers > 2 {
for i in 0..NUM_PLAYERS {
if data1 == dest_keys[i] {
if in_game[i] != 0 && (i as c_int) != consoleplayer {
chat_on = 1;
eatkey = 1;
HUlib_resetIText(&raw mut w_chat);
HU_queueChatChar((i + 1) as c_char);
break;
} else if (i as c_int) == consoleplayer {
let n = NUM_NOBRAINERS.fetch_add(1, Ordering::Relaxed) + 1;
let msg = if n < 3 {
HUSTR_TALKTOSELF1.as_ptr()
} else if n < 6 {
HUSTR_TALKTOSELF2.as_ptr()
} else if n < 9 {
HUSTR_TALKTOSELF3.as_ptr()
} else if n < 32 {
HUSTR_TALKTOSELF4.as_ptr()
} else {
HUSTR_TALKTOSELF5.as_ptr()
};
(*plr).message = msg as *mut c_char;
}
}
}
}
} else {
let mut c = ev_ref.data1 as u8;
if ALT_DOWN.load(Ordering::Relaxed) {
c = c.wrapping_sub(b'0');
if c > 9 {
return 0;
}
let cm = (&raw const chat_macros).as_ref().unwrap();
let mut macromessage = cm[c as usize];
HU_queueChatChar(KEY_ENTER as c_char);
while !macromessage.is_null() && *macromessage != 0 {
HU_queueChatChar(*macromessage);
macromessage = macromessage.add(1);
}
HU_queueChatChar(KEY_ENTER as c_char);
chat_on = 0;
let src_cstr = CStr::from_ptr(cm[c as usize]);
let dst = (&raw mut lastmessage).as_mut().unwrap();
let bytes = src_cstr.to_bytes_with_nul();
let len = bytes.len().min(dst.len());
for (d, &s) in dst[..len].iter_mut().zip(&bytes[..len]) {
*d = s as c_char;
}
(*plr).message = dst.as_mut_ptr();
eatkey = 1;
} else {
if get_language() == french {
c = foreign_translation(c) as u8;
}
if SHIFT_DOWN.load(Ordering::Relaxed) || (b'a'..=b'z').contains(&c) {
c = *shiftxform.add(c as usize) as u8;
}
eatkey = HUlib_keyInIText(&raw mut w_chat, c);
if eatkey != 0 {
HU_queueChatChar(c as c_char);
}
if c == (KEY_ENTER as u8) {
chat_on = 0;
if w_chat.l.len != 0 {
let src_cstr = CStr::from_ptr(w_chat.l.l.as_ptr());
let dst = (&raw mut lastmessage).as_mut().unwrap();
let bytes = src_cstr.to_bytes_with_nul();
let len = bytes.len().min(dst.len());
for (d, &s) in dst[..len].iter_mut().zip(&bytes[..len]) {
*d = s as c_char;
}
(*plr).message = dst.as_mut_ptr();
}
} else if c == (KEY_ESCAPE as u8) {
chat_on = 0;
}
}
}
eatkey
}
}
