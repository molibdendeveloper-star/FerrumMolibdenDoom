#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CString, c_char, c_int, c_void};

use crate::info::states;
use crate::m_fixed::fixed_mul;
use crate::m_random::p_random;
use crate::p_mobj::{P_RemoveMobj, P_SetMobjState, P_SpawnMobj};
use crate::p_pspr::P_DropWeapon;
use crate::r_bsp::subsector_t;
use crate::r_main::R_PointToAngle2;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};

const BONUSADD: c_int = 6;
const INVULNTICS: c_int = 30 * 35;
const INVISTICS: c_int = 60 * 35;
const INFRATICS: c_int = 120 * 35;
const IRONTICS: c_int = 60 * 35;
const QUARTER_FINE: usize = FINEANGLES / 4;
const ONFLOORZ_VAL: i32 = i32::MIN;
const CF_GODMODE_FLAG: c_int = 2;
const NUM_AMMO: usize = NUMAMMO as usize;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn I_Tactile(on: c_int, off: c_int, total: c_int);
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn AM_Stop();

    static mut weaponinfo: [weaponinfo_t; NUMWEAPONS as usize];
    static mut gameskill: skill_t;
    static mut gamemode: GameMode_t;
    static mut netgame: c_int;
    static mut deathmatch: c_int;
    static mut consoleplayer: c_int;
    static mut automapactive: c_int;
    static mut players: [player_t; NUM_PLAYERS];
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Interaction error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные p_inter.c ---
#[unsafe(no_mangle)]
pub static mut maxammo: [c_int; NUM_AMMO] = [200, 50, 300, 50];

#[unsafe(no_mangle)]
pub static mut clipammo: [c_int; NUM_AMMO] = [10, 4, 20, 1];

// --- P_GiveAmmo ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveAmmo(
    player: *mut player_t,
    ammo: ammotype_t,
    mut num: c_int,
) -> c_int {
    unsafe {
        if ammo == am_noammo {
            return 0;
        }

        if (ammo as c_int) < 0 || ammo > NUMAMMO {
            doom_error(&format!("P_GiveAmmo: bad type {}", ammo));
        }

        let idx = ammo as usize;
        if (*player).ammo[idx] == (*player).maxammo[idx] {
            return 0;
        }

        let clip_ptr = (&raw const clipammo) as *const c_int;
        let clip_sz = *clip_ptr.add(idx);

        if num != 0 {
            num *= clip_sz;
        } else {
            num = clip_sz / 2;
        }

        if gameskill == sk_baby || gameskill == sk_nightmare {
            num <<= 1;
        }

        let oldammo = (*player).ammo[idx];
        (*player).ammo[idx] += num;

        if (*player).ammo[idx] > (*player).maxammo[idx] {
            (*player).ammo[idx] = (*player).maxammo[idx];
        }

        if oldammo != 0 {
            return 1;
        }

        let ready = (*player).readyweapon as u32;
        match ammo {
            am_clip => {
                if ready == wp_fist {
                    if (*player).weaponowned[wp_chaingun as usize] != 0 {
                        (*player).pendingweapon = wp_chaingun as c_int;
                    } else {
                        (*player).pendingweapon = wp_pistol as c_int;
                    }
                }
            }
            am_shell => {
                if (ready == wp_fist || ready == wp_pistol)
                    && (*player).weaponowned[wp_shotgun as usize] != 0
                {
                    (*player).pendingweapon = wp_shotgun as c_int;
                }
            }
            am_cell => {
                if (ready == wp_fist || ready == wp_pistol)
                    && (*player).weaponowned[wp_plasma as usize] != 0
                {
                    (*player).pendingweapon = wp_plasma as c_int;
                }
            }
            am_misl => {
                if ready == wp_fist && (*player).weaponowned[wp_missile as usize] != 0 {
                    (*player).pendingweapon = wp_missile as c_int;
                }
            }
            _ => {}
        }

        1
    }
}

