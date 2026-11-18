#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_things::player_t;

use std::ffi::c_int;

use crate::info::states;
use crate::m_fixed::fixed_mul;
use crate::p_mobj::P_SetMobjState;
use crate::p_tick::get_leveltime;
use crate::r_bsp::subsector_t;
use crate::r_main::R_PointToAngle2;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, FINEMASK, finesine};

const INVERSECOLORMAP: c_int = 32;
const MAXBOB: i32 = 0x100000;
const QUARTER_FINE: usize = FINEANGLES / 4;
const ANG5: u32 = ANG90 / 18;
const VIEWHEIGHT_VAL: i32 = 41 * (FRACUNIT as i32);
const CF_NOCLIP_FLAG: u32 = 1;
const CF_NOMOMENTUM_FLAG: u32 = 4;

unsafe extern "C" {
    fn P_MovePsprites(player: *mut player_t);
    fn P_PlayerInSpecialSector(player: *mut player_t);
    fn P_UseLines(player: *mut player_t);
}

#[inline]
fn cmd_mut(player: &mut player_t) -> &mut ticcmd_t {
    unsafe { &mut *(player.cmd.as_mut_ptr() as *mut ticcmd_t) }
}

#[inline]
pub fn p_thrust(player: &mut player_t, angle: u32, move_val: i32) {
    let idx = (angle >> ANGLETOFINESHIFT) as usize;
    if let Some(mo) = unsafe { player.mo.as_mut() } {
        mo.momx += fixed_mul(move_val, finesine[idx + QUARTER_FINE]);
        mo.momy += fixed_mul(move_val, finesine[idx]);
    }
}

pub fn p_calc_height(player: &mut player_t, on_ground: bool) {
    let Some(mo) = (unsafe { player.mo.as_mut() }) else {
        return;
    };
    let frac_u = FRACUNIT as i32;

    let mut bob_val = fixed_mul(mo.momx, mo.momx) + fixed_mul(mo.momy, mo.momy);
    bob_val >>= 2;
    if bob_val > MAXBOB {
        bob_val = MAXBOB;
    }
    player.bob = bob_val;

    if (player.cheats as u32 & CF_NOMOMENTUM_FLAG) != 0 || !on_ground {
        player.viewz = mo.z + VIEWHEIGHT_VAL;
        if player.viewz > mo.ceilingz - 4 * frac_u {
            player.viewz = mo.ceilingz - 4 * frac_u;
        }
        player.viewz = mo.z + player.viewheight;
        return;
    }

    let angle = (((FINEANGLES as i32) / 20 * get_leveltime()) as usize) & FINEMASK;
    let bob = fixed_mul(player.bob / 2, finesine[angle]);

    if player.playerstate == (PST_LIVE as c_int) {
        player.viewheight += player.deltaviewheight;

        if player.viewheight > VIEWHEIGHT_VAL {
            player.viewheight = VIEWHEIGHT_VAL;
            player.deltaviewheight = 0;
        }

        if player.viewheight < VIEWHEIGHT_VAL / 2 {
            player.viewheight = VIEWHEIGHT_VAL / 2;
            if player.deltaviewheight <= 0 {
                player.deltaviewheight = 1;
            }
        }

        if player.deltaviewheight != 0 {
            player.deltaviewheight += frac_u / 4;
            if player.deltaviewheight == 0 {
                player.deltaviewheight = 1;
            }
        }
    }

    player.viewz = mo.z + player.viewheight + bob;
    if player.viewz > mo.ceilingz - 4 * frac_u {
        player.viewz = mo.ceilingz - 4 * frac_u;
    }
}

