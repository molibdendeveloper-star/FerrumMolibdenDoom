#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

use crate::d_items::weaponinfo;

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_void};
use crate::i_system::I_Tactile;

use crate::info::states;
use crate::m_fixed::fixed_mul;
use crate::m_random::p_random;
use crate::p_mobj::{P_RemoveMobj, P_SetMobjState, P_SpawnMobj};
use crate::p_pspr::P_DropWeapon;
use crate::r_bsp::subsector_t;
use crate::r_main::R_PointToAngle2;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};
use crate::v_video::doom_error;

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
    
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);
    fn AM_Stop();

    static mut gameskill: skill_t;
    static mut netgame: c_int;
    static mut deathmatch: c_int;
    static mut consoleplayer: c_int;
    static mut automapactive: c_int;
    static mut players: [player_t; NUM_PLAYERS];
}

pub static maxammo: [c_int; NUM_AMMO] = [200, 50, 300, 50];
pub static clipammo: [c_int; NUM_AMMO] = [10, 4, 20, 1];

#[inline]
fn is_console_player(player: &player_t) -> bool {
    unsafe {
        let players_base = (&raw const players) as *const player_t;
        (player as *const player_t) == players_base.add(consoleplayer as usize)
    }
}

#[inline]
fn player_index(player: &player_t) -> usize {
    unsafe {
        let players_base = (&raw const players) as *const player_t;
        ((player as *const player_t).offset_from(players_base) as usize).min(NUM_PLAYERS - 1)
    }
}

pub fn p_give_ammo(player: &mut player_t, ammo: ammotype_t, mut num: c_int) -> bool {
    if ammo == am_noammo {
        return false;
    }

    if (ammo as c_int) < 0 || ammo > NUMAMMO {
        doom_error(&format!("P_GiveAmmo: bad type {}", ammo));
    }

    let idx = ammo as usize;
    if player.ammo[idx] == player.maxammo[idx] {
        return false;
    }

    let clip_sz = clipammo[idx];
    if num != 0 {
        num *= clip_sz;
    } else {
        num = clip_sz / 2;
    }

    let skill = unsafe { gameskill };
    if skill == sk_baby || skill == sk_nightmare {
        num <<= 1;
    }

    let oldammo = player.ammo[idx];
    player.ammo[idx] += num;
    if player.ammo[idx] > player.maxammo[idx] {
        player.ammo[idx] = player.maxammo[idx];
    }

    if oldammo != 0 {
        return true;
    }

    let ready = player.readyweapon as u32;
    match ammo {
        am_clip => {
            if ready == wp_fist {
                player.pendingweapon = if player.weaponowned[wp_chaingun as usize] != 0 {
                    wp_chaingun as c_int
                } else {
                    wp_pistol as c_int
                };
            }
        }
        am_shell => {
            if (ready == wp_fist || ready == wp_pistol)
                && player.weaponowned[wp_shotgun as usize] != 0
            {
                player.pendingweapon = wp_shotgun as c_int;
            }
        }
        am_cell => {
            if (ready == wp_fist || ready == wp_pistol)
                && player.weaponowned[wp_plasma as usize] != 0
            {
                player.pendingweapon = wp_plasma as c_int;
            }
        }
        am_misl => {
            if ready == wp_fist && player.weaponowned[wp_missile as usize] != 0 {
                player.pendingweapon = wp_missile as c_int;
            }
        }
        _ => {}
    }

    true
}

pub fn p_give_weapon(player: &mut player_t, weapon: weapontype_t, dropped: bool) -> bool {
    let w_idx = weapon as usize;
    let ammo_type = weaponinfo[w_idx].ammo;
    let is_net = unsafe { netgame != 0 };
    let dm_mode = unsafe { deathmatch };

    if is_net && dm_mode != 2 && !dropped {
        if player.weaponowned[w_idx] != 0 {
            return false;
        }

        player.bonuscount += BONUSADD;
        player.weaponowned[w_idx] = 1;

        p_give_ammo(player, ammo_type, if dm_mode != 0 { 5 } else { 2 });
        player.pendingweapon = weapon as c_int;

        if is_console_player(player) {
            unsafe {
                S_StartSound(std::ptr::null_mut(), sfx_wpnup as c_int);
            }
        }
        return false;
    }

    let gaveammo = if ammo_type != am_noammo {
        p_give_ammo(player, ammo_type, if dropped { 1 } else { 2 })
    } else {
        false
    };

    let gaveweapon = if player.weaponowned[w_idx] != 0 {
        false
    } else {
        player.weaponowned[w_idx] = 1;
        player.pendingweapon = weapon as c_int;
        true
    };

    gaveweapon || gaveammo
}

