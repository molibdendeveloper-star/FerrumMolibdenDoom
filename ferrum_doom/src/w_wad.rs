#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::v_video::doom_error;
use crate::z_zone::{PU_PURGELEVEL, Z_ChangeTag2, Z_Free, Z_Malloc, get_block_tag};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct WadInfo {
    pub identification: [u8; 4],
    pub numlumps: c_int,
    pub infotableofs: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FileLump {
    pub filepos: c_int,
    pub size: c_int,
    pub name: [u8; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LumpInfo {
    pub name: [u8; 8],
    pub handle: c_int,
    pub position: c_int,
    pub size: c_int,
}

// Указатели для совместимости с модулями, которые читают lumpinfo / numlumps напрямую
#[unsafe(no_mangle)]
pub static mut lumpinfo: *mut LumpInfo = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numlumps: c_int = 0;

#[unsafe(no_mangle)]
pub static mut lumpcache: *mut *mut c_void = ptr::null_mut();

struct WadState {
    lumps: Vec<LumpInfo>,
    cache: Vec<usize>, // Храним *mut c_void как usize для Send
    open_files: Vec<File>,
    reload_name: Option<String>,
    reload_lump: usize,
    profile_info: Vec<[u8; 10]>,
    profile_count: usize,
}

impl WadState {
    const fn new() -> Self {
        Self {
            lumps: Vec::new(),
            cache: Vec::new(),
            open_files: Vec::new(),
            reload_name: None,
            reload_lump: 0,
            profile_info: Vec::new(),
            profile_count: 0,
        }
    }

    fn sync_legacy_globals(&mut self) {
        unsafe {
            numlumps = self.lumps.len() as c_int;
            lumpinfo = if self.lumps.is_empty() {
                ptr::null_mut()
            } else {
                self.lumps.as_mut_ptr()
            };
            lumpcache = if self.cache.is_empty() {
                ptr::null_mut()
            } else {
                self.cache.as_mut_ptr() as *mut *mut c_void
            };
        }
    }
}

static WAD_STATE: OnceLock<Mutex<WadState>> = OnceLock::new();

fn wad() -> &'static Mutex<WadState> {
    WAD_STATE.get_or_init(|| Mutex::new(WadState::new()))
}

fn extract_file_base(path_str: &str) -> [u8; 8] {
    let stem = Path::new(path_str)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if stem.len() > 8 {
        doom_error(&format!("Filename base of {} >8 chars", path_str));
    }

    let mut dest = [0u8; 8];
    for (i, b) in stem.bytes().enumerate() {
        dest[i] = b.to_ascii_uppercase();
    }
    dest
}

pub fn w_add_file(raw_filename: &str) {
    let mut st = wad().lock().unwrap();
    let (fname_str, is_reload) = if let Some(stripped) = raw_filename.strip_prefix('~') {
        st.reload_name = Some(stripped.to_string());
        st.reload_lump = st.lumps.len();
        (stripped, true)
    } else {
        (raw_filename, false)
    };

    let mut file = match File::open(fname_str) {
        Ok(f) => f,
        Err(_) => {
            println!(" couldn't open {}", fname_str);
            return;
        }
    };

    println!(" adding {}", fname_str);
    let mut filelumps: Vec<FileLump> = Vec::new();

    let is_wad = fname_str.len() >= 3
        && fname_str[fname_str.len() - 3..].eq_ignore_ascii_case("wad");

    if !is_wad {
        let file_len = file
            .metadata()
            .map(|m| m.len() as i32)
            .unwrap_or_else(|_| doom_error("Error fstating"));

        filelumps.push(FileLump {
            filepos: 0,
            size: i32::from_le(file_len),
            name: extract_file_base(fname_str),
        });
    } else {
        let mut header_buf = [0u8; size_of::<WadInfo>()];
        if file.read_exact(&mut header_buf).is_err() {
            doom_error(&format!("Wad file {} header read failed", fname_str));
        }

        let id = &header_buf[0..4];
        if id != b"IWAD" && id != b"PWAD" {
            doom_error(&format!(
                "Wad file {} doesn't have IWAD or PWAD id\n",
                fname_str
            ));
        }

        let wad_numlumps = i32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let infotableofs = i32::from_le_bytes(header_buf[8..12].try_into().unwrap());

        if file.seek(SeekFrom::Start(infotableofs as u64)).is_err() {
            doom_error(&format!("Wad file {} seek failed", fname_str));
        }

        filelumps.reserve(wad_numlumps as usize);
        for _ in 0..wad_numlumps {
            let mut entry = [0u8; size_of::<FileLump>()];
            if file.read_exact(&mut entry).is_err() {
                doom_error(&format!("Wad file {} directory read failed", fname_str));
            }
            let filepos = i32::from_le_bytes(entry[0..4].try_into().unwrap());
            let size = i32::from_le_bytes(entry[4..8].try_into().unwrap());
            let mut name = [0u8; 8];
            name.copy_from_slice(&entry[8..16]);

            filelumps.push(FileLump {
                filepos,
                size,
                name,
            });
        }
    }

    let storehandle = if is_reload {
        -1
    } else {
        let idx = st.open_files.len() as c_int;
        st.open_files.push(file);
        idx
    };

    for fl in filelumps {
        st.lumps.push(LumpInfo {
            name: fl.name,
            handle: storehandle,
            position: fl.filepos,
            size: fl.size,
        });
    }

    st.sync_legacy_globals();
}

pub fn w_check_num_for_name(name: &str) -> c_int {
    let mut target_name = [0u8; 8];
    for (i, &b) in name.as_bytes().iter().take(8).enumerate() {
        if b == 0 {
            break;
        }
        target_name[i] = b.to_ascii_uppercase();
    }

    let st = wad().lock().unwrap();
    for (i, lump) in st.lumps.iter().enumerate().rev() {
        if lump.name == target_name {
            return i as c_int;
        }
    }
    -1
}

pub fn w_get_num_for_name(name: &str) -> c_int {
    let i = w_check_num_for_name(name);
    if i == -1 {
        doom_error(&format!("W_GetNumForName: {} not found!", name));
    }
    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_AddFile(filename: *mut c_char) {
    if filename.is_null() {
        return;
    }
    if let Ok(fname_str) = unsafe { CStr::from_ptr(filename).to_str() } {
        w_add_file(fname_str);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn W_Reload() {
    let (fname_str, reload_lump) = {
        let st = wad().lock().unwrap();
        let Some(ref name) = st.reload_name else { return };
        (name.clone(), st.reload_lump)
    };

    let mut file = match File::open(&fname_str) {
        Ok(f) => f,
        Err(_) => doom_error(&format!("W_Reload: couldn't open {}", fname_str)),
    };

    let mut header_buf = [0u8; size_of::<WadInfo>()];
    let _ = file.read_exact(&mut header_buf);

    let lumpcount = i32::from_le_bytes(header_buf[4..8].try_into().unwrap()) as usize;
    let infotableofs = i32::from_le_bytes(header_buf[8..12].try_into().unwrap()) as u64;

    let _ = file.seek(SeekFrom::Start(infotableofs));

    let mut st = wad().lock().unwrap();
    for i in 0..lumpcount {
        let mut entry = [0u8; size_of::<FileLump>()];
        let _ = file.read_exact(&mut entry);

        let filepos = i32::from_le_bytes(entry[0..4].try_into().unwrap());
        let size = i32::from_le_bytes(entry[4..8].try_into().unwrap());

        let idx = reload_lump + i;
        if let Some(&cached_addr) = st.cache.get(idx) {
            if cached_addr != 0 {
                Z_Free(cached_addr as *mut c_void);
                st.cache[idx] = 0;
            }
        }

        if let Some(target) = st.lumps.get_mut(idx) {
            target.position = filepos;
            target.size = size;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_InitMultipleFiles(mut filenames: *mut *mut c_char) {
    {
        let mut st = wad().lock().unwrap();
        st.lumps.clear();
        st.cache.clear();
        st.sync_legacy_globals();
    }

    if !filenames.is_null() {
        unsafe {
            while !(*filenames).is_null() {
                W_AddFile(*filenames);
                filenames = filenames.add(1);
            }
        }
    }

    let mut st = wad().lock().unwrap();
    if st.lumps.is_empty() {
        doom_error("W_InitFiles: no files found");
    }

    let count = st.lumps.len();
    st.cache = vec![0usize; count];
    st.sync_legacy_globals();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_InitFile(filename: *mut c_char) {
    let mut names: [*mut c_char; 2] = [filename, ptr::null_mut()];
    unsafe {
        W_InitMultipleFiles(names.as_mut_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn W_NumLumps() -> c_int {
    wad().lock().unwrap().lumps.len() as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_CheckNumForName(name: *const c_char) -> c_int {
    if name.is_null() {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(name as *const u8, 8) };
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(8);
    let s = std::str::from_utf8(&bytes[..len]).unwrap_or("");
    w_check_num_for_name(s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_GetNumForName(name: *const c_char) -> c_int {
let i = unsafe { W_CheckNumForName(name) };
if i == -1 {
let s = if name.is_null() {
"NULL".to_string()
} else {
unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() }
};
doom_error(&format!("W_GetNumForName: {} not found!", s));
}
i
}
#[unsafe(no_mangle)]
pub extern "C" fn W_LumpLength(lump: c_int) -> c_int {
let st = wad().lock().unwrap();
if lump < 0 || (lump as usize) >= st.lumps.len() {
doom_error(&format!("W_LumpLength: {} >= numlumps", lump));
}
st.lumps[lump as usize].size
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_ReadLump(lump: c_int, dest: *mut c_void) {
let (info, reload_name) = {
let st = wad().lock().unwrap();
if lump < 0 || (lump as usize) >= st.lumps.len() {
doom_error(&format!("W_ReadLump: {} >= numlumps", lump));
}
(st.lumps[lump as usize], st.reload_name.clone())
};
let size = info.size as usize;
let dest_slice = unsafe { std::slice::from_raw_parts_mut(dest as *mut u8, size) };
if info.handle == -1 {
let fname_str = reload_name.unwrap_or_default();
let mut temp_file = File::open(&fname_str)
.unwrap_or_else(|_| doom_error(&format!("W_ReadLump: couldn't open {}", fname_str)));
temp_file
.seek(SeekFrom::Start(info.position as u64))
.unwrap_or_else(|_| doom_error("W_ReadLump: seek error"));
temp_file
.read_exact(dest_slice)
.unwrap_or_else(|e| doom_error(&format!("W_ReadLump: read error ({})", e)));
} else {
let mut st = wad().lock().unwrap();
let file = st
.open_files
.get_mut(info.handle as usize)
.unwrap_or_else(|| doom_error(&format!("W_ReadLump: invalid handle {}", info.handle)));
file.seek(SeekFrom::Start(info.position as u64))
.unwrap_or_else(|_| doom_error("W_ReadLump: seek error"));
file.read_exact(dest_slice)
.unwrap_or_else(|e| doom_error(&format!("W_ReadLump: read error ({})", e)));
}
}
#[unsafe(no_mangle)]
pub extern "C" fn W_CacheLumpNum(lump: c_int, tag: c_int) -> *mut c_void {
let (cached_addr, len, slot_ptr) = {
let mut st = wad().lock().unwrap();
if (lump as usize) >= st.lumps.len() {
doom_error(&format!("W_CacheLumpNum: {} >= numlumps", lump));
}
let idx = lump as usize;
let ptr = (&mut st.cache[idx] as *mut usize) as *mut c_void;
(st.cache[idx], st.lumps[idx].size, ptr)
};
if cached_addr == 0 {
let buf = Z_Malloc(len, tag, slot_ptr);
unsafe {
W_ReadLump(lump, buf);
}
buf
} else {
let ptr = cached_addr as *mut c_void;
Z_ChangeTag2(ptr, tag);
ptr
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_CacheLumpName(name: *const c_char, tag: c_int) -> *mut c_void {
W_CacheLumpNum(unsafe { W_GetNumForName(name) }, tag)
}
#[unsafe(no_mangle)]
pub extern "C" fn W_Profile() {
let mut st = wad().lock().unwrap();
if st.profile_count >= 10 {
return;
}
let count = st.lumps.len();
if st.profile_info.len() < count {
st.profile_info.resize(count, [0u8; 10]);
}
let col = st.profile_count;
for i in 0..count {
let ptr = st.cache.get(i).copied().unwrap_or(0) as *mut c_void;
let ch = match get_block_tag(ptr) {
None => b' ',
Some(tag) if tag < PU_PURGELEVEL => b'S',
Some(_) => b'P',
};
st.profile_info[i][col] = ch;
}
st.profile_count += 1;
if let Ok(mut out) = File::create("waddump.txt") {
for i in 0..count {
let raw_name = &st.lumps[i].name;
let mut name_str = String::with_capacity(8);
for &b in raw_name {
if b == 0 {
break;
}
name_str.push(b as char);
}
while name_str.len() < 8 {
name_str.push(' ');
}
let _ = write!(out, "{} ", name_str);
for j in 0..st.profile_count {
let _ = write!(out, "    {}", st.profile_info[i][j] as char);
}
let _ = writeln!(out);
}
}
}
