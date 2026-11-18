#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_int, c_void};
use std::ptr;

use crate::info::states;
use crate::m_fixed::{FRACBITS, fixed_mul};
use crate::m_random::p_random;
use crate::p_mobj::{P_SetMobjState, P_SpawnMobj, P_SpawnPlayerMissile};
use crate::p_tick::leveltime;
use crate::r_main::R_PointToAngle2;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, FINEMASK, finesine};

const LOWERSPEED: i32 = (FRACUNIT as i32) * 6;
const RAISESPEED: i32 = (FRACUNIT as i32) * 6;
const WEAPONBOTTOM: i32 = 128 * (FRACUNIT as i32);
const WEAPONTOP: i32 = 32 * (FRACUNIT as i32);
const BFGCELLS: c_int = 40;
const QUARTER_FINE: usize = FINEANGLES / 4;

unsafe extern "C" {
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn P_NoiseAlert(target: *mut mobj_t, emmiter: *mut mobj_t);
    fn P_AimLineAttack(t1: *mut mobj_t, angle: u32, distance: i32) -> i32;
    fn P_LineAttack(t1: *mut mobj_t, angle: u32, distance: i32, slope: i32, damage: c_int);
    fn P_DamageMobj(target: *mut mobj_t, inflictor: *mut mobj_t, source: *mut mobj_t, damage: c_int);

    static mut weaponinfo: [weaponinfo_t; NUMWEAPONS as usize];
    static mut gamemode: GameMode_t;
    static mut linetarget: *mut mobj_t;
}

// --- Глобальные переменные p_pspr.c ---
#[unsafe(no_mangle)]
pub static mut swingx: i32 = 0;

#[unsafe(no_mangle)]
pub static mut swingy: i32 = 0;

#[unsafe(no_mangle)]
pub static mut bulletslope: i32 = 0;

// --- P_SetPsprite ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetPsprite(
    player: *mut player_t,
    position: c_int,
    mut stnum: statenum_t,
) {
    unsafe {
        let pspr_base = (&raw mut (*player).psprites) as *mut pspdef_t;
        let psp = pspr_base.add(position as usize);
        let states_base = (&raw mut states) as *mut state_t;

        loop {
            if stnum == S_NULL {
                (*psp).state = ptr::null_mut();
                break;
            }

            let state = states_base.add(stnum as usize);
            (*psp).state = state;
            (*psp).tics = (*state).tics as c_int;

            if (*state).misc1 != 0 {
                (*psp).sx = ((*state).misc1 as i32) << FRACBITS;
                (*psp).sy = ((*state).misc2 as i32) << FRACBITS;
            }

            if let Some(action_fn) = (*state).action.acp2 {
                action_fn(player as *mut c_void, psp as *mut c_void);
                if (*psp).state.is_null() {
                    break;
                }
            }

            stnum = (*(*psp).state).nextstate;
            if (*psp).tics != 0 {
                break;
            }
        }
    }
}

// --- P_CalcSwing ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CalcSwing(player: *mut player_t) {
    unsafe {
        let swing = (*player).bob;

        let mut angle = (((FINEANGLES as i32) / 70 * leveltime) as usize) & FINEMASK;
        swingx = fixed_mul(swing, finesine[angle]);

        angle = (((FINEANGLES as i32) / 70 * leveltime + (FINEANGLES as i32) / 2) as usize) & FINEMASK;
        swingy = -fixed_mul(swingx, finesine[angle]);
    }
}

// --- P_BringUpWeapon ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BringUpWeapon(player: *mut player_t) {
    unsafe {
        if (*player).pendingweapon == (wp_nochange as c_int) {
            (*player).pendingweapon = (*player).readyweapon;
        }

        if (*player).pendingweapon == (wp_chainsaw as c_int) {
            S_StartSound((*player).mo as *mut c_void, sfx_sawup as c_int);
        }

        let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
        let newstate = (*winfo_ptr.add((*player).pendingweapon as usize)).upstate as statenum_t;

        (*player).pendingweapon = wp_nochange as c_int;
        let pspr_base = (&raw mut (*player).psprites) as *mut pspdef_t;
        (*pspr_base.add(ps_weapon as usize)).sy = WEAPONBOTTOM;

        P_SetPsprite(player, ps_weapon as c_int, newstate);
    }
}

