#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, CString, c_char, c_int, c_short, c_ushort, c_void};
use std::io::{self, Write};
use std::mem::size_of;
use std::ptr;

use crate::m_fixed::FRACBITS;
use crate::r_sky::skytexture;
use crate::r_things::{numsprites, sprites};
use crate::v_video::column_t;
use crate::w_wad::{
    W_CacheLumpName, W_CacheLumpNum, W_CheckNumForName, W_GetNumForName, W_LumpLength, W_ReadLump,
    lumpinfo,
};
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_ChangeTag2, Z_Free, Z_Malloc};

#[repr(C, packed)]
pub struct mappatch_t {
    pub originx: c_short,
    pub originy: c_short,
    pub patch: c_short,
    pub stepdir: c_short,
    pub colormap: c_short,
}

#[repr(C, packed)]
pub struct maptexture_t {
    pub name: [u8; 8],
    pub masked: c_int,
    pub width: c_short,
    pub height: c_short,
    pub columndirectory: c_int,
    pub patchcount: c_short,
    pub patches: [mappatch_t; 1],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct texpatch_t {
    pub originx: c_int,
    pub originy: c_int,
    pub patch: c_int,
}

#[repr(C)]
pub struct texture_t {
    pub name: [u8; 8],
    pub width: c_short,
    pub height: c_short,
    pub patchcount: c_short,
    pub patches: [texpatch_t; 1],
}

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn P_MobjThinker();

    static mut demoplayback: c_int;
    static mut numsectors: c_int;
    static mut sectors: *mut sector_t;
    static mut numsides: c_int;
    static mut sides: *mut side_t;
    static mut thinkercap: thinker_t;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Data error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные r_data.c ---
#[unsafe(no_mangle)]
pub static mut firstflat: c_int = 0;

#[unsafe(no_mangle)]
pub static mut lastflat: c_int = 0;

#[unsafe(no_mangle)]
pub static mut numflats: c_int = 0;

#[unsafe(no_mangle)]
pub static mut firstpatch: c_int = 0;

#[unsafe(no_mangle)]
pub static mut lastpatch: c_int = 0;

#[unsafe(no_mangle)]
pub static mut numpatches: c_int = 0;

#[unsafe(no_mangle)]
pub static mut firstspritelump: c_int = 0;

#[unsafe(no_mangle)]
pub static mut lastspritelump: c_int = 0;

#[unsafe(no_mangle)]
pub static mut numspritelumps: c_int = 0;

#[unsafe(no_mangle)]
pub static mut numtextures: c_int = 0;

#[unsafe(no_mangle)]
pub static mut textures: *mut *mut texture_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut texturewidthmask: *mut c_int = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut textureheight: *mut i32 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut texturecompositesize: *mut c_int = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut texturecolumnlump: *mut *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut texturecolumnofs: *mut *mut c_ushort = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut texturecomposite: *mut *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut flattranslation: *mut c_int = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut texturetranslation: *mut c_int = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut spritewidth: *mut i32 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut spriteoffset: *mut i32 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut spritetopoffset: *mut i32 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut colormaps: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut flatmemory: c_int = 0;

#[unsafe(no_mangle)]
pub static mut texturememory: c_int = 0;

#[unsafe(no_mangle)]
pub static mut spritememory: c_int = 0;

// --- R_DrawColumnInCache ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawColumnInCache(
    mut patch: *mut column_t,
    cache: *mut u8,
    originy: c_int,
    cacheheight: c_int,
) {
    unsafe {
        while (*patch).topdelta != 0xff {
            let source = (patch as *const u8).add(3);
            let mut count = (*patch).length as c_int;
            let mut position = originy + ((*patch).topdelta as c_int);

            if position < 0 {
                count += position;
                position = 0;
            }

            if position + count > cacheheight {
                count = cacheheight - position;
            }

            if count > 0 {
                ptr::copy_nonoverlapping(source, cache.add(position as usize), count as usize);
            }

            patch = (patch as *mut u8).add(((*patch).length as usize) + 4) as *mut column_t;
        }
    }
}

