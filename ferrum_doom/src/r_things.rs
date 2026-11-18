#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::is_modified_game;
use crate::r_segs::line_s;

use std::ffi::{CStr, CString, c_char, c_int, c_short, c_uint};
use std::mem::size_of;
use std::ptr;

use crate::m_fixed::{FRACBITS, fixed_div, fixed_mul};
use crate::r_bsp::{subsector_t, drawsegs, ds_p};
use crate::r_draw::{
    R_DrawTranslatedColumn, dc_colormap, dc_iscale, dc_source, dc_texturemid,
    dc_translation, dc_x, dc_yh, dc_yl, translationtables, viewheight, viewwidth,
};
use crate::r_segs::R_RenderMaskedSegRange;
use crate::v_video::column_t;
use crate::w_wad::{W_CacheLumpNum, lumpinfo, w_get_num_for_name};
use crate::z_zone::{PU_CACHE, PU_STATIC, Z_Malloc};

pub const MAXVISSPRITES: usize = 128;
const MINZ: i32 = (FRACUNIT as i32) * 4;
const BASEYCENTER: i32 = 100;
const SCR_W: usize = SCREENWIDTH as usize;

const EMPTY_SPRITEFRAME: spriteframe_t = spriteframe_t {
    rotate: 0,
    lump: [-1; 8],
    flip: [0; 8],
};

const EMPTY_VISSPRITE: vissprite_t = vissprite_t {
    prev: ptr::null_mut(),
    next: ptr::null_mut(),
    x1: 0,
    x2: 0,
    gx: 0,
    gy: 0,
    gz: 0,
    gzt: 0,
    startfrac: 0,
    scale: 0,
    xiscale: 0,
    texturemid: 0,
    patch: 0,
    colormap: ptr::null_mut(),
    mobjflags: 0,
};

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
    pub psprites: [pspdef_t; NUMPSPRITES as usize],
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

#[unsafe(no_mangle)]
pub static mut pspritescale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut pspriteiscale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut spritelights: *mut *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut negonearray: [c_short; SCR_W] = [-1; SCR_W];

#[unsafe(no_mangle)]
pub static mut screenheightarray: [c_short; SCR_W] = [0; SCR_W];

#[unsafe(no_mangle)]
pub static mut sprites: *mut spritedef_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numsprites: c_int = 0;

#[unsafe(no_mangle)]
pub static mut sprtemp: [spriteframe_t; 29] = [EMPTY_SPRITEFRAME; 29];

#[unsafe(no_mangle)]
pub static mut maxframe: c_int = 0;

#[unsafe(no_mangle)]
pub static mut spritename: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut vissprites: [vissprite_t; MAXVISSPRITES] = [EMPTY_VISSPRITE; MAXVISSPRITES];

#[unsafe(no_mangle)]
pub static mut vissprite_p: *mut vissprite_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut newvissprite: c_int = 0;

#[unsafe(no_mangle)]
pub static mut overflowsprite: vissprite_t = EMPTY_VISSPRITE;

#[unsafe(no_mangle)]
pub static mut mfloorclip: *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut mceilingclip: *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut spryscale: i32 = 0;

#[unsafe(no_mangle)]
pub static mut sprtopscreen: i32 = 0;