pub fn p_give_body(player: &mut player_t, num: c_int) -> bool {
    let max_h = MAXHEALTH as c_int;
    if player.health >= max_h {
        return false;
    }

    player.health = (player.health + num).min(max_h);
    if let Some(mo) = unsafe { player.mo.as_mut() } {
        mo.health = player.health;
    }
    true
}

pub fn p_give_armor(player: &mut player_t, armortype: c_int) -> bool {
    let hits = armortype * 100;
    if player.armorpoints >= hits {
        return false;
    }

    player.armortype = armortype;
    player.armorpoints = hits;
    true
}

pub fn p_give_card(player: &mut player_t, card: card_t) {
    let idx = card as usize;
    if player.cards[idx] != 0 {
        return;
    }
    player.bonuscount = BONUSADD;
    player.cards[idx] = 1;
}

pub fn p_give_power(player: &mut player_t, power: c_int) -> bool {
    let idx = power as usize;
    let p_u = power as u32;

    if p_u == pw_invulnerability {
        player.powers[idx] = INVULNTICS;
        return true;
    }
    if p_u == pw_invisibility {
        player.powers[idx] = INVISTICS;
        if let Some(mo) = unsafe { player.mo.as_mut() } {
            mo.flags |= MF_SHADOW as c_int;
        }
        return true;
    }
    if p_u == pw_infrared {
        player.powers[idx] = INFRATICS;
        return true;
    }
    if p_u == pw_ironfeet {
        player.powers[idx] = IRONTICS;
        return true;
    }
    if p_u == pw_strength {
        p_give_body(player, 100);
        player.powers[idx] = 1;
        return true;
    }

    if player.powers[idx] != 0 {
        return false;
    }

    player.powers[idx] = 1;
    true
}