// --- R_GenerateComposite ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_GenerateComposite(texnum: c_int) {
    unsafe {
        let tex_idx = texnum as usize;
        let texture = *textures.add(tex_idx);
        let comp_size = *texturecompositesize.add(tex_idx);
        let user_slot = texturecomposite.add(tex_idx) as *mut c_void;

        let block = Z_Malloc(comp_size, PU_STATIC, user_slot) as *mut u8;

        let collump = *texturecolumnlump.add(tex_idx);
        let colofs = *texturecolumnofs.add(tex_idx);

        let patches_ptr = (&raw const (*texture).patches) as *const texpatch_t;
        let patchcount = (*texture).patchcount as usize;
        let tex_w = (*texture).width as c_int;
        let tex_h = (*texture).height as c_int;

        for i in 0..patchcount {
            let patch = &*patches_ptr.add(i);
            let realpatch = W_CacheLumpNum(patch.patch, PU_CACHE) as *mut patch_t;
            let x1 = patch.originx;
            let mut x2 = x1 + ((*realpatch).width as c_int);

            let mut x = if x1 < 0 { 0 } else { x1 };
            if x2 > tex_w {
                x2 = tex_w;
            }

            let col_ofs_arr = (&raw const (*realpatch).columnofs) as *const c_int;

            while x < x2 {
                let xu = x as usize;
                if *collump.add(xu) >= 0 {
                    x += 1;
                    continue;
                }

                let c_ofs = *col_ofs_arr.add((x - x1) as usize) as usize;
                let patchcol = (realpatch as *mut u8).add(c_ofs) as *mut column_t;
                let dest_col = block.add(*colofs.add(xu) as usize);

                R_DrawColumnInCache(patchcol, dest_col, patch.originy, tex_h);
                x += 1;
            }
        }

        Z_ChangeTag2(block as *mut c_void, PU_CACHE);
    }
}

// --- R_GenerateLookup ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_GenerateLookup(texnum: c_int) {
    unsafe {
        let tex_idx = texnum as usize;
        let texture = *textures.add(tex_idx);

        *texturecomposite.add(tex_idx) = ptr::null_mut();
        *texturecompositesize.add(tex_idx) = 0;

        let collump = *texturecolumnlump.add(tex_idx);
        let colofs = *texturecolumnofs.add(tex_idx);

        let tex_w = (*texture).width as usize;
        let tex_h = (*texture).height as c_int;
        let mut patchcount = vec![0u8; tex_w];

        let patches_ptr = (&raw const (*texture).patches) as *const texpatch_t;
        let pcount = (*texture).patchcount as usize;

        for i in 0..pcount {
            let patch = &*patches_ptr.add(i);
            let realpatch = W_CacheLumpNum(patch.patch, PU_CACHE) as *mut patch_t;
            let x1 = patch.originx;
            let mut x2 = x1 + ((*realpatch).width as c_int);

            let mut x = if x1 < 0 { 0 } else { x1 };
            if x2 > (tex_w as c_int) {
                x2 = tex_w as c_int;
            }

            let col_ofs_arr = (&raw const (*realpatch).columnofs) as *const c_int;

            while x < x2 {
                let xu = x as usize;
                patchcount[xu] += 1;
                *collump.add(xu) = patch.patch as c_short;
                let c_ofs = *col_ofs_arr.add((x - x1) as usize);
                *colofs.add(xu) = (c_ofs + 3) as c_ushort;
                x += 1;
            }
        }

        for x in 0..tex_w {
            if patchcount[x] == 0 {
                let name_str = std::str::from_utf8(&(*texture).name)
                    .unwrap_or("????????")
                    .trim_end_matches('\0');
                println!("R_GenerateLookup: column without a patch ({})", name_str);
                return;
            }

            if patchcount[x] > 1 {
                *collump.add(x) = -1;
                let comp_size = *texturecompositesize.add(tex_idx);
                *colofs.add(x) = comp_size as c_ushort;

                if comp_size > 0x10000 - tex_h {
                    doom_error(&format!("R_GenerateLookup: texture {} is >64k", texnum));
                }

                *texturecompositesize.add(tex_idx) = comp_size + tex_h;
            }
        }
    }
}

// --- R_GetColumn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_GetColumn(tex: c_int, mut col: c_int) -> *mut u8 {
    unsafe {
        let tex_idx = tex as usize;
        col &= *texturewidthmask.add(tex_idx);
        let col_idx = col as usize;

        let lump = *(*texturecolumnlump.add(tex_idx)).add(col_idx) as c_int;
        let ofs = *(*texturecolumnofs.add(tex_idx)).add(col_idx) as usize;

        if lump > 0 {
            let lump_ptr = W_CacheLumpNum(lump, PU_CACHE) as *mut u8;
            return lump_ptr.add(ofs);
        }

        let comp_slot = texturecomposite.add(tex_idx);
        if (*comp_slot).is_null() {
            R_GenerateComposite(tex);
        }

        (*comp_slot).add(ofs)
    }
}

