#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

use crate::d_items::weaponinfo;

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_int, c_void};
use std::ptr;

use crate::info::states;
use crate::m_fixed::{FRACBITS, fixed_mul};
use crate::m_random::p_random;
use crate::p_mobj::{P_SpawnMobj, P_SpawnPlayerMissile, p_set_mobj_state};
use crate::p_tick::get_leveltime;
use crate::r_main::R_PointToAngle2;
use crate::s_sound::S_StartSound;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, FINEMASK, finesine};

const LOWERSPEED: i32 = (FRACUNIT as i32) * 6;
const RAISESPEED: i32 = (FRACUNIT as i32) * 6;
const WEAPONBOTTOM: i32 = 128 * (FRACUNIT as i32);
const WEAPONTOP: i32 = 32 * (FRACUNIT as i32);
const BFGCELLS: c_int = 40;
const QUARTER_FINE: usize = FINEANGLES / 4;

unsafe extern "C" {
    fn P_NoiseAlert(target: *mut mobj_t, emmiter: *mut mobj_t);
    fn P_AimLineAttack(t1: *mut mobj_t, angle: u32, distance: i32) -> i32;
    fn P_LineAttack(t1: *mut mobj_t, angle: u32, distance: i32, slope: i32, damage: c_int);
    fn P_DamageMobj(target: *mut mobj_t, inflictor: *mut mobj_t, source: *mut mobj_t, damage: c_int);

    static mut linetarget: *mut mobj_t;
}

#[unsafe(no_mangle)]
pub static mut swingx: i32 = 0;

#[unsafe(no_mangle)]
pub static mut swingy: i32 = 0;

#[unsafe(no_mangle)]
pub static mut bulletslope: i32 = 0;

pub fn p_set_psprite(player: &mut player_t, position: usize, mut stnum: statenum_t) {
    let states_arr = unsafe { (&raw mut states).as_mut().unwrap() };
    let player_ptr = player as *mut player_t;

    loop {
        if stnum == S_NULL {
            player.psprites[position].state = ptr::null_mut();
            break;
        }

        let state = &mut states_arr[stnum as usize];
        player.psprites[position].state = state;
        player.psprites[position].tics = state.tics as c_int;

        if state.misc1 != 0 {
            player.psprites[position].sx = (state.misc1 as i32) << FRACBITS;
            player.psprites[position].sy = (state.misc2 as i32) << FRACBITS;
        }

        if let Some(action_fn) = unsafe { state.action.acp2 } {
            let psp_ptr = &mut player.psprites[position] as *mut pspdef_t;
            unsafe {
                action_fn(player_ptr as *mut c_void, psp_ptr as *mut c_void);
            }
            if player.psprites[position].state.is_null() {
                break;
            }
        }

        stnum = unsafe { (*player.psprites[position].state).nextstate };
        if player.psprites[position].tics != 0 {
            break;
        }
    }
}

pub fn p_calc_swing(player: &player_t) {
    let swing = player.bob;
    let tics = get_leveltime();

    let angle_x = (((FINEANGLES as i32) / 70 * tics) as usize) & FINEMASK;
    let sx = fixed_mul(swing, finesine[angle_x]);

    let angle_y =
        (((FINEANGLES as i32) / 70 * tics + (FINEANGLES as i32) / 2) as usize) & FINEMASK;
    let sy = -fixed_mul(sx, finesine[angle_y]);

    unsafe {
        swingx = sx;
        swingy = sy;
    }
}

pub fn p_bring_up_weapon(player: &mut player_t) {
    if player.pendingweapon == (wp_nochange as c_int) {
        player.pendingweapon = player.readyweapon;
    }

    if player.pendingweapon == (wp_chainsaw as c_int) {
        unsafe {
            S_StartSound(player.mo as *mut c_void, sfx_sawup as c_int);
        }
    }

    let newstate = weaponinfo[player.pendingweapon as usize].upstate as statenum_t;
    player.pendingweapon = wp_nochange as c_int;
    player.psprites[ps_weapon as usize].sy = WEAPONBOTTOM;

    p_set_psprite(player, ps_weapon as usize, newstate);
}

