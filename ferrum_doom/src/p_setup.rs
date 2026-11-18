#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::info::sprnames;
use crate::m_bbox::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP, m_add_to_box, m_clear_box};
use crate::m_fixed::{FRACBITS, fixed_div};
use crate::p_mobj::{P_SpawnMapThing, iquehead, iquetail};
use crate::p_spec::{P_InitPicAnims, P_SpawnSpecials};
use crate::p_switch::P_InitSwitchList;
use crate::p_tick::{P_InitThinkers, leveltime};
use crate::r_bsp::subsector_t;
use crate::r_data::{R_FlatNumForName, R_PrecacheLevel, R_TextureNumForName};
use crate::r_things::R_InitSprites;
use crate::w_wad::{W_CacheLumpNum, W_GetNumForName, W_LumpLength, W_Reload};
use crate::z_zone::{PU_LEVEL, PU_PURGELEVEL, PU_STATIC, Z_Free, Z_FreeTags, Z_Malloc};

pub const MAX_DEATHMATCH_STARTS: usize = 10;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn S_Start();
    fn G_DeathMatchSpawnPlayer(playernum: c_int);

    static mut gamemode: GameMode_t;
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

// --- Глобальные переменные p_setup.c ---
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
    [unsafe { std::mem::zeroed() }; MAX_DEATHMATCH_STARTS];

#[unsafe(no_mangle)]
pub static mut deathmatch_p: *mut mapthing_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut playerstarts: [mapthing_t; NUM_PLAYERS] =
    [unsafe { std::mem::zeroed() }; NUM_PLAYERS];

// --- P_LoadVertexes ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadVertexes(lump: c_int) {
    unsafe {
        let lump_len = W_LumpLength(lump) as usize;
        numvertexes = (lump_len / size_of::<mapvertex_t>()) as c_int;

        let alloc_bytes = (numvertexes as usize) * size_of::<vertex_t>();
        vertexes = Z_Malloc(alloc_bytes as c_int, PU_LEVEL, ptr::null_mut()) as *mut vertex_t;

        let data = W_CacheLumpNum(lump, PU_STATIC) as *const mapvertex_t;
        for i in 0..(numvertexes as usize) {
            let mv = ptr::read_unaligned(data.add(i));
            let v = vertexes.add(i);
            (*v).x = (mv.x as i32) << FRACBITS;
            (*v).y = (mv.y as i32) << FRACBITS;
        }

        Z_Free(data as *mut c_void);
    }
}

// --- P_LoadSegs ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSegs(lump: c_int) {
    unsafe {
        let lump_len = W_LumpLength(lump) as usize;
        numsegs = (lump_len / size_of::<mapseg_t>()) as c_int;

        let alloc_bytes = (numsegs as usize) * size_of::<seg_t>();
        segs = Z_Malloc(alloc_bytes as c_int, PU_LEVEL, ptr::null_mut()) as *mut seg_t;
        ptr::write_bytes(segs as *mut u8, 0, alloc_bytes);

        let data = W_CacheLumpNum(lump, PU_STATIC) as *const mapseg_t;
        for i in 0..(numsegs as usize) {
            let ms = ptr::read_unaligned(data.add(i));
            let seg = segs.add(i);

            (*seg).v1 = vertexes.add(ms.v1 as usize);
            (*seg).v2 = vertexes.add(ms.v2 as usize);
            (*seg).angle = ((ms.angle as i32) << 16) as u32;
            (*seg).offset = (ms.offset as i32) << 16;

            let ldef = lines.add(ms.linedef as usize);
            (*seg).linedef = ldef;

            let side = ms.side as usize;
            let snum_ptr = (&raw const (*ldef).sidenum) as *const c_short;
            let snum0 = *snum_ptr.add(side) as usize;

            let side_ptr = sides.add(snum0);
            (*seg).sidedef = side_ptr;
            (*seg).frontsector = (*side_ptr).sector;

            if ((*ldef).flags as u32 & ML_TWOSIDED) != 0 {
                let snum1 = *snum_ptr.add(side ^ 1) as usize;
                (*seg).backsector = (*sides.add(snum1)).sector;
            } else {
                (*seg).backsector = ptr::null_mut();
            }
        }

        Z_Free(data as *mut c_void);
    }
}