pub fn p_move_player(player: &mut player_t) -> bool {
    let forward = cmd_mut(player).forwardmove;
    let side = cmd_mut(player).sidemove;
    let turn = (cmd_mut(player).angleturn as i32) << 16;

    let (angle, on_ground, should_run) = {
        let Some(mo) = (unsafe { player.mo.as_mut() }) else {
            return false;
        };
        mo.angle = mo.angle.wrapping_add(turn as u32);
        let on_ground = mo.z <= mo.floorz;

        let play_state_ptr = unsafe { (&raw const states as *const state_t).add(S_PLAY as usize) };
        let should_run = (forward != 0 || side != 0) && (mo.state == (play_state_ptr as *mut _));
        (mo.angle, on_ground, should_run)
    };

    if forward != 0 && on_ground {
        p_thrust(player, angle, (forward as i32) * 2048);
    }

    if side != 0 && on_ground {
        p_thrust(player, angle.wrapping_sub(ANG90), (side as i32) * 2048);
    }

    if should_run {
        unsafe {
            P_SetMobjState(player.mo as *mut crate::p_mobj::mobj_t, S_PLAY_RUN1);
        }
    }

    on_ground
}

pub fn p_death_think(player: &mut player_t) {
    unsafe {
        P_MovePsprites(player as *mut player_t);
    }

    let frac_u = FRACUNIT as i32;
    if player.viewheight > 6 * frac_u {
        player.viewheight -= frac_u;
    }
    if player.viewheight < 6 * frac_u {
        player.viewheight = 6 * frac_u;
    }

    player.deltaviewheight = 0;

    let on_ground = unsafe {
        player.mo.as_ref().map(|mo| mo.z <= mo.floorz).unwrap_or(false)
    };
    p_calc_height(player, on_ground);

    let mo_ptr = player.mo;
    let attacker_ptr = player.attacker;

    if !attacker_ptr.is_null() && attacker_ptr != mo_ptr {
        if let (Some(mo), Some(attacker)) = (unsafe { mo_ptr.as_mut() }, unsafe { attacker_ptr.as_ref() }) {
            let angle = unsafe { R_PointToAngle2(mo.x, mo.y, attacker.x, attacker.y) };
            let delta = angle.wrapping_sub(mo.angle);

            if delta < ANG5 || delta > 0u32.wrapping_sub(ANG5) {
                mo.angle = angle;
                if player.damagecount != 0 {
                    player.damagecount -= 1;
                }
            } else if delta < ANG180 {
                mo.angle = mo.angle.wrapping_add(ANG5);
            } else {
                mo.angle = mo.angle.wrapping_sub(ANG5);
            }
        }
    } else if player.damagecount != 0 {
        player.damagecount -= 1;
    }

    if (cmd_mut(player).buttons as u32 & BT_USE) != 0 {
        player.playerstate = PST_REBORN as c_int;
    }
}