// --- R_InitTextures ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitTextures() {
    unsafe {
        let pnames_lump = b"PNAMES\0".as_ptr() as *const c_char;
        let names = W_CacheLumpName(pnames_lump, PU_STATIC) as *mut u8;
        let nummappatches = ptr::read_unaligned(names as *const i32);
let name_p = names.add(4);
let mut patchlookup = vec![-1 as c_int; nummappatches as usize];
let mut name_buf = [0u8; 9];
for i in 0..(nummappatches as usize) {
ptr::copy_nonoverlapping(name_p.add(i * 8), name_buf.as_mut_ptr(), 8);
*name_buf.as_mut_ptr().add(8) = 0;
patchlookup[i] = W_CheckNumForName(name_buf.as_ptr() as *const c_char);
}
Z_Free(names as *mut c_void);
let tex1_lump = b"TEXTURE1\0".as_ptr() as *const c_char;
let maptex1 = W_CacheLumpName(tex1_lump, PU_STATIC) as *mut i32;
let mut maptex = maptex1;
let numtextures1 = ptr::read_unaligned(maptex);
let mut maxoff = W_LumpLength(W_GetNumForName(tex1_lump));
let mut directory = maptex.add(1);
let tex2_lump = b"TEXTURE2\0".as_ptr() as *const c_char;
let tex2_num = W_CheckNumForName(tex2_lump);
let (maptex2, numtextures2, maxoff2) = if tex2_num != -1 {
let m2 = W_CacheLumpName(tex2_lump, PU_STATIC) as *mut i32;
let n2 = ptr::read_unaligned(m2);
let off2 = W_LumpLength(tex2_num);
(m2, n2, off2)
} else {
(ptr::null_mut(), 0, 0)
};
numtextures = numtextures1 + numtextures2;
let n_tex = numtextures as usize;
textures = Z_Malloc((n_tex * size_of::<*mut texture_t>()) as c_int, PU_STATIC, ptr::null_mut())
as *mut *mut texture_t;
texturecolumnlump =
Z_Malloc((n_tex * size_of::<*mut c_short>()) as c_int, PU_STATIC, ptr::null_mut())
as *mut *mut c_short;
texturecolumnofs =
Z_Malloc((n_tex * size_of::<*mut c_ushort>()) as c_int, PU_STATIC, ptr::null_mut())
as *mut *mut c_ushort;
texturecomposite =
Z_Malloc((n_tex * size_of::<*mut u8>()) as c_int, PU_STATIC, ptr::null_mut())
as *mut *mut u8;
texturecompositesize =
Z_Malloc((n_tex * size_of::<c_int>()) as c_int, PU_STATIC, ptr::null_mut())
as *mut c_int;
texturewidthmask =
Z_Malloc((n_tex * size_of::<c_int>()) as c_int, PU_STATIC, ptr::null_mut())
as *mut c_int;
textureheight =
Z_Malloc((n_tex * size_of::<i32>()) as c_int, PU_STATIC, ptr::null_mut()) as *mut i32;
let s_start = W_GetNumForName(b"S_START\0".as_ptr() as *const c_char);
let s_end = W_GetNumForName(b"S_END\0".as_ptr() as *const c_char) - 1;
let temp3 = ((s_end - s_start + 63) / 64) + ((numtextures + 63) / 64);
print!("[");
for _ in 0..temp3 {
print!(" ");
}
print!("         ]");
for _ in 0..temp3 {
print!("\x08");
}
print!("\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08");
let _ = io::stdout().flush();
for i in 0..numtextures {
if (i & 63) == 0 {
print!(".");
let _ = io::stdout().flush();
}
if i == numtextures1 {
maptex = maptex2;
maxoff = maxoff2;
directory = maptex.add(1);
}
let offset = ptr::read_unaligned(directory);
directory = directory.add(1);
if offset > maxoff {
doom_error("R_InitTextures: bad texture directory");
}
let mtexture = (maptex as *const u8).add(offset as usize) as *const maptexture_t;
let patchcount = ptr::read_unaligned(&raw const (*mtexture).patchcount);
let width = ptr::read_unaligned(&raw const (*mtexture).width);
let height = ptr::read_unaligned(&raw const (*mtexture).height);
let tex_bytes =
size_of::<texture_t>() + size_of::<texpatch_t>() * ((patchcount as usize) - 1);
let texture = Z_Malloc(tex_bytes as c_int, PU_STATIC, ptr::null_mut()) as *mut texture_t;
let idx = i as usize;
*textures.add(idx) = texture;
(*texture).width = width;
(*texture).height = height;
(*texture).patchcount = patchcount;
(*texture).name = ptr::read_unaligned(&raw const (*mtexture).name);
let mpatch_base = (&raw const (*mtexture).patches) as *const mappatch_t;
let patch_base = (&raw mut (*texture).patches) as *mut texpatch_t;
for j in 0..(patchcount as usize) {
let mp = mpatch_base.add(j);
let p = patch_base.add(j);
let ox = ptr::read_unaligned(&raw const (*mp).originx);
let oy = ptr::read_unaligned(&raw const (*mp).originy);
let p_idx = ptr::read_unaligned(&raw const (*mp).patch) as usize;
(*p).originx = ox as c_int;
(*p).originy = oy as c_int;
(*p).patch = patchlookup[p_idx];
if (*p).patch == -1 {
let tname = std::str::from_utf8(&(*texture).name)
.unwrap_or("????????")
.trim_end_matches('\0');
doom_error(&format!("R_InitTextures: Missing patch in texture {}", tname));
}
}
*texturecolumnlump.add(idx) =
Z_Malloc((width as c_int) * 2, PU_STATIC, ptr::null_mut()) as *mut c_short;
*texturecolumnofs.add(idx) =
Z_Malloc((width as c_int) * 2, PU_STATIC, ptr::null_mut()) as *mut c_ushort;
let mut j = 1;
while j * 2 <= width {
j <<= 1;
}
*texturewidthmask.add(idx) = (j - 1) as c_int;
*textureheight.add(idx) = (height as i32) << FRACBITS;
}
Z_Free(maptex1 as *mut c_void);
if !maptex2.is_null() {
Z_Free(maptex2 as *mut c_void);
}
for i in 0..numtextures {
R_GenerateLookup(i);
}
texturetranslation =
Z_Malloc(((numtextures + 1) * 4) as c_int, PU_STATIC, ptr::null_mut()) as *mut c_int;
for i in 0..numtextures {
*texturetranslation.add(i as usize) = i;
}
}
}
// --- R_InitFlats ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitFlats() {
unsafe {
firstflat = W_GetNumForName(b"F_START\0".as_ptr() as *const c_char) + 1;
lastflat = W_GetNumForName(b"F_END\0".as_ptr() as *const c_char) - 1;
numflats = lastflat - firstflat + 1;
flattranslation =
Z_Malloc(((numflats + 1) * 4) as c_int, PU_STATIC, ptr::null_mut()) as *mut c_int;
for i in 0..numflats {
*flattranslation.add(i as usize) = i;
}
}
}
// --- R_InitSpriteLumps ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitSpriteLumps() {
unsafe {
firstspritelump = W_GetNumForName(b"S_START\0".as_ptr() as *const c_char) + 1;
lastspritelump = W_GetNumForName(b"S_END\0".as_ptr() as *const c_char) - 1;
numspritelumps = lastspritelump - firstspritelump + 1;
spritewidth = Z_Malloc(numspritelumps * 4, PU_STATIC, ptr::null_mut()) as *mut i32;
spriteoffset = Z_Malloc(numspritelumps * 4, PU_STATIC, ptr::null_mut()) as *mut i32;
spritetopoffset = Z_Malloc(numspritelumps * 4, PU_STATIC, ptr::null_mut()) as *mut i32;
for i in 0..numspritelumps {
if (i & 63) == 0 {
print!(".");
let _ = io::stdout().flush();
}
let patch = W_CacheLumpNum(firstspritelump + i, PU_CACHE) as *mut patch_t;
let idx = i as usize;
*spritewidth.add(idx) = ((*patch).width as i32) << FRACBITS;
*spriteoffset.add(idx) = ((*patch).leftoffset as i32) << FRACBITS;
*spritetopoffset.add(idx) = ((*patch).topoffset as i32) << FRACBITS;
}
}
}
// --- R_InitColormaps ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitColormaps() {
unsafe {
let lump = W_GetNumForName(b"COLORMAP\0".as_ptr() as *const c_char);
let length = W_LumpLength(lump) + 255;
let raw_ptr = Z_Malloc(length, PU_STATIC, ptr::null_mut()) as usize;
colormaps = ((raw_ptr + 255) & !255) as *mut u8;
W_ReadLump(lump, colormaps as *mut c_void);
}
}
// --- R_InitData ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitData() {
unsafe {
R_InitTextures();
print!("\nInitTextures");
R_InitFlats();
print!("\nInitFlats");
R_InitSpriteLumps();
print!("\nInitSprites");
R_InitColormaps();
print!("\nInitColormaps");
let _ = io::stdout().flush();
}
}
// --- R_FlatNumForName ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_FlatNumForName(name: *mut c_char) -> c_int {
unsafe {
let i = W_CheckNumForName(name);
if i == -1 {
let mut namet = [0u8; 9];
ptr::copy_nonoverlapping(name as *const u8, namet.as_mut_ptr(), 8);
let s = CStr::from_ptr(namet.as_ptr() as *const c_char)
.to_str()
.unwrap_or("????????");
doom_error(&format!("R_FlatNumForName: {} not found", s));
}
i - firstflat
}
}
// --- R_CheckTextureNumForName ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_CheckTextureNumForName(name: *mut c_char) -> c_int {
if name.is_null() {
return -1;
}
unsafe {
if *name == b'-' as c_char {
return 0;
}
let target = std::slice::from_raw_parts(name as *const u8, 8);
let target_len = target.iter().position(|&b| b == 0).unwrap_or(8);
let target_slice = &target[..target_len];
for i in 0..numtextures {
let tex = *textures.add(i as usize);
let tex_name = &(*tex).name;
let tex_len = tex_name.iter().position(|&b| b == 0).unwrap_or(8);
if tex_len == target_len && tex_name[..tex_len].eq_ignore_ascii_case(target_slice) {
return i;
}
}
}
-1
}
// --- R_TextureNumForName ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_TextureNumForName(name: *mut c_char) -> c_int {
unsafe {
let i = R_CheckTextureNumForName(name);
if i == -1 {
let s = if name.is_null() {
"NULL"
} else {
CStr::from_ptr(name).to_str().unwrap_or("????????")
};
doom_error(&format!("R_TextureNumForName: {} not found", s));
}
i
}
}
// --- R_PrecacheLevel ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PrecacheLevel() {
unsafe {
if demoplayback != 0 {
return;
}
let mut flatpresent = vec![0u8; numflats as usize];
for i in 0..(numsectors as usize) {
let sec = sectors.add(i);
flatpresent[(*sec).floorpic as usize] = 1;
flatpresent[(*sec).ceilingpic as usize] = 1;
}
flatmemory = 0;
for i in 0..(numflats as usize) {
if flatpresent[i] != 0 {
let lump = firstflat + (i as c_int);
flatmemory += (*lumpinfo.add(lump as usize)).size;
W_CacheLumpNum(lump, PU_CACHE);
}
}
let mut texturepresent = vec![0u8; numtextures as usize];
for i in 0..(numsides as usize) {
let side = sides.add(i);
texturepresent[(*side).toptexture as usize] = 1;
texturepresent[(*side).midtexture as usize] = 1;
texturepresent[(*side).bottomtexture as usize] = 1;
}
texturepresent[skytexture as usize] = 1;
texturememory = 0;
for i in 0..(numtextures as usize) {
if texturepresent[i] == 0 {
continue;
}
let texture = *textures.add(i);
let patches_ptr = (&raw const (*texture).patches) as *const texpatch_t;
let pcount = (*texture).patchcount as usize;
for j in 0..pcount {
let lump = (*patches_ptr.add(j)).patch;
texturememory += (*lumpinfo.add(lump as usize)).size;
W_CacheLumpNum(lump, PU_CACHE);
}
}
let mut spritepresent = vec![0u8; numsprites as usize];
let cap_ptr = &raw mut thinkercap;
let mut th = (*cap_ptr).next;
let mobj_thinker_ptr = P_MobjThinker as *const ();
while th != cap_ptr {
if let Some(func) = (*th).function.acp1 {
if (func as *const ()) == mobj_thinker_ptr {
let mobj = th as *const mobj_t;
spritepresent[(*mobj).sprite as usize] = 1;
}
}
th = (*th).next;
}
spritememory = 0;
for i in 0..(numsprites as usize) {
if spritepresent[i] == 0 {
continue;
}
let spr = sprites.add(i);
for j in 0..((*spr).numframes as usize) {
let sf = (*spr).spriteframes.add(j);
let lump_ptr = (&raw const (*sf).lump) as *const i16;
for k in 0..8 {
let lump = firstspritelump + (*lump_ptr.add(k) as c_int);
spritememory += (*lumpinfo.add(lump as usize)).size;
W_CacheLumpNum(lump, PU_CACHE);
}
}
}
}
}