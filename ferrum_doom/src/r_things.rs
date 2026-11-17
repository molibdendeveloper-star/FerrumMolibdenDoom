#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;

use std::ffi::{CStr, CString, c_char, c_int, c_short, c_uint};
use std::mem::size_of;
use std::ptr;

use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::r_bsp::{drawsegs, ds_p};
use crate::r_draw::{
    R_DrawTranslatedColumn, dc_colormap, dc_iscale, dc_source, dc_texturemid,
    dc_translation, dc_x, dc_yh, dc_yl, translationtables, viewheight, viewwidth,
};
use crate::r_segs::R_RenderMaskedSegRange;
use crate::v_video::{column_t, patch_t};
use crate::w_wad::{W_CacheLumpNum, W_GetNumForName, lumpinfo};
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_Malloc};

pub const MAXVISSPRITES: usize = 128;
pub const NUMPSPRITES: usize = 2;
const MINZ: i32 = (FRACUNIT as i32) * 4;
const BASEYCENTER: i32 = 100;
const SCR_W: usize = SCREENWIDTH as usize;

#[repr(C)]
pub struct player_s {
    pub mo: *mut mobj_t,
    pub playerstate: c_int,
    pub cmd: [u8; 8],
    pub viewz: i32,
    pub viewheight: i32,
    pub deltaviewheight: i32,
    pub bob: i32,
    pub health: c_int,
    pub armorpoints: c_int,
    pub armortype: c_int,
    pub powers: [c_int; 6],
    pub cards: [c_int; 6],
    pub backpack: c_int,
    pub frags: [c_int; 4],
    pub readyweapon: c_int,
    pub pendingweapon: c_int,
    pub weaponowned: [c_int; 9],
    pub ammo: [c_int; 4],
    pub maxammo: [c_int; 4],
    pub attackdown: c_int,
    pub usedown: c_int,
    pub cheats: c_int,
    pub refire: c_int,
    pub killcount: c_int,
    pub itemcount: c_int,
    pub secretcount: c_int,
    pub message: *mut c_char,
    pub damagecount: c_int,
    pub bonuscount: c_int,
    pub attacker: *mut mobj_t,
    pub extralight: c_int,
    pub fixedcolormap: c_int,
    pub colormap: c_int,
    pub psprites: [pspdef_t; NUMPSPRITES],
    pub didsecret: c_int,
}
pub type player_t = player_s;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn R_PointToAngle(x: i32, y: i32) -> u32;
    fn R_PointOnSegSide(x: i32, y: i32, line: *mut seg_t) -> c_int;

    static mut viewx: i32;
    static mut viewy: i32;
    static mut viewz: i32;
    static mut viewcos: i32;
    static mut viewsin: i32;
    static mut projection: i32;
    static mut centerxfrac: i32;
    static mut centeryfrac: i32;
    static mut detailshift: c_int;
    static mut extralight: c_int;
    static mut validcount: c_int;
    static mut viewangleoffset: c_int;
    static mut viewplayer: *mut player_t;

    static mut firstspritelump: c_int;
    static mut lastspritelump: c_int;
    static mut spriteoffset: *mut i32;
    static mut spritewidth: *mut i32;
    static mut spritetopoffset: *mut i32;
    static mut modifiedgame: c_int;

    static mut colormaps: *mut u8;
    static mut fixedcolormap: *mut u8;
    static mut scalelight: [[*mut u8; MAXLIGHTSCALE as usize]; LIGHTLEVELS as usize];

    static mut colfunc: Option<unsafe extern "C" fn()>;
    static mut basecolfunc: Option<unsafe extern "C" fn()>;
    static mut fuzzcolfunc: Option<unsafe extern "C" fn()>;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Things error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные r_things.c ---
#[unsafe(no_mangle)]
pub static mut pspritescale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut pspriteiscale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut spritelights: *mut *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut negonearray: [c_short; SCR_W] = [0; SCR_W];

#[unsafe(no_mangle)]
pub static mut screenheightarray: [c_short; SCR_W] = [0; SCR_W];

#[unsafe(no_mangle)]
pub static mut sprites: *mut spritedef_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numsprites: c_int = 0;

