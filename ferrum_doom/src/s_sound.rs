#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{CStr, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

use crate::m_fixed::{FRACBITS, fixed_mul};
use crate::m_random::m_random;
use crate::r_main::R_PointToAngle2;
use crate::sounds::{S_music, S_sfx, musicinfo_t, sfxinfo_t};
use crate::tables::{ANGLETOFINESHIFT, finesine};
use crate::v_video::doom_error;
use crate::w_wad::{W_CacheLumpNum, W_GetNumForName};
use crate::z_zone::{PU_CACHE, PU_MUSIC, PU_STATIC, Z_ChangeTag2 as Z_ChangeTag, Z_Malloc};

const NUM_PLAYERS: usize = MAXPLAYERS as usize;

const S_CLIPPING_DIST: c_int = 1200 * 0x10000;
const S_CLOSE_DIST: c_int = 160 * 0x10000;
const S_ATTENUATOR: c_int = (S_CLIPPING_DIST - S_CLOSE_DIST) >> FRACBITS;

const NORM_PITCH: c_int = 128;
const NORM_PRIORITY: c_int = 64;
const NORM_SEP: c_int = 128;
const S_STEREO_SWING: c_int = 96 * 0x10000;

// Музыкальные треки
const mus_None: c_int = 0;
const mus_e1m1: c_int = 1;
const mus_e1m5: c_int = 5;
const mus_e1m9: c_int = 9;
const mus_e2m4: c_int = 13;
const mus_e2m5: c_int = 14;
const mus_e2m6: c_int = 15;
const mus_e2m7: c_int = 16;
const mus_e3m2: c_int = 20;
const mus_e3m3: c_int = 21;
const mus_e3m4: c_int = 22;
const mus_runnin: c_int = 33;
const NUMMUSIC: c_int = 68;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct channel_t {
    pub sfxinfo: *mut sfxinfo_t,
    pub origin: *mut c_void,
    pub handle: c_int,
}

unsafe extern "C" {
    fn I_SetChannels();
    fn I_GetSfxLumpNum(sfxinfo: *mut sfxinfo_t) -> c_int;
    fn I_StartSound(id: c_int, vol: c_int, sep: c_int, pitch: c_int, priority: c_int) -> c_int;
    fn I_StopSound(handle: c_int);
    fn I_SoundIsPlaying(handle: c_int) -> c_int;
    fn I_UpdateSoundParams(handle: c_int, vol: c_int, sep: c_int, pitch: c_int);

    fn I_SetMusicVolume(volume: c_int);
    fn I_PauseSong(handle: c_int);
    fn I_ResumeSong(handle: c_int);
    fn I_RegisterSong(data: *mut c_void) -> c_int;
    fn I_PlaySong(handle: c_int, looping: c_int);
    fn I_StopSong(handle: c_int);
    fn I_UnRegisterSong(handle: c_int);

    static mut gamemode: GameMode_t;
    static mut gameepisode: c_int;
    static mut gamemap: c_int;
    static mut consoleplayer: c_int;
    static mut players: [player_t; NUM_PLAYERS];
}

// --- Глобальные переменные s_sound.c ---
#[unsafe(no_mangle)]
pub static snd_prefixen: [c_char; 12] = [
    b'P' as c_char,
    b'P' as c_char,
    b'A' as c_char,
    b'S' as c_char,
    b'S' as c_char,
    b'S' as c_char,
    b'M' as c_char,
    b'M' as c_char,
    b'M' as c_char,
    b'S' as c_char,
    b'S' as c_char,
    b'S' as c_char,
];

static mut channels: *mut channel_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut snd_SfxVolume: c_int = 15;

#[unsafe(no_mangle)]
pub static mut snd_MusicVolume: c_int = 15;

static mut mus_paused: c_int = 0;
static mut mus_playing: *mut musicinfo_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut numChannels: c_int = 8;

static mut nextcleanup: c_int = 0;