#[unsafe(no_mangle)]
pub static mut vsprsortedhead: vissprite_t = EMPTY_VISSPRITE;

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

        let temp = (&raw mut sprtemp).as_mut().unwrap();
        let f_idx = frame as usize;
        let frame_ch = (b'A' + (frame as u8)) as char;

        if rotation == 0 {
            if temp[f_idx].rotate == 0 {
                doom_error(&format!(
                    "R_InitSprites: Sprite {} frame {} has multip rot=0 lump",
                    sname, frame_ch
                ));
            }
            if temp[f_idx].rotate == 1 {
                doom_error(&format!(
                    "R_InitSprites: Sprite {} frame {} has rotations and a rot=0 lump",
                    sname, frame_ch
                ));
            }

            temp[f_idx].rotate = 0;
            temp[f_idx].lump.fill((lump - firstspritelump) as i16);
            temp[f_idx].flip.fill(flipped as u8);
            return;
        }

        if temp[f_idx].rotate == 0 {
            doom_error(&format!(
                "R_InitSprites: Sprite {} frame {} has rotations and a rot=0 lump",
                sname, frame_ch
            ));
        }

        temp[f_idx].rotate = 1;
        rotation -= 1;
        let r_idx = rotation as usize;

        if temp[f_idx].lump[r_idx] != -1 {
            let rot_ch = (b'1' + (rotation as u8)) as char;
            doom_error(&format!(
                "R_InitSprites: Sprite {} : {} : {} has two lumps mapped to it",
                sname, frame_ch, rot_ch
            ));
        }

        temp[f_idx].lump[r_idx] = (lump - firstspritelump) as i16;
        temp[f_idx].flip[r_idx] = flipped as u8;
    }
}

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

        let n_spr = numsprites as usize;
        let alloc_bytes = n_spr * size_of::<spritedef_t>();
        sprites = Z_Malloc(alloc_bytes as c_int, PU_STATIC, ptr::null_mut()) as *mut spritedef_t;
        let spr_slice = std::slice::from_raw_parts_mut(sprites, n_spr);
        let lumps = std::slice::from_raw_parts(lumpinfo, crate::w_wad::numlumps as usize);

        let start = (firstspritelump as usize);
        let end = (lastspritelump + 1) as usize;

        for (i, spr_def) in spr_slice.iter_mut().enumerate() {
            let name_ptr = *namelist.add(i);
            spritename = name_ptr;

            let temp = (&raw mut sprtemp).as_mut().unwrap();
            for sf in temp.iter_mut() {
                sf.rotate = -1;
                sf.lump.fill(-1);
                sf.flip.fill(0);
            }

            maxframe = -1;
            let sname_str = CStr::from_ptr(name_ptr).to_str().unwrap_or("????");
            let prefix = &sname_str.as_bytes()[..4.min(sname_str.len())];

            for l in start..end {
                let l_name = &lumps[l].name;
                if &l_name[..4] == prefix {
                    let frame = (l_name[4] as i32) - (b'A' as i32);
                    let rotation = (l_name[5] as i32) - (b'0' as i32);

                    let patched = if is_modified_game() {
                        let len = l_name.iter().position(|&b| b == 0).unwrap_or(8);
                        let s = std::str::from_utf8(&l_name[..len]).unwrap_or("");
                        w_get_num_for_name(s)
                    } else {
                        l as c_int
                    };

                    R_InstallSpriteLump(patched, frame as c_uint, rotation as c_uint, 0);

                    if l_name[6] != 0 {
                        let frame2 = (l_name[6] as i32) - (b'A' as i32);
                        let rotation2 = (l_name[7] as i32) - (b'0' as i32);
                        R_InstallSpriteLump(l as c_int, frame2 as c_uint, rotation2 as c_uint, 1);
                    }
                }
            }

            if maxframe == -1 {
                spr_def.numframes = 0;
                continue;
            }

            maxframe += 1;

for frame in 0..maxframe {
let f_idx = frame as usize;
let frame_ch = (b'A' + (frame as u8)) as char;
match temp[f_idx].rotate as i32 {
-1 => {
doom_error(&format!(
"R_InitSprites: No patches found for {} frame {}",
sname_str, frame_ch
));
}
0 => {}
1 => {
if temp[f_idx].lump.contains(&-1) {
doom_error(&format!(
"R_InitSprites: Sprite {} frame {} is missing rotations",
sname_str, frame_ch
));
}
}
_ => {}
}
}
let count = maxframe as usize;
spr_def.numframes = maxframe;
let frames_bytes = count * size_of::<spriteframe_t>();
let dest_frames =
Z_Malloc(frames_bytes as c_int, PU_STATIC, ptr::null_mut()) as *mut spriteframe_t;
spr_def.spriteframes = dest_frames;
let dst_slice = std::slice::from_raw_parts_mut(dest_frames, count);
dst_slice.copy_from_slice(&temp[..count]);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitSprites(namelist: *mut *mut c_char) {
unsafe {
(&raw mut negonearray).as_mut().unwrap().fill(-1);
R_InitSpriteDefs(namelist);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ClearSprites() {
unsafe {
vissprite_p = (&raw mut vissprites) as *mut vissprite_t;
}
}
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
dc_yl = ((topscreen + (FRACUNIT as i32) - 1) >> FRACBITS).max(ceil_clip_val + 1);
dc_yh = ((bottomscreen - 1) >> FRACBITS).min(floor_clip_val - 1);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawVisSprite(vis: *mut vissprite_t, _x1: c_int, _x2: c_int) {
let Some(v) = (unsafe { vis.as_ref() }) else { return };
unsafe {
let patch = W_CacheLumpNum(v.patch + firstspritelump, PU_CACHE) as *mut patch_t;
let p_ref = &*patch;
dc_colormap = v.colormap;
if dc_colormap.is_null() {
colfunc = fuzzcolfunc;
} else if (v.mobjflags as u32 & MF_TRANSLATION) != 0 {
colfunc = Some(R_DrawTranslatedColumn);
let shift_amt = (MF_TRANSSHIFT as i32) - 8;
let offset = ((v.mobjflags as u32 & MF_TRANSLATION) >> shift_amt) as isize;
dc_translation = translationtables.offset(-256 + offset);
}
dc_iscale = v.xiscale.abs() >> detailshift;
dc_texturemid = v.texturemid;
let mut frac = v.startfrac;
spryscale = v.scale;
sprtopscreen = centeryfrac - fixed_mul(dc_texturemid, spryscale);
let patch_w = p_ref.width as c_int;
let col_ofs = std::slice::from_raw_parts(p_ref.columnofs.as_ptr(), patch_w.max(0) as usize);
for x in v.x1..=v.x2 {
dc_x = x;
let texturecolumn = frac >> FRACBITS;
if texturecolumn < 0 || texturecolumn >= patch_w {
doom_error("R_DrawSpriteRange: bad texturecolumn");
}
let ofs = col_ofs[texturecolumn as usize] as usize;
let column = (patch as *mut u8).add(ofs) as *mut column_t;
R_DrawMaskedColumn(column);
frac = frac.wrapping_add(v.xiscale);
}
colfunc = basecolfunc;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_ProjectSprite(thing: *mut mobj_t) {
let Some(th) = (unsafe { thing.as_ref() }) else { return };
unsafe {
let tr_x = th.x - viewx;
let tr_y = th.y - viewy;
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
let spr_idx = th.sprite as u32;
if spr_idx >= (numsprites as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite number {} ",
th.sprite
));
}
let sprdef = &*sprites.add(spr_idx as usize);
let frame_idx = (th.frame as u32) & FF_FRAMEMASK;
if frame_idx >= (sprdef.numframes as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite frame {} : {} ",
th.sprite, th.frame
));
}
let sprframe = &*sprdef.spriteframes.add(frame_idx as usize);
let (lump, flip) = if sprframe.rotate != 0 {
let ang = R_PointToAngle(th.x, th.y);
let rot = ((ang
.wrapping_sub(th.angle)
.wrapping_add((ANG45 / 2).wrapping_mul(9)))
>> 29) as usize;
(sprframe.lump[rot] as c_int, sprframe.flip[rot] != 0)
} else {
(sprframe.lump[0] as c_int, sprframe.flip[0] != 0)
};
let lu = lump as usize;
tx -= *spriteoffset.add(lu);
let x1 = (centerxfrac + fixed_mul(tx, xscale)) >> FRACBITS;
if x1 > viewwidth {
return;
}
tx += *spritewidth.add(lu);
let x2 = ((centerxfrac + fixed_mul(tx, xscale)) >> FRACBITS) - 1;
if x2 < 0 {
return;
}
let vis = &mut *R_NewVisSprite();
vis.mobjflags = th.flags;
vis.scale = xscale << detailshift;
vis.gx = th.x;
vis.gy = th.y;
vis.gz = th.z;
vis.gzt = th.z + *spritetopoffset.add(lu);
vis.texturemid = vis.gzt - viewz;
vis.x1 = x1.max(0);
vis.x2 = x2.min(viewwidth - 1);
let iscale = fixed_div(FRACUNIT as i32, xscale);
if flip {
vis.startfrac = *spritewidth.add(lu) - 1;
vis.xiscale = -iscale;
} else {
vis.startfrac = 0;
vis.xiscale = iscale;
}
if vis.x1 > x1 {
vis.startfrac += vis.xiscale * (vis.x1 - x1);
}
vis.patch = lump;
if (th.flags as u32 & MF_SHADOW) != 0 {
vis.colormap = ptr::null_mut();
} else if !fixedcolormap.is_null() {
vis.colormap = fixedcolormap;
} else if (th.frame as u32 & FF_FULLBRIGHT) != 0 {
vis.colormap = colormaps;
} else {
let index = ((xscale >> (LIGHTSCALESHIFT - (detailshift as u32))) as u32)
.min(MAXLIGHTSCALE - 1) as usize;
vis.colormap = *spritelights.add(index);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_AddSprites(sec: *mut sector_t) {
let Some(s) = (unsafe { sec.as_mut() }) else { return };
unsafe {
if s.validcount == validcount {
return;
}
s.validcount = validcount;
let lightnum = (((s.lightlevel as c_int) >> LIGHTSEGSHIFT) + extralight)
.clamp(0, (LIGHTLEVELS as c_int) - 1) as usize;
let sl = (&raw const scalelight).as_ref().unwrap();
spritelights = sl[lightnum].as_ptr() as *mut *mut u8;
let mut thing = s.thinglist;
while !thing.is_null() {
R_ProjectSprite(thing);
thing = (*thing).snext;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawPSprite(psp: *mut pspdef_t) {
let Some(p) = (unsafe { psp.as_ref() }) else { return };
unsafe {
let st = &*p.state;
let spr_idx = st.sprite as u32;
if spr_idx >= (numsprites as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite number {} ",
st.sprite
));
}
let sprdef = &*sprites.add(spr_idx as usize);
let frame_idx = (st.frame as u32) & FF_FRAMEMASK;
if frame_idx >= (sprdef.numframes as u32) {
doom_error(&format!(
"R_ProjectSprite: invalid sprite frame {} : {} ",
st.sprite, st.frame
));
}
let sprframe = &*sprdef.spriteframes.add(frame_idx as usize);
let lump = sprframe.lump[0] as c_int;
let flip = sprframe.flip[0] != 0;
let lu = lump as usize;
let mut tx = p.sx - 160 * (FRACUNIT as i32);
tx -= *spriteoffset.add(lu);
let x1 = (centerxfrac + fixed_mul(tx, pspritescale)) >> FRACBITS;
if x1 > viewwidth {
return;
}
tx += *spritewidth.add(lu);
let x2 = ((centerxfrac + fixed_mul(tx, pspritescale)) >> FRACBITS) - 1;
if x2 < 0 {
return;
}
let mut avis = EMPTY_VISSPRITE;
avis.mobjflags = 0;
avis.texturemid =
(BASEYCENTER << FRACBITS) + (FRACUNIT as i32) / 2 - (p.sy - *spritetopoffset.add(lu));
avis.x1 = x1.max(0);
avis.x2 = x2.min(viewwidth - 1);
avis.scale = pspritescale << detailshift;
if flip {
avis.xiscale = -pspriteiscale;
avis.startfrac = *spritewidth.add(lu) - 1;
} else {
avis.xiscale = pspriteiscale;
avis.startfrac = 0;
}
if avis.x1 > x1 {
avis.startfrac += avis.xiscale * (avis.x1 - x1);
}
avis.patch = lump;
let invis = (*viewplayer).powers[pw_invisibility as usize];
if invis > 4 * 32 || (invis & 8) != 0 {
avis.colormap = ptr::null_mut();
} else if !fixedcolormap.is_null() {
avis.colormap = fixedcolormap;
} else if (st.frame as u32 & FF_FULLBRIGHT) != 0 {
avis.colormap = colormaps;
} else {
avis.colormap = *spritelights.add((MAXLIGHTSCALE as usize) - 1);
}
R_DrawVisSprite(&mut avis, avis.x1, avis.x2);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawPlayerSprites() {
unsafe {
let vp = viewplayer.as_mut().unwrap();
let sub = ((*vp.mo).subsector as *const subsector_t).as_ref().unwrap();
let sec = sub.sector.as_ref().unwrap();
let lightnum = (((sec.lightlevel as c_int) >> LIGHTSEGSHIFT) + extralight)
.clamp(0, (LIGHTLEVELS as c_int) - 1) as usize;
let sl = (&raw const scalelight).as_ref().unwrap();
spritelights = sl[lightnum].as_ptr() as *mut *mut u8;
mfloorclip = (&raw mut screenheightarray) as *mut c_short;
mceilingclip = (&raw mut negonearray) as *mut c_short;
for psp in &mut vp.psprites {
if !psp.state.is_null() {
R_DrawPSprite(psp);
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_SortVisSprites() {
unsafe {
let base = (&raw mut vissprites) as *mut vissprite_t;
let count = vissprite_p.offset_from(base);
let mut unsorted = EMPTY_VISSPRITE;
let unsorted_ptr = &mut unsorted as *mut vissprite_t;
unsorted.next = unsorted_ptr;
unsorted.prev = unsorted_ptr;
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
unsorted.next = base;
let last_vis = vissprite_p.sub(1);
(*last_vis).next = unsorted_ptr;
unsorted.prev = last_vis;
let sorted_head = &raw mut vsprsortedhead;
(*sorted_head).next = sorted_head;
(*sorted_head).prev = sorted_head;
for _ in 0..count {
let mut bestscale = i32::MAX;
let mut best = ptr::null_mut();
let mut cur = unsorted.next;
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_DrawSprite(spr: *mut vissprite_t) {
let Some(s) = (unsafe { spr.as_ref() }) else { return };
unsafe {
let mut clipbot: [c_short; SCR_W] = [-2; SCR_W];
let mut cliptop: [c_short; SCR_W] = [-2; SCR_W];
let ds_base = (&raw mut drawsegs) as *mut drawseg_t;
let ds_count = (ds_p as *mut drawseg_t).offset_from(ds_base).max(0) as usize;
let active_ds = std::slice::from_raw_parts_mut(ds_base, ds_count);
for ds in active_ds.iter_mut().rev() {
if ds.x1 > s.x2 || ds.x2 < s.x1 || (ds.silhouette == 0 && ds.maskedtexturecol.is_null())
{
continue;
}
let r1 = ds.x1.max(s.x1);
let r2 = ds.x2.min(s.x2);
let (lowscale, scale) = if ds.scale1 > ds.scale2 {
(ds.scale2, ds.scale1)
} else {
(ds.scale1, ds.scale2)
};
if scale < s.scale
|| (lowscale < s.scale && R_PointOnSegSide(s.gx, s.gy, ds.curline) == 0)
{
if !ds.maskedtexturecol.is_null() {
R_RenderMaskedSegRange(ds as *mut _ as *mut crate::r_segs::drawseg_t, r1, r2);
}
continue;
}
let mut silhouette = ds.silhouette;
if s.gz >= ds.bsilheight {
silhouette &= !(SIL_BOTTOM as c_int);
}
if s.gzt <= ds.tsilheight {
silhouette &= !(SIL_TOP as c_int);
}
if (silhouette & 1) != 0 {
for x in r1..=r2 {
let xu = x as usize;
if clipbot[xu] == -2 {
clipbot[xu] = *ds.sprbottomclip.add(xu);
}
}
}
if (silhouette & 2) != 0 {
for x in r1..=r2 {
let xu = x as usize;
if cliptop[xu] == -2 {
cliptop[xu] = *ds.sprtopclip.add(xu);
}
}
}
}
for x in s.x1..=s.x2 {
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
R_DrawVisSprite(spr, s.x1, s.x2);
}
}
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
let ds_count = (ds_p as *mut drawseg_t).offset_from(ds_base).max(0) as usize;
let active_ds = std::slice::from_raw_parts_mut(ds_base, ds_count);
for ds in active_ds.iter_mut().rev() {
if !ds.maskedtexturecol.is_null() {
R_RenderMaskedSegRange(ds as *mut _ as *mut crate::r_segs::drawseg_t, ds.x1, ds.x2);
}
}
if viewangleoffset == 0 {
R_DrawPlayerSprites();
}
}
}
