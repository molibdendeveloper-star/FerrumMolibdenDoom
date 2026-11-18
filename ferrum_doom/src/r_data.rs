#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, CString, c_char, c_int, c_short, c_ushort, c_void};
use std::io::{self, Write};
use std::mem::size_of;
use std::ptr;

use crate::m_fixed::FRACBITS;
use crate::r_sky::get_sky_texture;
use crate::r_things::{numsprites, sprites};
use crate::v_video::column_t;
use crate::w_wad::{
    W_CacheLumpNum, W_LumpLength, W_ReadLump, lumpinfo, w_check_num_for_name, w_get_num_for_name,
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

unsafe fn alloc_static_slice<T>(count: usize) -> (*mut T, &'static mut [T]) {
    unsafe {
        let bytes = (count * size_of::<T>()).max(8);
        let raw = Z_Malloc(bytes as c_int, PU_STATIC, ptr::null_mut()) as *mut T;
        (raw, std::slice::from_raw_parts_mut(raw, count))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawColumnInCache(
    mut patch: *mut column_t,
    cache: *mut u8,
    originy: c_int,
    cacheheight: c_int,
) {
    unsafe {
        let dst_col = std::slice::from_raw_parts_mut(cache, cacheheight.max(0) as usize);
        while (*patch).topdelta != 0xff {
            let mut count = (*patch).length as c_int;
            let mut position = originy + ((*patch).topdelta as c_int);
            let mut src_skip = 0usize;

            if position < 0 {
                src_skip = (-position) as usize;
                count += position;
                position = 0;
            }

            if position + count > cacheheight {
                count = cacheheight - position;
            }

            if count > 0 {
                let len = count as usize;
                let src = std::slice::from_raw_parts((patch as *const u8).add(3 + src_skip), len);
                let pos = position as usize;
                dst_col[pos..pos + len].copy_from_slice(src);
            }

            patch = (patch as *mut u8).add(((*patch).length as usize) + 4) as *mut column_t;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_GenerateComposite(texnum: c_int) {
    unsafe {
        let tex_idx = texnum as usize;
        let texture = &**textures.add(tex_idx);
        let comp_size = *texturecompositesize.add(tex_idx);
        let user_slot = texturecomposite.add(tex_idx) as *mut c_void;

        let block = Z_Malloc(comp_size, PU_STATIC, user_slot) as *mut u8;
        let tex_w = texture.width as c_int;
        let tex_h = texture.height as c_int;

        let collump = std::slice::from_raw_parts(*texturecolumnlump.add(tex_idx), tex_w as usize);
        let colofs = std::slice::from_raw_parts(*texturecolumnofs.add(tex_idx), tex_w as usize);
        let patches =
            std::slice::from_raw_parts(texture.patches.as_ptr(), texture.patchcount as usize);

        for patch in patches {
            let realpatch = W_CacheLumpNum(patch.patch, PU_CACHE) as *mut patch_t;
            let rp_ref = &*realpatch;
            let x1 = patch.originx;
            let x2 = (x1 + (rp_ref.width as c_int)).min(tex_w);
            let col_ofs_arr =
                std::slice::from_raw_parts(rp_ref.columnofs.as_ptr(), rp_ref.width.max(0) as usize);

            for x in x1.max(0)..x2 {
                let xu = x as usize;
                if collump[xu] >= 0 {
                    continue;
                }
                let c_ofs = col_ofs_arr[(x - x1) as usize] as usize;
                let patchcol = (realpatch as *mut u8).add(c_ofs) as *mut column_t;
                let dest_col = block.add(colofs[xu] as usize);

                R_DrawColumnInCache(patchcol, dest_col, patch.originy, tex_h);
            }
        }

        Z_ChangeTag2(block as *mut c_void, PU_CACHE);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_GenerateLookup(texnum: c_int) {
    unsafe {
        let tex_idx = texnum as usize;
        let texture = &**textures.add(tex_idx);

        *texturecomposite.add(tex_idx) = ptr::null_mut();
        *texturecompositesize.add(tex_idx) = 0;

        let tex_w = texture.width as usize;
        let tex_h = texture.height as c_int;
        let collump = std::slice::from_raw_parts_mut(*texturecolumnlump.add(tex_idx), tex_w);
        let colofs = std::slice::from_raw_parts_mut(*texturecolumnofs.add(tex_idx), tex_w);
        let mut patchcount = vec![0u8; tex_w];

        let patches =
            std::slice::from_raw_parts(texture.patches.as_ptr(), texture.patchcount as usize);

        for patch in patches {
            let realpatch = W_CacheLumpNum(patch.patch, PU_CACHE) as *mut patch_t;
            let rp_ref = &*realpatch;
            let x1 = patch.originx;
            let x2 = (x1 + (rp_ref.width as c_int)).min(tex_w as c_int);
            let col_ofs_arr =
                std::slice::from_raw_parts(rp_ref.columnofs.as_ptr(), rp_ref.width.max(0) as usize);

            for x in x1.max(0)..x2 {
                let xu = x as usize;
                patchcount[xu] += 1;
                collump[xu] = patch.patch as c_short;
                let c_ofs = col_ofs_arr[(x - x1) as usize];
                colofs[xu] = (c_ofs + 3) as c_ushort;
            }
        }

        for x in 0..tex_w {
            if patchcount[x] == 0 {
                let name_str = std::str::from_utf8(&texture.name)
                    .unwrap_or("????????")
                    .trim_end_matches('\0');
                println!("R_GenerateLookup: column without a patch ({})", name_str);
                return;
            }

            if patchcount[x] > 1 {
                collump[x] = -1;
                let comp_size = *texturecompositesize.add(tex_idx);
                colofs[x] = comp_size as c_ushort;

                if comp_size > 0x10000 - tex_h {
                    doom_error(&format!("R_GenerateLookup: texture {} is >64k", texnum));
                }

                *texturecompositesize.add(tex_idx) = comp_size + tex_h;
            }
        }
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitTextures() {
let pnames_lump = w_get_num_for_name("PNAMES");
let pnames_len = W_LumpLength(pnames_lump) as usize;
let names_raw = W_CacheLumpNum(pnames_lump, PU_STATIC) as *mut u8;
let patchlookup: Vec<c_int> = unsafe {
let bytes = std::slice::from_raw_parts(names_raw, pnames_len);
let nummappatches = i32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
(0..nummappatches)
.map(|i| {
let raw_name = &bytes[4 + i * 8..4 + (i + 1) * 8];
let len = raw_name.iter().position(|&b| b == 0).unwrap_or(8);
let s = std::str::from_utf8(&raw_name[..len]).unwrap_or("");
w_check_num_for_name(s)
})
.collect()
};
Z_Free(names_raw as *mut c_void);
let tex1_lump = w_get_num_for_name("TEXTURE1");
let maptex1 = W_CacheLumpNum(tex1_lump, PU_STATIC) as *mut i32;
let numtextures1 = unsafe { ptr::read_unaligned(maptex1) };
let maxoff1 = W_LumpLength(tex1_lump);
let tex2_num = w_check_num_for_name("TEXTURE2");
let (maptex2, numtextures2, maxoff2) = if tex2_num != -1 {
let m2 = W_CacheLumpNum(tex2_num, PU_STATIC) as *mut i32;
let n2 = unsafe { ptr::read_unaligned(m2) };
let off2 = W_LumpLength(tex2_num);
(m2, n2, off2)
} else {
(ptr::null_mut(), 0, 0)
};
unsafe {
numtextures = numtextures1 + numtextures2;
let n_tex = numtextures as usize;
let (tex_ptr, tex_slice) = alloc_static_slice::<*mut texture_t>(n_tex);
textures = tex_ptr;
let (clump_ptr, clump_slice) = alloc_static_slice::<*mut c_short>(n_tex);
texturecolumnlump = clump_ptr;
let (cofs_ptr, cofs_slice) = alloc_static_slice::<*mut c_ushort>(n_tex);
texturecolumnofs = cofs_ptr;
let (comp_ptr, _) = alloc_static_slice::<*mut u8>(n_tex);
texturecomposite = comp_ptr;
let (csize_ptr, _) = alloc_static_slice::<c_int>(n_tex);
texturecompositesize = csize_ptr;
let (wmask_ptr, wmask_slice) = alloc_static_slice::<c_int>(n_tex);
texturewidthmask = wmask_ptr;
let (theight_ptr, theight_slice) = alloc_static_slice::<i32>(n_tex);
textureheight = theight_ptr;
let s_start = w_get_num_for_name("S_START");
let s_end = w_get_num_for_name("S_END") - 1;
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
let mut maptex = maptex1;
let mut maxoff = maxoff1;
let mut directory = maptex.add(1);
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
tex_slice[idx] = texture;
(*texture).width = width;
(*texture).height = height;
(*texture).patchcount = patchcount;
(*texture).name = ptr::read_unaligned(&raw const (*mtexture).name);
let mpatch_base = (&raw const (*mtexture).patches) as *const mappatch_t;
let patch_dst =
std::slice::from_raw_parts_mut((*texture).patches.as_mut_ptr(), patchcount as usize);
for (j, p) in patch_dst.iter_mut().enumerate() {
let mp = mpatch_base.add(j);
let ox = ptr::read_unaligned(&raw const (*mp).originx);
let oy = ptr::read_unaligned(&raw const (*mp).originy);
let p_idx = ptr::read_unaligned(&raw const (*mp).patch) as usize;
p.originx = ox as c_int;
p.originy = oy as c_int;
p.patch = patchlookup[p_idx];
if p.patch == -1 {
let tname = std::str::from_utf8(&(*texture).name)
.unwrap_or("????????")
.trim_end_matches('\0');
doom_error(&format!("R_InitTextures: Missing patch in texture {}", tname));
}
}
clump_slice[idx] =
Z_Malloc((width as c_int) * 2, PU_STATIC, ptr::null_mut()) as *mut c_short;
cofs_slice[idx] =
Z_Malloc((width as c_int) * 2, PU_STATIC, ptr::null_mut()) as *mut c_ushort;
let mut j = 1;
while j * 2 <= width {
j <<= 1;
}
wmask_slice[idx] = (j - 1) as c_int;
theight_slice[idx] = (height as i32) << FRACBITS;
}
Z_Free(maptex1 as *mut c_void);
if !maptex2.is_null() {
Z_Free(maptex2 as *mut c_void);
}
for i in 0..numtextures {
R_GenerateLookup(i);
}
let (ttrans_ptr, ttrans_slice) = alloc_static_slice::<c_int>((numtextures + 1) as usize);
texturetranslation = ttrans_ptr;
for (i, slot) in ttrans_slice[..n_tex].iter_mut().enumerate() {
*slot = i as c_int;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitFlats() {
unsafe {
firstflat = w_get_num_for_name("F_START") + 1;
lastflat = w_get_num_for_name("F_END") - 1;
numflats = lastflat - firstflat + 1;
let n_flats = numflats as usize;
let (ftrans_ptr, ftrans_slice) = alloc_static_slice::<c_int>(n_flats + 1);
flattranslation = ftrans_ptr;
for (i, slot) in ftrans_slice[..n_flats].iter_mut().enumerate() {
*slot = i as c_int;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitSpriteLumps() {
unsafe {
firstspritelump = w_get_num_for_name("S_START") + 1;
lastspritelump = w_get_num_for_name("S_END") - 1;
numspritelumps = lastspritelump - firstspritelump + 1;
let n_spr = numspritelumps as usize;
let (sw_ptr, sw_slice) = alloc_static_slice::<i32>(n_spr);
let (so_ptr, so_slice) = alloc_static_slice::<i32>(n_spr);
let (sto_ptr, sto_slice) = alloc_static_slice::<i32>(n_spr);
spritewidth = sw_ptr;
spriteoffset = so_ptr;
spritetopoffset = sto_ptr;
for i in 0..n_spr {
if (i & 63) == 0 {
print!(".");
let _ = io::stdout().flush();
}
let patch = &*(W_CacheLumpNum(firstspritelump + (i as c_int), PU_CACHE) as *const patch_t);
sw_slice[i] = (patch.width as i32) << FRACBITS;
so_slice[i] = (patch.leftoffset as i32) << FRACBITS;
sto_slice[i] = (patch.topoffset as i32) << FRACBITS;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitColormaps() {
let lump = w_get_num_for_name("COLORMAP");
let length = W_LumpLength(lump) + 255;
let raw_ptr = Z_Malloc(length, PU_STATIC, ptr::null_mut()) as usize;
unsafe {
colormaps = ((raw_ptr + 255) & !255) as *mut u8;
W_ReadLump(lump, colormaps as *mut c_void);
}
}
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_FlatNumForName(name: *mut c_char) -> c_int {
if name.is_null() {
doom_error("R_FlatNumForName: NULL");
}
let bytes = unsafe { std::slice::from_raw_parts(name as *const u8, 8) };
let len = bytes.iter().position(|&b| b == 0).unwrap_or(8);
let s = std::str::from_utf8(&bytes[..len]).unwrap_or("????????");
let i = w_check_num_for_name(s);
if i == -1 {
doom_error(&format!("R_FlatNumForName: {} not found", s));
}
i - unsafe { firstflat }
}
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
let tex_slice = std::slice::from_raw_parts(textures, numtextures as usize);
for (i, &tex_ptr) in tex_slice.iter().enumerate() {
let tex_name = &(*tex_ptr).name;
let tex_len = tex_name.iter().position(|&b| b == 0).unwrap_or(8);
if tex_len == target_len && tex_name[..tex_len].eq_ignore_ascii_case(target_slice) {
return i as c_int;
}
}
}
-1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_TextureNumForName(name: *mut c_char) -> c_int {
let i = unsafe { R_CheckTextureNumForName(name) };
if i == -1 {
let s = if name.is_null() {
"NULL"
} else {
unsafe { CStr::from_ptr(name).to_str().unwrap_or("????????") }
};
doom_error(&format!("R_TextureNumForName: {} not found", s));
}
i
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_PrecacheLevel() {
unsafe {
if demoplayback != 0 {
return;
}
let lumps = std::slice::from_raw_parts(lumpinfo, crate::w_wad::numlumps as usize);
let mut flatpresent = vec![false; numflats as usize];
let secs = std::slice::from_raw_parts(sectors, numsectors as usize);
for sec in secs {
flatpresent[sec.floorpic as usize] = true;
flatpresent[sec.ceilingpic as usize] = true;
}
flatmemory = 0;
for (i, &present) in flatpresent.iter().enumerate() {
if present {
let lump = firstflat + (i as c_int);
flatmemory += lumps[lump as usize].size;
W_CacheLumpNum(lump, PU_CACHE);
}
}
let mut texturepresent = vec![false; numtextures as usize];
let sds = std::slice::from_raw_parts(sides, numsides as usize);
for side in sds {
texturepresent[side.toptexture as usize] = true;
texturepresent[side.midtexture as usize] = true;
texturepresent[side.bottomtexture as usize] = true;
}
texturepresent[get_sky_texture() as usize] = true;
texturememory = 0;
let tex_slice = std::slice::from_raw_parts(textures, numtextures as usize);
for (i, &present) in texturepresent.iter().enumerate() {
if !present {
continue;
}
let texture = &*tex_slice[i];
let patches =
std::slice::from_raw_parts(texture.patches.as_ptr(), texture.patchcount as usize);
for p in patches {
texturememory += lumps[p.patch as usize].size;
W_CacheLumpNum(p.patch, PU_CACHE);
}
}
let mut spritepresent = vec![false; numsprites as usize];
let cap_ptr = &raw mut thinkercap;
let mut th = (*cap_ptr).next;
let mobj_thinker_ptr = P_MobjThinker as *const ();
while th != cap_ptr {
if let Some(func) = (*th).function.acp1 {
if (func as *const ()) == mobj_thinker_ptr {
let mobj = (th as *const mobj_t).as_ref().unwrap();
spritepresent[mobj.sprite as usize] = true;
}
}
th = (*th).next;
}
spritememory = 0;
let spr_slice = std::slice::from_raw_parts(sprites, numsprites as usize);
for (i, &present) in spritepresent.iter().enumerate() {
if !present {
continue;
}
let spr = &spr_slice[i];
let frames =
std::slice::from_raw_parts(spr.spriteframes, spr.numframes as usize);
for sf in frames {
for &lump_idx in &sf.lump {
let lump = firstspritelump + (lump_idx as c_int);
spritememory += lumps[lump as usize].size;
W_CacheLumpNum(lump, PU_CACHE);
}
}
}
}
}