// --- P_LoadSubsectors ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSubsectors(lump: c_int) {
    unsafe {
        let lump_len = W_LumpLength(lump) as usize;
        numsubsectors = (lump_len / size_of::<mapsubsector_t>()) as c_int;

        let alloc_bytes = (numsubsectors as usize) * size_of::<subsector_t>();
        subsectors = Z_Malloc(alloc_bytes as c_int, PU_LEVEL, ptr::null_mut()) as *mut subsector_t;
        ptr::write_bytes(subsectors as *mut u8, 0, alloc_bytes);

        let data = W_CacheLumpNum(lump, PU_STATIC) as *const mapsubsector_t;
        for i in 0..(numsubsectors as usize) {
            let ms = ptr::read_unaligned(data.add(i));
            let ss = subsectors.add(i);
            (*ss).numlines = ms.numsegs;
            (*ss).firstline = ms.firstseg;
        }

        Z_Free(data as *mut c_void);
    }
}

// --- P_LoadSectors ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSectors(lump: c_int) {
    unsafe {
        let lump_len = W_LumpLength(lump) as usize;
        numsectors = (lump_len / size_of::<mapsector_t>()) as c_int;

        let alloc_bytes = (numsectors as usize) * size_of::<sector_t>();
        sectors = Z_Malloc(alloc_bytes as c_int, PU_LEVEL, ptr::null_mut()) as *mut sector_t;
        ptr::write_bytes(sectors as *mut u8, 0, alloc_bytes);

        let data = W_CacheLumpNum(lump, PU_STATIC) as *const mapsector_t;
        for i in 0..(numsectors as usize) {
            let mut ms = ptr::read_unaligned(data.add(i));
            let sec = sectors.add(i);

            (*sec).floorheight = (ms.floorheight as i32) << FRACBITS;
            (*sec).ceilingheight = (ms.ceilingheight as i32) << FRACBITS;
            (*sec).floorpic = R_FlatNumForName(ms.floorpic.as_mut_ptr()) as c_short;
            (*sec).ceilingpic = R_FlatNumForName(ms.ceilingpic.as_mut_ptr()) as c_short;
            (*sec).lightlevel = ms.lightlevel;
            (*sec).special = ms.special;
            (*sec).tag = ms.tag;
            (*sec).thinglist = ptr::null_mut();
        }

        Z_Free(data as *mut c_void);
    }
}

// --- P_LoadNodes ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadNodes(lump: c_int) {
    unsafe {
        let lump_len = W_LumpLength(lump) as usize;
        numnodes = (lump_len / size_of::<mapnode_t>()) as c_int;

        let alloc_bytes = (numnodes as usize) * size_of::<node_t>();
        nodes = Z_Malloc(alloc_bytes as c_int, PU_LEVEL, ptr::null_mut()) as *mut node_t;

        let data = W_CacheLumpNum(lump, PU_STATIC) as *const mapnode_t;
        for i in 0..(numnodes as usize) {
            let mn = ptr::read_unaligned(data.add(i));
            let no = nodes.add(i);

            (*no).x = (mn.x as i32) << FRACBITS;
            (*no).y = (mn.y as i32) << FRACBITS;
            (*no).dx = (mn.dx as i32) << FRACBITS;
            (*no).dy = (mn.dy as i32) << FRACBITS;

            let children_dst = (&raw mut (*no).children) as *mut u16;
            let bbox_dst = (&raw mut (*no).bbox) as *mut [i32; 4];

            for j in 0..2 {
                *children_dst.add(j) = mn.children[j] as u16;
                let row = (*bbox_dst.add(j)).as_mut_ptr();
                for k in 0..4 {
                    *row.add(k) = (mn.bbox[j][k] as i32) << FRACBITS;
                }
            }
        }

        Z_Free(data as *mut c_void);
    }
}

