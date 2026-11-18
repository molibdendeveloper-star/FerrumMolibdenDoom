#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::Mutex;

use crate::info::{mobjinfo, states};
use crate::m_fixed::{FRACBITS, fixed_mul};
use crate::m_random::p_random;
use crate::p_tick::{P_AddThinker, P_RemoveThinker, get_leveltime, is_thinker_removed};
use crate::r_bsp::subsector_t;
use crate::r_main::{R_PointInSubsector, R_PointToAngle2};
use crate::r_sky::get_sky_flat_num;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};
use crate::z_zone::{PU_LEVEL, Z_Malloc};

const STOPSPEED: i32 = 0x1000;
const FRICTION: i32 = 0xe800;
const ONFLOORZ: i32 = i32::MIN;
const ONCEILINGZ: i32 = i32::MAX;
const CF_NOMOMENTUM: u32 = 4;
const QUARTER_FINE: usize = FINEANGLES / 4;
const Q_SIZE: usize = ITEMQUESIZE as usize;
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

struct ItemRespawnQueue {
    items: [mapthing_t; Q_SIZE],
    times: [c_int; Q_SIZE],
    head: usize,
    tail: usize,
}

impl ItemRespawnQueue {
    const fn new() -> Self {
        Self {
            items: [EMPTY_MAPTHING; Q_SIZE],
            times: [0; Q_SIZE],
            head: 0,
            tail: 0,
        }
    }

    fn push(&mut self, spawnpoint: mapthing_t, time: c_int) {
        self.items[self.head] = spawnpoint;
        self.times[self.head] = time;
        self.head = (self.head + 1) & (Q_SIZE - 1);
        if self.head == self.tail {
            self.tail = (self.tail + 1) & (Q_SIZE - 1);
        }
    }

    fn pop_ready(&mut self, cur_time: c_int) -> Option<mapthing_t> {
        if self.head == self.tail {
            return None;
        }
        if cur_time - self.times[self.tail] < 30 * 35 {
            return None;
        }
        let item = self.items[self.tail];
        self.tail = (self.tail + 1) & (Q_SIZE - 1);
        Some(item)
    }
}

static RESPAWN_QUEUE: Mutex<ItemRespawnQueue> = Mutex::new(ItemRespawnQueue::new());

pub fn clear_respawn_queue() {
    if let Ok(mut q) = RESPAWN_QUEUE.lock() {
        q.head = 0;
        q.tail = 0;
    }
}

pub fn p_set_mobj_state(mobj: &mut mobj_t, mut state: statenum_t) -> bool {
    let states_arr = unsafe { (&raw mut states).as_mut().unwrap() };

    loop {
        if state == S_NULL {
            mobj.state = ptr::null_mut();
            unsafe {
                P_RemoveMobj(mobj as *mut mobj_t);
            }
            return false;
        }

        let st = &mut states_arr[state as usize];
        mobj.state = st;
        mobj.tics = st.tics as i32;
        mobj.sprite = st.sprite;
        mobj.frame = st.frame as i32;

        if let Some(action_fn) = unsafe { st.action.acp1 } {
            unsafe {
                action_fn((mobj as *mut mobj_t) as *mut c_void);
            }
        }

        state = st.nextstate;
        if mobj.tics != 0 {
            break;
        }
    }

    true
}

pub fn p_explode_missile(mo: &mut mobj_t) {
    mo.momx = 0;
    mo.momy = 0;
    mo.momz = 0;

    let info_arr = unsafe { (&raw const mobjinfo).as_ref().unwrap() };
    let death_st = info_arr[mo.type_ as usize].deathstate as statenum_t;
    p_set_mobj_state(mo, death_st);

    mo.tics -= p_random() & 3;
    if mo.tics < 1 {
        mo.tics = 1;
    }

    mo.flags &= !(MF_MISSILE as c_int);

    if let Some(info) = unsafe { mo.info.as_ref() } {
        if info.deathsound != 0 {
            unsafe {
                S_StartSound((mo as *mut mobj_t) as *mut c_void, info.deathsound);
            }
        }
    }
}

