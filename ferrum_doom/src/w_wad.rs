use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::path::Path;
use std::ptr;

use crate::z_zone::{PU_PURGELEVEL, Z_ChangeTag2, Z_Free, Z_Malloc};

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("WAD error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

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

// Глобальные переменные, доступные из оставшегося C-кода
#[unsafe(no_mangle)]
pub static mut lumpinfo: *mut LumpInfo = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numlumps: c_int = 0;

#[unsafe(no_mangle)]
pub static mut lumpcache: *mut *mut c_void = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut reloadlump: c_int = 0;

#[unsafe(no_mangle)]
pub static mut reloadname: *mut c_char = ptr::null_mut();

// Хранилище открытых Rust-файлов, сопоставленное по индексу handle
static mut OPEN_FILES: Vec<Option<File>> = Vec::new();

fn register_file(file: File) -> c_int {
    unsafe {
        let ptr = &raw mut OPEN_FILES;
        let idx = (*ptr).len();
        (*ptr).push(Some(file));
        idx as c_int
    }
}

fn get_file_mut(handle: c_int) -> Option<&'static mut File> {
    if handle < 0 {
        return None;
    }
    unsafe {
        let ptr = &raw mut OPEN_FILES;
        (&mut (*ptr)).get_mut(handle as usize).and_then(|opt| opt.as_mut())
    }
}