// --- P_GiveWeapon ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveWeapon(
    player: *mut player_t,
    weapon: weapontype_t,
    dropped: c_int,
) -> c_int {
    unsafe {
        let w_idx = weapon as usize;
        let winfo_ptr = (&raw const weaponinfo) as *const weaponinfo_t;
        let ammo_type = (*winfo_ptr.add(w_idx)).ammo;
        let players_base = (&raw mut players) as *mut player_t;

        if netgame != 0 && deathmatch != 2 && dropped == 0 {
            if (*player).weaponowned[w_idx] != 0 {
                return 0;
            }

            (*player).bonuscount += BONUSADD;
            (*player).weaponowned[w_idx] = 1;

            if deathmatch != 0 {
                P_GiveAmmo(player, ammo_type, 5);
            } else {
                P_GiveAmmo(player, ammo_type, 2);
            }
            (*player).pendingweapon = weapon as c_int;

            if player == players_base.add(consoleplayer as usize) {
                S_StartSound(std::ptr::null_mut(), sfx_wpnup as c_int);
            }
            return 0;
        }

        let gaveammo = if ammo_type != am_noammo {
            if dropped != 0 {
                P_GiveAmmo(player, ammo_type, 1) != 0
            } else {
                P_GiveAmmo(player, ammo_type, 2) != 0
            }
        } else {
            false
        };

        let gaveweapon = if (*player).weaponowned[w_idx] != 0 {
            false
        } else {
            (*player).weaponowned[w_idx] = 1;
            (*player).pendingweapon = weapon as c_int;
            true
        };

        (gaveweapon || gaveammo) as c_int
    }
}

// --- P_GiveBody ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveBody(player: *mut player_t, num: c_int) -> c_int {
    unsafe {
        let max_h = MAXHEALTH as c_int;
        if (*player).health >= max_h {
            return 0;
        }

        (*player).health += num;
        if (*player).health > max_h {
            (*player).health = max_h;
        }
        (*(*player).mo).health = (*player).health;

        1
    }
}

// --- P_GiveArmor ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveArmor(player: *mut player_t, armortype: c_int) -> c_int {
    unsafe {
        let hits = armortype * 100;
        if (*player).armorpoints >= hits {
            return 0;
        }

        (*player).armortype = armortype;
        (*player).armorpoints = hits;

        1
    }
}

// --- P_GiveCard ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveCard(player: *mut player_t, card: card_t) {
    unsafe {
        let idx = card as usize;
        if (*player).cards[idx] != 0 {
            return;
        }

        (*player).bonuscount = BONUSADD;
        (*player).cards[idx] = 1;
    }
}

// --- P_GivePower ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GivePower(player: *mut player_t, power: c_int) -> c_int {
    unsafe {
        let idx = power as usize;
        let p_u = power as u32;

        if p_u == pw_invulnerability {
            (*player).powers[idx] = INVULNTICS as c_int;
            return 1;
        }

        if p_u == pw_invisibility {
            (*player).powers[idx] = INVISTICS as c_int;
            (*(*player).mo).flags |= MF_SHADOW as c_int;
            return 1;
        }

        if p_u == pw_infrared {
            (*player).powers[idx] = INFRATICS as c_int;
            return 1;
        }

        if p_u == pw_ironfeet {
            (*player).powers[idx] = IRONTICS as c_int;
            return 1;
        }

        if p_u == pw_strength {
            P_GiveBody(player, 100);
            (*player).powers[idx] = 1;
            return 1;
        }

        if (*player).powers[idx] != 0 {
            return 0;
        }

        (*player).powers[idx] = 1;
        1
    }
}