pub fn p_touch_special_thing(special: &mut mobj_t, toucher: &mut mobj_t) {
    let delta = special.z - toucher.z;
    if delta > toucher.height || delta < -8 * (FRACUNIT as i32) {
        return;
    }
    if toucher.health <= 0 {
        return;
    }

    let Some(player) = (unsafe { (toucher.player as *mut player_t).as_mut() }) else {
        return;
    };

    let mut sound = sfx_itemup as c_int;
    let dropped = (special.flags as u32 & MF_DROPPED) != 0;
    let is_net = unsafe { netgame != 0 };

    match special.sprite {
        SPR_ARM1 => {
            if !p_give_armor(player, 1) {
                return;
            }
            player.message = GOTARMOR.as_ptr() as *mut c_char;
        }
        SPR_ARM2 => {
            if !p_give_armor(player, 2) {
                return;
            }
            player.message = GOTMEGA.as_ptr() as *mut c_char;
        }
        SPR_BON1 => {
            player.health = (player.health + 1).min(200);
            if let Some(mo) = unsafe { player.mo.as_mut() } {
                mo.health = player.health;
            }
            player.message = GOTHTHBONUS.as_ptr() as *mut c_char;
        }
        SPR_BON2 => {
            player.armorpoints = (player.armorpoints + 1).min(200);
            if player.armortype == 0 {
                player.armortype = 1;
            }
            player.message = GOTARMBONUS.as_ptr() as *mut c_char;
        }
        SPR_SOUL => {
            player.health = (player.health + 100).min(200);
            if let Some(mo) = unsafe { player.mo.as_mut() } {
                mo.health = player.health;
            }
            player.message = GOTSUPER.as_ptr() as *mut c_char;
            sound = sfx_getpow as c_int;
        }
        SPR_MEGA => {
            if get_gamemode() != commercial {
                return;
            }
            player.health = 200;
            if let Some(mo) = unsafe { player.mo.as_mut() } {
                mo.health = player.health;
            }
            p_give_armor(player, 2);
            player.message = GOTMSPHERE.as_ptr() as *mut c_char;
            sound = sfx_getpow as c_int;
        }
        SPR_BKEY => {
            if player.cards[it_bluecard as usize] == 0 {
                player.message = GOTBLUECARD.as_ptr() as *mut c_char;
            }
            p_give_card(player, it_bluecard);
            if is_net {
                return;
            }
        }
        SPR_YKEY => {
            if player.cards[it_yellowcard as usize] == 0 {
                player.message = GOTYELWCARD.as_ptr() as *mut c_char;
            }
            p_give_card(player, it_yellowcard);
            if is_net {
                return;
            }
        }
        SPR_RKEY => {
            if player.cards[it_redcard as usize] == 0 {
                player.message = GOTREDCARD.as_ptr() as *mut c_char;
            }
            p_give_card(player, it_redcard);
            if is_net {
                return;
            }
        }
        SPR_BSKU => {
            if player.cards[it_blueskull as usize] == 0 {
                player.message = GOTBLUESKUL.as_ptr() as *mut c_char;
            }
            p_give_card(player, it_blueskull);
            if is_net {
                return;
            }
        }
        SPR_YSKU => {
if player.cards[it_yellowskull as usize] == 0 {
player.message = GOTYELWSKUL.as_ptr() as *mut c_char;
}
p_give_card(player, it_yellowskull);
if is_net {
return;
}
}
SPR_RSKU => {
if player.cards[it_redskull as usize] == 0 {
player.message = GOTREDSKULL.as_ptr() as *mut c_char;
}
p_give_card(player, it_redskull);
if is_net {
return;
}
}
SPR_STIM => {
if !p_give_body(player, 10) {
return;
}
player.message = GOTSTIM.as_ptr() as *mut c_char;
}
SPR_MEDI => {
if !p_give_body(player, 25) {
return;
}
player.message = if player.health < 25 {
GOTMEDINEED.as_ptr() as *mut c_char
} else {
GOTMEDIKIT.as_ptr() as *mut c_char
};
}
SPR_PINV => {
if !p_give_power(player, pw_invulnerability as c_int) {
return;
}
player.message = GOTINVUL.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_PSTR => {
if !p_give_power(player, pw_strength as c_int) {
return;
}
player.message = GOTBERSERK.as_ptr() as *mut c_char;
if player.readyweapon != (wp_fist as c_int) {
player.pendingweapon = wp_fist as c_int;
}
sound = sfx_getpow as c_int;
}
SPR_PINS => {
if !p_give_power(player, pw_invisibility as c_int) {
return;
}
player.message = GOTINVIS.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_SUIT => {
if !p_give_power(player, pw_ironfeet as c_int) {
return;
}
player.message = GOTSUIT.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_PMAP => {
if !p_give_power(player, pw_allmap as c_int) {
return;
}
player.message = GOTMAP.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_PVIS => {
if !p_give_power(player, pw_infrared as c_int) {
return;
}
player.message = GOTVISOR.as_ptr() as *mut c_char;
sound = sfx_getpow as c_int;
}
SPR_CLIP => {
if !p_give_ammo(player, am_clip, if dropped { 0 } else { 1 }) {
return;
}
player.message = GOTCLIP.as_ptr() as *mut c_char;
}
SPR_AMMO => {
if !p_give_ammo(player, am_clip, 5) {
return;
}
player.message = GOTCLIPBOX.as_ptr() as *mut c_char;
}
SPR_ROCK => {
if !p_give_ammo(player, am_misl, 1) {
return;
}
player.message = GOTROCKET.as_ptr() as *mut c_char;
}
SPR_BROK => {
if !p_give_ammo(player, am_misl, 5) {
return;
}
player.message = GOTROCKBOX.as_ptr() as *mut c_char;
}
SPR_CELL => {
if !p_give_ammo(player, am_cell, 1) {
return;
}
player.message = GOTCELL.as_ptr() as *mut c_char;
}
SPR_CELP => {
if !p_give_ammo(player, am_cell, 5) {
return;
}
player.message = GOTCELLBOX.as_ptr() as *mut c_char;
}
SPR_SHEL => {
if !p_give_ammo(player, am_shell, 1) {
return;
}
player.message = GOTSHELLS.as_ptr() as *mut c_char;
}
SPR_SBOX => {
if !p_give_ammo(player, am_shell, 5) {
return;
}
player.message = GOTSHELLBOX.as_ptr() as *mut c_char;
}
SPR_BPAK => {
if player.backpack == 0 {
for i in 0..NUM_AMMO {
player.maxammo[i] *= 2;
}
player.backpack = 1;
}
for i in 0..NUM_AMMO {
p_give_ammo(player, i as ammotype_t, 1);
}
player.message = GOTBACKPACK.as_ptr() as *mut c_char;
}
SPR_BFUG => {
if !p_give_weapon(player, wp_bfg, false) {
return;
}
player.message = GOTBFG9000.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_MGUN => {
if !p_give_weapon(player, wp_chaingun, dropped) {
return;
}
player.message = GOTCHAINGUN.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_CSAW => {
if !p_give_weapon(player, wp_chainsaw, false) {
return;
}
player.message = GOTCHAINSAW.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_LAUN => {
if !p_give_weapon(player, wp_missile, false) {
return;
}
player.message = GOTLAUNCHER.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_PLAS => {
if !p_give_weapon(player, wp_plasma, false) {
return;
}
player.message = GOTPLASMA.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_SHOT => {
if !p_give_weapon(player, wp_shotgun, dropped) {
return;
}
player.message = GOTSHOTGUN.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
SPR_SGN2 => {
if !p_give_weapon(player, wp_supershotgun, dropped) {
return;
}
player.message = GOTSHOTGUN2.as_ptr() as *mut c_char;
sound = sfx_wpnup as c_int;
}
_ => {
doom_error("P_SpecialThing: Unknown gettable thing");
}
}
if (special.flags as u32 & MF_COUNTITEM) != 0 {
player.itemcount += 1;
}
unsafe {
P_RemoveMobj(special as *mut _);
}
player.bonuscount += BONUSADD;
if is_console_player(player) {
unsafe {
S_StartSound(std::ptr::null_mut(), sound);
}
}
}
pub fn p_kill_mobj(source: Option<&mobj_t>, target: &mut mobj_t) {
target.flags &= !((MF_SHOOTABLE | MF_FLOAT | MF_SKULLFLY) as c_int);
if target.type_ != MT_SKULL {
target.flags &= !(MF_NOGRAVITY as c_int);
}
target.flags |= (MF_CORPSE | MF_DROPOFF) as c_int;
target.height >>= 2;
let t_player_opt = unsafe { (target.player as *mut player_t).as_mut() };
let s_player_opt = source.and_then(|s| unsafe { (s.player as *mut player_t).as_mut() });
if let Some(s_player) = s_player_opt {
if (target.flags as u32 & MF_COUNTKILL) != 0 {
s_player.killcount += 1;
}
if let Some(ref t_player) = t_player_opt {
let victim_idx = player_index(t_player);
s_player.frags[victim_idx] += 1;
}
} else if unsafe { netgame == 0 } && (target.flags as u32 & MF_COUNTKILL) != 0 {
unsafe {
let players_base = (&raw mut players) as *mut player_t;
(*players_base).killcount += 1;
}
}
if let Some(t_player) = t_player_opt {
if source.is_none() {
let victim_idx = player_index(t_player);
t_player.frags[victim_idx] += 1;
}
target.flags &= !(MF_SOLID as c_int);
t_player.playerstate = PST_DEAD as c_int;
unsafe {
P_DropWeapon(t_player as *mut _);
if is_console_player(t_player) && automapactive != 0 {
AM_Stop();
}
}
}
if let Some(info) = unsafe { target.info.as_ref() } {
let next_state = if target.health < -info.spawnhealth && info.xdeathstate != 0 {
info.xdeathstate
} else {
info.deathstate
};
unsafe {
P_SetMobjState(target as *mut _, next_state as _);
}
}
target.tics -= p_random() & 3;
if target.tics < 1 {
target.tics = 1;
}
let item = match target.type_ {
MT_WOLFSS | MT_POSSESSED => MT_CLIP,
MT_SHOTGUY => MT_SHOTGUN,
MT_CHAINGUY => MT_CHAINGUN,
_ => return,
};
unsafe {
let mo = P_SpawnMobj(target.x, target.y, ONFLOORZ_VAL, item as _);
(*mo).flags |= MF_DROPPED as c_int;
}
}
pub fn p_damage_mobj(
target: &mut mobj_t,
inflictor: Option<&mobj_t>,
source: Option<&mobj_t>,
mut damage: c_int,
) {
if (target.flags as u32 & MF_SHOOTABLE) == 0 || target.health <= 0 {
return;
}
if (target.flags as u32 & MF_SKULLFLY) != 0 {
target.momx = 0;
target.momy = 0;
target.momz = 0;
}
let skill = unsafe { gameskill };
if !target.player.is_null() && skill == sk_baby {
damage >>= 1;
}
let Some(info) = (unsafe { target.info.as_ref() }) else {
return;
};
let frac_u = FRACUNIT as i32;
if let Some(infl) = inflictor {
let no_chainsaw = source
.and_then(|s| unsafe { (s.player as *const player_t).as_ref() })
.map(|p| p.readyweapon != (wp_chainsaw as c_int))
.unwrap_or(true);
if (target.flags as u32 & MF_NOCLIP) == 0 && no_chainsaw {
let mut ang = unsafe { R_PointToAngle2(infl.x, infl.y, target.x, target.y) };
let mass = if info.mass != 0 { info.mass as i64 } else { 100 };
let mut thrust = ((damage as i64) * ((frac_u >> 3) as i64) * 100 / mass) as i32;
if damage < 40
&& damage > target.health
&& (target.z - infl.z) > 64 * frac_u
&& (p_random() & 1) != 0
{
ang = ang.wrapping_add(ANG180);
thrust = thrust.wrapping_mul(4);
}
let idx = (ang >> ANGLETOFINESHIFT) as usize;
target.momx = target.momx.wrapping_add(fixed_mul(thrust, finesine[idx + QUARTER_FINE]));
target.momy = target.momy.wrapping_add(fixed_mul(thrust, finesine[idx]));
}
}
if let Some(player) = unsafe { (target.player as *mut player_t).as_mut() } {
let sec_special = unsafe {
(target.subsector as *const subsector_t)
.as_ref()
.and_then(|sub| sub.sector.as_ref())
.map(|s| s.special)
.unwrap_or(0)
};
if sec_special == 11 && damage >= target.health {
damage = target.health - 1;
}
if damage < 1000
&& ((player.cheats & CF_GODMODE_FLAG) != 0
|| player.powers[pw_invulnerability as usize] != 0)
{
return;
}
if player.armortype != 0 {
let mut saved = if player.armortype == 1 {
damage / 3
} else {
damage / 2
};
if player.armorpoints <= saved {
saved = player.armorpoints;
player.armortype = 0;
}
player.armorpoints -= saved;
damage -= saved;
}
player.health = (player.health - damage).max(0);
player.attacker = source.map(|s| (s as *const mobj_t) as *mut _).unwrap_or(std::ptr::null_mut());
player.damagecount = (player.damagecount + damage).min(100);
let temp = damage.min(100);
if is_console_player(player) {
unsafe {
I_Tactile(40, 10, 40 + temp * 2);
}
}
}
target.health -= damage;
if target.health <= 0 {
p_kill_mobj(source, target);
return;
}
if p_random() < info.painchance && (target.flags as u32 & MF_SKULLFLY) == 0 {
target.flags |= MF_JUSTHIT as c_int;
unsafe {
P_SetMobjState(target as *mut _, info.painstate as _);
}
}
target.reactiontime = 0;
if let Some(src) = source {
let target_ptr = target as *const mobj_t;
let src_ptr = src as *const mobj_t;
if (target.threshold == 0 || target.type_ == MT_VILE)
&& src_ptr != target_ptr
&& src.type_ != MT_VILE
{
target.target = src_ptr as *mut mobj_t;
target.threshold = BASETHRESHOLD as c_int;
let spawn_st = unsafe { (&raw const states as *const state_t).add(info.spawnstate as usize) as *mut _ };
if target.state == spawn_st && info.seestate != 0 {
unsafe {
P_SetMobjState(target as *mut _, info.seestate as _);
}
}
}
}
}
// --- Совместимые обёртки для вызывающих модулей ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveAmmo(player: *mut player_t, ammo: ammotype_t, num: c_int) -> c_int {
match unsafe { player.as_mut() } {
Some(p) => p_give_ammo(p, ammo, num) as c_int,
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveWeapon(player: *mut player_t, weapon: weapontype_t, dropped: c_int) -> c_int {
match unsafe { player.as_mut() } {
Some(p) => p_give_weapon(p, weapon, dropped != 0) as c_int,
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveBody(player: *mut player_t, num: c_int) -> c_int {
match unsafe { player.as_mut() } {
Some(p) => p_give_body(p, num) as c_int,
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveArmor(player: *mut player_t, armortype: c_int) -> c_int {
match unsafe { player.as_mut() } {
Some(p) => p_give_armor(p, armortype) as c_int,
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GiveCard(player: *mut player_t, card: card_t) {
if let Some(p) = unsafe { player.as_mut() } {
p_give_card(p, card);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_GivePower(player: *mut player_t, power: c_int) -> c_int {
match unsafe { player.as_mut() } {
Some(p) => p_give_power(p, power) as c_int,
None => 0,
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_TouchSpecialThing(special: *mut mobj_t, toucher: *mut mobj_t) {
if let (Some(sp), Some(tc)) = (unsafe { special.as_mut() }, unsafe { toucher.as_mut() }) {
p_touch_special_thing(sp, tc);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_KillMobj(source: *mut mobj_t, target: *mut mobj_t) {
if let Some(tgt) = unsafe { target.as_mut() } {
p_kill_mobj(unsafe { source.as_ref() }, tgt);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_DamageMobj(
target: *mut mobj_t,
inflictor: *mut mobj_t,
source: *mut mobj_t,
damage: c_int,
) {
if let Some(tgt) = unsafe { target.as_mut() } {
p_damage_mobj(
tgt,
unsafe { inflictor.as_ref() },
unsafe { source.as_ref() },
damage,
);
}
}
