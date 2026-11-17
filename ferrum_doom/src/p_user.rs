#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::c_int;

use crate::info::states;
use crate::m_fixed::fixed_mul;
use crate::p_mobj::P_SetMobjState;
use crate::p_tick::leveltime;
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

    static mut gamemode: GameMode_t;
}

// --- Глобальные переменные p_user.c ---
#[unsafe(no_mangle)]
pub static mut onground: c_int = 0;

// --- P_Thrust ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_Thrust(player: *mut player_t, mut angle: u32, move_val: i32) {
    unsafe {
        angle >>= ANGLETOFINESHIFT;
        let idx = angle as usize;
        let mo = (*player).mo;

        (*mo).momx += fixed_mul(move_val, finesine[idx + QUARTER_FINE]);
        (*mo).momy += fixed_mul(move_val, finesine[idx]);
    }
}

// --- P_CalcHeight ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_CalcHeight(player: *mut player_t) {
    unsafe {
        let mo = (*player).mo;
        let frac_u = FRACUNIT as i32;

        let mut bob_val = fixed_mul((*mo).momx, (*mo).momx) + fixed_mul((*mo).momy, (*mo).momy);
        bob_val >>= 2;
        if bob_val > MAXBOB {
            bob_val = MAXBOB;
        }
        (*player).bob = bob_val;

        if ((*player).cheats as u32 & CF_NOMOMENTUM_FLAG) != 0 || onground == 0 {
            (*player).viewz = (*mo).z + VIEWHEIGHT_VAL;
            if (*player).viewz > (*mo).ceilingz - 4 * frac_u {
                (*player).viewz = (*mo).ceilingz - 4 * frac_u;
            }
            (*player).viewz = (*mo).z + (*player).viewheight;
            return;
        }

        let angle = (((FINEANGLES as i32) / 20 * leveltime) as usize) & FINEMASK;
        let bob = fixed_mul((*player).bob / 2, finesine[angle]);

        if (*player).playerstate == (PST_LIVE as c_int) {
            (*player).viewheight += (*player).deltaviewheight;

            if (*player).viewheight > VIEWHEIGHT_VAL {
                (*player).viewheight = VIEWHEIGHT_VAL;
                (*player).deltaviewheight = 0;
            }

            if (*player).viewheight < VIEWHEIGHT_VAL / 2 {
                (*player).viewheight = VIEWHEIGHT_VAL / 2;
                if (*player).deltaviewheight <= 0 {
                    (*player).deltaviewheight = 1;
                }
            }

            if (*player).deltaviewheight != 0 {
                (*player).deltaviewheight += frac_u / 4;
                if (*player).deltaviewheight == 0 {
                    (*player).deltaviewheight = 1;
                }
            }
        }

        (*player).viewz = (*mo).z + (*player).viewheight + bob;
        if (*player).viewz > (*mo).ceilingz - 4 * frac_u {
            (*player).viewz = (*mo).ceilingz - 4 * frac_u;
        }
    }
}

// --- P_MovePlayer ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_MovePlayer(player: *mut player_t) {
    unsafe {
        let cmd = (&raw mut (*player).cmd) as *mut ticcmd_t;
        let mo = (*player).mo;

        let turn = ((*cmd).angleturn as i32) << 16;
        (*mo).angle = (*mo).angle.wrapping_add(turn as u32);

        onground = if (*mo).z <= (*mo).floorz { 1 } else { 0 };

        if (*cmd).forwardmove != 0 && onground != 0 {
            P_Thrust(player, (*mo).angle, ((*cmd).forwardmove as i32) * 2048);
        }

        if (*cmd).sidemove != 0 && onground != 0 {
            P_Thrust(
                player,
                (*mo).angle.wrapping_sub(ANG90),
                ((*cmd).sidemove as i32) * 2048,
            );
        }

        if (*cmd).forwardmove != 0 || (*cmd).sidemove != 0 {
            let states_base = (&raw const states) as *const state_t;
            let play_state = states_base.add(S_PLAY as usize);
            if (*mo).state == (play_state as *mut _) {
                P_SetMobjState(mo as *mut crate::p_mobj::mobj_t, S_PLAY_RUN1);
            }
        }
    }
}

// --- P_DeathThink ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DeathThink(player: *mut player_t) {
    unsafe {
        P_MovePsprites(player);

        let frac_u = FRACUNIT as i32;
        if (*player).viewheight > 6 * frac_u {
            (*player).viewheight -= frac_u;
        }
        if (*player).viewheight < 6 * frac_u {
            (*player).viewheight = 6 * frac_u;
        }

        (*player).deltaviewheight = 0;
        let mo = (*player).mo;
        onground = if (*mo).z <= (*mo).floorz { 1 } else { 0 };
        P_CalcHeight(player);

        let attacker = (*player).attacker;
        if !attacker.is_null() && attacker != mo {
            let angle = R_PointToAngle2((*mo).x, (*mo).y, (*attacker).x, (*attacker).y);
            let delta = angle.wrapping_sub((*mo).angle);

            if delta < ANG5 || delta > 0u32.wrapping_sub(ANG5) {
                (*mo).angle = angle;
                if (*player).damagecount != 0 {
                    (*player).damagecount -= 1;
                }
            } else if delta < ANG180 {
                (*mo).angle = (*mo).angle.wrapping_add(ANG5);
            } else {
                (*mo).angle = (*mo).angle.wrapping_sub(ANG5);
            }
        } else if (*player).damagecount != 0 {
            (*player).damagecount -= 1;
        }

        let cmd = (&raw const (*player).cmd) as *const ticcmd_t;
        if ((*cmd).buttons as u32 & BT_USE) != 0 {
            (*player).playerstate = PST_REBORN as c_int;
        }
    }
}