#[unsafe(no_mangle)]
pub static mut sprtemp: [spriteframe_t; 29] = [unsafe { std::mem::zeroed() }; 29];

#[unsafe(no_mangle)]
pub static mut maxframe: c_int = 0;

#[unsafe(no_mangle)]
pub static mut spritename: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut vissprites: [vissprite_t; MAXVISSPRITES] =
    [unsafe { std::mem::zeroed() }; MAXVISSPRITES];

#[unsafe(no_mangle)]
pub static mut vissprite_p: *mut vissprite_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut newvissprite: c_int = 0;

#[unsafe(no_mangle)]
pub static mut overflowsprite: vissprite_t = unsafe { std::mem::zeroed() };

#[unsafe(no_mangle)]
pub static mut mfloorclip: *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut mceilingclip: *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut spryscale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut sprtopscreen: i32 = 0;

#[unsafe(no_mangle)]
pub static mut vsprsortedhead: vissprite_t = unsafe { std::mem::zeroed() };

// --- R_InstallSpriteLump ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InstallSpriteLump(
    lump: c_int,
    frame: c_uint,
    mut rotation: c_uint,
    flipped: c_int,
) {
    unsafe {
        let sname = if spritename.is_null() {
            "????"
        } else {
            CStr::from_ptr(spritename).to_str().unwrap_or("????")
        };

        if frame >= 29 || rotation > 8 {
            doom_error(&format!(
                "R_InstallSpriteLump: Bad frame characters in lump {}",
                lump
            ));
        }

        if (frame as c_int) > maxframe {
            maxframe = frame as c_int;
        }

        let f_idx = frame as usize;
        let frame_ch = (b'A' + (frame as u8)) as char;

        if rotation == 0 {
            if sprtemp[f_idx].rotate == 0 {
                doom_error(&format!(
                    "R_InitSprites: Sprite {} frame {} has multip rot=0 lump",
                    sname, frame_ch
                ));
            }
            if sprtemp[f_idx].rotate == 1 {
                doom_error(&format!(
                    "R_InitSprites: Sprite {} frame {} has rotations and a rot=0 lump",
                    sname, frame_ch
                ));
            }

            sprtemp[f_idx].rotate = 0;
            for r in 0..8 {
                sprtemp[f_idx].lump[r] = (lump - firstspritelump) as i16;
                sprtemp[f_idx].flip[r] = flipped as u8;
            }
            return;
        }

        if sprtemp[f_idx].rotate == 0 {
            doom_error(&format!(
                "R_InitSprites: Sprite {} frame {} has rotations and a rot=0 lump",
                sname, frame_ch
            ));
        }

        sprtemp[f_idx].rotate = 1;
        rotation -= 1;
        let r_idx = rotation as usize;

        if sprtemp[f_idx].lump[r_idx] != -1 {
            let rot_ch = (b'1' + (rotation as u8)) as char;
            doom_error(&format!(
                "R_InitSprites: Sprite {} : {} : {} has two lumps mapped to it",
                sname, frame_ch, rot_ch
            ));
        }

        sprtemp[f_idx].lump[r_idx] = (lump - firstspritelump) as i16;
        sprtemp[f_idx].flip[r_idx] = flipped as u8;
    }
}