// --- P_CheckAmmo ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckAmmo(player: *mut player_t) -> c_int {
    unsafe {
        let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
        let ready = (*player).readyweapon as usize;
        let ammo = (*winfo_ptr.add(ready)).ammo;

        let count = if ready == (wp_bfg as usize) {
            BFGCELLS
        } else if ready == (wp_supershotgun as usize) {
            2
        } else {
            1
        };

        if ammo == am_noammo || (*player).ammo[ammo as usize] >= count {
            return 1;
        }

        loop {
            if (*player).weaponowned[wp_plasma as usize] != 0
                && (*player).ammo[am_cell as usize] != 0
                && gamemode != shareware
            {
                (*player).pendingweapon = wp_plasma as c_int;
            } else if (*player).weaponowned[wp_supershotgun as usize] != 0
                && (*player).ammo[am_shell as usize] > 2
                && gamemode == commercial
            {
                (*player).pendingweapon = wp_supershotgun as c_int;
            } else if (*player).weaponowned[wp_chaingun as usize] != 0
                && (*player).ammo[am_clip as usize] != 0
            {
                (*player).pendingweapon = wp_chaingun as c_int;
            } else if (*player).weaponowned[wp_shotgun as usize] != 0
                && (*player).ammo[am_shell as usize] != 0
            {
                (*player).pendingweapon = wp_shotgun as c_int;
            } else if (*player).ammo[am_clip as usize] != 0 {
                (*player).pendingweapon = wp_pistol as c_int;
            } else if (*player).weaponowned[wp_chainsaw as usize] != 0 {
                (*player).pendingweapon = wp_chainsaw as c_int;
            } else if (*player).weaponowned[wp_missile as usize] != 0
                && (*player).ammo[am_misl as usize] != 0
            {
                (*player).pendingweapon = wp_missile as c_int;
            } else if (*player).weaponowned[wp_bfg as usize] != 0
                && (*player).ammo[am_cell as usize] > 40
                && gamemode != shareware
            {
                (*player).pendingweapon = wp_bfg as c_int;
            } else {
                (*player).pendingweapon = wp_fist as c_int;
            }

            if (*player).pendingweapon != (wp_nochange as c_int) {
                break;
            }
        }

        let down_st = (*winfo_ptr.add(ready)).downstate as statenum_t;
        P_SetPsprite(player, ps_weapon as c_int, down_st);

        0
    }
}

// --- P_FireWeapon ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FireWeapon(player: *mut player_t) {
    unsafe {
        if P_CheckAmmo(player) == 0 {
            return;
        }

        let mo = (*player).mo as *mut mobj_t;
        P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY_ATK1);

        let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
        let newstate = (*winfo_ptr.add((*player).readyweapon as usize)).atkstate as statenum_t;
        P_SetPsprite(player, ps_weapon as c_int, newstate);
        P_NoiseAlert(mo, mo);
    }
}

// --- P_DropWeapon ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DropWeapon(player: *mut player_t) {
    unsafe {
        let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
        let down_st = (*winfo_ptr.add((*player).readyweapon as usize)).downstate as statenum_t;
        P_SetPsprite(player, ps_weapon as c_int, down_st);
    }
}