pub fn p_xy_movement(mo: &mut mobj_t) {
    let max_move = MAXMOVE as i32;
    let frac_u = FRACUNIT as i32;
    let mo_ptr = mo as *mut mobj_t;

    if mo.momx == 0 && mo.momy == 0 {
        if (mo.flags as u32 & MF_SKULLFLY) != 0 {
            mo.flags &= !(MF_SKULLFLY as c_int);
            mo.momx = 0;
            mo.momy = 0;
            mo.momz = 0;

            let spawn_st = unsafe { (*mo.info).spawnstate as statenum_t };
            p_set_mobj_state(mo, spawn_st);
        }
        return;
    }

    mo.momx = mo.momx.clamp(-max_move, max_move);
    mo.momy = mo.momy.clamp(-max_move, max_move);

    let mut xmove = mo.momx;
    let mut ymove = mo.momy;

    loop {
        let (ptryx, ptryy) = if xmove > max_move / 2 || ymove > max_move / 2 {
            let px = mo.x + xmove / 2;
            let py = mo.y + ymove / 2;
            xmove >>= 1;
            ymove >>= 1;
            (px, py)
        } else {
            let px = mo.x + xmove;
            let py = mo.y + ymove;
            xmove = 0;
            ymove = 0;
            (px, py)
        };

        if unsafe { P_TryMove(mo_ptr, ptryx, ptryy) } == 0 {
            if !mo.player.is_null() {
                unsafe {
                    P_SlideMove(mo_ptr);
                }
            } else if (mo.flags as u32 & MF_MISSILE) != 0 {
                let hit_sky = unsafe {
                    ceilingline
                        .as_ref()
                        .and_then(|cl| cl.backsector.as_ref())
                        .map(|bs| (bs.ceilingpic as c_int) == get_sky_flat_num())
                        .unwrap_or(false)
                };
                if hit_sky {
                    unsafe {
                        P_RemoveMobj(mo_ptr);
                    }
                    return;
                }
                p_explode_missile(mo);
            } else {
                mo.momx = 0;
                mo.momy = 0;
            }
        }

        if xmove == 0 && ymove == 0 {
            break;
        }
    }

    if let Some(player) = unsafe { (mo.player as *mut player_t).as_ref() } {
        if (player.cheats as u32 & CF_NOMOMENTUM) != 0 {
            mo.momx = 0;
            mo.momy = 0;
            return;
        }
    }

    if (mo.flags as u32 & (MF_MISSILE | MF_SKULLFLY)) != 0 || mo.z > mo.floorz {
        return;
    }

    if (mo.flags as u32 & MF_CORPSE) != 0 {
        if mo.momx > frac_u / 4
            || mo.momx < -frac_u / 4
            || mo.momy > frac_u / 4
            || mo.momy < -frac_u / 4
        {
            let sec_floor = unsafe {
                (mo.subsector as *const subsector_t)
                    .as_ref()
                    .and_then(|sub| sub.sector.as_ref())
                    .map(|sec| sec.floorheight)
                    .unwrap_or(mo.floorz)
            };
            if mo.floorz != sec_floor {
                return;
            }
        }
    }

    let player_opt = unsafe { (mo.player as *mut player_t).as_mut() };
    let no_cmd = match player_opt {
        None => true,
        Some(ref p) => (p.cmd[0] as i8) == 0 && (p.cmd[1] as i8) == 0,
    };

    if mo.momx > -STOPSPEED
        && mo.momx < STOPSPEED
        && mo.momy > -STOPSPEED
        && mo.momy < STOPSPEED
        && no_cmd
    {
        if let Some(player) = player_opt {
            if let Some(p_mo) = unsafe { (player.mo as *mut mobj_t).as_mut() } {
                let states_base = (&raw const states) as *const state_t;
                let state_idx = unsafe { p_mo.state.offset_from(states_base) };
                if ((state_idx - (S_PLAY_RUN1 as isize)) as usize) < 4 {
                    p_set_mobj_state(p_mo, S_PLAY);
                }
            }
        }

        mo.momx = 0;
        mo.momy = 0;
    } else {
        mo.momx = fixed_mul(mo.momx, FRICTION);
        mo.momy = fixed_mul(mo.momy, FRICTION);
    }
}

