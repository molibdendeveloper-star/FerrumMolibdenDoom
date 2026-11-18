#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::{get_gamemode};
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_void};
use std::ptr;

use crate::info::mobjinfo;
use crate::m_fixed::fixed_mul;
use crate::m_random::p_random;
use crate::p_floor::EV_DoFloor;
use crate::p_inter::P_DamageMobj;
use crate::p_map::{
    P_AimLineAttack, P_CheckPosition, P_LineAttack, P_RadiusAttack,
    P_TeleportMove, P_TryMove, floatok, numspechit, spechit, tmfloorz,
};
use crate::p_maputl::{
    P_AproxDistance, P_BlockThingsIterator, P_LineOpening, P_SetThingPosition, P_UnsetThingPosition, openrange,
};
use crate::p_mobj::{
    P_MobjThinker, P_RemoveMobj, P_SetMobjState, P_SpawnMissile, P_SpawnMobj, P_SpawnPuff,
};
use crate::p_pspr::A_ReFire;
use crate::p_setup::{bmaporgx, bmaporgy, sides};
use crate::p_sight::P_CheckSight;
use crate::p_switch::P_UseSpecialLine;
use crate::p_tick::thinkercap;
use crate::r_bsp::subsector_t;
use crate::r_main::{R_PointToAngle2, validcount};
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};

const QUARTER_FINE: usize = FINEANGLES / 4;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;

// Направления движения (dirtype_t)
const DI_EAST: u32 = 0;
const DI_NORTHEAST: u32 = 1;
const DI_NORTH: u32 = 2;
const DI_NORTHWEST: u32 = 3;
const DI_WEST: u32 = 4;
const DI_SOUTHWEST: u32 = 5;
const DI_SOUTH: u32 = 6;
const DI_SOUTHEAST: u32 = 7;
const DI_NODIR: u32 = 8;

static OPPOSITE: [u32; 9] = [
    DI_WEST,
    DI_SOUTHWEST,
    DI_SOUTH,
    DI_SOUTHEAST,
    DI_EAST,
    DI_NORTHEAST,
    DI_NORTH,
    DI_NORTHWEST,
    DI_NODIR,
];

static DIAGS: [u32; 4] = [DI_NORTHWEST, DI_NORTHEAST, DI_SOUTHWEST, DI_SOUTHEAST];

const FATSPREAD: u32 = ANG90 / 8;
const SKULLSPEED: i32 = 20 * (FRACUNIT as i32);

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn G_ExitLevel();
    fn EV_DoDoor(line: *mut line_t, door_type: vldoor_e) -> c_int;

    static mut gameskill: skill_t;
    static mut gameepisode: c_int;
    static mut gamemap: c_int;
    static mut fastparm: c_int;
    static mut netgame: c_int;
    static mut gametic: c_int;
    static mut playeringame: [c_int; NUM_PLAYERS];
    static mut players: [player_t; NUM_PLAYERS];
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Enemy AI error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные p_enemy.c ---
#[unsafe(no_mangle)]
pub static mut soundtarget: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut xspeed: [i32; 8] = [
    FRACUNIT as i32,
    47000,
    0,
    -47000,
    -(FRACUNIT as i32),
    -47000,
    0,
    47000,
];

#[unsafe(no_mangle)]
pub static mut yspeed: [i32; 8] = [
    0,
    47000,
    FRACUNIT as i32,
    47000,
    0,
    -47000,
    -(FRACUNIT as i32),
    -47000,
];

#[unsafe(no_mangle)]
pub static mut TRACEANGLE: i32 = 0xc000000;

#[unsafe(no_mangle)]
pub static mut corpsehit: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut vileobj: *mut mobj_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut viletryx: i32 = 0;

#[unsafe(no_mangle)]
pub static mut viletryy: i32 = 0;

#[unsafe(no_mangle)]
pub static mut braintargets: [*mut mobj_t; 32] = [ptr::null_mut(); 32];

#[unsafe(no_mangle)]
pub static mut numbraintargets: c_int = 0;

#[unsafe(no_mangle)]
pub static mut braintargeton: c_int = 0;

// --- P_RecursiveSound ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RecursiveSound(sec: *mut sector_t, soundblocks: c_int) {
    unsafe {
        if (*sec).validcount == validcount && (*sec).soundtraversed <= soundblocks + 1 {
            return;
        }

        (*sec).validcount = validcount;
        (*sec).soundtraversed = soundblocks + 1;
        (*sec).soundtarget = soundtarget;

        let lines_ptr = (*sec).lines;
        let sides_base = sides as *mut side_t;

        for i in 0..((*sec).linecount as usize) {
            let check = *lines_ptr.add(i);
            if ((*check).flags as u32 & ML_TWOSIDED) == 0 {
                continue;
            }

            P_LineOpening(check as *mut _);
            if openrange <= 0 {
                continue;
            }

            let snum_ptr = (&raw const (*check).sidenum) as *const i16;
            let s0 = *snum_ptr as usize;
            let s1 = *snum_ptr.add(1) as usize;

            let other = if (*sides_base.add(s0)).sector == sec {
                (*sides_base.add(s1)).sector
            } else {
                (*sides_base.add(s0)).sector
            };

            if ((*check).flags as u32 & ML_SOUNDBLOCK) != 0 {
                if soundblocks == 0 {
                    P_RecursiveSound(other, 1);
                }
            } else {
                P_RecursiveSound(other, soundblocks);
            }
        }
    }
}

// --- P_NoiseAlert ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_NoiseAlert(target: *mut mobj_t, emmiter: *mut mobj_t) {
    unsafe {
        soundtarget = target;
        validcount += 1;
        let sub = (*emmiter).subsector as *mut subsector_t;
        P_RecursiveSound((*sub).sector as *mut sector_t, 0);
    }
}

