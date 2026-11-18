#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::info::sprnames;
use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP, m_add_to_box, m_clear_box};
use crate::m_fixed::{FRACBITS, fixed_div};
use crate::p_mobj::{P_SpawnMapThing, clear_respawn_queue};
use crate::p_spec::{P_InitPicAnims, P_SpawnSpecials};
use crate::p_switch::P_InitSwitchList;
use crate::p_tick::{P_InitThinkers, set_leveltime};
use crate::r_bsp::subsector_t;
use crate::r_data::{R_FlatNumForName, R_PrecacheLevel, R_TextureNumForName};
use crate::r_things::R_InitSprites;
use crate::w_wad::{W_CacheLumpNum, W_LumpLength, W_Reload, w_get_num_for_name};
use crate::z_zone::{PU_LEVEL, PU_PURGELEVEL, PU_STATIC, Z_Free, Z_FreeTags, Z_Malloc};

pub const MAX_DEATHMATCH_STARTS: usize = 10;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;

const EMPTY_MAPTHING: mapthing_t = mapthing_t {
    x: 0,
    y: 0,
    angle: 0,
    type_: 0,
    options: 0,
};

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn S_Start();
    fn G_DeathMatchSpawnPlayer(playernum: c_int);
    static mut wminfo: wbstartstruct_t;
    static mut consoleplayer: c_int;
    static mut totalkills: c_int;
    static mut totalitems: c_int;
    static mut totalsecret: c_int;
    static mut bodyqueslot: c_int;
    static mut deathmatch: c_int;
    static mut precache: c_int;
    static mut playeringame: [c_int; NUM_PLAYERS];
    static mut players: [player_t; NUM_PLAYERS];
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Setup error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

#[unsafe(no_mangle)]
pub static mut numvertexes: c_int = 0;

#[unsafe(no_mangle)]
pub static mut vertexes: *mut vertex_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numsegs: c_int = 0;

#[unsafe(no_mangle)]
pub static mut segs: *mut seg_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numsectors: c_int = 0;

#[unsafe(no_mangle)]
pub static mut sectors: *mut sector_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numsubsectors: c_int = 0;

#[unsafe(no_mangle)]
pub static mut subsectors: *mut subsector_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numnodes: c_int = 0;

#[unsafe(no_mangle)]
pub static mut nodes: *mut node_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numlines: c_int = 0;

#[unsafe(no_mangle)]
pub static mut lines: *mut line_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numsides: c_int = 0;

#[unsafe(no_mangle)]
pub static mut sides: *mut side_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut bmapwidth: c_int = 0;

#[unsafe(no_mangle)]
pub static mut bmapheight: c_int = 0;

#[unsafe(no_mangle)]
pub static mut blockmap: *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut blockmaplump: *mut c_short = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut bmaporgx: i32 = 0;

#[unsafe(no_mangle)]
pub static mut bmaporgy: i32 = 0;

#[unsafe(no_mangle)]
pub static mut blocklinks: *mut *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut rejectmatrix: *mut u8 = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut deathmatchstarts: [mapthing_t; MAX_DEATHMATCH_STARTS] =
    [EMPTY_MAPTHING; MAX_DEATHMATCH_STARTS];

#[unsafe(no_mangle)]
pub static mut deathmatch_p: *mut mapthing_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut playerstarts: [mapthing_t; NUM_PLAYERS] = [EMPTY_MAPTHING; NUM_PLAYERS];

unsafe fn alloc_zeroed_level<T>(count: usize) -> (*mut T, &'static mut [T]) {
    unsafe {
        let bytes = count * size_of::<T>();
        let raw = Z_Malloc(bytes.max(8) as c_int, PU_LEVEL, ptr::null_mut()) as *mut T;
        ptr::write_bytes(raw as *mut u8, 0, bytes);
        (raw, std::slice::from_raw_parts_mut(raw, count))
    }
}

