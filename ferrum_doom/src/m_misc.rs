use std::ffi::{CStr, CString, c_char, c_int, c_short, c_void};
use std::fs::{self, File};
use std::io::Write;
use std::mem::size_of;
use std::path::Path;
use std::ptr;

use crate::m_argv::{m_check_parm, myargc, myargv};
use crate::m_swap::swap_short;
use crate::w_wad::W_CacheLumpName;
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_Free, Z_Malloc};

pub const SCREENWIDTH: c_int = 320;
pub const SCREENHEIGHT: c_int = 200;
pub const HU_FONTSTART: u8 = b'!';
pub const HU_FONTSIZE: usize = 64;

// Коды клавиш из doomdef.h
const KEY_RIGHTARROW: isize = 0xae;
const KEY_LEFTARROW: isize = 0xac;
const KEY_RCTRL: isize = 0x80 + 0x1d;
const KEY_RALT: isize = 0x80 + 0x38;
const KEY_RSHIFT: isize = 0x80 + 0x36;

// Макросы чата по умолчанию из dstrings.h
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
    fn I_Error(error: *const c_char, ...);
    fn I_ReadScreen(scr: *mut u8);
    fn V_DrawPatch(x: c_int, y: c_int, scrn: c_int, patch: *mut Patch);
    fn V_DrawPatchDirect(x: c_int, y: c_int, scrn: c_int, patch: *mut Patch);

    static mut hu_font: [*mut Patch; HU_FONTSIZE];
    static mut screens: [*mut u8; 5];
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
    static mut usegamma: c_int;
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

#[unsafe(no_mangle)]
pub static mut defaultfile: *mut c_char = ptr::null_mut();

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Misc error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- M_DrawText ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_DrawText(
    mut x: c_int,
    y: c_int,
    direct: c_int,
    mut string: *mut c_char,
) -> c_int {
    if string.is_null() {
        return x;
    }

    unsafe {
        while *string != 0 {
            let ch = (*string as u8).to_ascii_uppercase();
            string = string.add(1);

            if ch < HU_FONTSTART {
                x += 4;
                continue;
            }

            let c = (ch - HU_FONTSTART) as usize;
            if c >= HU_FONTSIZE {
                x += 4;
                continue;
            }

            let patch = hu_font[c];
            let w = swap_short((*patch).width as u16) as i16 as c_int;
            if x + w > SCREENWIDTH {
                break;
            }

            if direct != 0 {
                V_DrawPatchDirect(x, y, 0, patch);
            } else {
                V_DrawPatch(x, y, 0, patch);
            }
            x += w;
        }
    }

    x
}

// --- M_WriteFile ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_WriteFile(
    name: *const c_char,
    source: *mut c_void,
    length: c_int,
) -> c_int {
    if name.is_null() || source.is_null() || length < 0 {
        return 0;
    }

    unsafe {
        let fname = match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let slice = std::slice::from_raw_parts(source as *const u8, length as usize);
        match fs::write(fname, slice) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

// --- M_ReadFile ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ReadFile(name: *const c_char, buffer: *mut *mut u8) -> c_int {
    unsafe {
        let fname = CStr::from_ptr(name)
            .to_str()
            .unwrap_or_else(|_| doom_error("Couldn't read file (invalid path)"));

        let data = fs::read(fname)
            .unwrap_or_else(|_| doom_error(&format!("Couldn't read file {}", fname)));

        let length = data.len() as c_int;
        let buf = Z_Malloc(length, PU_STATIC, ptr::null_mut()) as *mut u8;
        ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());

        *buffer = buf;
        length
    }
}