// --- P_CheckMeleeRange ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckMeleeRange(actor: *mut mobj_t) -> c_int {
    unsafe {
        let pl = (*actor).target;
        if pl.is_null() {
            return 0;
        }

        let dist = P_AproxDistance((*pl).x.wrapping_sub((*actor).x), (*pl).y.wrapping_sub((*actor).y));
        let limit = (MELEERANGE as i32) - 20 * (FRACUNIT as i32) + (*(*pl).info).radius;

        if dist >= limit {
            return 0;
        }

        if P_CheckSight(actor as *mut _, pl as *mut _) == 0 {
            return 0;
        }

        1
    }
}

// --- P_CheckMissileRange ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckMissileRange(actor: *mut mobj_t) -> c_int {
    unsafe {
        let target = (*actor).target;
        if P_CheckSight(actor as *mut _, target as *mut _) == 0 {
            return 0;
        }

        if ((*actor).flags as u32 & MF_JUSTHIT) != 0 {
            (*actor).flags &= !(MF_JUSTHIT as c_int);
            return 1;
        }

        if (*actor).reactiontime != 0 {
            return 0;
        }

        let frac_u = FRACUNIT as i32;
        let mut dist = P_AproxDistance(
            (*actor).x.wrapping_sub((*target).x),
            (*actor).y.wrapping_sub((*target).y),
        ) - 64 * frac_u;

        if (*(*actor).info).meleestate == 0 {
            dist -= 128 * frac_u;
        }

        dist >>= 16;

        let t = (*actor).type_;
        if t == MT_VILE && dist > 14 * 64 {
            return 0;
        }

        if t == MT_UNDEAD {
            if dist < 196 {
                return 0;
            }
            dist >>= 1;
        }

        if t == MT_CYBORG || t == MT_SPIDER || t == MT_SKULL {
            dist >>= 1;
        }

        if dist > 200 {
            dist = 200;
        }
        if t == MT_CYBORG && dist > 160 {
            dist = 160;
        }

        if p_random() < dist {
            return 0;
        }

        1
    }
}

// --- P_Move ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_Move(actor: *mut mobj_t) -> c_int {
    unsafe {
        let dir = (*actor).movedir as u32;
        if dir == DI_NODIR {
            return 0;
        }

        if dir >= 8 {
            doom_error("Weird actor->movedir!");
        }

        let speed = (*(*actor).info).speed;
        let xs = *(&raw const xspeed as *const i32).add(dir as usize);
        let ys = *(&raw const yspeed as *const i32).add(dir as usize);

        let tryx = (*actor).x.wrapping_add(speed.wrapping_mul(xs));
        let tryy = (*actor).y.wrapping_add(speed.wrapping_mul(ys));

        let try_ok = P_TryMove(actor as *mut _, tryx, tryy);

        if try_ok == 0 {
            if ((*actor).flags as u32 & MF_FLOAT) != 0 && floatok != 0 {
                let flt_spd = FLOATSPEED as i32;
                if (*actor).z < tmfloorz {
                    (*actor).z += flt_spd;
                } else {
                    (*actor).z -= flt_spd;
                }
                (*actor).flags |= MF_INFLOAT as c_int;
                return 1;
            }

            if numspechit == 0 {
                return 0;
            }

            (*actor).movedir = DI_NODIR as c_int;
            let mut good = 0;
            let spec_ptr = (&raw const spechit) as *const *mut line_t;

            while numspechit > 0 {
                numspechit -= 1;
                let ld = *spec_ptr.add(numspechit as usize);
                if P_UseSpecialLine(actor as *mut _, ld as *mut _, 0) != 0 {
                    good = 1;
                }
            }
            return good;
        } else {
            (*actor).flags &= !(MF_INFLOAT as c_int);
        }

        if ((*actor).flags as u32 & MF_FLOAT) == 0 {
            (*actor).z = (*actor).floorz;
        }

        1
    }
}

// --- P_TryWalk ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TryWalk(actor: *mut mobj_t) -> c_int {
    unsafe {
        if P_Move(actor) == 0 {
            return 0;
        }
        (*actor).movecount = p_random() & 15;
        1
    }
}

