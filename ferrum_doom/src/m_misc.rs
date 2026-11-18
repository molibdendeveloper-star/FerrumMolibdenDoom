use std::ffi::{CStr, CString, c_char, c_int, c_short, c_void};
use std::fs::{self, File};
use std::io::Write;
use std::mem::size_of;
use std::path::Path;
use std::ptr;
use std::sync::{OnceLock, RwLock};

use crate::i_video::I_ReadScreen;
use crate::m_argv::{args_len, get_arg, m_check_parm};
use crate::m_swap::swap_short;
use crate::v_video::{V_DrawPatch, V_DrawPatchDirect, doom_error, screens, usegamma};
use crate::w_wad::W_CacheLumpName;
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_Malloc};

pub const SCREENWIDTH: c_int = 320;
pub const SCREENHEIGHT: c_int = 200;
pub const HU_FONTSTART: u8 = b'!';
pub const HU_FONTSIZE: usize = 64;

const KEY_RIGHTARROW: isize = 0xae;
const KEY_LEFTARROW: isize = 0xac;
const KEY_RCTRL: isize = 0x80 + 0x1d;
const KEY_RALT: isize = 0x80 + 0x38;
const KEY_RSHIFT: isize = 0x80 + 0x36;

const HUSTR_CHATMACRO0: &[u8] = b"No\0";
const HUSTR_CHATMACRO1: &[u8] = b"I'm ready to kick butt!\0";
const HUSTR_CHATMACRO2: &[u8] = b"I'm OK.\0";
const HUSTR_CHATMACRO3: &[u8] = b"I'm not looking too good!\0";
const HUSTR_CHATMACRO4: &[u8] = b"Help!\0";
const HUSTR_CHATMACRO5: &[u8] = b"You suck!\0";
const HUSTR_CHATMACRO6: &[u8] = b"Next time, scumbag...\0";
const HUSTR_CHATMACRO7: &[u8] = b"Come here!\0";
const HUSTR_CHATMACRO8: &[u8] = b"I'll take care of it.\0";
const HUSTR_CHATMACRO9: &[u8] = b"Yes\0";

#[repr(C)]
pub struct Patch {
    pub width: c_short,
    pub height: c_short,
    pub leftoffset: c_short,
    pub topoffset: c_short,
    pub columnofs: [c_int; 8],
}

unsafe extern "C" {
    static mut hu_font: [*mut Patch; HU_FONTSIZE];
    static mut basedefault: [c_char; 1024];

    static mut key_right: c_int;
    static mut key_left: c_int;
    static mut key_up: c_int;
    static mut key_down: c_int;
    static mut key_strafeleft: c_int;
    static mut key_straferight: c_int;
    static mut key_fire: c_int;
    static mut key_use: c_int;
    static mut key_strafe: c_int;
    static mut key_speed: c_int;

    static mut mousebfire: c_int;
    static mut mousebstrafe: c_int;
    static mut mousebforward: c_int;

    static mut joybfire: c_int;
    static mut joybstrafe: c_int;
    static mut joybuse: c_int;
    static mut joybspeed: c_int;

    static mut mouseSensitivity: c_int;
    static mut showMessages: c_int;
    static mut detailLevel: c_int;
    static mut screenblocks: c_int;
    static mut numChannels: c_int;
    static mut snd_SfxVolume: c_int;
    static mut snd_MusicVolume: c_int;
    static mut chat_macros: [*mut c_char; 10];
}

#[unsafe(no_mangle)]
pub static mut usemouse: c_int = 0;

#[unsafe(no_mangle)]
pub static mut usejoystick: c_int = 0;

#[unsafe(no_mangle)]
pub static mut mousetype: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut mousedev: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numdefaults: c_int = 0;

static DEFAULT_FILE: OnceLock<RwLock<String>> = OnceLock::new();

fn default_file_lock() -> &'static RwLock<String> {
    DEFAULT_FILE.get_or_init(|| RwLock::new(String::new()))
}

