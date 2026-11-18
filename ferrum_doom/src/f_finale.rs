#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::hu_stuff::hu_font;
use crate::info::{mobjinfo, states};
use crate::m_misc::HU_FONTSIZE;
use crate::r_data::firstspritelump;
use crate::r_things::sprites;
use crate::v_video::{V_DrawPatch, V_DrawPatchFlipped, V_MarkRect, screens};
use crate::w_wad::{W_CacheLumpName, W_CacheLumpNum};
use crate::z_zone::{PU_CACHE, PU_LEVEL};

const mus_victor: u32 = 31;
const mus_bunny: u32 = 30;
const mus_read_m: u32 = 67;
const mus_evil: u32 = 65;

const TEXTSPEED: c_int = 3;
const TEXTWAIT: c_int = 250;
const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const SCR_W: usize = SCREENWIDTH as usize;
const SCR_H: usize = SCREENHEIGHT as usize;

const E1TEXT_STR: &[u8] = b"Once you beat the big badasses and\nclean out the moon base you're supposed\nto win, aren't you? Aren't you? Where's\nyour fat reward and ticket home? What\nthe hell is this? It's not supposed to\nend this way!\n\nIt stinks like rotten meat, but looks\nlike the lost Deimos base.  Looks like\nyou're stuck on The Shores of Hell.\nThe only way out is through.\n\nTo continue the DOOM experience, play\nThe Shores of Hell and its amazing\nsequel, Inferno!\n\0";
const E2TEXT_STR: &[u8] = b"You've done it! The hideous cyber-\ndemon lord that ruled the lost Deimos\nmoon base has been slain and you\nare triumphant! But ... where are\nyou? You clamber to the edge of the\nmoon and look down to see the awful\ntruth.\n\nDeimos floats above Hell itself!\nYou've never heard of anyone escaping\nfrom Hell, but you'll make the bastards\nsorry they ever heard of you! Quickly,\nyou rappel down to  the surface of\nHell.\n\nNow, it's on to the final chapter of\nDOOM! -- Inferno.\0";
const E3TEXT_STR: &[u8] = b"The loathsome spiderdemon that\nmasterminded the invasion of the moon\nbases and caused so much death has had\nits ass kicked for all time.\n\nA hidden doorway opens and you enter.\nYou've proven too tough for Hell to\ncontain, and now Hell at last plays\nfair -- for you emerge from the door\nto see the green fields of Earth!\nHome at last.\n\nYou wonder what's been happening on\nEarth while you were battling evil\nunleashed. It's good that no Hell-\nspawn could have come through that\ndoor with you ...\0";
const E4TEXT_STR: &[u8] = b"the spider mastermind must have sent forth\nits legions of hellspawn before your\nfinal confrontation with that terrible\nbeast from hell.  but you stepped\nforward and brought forth eternal\ndamnation and suffering upon the\nhorde as a true hero would in the\nface of something so evil.\n\nbesides, someone was gonna pay for what\nhappened to daisy, your pet rabbit.\n\nbut now, you see spread before you\nmore potential pain and gibbitude as\na nation of demons run amok among\nour cities.\n\nnext stop, hell on earth!\0";