fn read_lump_vec<T: Copy>(lump: c_int) -> Vec<T> {
    let lump_len = W_LumpLength(lump).max(0) as usize;
    let count = lump_len / size_of::<T>();
    let raw = W_CacheLumpNum(lump, PU_STATIC);
    let mut out = Vec::with_capacity(count);
    unsafe {
        let src = raw as *const T;
        for i in 0..count {
            out.push(ptr::read_unaligned(src.add(i)));
        }
    }
    Z_Free(raw);
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadVertexes(lump: c_int) {
    let raw_verts = read_lump_vec::<mapvertex_t>(lump);
    unsafe {
        numvertexes = raw_verts.len() as c_int;
        let (ptr, dst) = alloc_zeroed_level::<vertex_t>(raw_verts.len());
        vertexes = ptr;
        for (v, mv) in dst.iter_mut().zip(raw_verts) {
            v.x = (mv.x as i32) << FRACBITS;
            v.y = (mv.y as i32) << FRACBITS;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSegs(lump: c_int) {
    let raw_segs = read_lump_vec::<mapseg_t>(lump);
    unsafe {
        numsegs = raw_segs.len() as c_int;
        let (ptr, dst) = alloc_zeroed_level::<seg_t>(raw_segs.len());
        segs = ptr;
        let lns = std::slice::from_raw_parts_mut(lines, numlines as usize);
        let sds = std::slice::from_raw_parts_mut(sides, numsides as usize);

        for (seg, ms) in dst.iter_mut().zip(raw_segs) {
            seg.v1 = vertexes.add(ms.v1 as usize);
            seg.v2 = vertexes.add(ms.v2 as usize);
            seg.angle = ((ms.angle as i32) << 16) as u32;
            seg.offset = (ms.offset as i32) << 16;

            let ldef = &mut lns[ms.linedef as usize];
            seg.linedef = ldef;

            let side = ms.side as usize;
            let snum0 = ldef.sidenum[side] as usize;
            let side_ref = &mut sds[snum0];
            seg.sidedef = side_ref;
            seg.frontsector = side_ref.sector;

            if (ldef.flags as u32 & ML_TWOSIDED) != 0 {
                let snum1 = ldef.sidenum[side ^ 1] as usize;
                seg.backsector = sds[snum1].sector;
            } else {
                seg.backsector = ptr::null_mut();
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSubsectors(lump: c_int) {
    let raw_ss = read_lump_vec::<mapsubsector_t>(lump);
    unsafe {
        numsubsectors = raw_ss.len() as c_int;
        let (ptr, dst) = alloc_zeroed_level::<subsector_t>(raw_ss.len());
        subsectors = ptr;
        for (ss, ms) in dst.iter_mut().zip(raw_ss) {
            ss.numlines = ms.numsegs;
            ss.firstline = ms.firstseg;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSectors(lump: c_int) {
    let raw_secs = read_lump_vec::<mapsector_t>(lump);
    unsafe {
        numsectors = raw_secs.len() as c_int;
        let (ptr, dst) = alloc_zeroed_level::<sector_t>(raw_secs.len());
        sectors = ptr;
        for (sec, mut ms) in dst.iter_mut().zip(raw_secs) {
            sec.floorheight = (ms.floorheight as i32) << FRACBITS;
            sec.ceilingheight = (ms.ceilingheight as i32) << FRACBITS;
            sec.floorpic = R_FlatNumForName(ms.floorpic.as_mut_ptr()) as c_short;
            sec.ceilingpic = R_FlatNumForName(ms.ceilingpic.as_mut_ptr()) as c_short;
            sec.lightlevel = ms.lightlevel;
            sec.special = ms.special;
            sec.tag = ms.tag;
            sec.thinglist = ptr::null_mut();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadNodes(lump: c_int) {
    let raw_nodes = read_lump_vec::<mapnode_t>(lump);
    unsafe {
        numnodes = raw_nodes.len() as c_int;
        let (ptr, dst) = alloc_zeroed_level::<node_t>(raw_nodes.len());
        nodes = ptr;
        for (no, mn) in dst.iter_mut().zip(raw_nodes) {
            no.x = (mn.x as i32) << FRACBITS;
            no.y = (mn.y as i32) << FRACBITS;
            no.dx = (mn.dx as i32) << FRACBITS;
            no.dy = (mn.dy as i32) << FRACBITS;

            for j in 0..2 {
                no.children[j] = mn.children[j] as u16;
                for k in 0..4 {
                    no.bbox[j][k] = (mn.bbox[j][k] as i32) << FRACBITS;
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadThings(lump: c_int) {
    let raw_things = read_lump_vec::<mapthing_t>(lump);
    let is_comm = get_gamemode() == commercial;

    for mut mt in raw_things {
        if !is_comm {
            let is_doom2_only = matches!(
                mt.type_,
                68 | 64 | 88 | 89 | 69 | 67 | 71 | 65 | 66 | 84
            );
            if is_doom2_only {
                break;
            }
        }
        unsafe {
            P_SpawnMapThing((&raw mut mt) as *mut crate::p_mobj::mapthing_t);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadLineDefs(lump: c_int) {
    let raw_lines = read_lump_vec::<maplinedef_t>(lump);
    unsafe {
        numlines = raw_lines.len() as c_int;
        let (ptr, dst) = alloc_zeroed_level::<line_t>(raw_lines.len());
        lines = ptr;
        let sds = std::slice::from_raw_parts_mut(sides, numsides as usize);

        for (ld, mld) in dst.iter_mut().zip(raw_lines) {
            ld.flags = mld.flags;
            ld.special = mld.special;
            ld.tag = mld.tag;

            let v1 = vertexes.add(mld.v1 as usize) as *mut crate::r_segs::vertex_t;
            let v2 = vertexes.add(mld.v2 as usize) as *mut crate::r_segs::vertex_t;
            ld.v1 = v1;
            ld.v2 = v2;

            let dx = (*v2).x - (*v1).x;
            let dy = (*v2).y - (*v1).y;
            ld.dx = dx;
            ld.dy = dy;

            ld.slopetype = if dx == 0 {
                ST_VERTICAL
            } else if dy == 0 {
                ST_HORIZONTAL
            } else if fixed_div(dy, dx) > 0 {
ST_POSITIVE
} else {
ST_NEGATIVE
};
if (*v1).x < (*v2).x {
ld.bbox[BOXLEFT] = (*v1).x;
ld.bbox[BOXRIGHT] = (*v2).x;
} else {
ld.bbox[BOXLEFT] = (*v2).x;
ld.bbox[BOXRIGHT] = (*v1).x;
}
if (*v1).y < (*v2).y {
ld.bbox[BOXBOTTOM] = (*v1).y;
ld.bbox[BOXTOP] = (*v2).y;
} else {
ld.bbox[BOXBOTTOM] = (*v2).y;
ld.bbox[BOXTOP] = (*v1).y;
}
let s0 = mld.sidenum[0];
let s1 = mld.sidenum[1];
ld.sidenum[0] = s0;
ld.sidenum[1] = s1;
ld.frontsector = if s0 != -1 {
sds[s0 as usize].sector as *mut crate::r_segs::sector_t
} else {
ptr::null_mut()
};
ld.backsector = if s1 != -1 {
sds[s1 as usize].sector as *mut crate::r_segs::sector_t
} else {
ptr::null_mut()
};
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSideDefs(lump: c_int) {
let raw_sides = read_lump_vec::<mapsidedef_t>(lump);
unsafe {
numsides = raw_sides.len() as c_int;
let (ptr, dst) = alloc_zeroed_level::<side_t>(raw_sides.len());
sides = ptr;
for (sd, mut msd) in dst.iter_mut().zip(raw_sides) {
sd.textureoffset = (msd.textureoffset as i32) << FRACBITS;
sd.rowoffset = (msd.rowoffset as i32) << FRACBITS;
sd.toptexture = R_TextureNumForName(msd.toptexture.as_mut_ptr()) as c_short;
sd.bottomtexture = R_TextureNumForName(msd.bottomtexture.as_mut_ptr()) as c_short;
sd.midtexture = R_TextureNumForName(msd.midtexture.as_mut_ptr()) as c_short;
sd.sector = sectors.add(msd.sector as usize);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadBlockMap(lump: c_int) {
unsafe {
blockmaplump = W_CacheLumpNum(lump, PU_LEVEL) as *mut c_short;
blockmap = blockmaplump.add(4);
bmaporgx = (*blockmaplump as i32) << FRACBITS;
bmaporgy = (*blockmaplump.add(1) as i32) << FRACBITS;
bmapwidth = *blockmaplump.add(2) as c_int;
bmapheight = *blockmaplump.add(3) as c_int;
let count = (bmapwidth as usize) * (bmapheight as usize);
let (ptr, _) = alloc_zeroed_level::<*mut mobj_t>(count);
blocklinks = ptr;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GroupLines() {
unsafe {
let ss_slice = std::slice::from_raw_parts_mut(subsectors, numsubsectors as usize);
let seg_slice = std::slice::from_raw_parts(segs, numsegs as usize);
for ss in ss_slice {
let seg = &seg_slice[ss.firstline as usize];
ss.sector = (*seg.sidedef).sector as *mut crate::r_bsp::sector_t;
}
let lns = std::slice::from_raw_parts_mut(lines, numlines as usize);
let mut total = 0usize;
for li in lns.iter() {
total += 1;
let fsec = li.frontsector as *mut sector_t;
(*fsec).linecount += 1;
let bsec = li.backsector as *mut sector_t;
if !bsec.is_null() && bsec != fsec {
(*bsec).linecount += 1;
total += 1;
}
}
let mut linebuffer = Z_Malloc(
(total * size_of::<*mut line_t>()) as c_int,
PU_LEVEL,
ptr::null_mut(),
) as *mut *mut line_t;
let mut bbox = [0i32; 4];
let max_rad = MAXRADIUS as i32;
let shift = MAPBLOCKSHIFT as u32;
let secs = std::slice::from_raw_parts_mut(sectors, numsectors as usize);
for sector in secs {
let sec_ptr = sector as *mut sector_t;
m_clear_box(&mut bbox);
sector.lines = linebuffer;
for li in lns.iter_mut() {
let fsec = li.frontsector as *mut sector_t;
let bsec = li.backsector as *mut sector_t;
if fsec == sec_ptr || bsec == sec_ptr {
*linebuffer = li;
linebuffer = linebuffer.add(1);
let v1 = &*li.v1;
let v2 = &*li.v2;
m_add_to_box(&mut bbox, v1.x, v1.y);
m_add_to_box(&mut bbox, v2.x, v2.y);
}
}
if linebuffer.offset_from(sector.lines) != (sector.linecount as isize) {
doom_error("P_GroupLines: miscounted");
}
sector.soundorg.x = ((bbox[BOXRIGHT] as i64 + bbox[BOXLEFT] as i64) / 2) as i32;
sector.soundorg.y = ((bbox[BOXTOP] as i64 + bbox[BOXBOTTOM] as i64) / 2) as i32;
let mut block = bbox[BOXTOP].saturating_sub(bmaporgy).saturating_add(max_rad) >> shift;
sector.blockbox[BOXTOP] = block.min(bmapheight - 1);
block = bbox[BOXBOTTOM].saturating_sub(bmaporgy).saturating_sub(max_rad) >> shift;
sector.blockbox[BOXBOTTOM] = block.max(0);
block = bbox[BOXRIGHT].saturating_sub(bmaporgx).saturating_add(max_rad) >> shift;
sector.blockbox[BOXRIGHT] = block.min(bmapwidth - 1);
block = bbox[BOXLEFT].saturating_sub(bmaporgx).saturating_sub(max_rad) >> shift;
sector.blockbox[BOXLEFT] = block.max(0);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetupLevel(
episode: c_int,
map: c_int,
_playermask: c_int,
_skill: skill_t,
) {
unsafe {
totalkills = 0;
totalitems = 0;
totalsecret = 0;
wminfo.maxfrags = 0;
wminfo.partime = 180;
let players_arr = (&raw mut players).as_mut().unwrap();
for p in players_arr.iter_mut() {
p.killcount = 0;
p.secretcount = 0;
p.itemcount = 0;
}
players_arr[consoleplayer as usize].viewz = 1;
S_Start();
Z_FreeTags(PU_LEVEL, PU_PURGELEVEL - 1);
P_InitThinkers();
W_Reload();
let lump_str = if get_gamemode() == commercial {
if map < 10 {
format!("map0{}", map)
} else {
format!("map{}", map)
}
} else {
format!("E{}M{}", episode, map)
};
let lumpnum = w_get_num_for_name(&lump_str);
set_leveltime(0);
P_LoadBlockMap(lumpnum + (ML_BLOCKMAP as c_int));
P_LoadVertexes(lumpnum + (ML_VERTEXES as c_int));
P_LoadSectors(lumpnum + (ML_SECTORS as c_int));
P_LoadSideDefs(lumpnum + (ML_SIDEDEFS as c_int));
P_LoadLineDefs(lumpnum + (ML_LINEDEFS as c_int));
P_LoadSubsectors(lumpnum + (ML_SSECTORS as c_int));
P_LoadNodes(lumpnum + (ML_NODES as c_int));
P_LoadSegs(lumpnum + (ML_SEGS as c_int));
rejectmatrix = W_CacheLumpNum(lumpnum + (ML_REJECT as c_int), PU_LEVEL) as *mut u8;
P_GroupLines();
bodyqueslot = 0;
deathmatch_p = (&raw mut deathmatchstarts) as *mut mapthing_t;
P_LoadThings(lumpnum + (ML_THINGS as c_int));
if deathmatch != 0 {
let in_game = (&raw const playeringame).as_ref().unwrap();
for i in 0..NUM_PLAYERS {
if in_game[i] != 0 {
players_arr[i].mo = ptr::null_mut();
G_DeathMatchSpawnPlayer(i as c_int);
}
}
}
clear_respawn_queue();
P_SpawnSpecials();
if precache != 0 {
R_PrecacheLevel();
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_Init() {
unsafe {
P_InitSwitchList();
P_InitPicAnims();
R_InitSprites((&raw mut sprnames) as *mut *mut c_char);
}
}