// --- P_TouchSpecialThing ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TouchSpecialThing(special: *mut mobj_t, toucher: *mut mobj_t) {
    unsafe {
        let delta = (*special).z - (*toucher).z;
        if delta > (*toucher).height || delta < -8 * (FRACUNIT as i32) {
            return;
        }

        if (*toucher).health <= 0 {
            return;
        }

        let mut sound = sfx_itemup as c_int;
        let player = (*toucher).player;
        let dropped = ((*special).flags as u32 & MF_DROPPED) as c_int;

        match (*special).sprite {
            // Armor
            SPR_ARM1 => {
                if P_GiveArmor(player, 1) == 0 {
                    return;
                }
                (*player).message = GOTARMOR.as_ptr() as *mut c_char;
            }
            SPR_ARM2 => {
                if P_GiveArmor(player, 2) == 0 {
                    return;
                }
                (*player).message = GOTMEGA.as_ptr() as *mut c_char;
            }

            // Bonus items
            SPR_BON1 => {
                (*player).health += 1;
                if (*player).health > 200 {
                    (*player).health = 200;
                }
                (*(*player).mo).health = (*player).health;
                (*player).message = GOTHTHBONUS.as_ptr() as *mut c_char;
            }
            SPR_BON2 => {
                (*player).armorpoints += 1;
                if (*player).armorpoints > 200 {
                    (*player).armorpoints = 200;
                }
                if (*player).armortype == 0 {
                    (*player).armortype = 1;
                }
                (*player).message = GOTARMBONUS.as_ptr() as *mut c_char;
            }
            SPR_SOUL => {
                (*player).health += 100;
                if (*player).health > 200 {
                    (*player).health = 200;
                }
                (*(*player).mo).health = (*player).health;
                (*player).message = GOTSUPER.as_ptr() as *mut c_char;
                sound = sfx_getpow as c_int;
            }
            SPR_MEGA => {
if gamemode != commercial {
return;
}
(*player).health = 200;
(*(*player).mo).health = (*player).health;
P_GiveArmor(player, 2);
(*player).message = GOTMSPHERE.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
// Cards
SPR_BKEY => {
if (*player).cards[it_bluecard as usize] == 0 {
(*player).message = GOTBLUECARD.as_ptr() as *mut c_char;
}
P_GiveCard(player, it_bluecard);
if netgame != 0 {
return;
}
}
SPR_YKEY => {
if (*player).cards[it_yellowcard as usize] == 0 {
(*player).message = GOTYELWCARD.as_ptr() as *mut c_char;
}
P_GiveCard(player, it_yellowcard);
if netgame != 0 {
return;
}
}
SPR_RKEY => {
if (*player).cards[it_redcard as usize] == 0 {
(*player).message = GOTREDCARD.as_ptr() as *mut c_char;
}
P_GiveCard(player, it_redcard);
if netgame != 0 {
return;
}
}
SPR_BSKU => {
if (*player).cards[it_blueskull as usize] == 0 {
(*player).message = GOTBLUESKUL.as_ptr() as *mut c_char;
}
P_GiveCard(player, it_blueskull);
if netgame != 0 {
return;
}
}
SPR_YSKU => {
if (*player).cards[it_yellowskull as usize] == 0 {
(*player).message = GOTYELWSKUL.as_ptr() as *mut c_char;
}
P_GiveCard(player, it_yellowskull);
if netgame != 0 {
return;
}
}
SPR_RSKU => {
if (*player).cards[it_redskull as usize] == 0 {
(*player).message = GOTREDSKULL.as_ptr() as *mut c_char;
}
P_GiveCard(player, it_redskull);
if netgame != 0 {
return;
}
}
// Medikits, heals
SPR_STIM => {
if P_GiveBody(player, 10) == 0 {
return;
}
(*player).message = GOTSTIM.as_ptr() as *mut c_char;
}
SPR_MEDI => {
if P_GiveBody(player, 25) == 0 {
return;
}
if (*player).health < 25 {
(*player).message = GOTMEDINEED.as_ptr() as *mut c_char;
} else {
(*player).message = GOTMEDIKIT.as_ptr() as *mut c_char;
}
}
// Power ups
SPR_PINV => {
if P_GivePower(player, pw_invulnerability as c_int) == 0 {
return;
}
(*player).message = GOTINVUL.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_PSTR => {
if P_GivePower(player, pw_strength as c_int) == 0 {
return;
}
(*player).message = GOTBERSERK.as_ptr() as *mut c_char;
if (*player).readyweapon != (wp_fist as c_int) {
(*player).pendingweapon = wp_fist as c_int;
}
sound = sfx_getpow as c_int;
}
SPR_PINS => {
if P_GivePower(player, pw_invisibility as c_int) == 0 {
return;
}
(*player).message = GOTINVIS.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_SUIT => {
if P_GivePower(player, pw_ironfeet as c_int) == 0 {
return;
}
(*player).message = GOTSUIT.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_PMAP => {
if P_GivePower(player, pw_allmap as c_int) == 0 {
return;
}
(*player).message = GOTMAP.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_PVIS => {
if P_GivePower(player, pw_infrared as c_int) == 0 {
return;
}
(*player).message = GOTVISOR.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
// Ammo
SPR_CLIP => {
let amt = if dropped != 0 { 0 } else { 1 };
if P_GiveAmmo(player, am_clip, amt) == 0 {
return;
}
(*player).message = GOTCLIP.as_ptr() as *mut c_char;
}
SPR_AMMO => {
if P_GiveAmmo(player, am_clip, 5) == 0 {
return;
}
(*player).message = GOTCLIPBOX.as_ptr() as *mut c_char;
}
SPR_ROCK => {
if P_GiveAmmo(player, am_misl, 1) == 0 {
return;
}
(*player).message = GOTROCKET.as_ptr() as *mut c_char;
}
SPR_BROK => {
if P_GiveAmmo(player, am_misl, 5) == 0 {
return;
}
(*player).message = GOTROCKBOX.as_ptr() as *mut c_char;
}
SPR_CELL => {
if P_GiveAmmo(player, am_cell, 1) == 0 {
return;
}
(*player).message = GOTCELL.as_ptr() as *mut c_char;
}
SPR_CELP => {
if P_GiveAmmo(player, am_cell, 5) == 0 {
return;
}
(*player).message = GOTCELLBOX.as_ptr() as *mut c_char;
}
SPR_SHEL => {
if P_GiveAmmo(player, am_shell, 1) == 0 {
return;
}
(*player).message = GOTSHELLS.as_ptr() as *mut c_char;
}
SPR_SBOX => {
if P_GiveAmmo(player, am_shell, 5) == 0 {
return;
}
(*player).message = GOTSHELLBOX.as_ptr() as *mut c_char;
}
SPR_BPAK => {
if (*player).backpack == 0 {
for i in 0..NUM_AMMO {
(*player).maxammo[i] *= 2;
}
(*player).backpack = 1;
}
for i in 0..NUM_AMMO {
P_GiveAmmo(player, i as ammotype_t, 1);
}
(*player).message = GOTBACKPACK.as_ptr() as *mut c_char;
}
// Weapons
SPR_BFUG => {
if P_GiveWeapon(player, wp_bfg, 0) == 0 {
return;
}
(*player).message = GOTBFG9000.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_MGUN => {
if P_GiveWeapon(player, wp_chaingun, dropped) == 0 {
return;
}
(*player).message = GOTCHAINGUN.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_CSAW => {
if P_GiveWeapon(player, wp_chainsaw, 0) == 0 {
return;
}
(*player).message = GOTCHAINSAW.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_LAUN => {
if P_GiveWeapon(player, wp_missile, 0) == 0 {
return;
}
(*player).message = GOTLAUNCHER.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_PLAS => {
if P_GiveWeapon(player, wp_plasma, 0) == 0 {
return;
}
(*player).message = GOTPLASMA.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_SHOT => {
if P_GiveWeapon(player, wp_shotgun, dropped) == 0 {
return;
}
(*player).message = GOTSHOTGUN.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_SGN2 => {
if P_GiveWeapon(player, wp_supershotgun, dropped) == 0 {
return;
}
(*player).message = GOTSHOTGUN2.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
_ => {
doom_error("P_SpecialThing: Unknown gettable thing");
}
}
if ((*special).flags as u32 & MF_COUNTITEM) != 0 {
(*player).itemcount += 1;
}
P_RemoveMobj(special as *mut _);
(*player).bonuscount += BONUSADD;
let players_base = (&raw mut players) as *mut player_t;
if player == players_base.add(consoleplayer as usize) {
S_StartSound(std::ptr::null_mut(), sound);
}
}
}
// --- P_KillMobj ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_KillMobj(source: *mut mobj_t, target: *mut mobj_t) {
unsafe {
(*target).flags &= !((MF_SHOOTABLE | MF_FLOAT | MF_SKULLFLY) as c_int);
if (*target).type_ != MT_SKULL {
(*target).flags &= !(MF_NOGRAVITY as c_int);
}
(*target).flags |= (MF_CORPSE | MF_DROPOFF) as c_int;
(*target).height >>= 2;
let players_base = (&raw mut players) as *mut player_t;
let t_player = (*target).player;
if !source.is_null() && !(*source).player.is_null() {
let s_player = (*source).player;
if ((*target).flags as u32 & MF_COUNTKILL) != 0 {
(*s_player).killcount += 1;
}
if !t_player.is_null() {
let victim_idx = t_player.offset_from(players_base) as usize;
(*s_player).frags[victim_idx] += 1;
}
} else if netgame == 0 && ((*target).flags as u32 & MF_COUNTKILL) != 0 {
(*players_base).killcount += 1;
}
if !t_player.is_null() {
if source.is_null() {
let victim_idx = t_player.offset_from(players_base) as usize;
(*t_player).frags[victim_idx] += 1;
}
(*target).flags &= !(MF_SOLID as c_int);
(*t_player).playerstate = PST_DEAD as c_int;
P_DropWeapon(t_player);
if t_player == players_base.add(consoleplayer as usize) && automapactive != 0 {
AM_Stop();
}
}
let info = (*target).info;
if (*target).health < -(*info).spawnhealth && (*info).xdeathstate != 0 {
P_SetMobjState(target as *mut _, (*info).xdeathstate as _);
} else {
P_SetMobjState(target as *mut _, (*info).deathstate as _);
}
(*target).tics -= p_random() & 3;
if (*target).tics < 1 {
(*target).tics = 1;
}
let item = match (*target).type_ {
MT_WOLFSS | MT_POSSESSED => MT_CLIP,
MT_SHOTGUY => MT_SHOTGUN,
MT_CHAINGUY => MT_CHAINGUN,
_ => return,
};
let mo = P_SpawnMobj((*target).x, (*target).y, ONFLOORZ_VAL, item as _);
(*mo).flags |= MF_DROPPED as c_int;
}
}
// --- P_DamageMobj ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DamageMobj(
target: *mut mobj_t,
inflictor: *mut mobj_t,
source: *mut mobj_t,
mut damage: c_int,
) {
unsafe {
if ((*target).flags as u32 & MF_SHOOTABLE) == 0 {
return;
}
if (*target).health <= 0 {
return;
}
if ((*target).flags as u32 & MF_SKULLFLY) != 0 {
(*target).momx = 0;
(*target).momy = 0;
(*target).momz = 0;
}
let player = (*target).player;
if !player.is_null() && gameskill == sk_baby {
damage >>= 1;
}
let info = (*target).info;
let frac_u = FRACUNIT as i32;
if !inflictor.is_null()
&& ((*target).flags as u32 & MF_NOCLIP) == 0
&& (source.is_null()
|| (*source).player.is_null()
|| (*(*source).player).readyweapon != (wp_chainsaw as c_int))
{
let mut ang = R_PointToAngle2((*inflictor).x, (*inflictor).y, (*target).x, (*target).y);
let mass = if (*info).mass != 0 { (*info).mass as i64 } else { 100 };
let mut thrust =
((damage as i64) * ((frac_u >> 3) as i64) * 100 / mass) as i32;
if damage < 40
&& damage > (*target).health
&& ((*target).z - (*inflictor).z) > 64 * frac_u
&& (p_random() & 1) != 0
{
ang = ang.wrapping_add(ANG180);
thrust = thrust.wrapping_mul(4);
}
let idx = (ang >> ANGLETOFINESHIFT) as usize;
(*target).momx = (*target).momx.wrapping_add(fixed_mul(thrust, finesine[idx + QUARTER_FINE]));
(*target).momy = (*target).momy.wrapping_add(fixed_mul(thrust, finesine[idx]));
}
if !player.is_null() {
let sub = (*target).subsector as *mut subsector_t;
let sec = (*sub).sector;
if (*sec).special == 11 && damage >= (*target).health {
damage = (*target).health - 1;
}
if damage < 1000
&& (((*player).cheats & CF_GODMODE_FLAG) != 0
|| (*player).powers[pw_invulnerability as usize] != 0)
{
return;
}
if (*player).armortype != 0 {
let mut saved = if (*player).armortype == 1 {
damage / 3
} else {
damage / 2
};
if (*player).armorpoints <= saved {
saved = (*player).armorpoints;
(*player).armortype = 0;
}
(*player).armorpoints -= saved;
damage -= saved;
}
(*player).health -= damage;
if (*player).health < 0 {
(*player).health = 0;
}
(*player).attacker = source as *mut _;
(*player).damagecount += damage;
if (*player).damagecount > 100 {
(*player).damagecount = 100;
}
let temp = if damage < 100 { damage } else { 100 };
let players_base = (&raw mut players) as *mut player_t;
if player == players_base.add(consoleplayer as usize) {
I_Tactile(40, 10, 40 + temp * 2);
}
}
(*target).health -= damage;
if (*target).health <= 0 {
P_KillMobj(source, target);
return;
}
if p_random() < (*info).painchance && ((*target).flags as u32 & MF_SKULLFLY) == 0 {
(*target).flags |= MF_JUSTHIT as c_int;
P_SetMobjState(target as *mut _, (*info).painstate as _);
}
(*target).reactiontime = 0;
if ((*target).threshold == 0 || (*target).type_ == MT_VILE)
&& !source.is_null()
&& source != target
&& (*source).type_ != MT_VILE
{
(*target).target = source;
(*target).threshold = BASETHRESHOLD as c_int;
let states_base = (&raw const states) as *const state_t;
let spawn_st = states_base.add((*info).spawnstate as usize) as *mut _;
if (*target).state == spawn_st && (*info).seestate != 0 {
P_SetMobjState(target as *mut _, (*info).seestate as _);
}
}
}
}