/// Извлекает базовое имя файла (до 8 символов в верхнем регистре) для одиночных файлов-лампов
fn extract_file_base(path_str: &str) -> [u8; 8] {
    let path = Path::new(path_str);
    let stem = path
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_AddFile(mut filename: *mut c_char) {
    if filename.is_null() {
        return;
    }

    unsafe {
        // Обработка префикса '~' для перезагружаемых карт
        if *filename == b'~' as c_char {
            filename = filename.add(1);
            reloadname = filename;
            reloadlump = numlumps;
        }

        let fname_str = match CStr::from_ptr(filename).to_str() {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut file = match File::open(fname_str) {
            Ok(f) => f,
            Err(_) => {
                println!(" couldn't open {}", fname_str);
                return;
            }
        };

        println!(" adding {}", fname_str);
        let startlump = numlumps as usize;
        let mut filelumps: Vec<FileLump> = Vec::new();

        let is_wad = fname_str.len() >= 3
            && fname_str[fname_str.len() - 3..].eq_ignore_ascii_case("wad");

        if !is_wad {
            // Одиночный ресурс (не WAD-архив)
            let file_len = file
                .metadata()
                .map(|m| m.len() as i32)
                .unwrap_or_else(|_| doom_error("Error fstating"));

            let name = extract_file_base(fname_str);
            filelumps.push(FileLump {
                filepos: i32::from_le(file_len),
                size: i32::from_le(file_len),
                name,
            });
            // Исправляем filepos на 0, как в оригинале
            filelumps[0].filepos = 0;
            numlumps += 1;
        } else {
            // Полноценный IWAD или PWAD файл
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

            numlumps += wad_numlumps;
        }

        // Расширяем глобальный массив lumpinfo через стандартный realloc для совместимости с C
        let new_bytes = (numlumps as usize) * size_of::<LumpInfo>();
        let new_ptr = libc_realloc(lumpinfo as *mut c_void, new_bytes) as *mut LumpInfo;
        if new_ptr.is_null() {
            doom_error("Couldn't realloc lumpinfo");
        }
        lumpinfo = new_ptr;

        let storehandle = if !reloadname.is_null() {
            // Файл закроется автоматически при выходе из области видимости
            -1
        } else {
            register_file(file)
        };

        for (offset, fl) in filelumps.iter().enumerate() {
            let target = lumpinfo.add(startlump + offset);
            (*target).handle = storehandle;
            (*target).position = fl.filepos;
            (*target).size = fl.size;
            (*target).name = fl.name;
        }
    }
}

unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(size: usize) -> *mut c_void;
    #[link_name = "realloc"]
    fn libc_realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_Reload() {
    unsafe {
        if reloadname.is_null() {
            return;
        }

        let fname_str = match CStr::from_ptr(reloadname).to_str() {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut file = match File::open(fname_str) {
            Ok(f) => f,
            Err(_) => doom_error(&format!("W_Reload: couldn't open {}", fname_str)),
        };

        let mut header_buf = [0u8; size_of::<WadInfo>()];
        let _ = file.read_exact(&mut header_buf);

        let lumpcount = i32::from_le_bytes(header_buf[4..8].try_into().unwrap()) as usize;
        let infotableofs = i32::from_le_bytes(header_buf[8..12].try_into().unwrap()) as u64;

        let _ = file.seek(SeekFrom::Start(infotableofs));

        for i in 0..lumpcount {
            let mut entry = [0u8; size_of::<FileLump>()];
            let _ = file.read_exact(&mut entry);

            let filepos = i32::from_le_bytes(entry[0..4].try_into().unwrap());
            let size = i32::from_le_bytes(entry[4..8].try_into().unwrap());

            let idx = (reloadlump as usize) + i;
            let cache_slot = lumpcache.add(idx);
            if !(*cache_slot).is_null() {
                Z_Free(*cache_slot);
            }

            let target = lumpinfo.add(idx);
            (*target).position = filepos;
            (*target).size = size;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_InitMultipleFiles(mut filenames: *mut *mut c_char) {
    unsafe {
        numlumps = 0;
        lumpinfo = libc_malloc(1) as *mut LumpInfo;

        if !filenames.is_null() {
            while !(*filenames).is_null() {
                W_AddFile(*filenames);
                filenames = filenames.add(1);
            }
        }

        if numlumps == 0 {
            doom_error("W_InitFiles: no files found");
        }

        let size = (numlumps as usize) * size_of::<*mut c_void>();
        lumpcache = libc_malloc(size) as *mut *mut c_void;
        if lumpcache.is_null() {
            doom_error("Couldn't allocate lumpcache");
        }

        ptr::write_bytes(lumpcache as *mut u8, 0, size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_InitFile(filename: *mut c_char) {
    let mut names: [*mut c_char; 2] = [filename, ptr::null_mut()];
    unsafe {
        W_InitMultipleFiles(names.as_mut_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_NumLumps() -> c_int {
    unsafe { numlumps }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_CheckNumForName(name: *const c_char) -> c_int {
    if name.is_null() {
        return -1;
    }

    unsafe {
        // Копируем до 8 байт и переводим в верхний регистр (без учета неиспользуемого хвоста после '\0')
        let mut target_name = [0u8; 8];
        for i in 0..8 {
            let b = *name.add(i) as u8;
            if b == 0 {
                break;
            }
            target_name[i] = b.to_ascii_uppercase();
        }

        // Сканируем с конца, чтобы PWAD-файлы перекрывали ресурсы оригинального IWAD
        let mut i = numlumps;
        while i > 0 {
            i -= 1;
            let current = &(*lumpinfo.add(i as usize)).name;
            if current == &target_name {
                return i;
            }
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_GetNumForName(name: *const c_char) -> c_int {
    unsafe {
        let i = W_CheckNumForName(name);
        if i == -1 {
            let s = if name.is_null() {
                "NULL"
            } else {
                CStr::from_ptr(name).to_str().unwrap_or("INVALID")
            };
            doom_error(&format!("W_GetNumForName: {} not found!", s));
        }
    i
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_LumpLength(lump: c_int) -> c_int {
unsafe {
if lump < 0 || lump >= numlumps {
doom_error(&format!("W_LumpLength: {} >= numlumps", lump));
}
(*lumpinfo.add(lump as usize)).size
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_ReadLump(lump: c_int, dest: *mut c_void) {
unsafe {
if lump < 0 || lump >= numlumps {
doom_error(&format!("W_ReadLump: {} >= numlumps", lump));
}
let l = &*lumpinfo.add(lump as usize);
let size = l.size as usize;
let dest_slice = std::slice::from_raw_parts_mut(dest as *mut u8, size);
if l.handle == -1 {
let fname_str = CStr::from_ptr(reloadname).to_str().unwrap_or("");
let mut temp_file = File::open(fname_str)
.unwrap_or_else(|_| doom_error(&format!("W_ReadLump: couldn't open {}", fname_str)));
temp_file
.seek(SeekFrom::Start(l.position as u64))
.unwrap_or_else(|_| doom_error("W_ReadLump: seek error"));
temp_file
.read_exact(dest_slice)
.unwrap_or_else(|e| doom_error(&format!("W_ReadLump: read error ({})", e)));
} else {
let file = get_file_mut(l.handle)
.unwrap_or_else(|| doom_error(&format!("W_ReadLump: invalid handle {}", l.handle)));
file.seek(SeekFrom::Start(l.position as u64))
.unwrap_or_else(|_| doom_error("W_ReadLump: seek error"));
file.read_exact(dest_slice)
.unwrap_or_else(|e| doom_error(&format!("W_ReadLump: read error ({})", e)));
}
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_CacheLumpNum(lump: c_int, tag: c_int) -> *mut c_void {
unsafe {
if (lump as u32) >= (numlumps as u32) {
doom_error(&format!("W_CacheLumpNum: {} >= numlumps", lump));
}
let cache_slot = lumpcache.add(lump as usize);
if (*cache_slot).is_null() {
let len = W_LumpLength(lump);
// Выделяем память в Zone Memory и регистрируем cache_slot как владельца (user)
Z_Malloc(len, tag, cache_slot as *mut c_void);
W_ReadLump(lump, *cache_slot);
} else {
Z_ChangeTag2(*cache_slot, tag);
}
*cache_slot
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_CacheLumpName(name: *const c_char, tag: c_int) -> *mut c_void {
unsafe { W_CacheLumpNum(W_GetNumForName(name), tag) }
}

// --- Профилирование кэша (W_Profile) ---
static mut PROFILE_INFO: [[u8; 10]; 2500] = [[0; 10]; 2500];
static mut PROFILE_COUNT: usize = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn W_Profile() {
use crate::z_zone::MemBlock;
unsafe {
if PROFILE_COUNT >= 10 {
return;
}
let count = (numlumps as usize).min(2500);
for i in 0..count {
let ptr = *lumpcache.add(i);
let ch = if ptr.is_null() {
b' '
} else {
let block = (ptr as *mut u8).sub(size_of::<MemBlock>()) as *mut MemBlock;
if (*block).tag < PU_PURGELEVEL {
b'S'
} else {
b'P'
}
};
PROFILE_INFO[i][PROFILE_COUNT] = ch;
}
PROFILE_COUNT += 1;
if let Ok(mut out) = File::create("waddump.txt") {
for i in 0..count {
let raw_name = &(*lumpinfo.add(i)).name;
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
for j in 0..PROFILE_COUNT {
let _ = write!(out, "    {}", PROFILE_INFO[i][j] as char);
}
let _ = writeln!(out);
}
}
}
}