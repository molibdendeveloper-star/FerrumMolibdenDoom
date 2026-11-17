#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

use crate::info::{mobjinfo, states};
use crate::m_fixed::{FRACBITS, fixed_mul};
use crate::m_random::p_random;
use crate::p_tick::{P_AddThinker, P_RemoveThinker, leveltime};
use crate::r_main::{R_PointInSubsector, R_PointToAngle2};
use crate::r_sky::skyflatnum;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};
use crate::z_zone::{PU_LEVEL, Z_Malloc};

const STOPSPEED: i32 = 0x1000;
const FRICTION: i32 = 0xe800;
const ONFLOORZ: i32 = i32::MIN;
const ONCEILINGZ: i32 = i32::MAX;
const CF_NOMOMENTUM: u32 = 4;
use crate::r_bsp::subsector_t;
const QUARTER_FINE: usize = FINEANGLES / 4;
const Q_SIZE: usize = ITEMQUESIZE as usize;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn S_StopSound(origin: *mut c_void);

    fn P_TryMove(thing: *mut mobj_t, x: i32, y: i32) -> c_int;
    fn P_SlideMove(mo: *mut mobj_t);
    fn P_AproxDistance(dx: i32, dy: i32) -> i32;
    fn P_CheckPosition(thing: *mut mobj_t, x: i32, y: i32) -> c_int;
    fn P_SetThingPosition(thing: *mut mobj_t);
    fn P_UnsetThingPosition(thing: *mut mobj_t);
    fn P_SetupPsprites(curplayer: *mut player_t);
    fn P_AimLineAttack(t1: *mut mobj_t, angle: u32, distance: i32) -> i32;
    fn P_MobjThinker_c_alias();

    fn G_PlayerReborn(player: c_int);
    fn ST_Start();
    fn HU_Start();

    static mut ceilingline: *mut line_t;
    static mut respawnmonsters: c_int;
    static mut gameskill: skill_t;
    static mut deathmatch: c_int;
    static mut netgame: c_int;
    static mut nomonsters: c_int;
    static mut consoleplayer: c_int;
    static mut playeringame: [c_int; NUM_PLAYERS];
    static mut players: [player_t; NUM_PLAYERS];

    static mut deathmatchstarts: [mapthing_t; 10];
    static mut deathmatch_p: *mut mapthing_t;
    static mut playerstarts: [mapthing_t; NUM_PLAYERS];

    static mut totalkills: c_int;
    static mut totalitems: c_int;
    static mut attackrange: i32;
    static mut linetarget: *mut mobj_t;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Mobj error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные p_mobj.c ---
#[unsafe(no_mangle)]
pub static mut test: c_int = 0;

#[unsafe(no_mangle)]
pub static mut itemrespawnque: [mapthing_t; Q_SIZE] = [unsafe { std::mem::zeroed() }; Q_SIZE];

#[unsafe(no_mangle)]
pub static mut itemrespawntime: [c_int; Q_SIZE] = [0; Q_SIZE];

#[unsafe(no_mangle)]
pub static mut iquehead: c_int = 0;

#[unsafe(no_mangle)]
pub static mut iquetail: c_int = 0;

// --- P_SetMobjState ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetMobjState(mobj: *mut mobj_t, mut state: statenum_t) -> c_int {
    unsafe {
        let states_base = (&raw mut states) as *mut state_t;

        loop {
            if state == S_NULL {
                (*mobj).state = ptr::null_mut();
                P_RemoveMobj(mobj);
                return 0;
            }

            let st = states_base.add(state as usize);
            (*mobj).state = st;
            (*mobj).tics = (*st).tics as i32;
            (*mobj).sprite = (*st).sprite;
            (*mobj).frame = (*st).frame as i32;

            if let Some(action_fn) = (*st).action.acp1 {
                action_fn(mobj as *mut c_void);
            }

            state = (*st).nextstate;
            if (*mobj).tics != 0 {
                break;
            }
        }

        1
    }
}