pub fn p_check_ammo(player: &mut player_t) -> bool {
    let ready = player.readyweapon as usize;
    let ammo = weaponinfo[ready].ammo;

    let count = if ready == (wp_bfg as usize) {
        BFGCELLS
    } else if ready == (wp_supershotgun as usize) {
        2
    } else {
        1
    };

    if ammo == am_noammo || player.ammo[ammo as usize] >= count {
        return true;
    }

    loop {
        if player.weaponowned[wp_plasma as usize] != 0
            && player.ammo[am_cell as usize] != 0
            && get_gamemode() != shareware
        {
            player.pendingweapon = wp_plasma as c_int;
        } else if player.weaponowned[wp_supershotgun as usize] != 0
            && player.ammo[am_shell as usize] > 2
            && get_gamemode() == commercial
        {
            player.pendingweapon = wp_supershotgun as c_int;
        } else if player.weaponowned[wp_chaingun as usize] != 0
            && player.ammo[am_clip as usize] != 0
        {
            player.pendingweapon = wp_chaingun as c_int;
        } else if player.weaponowned[wp_shotgun as usize] != 0
            && player.ammo[am_shell as usize] != 0
        {
            player.pendingweapon = wp_shotgun as c_int;
        } else if player.ammo[am_clip as usize] != 0 {
            player.pendingweapon = wp_pistol as c_int;
        } else if player.weaponowned[wp_chainsaw as usize] != 0 {
            player.pendingweapon = wp_chainsaw as c_int;
        } else if player.weaponowned[wp_missile as usize] != 0
            && player.ammo[am_misl as usize] != 0
        {
            player.pendingweapon = wp_missile as c_int;
        } else if player.weaponowned[wp_bfg as usize] != 0
            && player.ammo[am_cell as usize] > 40
            && get_gamemode() != shareware
        {
            player.pendingweapon = wp_bfg as c_int;
        } else {
            player.pendingweapon = wp_fist as c_int;
        }

        if player.pendingweapon != (wp_nochange as c_int) {
            break;
        }
    }

    let down_st = weaponinfo[ready].downstate as statenum_t;
    p_set_psprite(player, ps_weapon as usize, down_st);
    false
}

pub fn p_fire_weapon(player: &mut player_t) {
    if !p_check_ammo(player) {
        return;
    }

    if let Some(mo) = unsafe { (player.mo as *mut crate::p_mobj::mobj_t).as_mut() } {
        p_set_mobj_state(mo, S_PLAY_ATK1);
    }

    let newstate = weaponinfo[player.readyweapon as usize].atkstate as statenum_t;
    p_set_psprite(player, ps_weapon as usize, newstate);

    let mo_ptr = player.mo as *mut mobj_t;
    unsafe {
        P_NoiseAlert(mo_ptr, mo_ptr);
    }
}

pub fn p_drop_weapon(player: &mut player_t) {
    let down_st = weaponinfo[player.readyweapon as usize].downstate as statenum_t;
    p_set_psprite(player, ps_weapon as usize, down_st);
}

pub fn p_bullet_slope(mo: &mut mobj_t) {
    let aim_dist = 16 * 64 * (FRACUNIT as i32);
    let mo_ptr = mo as *mut mobj_t;
    let mut an = mo.angle;

    unsafe {
        bulletslope = P_AimLineAttack(mo_ptr, an, aim_dist);
        if linetarget.is_null() {
            an = an.wrapping_add(1 << 26);
            bulletslope = P_AimLineAttack(mo_ptr, an, aim_dist);
            if linetarget.is_null() {
                an = an.wrapping_sub(2 << 26);
                bulletslope = P_AimLineAttack(mo_ptr, an, aim_dist);
            }
        }
    }
}