pub fn m_draw_text(mut x: c_int, y: c_int, direct: bool, text: &str) -> c_int {
    for b in text.bytes() {
        let ch = b.to_ascii_uppercase();
        if ch < HU_FONTSTART {
            x += 4;
            continue;
        }
        let c = (ch - HU_FONTSTART) as usize;
        if c >= HU_FONTSIZE {
            x += 4;
            continue;
        }

        let patch = unsafe { hu_font[c] };
        if patch.is_null() {
            x += 4;
            continue;
        }
        let w = swap_short(unsafe { (*patch).width as u16 }) as i16 as c_int;
        if x + w > SCREENWIDTH {
            break;
        }

        unsafe {
            if direct {
                V_DrawPatchDirect(x, y, 0, patch as *mut crate::bindings::patch_t);
            } else {
                V_DrawPatch(x, y, 0, patch as *mut crate::bindings::patch_t);
            }
        }
        x += w;
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawText(
    x: c_int,
    y: c_int,
    direct: c_int,
    string: *mut c_char,
) -> c_int {
    if string.is_null() {
        return x;
    }
    let text = unsafe { CStr::from_ptr(string).to_string_lossy() };
    m_draw_text(x, y, direct != 0, &text)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_WriteFile(
    name: *const c_char,
    source: *mut c_void,
    length: c_int,
) -> c_int {
    if name.is_null() || source.is_null() || length < 0 {
        return 0;
    }

    let Ok(fname) = (unsafe { CStr::from_ptr(name).to_str() }) else {
        return 0;
    };
    let slice = unsafe { std::slice::from_raw_parts(source as *const u8, length as usize) };
    fs::write(fname, slice).is_ok() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ReadFile(name: *const c_char, buffer: *mut *mut u8) -> c_int {
    let fname = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or_else(|_| doom_error("Couldn't read file (invalid path)"));

    let data = fs::read(fname)
        .unwrap_or_else(|_| doom_error(&format!("Couldn't read file {}", fname)));

    let length = data.len() as c_int;
    unsafe {
        let buf = Z_Malloc(length, PU_STATIC, ptr::null_mut()) as *mut u8;
        ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());
        *buffer = buf;
    }
    length
}

enum DefaultTarget {
    Int(*mut c_int, c_int),
    Str(*mut *mut c_char, &'static [u8]),
}

struct DefaultEntry {
    name: &'static str,
    target: DefaultTarget,
}

unsafe fn get_defaults_table() -> Vec<DefaultEntry> {
    unsafe {
        vec![
            DefaultEntry { name: "mouse_sensitivity", target: DefaultTarget::Int(&raw mut mouseSensitivity, 5) },
            DefaultEntry { name: "sfx_volume", target: DefaultTarget::Int(&raw mut snd_SfxVolume, 8) },
            DefaultEntry { name: "music_volume", target: DefaultTarget::Int(&raw mut snd_MusicVolume, 8) },
            DefaultEntry { name: "show_messages", target: DefaultTarget::Int(&raw mut showMessages, 1) },
            DefaultEntry { name: "key_right", target: DefaultTarget::Int(&raw mut key_right, KEY_RIGHTARROW as c_int) },
            DefaultEntry { name: "key_left", target: DefaultTarget::Int(&raw mut key_left, KEY_LEFTARROW as c_int) },
            DefaultEntry { name: "key_up", target: DefaultTarget::Int(&raw mut key_up, b'w' as c_int) },
            DefaultEntry { name: "key_down", target: DefaultTarget::Int(&raw mut key_down, b's' as c_int) },
            DefaultEntry { name: "key_strafeleft", target: DefaultTarget::Int(&raw mut key_strafeleft, b'a' as c_int) },
            DefaultEntry { name: "key_straferight", target: DefaultTarget::Int(&raw mut key_straferight, b'd' as c_int) },
            DefaultEntry { name: "key_fire", target: DefaultTarget::Int(&raw mut key_fire, KEY_RCTRL as c_int) },
            DefaultEntry { name: "key_use", target: DefaultTarget::Int(&raw mut key_use, b'e' as c_int) },
            DefaultEntry { name: "key_strafe", target: DefaultTarget::Int(&raw mut key_strafe, KEY_RALT as c_int) },
            DefaultEntry { name: "key_speed", target: DefaultTarget::Int(&raw mut key_speed, KEY_RSHIFT as c_int) },
            DefaultEntry { name: "use_mouse", target: DefaultTarget::Int(&raw mut usemouse, 1) },
            DefaultEntry { name: "mouseb_fire", target: DefaultTarget::Int(&raw mut mousebfire, 0) },
            DefaultEntry { name: "mouseb_strafe", target: DefaultTarget::Int(&raw mut mousebstrafe, 1) },
            DefaultEntry { name: "mouseb_forward", target: DefaultTarget::Int(&raw mut mousebforward, -1) },
            DefaultEntry { name: "mousedev", target: DefaultTarget::Str(&raw mut mousedev, b"/dev/ttyS0\0") },
            DefaultEntry { name: "mousetype", target: DefaultTarget::Str(&raw mut mousetype, b"microsoft\0") },
            DefaultEntry { name: "use_joystick", target: DefaultTarget::Int(&raw mut usejoystick, 0) },
            DefaultEntry { name: "joyb_fire", target: DefaultTarget::Int(&raw mut joybfire, 0) },
            DefaultEntry { name: "joyb_strafe", target: DefaultTarget::Int(&raw mut joybstrafe, 1) },
            DefaultEntry { name: "joyb_use", target: DefaultTarget::Int(&raw mut joybuse, 3) },
            DefaultEntry { name: "joyb_speed", target: DefaultTarget::Int(&raw mut joybspeed, 2) },
            DefaultEntry { name: "screenblocks", target: DefaultTarget::Int(&raw mut screenblocks, 9) },
            DefaultEntry { name: "detaillevel", target: DefaultTarget::Int(&raw mut detailLevel, 0) },
            DefaultEntry { name: "snd_channels", target: DefaultTarget::Int(&raw mut numChannels, 3) },
            DefaultEntry { name: "usegamma", target: DefaultTarget::Int(&raw mut usegamma, 0) },
            DefaultEntry { name: "chatmacro0", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char), HUSTR_CHATMACRO0) },
            DefaultEntry { name: "chatmacro1", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(1), HUSTR_CHATMACRO1) },
            DefaultEntry { name: "chatmacro2", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(2), HUSTR_CHATMACRO2) },
            DefaultEntry { name: "chatmacro3", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(3), HUSTR_CHATMACRO3) },
            DefaultEntry { name: "chatmacro4", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(4), HUSTR_CHATMACRO4) },
            DefaultEntry { name: "chatmacro5", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(5), HUSTR_CHATMACRO5) },
            DefaultEntry { name: "chatmacro6", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(6), HUSTR_CHATMACRO6) },
            DefaultEntry { name: "chatmacro7", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(7), HUSTR_CHATMACRO7) },