// --- A_WeaponReady ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_WeaponReady(player: *mut player_t, psp: *mut pspdef_t) {
    unsafe {
        let mo = (*player).mo as *mut mobj_t;
        let states_base = (&raw const states) as *const state_t;
        let atk1 = states_base.add(S_PLAY_ATK1 as usize) as *mut _;
        let atk2 = states_base.add(S_PLAY_ATK2 as usize) as *mut _;

        if (*mo).state == atk1 || (*mo).state == atk2 {
            P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY);
        }

        let saw_st = states_base.add(S_SAW as usize) as *mut _;
        if (*player).readyweapon == (wp_chainsaw as c_int) && (*psp).state == saw_st {
            S_StartSound(mo as *mut c_void, sfx_sawidl as c_int);
        }

        if (*player).pendingweapon != (wp_nochange as c_int) || (*player).health == 0 {
            let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
            let newstate = (*winfo_ptr.add((*player).readyweapon as usize)).downstate as statenum_t;
            P_SetPsprite(player, ps_weapon as c_int, newstate);
            return;
        }

        let cmd = (&raw const (*player).cmd) as *const ticcmd_t;
        if ((*cmd).buttons as u32 & BT_ATTACK) != 0 {
            if (*player).attackdown == 0
                || ((*player).readyweapon != (wp_missile as c_int)
                    && (*player).readyweapon != (wp_bfg as c_int))
            {
                (*player).attackdown = 1;
                P_FireWeapon(player);
                return;
            }
        } else {
            (*player).attackdown = 0;
        }

        let mut angle = ((128 * leveltime) as usize) & FINEMASK;
        let frac_u = FRACUNIT as i32;
        (*psp).sx = frac_u + fixed_mul((*player).bob, finesine[angle + QUARTER_FINE]);
        angle &= (FINEANGLES / 2) - 1;
        (*psp).sy = WEAPONTOP + fixed_mul((*player).bob, finesine[angle]);
    }
}