// --- S_Init ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_Init(sfx_volume: c_int, music_volume: c_int) {
    unsafe {
        eprintln!("S_Init: default sfx volume {}", sfx_volume);

        I_SetChannels();
        S_SetSfxVolume(sfx_volume);
        S_SetMusicVolume(music_volume);

        let n_ch = numChannels as usize;
        channels = Z_Malloc(
            (n_ch * size_of::<channel_t>()) as c_int,
            PU_STATIC,
            ptr::null_mut(),
        ) as *mut channel_t;

        for i in 0..n_ch {
            (*channels.add(i)).sfxinfo = ptr::null_mut();
        }

        mus_paused = 0;

        let sfx_base = (&raw mut S_sfx) as *mut sfxinfo_t;
        for i in 1..(NUMSFX as usize) {
            let s = sfx_base.add(i);
            (*s).lumpnum = -1;
            (*s).usefulness = -1;
        }
    }
}

// --- S_Start ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_Start() {
    unsafe {
        for cnum in 0..numChannels {
            if !(*channels.add(cnum as usize)).sfxinfo.is_null() {
                S_StopChannel(cnum);
            }
        }

        mus_paused = 0;

        let mnum = if gamemode == commercial {
            mus_runnin + gamemap - 1
        } else {
            let spmus: [c_int; 9] = [
                mus_e3m4,
                mus_e3m2,
                mus_e3m3,
                mus_e1m5,
                mus_e2m7,
                mus_e2m4,
                mus_e2m6,
                mus_e2m5,
                mus_e1m9,
            ];

            if gameepisode < 4 {
                mus_e1m1 + (gameepisode - 1) * 9 + gamemap - 1
            } else {
                let idx = (gamemap - 1).clamp(0, 8) as usize;
                spmus[idx]
            }
        };

        S_ChangeMusic(mnum, 1);
        nextcleanup = 15;
    }
}