pub fn p_player_think(player: &mut player_t) {
    if let Some(mo) = unsafe { player.mo.as_mut() } {
        if (player.cheats as u32 & CF_NOCLIP_FLAG) != 0 {
            mo.flags |= MF_NOCLIP as c_int;
        } else {
            mo.flags &= !(MF_NOCLIP as c_int);
        }

        if (mo.flags as u32 & MF_JUSTATTACKED) != 0 {
            cmd_mut(player).angleturn = 0;
            cmd_mut(player).forwardmove = (0xc800 / 512) as i8;
            cmd_mut(player).sidemove = 0;
            mo.flags &= !(MF_JUSTATTACKED as c_int);
        }
    }

    if player.playerstate == (PST_DEAD as c_int) {
        p_death_think(player);
        return;
    }

    let on_ground = if let Some(mo) = unsafe { player.mo.as_mut() } {
        if mo.reactiontime != 0 {
            mo.reactiontime -= 1;
            mo.z <= mo.floorz
        } else {
            p_move_player(player)
        }
    } else {
        false
    };

    p_calc_height(player, on_ground);

    let in_special = unsafe {
        player
            .mo
            .as_ref()
            .and_then(|mo| (mo.subsector as *mut subsector_t).as_ref())
            .and_then(|sub| sub.sector.as_ref())
            .map(|sec| sec.special != 0)
            .unwrap_or(false)
    };
    if in_special {
        unsafe {
            P_PlayerInSpecialSector(player as *mut player_t);
        }
    }

    let buttons = cmd_mut(player).buttons as u32;
    if (buttons & BT_SPECIAL) != 0 {
        cmd_mut(player).buttons = 0;
    }

    if (buttons & BT_CHANGE) != 0 {
        let mut newweapon = ((buttons & BT_WEAPONMASK) >> BT_WEAPONSHIFT) as weapontype_t;

        if newweapon == wp_fist
            && player.weaponowned[wp_chainsaw as usize] != 0
            && !(player.readyweapon == (wp_chainsaw as c_int)
                && player.powers[pw_strength as usize] != 0)
        {
            newweapon = wp_chainsaw;
        }

        if get_gamemode() == commercial
            && newweapon == wp_shotgun
            && player.weaponowned[wp_supershotgun as usize] != 0
            && player.readyweapon != (wp_supershotgun as c_int)
        {
            newweapon = wp_supershotgun;
        }

        if player.weaponowned[newweapon as usize] != 0
            && (newweapon as c_int) != player.readyweapon
        {
            if (newweapon != wp_plasma && newweapon != wp_bfg) || get_gamemode() != shareware {
                player.pendingweapon = newweapon as c_int;
            }
        }
    }

    if (buttons & BT_USE) != 0 {
        if player.usedown == 0 {
            unsafe {
                P_UseLines(player as *mut player_t);
            }
            player.usedown = 1;
        }
    } else {
        player.usedown = 0;
    }

    unsafe {
        P_MovePsprites(player as *mut player_t);
    }

    if player.powers[pw_strength as usize] != 0 {
        player.powers[pw_strength as usize] += 1;
    }

    if player.powers[pw_invulnerability as usize] != 0 {
        player.powers[pw_invulnerability as usize] -= 1;
    }

    if player.powers[pw_invisibility as usize] != 0 {
        player.powers[pw_invisibility as usize] -= 1;
        if player.powers[pw_invisibility as usize] == 0 {
            if let Some(mo) = unsafe { player.mo.as_mut() } {
                mo.flags &= !(MF_SHADOW as c_int);
            }
        }
    }

    if player.powers[pw_infrared as usize] != 0 {
        player.powers[pw_infrared as usize] -= 1;
    }

    if player.powers[pw_ironfeet as usize] != 0 {
        player.powers[pw_ironfeet as usize] -= 1;
    }

    if player.damagecount != 0 {
        player.damagecount -= 1;
    }

    if player.bonuscount != 0 {
        player.bonuscount -= 1;
    }

    let invul = player.powers[pw_invulnerability as usize];
    let infra = player.powers[pw_infrared as usize];

    if invul != 0 {
        player.fixedcolormap = if invul > 4 * 32 || (invul & 8) != 0 {
            INVERSECOLORMAP
        } else {
            0
        };
    } else if infra != 0 {
        player.fixedcolormap = if infra > 4 * 32 || (infra & 8) != 0 {
            1
        } else {
            0
        };
    } else {
        player.fixedcolormap = 0;
    }
}

// --- Тонкие совместимые обёртки для вызывающих модулей ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_Thrust(player: *mut player_t, angle: u32, move_val: i32) {
    if let Some(p) = unsafe { player.as_mut() } {
        p_thrust(p, angle, move_val);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CalcHeight(player: *mut player_t) {
    if let Some(p) = unsafe { player.as_mut() } {
let on_ground = unsafe { p.mo.as_ref().map(|mo| mo.z <= mo.floorz).unwrap_or(false) };
p_calc_height(p, on_ground);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MovePlayer(player: *mut player_t) {
if let Some(p) = unsafe { player.as_mut() } {
p_move_player(p);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DeathThink(player: *mut player_t) {
if let Some(p) = unsafe { player.as_mut() } {
p_death_think(p);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PlayerThink(player: *mut player_t) {
if let Some(p) = unsafe { player.as_mut() } {
p_player_think(p);
}
}