// --- P_ExplodeMissile ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ExplodeMissile(mo: *mut mobj_t) {
    unsafe {
        (*mo).momx = 0;
        (*mo).momy = 0;
        (*mo).momz = 0;

        let info_base = (&raw const mobjinfo) as *const mobjinfo_t;
        let death_st = (*info_base.add((*mo).type_ as usize)).deathstate as statenum_t;
        P_SetMobjState(mo, death_st);

        (*mo).tics -= p_random() & 3;
        if (*mo).tics < 1 {
            (*mo).tics = 1;
        }

        (*mo).flags &= !(MF_MISSILE as c_int);

        let info = (*mo).info;
        if (*info).deathsound != 0 {
            S_StartSound(mo as *mut c_void, (*info).deathsound);
        }
    }
}

// --- P_XYMovement ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_XYMovement(mo: *mut mobj_t) {
    unsafe {
        let max_move = MAXMOVE as i32;
        let frac_u = FRACUNIT as i32;

        if (*mo).momx == 0 && (*mo).momy == 0 {
            if ((*mo).flags as u32 & MF_SKULLFLY) != 0 {
                (*mo).flags &= !(MF_SKULLFLY as c_int);
                (*mo).momx = 0;
                (*mo).momy = 0;
                (*mo).momz = 0;

                let spawn_st = (*(*mo).info).spawnstate as statenum_t;
                P_SetMobjState(mo, spawn_st);
            }
            return;
        }

        let player = (*mo).player;

        if (*mo).momx > max_move {
            (*mo).momx = max_move;
        } else if (*mo).momx < -max_move {
            (*mo).momx = -max_move;
        }

        if (*mo).momy > max_move {
            (*mo).momy = max_move;
        } else if (*mo).momy < -max_move {
            (*mo).momy = -max_move;
        }

        let mut xmove = (*mo).momx;
        let mut ymove = (*mo).momy;

        loop {
            let (ptryx, ptryy) = if xmove > max_move / 2 || ymove > max_move / 2 {
                let px = (*mo).x + xmove / 2;
                let py = (*mo).y + ymove / 2;
                xmove >>= 1;
                ymove >>= 1;
                (px, py)
            } else {
                let px = (*mo).x + xmove;
                let py = (*mo).y + ymove;
                xmove = 0;
                ymove = 0;
                (px, py)
            };

            if P_TryMove(mo, ptryx, ptryy) == 0 {
                if !(*mo).player.is_null() {
                    P_SlideMove(mo);
                } else if ((*mo).flags as u32 & MF_MISSILE) != 0 {
                    if !ceilingline.is_null() {
                        let bsec = (*ceilingline).backsector;
                        if !bsec.is_null() && ((*bsec).ceilingpic as c_int) == skyflatnum {
                            P_RemoveMobj(mo);
                            return;
                        }
                    }
                    P_ExplodeMissile(mo);
                } else {
                    (*mo).momx = 0;
                    (*mo).momy = 0;
                }
            }

            if xmove == 0 && ymove == 0 {
                break;
            }
        }

        if !player.is_null() && ((*player).cheats as u32 & CF_NOMOMENTUM) != 0 {
            (*mo).momx = 0;
            (*mo).momy = 0;
            return;
        }

        if ((*mo).flags as u32 & (MF_MISSILE | MF_SKULLFLY)) != 0 {
            return;
        }

        if (*mo).z > (*mo).floorz {
            return;
        }

        if ((*mo).flags as u32 & MF_CORPSE) != 0 {
            if (*mo).momx > frac_u / 4
                || (*mo).momx < -frac_u / 4
                || (*mo).momy > frac_u / 4
                || (*mo).momy < -frac_u / 4
            {
                let sub = (*mo).subsector as *mut subsector_t;
                let sec = (*sub).sector;
                if (*mo).floorz != (*sec).floorheight {
                    return;
                }
            }
        }

        let no_cmd = if player.is_null() {
            true
        } else {
            let fwd = *(*player).cmd.as_ptr() as i8;
            let side = *(*player).cmd.as_ptr().add(1) as i8;
            fwd == 0 && side == 0
        };

        if (*mo).momx > -STOPSPEED
            && (*mo).momx < STOPSPEED
            && (*mo).momy > -STOPSPEED
            && (*mo).momy < STOPSPEED
            && no_cmd
        {
            if !player.is_null() {
                let p_mo = (*player).mo as *mut mobj_t;
                let states_base = (&raw const states) as *const state_t;
                let state_idx = (*p_mo).state.offset_from(states_base) as isize;
                if ((state_idx - (S_PLAY_RUN1 as isize)) as usize) < 4 {
                    P_SetMobjState(p_mo, S_PLAY);
                }
            }

            (*mo).momx = 0;
            (*mo).momy = 0;
        } else {
            (*mo).momx = fixed_mul((*mo).momx, FRICTION);
            (*mo).momy = fixed_mul((*mo).momy, FRICTION);
        }
    }
}