// --- A_ReFire ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_ReFire(player: *mut player_t, _psp: *mut pspdef_t) {
    unsafe {
        let cmd = (&raw const (*player).cmd) as *const ticcmd_t;
        if ((*cmd).buttons as u32 & BT_ATTACK) != 0
            && (*player).pendingweapon == (wp_nochange as c_int)
            && (*player).health != 0
        {
            (*player).refire += 1;
P_FireWeapon(player);
} else {
(*player).refire = 0;
P_CheckAmmo(player);
}
}
}
// --- A_CheckReload ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_CheckReload(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
P_CheckAmmo(player);
}
}
// --- A_Lower ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Lower(player: *mut player_t, psp: *mut pspdef_t) {
unsafe {
(*psp).sy += LOWERSPEED;
if (*psp).sy < WEAPONBOTTOM {
return;
}
if (*player).playerstate == (PST_DEAD as c_int) {
(*psp).sy = WEAPONBOTTOM;
return;
}
if (*player).health == 0 {
P_SetPsprite(player, ps_weapon as c_int, S_NULL);
return;
}
(*player).readyweapon = (*player).pendingweapon;
P_BringUpWeapon(player);
}
}
// --- A_Raise ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Raise(player: *mut player_t, psp: *mut pspdef_t) {
unsafe {
(*psp).sy -= RAISESPEED;
if (*psp).sy > WEAPONTOP {
return;
}
(*psp).sy = WEAPONTOP;
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let newstate = (*winfo_ptr.add((*player).readyweapon as usize)).readystate as statenum_t;
P_SetPsprite(player, ps_weapon as c_int, newstate);
}
}
// --- A_GunFlash ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_GunFlash(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let mo = (*player).mo;
P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY_ATK2);
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let flash_st = (*winfo_ptr.add((*player).readyweapon as usize)).flashstate as statenum_t;
P_SetPsprite(player, ps_flash as c_int, flash_st);
}
}
// --- A_Punch ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Punch(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let mut damage = ((p_random() % 10) + 1) << 1;
if (*player).powers[pw_strength as usize] != 0 {
damage *= 10;
}
let mo = (*player).mo as *mut mobj_t;
let r_diff = (p_random() - p_random()) << 18;
let angle = (*mo).angle.wrapping_add(r_diff as u32);
let melee = MELEERANGE as i32;
let slope = P_AimLineAttack(mo, angle, melee);
P_LineAttack(mo, angle, melee, slope, damage);
if !linetarget.is_null() {
S_StartSound(mo as *mut c_void, sfx_punch as c_int);
(*mo).angle = R_PointToAngle2((*mo).x, (*mo).y, (*linetarget).x, (*linetarget).y);
}
}
}
// --- A_Saw ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Saw(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let damage = 2 * ((p_random() % 10) + 1);
let mo = (*player).mo as *mut mobj_t;
let r_diff = (p_random() - p_random()) << 18;
let mut angle = (*mo).angle.wrapping_add(r_diff as u32);
let melee = (MELEERANGE as i32) + 1;
let slope = P_AimLineAttack(mo, angle, melee);
P_LineAttack(mo, angle, melee, slope, damage);
if linetarget.is_null() {
S_StartSound(mo as *mut c_void, sfx_sawful as c_int);
return;
}
S_StartSound(mo as *mut c_void, sfx_sawhit as c_int);
angle = R_PointToAngle2((*mo).x, (*mo).y, (*linetarget).x, (*linetarget).y);
let diff = angle.wrapping_sub((*mo).angle);
if diff > ANG180 {
if diff < 0u32.wrapping_sub(ANG90 / 20) {
(*mo).angle = angle.wrapping_add(ANG90 / 21);
} else {
(*mo).angle = (*mo).angle.wrapping_sub(ANG90 / 20);
}
} else if diff > ANG90 / 20 {
(*mo).angle = angle.wrapping_sub(ANG90 / 21);
} else {
(*mo).angle = (*mo).angle.wrapping_add(ANG90 / 20);
}
(*mo).flags |= MF_JUSTATTACKED as c_int;
}
}
// --- A_FireMissile ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireMissile(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let ammo_idx = (*winfo_ptr.add((*player).readyweapon as usize)).ammo as usize;
(*player).ammo[ammo_idx] -= 1;
P_SpawnPlayerMissile((*player).mo as *mut crate::p_mobj::mobj_t, MT_ROCKET);
}
}
// --- A_FireBFG ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireBFG(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let ammo_idx = (*winfo_ptr.add((*player).readyweapon as usize)).ammo as usize;
(*player).ammo[ammo_idx] -= BFGCELLS;
P_SpawnPlayerMissile((*player).mo as *mut crate::p_mobj::mobj_t, MT_BFG);
}
}
// --- A_FirePlasma ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FirePlasma(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let w = &*winfo_ptr.add((*player).readyweapon as usize);
(*player).ammo[w.ammo as usize] -= 1;
let flash_st = ((w.flashstate as c_int) + (p_random() & 1)) as statenum_t;
P_SetPsprite(player, ps_flash as c_int, flash_st);
P_SpawnPlayerMissile((*player).mo as *mut crate::p_mobj::mobj_t, MT_PLASMA);
}
}
// --- P_BulletSlope ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BulletSlope(mo: *mut mobj_t) {
unsafe {
let aim_dist = 16 * 64 * (FRACUNIT as i32);
let mut an = (*mo).angle;
bulletslope = P_AimLineAttack(mo, an, aim_dist);
if linetarget.is_null() {
an = an.wrapping_add(1 << 26);
bulletslope = P_AimLineAttack(mo, an, aim_dist);
if linetarget.is_null() {
an = an.wrapping_sub(2 << 26);
bulletslope = P_AimLineAttack(mo, an, aim_dist);
}
}
}
}
// --- P_GunShot ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GunShot(mo: *mut mobj_t, accurate: c_int) {
unsafe {
let damage = 5 * ((p_random() % 3) + 1);
let mut angle = (*mo).angle;
if accurate == 0 {
let r_diff = (p_random() - p_random()) << 18;
angle = angle.wrapping_add(r_diff as u32);
}
P_LineAttack(mo, angle, MISSILERANGE as i32, bulletslope, damage);
}
}
// --- A_FirePistol ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FirePistol(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let mo = (*player).mo as *mut mobj_t;
S_StartSound(mo as *mut c_void, sfx_pistol as c_int);
P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY_ATK2);
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let w = &*winfo_ptr.add((*player).readyweapon as usize);
(*player).ammo[w.ammo as usize] -= 1;
P_SetPsprite(player, ps_flash as c_int, w.flashstate as statenum_t);
P_BulletSlope(mo);
P_GunShot(mo, ((*player).refire == 0) as c_int);
}
}
// --- A_FireShotgun ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireShotgun(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let mo = (*player).mo as *mut mobj_t;
S_StartSound(mo as *mut c_void, sfx_shotgn as c_int);
P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY_ATK2);
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let w = &*winfo_ptr.add((*player).readyweapon as usize);
(*player).ammo[w.ammo as usize] -= 1;
P_SetPsprite(player, ps_flash as c_int, w.flashstate as statenum_t);
P_BulletSlope(mo);
for _ in 0..7 {
P_GunShot(mo, 0);
}
}
}
// --- A_FireShotgun2 ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireShotgun2(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
let mo = (*player).mo as *mut mobj_t;
S_StartSound(mo as *mut c_void, sfx_dshtgn as c_int);
P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY_ATK2);
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let w = &*winfo_ptr.add((*player).readyweapon as usize);
(*player).ammo[w.ammo as usize] -= 2;
P_SetPsprite(player, ps_flash as c_int, w.flashstate as statenum_t);
P_BulletSlope(mo);
let range = MISSILERANGE as i32;
for _ in 0..20 {
let damage = 5 * ((p_random() % 3) + 1);
let r_ang = (p_random() - p_random()) << 19;
let angle = (*mo).angle.wrapping_add(r_ang as u32);
let r_slope = (p_random() - p_random()) << 5;
P_LineAttack(mo, angle, range, bulletslope + r_slope, damage);
}
}
}
// --- A_FireCGun ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireCGun(player: *mut player_t, psp: *mut pspdef_t) {
unsafe {
let mo = (*player).mo as *mut mobj_t;
S_StartSound(mo as *mut c_void, sfx_pistol as c_int);
let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
let w = &*winfo_ptr.add((*player).readyweapon as usize);
if (*player).ammo[w.ammo as usize] == 0 {
return;
}
P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY_ATK2);
(*player).ammo[w.ammo as usize] -= 1;
let states_base = (&raw const states) as *const state_t;
let chain1_ptr = states_base.add(S_CHAIN1 as usize);
let state_offset = ((*psp).state as *const state_t).offset_from(chain1_ptr) as c_int;
let flash_st = ((w.flashstate as c_int) + state_offset) as statenum_t;
P_SetPsprite(player, ps_flash as c_int, flash_st);
P_BulletSlope(mo);
P_GunShot(mo, ((*player).refire == 0) as c_int);
}
}
// --- A_Light0 / A_Light1 / A_Light2 ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Light0(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
(*player).extralight = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Light1(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
(*player).extralight = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Light2(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
(*player).extralight = 2;
}
}
// --- A_BFGSpray ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BFGSpray(mo: *mut mobj_t) {
unsafe {
let aim_dist = 16 * 64 * (FRACUNIT as i32);
let base_an = (*mo).angle.wrapping_sub(ANG90 / 2);
let step_an = ANG90 / 40;
for i in 0..40u32 {
let an = base_an.wrapping_add(step_an.wrapping_mul(i));
P_AimLineAttack((*mo).target, an, aim_dist);
if linetarget.is_null() {
continue;
}
let target = linetarget;
P_SpawnMobj(
(*target).x,
(*target).y,
(*target).z + ((*target).height >> 2),
MT_EXTRABFG,
);
let mut damage = 0;
for _ in 0..15 {
damage += (p_random() & 7) + 1;
}
P_DamageMobj(target, (*mo).target, (*mo).target, damage);
}
}
}
// --- A_BFGsound ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BFGsound(player: *mut player_t, _psp: *mut pspdef_t) {
unsafe {
S_StartSound((*player).mo as *mut c_void, sfx_bfg as c_int);
}
}
// --- P_SetupPsprites ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetupPsprites(player: *mut player_t) {
unsafe {
let pspr_base = (&raw mut (*player).psprites) as *mut pspdef_t;
for i in 0..(NUMPSPRITES as usize) {
(*pspr_base.add(i)).state = ptr::null_mut();
}
(*player).pendingweapon = (*player).readyweapon;
P_BringUpWeapon(player);
}
}
// --- P_MovePsprites ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MovePsprites(player: *mut player_t) {
unsafe {
let pspr_base = (&raw mut (*player).psprites) as *mut pspdef_t;
for i in 0..(NUMPSPRITES as usize) {
let psp = pspr_base.add(i);
let state = (*psp).state;
if !state.is_null() && (*psp).tics != -1 {
(*psp).tics -= 1;
if (*psp).tics == 0 {
P_SetPsprite(player, i as c_int, (*state).nextstate);
}
}
}
let weapon_psp = pspr_base.add(ps_weapon as usize);
let flash_psp = pspr_base.add(ps_flash as usize);
(*flash_psp).sx = (*weapon_psp).sx;
(*flash_psp).sy = (*weapon_psp).sy;
}
}