// --- P_LoadThings ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadThings(lump: c_int) {
    unsafe {
        let data = W_CacheLumpNum(lump, PU_STATIC) as *const mapthing_t;
        let numthings = (W_LumpLength(lump) as usize) / size_of::<mapthing_t>();

        for i in 0..numthings {
            let mut mt = ptr::read_unaligned(data.add(i));

            if gamemode != commercial {
                let is_doom2_only = matches!(
                    mt.type_,
68 | 64 | 88 | 89 | 69 | 67 | 71 | 65 | 66 | 84
);
if is_doom2_only {
break;
}
}
P_SpawnMapThing((&raw mut mt) as *mut crate::p_mobj::mapthing_t);
}
Z_Free(data as *mut c_void);
}
}
// --- P_LoadLineDefs ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadLineDefs(lump: c_int) {
unsafe {
let lump_len = W_LumpLength(lump) as usize;
numlines = (lump_len / size_of::<maplinedef_t>()) as c_int;
let alloc_bytes = (numlines as usize) * size_of::<line_t>();
lines = Z_Malloc(alloc_bytes as c_int, PU_LEVEL, ptr::null_mut()) as *mut line_t;
ptr::write_bytes(lines as *mut u8, 0, alloc_bytes);
let data = W_CacheLumpNum(lump, PU_STATIC) as *const maplinedef_t;
for i in 0..(numlines as usize) {
let mld = ptr::read_unaligned(data.add(i));
let ld = lines.add(i);
(*ld).flags = mld.flags;
(*ld).special = mld.special;
(*ld).tag = mld.tag;
let v1 = vertexes.add(mld.v1 as usize) as *mut crate::r_segs::vertex_t;
let v2 = vertexes.add(mld.v2 as usize) as *mut crate::r_segs::vertex_t;
(*ld).v1 = v1;
(*ld).v2 = v2;
let dx = (*v2).x - (*v1).x;
let dy = (*v2).y - (*v1).y;
(*ld).dx = dx;
(*ld).dy = dy;
(*ld).slopetype = if dx == 0 {
ST_VERTICAL
} else if dy == 0 {
ST_HORIZONTAL
} else if fixed_div(dy, dx) > 0 {
ST_POSITIVE
} else {
ST_NEGATIVE
};
let bbox_ptr = (&raw mut (*ld).bbox) as *mut i32;
if (*v1).x < (*v2).x {
*bbox_ptr.add(BOXLEFT) = (*v1).x;
*bbox_ptr.add(BOXRIGHT) = (*v2).x;
} else {
*bbox_ptr.add(BOXLEFT) = (*v2).x;
*bbox_ptr.add(BOXRIGHT) = (*v1).x;
}
if (*v1).y < (*v2).y {
*bbox_ptr.add(BOXBOTTOM) = (*v1).y;
*bbox_ptr.add(BOXTOP) = (*v2).y;
} else {
*bbox_ptr.add(BOXBOTTOM) = (*v2).y;
*bbox_ptr.add(BOXTOP) = (*v1).y;
}
let snum_ptr = (&raw mut (*ld).sidenum) as *mut c_short;
            let s0 = *mld.sidenum.as_ptr();
            let s1 = *mld.sidenum.as_ptr().add(1);
            *snum_ptr = s0;
            *snum_ptr.add(1) = s1;

            if s0 != -1 {
                (*ld).frontsector = (*sides.add(s0 as usize)).sector as *mut crate::r_segs::sector_t;
            } else {
                (*ld).frontsector = ptr::null_mut();
            }

            if s1 != -1 {
                (*ld).backsector = (*sides.add(s1 as usize)).sector as *mut crate::r_segs::sector_t;
            } else {
                (*ld).backsector = ptr::null_mut();
            }
}
Z_Free(data as *mut c_void);
}
}
// --- P_LoadSideDefs ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadSideDefs(lump: c_int) {
unsafe {
let lump_len = W_LumpLength(lump) as usize;
numsides = (lump_len / size_of::<mapsidedef_t>()) as c_int;
let alloc_bytes = (numsides as usize) * size_of::<side_t>();
sides = Z_Malloc(alloc_bytes as c_int, PU_LEVEL, ptr::null_mut()) as *mut side_t;
ptr::write_bytes(sides as *mut u8, 0, alloc_bytes);
let data = W_CacheLumpNum(lump, PU_STATIC) as *const mapsidedef_t;
for i in 0..(numsides as usize) {
let mut msd = ptr::read_unaligned(data.add(i));
let sd = sides.add(i);
(*sd).textureoffset = (msd.textureoffset as i32) << FRACBITS;
(*sd).rowoffset = (msd.rowoffset as i32) << FRACBITS;
(*sd).toptexture = R_TextureNumForName(msd.toptexture.as_mut_ptr()) as c_short;
(*sd).bottomtexture = R_TextureNumForName(msd.bottomtexture.as_mut_ptr()) as c_short;
(*sd).midtexture = R_TextureNumForName(msd.midtexture.as_mut_ptr()) as c_short;
(*sd).sector = sectors.add(msd.sector as usize);
}
Z_Free(data as *mut c_void);
}
}
// --- P_LoadBlockMap ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LoadBlockMap(lump: c_int) {
unsafe {
blockmaplump = W_CacheLumpNum(lump, PU_LEVEL) as *mut c_short;
blockmap = blockmaplump.add(4);
bmaporgx = (*blockmaplump as i32) << FRACBITS;
bmaporgy = (*blockmaplump.add(1) as i32) << FRACBITS;
bmapwidth = *blockmaplump.add(2) as c_int;
bmapheight = *blockmaplump.add(3) as c_int;
let count = size_of::<*mut mobj_t>() * (bmapwidth as usize) * (bmapheight as usize);
blocklinks = Z_Malloc(count as c_int, PU_LEVEL, ptr::null_mut()) as *mut *mut mobj_t;
ptr::write_bytes(blocklinks as *mut u8, 0, count);
}
}
// --- P_GroupLines ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GroupLines() {
unsafe {
for i in 0..(numsubsectors as usize) {
let ss = subsectors.add(i);
let seg = segs.add((*ss).firstline as usize);
(*ss).sector = (*(*seg).sidedef).sector as *mut crate::r_bsp::sector_t;
}
let mut total = 0usize;
for i in 0..(numlines as usize) {
let li = lines.add(i);
total += 1;
let fsec = (*li).frontsector as *mut sector_t;
(*fsec).linecount += 1;
let bsec = (*li).backsector as *mut sector_t;
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
for i in 0..(numsectors as usize) {
let sector = sectors.add(i);
m_clear_box(&mut bbox);
(*sector).lines = linebuffer;
for j in 0..(numlines as usize) {
let li = lines.add(j);
let fsec = (*li).frontsector as *mut sector_t;
let bsec = (*li).backsector as *mut sector_t;
if fsec == sector || bsec == sector {
*linebuffer = li;
linebuffer = linebuffer.add(1);
let v1 = (*li).v1;
let v2 = (*li).v2;
m_add_to_box(&mut bbox, (*v1).x, (*v1).y);
m_add_to_box(&mut bbox, (*v2).x, (*v2).y);
}
}
if linebuffer.offset_from((*sector).lines) != ((*sector).linecount as isize) {
doom_error("P_GroupLines: miscounted");
}
(*sector).soundorg.x = ((bbox[BOXRIGHT] as i64 + bbox[BOXLEFT] as i64) / 2) as i32;
(*sector).soundorg.y = ((bbox[BOXTOP] as i64 + bbox[BOXBOTTOM] as i64) / 2) as i32;
let blockbox_ptr = (&raw mut (*sector).blockbox) as *mut c_int;
let mut block = bbox[BOXTOP].saturating_sub(bmaporgy).saturating_add(max_rad) >> shift;
if block >= bmapheight {
block = bmapheight - 1;
}
*blockbox_ptr.add(BOXTOP) = block;
block = bbox[BOXBOTTOM].saturating_sub(bmaporgy).saturating_sub(max_rad) >> shift;
if block < 0 {
block = 0;
}
*blockbox_ptr.add(BOXBOTTOM) = block;
block = bbox[BOXRIGHT].saturating_sub(bmaporgx).saturating_add(max_rad) >> shift;
if block >= bmapwidth {
block = bmapwidth - 1;
}
*blockbox_ptr.add(BOXRIGHT) = block;
block = bbox[BOXLEFT].saturating_sub(bmaporgx).saturating_sub(max_rad) >> shift;
if block < 0 {
block = 0;
}
*blockbox_ptr.add(BOXLEFT) = block;
}
}
}
// --- P_SetupLevel ---
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
let players_ptr = (&raw mut players) as *mut player_t;
for i in 0..NUM_PLAYERS {
let p = players_ptr.add(i);
(*p).killcount = 0;
(*p).secretcount = 0;
(*p).itemcount = 0;
}
(*players_ptr.add(consoleplayer as usize)).viewz = 1;
S_Start();
Z_FreeTags(PU_LEVEL, PU_PURGELEVEL - 1);
P_InitThinkers();
W_Reload();
let lump_str = if gamemode == commercial {
if map < 10 {
format!("map0{}", map)
} else {
format!("map{}", map)
}
} else {
format!("E{}M{}", episode, map)
};
let c_lump = CString::new(lump_str).unwrap();
let lumpnum = W_GetNumForName(c_lump.as_ptr());
leveltime = 0;
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
let in_game_ptr = (&raw const playeringame) as *const c_int;
for i in 0..NUM_PLAYERS {
if *in_game_ptr.add(i) != 0 {
(*players_ptr.add(i)).mo = ptr::null_mut();
G_DeathMatchSpawnPlayer(i as c_int);
}
}
}
iquehead = 0;
iquetail = 0;
P_SpawnSpecials();
if precache != 0 {
R_PrecacheLevel();
}
}
}
// --- P_Init ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_Init() {
unsafe {
P_InitSwitchList();
P_InitPicAnims();
R_InitSprites((&raw mut sprnames) as *mut *mut c_char);
}
}