const C1TEXT_STR: &[u8] = b"YOU HAVE ENTERED DEEPLY INTO THE INFESTED\nSTARPORT. BUT SOMETHING IS WRONG. THE\nMONSTERS HAVE BROUGHT THEIR OWN REALITY\nWITH THEM, AND THE STARPORT'S TECHNOLOGY\nIS BEING SUBVERTED BY THEIR PRESENCE.\n\nAHEAD, YOU SEE AN OUTPOST OF HELL, A\nFORTIFIED ZONE. IF YOU CAN GET PAST IT,\nYOU CAN PENETRATE INTO THE HAUNTED HEART\nOF THE STARBASE AND FIND THE CONTROLLING\nSWITCH WHICH HOLDS EARTH'S POPULATION\nHOSTAGE.\0";
const C2TEXT_STR: &[u8] = b"YOU HAVE WON! YOUR VICTORY HAS ENABLED\nHUMANKIND TO EVACUATE EARTH AND ESCAPE\nTHE NIGHTMARE.  NOW YOU ARE THE ONLY\nHUMAN LEFT ON THE FACE OF THE EARTH.\nCANNIBAL MUTATIONS, CARNIVOROUS ALIENS,\nAND EVIL SPIRITS ARE YOUR ONLY NEIGHBORS.\nYOU SIT BACK AND WAIT FOR DEATH, CONTENT\nTHAT YOU HAVE SAVED YOUR SPECIES.\n\nBUT THEN, EARTH CONTROL BEAMS DOWN A\nMESSAGE FROM SPACE: \"SENSORS HAVE LOCATED\nTHE SOURCE OF THE ALIEN INVASION. IF YOU\nGO THERE, YOU MAY BE ABLE TO BLOCK THEIR\nENTRY.  THE ALIEN BASE IS IN THE HEART OF\nYOUR OWN HOME CITY, NOT FAR FROM THE\nSTARPORT.\" SLOWLY AND PAINFULLY YOU GET\nUP AND RETURN TO THE FRAY.\0";
const C3TEXT_STR: &[u8] = b"YOU ARE AT THE CORRUPT HEART OF THE CITY,\nSURROUNDED BY THE CORPSES OF YOUR ENEMIES.\nYOU SEE NO WAY TO DESTROY THE CREATURES'\nENTRANCE ON THIS SIDE, SO YOU CLENCH YOUR\nTEETH AND PLUNGE THROUGH THE PORTAL.\n\nTHERE MUST BE A WAY TO CLOSE IT ON THE\nOTHER SIDE. WHAT DO YOU CARE IF YOU'VE\nGOT TO GO THROUGH HELL TO GET TO IT?\0";
const C4TEXT_STR: &[u8] = b"THE HORRENDOUS VISAGE OF THE BIGGEST\nDEMON YOU'VE EVER SEEN CRUMBLES BEFORE\nYOU, AFTER YOU PUMP YOUR ROCKETS INTO\nHIS EXPOSED BRAIN. THE MONSTER SHRIVELS\nUP AND DIES, ITS THRASHING LIMBS\nDEVASTATING UNTOLD MILES OF HELL'S\nSURFACE.\n\nYOU'VE DONE IT. THE INVASION IS OVER.\nEARTH IS SAVED. HELL IS A WRECK. YOU\nWONDER WHERE BAD FOLKS WILL GO WHEN THEY\nDIE, NOW. WIPING THE SWEAT FROM YOUR\nFOREHEAD YOU BEGIN THE LONG TREK BACK\nHOME. REBUILDING EARTH OUGHT TO BE A\nLOT MORE FUN THAN RUINING IT WAS.\n\0";
const C5TEXT_STR: &[u8] = b"CONGRATULATIONS, YOU'VE FOUND THE SECRET\nLEVEL! LOOKS LIKE IT'S BEEN BUILT BY\nHUMANS, RATHER THAN DEMONS. YOU WONDER\nWHO THE INMATES OF THIS CORNER OF HELL\nWILL BE.\0";
const C6TEXT_STR: &[u8] = b"CONGRATULATIONS, YOU'VE FOUND THE\nSUPER SECRET LEVEL! YOU'D BETTER\nBLAZE THROUGH THIS ONE!\0";

unsafe extern "C" {
    fn S_ChangeMusic(musicnum: c_int, looping: c_int);
    fn S_StartMusic(musicnum: c_int);
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);

    static mut gameaction: gameaction_t;
    static mut gamestate: gamestate_t;
    static mut wipegamestate: gamestate_t;
    static mut viewactive: c_int;
    static mut automapactive: c_int;
    static mut gameepisode: c_int;
    static mut gamemap: c_int;
    static mut players: [player_t; NUM_PLAYERS];
}