// --- P_NewChaseDir ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_NewChaseDir(actor: *mut mobj_t) {
    unsafe {
        let target = (*actor).target;
        if target.is_null() {
            doom_error("P_NewChaseDir: called with no target");
        }

        let olddir = (*actor).movedir as u32;
        let opp_ptr = (&raw const OPPOSITE) as *const u32;
        let turnaround = *opp_ptr.add(olddir as usize);

        let deltax = (*target).x.wrapping_sub((*actor).x);
let deltay = (*target).y.wrapping_sub((*actor).y);
let thresh = 10 * (FRACUNIT as i32);
let mut d1 = if deltax > thresh {
DI_EAST
} else if deltax < -thresh {
DI_WEST
} else {
DI_NODIR
};
let mut d2 = if deltay < -thresh {
DI_SOUTH
} else if deltay > thresh {
DI_NORTH
} else {
DI_NODIR
};
if d1 != DI_NODIR && d2 != DI_NODIR {
let diag_idx = (((deltay < 0) as usize) << 1) + ((deltax > 0) as usize);
let diags_ptr = (&raw const DIAGS) as *const u32;
(*actor).movedir = *diags_ptr.add(diag_idx) as c_int;
if ((*actor).movedir as u32) != turnaround && P_TryWalk(actor) != 0 {
return;
}
}
if p_random() > 200 || deltay.wrapping_abs() > deltax.wrapping_abs() {
std::mem::swap(&mut d1, &mut d2);
}
if d1 == turnaround {
d1 = DI_NODIR;
}
if d2 == turnaround {
d2 = DI_NODIR;
}
if d1 != DI_NODIR {
(*actor).movedir = d1 as c_int;
if P_TryWalk(actor) != 0 {
return;
}
}
if d2 != DI_NODIR {
(*actor).movedir = d2 as c_int;
if P_TryWalk(actor) != 0 {
return;
}
}
if olddir != DI_NODIR {
(*actor).movedir = olddir as c_int;
if P_TryWalk(actor) != 0 {
return;
}
}
if (p_random() & 1) != 0 {
for tdir in DI_EAST..=DI_SOUTHEAST {
if tdir != turnaround {
(*actor).movedir = tdir as c_int;
if P_TryWalk(actor) != 0 {
return;
}
}
}
} else {
let mut tdir = DI_SOUTHEAST as c_int;
while tdir >= (DI_EAST as c_int) {
if (tdir as u32) != turnaround {
(*actor).movedir = tdir;
if P_TryWalk(actor) != 0 {
return;
}
}
tdir -= 1;
}
}
if turnaround != DI_NODIR {
(*actor).movedir = turnaround as c_int;
if P_TryWalk(actor) != 0 {
return;
}
}
(*actor).movedir = DI_NODIR as c_int;
}
}
// --- P_LookForPlayers ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_LookForPlayers(actor: *mut mobj_t, allaround: c_int) -> c_int {
unsafe {
let mut c = 0;
let stop = ((*actor).lastlook - 1) & 3;
let in_game_ptr = (&raw const playeringame) as *const c_int;
let players_base = (&raw mut players) as *mut player_t;
loop {
let look = (*actor).lastlook as usize;
if *in_game_ptr.add(look) == 0 {
(*actor).lastlook = ((*actor).lastlook + 1) & 3;
continue;
}
if c == 2 || (*actor).lastlook == stop {
return 0;
}
c += 1;
let player = players_base.add(look);
if (*player).health <= 0 {
(*actor).lastlook = ((*actor).lastlook + 1) & 3;
continue;
}
let p_mo = (*player).mo as *mut mobj_t;
if P_CheckSight(actor as *mut _, p_mo as *mut _) == 0 {
(*actor).lastlook = ((*actor).lastlook + 1) & 3;
continue;
}
if allaround == 0 {
let an = R_PointToAngle2((*actor).x, (*actor).y, (*p_mo).x, (*p_mo).y)
.wrapping_sub((*actor).angle);
if an > ANG90 && an < ANG270 {
let dist = P_AproxDistance(
(*p_mo).x.wrapping_sub((*actor).x),
(*p_mo).y.wrapping_sub((*actor).y),
);
if dist > (MELEERANGE as i32) {
(*actor).lastlook = ((*actor).lastlook + 1) & 3;
continue;
}
}
}
(*actor).target = p_mo;
return 1;
}
}
}
// --- A_KeenDie ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_KeenDie(mo: *mut mobj_t) {
unsafe {
A_Fall(mo);
let cap = &raw mut thinkercap;
let mut th = (*cap).next;
let mobj_thinker_addr = P_MobjThinker as *const ();
while th != cap {
let is_mobj = match (*th).function.acp1 {
Some(f) => (f as *const ()) == mobj_thinker_addr,
None => false,
};
if is_mobj {
let mo2 = th as *mut mobj_t;
if mo2 != mo && (*mo2).type_ == (*mo).type_ && (*mo2).health > 0 {
return;
}
}
th = (*th).next;
}
let mut junk: line_t = std::mem::zeroed();
junk.tag = 666;
EV_DoDoor(&raw mut junk, open);
}
}
// --- A_Look ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Look(actor: *mut mobj_t) {
unsafe {
(*actor).threshold = 0;
let sub = (*actor).subsector as *mut subsector_t;
let sec = (*sub).sector as *mut sector_t;
let targ = (*sec).soundtarget;
let mut see_you = false;
if !targ.is_null() && ((*targ).flags as u32 & MF_SHOOTABLE) != 0 {
(*actor).target = targ;
if ((*actor).flags as u32 & MF_AMBUSH) != 0 {
if P_CheckSight(actor as *mut _, targ as *mut _) != 0 {
see_you = true;
}
} else {
see_you = true;
}
}
if !see_you && P_LookForPlayers(actor, 0) == 0 {
return;
}
let info = (*actor).info;
if (*info).seesound != 0 {
let sound = match (*info).seesound as u32 {
sfx_posit1 | sfx_posit2 | sfx_posit3 => (sfx_posit1 as c_int) + (p_random() % 3),
sfx_bgsit1 | sfx_bgsit2 => (sfx_bgsit1 as c_int) + (p_random() % 2),
other => other as c_int,
};
if (*actor).type_ == MT_SPIDER || (*actor).type_ == MT_CYBORG {
S_StartSound(ptr::null_mut(), sound);
} else {
S_StartSound(actor as *mut c_void, sound);
}
}
P_SetMobjState(actor as *mut _, (*info).seestate as _);
}
}
// --- A_Chase ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Chase(actor: *mut mobj_t) {
unsafe {
if (*actor).reactiontime != 0 {
(*actor).reactiontime -= 1;
}
if (*actor).threshold != 0 {
let target = (*actor).target;
if target.is_null() || (*target).health <= 0 {
(*actor).threshold = 0;
} else {
(*actor).threshold -= 1;
}
}
if (*actor).movedir < 8 {
(*actor).angle &= 7u32 << 29;
let delta = ((*actor).angle as i32).wrapping_sub((*actor).movedir << 29);
if delta > 0 {
(*actor).angle = (*actor).angle.wrapping_sub(ANG90 / 2);
} else if delta < 0 {
(*actor).angle = (*actor).angle.wrapping_add(ANG90 / 2);
}
}
let target = (*actor).target;
let info = (*actor).info;
if target.is_null() || ((*target).flags as u32 & MF_SHOOTABLE) == 0 {
if P_LookForPlayers(actor, 1) != 0 {
return;
}
P_SetMobjState(actor as *mut _, (*info).spawnstate as _);
return;
}
if ((*actor).flags as u32 & MF_JUSTATTACKED) != 0 {
(*actor).flags &= !(MF_JUSTATTACKED as c_int);
if gameskill != sk_nightmare && fastparm == 0 {
P_NewChaseDir(actor);
}
return;
}
if (*info).meleestate != 0 && P_CheckMeleeRange(actor) != 0 {
if (*info).attacksound != 0 {
S_StartSound(actor as *mut c_void, (*info).attacksound);
}
P_SetMobjState(actor as *mut _, (*info).meleestate as _);
return;
}
if (*info).missilestate != 0 {
let skip_missile =
gameskill < sk_nightmare && fastparm == 0 && (*actor).movecount != 0;
if !skip_missile && P_CheckMissileRange(actor) != 0 {
P_SetMobjState(actor as *mut _, (*info).missilestate as _);
(*actor).flags |= MF_JUSTATTACKED as c_int;
return;
}
}
if netgame != 0 && (*actor).threshold == 0 && P_CheckSight(actor as *mut _, target as *mut _) == 0
{
if P_LookForPlayers(actor, 1) != 0 {
return;
}
}
(*actor).movecount -= 1;
if (*actor).movecount < 0 || P_Move(actor) == 0 {
P_NewChaseDir(actor);
}
if (*info).activesound != 0 && p_random() < 3 {
S_StartSound(actor as *mut c_void, (*info).activesound);
}
}
}
// --- A_FaceTarget ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FaceTarget(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
(*actor).flags &= !(MF_AMBUSH as c_int);
(*actor).angle = R_PointToAngle2((*actor).x, (*actor).y, (*target).x, (*target).y);
if ((*target).flags as u32 & MF_SHADOW) != 0 {
let diff = (p_random() - p_random()) << 21;
(*actor).angle = (*actor).angle.wrapping_add(diff as u32);
}
}
}
// --- Атаки монстров ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_PosAttack(actor: *mut mobj_t) {
unsafe {
if (*actor).target.is_null() {
return;
}
A_FaceTarget(actor);
let mut angle = (*actor).angle;
let range = MISSILERANGE as i32;
let slope = P_AimLineAttack(actor as *mut _, angle, range);
S_StartSound(actor as *mut c_void, sfx_pistol as c_int);
angle = angle.wrapping_add(((p_random() - p_random()) << 20) as u32);
let damage = ((p_random() % 5) + 1) * 3;
P_LineAttack(actor as *mut _, angle, range, slope, damage);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SPosAttack(actor: *mut mobj_t) {
unsafe {
if (*actor).target.is_null() {
return;
}
S_StartSound(actor as *mut c_void, sfx_shotgn as c_int);
A_FaceTarget(actor);
let bangle = (*actor).angle;
let range = MISSILERANGE as i32;
let slope = P_AimLineAttack(actor as *mut _, bangle, range);
for _ in 0..3 {
let angle = bangle.wrapping_add(((p_random() - p_random()) << 20) as u32);
let damage = ((p_random() % 5) + 1) * 3;
P_LineAttack(actor as *mut _, angle, range, slope, damage);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_CPosAttack(actor: *mut mobj_t) {
unsafe {
if (*actor).target.is_null() {
return;
}
S_StartSound(actor as *mut c_void, sfx_shotgn as c_int);
A_FaceTarget(actor);
let bangle = (*actor).angle;
let range = MISSILERANGE as i32;
let slope = P_AimLineAttack(actor as *mut _, bangle, range);
let angle = bangle.wrapping_add(((p_random() - p_random()) << 20) as u32);
let damage = ((p_random() % 5) + 1) * 3;
P_LineAttack(actor as *mut _, angle, range, slope, damage);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_CPosRefire(actor: *mut mobj_t) {
unsafe {
A_FaceTarget(actor);
if p_random() < 40 {
return;
}
let target = (*actor).target;
if target.is_null()
|| (*target).health <= 0
|| P_CheckSight(actor as *mut _, target as *mut _) == 0
{
P_SetMobjState(actor as *mut _, (*(*actor).info).seestate as _);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SpidRefire(actor: *mut mobj_t) {
unsafe {
A_FaceTarget(actor);
if p_random() < 10 {
return;
}
let target = (*actor).target;
if target.is_null()
|| (*target).health <= 0
|| P_CheckSight(actor as *mut _, target as *mut _) == 0
{
P_SetMobjState(actor as *mut _, (*(*actor).info).seestate as _);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BspiAttack(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
P_SpawnMissile(actor as *mut _, target as *mut _, MT_ARACHPLAZ);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_TroopAttack(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
if P_CheckMeleeRange(actor) != 0 {
S_StartSound(actor as *mut c_void, sfx_claw as c_int);
let damage = ((p_random() % 8) + 1) * 3;
P_DamageMobj(target as *mut _, actor as *mut _, actor as *mut _, damage);
return;
}
P_SpawnMissile(actor as *mut _, target as *mut _, MT_TROOPSHOT);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SargAttack(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
if P_CheckMeleeRange(actor) != 0 {
let damage = ((p_random() % 10) + 1) * 4;
P_DamageMobj(target as *mut _, actor as *mut _, actor as *mut _, damage);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_HeadAttack(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
if P_CheckMeleeRange(actor) != 0 {
let damage = ((p_random() % 6) + 1) * 10;
P_DamageMobj(target as *mut _, actor as *mut _, actor as *mut _, damage);
return;
}
P_SpawnMissile(actor as *mut _, target as *mut _, MT_HEADSHOT);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_CyberAttack(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
P_SpawnMissile(actor as *mut _, target as *mut _, MT_ROCKET);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BruisAttack(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
if P_CheckMeleeRange(actor) != 0 {
S_StartSound(actor as *mut c_void, sfx_claw as c_int);
let damage = ((p_random() % 8) + 1) * 10;
P_DamageMobj(target as *mut _, actor as *mut _, actor as *mut _, damage);
return;
}
P_SpawnMissile(actor as *mut _, target as *mut _, MT_BRUISERSHOT);
}
}
// --- Ревенант (самонаводящиеся ракеты) ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SkelMissile(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
let frac_u = FRACUNIT as i32;
(*actor).z += 16 * frac_u;
let mo = P_SpawnMissile(actor as *mut _, target as *mut _, MT_TRACER) as *mut mobj_t;
(*actor).z -= 16 * frac_u;
(*mo).x = (*mo).x.wrapping_add((*mo).momx);
(*mo).y = (*mo).y.wrapping_add((*mo).momy);
(*mo).tracer = target;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Tracer(actor: *mut mobj_t) {
unsafe {
if (gametic & 3) != 0 {
return;
}
P_SpawnPuff((*actor).x, (*actor).y, (*actor).z);
let th = P_SpawnMobj(
(*actor).x.wrapping_sub((*actor).momx),
(*actor).y.wrapping_sub((*actor).momy),
(*actor).z,
MT_SMOKE,
) as *mut mobj_t;
let frac_u = FRACUNIT as i32;
(*th).momz = frac_u;
(*th).tics -= p_random() & 3;
if (*th).tics < 1 {
(*th).tics = 1;
}
let dest = (*actor).tracer;
if dest.is_null() || (*dest).health <= 0 {
return;
}
let exact = R_PointToAngle2((*actor).x, (*actor).y, (*dest).x, (*dest).y);
let trace_ang = TRACEANGLE as u32;
if exact != (*actor).angle {
if exact.wrapping_sub((*actor).angle) > 0x8000_0000 {
(*actor).angle = (*actor).angle.wrapping_sub(trace_ang);
if exact.wrapping_sub((*actor).angle) < 0x8000_0000 {
(*actor).angle = exact;
}
} else {
(*actor).angle = (*actor).angle.wrapping_add(trace_ang);
if exact.wrapping_sub((*actor).angle) > 0x8000_0000 {
(*actor).angle = exact;
}
}
}
let idx = ((*actor).angle >> ANGLETOFINESHIFT) as usize;
let speed = (*(*actor).info).speed;
(*actor).momx = fixed_mul(speed, finesine[idx + QUARTER_FINE]);
(*actor).momy = fixed_mul(speed, finesine[idx]);
let mut dist = P_AproxDistance(
(*dest).x.wrapping_sub((*actor).x),
(*dest).y.wrapping_sub((*actor).y),
) / speed;
if dist < 1 {
dist = 1;
}
let slope = ((*dest).z + 40 * frac_u - (*actor).z) / dist;
if slope < (*actor).momz {
(*actor).momz -= frac_u / 8;
} else {
(*actor).momz += frac_u / 8;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SkelWhoosh(actor: *mut mobj_t) {
unsafe {
if (*actor).target.is_null() {
return;
}
A_FaceTarget(actor);
S_StartSound(actor as *mut c_void, sfx_skeswg as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SkelFist(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
if P_CheckMeleeRange(actor) != 0 {
let damage = ((p_random() % 10) + 1) * 6;
S_StartSound(actor as *mut c_void, sfx_skepch as c_int);
P_DamageMobj(target as *mut _, actor as *mut _, actor as *mut _, damage);
}
}
}
// --- Архвайл (воскрешение и пламя) ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PIT_VileCheck(thing: *mut mobj_t) -> c_int {
unsafe {
if ((*thing).flags as u32 & MF_CORPSE) == 0 {
return 1;
}
if (*thing).tics != -1 {
return 1;
}
let info = (*thing).info;
if (*info).raisestate == (S_NULL as i32) {
return 1;
}
let info_base = (&raw const mobjinfo) as *const mobjinfo_t;
let maxdist = (*info).radius + (*info_base.add(MT_VILE as usize)).radius;
if ((*thing).x.wrapping_sub(viletryx)).wrapping_abs() > maxdist
|| ((*thing).y.wrapping_sub(viletryy)).wrapping_abs() > maxdist
{
return 1;
}
corpsehit = thing;
(*corpsehit).momx = 0;
(*corpsehit).momy = 0;
(*corpsehit).height <<= 2;
let check = P_CheckPosition(corpsehit as *mut _, (*corpsehit).x, (*corpsehit).y);
(*corpsehit).height >>= 2;
if check == 0 {
return 1;
}
0
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_VileChase(actor: *mut mobj_t) {
unsafe {
let dir = (*actor).movedir as u32;
if dir != DI_NODIR {
let speed = (*(*actor).info).speed;
let xs = *(&raw const xspeed as *const i32).add(dir as usize);
let ys = *(&raw const yspeed as *const i32).add(dir as usize);
viletryx = (*actor).x.wrapping_add(speed.wrapping_mul(xs));
viletryy = (*actor).y.wrapping_add(speed.wrapping_mul(ys));
let max_rad = MAXRADIUS as i32;
let shift = MAPBLOCKSHIFT as u32;
let xl = (viletryx.wrapping_sub(bmaporgx).wrapping_sub(max_rad * 2)) >> shift;
let xh = (viletryx.wrapping_sub(bmaporgx).wrapping_add(max_rad * 2)) >> shift;
let yl = (viletryy.wrapping_sub(bmaporgy).wrapping_sub(max_rad * 2)) >> shift;
let yh = (viletryy.wrapping_sub(bmaporgy).wrapping_add(max_rad * 2)) >> shift;
vileobj = actor;
let check_fn: unsafe extern "C" fn(*mut _) -> c_int =
std::mem::transmute(PIT_VileCheck as unsafe extern "C" fn(*mut mobj_t) -> c_int);
for bx in xl..=xh {
for by in yl..=yh {
if P_BlockThingsIterator(bx, by, Some(check_fn)) == 0 {
let temp = (*actor).target;
(*actor).target = corpsehit;
A_FaceTarget(actor);
(*actor).target = temp;
P_SetMobjState(actor as *mut _, S_VILE_HEAL1);
S_StartSound(corpsehit as *mut c_void, sfx_slop as c_int);
let info = (*corpsehit).info;
P_SetMobjState(corpsehit as *mut _, (*info).raisestate as _);
(*corpsehit).height <<= 2;
(*corpsehit).flags = (*info).flags;
(*corpsehit).health = (*info).spawnhealth;
(*corpsehit).target = ptr::null_mut();
return;
}
}
}
}
A_Chase(actor);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_VileStart(actor: *mut mobj_t) {
unsafe {
S_StartSound(actor as *mut c_void, sfx_vilatk as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_StartFire(actor: *mut mobj_t) {
unsafe {
S_StartSound(actor as *mut c_void, sfx_flamst as c_int);
A_Fire(actor);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireCrackle(actor: *mut mobj_t) {
unsafe {
S_StartSound(actor as *mut c_void, sfx_flame as c_int);
A_Fire(actor);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Fire(actor: *mut mobj_t) {
unsafe {
let dest = (*actor).tracer;
if dest.is_null() {
return;
}
if P_CheckSight((*actor).target as *mut _, dest as *mut _) == 0 {
return;
}
let an = ((*dest).angle >> ANGLETOFINESHIFT) as usize;
let dist = 24 * (FRACUNIT as i32);
P_UnsetThingPosition(actor as *mut _);
(*actor).x = (*dest).x.wrapping_add(fixed_mul(dist, finesine[an + QUARTER_FINE]));
(*actor).y = (*dest).y.wrapping_add(fixed_mul(dist, finesine[an]));
(*actor).z = (*dest).z;
P_SetThingPosition(actor as *mut _);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_VileTarget(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
let fog = P_SpawnMobj((*target).x, (*target).x, (*target).z, MT_FIRE) as *mut mobj_t;
(*actor).tracer = fog;
(*fog).target = actor;
(*fog).tracer = target;
A_Fire(fog);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_VileAttack(actor: *mut mobj_t) {
unsafe {
let target = (*actor).target;
if target.is_null() {
return;
}
A_FaceTarget(actor);
if P_CheckSight(actor as *mut _, target as *mut _) == 0 {
return;
}
S_StartSound(actor as *mut c_void, sfx_barexp as c_int);
P_DamageMobj(target as *mut _, actor as *mut _, actor as *mut _, 20);
let mass = (*(*target).info).mass;
if mass != 0 {
(*target).momz = (1000 * (FRACUNIT as i32)) / mass;
}
let an = ((*actor).angle >> ANGLETOFINESHIFT) as usize;
let fire = (*actor).tracer;
if fire.is_null() {
return;
}
let dist = 24 * (FRACUNIT as i32);
(*fire).x = (*target).x.wrapping_sub(fixed_mul(dist, finesine[an + QUARTER_FINE]));
(*fire).y = (*target).y.wrapping_sub(fixed_mul(dist, finesine[an]));
P_RadiusAttack(fire as *mut _, actor as *mut _, 70);
}
}
// --- Манкубус ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FatRaise(actor: *mut mobj_t) {
unsafe {
A_FaceTarget(actor);
S_StartSound(actor as *mut c_void, sfx_manatk as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FatAttack1(actor: *mut mobj_t) {
unsafe {
A_FaceTarget(actor);
(*actor).angle = (*actor).angle.wrapping_add(FATSPREAD);
let target = (*actor).target as *mut _;
P_SpawnMissile(actor as *mut _, target, MT_FATSHOT);
let mo = P_SpawnMissile(actor as *mut _, target, MT_FATSHOT) as *mut mobj_t;
(*mo).angle = (*mo).angle.wrapping_add(FATSPREAD);
let an = ((*mo).angle >> ANGLETOFINESHIFT) as usize;
let spd = (*(*mo).info).speed;
(*mo).momx = fixed_mul(spd, finesine[an + QUARTER_FINE]);
(*mo).momy = fixed_mul(spd, finesine[an]);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FatAttack2(actor: *mut mobj_t) {
unsafe {
A_FaceTarget(actor);
(*actor).angle = (*actor).angle.wrapping_sub(FATSPREAD);
let target = (*actor).target as *mut _;
P_SpawnMissile(actor as *mut _, target, MT_FATSHOT);
let mo = P_SpawnMissile(actor as *mut _, target, MT_FATSHOT) as *mut mobj_t;
(*mo).angle = (*mo).angle.wrapping_sub(FATSPREAD * 2);
let an = ((*mo).angle >> ANGLETOFINESHIFT) as usize;
let spd = (*(*mo).info).speed;
(*mo).momx = fixed_mul(spd, finesine[an + QUARTER_FINE]);
(*mo).momy = fixed_mul(spd, finesine[an]);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FatAttack3(actor: *mut mobj_t) {
unsafe {
A_FaceTarget(actor);
let target = (*actor).target as *mut _;
let mo1 = P_SpawnMissile(actor as *mut _, target, MT_FATSHOT) as *mut mobj_t;
(*mo1).angle = (*mo1).angle.wrapping_sub(FATSPREAD / 2);
let an1 = ((*mo1).angle >> ANGLETOFINESHIFT) as usize;
let spd1 = (*(*mo1).info).speed;
(*mo1).momx = fixed_mul(spd1, finesine[an1 + QUARTER_FINE]);
(*mo1).momy = fixed_mul(spd1, finesine[an1]);
let mo2 = P_SpawnMissile(actor as *mut _, target, MT_FATSHOT) as *mut mobj_t;
(*mo2).angle = (*mo2).angle.wrapping_add(FATSPREAD / 2);
let an2 = ((*mo2).angle >> ANGLETOFINESHIFT) as usize;
let spd2 = (*(*mo2).info).speed;
(*mo2).momx = fixed_mul(spd2, finesine[an2 + QUARTER_FINE]);
(*mo2).momy = fixed_mul(spd2, finesine[an2]);
}
}
// --- Потерянная душа и Элементаль боли ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SkullAttack(actor: *mut mobj_t) {
unsafe {
let dest = (*actor).target;
if dest.is_null() {
return;
}
(*actor).flags |= MF_SKULLFLY as c_int;
S_StartSound(actor as *mut c_void, (*(*actor).info).attacksound);
A_FaceTarget(actor);
let an = ((*actor).angle >> ANGLETOFINESHIFT) as usize;
(*actor).momx = fixed_mul(SKULLSPEED, finesine[an + QUARTER_FINE]);
(*actor).momy = fixed_mul(SKULLSPEED, finesine[an]);
let mut dist = P_AproxDistance(
(*dest).x.wrapping_sub((*actor).x),
(*dest).y.wrapping_sub((*actor).y),
) / SKULLSPEED;
if dist < 1 {
dist = 1;
}
(*actor).momz = ((*dest).z + ((*dest).height >> 1) - (*actor).z) / dist;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_PainShootSkull(actor: *mut mobj_t, angle: u32) {
unsafe {
let mut count = 0;
let cap = &raw mut thinkercap;
let mut th = (*cap).next;
let mobj_thinker_addr = P_MobjThinker as *const ();
while th != cap {
let is_mobj = match (*th).function.acp1 {
Some(f) => (f as *const ()) == mobj_thinker_addr,
None => false,
};
if is_mobj && (*(th as *mut mobj_t)).type_ == MT_SKULL {
count += 1;
}
th = (*th).next;
}
if count > 20 {
return;
}
let an = (angle >> ANGLETOFINESHIFT) as usize;
let info_base = (&raw const mobjinfo) as *const mobjinfo_t;
let skull_rad = (*info_base.add(MT_SKULL as usize)).radius;
let frac_u = FRACUNIT as i32;
let prestep = 4 * frac_u + 3 * ((*(*actor).info).radius + skull_rad) / 2;
let x = (*actor).x.wrapping_add(fixed_mul(prestep, finesine[an + QUARTER_FINE]));
let y = (*actor).y.wrapping_add(fixed_mul(prestep, finesine[an]));
let z = (*actor).z + 8 * frac_u;
let newmobj = P_SpawnMobj(x, y, z, MT_SKULL) as *mut mobj_t;
if P_TryMove(newmobj as *mut _, (*newmobj).x, (*newmobj).y) == 0 {
P_DamageMobj(newmobj as *mut _, actor as *mut _, actor as *mut _, 10000);
return;
}
(*newmobj).target = (*actor).target;
A_SkullAttack(newmobj);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_PainAttack(actor: *mut mobj_t) {
unsafe {
if (*actor).target.is_null() {
return;
}
A_FaceTarget(actor);
A_PainShootSkull(actor, (*actor).angle);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_PainDie(actor: *mut mobj_t) {
unsafe {
A_Fall(actor);
A_PainShootSkull(actor, (*actor).angle.wrapping_add(ANG90));
A_PainShootSkull(actor, (*actor).angle.wrapping_add(ANG180));
A_PainShootSkull(actor, (*actor).angle.wrapping_add(ANG270));
}
}
// --- Звуки смерти, боли, взрывы и триггеры боссов ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Scream(actor: *mut mobj_t) {
unsafe {
let dsound = (*(*actor).info).deathsound;
if dsound == 0 {
return;
}
let sound = match dsound as u32 {
sfx_podth1 | sfx_podth2 | sfx_podth3 => (sfx_podth1 as c_int) + (p_random() % 3),
sfx_bgdth1 | sfx_bgdth2 => (sfx_bgdth1 as c_int) + (p_random() % 2),
other => other as c_int,
};
if (*actor).type_ == MT_SPIDER || (*actor).type_ == MT_CYBORG {
S_StartSound(ptr::null_mut(), sound);
} else {
S_StartSound(actor as *mut c_void, sound);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_XScream(actor: *mut mobj_t) {
unsafe {
S_StartSound(actor as *mut c_void, sfx_slop as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Pain(actor: *mut mobj_t) {
unsafe {
let psound = (*(*actor).info).painsound;
if psound != 0 {
S_StartSound(actor as *mut c_void, psound);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Fall(actor: *mut mobj_t) {
unsafe {
(*actor).flags &= !(MF_SOLID as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Explode(thingy: *mut mobj_t) {
unsafe {
P_RadiusAttack(thingy as *mut _, (*thingy).target as *mut _, 128);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BossDeath(mo: *mut mobj_t) {
unsafe {
if get_gamemode() == commercial {
if gamemap != 7 {
return;
}
if (*mo).type_ != MT_FATSO && (*mo).type_ != MT_BABY {
return;
}
} else {
match gameepisode {
1 => {
if gamemap != 8 || (*mo).type_ != MT_BRUISER {
return;
}
}
2 => {
if gamemap != 8 || (*mo).type_ != MT_CYBORG {
return;
}
}
3 => {
if gamemap != 8 || (*mo).type_ != MT_SPIDER {
return;
}
}
4 => match gamemap {
6 => {
if (*mo).type_ != MT_CYBORG {
return;
}
}
8 => {
if (*mo).type_ != MT_SPIDER {
return;
}
}
_ => return,
},
_ => {
if gamemap != 8 {
return;
}
}
}
}
let in_game_ptr = (&raw const playeringame) as *const c_int;
let players_base = (&raw const players) as *const player_t;
let mut any_alive = false;
for i in 0..NUM_PLAYERS {
if *in_game_ptr.add(i) != 0 && (*players_base.add(i)).health > 0 {
any_alive = true;
break;
}
}
if !any_alive {
return;
}
let cap = &raw mut thinkercap;
let mut th = (*cap).next;
let mobj_thinker_addr = P_MobjThinker as *const ();
while th != cap {
let is_mobj = match (*th).function.acp1 {
Some(f) => (f as *const ()) == mobj_thinker_addr,
None => false,
};
if is_mobj {
let mo2 = th as *mut mobj_t;
if mo2 != mo && (*mo2).type_ == (*mo).type_ && (*mo2).health > 0 {
return;
}
}
th = (*th).next;
}
let mut junk: line_t = std::mem::zeroed();
if get_gamemode() == commercial {
if gamemap == 7 {
if (*mo).type_ == MT_FATSO {
junk.tag = 666;
EV_DoFloor(&raw mut junk as *mut _, lowerFloorToLowest);
return;
}
if (*mo).type_ == MT_BABY {
junk.tag = 667;
EV_DoFloor(&raw mut junk as *mut _, raiseToTexture);
return;
}
}
} else {
match gameepisode {
1 => {
junk.tag = 666;
EV_DoFloor(&raw mut junk as *mut _, lowerFloorToLowest);
return;
}
4 => match gamemap {
6 => {
junk.tag = 666;
EV_DoDoor(&raw mut junk, blazeOpen);
return;
}
8 => {
junk.tag = 666;
EV_DoFloor(&raw mut junk as *mut _, lowerFloorToLowest);
return;
}
_ => {}
},
_ => {}
}
}
G_ExitLevel();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Hoof(mo: *mut mobj_t) {
unsafe {
S_StartSound(mo as *mut c_void, sfx_hoof as c_int);
A_Chase(mo);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Metal(mo: *mut mobj_t) {
unsafe {
S_StartSound(mo as *mut c_void, sfx_metal as c_int);
A_Chase(mo);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BabyMetal(mo: *mut mobj_t) {
unsafe {
S_StartSound(mo as *mut c_void, sfx_bspwlk as c_int);
A_Chase(mo);
}
}
// --- Перезарядка двустволки ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_OpenShotgun2(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
S_StartSound((*player).mo as *mut c_void, sfx_dbopn as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_LoadShotgun2(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
S_StartSound((*player).mo as *mut c_void, sfx_dbload as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_CloseShotgun2(player: *mut player_t, psp: *mut pspdef_t) {
unsafe {
S_StartSound((*player).mo as *mut c_void, sfx_dbcls as c_int);
A_ReFire(player, psp as *mut _);
}
}
// --- Финальный босс DOOM II (Icon of Sin) ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BrainAwake(_mo: *mut mobj_t) {
unsafe {
numbraintargets = 0;
braintargeton = 0;
let cap = &raw mut thinkercap;
let mut th = (*cap).next;
let mobj_thinker_addr = P_MobjThinker as *const ();
let targets_ptr = (&raw mut braintargets) as *mut *mut mobj_t;
while th != cap {
let is_mobj = match (*th).function.acp1 {
Some(f) => (f as *const ()) == mobj_thinker_addr,
None => false,
};
if is_mobj {
let m = th as *mut mobj_t;
if (*m).type_ == MT_BOSSTARGET {
*targets_ptr.add(numbraintargets as usize) = m;
numbraintargets += 1;
}
}
th = (*th).next;
}
S_StartSound(ptr::null_mut(), sfx_bossit as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BrainPain(_mo: *mut mobj_t) {
unsafe {
S_StartSound(ptr::null_mut(), sfx_bospn as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BrainScream(mo: *mut mobj_t) {
unsafe {
let frac_u = FRACUNIT as i32;
let mut x = (*mo).x - 196 * frac_u;
let end_x = (*mo).x + 320 * frac_u;
while x < end_x {
let y = (*mo).y - 320 * frac_u;
let z = 128 + p_random() * 2 * frac_u;
let th = P_SpawnMobj(x, y, z, MT_ROCKET) as *mut mobj_t;
(*th).momz = p_random() * 512;
P_SetMobjState(th as *mut _, S_BRAINEXPLODE1);
(*th).tics -= p_random() & 7;
if (*th).tics < 1 {
(*th).tics = 1;
}
x += frac_u * 8;
}
S_StartSound(ptr::null_mut(), sfx_bosdth as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BrainExplode(mo: *mut mobj_t) {
unsafe {
let frac_u = FRACUNIT as i32;
let x = (*mo).x + (p_random() - p_random()) * 2048;
let y = (*mo).y;
let z = 128 + p_random() * 2 * frac_u;
let th = P_SpawnMobj(x, y, z, MT_ROCKET) as *mut mobj_t;
(*th).momz = p_random() * 512;
P_SetMobjState(th as *mut _, S_BRAINEXPLODE1);
(*th).tics -= p_random() & 7;
if (*th).tics < 1 {
(*th).tics = 1;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BrainDie(_mo: *mut mobj_t) {
unsafe {
G_ExitLevel();
}
}
static mut BRAIN_SPIT_EASY: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BrainSpit(mo: *mut mobj_t) {
unsafe {
BRAIN_SPIT_EASY ^= 1;
if gameskill <= sk_easy && BRAIN_SPIT_EASY == 0 {
return;
}
if numbraintargets == 0 {
return;
}
let targets_ptr = (&raw const braintargets) as *const *mut mobj_t;
let targ = *targets_ptr.add(braintargeton as usize);
braintargeton = (braintargeton + 1) % numbraintargets;
let newmobj = P_SpawnMissile(mo as *mut _, targ as *mut _, MT_SPAWNSHOT) as *mut mobj_t;
(*newmobj).target = targ;
let st_tics = (*(*newmobj).state).tics as i32;
if (*newmobj).momy != 0 && st_tics != 0 {
(*newmobj).reactiontime = (((*targ).y - (*mo).y) / (*newmobj).momy) / st_tics;
}
S_StartSound(ptr::null_mut(), sfx_bospit as c_int);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SpawnSound(mo: *mut mobj_t) {
unsafe {
S_StartSound(mo as *mut c_void, sfx_boscub as c_int);
A_SpawnFly(mo);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_SpawnFly(mo: *mut mobj_t) {
unsafe {
(*mo).reactiontime -= 1;
if (*mo).reactiontime != 0 {
return;
}
let targ = (*mo).target;
let fog = P_SpawnMobj((*targ).x, (*targ).y, (*targ).z, MT_SPAWNFIRE);
S_StartSound(fog as *mut c_void, sfx_telept as c_int);
let r = p_random();
let m_type = if r < 50 {
MT_TROOP
} else if r < 90 {
MT_SERGEANT
} else if r < 120 {
MT_SHADOWS
} else if r < 130 {
MT_PAIN
} else if r < 160 {
MT_HEAD
} else if r < 162 {
MT_VILE
} else if r < 172 {
MT_UNDEAD
} else if r < 192 {
MT_BABY
} else if r < 222 {
MT_FATSO
} else if r < 246 {
MT_KNIGHT
} else {
MT_BRUISER
};
let newmobj = P_SpawnMobj((*targ).x, (*targ).y, (*targ).z, m_type) as *mut mobj_t;
if P_LookForPlayers(newmobj, 1) != 0 {
P_SetMobjState(newmobj as *mut _, (*(*newmobj).info).seestate as _);
}
P_TeleportMove(newmobj as *mut _, (*newmobj).x, (*newmobj).y);
P_RemoveMobj(mo as *mut _);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_PlayerScream(mo: *mut mobj_t) {
unsafe {
let mut sound = sfx_pldeth as c_int;
if get_gamemode() == commercial && (*mo).health < -50 {
sound = sfx_pdiehi as c_int;
}
S_StartSound(mo as *mut c_void, sound);
}
}