pub fn p_gun_shot(mo: &mut mobj_t, accurate: bool) {
    let damage = 5 * ((p_random() % 3) + 1);
    let mut angle = mo.angle;
    if !accurate {
        let r_diff = (p_random() - p_random()) << 18;
        angle = angle.wrapping_add(r_diff as u32);
    }
    unsafe {
        P_LineAttack(mo as *mut mobj_t, angle, MISSILERANGE as i32, bulletslope, damage);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetPsprite(player: *mut player_t, position: c_int, stnum: statenum_t) {
    if let Some(p) = unsafe { player.as_mut() } {
        p_set_psprite(p, position as usize, stnum);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CalcSwing(player: *mut player_t) {
    if let Some(p) = unsafe { player.as_ref() } {
        p_calc_swing(p);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BringUpWeapon(player: *mut player_t) {
    if let Some(p) = unsafe { player.as_mut() } {
        p_bring_up_weapon(p);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CheckAmmo(player: *mut player_t) -> c_int {
    match unsafe { player.as_mut() } {
        Some(p) => p_check_ammo(p) as c_int,
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_FireWeapon(player: *mut player_t) {
    if let Some(p) = unsafe { player.as_mut() } {
        p_fire_weapon(p);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DropWeapon(player: *mut player_t) {
    if let Some(p) = unsafe { player.as_mut() } {
        p_drop_weapon(p);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_WeaponReady(player: *mut player_t, psp: *mut pspdef_t) {
    let (Some(p), Some(ps)) = (unsafe { player.as_mut() }, unsafe { psp.as_mut() }) else {
        return;
    };
    unsafe {
        let mo = &mut *(p.mo as *mut crate::p_mobj::mobj_t);
        let states_base = (&raw const states) as *const state_t;
        let atk1 = states_base.add(S_PLAY_ATK1 as usize) as *mut _;
        let atk2 = states_base.add(S_PLAY_ATK2 as usize) as *mut _;

        if mo.state == atk1 || mo.state == atk2 {
            p_set_mobj_state(mo, S_PLAY);
        }

        let saw_st = states_base.add(S_SAW as usize) as *mut _;
        if p.readyweapon == (wp_chainsaw as c_int) && ps.state == saw_st {
            S_StartSound(p.mo as *mut c_void, sfx_sawidl as c_int);
        }

        if p.pendingweapon != (wp_nochange as c_int) || p.health == 0 {
            let newstate = weaponinfo[p.readyweapon as usize].downstate as statenum_t;
            p_set_psprite(p, ps_weapon as usize, newstate);
            return;
        }

        if (p.cmd[7] & (BT_ATTACK as u8)) != 0 {
            if p.attackdown == 0
                || (p.readyweapon != (wp_missile as c_int) && p.readyweapon != (wp_bfg as c_int))
            {
                p.attackdown = 1;
                p_fire_weapon(p);
                return;
            }
} else {
p.attackdown = 0;
}
let mut angle = ((128 * get_leveltime()) as usize) & FINEMASK;
let frac_u = FRACUNIT as i32;
ps.sx = frac_u + fixed_mul(p.bob, finesine[angle + QUARTER_FINE]);
angle &= (FINEANGLES / 2) - 1;
ps.sy = WEAPONTOP + fixed_mul(p.bob, finesine[angle]);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_ReFire(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
if (p.cmd[7] & (BT_ATTACK as u8)) != 0
&& p.pendingweapon == (wp_nochange as c_int)
&& p.health != 0
{
p.refire += 1;
p_fire_weapon(p);
} else {
p.refire = 0;
p_check_ammo(p);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_CheckReload(player: *mut player_t, _psp: *mut pspdef_t) {
if let Some(p) = unsafe { player.as_mut() } {
p_check_ammo(p);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Lower(player: *mut player_t, psp: *mut pspdef_t) {
let (Some(p), Some(ps)) = (unsafe { player.as_mut() }, unsafe { psp.as_mut() }) else {
return;
};
ps.sy += LOWERSPEED;
if ps.sy < WEAPONBOTTOM {
return;
}
if p.playerstate == (PST_DEAD as c_int) {
ps.sy = WEAPONBOTTOM;
return;
}
if p.health == 0 {
p_set_psprite(p, ps_weapon as usize, S_NULL);
return;
}
p.readyweapon = p.pendingweapon;
p_bring_up_weapon(p);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Raise(player: *mut player_t, psp: *mut pspdef_t) {
let (Some(p), Some(ps)) = (unsafe { player.as_mut() }, unsafe { psp.as_mut() }) else {
return;
};
ps.sy -= RAISESPEED;
if ps.sy > WEAPONTOP {
return;
}
ps.sy = WEAPONTOP;
let newstate = weaponinfo[p.readyweapon as usize].readystate as statenum_t;
p_set_psprite(p, ps_weapon as usize, newstate);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_GunFlash(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
if let Some(mo) = unsafe { (p.mo as *mut crate::p_mobj::mobj_t).as_mut() } {
p_set_mobj_state(mo, S_PLAY_ATK2);
}
let flash_st = weaponinfo[p.readyweapon as usize].flashstate as statenum_t;
p_set_psprite(p, ps_flash as usize, flash_st);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Punch(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let mut damage = ((p_random() % 10) + 1) << 1;
if p.powers[pw_strength as usize] != 0 {
damage *= 10;
}
let mo_ptr = p.mo as *mut mobj_t;
let Some(mo) = (unsafe { mo_ptr.as_mut() }) else { return };
let r_diff = (p_random() - p_random()) << 18;
let angle = mo.angle.wrapping_add(r_diff as u32);
let melee = MELEERANGE as i32;
unsafe {
let slope = P_AimLineAttack(mo_ptr, angle, melee);
P_LineAttack(mo_ptr, angle, melee, slope, damage);
if let Some(lt) = linetarget.as_ref() {
S_StartSound(mo_ptr as *mut c_void, sfx_punch as c_int);
mo.angle = R_PointToAngle2(mo.x, mo.y, lt.x, lt.y);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Saw(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let damage = 2 * ((p_random() % 10) + 1);
let mo_ptr = p.mo as *mut mobj_t;
let Some(mo) = (unsafe { mo_ptr.as_mut() }) else { return };
let r_diff = (p_random() - p_random()) << 18;
let mut angle = mo.angle.wrapping_add(r_diff as u32);
let melee = (MELEERANGE as i32) + 1;
unsafe {
let slope = P_AimLineAttack(mo_ptr, angle, melee);
P_LineAttack(mo_ptr, angle, melee, slope, damage);
let Some(lt) = linetarget.as_ref() else {
S_StartSound(mo_ptr as *mut c_void, sfx_sawful as c_int);
return;
};
S_StartSound(mo_ptr as *mut c_void, sfx_sawhit as c_int);
angle = R_PointToAngle2(mo.x, mo.y, lt.x, lt.y);
let diff = angle.wrapping_sub(mo.angle);
if diff > ANG180 {
if diff < 0u32.wrapping_sub(ANG90 / 20) {
mo.angle = angle.wrapping_add(ANG90 / 21);
} else {
mo.angle = mo.angle.wrapping_sub(ANG90 / 20);
}
} else if diff > ANG90 / 20 {
mo.angle = angle.wrapping_sub(ANG90 / 21);
} else {
mo.angle = mo.angle.wrapping_add(ANG90 / 20);
}
mo.flags |= MF_JUSTATTACKED as c_int;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireMissile(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let ammo_idx = weaponinfo[p.readyweapon as usize].ammo as usize;
p.ammo[ammo_idx] -= 1;
unsafe {
P_SpawnPlayerMissile(p.mo as *mut crate::p_mobj::mobj_t, MT_ROCKET);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireBFG(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let ammo_idx = weaponinfo[p.readyweapon as usize].ammo as usize;
p.ammo[ammo_idx] -= BFGCELLS;
unsafe {
P_SpawnPlayerMissile(p.mo as *mut crate::p_mobj::mobj_t, MT_BFG);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FirePlasma(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let w = &weaponinfo[p.readyweapon as usize];
p.ammo[w.ammo as usize] -= 1;
let flash_st = ((w.flashstate as c_int) + (p_random() & 1)) as statenum_t;
p_set_psprite(p, ps_flash as usize, flash_st);
unsafe {
P_SpawnPlayerMissile(p.mo as *mut crate::p_mobj::mobj_t, MT_PLASMA);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_BulletSlope(mo: *mut mobj_t) {
if let Some(m) = unsafe { mo.as_mut() } {
p_bullet_slope(m);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GunShot(mo: *mut mobj_t, accurate: c_int) {
if let Some(m) = unsafe { mo.as_mut() } {
p_gun_shot(m, accurate != 0);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FirePistol(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let Some(mo) = (unsafe { (p.mo as *mut mobj_t).as_mut() }) else { return };
unsafe {
S_StartSound(p.mo as *mut c_void, sfx_pistol as c_int);
p_set_mobj_state(&mut *(p.mo as *mut crate::p_mobj::mobj_t), S_PLAY_ATK2);
}
let w = &weaponinfo[p.readyweapon as usize];
p.ammo[w.ammo as usize] -= 1;
p_set_psprite(p, ps_flash as usize, w.flashstate as statenum_t);
p_bullet_slope(mo);
p_gun_shot(mo, p.refire == 0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireShotgun(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let Some(mo) = (unsafe { (p.mo as *mut mobj_t).as_mut() }) else { return };
unsafe {
S_StartSound(p.mo as *mut c_void, sfx_shotgn as c_int);
p_set_mobj_state(&mut *(p.mo as *mut crate::p_mobj::mobj_t), S_PLAY_ATK2);
}
let w = &weaponinfo[p.readyweapon as usize];
p.ammo[w.ammo as usize] -= 1;
p_set_psprite(p, ps_flash as usize, w.flashstate as statenum_t);
p_bullet_slope(mo);
for _ in 0..7 {
p_gun_shot(mo, false);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireShotgun2(player: *mut player_t, _psp: *mut pspdef_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
let mo_ptr = p.mo as *mut mobj_t;
let Some(mo) = (unsafe { mo_ptr.as_mut() }) else { return };
unsafe {
S_StartSound(p.mo as *mut c_void, sfx_dshtgn as c_int);
p_set_mobj_state(&mut *(p.mo as *mut crate::p_mobj::mobj_t), S_PLAY_ATK2);
}
let w = &weaponinfo[p.readyweapon as usize];
p.ammo[w.ammo as usize] -= 2;
p_set_psprite(p, ps_flash as usize, w.flashstate as statenum_t);
p_bullet_slope(mo);
let range = MISSILERANGE as i32;
for _ in 0..20 {
let damage = 5 * ((p_random() % 3) + 1);
let r_ang = (p_random() - p_random()) << 19;
let angle = mo.angle.wrapping_add(r_ang as u32);
let r_slope = (p_random() - p_random()) << 5;
unsafe {
P_LineAttack(mo_ptr, angle, range, bulletslope + r_slope, damage);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_FireCGun(player: *mut player_t, psp: *mut pspdef_t) {
let (Some(p), Some(ps)) = (unsafe { player.as_mut() }, unsafe { psp.as_ref() }) else {
return;
};
let Some(mo) = (unsafe { (p.mo as *mut mobj_t).as_mut() }) else { return };
unsafe {
S_StartSound(p.mo as *mut c_void, sfx_pistol as c_int);
}
let w = &weaponinfo[p.readyweapon as usize];
if p.ammo[w.ammo as usize] == 0 {
return;
}
unsafe {
p_set_mobj_state(&mut *(p.mo as *mut crate::p_mobj::mobj_t), S_PLAY_ATK2);
}
p.ammo[w.ammo as usize] -= 1;
let state_offset = unsafe {
let states_base = (&raw const states) as *const state_t;
let chain1_ptr = states_base.add(S_CHAIN1 as usize);
(ps.state as *const state_t).offset_from(chain1_ptr) as c_int
};
let flash_st = ((w.flashstate as c_int) + state_offset) as statenum_t;
p_set_psprite(p, ps_flash as usize, flash_st);
p_bullet_slope(mo);
p_gun_shot(mo, p.refire == 0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Light0(player: *mut player_t, _psp: *mut pspdef_t) {
if let Some(p) = unsafe { player.as_mut() } {
p.extralight = 0;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Light1(player: *mut player_t, _psp: *mut pspdef_t) {
if let Some(p) = unsafe { player.as_mut() } {
p.extralight = 1;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_Light2(player: *mut player_t, _psp: *mut pspdef_t) {
if let Some(p) = unsafe { player.as_mut() } {
p.extralight = 2;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BFGSpray(mo: *mut mobj_t) {
let Some(m) = (unsafe { mo.as_ref() }) else { return };
let aim_dist = 16 * 64 * (FRACUNIT as i32);
let base_an = m.angle.wrapping_sub(ANG90 / 2);
let step_an = ANG90 / 40;
unsafe {
for i in 0..40u32 {
let an = base_an.wrapping_add(step_an.wrapping_mul(i));
P_AimLineAttack(m.target, an, aim_dist);
let Some(target) = linetarget.as_ref() else { continue };
P_SpawnMobj(
target.x,
target.y,
target.z + (target.height >> 2),
MT_EXTRABFG,
);
let mut damage = 0;
for _ in 0..15 {
damage += (p_random() & 7) + 1;
}
P_DamageMobj(linetarget, m.target, m.target, damage);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn A_BFGsound(player: *mut player_t, _psp: *mut pspdef_t) {
if let Some(p) = unsafe { player.as_ref() } {
unsafe {
S_StartSound(p.mo as *mut c_void, sfx_bfg as c_int);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SetupPsprites(player: *mut player_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
for psp in &mut p.psprites {
psp.state = ptr::null_mut();
}
p.pendingweapon = p.readyweapon;
p_bring_up_weapon(p);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MovePsprites(player: *mut player_t) {
let Some(p) = (unsafe { player.as_mut() }) else { return };
for i in 0..(NUMPSPRITES as usize) {
let state = p.psprites[i].state;
if !state.is_null() && p.psprites[i].tics != -1 {
p.psprites[i].tics -= 1;
if p.psprites[i].tics == 0 {
p_set_psprite(p, i, unsafe { (*state).nextstate });
}
}
}
p.psprites[ps_flash as usize].sx = p.psprites[ps_weapon as usize].sx;
p.psprites[ps_flash as usize].sy = p.psprites[ps_weapon as usize].sy;
}