// --- P_PlayerThink ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_PlayerThink(player: *mut player_t) {
    unsafe {
        let mo = (*player).mo;
        if ((*player).cheats as u32 & CF_NOCLIP_FLAG) != 0 {
            (*mo).flags |= MF_NOCLIP as c_int;
        } else {
            (*mo).flags &= !(MF_NOCLIP as c_int);
        }

        let cmd = (&raw mut (*player).cmd) as *mut ticcmd_t;
        if ((*mo).flags as u32 & MF_JUSTATTACKED) != 0 {
            (*cmd).angleturn = 0;
            (*cmd).forwardmove = (0xc800 / 512) as i8;
            (*cmd).sidemove = 0;
            (*mo).flags &= !(MF_JUSTATTACKED as c_int);
        }

        if (*player).playerstate == (PST_DEAD as c_int) {
            P_DeathThink(player);
            return;
        }

        if (*mo).reactiontime != 0 {
            (*mo).reactiontime -= 1;
        } else {
            P_MovePlayer(player);
        }

        P_CalcHeight(player);

        let sub = (*mo).subsector as *mut subsector_t;
        let sec = (*sub).sector;
        if (*sec).special != 0 {
            P_PlayerInSpecialSector(player);
        }

        if ((*cmd).buttons as u32 & BT_SPECIAL) != 0 {
            (*cmd).buttons = 0;
        }

        if ((*cmd).buttons as u32 & BT_CHANGE) != 0 {
            let mut newweapon =
                (((*cmd).buttons as u32 & BT_WEAPONMASK) >> BT_WEAPONSHIFT) as weapontype_t;

            if newweapon == wp_fist
                && (*player).weaponowned[wp_chainsaw as usize] != 0
                && !((*player).readyweapon == (wp_chainsaw as c_int)
                    && (*player).powers[pw_strength as usize] != 0)
            {
                newweapon = wp_chainsaw;
            }

            if gamemode == commercial
                && newweapon == wp_shotgun
                && (*player).weaponowned[wp_supershotgun as usize] != 0
                && (*player).readyweapon != (wp_supershotgun as c_int)
            {
                newweapon = wp_supershotgun;
            }

            if (*player).weaponowned[newweapon as usize] != 0
                && (newweapon as c_int) != (*player).readyweapon
            {
                if (newweapon != wp_plasma && newweapon != wp_bfg) || gamemode != shareware {
                    (*player).pendingweapon = newweapon as c_int;
                }
            }
        }

        if ((*cmd).buttons as u32 & BT_USE) != 0 {
            if (*player).usedown == 0 {
                P_UseLines(player);
                (*player).usedown = 1;
            }
        } else {
            (*player).usedown = 0;
        }

        P_MovePsprites(player);

        if (*player).powers[pw_strength as usize] != 0 {
            (*player).powers[pw_strength as usize] += 1;
        }

        if (*player).powers[pw_invulnerability as usize] != 0 {
            (*player).powers[pw_invulnerability as usize] -= 1;
        }

        if (*player).powers[pw_invisibility as usize] != 0 {
            (*player).powers[pw_invisibility as usize] -= 1;
            if (*player).powers[pw_invisibility as usize] == 0 {
                (*mo).flags &= !(MF_SHADOW as c_int);
            }
        }

        if (*player).powers[pw_infrared as usize] != 0 {
            (*player).powers[pw_infrared as usize] -= 1;
        }

        if (*player).powers[pw_ironfeet as usize] != 0 {
            (*player).powers[pw_ironfeet as usize] -= 1;
        }

        if (*player).damagecount != 0 {
            (*player).damagecount -= 1;
        }

        if (*player).bonuscount != 0 {
            (*player).bonuscount -= 1;
        }

        let invul = (*player).powers[pw_invulnerability as usize];
        let infra = (*player).powers[pw_infrared as usize];

        if invul != 0 {
(*player).fixedcolormap = if invul > 4 * 32 || (invul & 8) != 0 {
INVERSECOLORMAP
} else {
0
};
} else if infra != 0 {
(*player).fixedcolormap = if infra > 4 * 32 || (infra & 8) != 0 {
1
} else {
0
};
} else {
(*player).fixedcolormap = 0;
}
}
}