// --- Конфигурация (M_SaveDefaults / M_LoadDefaults) ---
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
            // Ваша современная раскладка WASD + E
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
unsafe {
if defaultfile.is_null() {
return;
}
let fname = match CStr::from_ptr(defaultfile).to_str() {
Ok(s) => s,
Err(_) => return,
};
let mut out = match File::create(fname) {
Ok(f) => f,
Err(_) => return,
};
let table = get_defaults_table();
for entry in &table {
match entry.target {
DefaultTarget::Int(ptr, _) => {
let v = *ptr;
let _ = writeln!(out, "{}\t\t{}", entry.name, v);
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
// 1. Устанавливаем значения по умолчанию
for entry in &table {
match entry.target {
DefaultTarget::Int(ptr, def_val) => {
*ptr = def_val;
}
DefaultTarget::Str(ptr, def_bytes) => {
*ptr = def_bytes.as_ptr() as *mut c_char;
}
}
}
// 2. Проверяем флаг -config
let i = m_check_parm("-config");
if i > 0 && i < myargc - 1 {
defaultfile = *myargv.add((i + 1) as usize);
if let Ok(s) = CStr::from_ptr(defaultfile).to_str() {
println!("\tdefault file: {}", s);
}
} else {
defaultfile = (&raw mut basedefault) as *mut c_char;
}
if defaultfile.is_null() {
return;
}
// 3. Читаем файл конфигурации, если он существует
let fname = match CStr::from_ptr(defaultfile).to_str() {
Ok(s) => s,
Err(_) => return,
};
if let Ok(content) = fs::read_to_string(fname) {
for line in content.lines() {
let trimmed = line.trim();
if trimmed.is_empty() {
continue;
}
// Разделяем имя параметра и его значение по первому пробелу/табуляции
let mut parts = trimmed.splitn(2, char::is_whitespace);
let key = match parts.next() {
Some(k) => k.trim(),
None => continue,
};
let val_str = match parts.next() {
Some(v) => v.trim(),
None => continue,
};
if let Some(entry) = table.iter().find(|e| e.name == key) {
if val_str.starts_with('"') && val_str.ends_with('"') && val_str.len() >= 2 {
if let DefaultTarget::Str(ptr, _) = entry.target {
let inner = &val_str[1..val_str.len() - 1];
if let Ok(c_str) = CString::new(inner) {
// Передаем владение C-строке (в DOOM конфиг живет до конца работы программы)
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
// --- Скриншоты PCX (WritePCXfile / M_ScreenShot) ---
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn WritePCXfile(
filename: *mut c_char,
data: *mut u8,
width: c_int,
height: c_int,
palette: *mut u8,
) {
unsafe {
let total_pixels = (width * height) as usize;
let alloc_size = total_pixels * 2 + 1000;
let pcx_ptr = Z_Malloc(alloc_size as c_int, PU_STATIC, ptr::null_mut()) as *mut u8;
let header = pcx_ptr as *mut PcxHeader;
ptr::write_bytes(header, 0, 1);
(*header).manufacturer = 0x0a;
(*header).version = 5;
(*header).encoding = 1;
(*header).bits_per_pixel = 8;
(*header).xmin = 0;
(*header).ymin = 0;
(*header).xmax = swap_short((width - 1) as u16);
(*header).ymax = swap_short((height - 1) as u16);
(*header).hres = swap_short(width as u16);
(*header).vres = swap_short(height as u16);
(*header).color_planes = 1;
(*header).bytes_per_line = swap_short(width as u16);
(*header).palette_type = swap_short(2);
let mut pack = pcx_ptr.add(size_of::<PcxHeader>());
let mut src = data;
for _ in 0..total_pixels {
let b = *src;
src = src.add(1);
if (b & 0xc0) != 0xc0 {
*pack = b;
pack = pack.add(1);
} else {
*pack = 0xc1;
pack = pack.add(1);
*pack = b;
pack = pack.add(1);
}
}
// Записываем палитру (768 байт)
*pack = 0x0c;
pack = pack.add(1);
ptr::copy_nonoverlapping(palette, pack, 768);
pack = pack.add(768);
let length = pack.offset_from(pcx_ptr) as c_int;
M_WriteFile(filename, pcx_ptr as *mut c_void, length);
Z_Free(pcx_ptr as *mut c_void);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn M_ScreenShot() {
    unsafe {
        let linear = screens[2];
        I_ReadScreen(linear);

        let mut lbmname = *b"DOOM00.pcx\0";
        let mut found = false;

        for i in 0..=99 {
            lbmname[4] = (i / 10) as u8 + b'0';
            lbmname[5] = (i % 10) as u8 + b'0';
            let fname_str = std::str::from_utf8(&lbmname[..10]).unwrap();
            if !Path::new(fname_str).exists() {
                found = true;
                break;
            }
        }
        if !found {
            doom_error("M_ScreenShot: Couldn't create a PCX");
        }
        let playpal_name = b"PLAYPAL\0";
        let pal = W_CacheLumpName(playpal_name.as_ptr() as *const c_char, PU_CACHE) as *mut u8;
        WritePCXfile(
            lbmname.as_mut_ptr() as *mut c_char,
            linear,
            SCREENWIDTH,
            SCREENHEIGHT,
            pal,
        );
    }
}