pub fn p_z_movement(mo: &mut mobj_t) {
    let view_h = VIEWHEIGHT as i32;
    let float_spd = FLOATSPEED as i32;
    let grav = GRAVITY as i32;
    let mo_ptr = mo as *mut mobj_t;

    if let Some(player) = unsafe { (mo.player as *mut player_t).as_mut() } {
        if mo.z < mo.floorz {
            player.viewheight -= mo.floorz - mo.z;
            player.deltaviewheight = (view_h - player.viewheight) >> 3;
        }
    }

    mo.z += mo.momz;

    if (mo.flags as u32 & MF_FLOAT) != 0 && (mo.flags as u32 & (MF_SKULLFLY | MF_INFLOAT)) == 0 {
        if let Some(target) = unsafe { mo.target.as_ref() } {
            let dist = unsafe { P_AproxDistance(mo.x - target.x, mo.y - target.y) };
            let delta = (target.z + (mo.height >> 1)) - mo.z;

            if delta < 0 && dist < -(delta * 3) {
                mo.z -= float_spd;
} else if delta > 0 && dist < (delta * 3) {
mo.z += float_spd;
}
}
}
if mo.z <= mo.floorz {
if (mo.flags as u32 & MF_SKULLFLY) != 0 {
mo.momz = -mo.momz;
}
if mo.momz < 0 {
if let Some(player) = unsafe { (mo.player as *mut player_t).as_mut() } {
if mo.momz < -grav * 8 {
player.deltaviewheight = mo.momz >> 3;
unsafe {
S_StartSound(mo_ptr as *mut c_void, sfx_oof as c_int);
}
}
}
mo.momz = 0;
}
mo.z = mo.floorz;
if (mo.flags as u32 & MF_MISSILE) != 0 && (mo.flags as u32 & MF_NOCLIP) == 0 {
p_explode_missile(mo);
return;
}
} else if (mo.flags as u32 & MF_NOGRAVITY) == 0 {
if mo.momz == 0 {
mo.momz = -grav * 2;
} else {
mo.momz -= grav;
}
}
if mo.z + mo.height > mo.ceilingz {
if mo.momz > 0 {
mo.momz = 0;
}
mo.z = mo.ceilingz - mo.height;
if (mo.flags as u32 & MF_SKULLFLY) != 0 {
mo.momz = -mo.momz;
}
if (mo.flags as u32 & MF_MISSILE) != 0 && (mo.flags as u32 & MF_NOCLIP) == 0 {
p_explode_missile(mo);
}
}
}
pub fn p_mobj_thinker(mobj: &mut mobj_t) {
if mobj.momx != 0 || mobj.momy != 0 || (mobj.flags as u32 & MF_SKULLFLY) != 0 {
p_xy_movement(mobj);
if is_thinker_removed(&mobj.thinker) {
return;
}
}
if mobj.z != mobj.floorz || mobj.momz != 0 {
p_z_movement(mobj);
if is_thinker_removed(&mobj.thinker) {
return;
}
}
if mobj.tics != -1 {
mobj.tics -= 1;
if mobj.tics == 0 {
let next_st = unsafe { (*mobj.state).nextstate };
if !p_set_mobj_state(mobj, next_st) {
return;
}
}
} else {
if (mobj.flags as u32 & MF_COUNTKILL) == 0 || unsafe { respawnmonsters == 0 } {
return;
}
mobj.movecount += 1;
if mobj.movecount < 12 * 35 || (get_leveltime() & 31) != 0 || p_random() > 4 {
return;
}
unsafe {
P_NightmareRespawn(mobj as *mut mobj_t);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetMobjState(mobj: *mut mobj_t, state: statenum_t) -> c_int {
match unsafe { mobj.as_mut() } {
Some(mo) => p_set_mobj_state(mo, state) as c_int,
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ExplodeMissile(mo: *mut mobj_t) {
if let Some(m) = unsafe { mo.as_mut() } {
p_explode_missile(m);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_XYMovement(mo: *mut mobj_t) {
if let Some(m) = unsafe { mo.as_mut() } {
p_xy_movement(m);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ZMovement(mo: *mut mobj_t) {
if let Some(m) = unsafe { mo.as_mut() } {
p_z_movement(m);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_NightmareRespawn(mobj: *mut mobj_t) {
let Some(mo_old) = (unsafe { mobj.as_mut() }) else { return };
unsafe {
let x = (mo_old.spawnpoint.x as i32) << FRACBITS;
let y = (mo_old.spawnpoint.y as i32) << FRACBITS;
if P_CheckPosition(mobj, x, y) == 0 {
return;
}
let old_sec = (*((mo_old.subsector) as *mut subsector_t)).sector;
let fog1 = P_SpawnMobj(mo_old.x, mo_old.y, (*old_sec).floorheight, MT_TFOG);
S_StartSound(fog1 as *mut c_void, sfx_telept as c_int);
let ss = R_PointInSubsector(x, y) as *mut subsector_t;
let fog2 = P_SpawnMobj(x, y, (*((*ss).sector)).floorheight, MT_TFOG);
S_StartSound(fog2 as *mut c_void, sfx_telept as c_int);
let mthing = mo_old.spawnpoint;
let z = if ((*mo_old.info).flags as u32 & MF_SPAWNCEILING) != 0 {
ONCEILINGZ
} else {
ONFLOORZ
};
let mo = &mut *P_SpawnMobj(x, y, z, mo_old.type_);
mo.spawnpoint = mthing;
mo.angle = ANG45.wrapping_mul((mthing.angle / 45) as u32);
if (mthing.options as u32 & MTF_AMBUSH) != 0 {
mo.flags |= MF_AMBUSH as c_int;
}
mo.reactiontime = 18;
P_RemoveMobj(mobj);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MobjThinker(mobj: *mut mobj_t) {
if let Some(m) = unsafe { mobj.as_mut() } {
p_mobj_thinker(m);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnMobj(
x: i32,
y: i32,
z: i32,
mobj_type: mobjtype_t,
) -> *mut mobj_t {
unsafe {
let mobj_ptr =
Z_Malloc(size_of::<mobj_t>() as c_int, PU_LEVEL, ptr::null_mut()) as *mut mobj_t;
ptr::write_bytes(mobj_ptr as *mut u8, 0, size_of::<mobj_t>());
let mobj = &mut *mobj_ptr;
let info_arr = (&raw mut mobjinfo).as_mut().unwrap();
let info = &mut info_arr[mobj_type as usize];
mobj.type_ = mobj_type;
mobj.info = info;
mobj.x = x;
mobj.y = y;
mobj.radius = info.radius;
mobj.height = info.height;
mobj.flags = info.flags;
mobj.health = info.spawnhealth;
if gameskill != sk_nightmare {
mobj.reactiontime = info.reactiontime;
}
mobj.lastlook = p_random() % (MAXPLAYERS as c_int);
let states_arr = (&raw mut states).as_mut().unwrap();
let st = &mut states_arr[info.spawnstate as usize];
mobj.state = st;
mobj.tics = st.tics as i32;
mobj.sprite = st.sprite;
mobj.frame = st.frame as i32;
P_SetThingPosition(mobj_ptr);
let sec = &*(*(mobj.subsector as *mut subsector_t)).sector;
mobj.floorz = sec.floorheight;
mobj.ceilingz = sec.ceilingheight;
mobj.z = if z == ONFLOORZ {
mobj.floorz
} else if z == ONCEILINGZ {
mobj.ceilingz - info.height
} else {
z
};
let thinker_fn: unsafe extern "C" fn(*mut c_void) =
std::mem::transmute(P_MobjThinker as unsafe extern "C" fn(*mut mobj_t));
mobj.thinker.function.acp1 = Some(thinker_fn);
P_AddThinker((&raw mut mobj.thinker) as *mut crate::p_tick::thinker_t);
mobj_ptr
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RemoveMobj(mobj: *mut mobj_t) {
let Some(mo) = (unsafe { mobj.as_mut() }) else { return };
if (mo.flags as u32 & MF_SPECIAL) != 0
&& (mo.flags as u32 & MF_DROPPED) == 0
&& mo.type_ != MT_INV
&& mo.type_ != MT_INS
{
RESPAWN_QUEUE
.lock()
.unwrap()
.push(mo.spawnpoint, get_leveltime());
}
unsafe {
P_UnsetThingPosition(mobj);
S_StopSound(mobj as *mut c_void);
P_RemoveThinker((&raw mut mo.thinker) as *mut crate::p_tick::thinker_t);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RespawnSpecials() {
if unsafe { deathmatch != 2 } {
return;
}
let Some(mthing) = RESPAWN_QUEUE.lock().unwrap().pop_ready(get_leveltime()) else {
return;
};
unsafe {
let x = (mthing.x as i32) << FRACBITS;
let y = (mthing.y as i32) << FRACBITS;
let ss = R_PointInSubsector(x, y) as *mut subsector_t;
let fog = P_SpawnMobj(x, y, (*((*ss).sector)).floorheight, MT_IFOG);
S_StartSound(fog as *mut c_void, sfx_itmbk as c_int);
let info_arr = (&raw const mobjinfo).as_ref().unwrap();
let found_idx = info_arr
.iter()
.position(|info| (mthing.type_ as c_int) == info.doomednum)
.unwrap_or(0);
let z = if (info_arr[found_idx].flags as u32 & MF_SPAWNCEILING) != 0 {
ONCEILINGZ
} else {
ONFLOORZ
};
let mo = &mut *P_SpawnMobj(x, y, z, found_idx as mobjtype_t);
mo.spawnpoint = mthing;
mo.angle = ANG45.wrapping_mul((mthing.angle / 45) as u32);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnPlayer(mthing: *mut mapthing_t) {
let Some(mt) = (unsafe { mthing.as_ref() }) else { return };
unsafe {
let p_idx = (mt.type_ as usize) - 1;
let in_game = (&raw const playeringame).as_ref().unwrap();
if in_game[p_idx] == 0 {
return;
}
let players_arr = (&raw mut players).as_mut().unwrap();
let p = &mut players_arr[p_idx];
if p.playerstate == (PST_REBORN as c_int) {
G_PlayerReborn(p_idx as c_int);
}
let x = (mt.x as i32) << FRACBITS;
let y = (mt.y as i32) << FRACBITS;
let mobj_ptr = P_SpawnMobj(x, y, ONFLOORZ, MT_PLAYER);
let mobj = &mut *mobj_ptr;
if mt.type_ > 1 {
mobj.flags |= ((mt.type_ as c_int) - 1) << MF_TRANSSHIFT;
}
mobj.angle = ANG45.wrapping_mul((mt.angle / 45) as u32);
mobj.player = p;
mobj.health = p.health;
p.mo = mobj_ptr as *mut crate::r_things::mobj_t;
p.playerstate = PST_LIVE as c_int;
p.refire = 0;
p.message = ptr::null_mut();
p.damagecount = 0;
p.bonuscount = 0;
p.extralight = 0;
p.fixedcolormap = 0;
p.viewheight = VIEWHEIGHT as i32;
P_SetupPsprites(p);
if deathmatch != 0 {
p.cards.fill(1);
}
if (p_idx as c_int) == consoleplayer {
ST_Start();
HU_Start();
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnMapThing(mthing: *mut mapthing_t) {
let Some(mt) = (unsafe { mthing.as_ref() }) else { return };
unsafe {
let mtype = mt.type_;
if mtype == 11 {
let dm_base = (&raw mut deathmatchstarts) as *mut mapthing_t;
if deathmatch_p < dm_base.add(10) {
*deathmatch_p = *mt;
deathmatch_p = deathmatch_p.add(1);
}
return;
}
if mtype <= 4 {
let p_idx = (mtype as usize) - 1;
let starts = (&raw mut playerstarts).as_mut().unwrap();
starts[p_idx] = *mt;
if deathmatch == 0 {
P_SpawnPlayer(mthing);
}
return;
}
if netgame == 0 && (mt.options & 16) != 0 {
return;
}
let bit = if gameskill == sk_baby {
1
} else if gameskill == sk_nightmare {
4
} else {
1 << ((gameskill as i32) - 1)
};
if (mt.options & bit) == 0 {
return;
}
let info_arr = (&raw const mobjinfo).as_ref().unwrap();
let Some(found_idx) = info_arr
.iter()
.position(|info| (mtype as c_int) == info.doomednum)
else {
doom_error(&format!(
"P_SpawnMapThing: Unknown type {} at ({}, {})",
mtype, mt.x, mt.y
));
};
let info = &info_arr[found_idx];
if deathmatch != 0 && (info.flags as u32 & MF_NOTDMATCH) != 0 {
return;
}
if nomonsters != 0
&& (found_idx == (MT_SKULL as usize) || (info.flags as u32 & MF_COUNTKILL) != 0)
{
return;
}
let x = (mt.x as i32) << FRACBITS;
let y = (mt.y as i32) << FRACBITS;
let z = if (info.flags as u32 & MF_SPAWNCEILING) != 0 {
ONCEILINGZ
} else {
ONFLOORZ
};
let mobj = &mut *P_SpawnMobj(x, y, z, found_idx as mobjtype_t);
mobj.spawnpoint = *mt;
if mobj.tics > 0 {
mobj.tics = 1 + (p_random() % mobj.tics);
}
if (mobj.flags as u32 & MF_COUNTKILL) != 0 {
totalkills += 1;
}
if (mobj.flags as u32 & MF_COUNTITEM) != 0 {
totalitems += 1;
}
mobj.angle = ANG45.wrapping_mul((mt.angle / 45) as u32);
if (mt.options as u32 & MTF_AMBUSH) != 0 {
mobj.flags |= MF_AMBUSH as c_int;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnPuff(x: i32, y: i32, mut z: i32) {
let r_diff = p_random() - p_random();
z += r_diff << 10;
let th = unsafe { &mut *P_SpawnMobj(x, y, z, MT_PUFF) };
th.momz = FRACUNIT as i32;
th.tics -= p_random() & 3;
if th.tics < 1 {
th.tics = 1;
}
if unsafe { attackrange == (MELEERANGE as i32) } {
p_set_mobj_state(th, S_PUFF3);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnBlood(x: i32, y: i32, mut z: i32, damage: c_int) {
let r_diff = p_random() - p_random();
z += r_diff << 10;
let th = unsafe { &mut *P_SpawnMobj(x, y, z, MT_BLOOD) };
th.momz = (FRACUNIT as i32) * 2;
th.tics -= p_random() & 3;
if th.tics < 1 {
th.tics = 1;
}
if (9..=12).contains(&damage) {
p_set_mobj_state(th, S_BLOOD2);
} else if damage < 9 {
p_set_mobj_state(th, S_BLOOD3);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckMissileSpawn(th: *mut mobj_t) {
let Some(m) = (unsafe { th.as_mut() }) else { return };
m.tics -= p_random() & 3;
if m.tics < 1 {
m.tics = 1;
}
m.x += m.momx >> 1;
m.y += m.momy >> 1;
m.z += m.momz >> 1;
if unsafe { P_TryMove(th, m.x, m.y) } == 0 {
p_explode_missile(m);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnMissile(
source: *mut mobj_t,
dest: *mut mobj_t,
mobj_type: mobjtype_t,
) -> *mut mobj_t {
let (Some(src), Some(dst)) = (unsafe { source.as_ref() }, unsafe { dest.as_ref() }) else {
return ptr::null_mut();
};
unsafe {
let frac_u = FRACUNIT as i32;
let th_ptr = P_SpawnMobj(src.x, src.y, src.z + 4 * 8 * frac_u, mobj_type);
let th = &mut *th_ptr;
let info = &*th.info;
if info.seesound != 0 {
S_StartSound(th_ptr as *mut c_void, info.seesound);
}
th.target = source;
let mut an = R_PointToAngle2(src.x, src.y, dst.x, dst.y);
if (dst.flags as u32 & MF_SHADOW) != 0 {
let r_diff = (p_random() - p_random()) << 20;
an = an.wrapping_add(r_diff as u32);
}
th.angle = an;
let idx = (an >> ANGLETOFINESHIFT) as usize;
let speed = info.speed;
th.momx = fixed_mul(speed, finesine[idx + QUARTER_FINE]);
th.momy = fixed_mul(speed, finesine[idx]);
let dist = (P_AproxDistance(dst.x - src.x, dst.y - src.y) / speed).max(1);
th.momz = (dst.z - src.z) / dist;
P_CheckMissileSpawn(th_ptr);
th_ptr
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnPlayerMissile(source: *mut mobj_t, mobj_type: mobjtype_t) {
let Some(src) = (unsafe { source.as_ref() }) else { return };
unsafe {
let frac_u = FRACUNIT as i32;
let aim_dist = 16 * 64 * frac_u;
let mut an = src.angle;
let mut slope = P_AimLineAttack(source, an, aim_dist);
if linetarget.is_null() {
an = an.wrapping_add(1 << 26);
slope = P_AimLineAttack(source, an, aim_dist);
if linetarget.is_null() {
an = an.wrapping_sub(2 << 26);
slope = P_AimLineAttack(source, an, aim_dist);
}
if linetarget.is_null() {
an = src.angle;
slope = 0;
}
}
let th_ptr = P_SpawnMobj(src.x, src.y, src.z + 4 * 8 * frac_u, mobj_type);
let th = &mut *th_ptr;
let info = &*th.info;
if info.seesound != 0 {
S_StartSound(th_ptr as *mut c_void, info.seesound);
}
th.target = source;
th.angle = an;
let idx = (an >> ANGLETOFINESHIFT) as usize;
let speed = info.speed;
th.momx = fixed_mul(speed, finesine[idx + QUARTER_FINE]);
th.momy = fixed_mul(speed, finesine[idx]);
th.momz = fixed_mul(speed, slope);
P_CheckMissileSpawn(th_ptr);
}
}