DefaultEntry { name: "chatmacro8", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(8), HUSTR_CHATMACRO8) },
DefaultEntry { name: "chatmacro9", target: DefaultTarget::Str((&raw mut chat_macros as *mut *mut c_char).add(9), HUSTR_CHATMACRO9) },
]
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_SaveDefaults() {
let fname = default_file_lock().read().map(|g| g.clone()).unwrap_or_default();
if fname.is_empty() {
return;
}
let Ok(mut out) = File::create(&fname) else {
return;
};
unsafe {
let table = get_defaults_table();
for entry in &table {
match entry.target {
DefaultTarget::Int(ptr, _) => {
let _ = writeln!(out, "{}\t\t{}", entry.name, *ptr);
}
DefaultTarget::Str(ptr, _) => {
let c_ptr = *ptr;
let s = if c_ptr.is_null() {
""
} else {
CStr::from_ptr(c_ptr).to_str().unwrap_or("")
};
let _ = writeln!(out, "{}\t\t\"{}\"", entry.name, s);
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_LoadDefaults() {
unsafe {
let table = get_defaults_table();
numdefaults = table.len() as c_int;
for entry in &table {
match entry.target {
DefaultTarget::Int(ptr, def_val) => *ptr = def_val,
DefaultTarget::Str(ptr, def_bytes) => *ptr = def_bytes.as_ptr() as *mut c_char,
}
}
let i = m_check_parm("-config");
let cfg_path = if i > 0 && (i as usize) + 1 < args_len() {
let s = get_arg((i + 1) as usize).unwrap_or_default();
println!("\tdefault file: {}", s);
s
} else {
CStr::from_ptr((&raw const basedefault) as *const c_char)
.to_string_lossy()
.into_owned()
};
if let Ok(mut guard) = default_file_lock().write() {
*guard = cfg_path.clone();
}
if cfg_path.is_empty() {
return;
}
if let Ok(content) = fs::read_to_string(&cfg_path) {
for line in content.lines() {
let trimmed = line.trim();
if trimmed.is_empty() {
continue;
}
let mut parts = trimmed.splitn(2, char::is_whitespace);
let Some(key) = parts.next().map(str::trim) else { continue };
let Some(val_str) = parts.next().map(str::trim) else { continue };
if let Some(entry) = table.iter().find(|e| e.name == key) {
if val_str.starts_with('"') && val_str.ends_with('"') && val_str.len() >= 2 {
if let DefaultTarget::Str(ptr, _) = entry.target {
let inner = &val_str[1..val_str.len() - 1];
if let Ok(c_str) = CString::new(inner) {
*ptr = c_str.into_raw();
}
}
} else if let DefaultTarget::Int(ptr, _) = entry.target {
let parsed = if let Some(hex) = val_str.strip_prefix("0x") {
i32::from_str_radix(hex, 16).ok()
} else {
val_str.parse::<i32>().ok()
};
if let Some(num) = parsed {
*ptr = num;
}
}
}
}
}
}
}
#[repr(C, packed)]
struct PcxHeader {
manufacturer: u8,
version: u8,
encoding: u8,
bits_per_pixel: u8,
xmin: u16,
ymin: u16,
xmax: u16,
ymax: u16,
hres: u16,
vres: u16,
palette: [u8; 48],
reserved: u8,
color_planes: u8,
bytes_per_line: u16,
palette_type: u16,
filler: [u8; 58],
}
/// Безопасная упаковка и запись скриншота в формате PCX через Vec<u8>.
pub fn write_pcx_file(filename: &str, data: &[u8], width: u16, height: u16, palette: &[u8]) {
let header = PcxHeader {
manufacturer: 0x0a,
version: 5,
encoding: 1,
bits_per_pixel: 8,
xmin: 0,
ymin: 0,
xmax: swap_short(width - 1),
ymax: swap_short(height - 1),
hres: swap_short(width),
vres: swap_short(height),
palette: [0; 48],
reserved: 0,
color_planes: 1,
bytes_per_line: swap_short(width),
palette_type: swap_short(2),
filler: [0; 58],
};
let mut pcx = Vec::with_capacity(size_of::<PcxHeader>() + data.len() * 2 + 769);
let header_bytes = unsafe {
std::slice::from_raw_parts((&header as *const PcxHeader) as *const u8, size_of::<PcxHeader>())
};
pcx.extend_from_slice(header_bytes);
for &b in data {
if (b & 0xc0) != 0xc0 {
pcx.push(b);
} else {
pcx.push(0xc1);
pcx.push(b);
}
}
pcx.push(0x0c);
pcx.extend_from_slice(&palette[..768.min(palette.len())]);
let _ = fs::write(filename, &pcx);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WritePCXfile(
filename: *mut c_char,
data: *mut u8,
width: c_int,
height: c_int,
palette: *mut u8,
) {
if filename.is_null() || data.is_null() || palette.is_null() || width <= 0 || height <= 0 {
return;
}
let Ok(fname) = (unsafe { CStr::from_ptr(filename).to_str() }) else {
return;
};
let total = (width * height) as usize;
let pixels = unsafe { std::slice::from_raw_parts(data, total) };
let pal = unsafe { std::slice::from_raw_parts(palette, 768) };
write_pcx_file(fname, pixels, width as u16, height as u16, pal);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ScreenShot() {
let total = (SCREENWIDTH * SCREENHEIGHT) as usize;
let (pixels, pal) = unsafe {
let linear = screens[2];
I_ReadScreen(linear);
let pal_ptr = W_CacheLumpName(b"PLAYPAL\0".as_ptr() as *const c_char, PU_CACHE) as *const u8;
(
std::slice::from_raw_parts(linear, total),
std::slice::from_raw_parts(pal_ptr, 768),
)
};
for i in 0..=99 {
let fname = format!("DOOM{:02}.pcx", i);
if !Path::new(&fname).exists() {
write_pcx_file(&fname, pixels, SCREENWIDTH as u16, SCREENHEIGHT as u16, pal);
return;
}
}
doom_error("M_ScreenShot: Couldn't create a PCX");
}