#[unsafe(no_mangle)]
pub static mut finalestage: c_int = 0;

#[unsafe(no_mangle)]
pub static mut finalecount: c_int = 0;

#[unsafe(no_mangle)]
pub static mut e1text: *mut c_char = E1TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut e2text: *mut c_char = E2TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut e3text: *mut c_char = E3TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut e4text: *mut c_char = E4TEXT_STR.as_ptr() as *mut c_char;

#[unsafe(no_mangle)]
pub static mut c1text: *mut c_char = C1TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut c2text: *mut c_char = C2TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut c3text: *mut c_char = C3TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut c4text: *mut c_char = C4TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut c5text: *mut c_char = C5TEXT_STR.as_ptr() as *mut c_char;
#[unsafe(no_mangle)]
pub static mut c6text: *mut c_char = C6TEXT_STR.as_ptr() as *mut c_char;

#[unsafe(no_mangle)]
pub static mut finaletext: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut finaleflat: *mut c_char = ptr::null_mut();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_StartFinale() {
    unsafe {
        gameaction = ga_nothing;
        gamestate = GS_FINALE;
        viewactive = 0;
        automapactive = 0;

        match get_gamemode() {
            shareware | registered | retail => {
                S_ChangeMusic(mus_victor as c_int, 1);
                match gameepisode {
                    1 => {
                        finaleflat = b"FLOOR4_8\0".as_ptr() as *mut c_char;
                        finaletext = e1text;
                    }
                    2 => {
                        finaleflat = b"SFLR6_1\0".as_ptr() as *mut c_char;
                        finaletext = e2text;
                    }
                    3 => {
                        finaleflat = b"MFLR8_4\0".as_ptr() as *mut c_char;
                        finaletext = e3text;
                    }
                    4 => {
                        finaleflat = b"MFLR8_3\0".as_ptr() as *mut c_char;
                        finaletext = e4text;
                    }
                    _ => {}
                }
            }
            commercial => {
                S_ChangeMusic(mus_read_m as c_int, 1);
                match gamemap {
                    6 => {
                        finaleflat = b"SLIME16\0".as_ptr() as *mut c_char;
                        finaletext = c1text;
                    }
                    11 => {
                        finaleflat = b"RROCK14\0".as_ptr() as *mut c_char;
                        finaletext = c2text;
                    }
                    20 => {
                        finaleflat = b"RROCK07\0".as_ptr() as *mut c_char;
                        finaletext = c3text;
                    }
                    30 => {
                        finaleflat = b"RROCK17\0".as_ptr() as *mut c_char;
                        finaletext = c4text;
                    }
                    15 => {
                        finaleflat = b"RROCK13\0".as_ptr() as *mut c_char;
                        finaletext = c5text;
                    }
                    31 => {
                        finaleflat = b"RROCK19\0".as_ptr() as *mut c_char;
                        finaletext = c6text;
                    }
                    _ => {}
                }
            }
            _ => {
                S_ChangeMusic(mus_read_m as c_int, 1);
                finaleflat = b"F_SKY1\0".as_ptr() as *mut c_char;
                finaletext = c1text;
            }
        }

        finalestage = 0;
        finalecount = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_Responder(event: *mut event_t) -> c_int {
    if unsafe { finalestage == 2 } {
        return unsafe { F_CastResponder(event) };
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_Ticker() {
unsafe {
if get_gamemode() == commercial && finalecount > 50 {
let players_arr = (&raw const players).as_ref().unwrap();
let any_button = players_arr.iter().any(|p| p.cmd[7] != 0);
if any_button {
if gamemap == 30 {
F_StartCast();
} else {
gameaction = ga_worlddone;
}
}
}
finalecount += 1;
if finalestage == 2 {
F_CastTicker();
return;
}
if get_gamemode() == commercial {
return;
}
let text_len = if !finaletext.is_null() {
CStr::from_ptr(finaletext).to_bytes().len() as c_int
} else {
0
};
if finalestage == 0 && finalecount > text_len * TEXTSPEED + TEXTWAIT {
finalecount = 0;
finalestage = 1;
wipegamestate = -1i32 as gamestate_t;
if gameepisode == 3 {
S_StartMusic(mus_bunny as c_int);
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_TextWrite() {
unsafe {
let src_ptr = W_CacheLumpName(finaleflat, PU_CACHE) as *const u8;
let src = std::slice::from_raw_parts(src_ptr, 64 * 64);
let scr0 = *(&raw const screens as *const *mut u8);
let dst = std::slice::from_raw_parts_mut(scr0, SCR_W * SCR_H);
for y in 0..SCR_H {
let row_src = &src[((y & 63) << 6)..((y & 63) << 6) + 64];
let dst_row = &mut dst[y * SCR_W..(y + 1) * SCR_W];
for chunk in dst_row.chunks_mut(64) {
chunk.copy_from_slice(&row_src[..chunk.len()]);
}
}
V_MarkRect(0, 0, SCREENWIDTH as c_int, SCREENHEIGHT as c_int);
let mut cx = 10;
let mut cy = 10;
let mut count = ((finalecount - 10) / TEXTSPEED).max(0) as usize;
if finaletext.is_null() {
return;
}
let bytes = CStr::from_ptr(finaletext).to_bytes();
let font = (&raw const hu_font).as_ref().unwrap();
let f_start = HU_FONTSTART as c_int;
let f_size = HU_FONTSIZE as c_int;
for &c_raw in bytes.iter().take(count) {
let _ = &mut count;
if c_raw == 0 {
break;
}
if c_raw == b'\n' {
cx = 10;
cy += 11;
continue;
}
let c = (c_raw.to_ascii_uppercase() as c_int) - f_start;
if c < 0 || c > f_size {
cx += 4;
continue;
}
let patch = font[c as usize];
let w = (*patch).width as c_int;
if cx + w > (SCREENWIDTH as c_int) {
break;
}
V_DrawPatch(cx, cy, 0, patch as *mut _);
cx += w;
}
}
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct castinfo_t {
pub name: *const c_char,
pub m_type: mobjtype_t,
}
unsafe impl Sync for castinfo_t {}
pub static CAST_ORDER: [castinfo_t; 18] = [
castinfo_t { name: CC_ZOMBIE.as_ptr() as *const c_char, m_type: MT_POSSESSED },
castinfo_t { name: CC_SHOTGUN.as_ptr() as *const c_char, m_type: MT_SHOTGUY },
castinfo_t { name: CC_HEAVY.as_ptr() as *const c_char, m_type: MT_CHAINGUY },
castinfo_t { name: CC_IMP.as_ptr() as *const c_char, m_type: MT_TROOP },
castinfo_t { name: CC_DEMON.as_ptr() as *const c_char, m_type: MT_SERGEANT },
castinfo_t { name: CC_LOST.as_ptr() as *const c_char, m_type: MT_SKULL },
castinfo_t { name: CC_CACO.as_ptr() as *const c_char, m_type: MT_HEAD },
castinfo_t { name: CC_HELL.as_ptr() as *const c_char, m_type: MT_KNIGHT },
castinfo_t { name: CC_BARON.as_ptr() as *const c_char, m_type: MT_BRUISER },
castinfo_t { name: CC_ARACH.as_ptr() as *const c_char, m_type: MT_BABY },
castinfo_t { name: CC_PAIN.as_ptr() as *const c_char, m_type: MT_PAIN },
castinfo_t { name: CC_REVEN.as_ptr() as *const c_char, m_type: MT_UNDEAD },
castinfo_t { name: CC_MANCU.as_ptr() as *const c_char, m_type: MT_FATSO },
castinfo_t { name: CC_ARCH.as_ptr() as *const c_char, m_type: MT_VILE },
castinfo_t { name: CC_SPIDER.as_ptr() as *const c_char, m_type: MT_SPIDER },
castinfo_t { name: CC_CYBER.as_ptr() as *const c_char, m_type: MT_CYBORG },
castinfo_t { name: CC_HERO.as_ptr() as *const c_char, m_type: MT_PLAYER },
castinfo_t { name: ptr::null(), m_type: 0 },
];
#[unsafe(no_mangle)]
pub static mut castnum: c_int = 0;
#[unsafe(no_mangle)]
pub static mut casttics: c_int = 0;
#[unsafe(no_mangle)]
pub static mut caststate: *mut state_t = ptr::null_mut();
#[unsafe(no_mangle)]
pub static mut castdeath: c_int = 0;
#[unsafe(no_mangle)]
pub static mut castframes: c_int = 0;
#[unsafe(no_mangle)]
pub static mut castonmelee: c_int = 0;
#[unsafe(no_mangle)]
pub static mut castattacking: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_StartCast() {
unsafe {
wipegamestate = -1i32 as gamestate_t;
castnum = 0;
let info_arr = (&raw const mobjinfo).as_ref().unwrap();
let states_arr = (&raw mut states).as_mut().unwrap();
let m_type = CAST_ORDER[0].m_type as usize;
let see_st = info_arr[m_type].seestate as usize;
caststate = &mut states_arr[see_st];
casttics = (*caststate).tics as c_int;
castdeath = 0;
finalestage = 2;
castframes = 0;
castonmelee = 0;
castattacking = 0;
S_ChangeMusic(mus_evil as c_int, 1);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_CastTicker() {
unsafe {
casttics -= 1;
if casttics > 0 {
return;
}
let info_arr = (&raw const mobjinfo).as_ref().unwrap();
let states_arr = (&raw mut states).as_mut().unwrap();
if (*caststate).tics == -1 || (*caststate).nextstate == S_NULL {
castnum += 1;
castdeath = 0;
if CAST_ORDER[castnum as usize].name.is_null() {
castnum = 0;
}
let m_type = CAST_ORDER[castnum as usize].m_type as usize;
let see_snd = info_arr[m_type].seesound;
if see_snd != 0 {
S_StartSound(ptr::null_mut(), see_snd);
}
let see_st = info_arr[m_type].seestate as usize;
caststate = &mut states_arr[see_st];
castframes = 0;
} else {
if caststate == (&mut states_arr[S_PLAY_ATK1 as usize] as *mut state_t) {
castattacking = 0;
castframes = 0;
let m_type = CAST_ORDER[castnum as usize].m_type as usize;
caststate = &mut states_arr[info_arr[m_type].seestate as usize];
casttics = (*caststate).tics as c_int;
if casttics == -1 {
casttics = 15;
}
return;
}
let st = (*caststate).nextstate;
caststate = &mut states_arr[st as usize];
castframes += 1;
let sfx = match st {
S_PLAY_ATK1 => sfx_dshtgn as c_int,
S_POSS_ATK2 => sfx_pistol as c_int,
S_SPOS_ATK2 => sfx_shotgn as c_int,
S_VILE_ATK2 => sfx_vilatk as c_int,
S_SKEL_FIST2 => sfx_skeswg as c_int,
S_SKEL_FIST4 => sfx_skepch as c_int,
S_SKEL_MISS2 => sfx_skeatk as c_int,
S_FATT_ATK8 | S_FATT_ATK5 | S_FATT_ATK2 => sfx_firsht as c_int,
S_CPOS_ATK2 | S_CPOS_ATK3 | S_CPOS_ATK4 => sfx_shotgn as c_int,
S_TROO_ATK3 => sfx_claw as c_int,
S_SARG_ATK2 => sfx_sgtatk as c_int,
S_BOSS_ATK2 | S_BOS2_ATK2 | S_HEAD_ATK2 => sfx_firsht as c_int,
S_SKULL_ATK2 => sfx_sklatk as c_int,
S_SPID_ATK2 | S_SPID_ATK3 => sfx_shotgn as c_int,
S_BSPI_ATK2 => sfx_plasma as c_int,
S_CYBER_ATK2 | S_CYBER_ATK4 | S_CYBER_ATK6 => sfx_rlaunc as c_int,
S_PAIN_ATK3 => sfx_sklatk as c_int,
_ => 0,
};
if sfx != 0 {
S_StartSound(ptr::null_mut(), sfx);
}
}
let m_type = CAST_ORDER[castnum as usize].m_type as usize;
let info = &info_arr[m_type];
if castframes == 12 {
castattacking = 1;
caststate = if castonmelee != 0 {
&mut states_arr[info.meleestate as usize]
} else {
&mut states_arr[info.missilestate as usize]
};
castonmelee ^= 1;
if caststate == (&mut states_arr[S_NULL as usize] as *mut state_t) {
caststate = if castonmelee != 0 {
&mut states_arr[info.meleestate as usize]
} else {
&mut states_arr[info.missilestate as usize]
};
}
}
if castattacking != 0
&& (castframes == 24
|| caststate == (&mut states_arr[info.seestate as usize] as *mut state_t))
{
castattacking = 0;
castframes = 0;
caststate = &mut states_arr[info.seestate as usize];
}
casttics = (*caststate).tics as c_int;
if casttics == -1 {
casttics = 15;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_CastResponder(ev: *mut event_t) -> c_int {
let Some(ev_ref) = (unsafe { ev.as_ref() }) else {
return 0;
};
if ev_ref.type_ != ev_keydown {
return 0;
}
unsafe {
if castdeath != 0 {
return 1;
}
castdeath = 1;
let info_arr = (&raw const mobjinfo).as_ref().unwrap();
let states_arr = (&raw mut states).as_mut().unwrap();
let m_type = CAST_ORDER[castnum as usize].m_type as usize;
let info = &info_arr[m_type];
caststate = &mut states_arr[info.deathstate as usize];
casttics = (*caststate).tics as c_int;
castframes = 0;
castattacking = 0;
if info.deathsound != 0 {
S_StartSound(ptr::null_mut(), info.deathsound);
}
1
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_CastPrint(text: *const c_char) {
if text.is_null() {
return;
}
unsafe {
let font = (&raw const hu_font).as_ref().unwrap();
let f_start = HU_FONTSTART as c_int;
let f_size = HU_FONTSIZE as c_int;
let bytes = CStr::from_ptr(text).to_bytes();
let width: c_int = bytes
.iter()
.map(|&b| {
let c = (b.to_ascii_uppercase() as c_int) - f_start;
if c < 0 || c > f_size {
4
} else {
(*font[c as usize]).width as c_int
}
})
.sum();
let mut cx = 160 - width / 2;
for &b in bytes {
let c = (b.to_ascii_uppercase() as c_int) - f_start;
if c < 0 || c > f_size {
cx += 4;
continue;
}
let p = font[c as usize];
let w = (*p).width as c_int;
V_DrawPatch(cx, 180, 0, p as *mut _);
cx += w;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_CastDrawer() {
unsafe {
let bg = W_CacheLumpName(b"BOSSBACK\0".as_ptr() as *mut c_char, PU_CACHE) as *mut _;
V_DrawPatch(0, 0, 0, bg);
F_CastPrint(CAST_ORDER[castnum as usize].name);
let sprites_base = sprites as *const spritedef_t;
let sprdef = &*sprites_base.add((*caststate).sprite as usize);
let sprframe = &*sprdef
.spriteframes
.add(((*caststate).frame & (FF_FRAMEMASK as i64)) as usize);
let lump = sprframe.lump[0] as c_int;
let flip = sprframe.flip[0];
let patch = W_CacheLumpNum(lump + firstspritelump, PU_CACHE) as *mut _;
if flip != 0 {
V_DrawPatchFlipped(160, 170, 0, patch);
} else {
V_DrawPatch(160, 170, 0, patch);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_DrawPatchCol(x: c_int, patch: *mut patch_t, col: c_int) {
let Some(p) = (unsafe { patch.as_ref() }) else { return };
unsafe {
let ofs = p.columnofs[col as usize] as usize;
let patch_bytes = (patch as *const u8).add(ofs);
let scr0 = *(&raw const screens as *const *mut u8);
let dst = std::slice::from_raw_parts_mut(scr0, SCR_W * SCR_H);
let xu = x as usize;
let mut col_ofs = 0usize;
loop {
let topdelta = *patch_bytes.add(col_ofs);
if topdelta == 0xff {
break;
}
let length = *patch_bytes.add(col_ofs + 1) as usize;
let src_col = std::slice::from_raw_parts(patch_bytes.add(col_ofs + 3), length);
let start_y = topdelta as usize;
for (row, &px) in src_col.iter().enumerate() {
dst[(start_y + row) * SCR_W + xu] = px;
}
col_ofs += length + 4;
}
}
}
static LAST_STAGE: AtomicI32 = AtomicI32::new(0);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_BunnyScroll() {
unsafe {
let p1 = W_CacheLumpName(b"PFUB2\0".as_ptr() as *mut c_char, PU_LEVEL) as *mut patch_t;
let p2 = W_CacheLumpName(b"PFUB1\0".as_ptr() as *mut c_char, PU_LEVEL) as *mut patch_t;
let scr_w = SCREENWIDTH as c_int;
let scr_h = SCREENHEIGHT as c_int;
V_MarkRect(0, 0, scr_w, scr_h);
let scrolled = (320 - (finalecount - 230) / 2).clamp(0, 320);
for x in 0..scr_w {
if x + scrolled < 320 {
F_DrawPatchCol(x, p1, x + scrolled);
} else {
F_DrawPatchCol(x, p2, x + scrolled - 320);
}
}
if finalecount < 1130 {
return;
}
let cx = (scr_w - 13 * 8) / 2;
let cy = (scr_h - 8 * 8) / 2;
if finalecount < 1180 {
let end0 = W_CacheLumpName(b"END0\0".as_ptr() as *mut c_char, PU_CACHE) as *mut _;
V_DrawPatch(cx, cy, 0, end0);
LAST_STAGE.store(0, Ordering::Relaxed);
return;
}
let stage = ((finalecount - 1180) / 5).min(6);
let prev = LAST_STAGE.load(Ordering::Relaxed);
if stage > prev {
S_StartSound(ptr::null_mut(), sfx_pistol as c_int);
LAST_STAGE.store(stage, Ordering::Relaxed);
}
let name = format!("END{}\0", stage);
let end_patch = W_CacheLumpName(name.as_ptr() as *mut c_char, PU_CACHE) as *mut _;
V_DrawPatch(cx, cy, 0, end_patch);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn F_Drawer() {
unsafe {
if finalestage == 2 {
F_CastDrawer();
return;
}
if finalestage == 0 {
F_TextWrite();
} else {
match gameepisode {
1 => {
let lump = if get_gamemode() == retail {
b"CREDIT\0".as_ptr()
} else {
b"HELP2\0".as_ptr()
};
let p = W_CacheLumpName(lump as *mut c_char, PU_CACHE) as *mut _;
V_DrawPatch(0, 0, 0, p);
}
2 => {
let p = W_CacheLumpName(b"VICTORY2\0".as_ptr() as *mut c_char, PU_CACHE) as *mut _;
V_DrawPatch(0, 0, 0, p);
}
3 => {
F_BunnyScroll();
}
4 => {
let p = W_CacheLumpName(b"ENDPIC\0".as_ptr() as *mut c_char, PU_CACHE) as *mut _;
V_DrawPatch(0, 0, 0, p);
}
_ => {}
}
}
}
}