// --- R_InitSpriteDefs ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitSpriteDefs(namelist: *mut *mut c_char) {
    if namelist.is_null() {
        return;
    }

    unsafe {
        let mut check = namelist;
        while !(*check).is_null() {
            check = check.add(1);
        }

        numsprites = check.offset_from(namelist) as c_int;
        if numsprites == 0 {
            return;
        }

        let alloc_bytes = (numsprites as usize) * size_of::<spritedef_t>();
        sprites = Z_Malloc(alloc_bytes as c_int, PU_STATIC, ptr::null_mut()) as *mut spritedef_t;

        let start = firstspritelump - 1;
        let end = lastspritelump + 1;

        for i in 0..(numsprites as usize) {
            let name_ptr = *namelist.add(i);
            spritename = name_ptr;

            ptr::write_bytes(
                (&raw mut sprtemp) as *mut u8,
                0xff,
                size_of::<[spriteframe_t; 29]>(),
            );

            maxframe = -1;
            let intname = std::ptr::read_unaligned(name_ptr as *const i32);
            let sname_str = CStr::from_ptr(name_ptr).to_str().unwrap_or("????");

            for l in (start + 1)..end {
                let l_info = lumpinfo.add(l as usize);
                let l_name_ptr = (&raw const (*l_info).name) as *const c_char;
                let l_intname = std::ptr::read_unaligned(l_name_ptr as *const i32);

                if l_intname == intname {
                    let b4 = *l_name_ptr.add(4) as u8;
                    let b5 = *l_name_ptr.add(5) as u8;
                    let frame = (b4 as i32) - (b'A' as i32);
                    let rotation = (b5 as i32) - (b'0' as i32);

                    let patched = if modifiedgame != 0 {
                        W_GetNumForName(l_name_ptr)
                    } else {
                        l
                    };

                    R_InstallSpriteLump(patched, frame as c_uint, rotation as c_uint, 0);

                    let b6 = *l_name_ptr.add(6) as u8;
                    if b6 != 0 {
                        let b7 = *l_name_ptr.add(7) as u8;
                        let frame2 = (b6 as i32) - (b'A' as i32);
                        let rotation2 = (b7 as i32) - (b'0' as i32);
                        R_InstallSpriteLump(l, frame2 as c_uint, rotation2 as c_uint, 1);
                    }
                }
            }

            let spr_def = sprites.add(i);
            if maxframe == -1 {
                (*spr_def).numframes = 0;
                continue;
            }

            maxframe += 1;

            for frame in 0..maxframe {
                let f_idx = frame as usize;
                let frame_ch = (b'A' + (frame as u8)) as char;
                match sprtemp[f_idx].rotate as i32 {
                    -1 => {
                        doom_error(&format!(
"R_InitSprites: No patches found for {} frame {}",
sname_str, frame_ch
));
}
0 => {}
1 => {
for rot in 0..8 {
if sprtemp[f_idx].lump[rot] == -1 {
doom_error(&format!(
"R_InitSprites: Sprite {} frame {} is missing rotations",
sname_str, frame_ch
));
}
}
}
_ => {}
}
}
(*spr_def).numframes = maxframe;
let frames_bytes = (maxframe as usize) * size_of::<spriteframe_t>();
let dest_frames = Z_Malloc(frames_bytes as c_int, PU_STATIC, ptr::null_mut()) as *mut spriteframe_t;
(*spr_def).spriteframes = dest_frames;
ptr::copy_nonoverlapping(
(&raw const sprtemp) as *const spriteframe_t,
dest_frames,
maxframe as usize,
);
}
}
}
// --- R_InitSprites ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitSprites(namelist: *mut *mut c_char) {
unsafe {
for i in 0..SCR_W {
negonearray[i] = -1;
}
R_InitSpriteDefs(namelist);
}
}
// --- R_ClearSprites ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ClearSprites() {
unsafe {
vissprite_p = (&raw mut vissprites) as *mut vissprite_t;
}
}
// --- R_NewVisSprite ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_NewVisSprite() -> *mut vissprite_t {
unsafe {
let base = (&raw mut vissprites) as *mut vissprite_t;
if vissprite_p == base.add(MAXVISSPRITES) {
return &raw mut overflowsprite;
}
let res = vissprite_p;
vissprite_p = vissprite_p.add(1);
res
}
}
// --- R_DrawMaskedColumn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawMaskedColumn(mut column: *mut column_t) {
unsafe {
let basetexturemid = dc_texturemid;
let floor_clip_val = *mfloorclip.add(dc_x as usize) as c_int;
let ceil_clip_val = *mceilingclip.add(dc_x as usize) as c_int;
while (*column).topdelta != 0xff {
let topdelta = (*column).topdelta as i32;
let length = (*column).length as i32;
let topscreen = sprtopscreen.wrapping_add(spryscale.wrapping_mul(topdelta));
let bottomscreen = topscreen.wrapping_add(spryscale.wrapping_mul(length));
dc_yl = (topscreen + (FRACUNIT as i32) - 1) >> FRACBITS;
dc_yh = (bottomscreen - 1) >> FRACBITS;
if dc_yh >= floor_clip_val {
dc_yh = floor_clip_val - 1;
}
if dc_yl <= ceil_clip_val {
dc_yl = ceil_clip_val + 1;
}
if dc_yl <= dc_yh {
dc_source = (column as *mut u8).add(3);
dc_texturemid = basetexturemid.wrapping_sub(topdelta << FRACBITS);
if let Some(func) = colfunc {
func();
}
}
column = (column as *mut u8).add((length as usize) + 4) as *mut column_t;
}
dc_texturemid = basetexturemid;
}
}
// --- R_DrawVisSprite ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawVisSprite(vis: *mut vissprite_t, _x1: c_int, _x2: c_int) {
unsafe {
let patch = W_CacheLumpNum((*vis).patch + firstspritelump, PU_CACHE) as *mut patch_t;
dc_colormap = (*vis).colormap;
if dc_colormap.is_null() {
colfunc = fuzzcolfunc;
} else if ((*vis).mobjflags as u32 & MF_TRANSLATION) != 0 {
colfunc = Some(R_DrawTranslatedColumn);
let shift_amt = (MF_TRANSSHIFT as i32) - 8;
let offset = (((*vis).mobjflags as u32 & MF_TRANSLATION) >> shift_amt) as isize;
dc_translation = translationtables.offset(-256 + offset);
}
dc_iscale = (*vis).xiscale.abs() >> detailshift;
dc_texturemid = (*vis).texturemid;
let mut frac = (*vis).startfrac;
spryscale = (*vis).scale;
sprtopscreen = centeryfrac - fixed_mul(dc_texturemid, spryscale);
let col_ofs_ptr = (&raw const (*patch).columnofs) as *const c_int;
let patch_w = (*patch).width as c_int;
for x in (*vis).x1..=(*vis).x2 {
dc_x = x;
let texturecolumn = frac >> FRACBITS;
if texturecolumn < 0 || texturecolumn >= patch_w {
doom_error("R_DrawSpriteRange: bad texturecolumn");
}
let ofs = *col_ofs_ptr.add(texturecolumn as usize) as usize;
let column = (patch as *mut u8).add(ofs) as *mut column_t;
R_DrawMaskedColumn(column);
frac = frac.wrapping_add((*vis).xiscale);
}
colfunc = basecolfunc;
}
}
// --- R_ProjectSprite ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ProjectSprite(thing: *mut mobj_t) {
unsafe {
let tr_x = (*thing).x - viewx;
let tr_y = (*thing).y - viewy;
let gxt = fixed_mul(tr_x, viewcos);
let gyt = -fixed_mul(tr_y, viewsin);
let tz = gxt - gyt;
if tz < MINZ {
return;
}
let xscale = fixed_div(projection, tz);
let gxt2 = -fixed_mul(tr_x, viewsin);
let gyt2 = fixed_mul(tr_y, viewcos);
let mut tx = -(gyt2 + gxt2);
if tx.abs() > (tz << 2) {
return;
}
let spr_idx = (*thing).sprite as u32;
if spr_idx >= (numsprites as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite number {} ",
(*thing).sprite
));
}
let sprdef = sprites.add(spr_idx as usize);
let frame_idx = ((*thing).frame as u32) & FF_FRAMEMASK;
if frame_idx >= ((*sprdef).numframes as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite frame {} : {} ",
(*thing).sprite,
(*thing).frame
));
}
let sprframe = (*sprdef).spriteframes.add(frame_idx as usize);
let lump_ptr = (&raw const (*sprframe).lump) as *const i16;
let flip_ptr = (&raw const (*sprframe).flip) as *const u8;
let (lump, flip) = if (*sprframe).rotate != 0 {
let ang = R_PointToAngle((*thing).x, (*thing).y);
let rot = (ang
.wrapping_sub((*thing).angle)
.wrapping_add((ANG45 / 2).wrapping_mul(9)))
>> 29;
(
*lump_ptr.add(rot as usize) as c_int,
*flip_ptr.add(rot as usize) != 0,
)
} else {
(*lump_ptr as c_int, *flip_ptr != 0)
};
tx -= *spriteoffset.add(lump as usize);
let x1 = (centerxfrac + fixed_mul(tx, xscale)) >> FRACBITS;
if x1 > viewwidth {
return;
}
tx += *spritewidth.add(lump as usize);
let x2 = ((centerxfrac + fixed_mul(tx, xscale)) >> FRACBITS) - 1;
if x2 < 0 {
return;
}
let vis = R_NewVisSprite();
(*vis).mobjflags = (*thing).flags;
(*vis).scale = xscale << detailshift;
(*vis).gx = (*thing).x;
(*vis).gy = (*thing).y;
(*vis).gz = (*thing).z;
(*vis).gzt = (*thing).z + *spritetopoffset.add(lump as usize);
(*vis).texturemid = (*vis).gzt - viewz;
(*vis).x1 = if x1 < 0 { 0 } else { x1 };
(*vis).x2 = if x2 >= viewwidth { viewwidth - 1 } else { x2 };
let iscale = fixed_div(FRACUNIT as i32, xscale);
if flip {
(*vis).startfrac = *spritewidth.add(lump as usize) - 1;
(*vis).xiscale = -iscale;
} else {
(*vis).startfrac = 0;
(*vis).xiscale = iscale;
}
if (*vis).x1 > x1 {
(*vis).startfrac += (*vis).xiscale * ((*vis).x1 - x1);
}
(*vis).patch = lump;
if ((*thing).flags as u32 & MF_SHADOW) != 0 {
(*vis).colormap = ptr::null_mut();
} else if !fixedcolormap.is_null() {
(*vis).colormap = fixedcolormap;
} else if ((*thing).frame as u32 & FF_FULLBRIGHT) != 0 {
(*vis).colormap = colormaps;
} else {
let mut index = (xscale >> (LIGHTSCALESHIFT - (detailshift as u32))) as u32;
if index >= MAXLIGHTSCALE {
index = MAXLIGHTSCALE - 1;
}
(*vis).colormap = *spritelights.add(index as usize);
}
}
}
// --- R_AddSprites ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_AddSprites(sec: *mut sector_t) {
unsafe {
if (*sec).validcount == validcount {
return;
}
(*sec).validcount = validcount;
let lightnum = (((*sec).lightlevel as c_int) >> LIGHTSEGSHIFT) + extralight;
let sl_ptr = (&raw const scalelight) as *const [*mut u8; MAXLIGHTSCALE as usize];
if lightnum < 0 {
spritelights = (*sl_ptr).as_ptr() as *mut *mut u8;
} else if lightnum >= LIGHTLEVELS as c_int {
spritelights = (*sl_ptr.add((LIGHTLEVELS as usize) - 1)).as_ptr() as *mut *mut u8;
} else {
spritelights = (*sl_ptr.add(lightnum as usize)).as_ptr() as *mut *mut u8;
}
let mut thing = (*sec).thinglist;
while !thing.is_null() {
R_ProjectSprite(thing);
thing = (*thing).snext;
}
}
}
// --- R_DrawPSprite ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawPSprite(psp: *mut pspdef_t) {
unsafe {
let st = (*psp).state;
let spr_idx = (*st).sprite as u32;
if spr_idx >= (numsprites as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite number {} ",
(*st).sprite
));
}
let sprdef = sprites.add(spr_idx as usize);
let frame_idx = ((*st).frame as u32) & FF_FRAMEMASK;
if frame_idx >= ((*sprdef).numframes as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite frame {} : {} ",
(*st).sprite,
(*st).frame
));
}
let sprframe = (*sprdef).spriteframes.add(frame_idx as usize);
let lump_ptr = (&raw const (*sprframe).lump) as *const i16;
let flip_ptr = (&raw const (*sprframe).flip) as *const u8;
let lump = *lump_ptr as c_int;
let flip = *flip_ptr != 0;
let mut tx = (*psp).sx - 160 * (FRACUNIT as i32);
tx -= *spriteoffset.add(lump as usize);
let x1 = (centerxfrac + fixed_mul(tx, pspritescale)) >> FRACBITS;
if x1 > viewwidth {
return;
}
tx += *spritewidth.add(lump as usize);
let x2 = ((centerxfrac + fixed_mul(tx, pspritescale)) >> FRACBITS) - 1;
if x2 < 0 {
return;
}
let mut avis: vissprite_t = std::mem::zeroed();
let vis = &mut avis as *mut vissprite_t;
(*vis).mobjflags = 0;
(*vis).texturemid =
(BASEYCENTER << FRACBITS) + (FRACUNIT as i32) / 2 - ((*psp).sy - *spritetopoffset.add(lump as usize));
(*vis).x1 = if x1 < 0 { 0 } else { x1 };
(*vis).x2 = if x2 >= viewwidth { viewwidth - 1 } else { x2 };
(*vis).scale = pspritescale << detailshift;
if flip {
(*vis).xiscale = -pspriteiscale;
(*vis).startfrac = *spritewidth.add(lump as usize) - 1;
} else {
(*vis).xiscale = pspriteiscale;
(*vis).startfrac = 0;
}
if (*vis).x1 > x1 {
(*vis).startfrac += (*vis).xiscale * ((*vis).x1 - x1);
}
(*vis).patch = lump;
let invis = (*viewplayer).powers[pw_invisibility as usize];
if invis > 4 * 32 || (invis & 8) != 0 {
(*vis).colormap = ptr::null_mut();
} else if !fixedcolormap.is_null() {
(*vis).colormap = fixedcolormap;
} else if ((*st).frame as u32 & FF_FULLBRIGHT) != 0 {
(*vis).colormap = colormaps;
} else {
(*vis).colormap = *spritelights.add((MAXLIGHTSCALE as usize) - 1);
}
R_DrawVisSprite(vis, (*vis).x1, (*vis).x2);
}
}
// --- R_DrawPlayerSprites ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawPlayerSprites() {
unsafe {
let mo = (*viewplayer).mo;
let sub = (*mo).subsector as *mut crate::r_bsp::subsector_t;
let sec = (*sub).sector;
let lightnum = (((*sec).lightlevel as c_int) >> LIGHTSEGSHIFT) + extralight;
let sl_ptr = (&raw const scalelight) as *const [*mut u8; MAXLIGHTSCALE as usize];
if lightnum < 0 {
spritelights = (*sl_ptr).as_ptr() as *mut *mut u8;
} else if lightnum >= LIGHTLEVELS as c_int {
spritelights = (*sl_ptr.add((LIGHTLEVELS as usize) - 1)).as_ptr() as *mut *mut u8;
} else {
spritelights = (*sl_ptr.add(lightnum as usize)).as_ptr() as *mut *mut u8;
}
mfloorclip = (&raw mut screenheightarray) as *mut c_short;
mceilingclip = (&raw mut negonearray) as *mut c_short;
let pspr_base = (&raw mut (*viewplayer).psprites) as *mut pspdef_t;
for i in 0..NUMPSPRITES {
let psp = pspr_base.add(i);
if !(*psp).state.is_null() {
R_DrawPSprite(psp);
}
}
}
}
// --- R_SortVisSprites ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_SortVisSprites() {
unsafe {
let base = (&raw mut vissprites) as *mut vissprite_t;
let count = vissprite_p.offset_from(base);
let mut unsorted: vissprite_t = std::mem::zeroed();
let unsorted_ptr = &mut unsorted as *mut vissprite_t;
(*unsorted_ptr).next = unsorted_ptr;
(*unsorted_ptr).prev = unsorted_ptr;
if count <= 0 {
return;
}
let mut ds = base;
while ds < vissprite_p {
(*ds).next = ds.add(1);
(*ds).prev = ds.sub(1);
ds = ds.add(1);
}
(*base).prev = unsorted_ptr;
(*unsorted_ptr).next = base;
let last_vis = vissprite_p.sub(1);
(*last_vis).next = unsorted_ptr;
(*unsorted_ptr).prev = last_vis;
let sorted_head = &raw mut vsprsortedhead;
(*sorted_head).next = sorted_head;
(*sorted_head).prev = sorted_head;
for _ in 0..count {
let mut bestscale = i32::MAX;
let mut best = ptr::null_mut();
let mut cur = (*unsorted_ptr).next;
while cur != unsorted_ptr {
if (*cur).scale < bestscale {
bestscale = (*cur).scale;
best = cur;
}
cur = (*cur).next;
}
if !best.is_null() {
(*(*best).next).prev = (*best).prev;
(*(*best).prev).next = (*best).next;
(*best).next = sorted_head;
(*best).prev = (*sorted_head).prev;
(*(*sorted_head).prev).next = best;
(*sorted_head).prev = best;
}
}
}
}
// --- R_DrawSprite ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawSprite(spr: *mut vissprite_t) {
unsafe {
let mut clipbot: [c_short; SCR_W] = [0; SCR_W];
let mut cliptop: [c_short; SCR_W] = [0; SCR_W];
for x in (*spr).x1..=(*spr).x2 {
let xu = x as usize;
clipbot[xu] = -2;
cliptop[xu] = -2;
}
let ds_base = (&raw mut drawsegs) as *mut drawseg_t;
let mut ds = (ds_p as *mut drawseg_t).sub(1);
while ds >= ds_base {
if (*ds).x1 > (*spr).x2
|| (*ds).x2 < (*spr).x1
|| ((*ds).silhouette == 0 && (*ds).maskedtexturecol.is_null())
{
ds = ds.sub(1);
continue;
}
let r1 = if (*ds).x1 < (*spr).x1 { (*spr).x1 } else { (*ds).x1 };
let r2 = if (*ds).x2 > (*spr).x2 { (*spr).x2 } else { (*ds).x2 };
let (lowscale, scale) = if (*ds).scale1 > (*ds).scale2 {
((*ds).scale2, (*ds).scale1)
} else {
((*ds).scale1, (*ds).scale2)
};
if scale < (*spr).scale
|| (lowscale < (*spr).scale
&& R_PointOnSegSide((*spr).gx, (*spr).gy, (*ds).curline) == 0)
{
if !(*ds).maskedtexturecol.is_null() {
R_RenderMaskedSegRange(ds as *mut crate::r_segs::drawseg_t, r1, r2);
}
ds = ds.sub(1);
continue;
}
let mut silhouette = (*ds).silhouette;
if (*spr).gz >= (*ds).bsilheight {
silhouette &= !(SIL_BOTTOM as c_int);
}
if (*spr).gzt <= (*ds).tsilheight {
silhouette &= !(SIL_TOP as c_int);
}
if silhouette == 1 {
for x in r1..=r2 {
let xu = x as usize;
if clipbot[xu] == -2 {
clipbot[xu] = *(*ds).sprbottomclip.add(xu);
}
}
} else if silhouette == 2 {
for x in r1..=r2 {
let xu = x as usize;
if cliptop[xu] == -2 {
cliptop[xu] = *(*ds).sprtopclip.add(xu);
}
}
} else if silhouette == 3 {
for x in r1..=r2 {
let xu = x as usize;
if clipbot[xu] == -2 {
clipbot[xu] = *(*ds).sprbottomclip.add(xu);
}
if cliptop[xu] == -2 {
cliptop[xu] = *(*ds).sprtopclip.add(xu);
}
}
}
ds = ds.sub(1);
}
for x in (*spr).x1..=(*spr).x2 {
let xu = x as usize;
if clipbot[xu] == -2 {
clipbot[xu] = viewheight as c_short;
}
if cliptop[xu] == -2 {
cliptop[xu] = -1;
}
}
mfloorclip = clipbot.as_mut_ptr();
mceilingclip = cliptop.as_mut_ptr();
R_DrawVisSprite(spr, (*spr).x1, (*spr).x2);
}
}
// --- R_DrawMasked ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawMasked() {
unsafe {
R_SortVisSprites();
let base = (&raw mut vissprites) as *mut vissprite_t;
let sorted_head = &raw mut vsprsortedhead;
if vissprite_p > base {
let mut spr = (*sorted_head).next;
while spr != sorted_head {
R_DrawSprite(spr);
spr = (*spr).next;
}
}
let ds_base = (&raw mut drawsegs) as *mut drawseg_t;
let mut ds = (ds_p as *mut drawseg_t).sub(1);
while ds >= ds_base {
if !(*ds).maskedtexturecol.is_null() {
R_RenderMaskedSegRange(ds as *mut crate::r_segs::drawseg_t, (*ds).x1, (*ds).x2);
}
ds = ds.sub(1);
}
if viewangleoffset == 0 {
R_DrawPlayerSprites();
}
}
}