// --- S_StartSoundAtVolume / S_StartSound ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StartSoundAtVolume(origin_p: *mut c_void, sfx_id: c_int, mut volume: c_int) {
    unsafe {
        let origin = origin_p as *mut mobj_t;

        if sfx_id < 1 || sfx_id > (NUMSFX as c_int) {
            doom_error(&format!("Bad sfx #: {}", sfx_id));
        }

        let sfx_base = (&raw mut S_sfx) as *mut sfxinfo_t;
        let sfx = sfx_base.add(sfx_id as usize);

        let mut pitch: c_int;
        let priority: c_int;

        if !(*sfx).link.is_null() {
            pitch = (*sfx).pitch;
            priority = (*sfx).priority;
            volume += (*sfx).volume;

            if volume < 1 {
                return;
            }
            if volume > snd_SfxVolume {
                volume = snd_SfxVolume;
            }
        } else {
            pitch = NORM_PITCH;
            priority = NORM_PRIORITY;
        }

        let players_base = (&raw const players) as *const player_t;
        let listener = (*players_base.add(consoleplayer as usize)).mo as *mut mobj_t;

        let mut sep: c_int = NORM_SEP;

        if !origin.is_null() && origin != listener {
            let rc = S_AdjustSoundParams(listener, origin, &raw mut volume, &raw mut sep, &raw mut pitch);
            if (*origin).x == (*listener).x && (*origin).y == (*listener).y {
                sep = NORM_SEP;
            }
            if rc == 0 {
                return;
            }
        }

        if sfx_id >= (sfx_sawup as c_int) && sfx_id <= (sfx_sawhit as c_int) {
            pitch += 8 - (m_random() & 15);
            pitch = pitch.clamp(0, 255);
        } else if sfx_id != (sfx_itemup as c_int) && sfx_id != (sfx_tink as c_int) {
            pitch += 16 - (m_random() & 31);
            pitch = pitch.clamp(0, 255);
        }

        S_StopSound(origin_p);

        let cnum = S_getChannel(origin_p, sfx);
        if cnum < 0 {
            return;
        }

        if (*sfx).lumpnum < 0 {
            (*sfx).lumpnum = I_GetSfxLumpNum(sfx);
        }

        if (*sfx).data.is_null() {
            eprintln!("S_StartSoundAtVolume: 16bit and not pre-cached - wtf?");
        }

        if (*sfx).usefulness < 0 {
            (*sfx).usefulness = 1;
        } else {
            (*sfx).usefulness += 1;
        }

        (*channels.add(cnum as usize)).handle = I_StartSound(sfx_id, volume, sep, pitch, priority);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StartSound(origin: *mut c_void, sfx_id: c_int) {
    unsafe {
        S_StartSoundAtVolume(origin, sfx_id, snd_SfxVolume);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StopSound(origin: *mut c_void) {
    unsafe {
        for cnum in 0..numChannels {
            let c = channels.add(cnum as usize);
            if !(*c).sfxinfo.is_null() && (*c).origin == origin {
                S_StopChannel(cnum);
                break;
            }
        }
    }
}

// --- Pause / Resume Music ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_PauseSound() {
    unsafe {
        if !mus_playing.is_null() && mus_paused == 0 {
            I_PauseSong((*mus_playing).handle);
            mus_paused = 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_ResumeSound() {
    unsafe {
        if !mus_playing.is_null() && mus_paused != 0 {
            I_ResumeSong((*mus_playing).handle);
            mus_paused = 0;
        }
    }
}

// --- S_UpdateSounds ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_UpdateSounds(listener_p: *mut c_void) {
    unsafe {
        let listener = listener_p as *mut mobj_t;

        for cnum in 0..numChannels {
            let c = channels.add(cnum as usize);
            let sfx = (*c).sfxinfo;

            if !sfx.is_null() {
                if I_SoundIsPlaying((*c).handle) != 0 {
                    let mut volume = snd_SfxVolume;
                    let mut pitch = NORM_PITCH;
                    let mut sep = NORM_SEP;

                    if !(*sfx).link.is_null() {
                        pitch = (*sfx).pitch;
                        volume += (*sfx).volume;
                        if volume < 1 {
                            S_StopChannel(cnum);
                            continue;
                        } else if volume > snd_SfxVolume {
                            volume = snd_SfxVolume;
                        }
                    }

                    if !(*c).origin.is_null() && listener_p != (*c).origin {
                        let audible = S_AdjustSoundParams(
                            listener,
                            (*c).origin as *mut mobj_t,
                            &raw mut volume,
                            &raw mut sep,
                            &raw mut pitch,
                        );

                        if audible == 0 {
                            S_StopChannel(cnum);
                        } else {
                            I_UpdateSoundParams((*c).handle, volume, sep, pitch);
                        }
                    }
} else {
S_StopChannel(cnum);
}
}
}
}
}
// --- Volume / Music ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_SetMusicVolume(volume: c_int) {
unsafe {
if !(0..=127).contains(&volume) {
doom_error(&format!("Attempt to set music volume at {}", volume));
}
I_SetMusicVolume(127);
I_SetMusicVolume(volume);
snd_MusicVolume = volume;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_SetSfxVolume(volume: c_int) {
unsafe {
if !(0..=127).contains(&volume) {
doom_error(&format!("Attempt to set sfx volume at {}", volume));
}
snd_SfxVolume = volume;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StartMusic(m_id: c_int) {
unsafe {
S_ChangeMusic(m_id, 0);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_ChangeMusic(musicnum: c_int, looping: c_int) {
unsafe {
if musicnum <= mus_None || musicnum >= NUMMUSIC {
doom_error(&format!("Bad music number {}", musicnum));
}
let music_base = (&raw mut S_music) as *mut musicinfo_t;
let music = music_base.add(musicnum as usize);
if mus_playing == music {
return;
}
S_StopMusic();
if (*music).lumpnum == 0 {
let m_name = CStr::from_ptr((*music).name).to_str().unwrap_or("");
let lump_name = format!("d_{}\0", m_name);
(*music).lumpnum = W_GetNumForName(lump_name.as_ptr() as *mut c_char);
}
(*music).data = W_CacheLumpNum((*music).lumpnum, PU_MUSIC);
(*music).handle = I_RegisterSong((*music).data);
I_PlaySong((*music).handle, looping);
mus_playing = music;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StopMusic() {
unsafe {
if !mus_playing.is_null() {
if mus_paused != 0 {
I_ResumeSong((*mus_playing).handle);
}
I_StopSong((*mus_playing).handle);
I_UnRegisterSong((*mus_playing).handle);
Z_ChangeTag((*mus_playing).data, PU_CACHE);
(*mus_playing).data = ptr::null_mut();
mus_playing = ptr::null_mut();
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StopChannel(cnum: c_int) {
unsafe {
let c = channels.add(cnum as usize);
if !(*c).sfxinfo.is_null() {
if I_SoundIsPlaying((*c).handle) != 0 {
I_StopSound((*c).handle);
}
for i in 0..numChannels {
if cnum != i && (*c).sfxinfo == (*channels.add(i as usize)).sfxinfo {
break;
}
}
(*(*c).sfxinfo).usefulness -= 1;
(*c).sfxinfo = ptr::null_mut();
}
}
}
// --- S_AdjustSoundParams ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_AdjustSoundParams(
listener: *mut mobj_t,
source: *mut mobj_t,
vol: *mut c_int,
sep: *mut c_int,
_pitch: *mut c_int,
) -> c_int {
unsafe {
let adx = ((*listener).x - (*source).x).abs();
let ady = ((*listener).y - (*source).y).abs();
let mut approx_dist = adx + ady - ((if adx < ady { adx } else { ady }) >> 1);
if gamemap != 8 && approx_dist > S_CLIPPING_DIST {
return 0;
}
let mut angle = R_PointToAngle2((*listener).x, (*listener).y, (*source).x, (*source).y);
if angle > (*listener).angle {
angle = angle.wrapping_sub((*listener).angle);
} else {
angle = angle.wrapping_add(0xffff_ffffu32.wrapping_sub((*listener).angle));
}
angle >>= ANGLETOFINESHIFT;
let sin_val = *finesine.as_ptr().add(angle as usize);
*sep = 128 - (fixed_mul(S_STEREO_SWING, sin_val) >> FRACBITS);
if approx_dist < S_CLOSE_DIST {
*vol = snd_SfxVolume;
} else if gamemap == 8 {
if approx_dist > S_CLIPPING_DIST {
approx_dist = S_CLIPPING_DIST;
}
*vol = 15 + ((snd_SfxVolume - 15) * ((S_CLIPPING_DIST - approx_dist) >> FRACBITS)) / S_ATTENUATOR;
} else {
*vol = (snd_SfxVolume * ((S_CLIPPING_DIST - approx_dist) >> FRACBITS)) / S_ATTENUATOR;
}
(*vol > 0) as c_int
}
}
// --- S_getChannel ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_getChannel(origin: *mut c_void, sfxinfo: *mut sfxinfo_t) -> c_int {
unsafe {
let mut cnum = 0;
while cnum < numChannels {
let c = channels.add(cnum as usize);
if (*c).sfxinfo.is_null() {
break;
} else if !origin.is_null() && (*c).origin == origin {
S_StopChannel(cnum);
break;
}
cnum += 1;
}
if cnum == numChannels {
cnum = 0;
while cnum < numChannels {
if (*(*channels.add(cnum as usize)).sfxinfo).priority >= (*sfxinfo).priority {
break;
}
cnum += 1;
}
if cnum == numChannels {
return -1;
} else {
S_StopChannel(cnum);
}
}
let c = channels.add(cnum as usize);
(*c).sfxinfo = sfxinfo;
(*c).origin = origin;
cnum
}
}