// --- P_ZMovement ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ZMovement(mo: *mut mobj_t) {
    unsafe {
        let player = (*mo).player;
        let view_h = VIEWHEIGHT as i32;
        let float_spd = FLOATSPEED as i32;
        let grav = GRAVITY as i32;

        if !player.is_null() && (*mo).z < (*mo).floorz {
            (*player).viewheight -= (*mo).floorz - (*mo).z;
            (*player).deltaviewheight = (view_h - (*player).viewheight) >> 3;
        }

        (*mo).z += (*mo).momz;

        if ((*mo).flags as u32 & MF_FLOAT) != 0 && !(*mo).target.is_null() {
            if ((*mo).flags as u32 & (MF_SKULLFLY | MF_INFLOAT)) == 0 {
                let target = (*mo).target;
                let dist = P_AproxDistance((*mo).x - (*target).x, (*mo).y - (*target).y);
                let delta = ((*target).z + ((*mo).height >> 1)) - (*mo).z;

                if delta < 0 && dist < -(delta * 3) {
                    (*mo).z -= float_spd;
                } else if delta > 0 && dist < (delta * 3) {
                    (*mo).z += float_spd;
                }
            }
        }

        if (*mo).z <= (*mo).floorz {
            if ((*mo).flags as u32 & MF_SKULLFLY) != 0 {
                (*mo).momz = -(*mo).momz;
            }

            if (*mo).momz < 0 {
                if !player.is_null() && (*mo).momz < -grav * 8 {
                    (*player).deltaviewheight = (*mo).momz >> 3;
S_StartSound(mo as *mut c_void, sfx_oof as c_int);
}
(*mo).momz = 0;
}
(*mo).z = (*mo).floorz;
if ((*mo).flags as u32 & MF_MISSILE) != 0 && ((*mo).flags as u32 & MF_NOCLIP) == 0 {
P_ExplodeMissile(mo);
return;
}
} else if ((*mo).flags as u32 & MF_NOGRAVITY) == 0 {
if (*mo).momz == 0 {
(*mo).momz = -grav * 2;
} else {
(*mo).momz -= grav;
}
}
if (*mo).z + (*mo).height > (*mo).ceilingz {
if (*mo).momz > 0 {
(*mo).momz = 0;
}
(*mo).z = (*mo).ceilingz - (*mo).height;
if ((*mo).flags as u32 & MF_SKULLFLY) != 0 {
(*mo).momz = -(*mo).momz;
}
if ((*mo).flags as u32 & MF_MISSILE) != 0 && ((*mo).flags as u32 & MF_NOCLIP) == 0 {
P_ExplodeMissile(mo);
return;
}
}
}
}
// --- P_NightmareRespawn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_NightmareRespawn(mobj: *mut mobj_t) {
unsafe {
let x = ((*mobj).spawnpoint.x as i32) << FRACBITS;
let y = ((*mobj).spawnpoint.y as i32) << FRACBITS;
if P_CheckPosition(mobj, x, y) == 0 {
return;
}
let old_sec = (*((*mobj).subsector as *mut subsector_t)).sector;
let fog1 = P_SpawnMobj((*mobj).x, (*mobj).y, (*old_sec).floorheight, MT_TFOG);
S_StartSound(fog1 as *mut c_void, sfx_telept as c_int);
let ss = R_PointInSubsector(x, y) as *mut subsector_t;
let fog2 = P_SpawnMobj(x, y, (*((*ss).sector)).floorheight, MT_TFOG);
S_StartSound(fog2 as *mut c_void, sfx_telept as c_int);
let mthing = &(*mobj).spawnpoint;
let z = if ((*(*mobj).info).flags as u32 & MF_SPAWNCEILING) != 0 {
ONCEILINGZ as i32
} else {
ONFLOORZ as i32
};
let mo = P_SpawnMobj(x, y, z, (*mobj).type_);
(*mo).spawnpoint = *mthing;
(*mo).angle = ANG45.wrapping_mul((mthing.angle / 45) as u32);
if (mthing.options as u32 & MTF_AMBUSH) != 0 {
(*mo).flags |= MF_AMBUSH as c_int;
}
(*mo).reactiontime = 18;
P_RemoveMobj(mobj);
}
}
// --- P_MobjThinker ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MobjThinker(mobj: *mut mobj_t) {
unsafe {
let tombstone = std::mem::transmute::<isize, Option<unsafe extern "C" fn()>>(-1);
if (*mobj).momx != 0 || (*mobj).momy != 0 || ((*mobj).flags as u32 & MF_SKULLFLY) != 0 {
P_XYMovement(mobj);
if (*mobj).thinker.function.acv == tombstone {
return;
}
}
if (*mobj).z != (*mobj).floorz || (*mobj).momz != 0 {
P_ZMovement(mobj);
if (*mobj).thinker.function.acv == tombstone {
return;
}
}
if (*mobj).tics != -1 {
(*mobj).tics -= 1;
if (*mobj).tics == 0 {
if P_SetMobjState(mobj, (*(*mobj).state).nextstate) == 0 {
return;
}
}
} else {
if ((*mobj).flags as u32 & MF_COUNTKILL) == 0 {
return;
}
if respawnmonsters == 0 {
return;
}
(*mobj).movecount += 1;
if (*mobj).movecount < 12 * 35 {
return;
}
if (leveltime & 31) != 0 {
return;
}
if p_random() > 4 {
return;
}
P_NightmareRespawn(mobj);
}
}
}
// --- P_SpawnMobj ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnMobj(
x: i32,
y: i32,
z: i32,
mobj_type: mobjtype_t,
) -> *mut mobj_t {
unsafe {
let mobj = Z_Malloc(size_of::<mobj_t>() as c_int, PU_LEVEL, ptr::null_mut()) as *mut mobj_t;
ptr::write_bytes(mobj as *mut u8, 0, size_of::<mobj_t>());
let info_base = (&raw mut mobjinfo) as *mut mobjinfo_t;
let info = info_base.add(mobj_type as usize);
(*mobj).type_ = mobj_type;
(*mobj).info = info;
(*mobj).x = x;
(*mobj).y = y;
(*mobj).radius = (*info).radius;
(*mobj).height = (*info).height;
(*mobj).flags = (*info).flags;
(*mobj).health = (*info).spawnhealth;
if gameskill != sk_nightmare {
(*mobj).reactiontime = (*info).reactiontime;
}
(*mobj).lastlook = p_random() % (MAXPLAYERS as c_int);
let states_base = (&raw mut states) as *mut state_t;
let st = states_base.add((*info).spawnstate as usize);
(*mobj).state = st;
(*mobj).tics = (*st).tics as i32;
(*mobj).sprite = (*st).sprite;
(*mobj).frame = (*st).frame as i32;
P_SetThingPosition(mobj);
let sec = (*((*mobj).subsector as *mut subsector_t)).sector;
(*mobj).floorz = (*sec).floorheight;
(*mobj).ceilingz = (*sec).ceilingheight;
if z == (ONFLOORZ as i32) {
(*mobj).z = (*mobj).floorz;
} else if z == (ONCEILINGZ as i32) {
(*mobj).z = (*mobj).ceilingz - (*info).height;
} else {
(*mobj).z = z;
}
let thinker_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(P_MobjThinker as unsafe extern "C" fn(*mut mobj_t));
(*mobj).thinker.function.acp1 = Some(thinker_fn);
P_AddThinker((&raw mut (*mobj).thinker) as *mut crate::p_tick::thinker_t);
mobj
}
}
// --- P_RemoveMobj ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RemoveMobj(mobj: *mut mobj_t) {
unsafe {
if ((*mobj).flags as u32 & MF_SPECIAL) != 0
&& ((*mobj).flags as u32 & MF_DROPPED) == 0
&& (*mobj).type_ != MT_INV
&& (*mobj).type_ != MT_INS
{
let q_ptr = (&raw mut itemrespawnque) as *mut mapthing_t;
let t_ptr = (&raw mut itemrespawntime) as *mut c_int;
let head_idx = iquehead as usize;
*q_ptr.add(head_idx) = (*mobj).spawnpoint;
*t_ptr.add(head_idx) = leveltime;
iquehead = (iquehead + 1) & ((ITEMQUESIZE as c_int) - 1);
if iquehead == iquetail {
iquetail = (iquetail + 1) & ((ITEMQUESIZE as c_int) - 1);
}
}
P_UnsetThingPosition(mobj);
S_StopSound(mobj as *mut c_void);
P_RemoveThinker((&raw mut (*mobj).thinker) as *mut crate::p_tick::thinker_t);
}
}
// --- P_RespawnSpecials ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RespawnSpecials() {
unsafe {
if deathmatch != 2 {
return;
}
if iquehead == iquetail {
return;
}
let t_ptr = (&raw const itemrespawntime) as *const c_int;
let tail_idx = iquetail as usize;
if leveltime - *t_ptr.add(tail_idx) < 30 * 35 {
return;
}
let q_ptr = (&raw const itemrespawnque) as *const mapthing_t;
let mthing = &*q_ptr.add(tail_idx);
let x = (mthing.x as i32) << FRACBITS;
let y = (mthing.y as i32) << FRACBITS;
let ss = R_PointInSubsector(x, y) as *mut subsector_t;
let fog = P_SpawnMobj(x, y, (*((*ss).sector)).floorheight, MT_IFOG);
S_StartSound(fog as *mut c_void, sfx_itmbk as c_int);
let info_base = (&raw const mobjinfo) as *const mobjinfo_t;
let mut found_idx = 0usize;
for i in 0..(NUMMOBJTYPES as usize) {
if (mthing.type_ as c_int) == (*info_base.add(i)).doomednum {
found_idx = i;
break;
}
}
let z = if ((*info_base.add(found_idx)).flags as u32 & MF_SPAWNCEILING) != 0 {
ONCEILINGZ as i32
} else {
ONFLOORZ as i32
};
let mo = P_SpawnMobj(x, y, z, found_idx as mobjtype_t);
(*mo).spawnpoint = *mthing;
(*mo).angle = ANG45.wrapping_mul((mthing.angle / 45) as u32);
iquetail = (iquetail + 1) & ((ITEMQUESIZE as c_int) - 1);
}
}
// --- P_SpawnPlayer ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnPlayer(mthing: *mut mapthing_t) {
unsafe {
let p_idx = ((*mthing).type_ as usize) - 1;
let in_game_ptr = (&raw const playeringame) as *const c_int;
if *in_game_ptr.add(p_idx) == 0 {
return;
}
let players_ptr = (&raw mut players) as *mut player_t;
let p = players_ptr.add(p_idx);
if (*p).playerstate == (PST_REBORN as c_int) {
G_PlayerReborn(p_idx as c_int);
}
let x = ((*mthing).x as i32) << FRACBITS;
let y = ((*mthing).y as i32) << FRACBITS;
let z = ONFLOORZ as i32;
let mobj = P_SpawnMobj(x, y, z, MT_PLAYER);
if (*mthing).type_ > 1 {
(*mobj).flags |= (((*mthing).type_ as c_int) - 1) << MF_TRANSSHIFT;
}
(*mobj).angle = ANG45.wrapping_mul(((*mthing).angle / 45) as u32);
(*mobj).player = p;
(*mobj).health = (*p).health;
(*p).mo = mobj as *mut crate::r_things::mobj_t;
(*p).playerstate = PST_LIVE as c_int;
(*p).refire = 0;
(*p).message = ptr::null_mut();
(*p).damagecount = 0;
(*p).bonuscount = 0;
(*p).extralight = 0;
(*p).fixedcolormap = 0;
(*p).viewheight = VIEWHEIGHT as i32;
P_SetupPsprites(p);
if deathmatch != 0 {
for i in 0..(NUMCARDS as usize) {
(*p).cards[i] = 1;
}
}
if (p_idx as c_int) == consoleplayer {
ST_Start();
HU_Start();
}
}
}
// --- P_SpawnMapThing ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnMapThing(mthing: *mut mapthing_t) {
unsafe {
let mtype = (*mthing).type_;
if mtype == 11 {
let dm_base = (&raw mut deathmatchstarts) as *mut mapthing_t;
if deathmatch_p < dm_base.add(10) {
*deathmatch_p = *mthing;
deathmatch_p = deathmatch_p.add(1);
}
return;
}
if mtype <= 4 {
let p_idx = (mtype as usize) - 1;
let starts_ptr = (&raw mut playerstarts) as *mut mapthing_t;
*starts_ptr.add(p_idx) = *mthing;
if deathmatch == 0 {
P_SpawnPlayer(mthing);
}
return;
}
if netgame == 0 && ((*mthing).options & 16) != 0 {
return;
}
let bit = if gameskill == sk_baby {
1
} else if gameskill == sk_nightmare {
4
} else {
1 << ((gameskill as i32) - 1)
};
if ((*mthing).options & bit) == 0 {
return;
}
let info_base = (&raw const mobjinfo) as *const mobjinfo_t;
let mut found_idx = NUMMOBJTYPES as usize;
for i in 0..(NUMMOBJTYPES as usize) {
if (mtype as c_int) == (*info_base.add(i)).doomednum {
found_idx = i;
break;
}
}
if found_idx == (NUMMOBJTYPES as usize) {
doom_error(&format!(
"P_SpawnMapThing: Unknown type {} at ({}, {})",
mtype,
(*mthing).x,
(*mthing).y
));
}
let info = info_base.add(found_idx);
if deathmatch != 0 && ((*info).flags as u32 & MF_NOTDMATCH) != 0 {
return;
}
if nomonsters != 0
&& (found_idx == (MT_SKULL as usize) || ((*info).flags as u32 & MF_COUNTKILL) != 0)
{
return;
}
let x = ((*mthing).x as i32) << FRACBITS;
let y = ((*mthing).y as i32) << FRACBITS;
let z = if ((*info).flags as u32 & MF_SPAWNCEILING) != 0 {
ONCEILINGZ as i32
} else {
ONFLOORZ as i32
};
let mobj = P_SpawnMobj(x, y, z, found_idx as mobjtype_t);
(*mobj).spawnpoint = *mthing;
if (*mobj).tics > 0 {
(*mobj).tics = 1 + (p_random() % (*mobj).tics);
}
if ((*mobj).flags as u32 & MF_COUNTKILL) != 0 {
totalkills += 1;
}
if ((*mobj).flags as u32 & MF_COUNTITEM) != 0 {
totalitems += 1;
}
(*mobj).angle = ANG45.wrapping_mul(((*mthing).angle / 45) as u32);
if ((*mthing).options as u32 & MTF_AMBUSH) != 0 {
(*mobj).flags |= MF_AMBUSH as c_int;
}
}
}
// --- P_SpawnPuff ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnPuff(x: i32, y: i32, mut z: i32) {
unsafe {
let r_diff = p_random() - p_random();
z += r_diff << 10;
let th = P_SpawnMobj(x, y, z, MT_PUFF);
(*th).momz = FRACUNIT as i32;
(*th).tics -= p_random() & 3;
if (*th).tics < 1 {
(*th).tics = 1;
}
if attackrange == (MELEERANGE as i32) {
P_SetMobjState(th, S_PUFF3);
}
}
}
// --- P_SpawnBlood ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnBlood(x: i32, y: i32, mut z: i32, damage: c_int) {
unsafe {
let r_diff = p_random() - p_random();
z += r_diff << 10;
let th = P_SpawnMobj(x, y, z, MT_BLOOD);
(*th).momz = (FRACUNIT as i32) * 2;
(*th).tics -= p_random() & 3;
if (*th).tics < 1 {
(*th).tics = 1;
}
if (9..=12).contains(&damage) {
P_SetMobjState(th, S_BLOOD2);
} else if damage < 9 {
P_SetMobjState(th, S_BLOOD3);
}
}
}
// --- P_CheckMissileSpawn ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckMissileSpawn(th: *mut mobj_t) {
unsafe {
(*th).tics -= p_random() & 3;
if (*th).tics < 1 {
(*th).tics = 1;
}
(*th).x += (*th).momx >> 1;
(*th).y += (*th).momy >> 1;
(*th).z += (*th).momz >> 1;
if P_TryMove(th, (*th).x, (*th).y) == 0 {
P_ExplodeMissile(th);
}
}
}
// --- P_SpawnMissile ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnMissile(
source: *mut mobj_t,
dest: *mut mobj_t,
mobj_type: mobjtype_t,
) -> *mut mobj_t {
unsafe {
let frac_u = FRACUNIT as i32;
let th = P_SpawnMobj((*source).x, (*source).y, (*source).z + 4 * 8 * frac_u, mobj_type);
let info = (*th).info;
if (*info).seesound != 0 {
S_StartSound(th as *mut c_void, (*info).seesound);
}
(*th).target = source;
let mut an = R_PointToAngle2((*source).x, (*source).y, (*dest).x, (*dest).y);
if ((*dest).flags as u32 & MF_SHADOW) != 0 {
let r_diff = (p_random() - p_random()) << 20;
an = an.wrapping_add(r_diff as u32);
}
(*th).angle = an;
let idx = (an >> ANGLETOFINESHIFT) as usize;
let speed = (*info).speed;
(*th).momx = fixed_mul(speed, finesine[idx + QUARTER_FINE]);
(*th).momy = fixed_mul(speed, finesine[idx]);
let mut dist = P_AproxDistance((*dest).x - (*source).x, (*dest).y - (*source).y);
dist /= speed;
if dist < 1 {
dist = 1;
}
(*th).momz = ((*dest).z - (*source).z) / dist;
P_CheckMissileSpawn(th);
th
}
}
// --- P_SpawnPlayerMissile ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnPlayerMissile(source: *mut mobj_t, mobj_type: mobjtype_t) {
unsafe {
let frac_u = FRACUNIT as i32;
let aim_dist = 16 * 64 * frac_u;
let mut an = (*source).angle;
let mut slope = P_AimLineAttack(source, an, aim_dist);
if linetarget.is_null() {
an = an.wrapping_add(1 << 26);
slope = P_AimLineAttack(source, an, aim_dist);
if linetarget.is_null() {
an = an.wrapping_sub(2 << 26);
slope = P_AimLineAttack(source, an, aim_dist);
}
if linetarget.is_null() {
an = (*source).angle;
slope = 0;
}
}
let x = (*source).x;
let y = (*source).y;
let z = (*source).z + 4 * 8 * frac_u;
let th = P_SpawnMobj(x, y, z, mobj_type);
let info = (*th).info;
if (*info).seesound != 0 {
S_StartSound(th as *mut c_void, (*info).seesound);
}
(*th).target = source;
(*th).angle = an;
let idx = (an >> ANGLETOFINESHIFT) as usize;
let speed = (*info).speed;
(*th).momx = fixed_mul(speed, finesine[idx + QUARTER_FINE]);
(*th).momy = fixed_mul(speed, finesine[idx]);
(*th).momz = fixed_mul(speed, slope);
P_CheckMissileSpawn(th);
}
}