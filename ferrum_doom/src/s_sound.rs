#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::doomstat::get_gamemode;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_void};
use std::sync::{Mutex, OnceLock};

use crate::i_sound::{
    I_PauseSong, I_PlaySong, I_RegisterSong, I_ResumeSong, I_SetChannels, I_SetMusicVolume,
    I_SoundIsPlaying, I_StartSound, I_StopSong, I_StopSound, I_UnRegisterSong,
    I_UpdateSoundParams, get_sfx_lump_num,
};
use crate::m_fixed::{FRACBITS, fixed_mul};
use crate::m_random::m_random;
use crate::r_main::R_PointToAngle2;
use crate::sounds::{S_MUSIC, S_SFX};
use crate::tables::{ANGLETOFINESHIFT, finesine};
use crate::v_video::doom_error;
use crate::w_wad::{W_CacheLumpNum, w_get_num_for_name};
use crate::z_zone::PU_MUSIC;

const NUM_PLAYERS: usize = MAXPLAYERS as usize;

const S_CLIPPING_DIST: c_int = 1200 * 0x10000;
const S_CLOSE_DIST: c_int = 160 * 0x10000;
const S_ATTENUATOR: c_int = (S_CLIPPING_DIST - S_CLOSE_DIST) >> FRACBITS;

const NORM_PITCH: c_int = 128;
const NORM_PRIORITY: c_int = 64;
const NORM_SEP: c_int = 128;
const S_STEREO_SWING: c_int = 96 * 0x10000;

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

#[derive(Copy, Clone, Default)]
struct ChannelSlot {
    sfx_id: Option<usize>,
    origin_addr: usize,
    handle: c_int,
}

struct SoundState {
    channels: Vec<ChannelSlot>,
    mus_paused: bool,
    mus_playing_idx: Option<usize>,
    next_cleanup: c_int,
}

impl SoundState {
    const fn new() -> Self {
        Self {
            channels: Vec::new(),
            mus_paused: false,
            mus_playing_idx: None,
            next_cleanup: 0,
        }
    }

    fn stop_channel(&mut self, cnum: usize) {
        let Some(ch) = self.channels.get_mut(cnum) else { return };
        if let Some(sfx_idx) = ch.sfx_id.take() {
            if I_SoundIsPlaying(ch.handle) != 0 {
                I_StopSound(ch.handle);
            }
            if let Ok(mut sfx_table) = S_SFX.lock() {
                sfx_table[sfx_idx].usefulness -= 1;
            }
        }
    }

    fn allocate_channel(&mut self, origin_addr: usize, sfx_idx: usize) -> Option<usize> {
        let n_ch = self.channels.len();
        if n_ch == 0 {
            return None;
        }

        for cnum in 0..n_ch {
            if self.channels[cnum].sfx_id.is_none() {
                self.channels[cnum].sfx_id = Some(sfx_idx);
                self.channels[cnum].origin_addr = origin_addr;
                return Some(cnum);
            }
            if origin_addr != 0 && self.channels[cnum].origin_addr == origin_addr {
                self.stop_channel(cnum);
                self.channels[cnum].sfx_id = Some(sfx_idx);
                self.channels[cnum].origin_addr = origin_addr;
                return Some(cnum);
            }
        }

        let target_prio = S_SFX.lock().unwrap()[sfx_idx].priority;
        for cnum in 0..n_ch {
            if let Some(ch_sfx) = self.channels[cnum].sfx_id {
                if S_SFX.lock().unwrap()[ch_sfx].priority >= target_prio {
                    self.stop_channel(cnum);
                    self.channels[cnum].sfx_id = Some(sfx_idx);
                    self.channels[cnum].origin_addr = origin_addr;
                    return Some(cnum);
                }
            }
        }

        None
    }
}

static SOUND_STATE: OnceLock<Mutex<SoundState>> = OnceLock::new();

fn sound_state() -> &'static Mutex<SoundState> {
    SOUND_STATE.get_or_init(|| Mutex::new(SoundState::new()))
}

unsafe extern "C" {
    static mut gameepisode: c_int;
    static mut gamemap: c_int;
    static mut consoleplayer: c_int;
    static mut players: [player_t; NUM_PLAYERS];
}

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

#[unsafe(no_mangle)]
pub static mut snd_SfxVolume: c_int = 15;

#[unsafe(no_mangle)]
pub static mut snd_MusicVolume: c_int = 15;

#[unsafe(no_mangle)]
pub static mut numChannels: c_int = 8;

pub fn s_adjust_sound_params(
    listener: &mobj_t,
    source: &mobj_t,
    base_vol: c_int,
    cur_map: c_int,
) -> Option<(c_int, c_int)> {
    let adx = (listener.x - source.x).abs();
    let ady = (listener.y - source.y).abs();
    let mut approx_dist = adx + ady - (adx.min(ady) >> 1);

    if cur_map != 8 && approx_dist > S_CLIPPING_DIST {
        return None;
    }

    let mut angle = unsafe { R_PointToAngle2(listener.x, listener.y, source.x, source.y) };
    if angle > listener.angle {
        angle = angle.wrapping_sub(listener.angle);
    } else {
        angle = angle.wrapping_add(0xffff_ffffu32.wrapping_sub(listener.angle));
    }

    angle >>= ANGLETOFINESHIFT;
    let sin_val = finesine[angle as usize];
    let sep = 128 - (fixed_mul(S_STEREO_SWING, sin_val) >> FRACBITS);

    let vol = if approx_dist < S_CLOSE_DIST {
        base_vol
    } else if cur_map == 8 {
        if approx_dist > S_CLIPPING_DIST {
            approx_dist = S_CLIPPING_DIST;
        }
        15 + ((base_vol - 15) * ((S_CLIPPING_DIST - approx_dist) >> FRACBITS)) / S_ATTENUATOR
    } else {
        (base_vol * ((S_CLIPPING_DIST - approx_dist) >> FRACBITS)) / S_ATTENUATOR
    };

    if vol > 0 {
        Some((vol, sep))
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_Init(sfx_volume: c_int, music_volume: c_int) {
    eprintln!("S_Init: default sfx volume {}", sfx_volume);

    I_SetChannels();
    S_SetSfxVolume(sfx_volume);
    S_SetMusicVolume(music_volume);

    let n_ch = unsafe { numChannels.max(0) as usize };
    {
        let mut st = sound_state().lock().unwrap();
        st.channels = vec![ChannelSlot::default(); n_ch];
        st.mus_paused = false;
    }

    if let Ok(mut sfx_table) = S_SFX.lock() {
        for s in sfx_table.iter_mut().skip(1) {
            s.lumpnum = -1;
            s.usefulness = -1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_Start() {
    {
        let mut st = sound_state().lock().unwrap();
        for cnum in 0..st.channels.len() {
            if st.channels[cnum].sfx_id.is_some() {
                st.stop_channel(cnum);
            }
        }
        st.mus_paused = false;
        st.next_cleanup = 15;
    }

    let (ep, map) = unsafe { (gameepisode, gamemap) };
    let mnum = if get_gamemode() == commercial {
        mus_runnin + map - 1
    } else {
        let spmus: [c_int; 9] = [
            mus_e3m4, mus_e3m2, mus_e3m3, mus_e1m5, mus_e2m7, mus_e2m4, mus_e2m6, mus_e2m5,
            mus_e1m9,
        ];

        if ep < 4 {
            mus_e1m1 + (ep - 1) * 9 + map - 1
        } else {
            let idx = (map - 1).clamp(0, 8) as usize;
            spmus[idx]
        }
    };

    S_ChangeMusic(mnum, 1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StartSoundAtVolume(origin_p: *mut c_void, sfx_id: c_int, mut volume: c_int) {
    if sfx_id < 1 || sfx_id > (NUMSFX as c_int) {
        doom_error(&format!("Bad sfx #: {}", sfx_id));
    }

    let sfx_idx = sfx_id as usize;
    let base_sfx_vol = unsafe { snd_SfxVolume };

    let (has_link, sfx_pitch, sfx_prio, sfx_vol) = {
        let sfx_table = S_SFX.lock().unwrap();
        let s = &sfx_table[sfx_idx];
        (s.link_id.is_some(), s.pitch, s.priority, s.volume)
    };

    let mut pitch: c_int;
    let priority: c_int;

    if has_link {
        pitch = sfx_pitch;
        priority = sfx_prio;
        volume += sfx_vol;

        if volume < 1 {
            return;
        }
        if volume > base_sfx_vol {
            volume = base_sfx_vol;
        }
    } else {
        pitch = NORM_PITCH;
        priority = NORM_PRIORITY;
    }

    let listener_opt = unsafe {
        let players_arr = (&raw const players).as_ref().unwrap();
        (players_arr[consoleplayer as usize].mo as *const mobj_t).as_ref()
    };
    let origin_opt = unsafe { (origin_p as *const mobj_t).as_ref() };

    let mut sep: c_int = NORM_SEP;

    if let (Some(listener), Some(origin)) = (listener_opt, origin_opt) {
        if !std::ptr::eq(origin, listener) {
            let cur_map = unsafe { gamemap };
            let Some((adj_vol, adj_sep)) = s_adjust_sound_params(listener, origin, volume, cur_map)
            else {
                return;
            };
            volume = adj_vol;
            sep = if origin.x == listener.x && origin.y == listener.y {
                NORM_SEP
            } else {
                adj_sep
            };
        }
    }

    if sfx_id >= (sfx_sawup as c_int) && sfx_id <= (sfx_sawhit as c_int) {
        pitch = (pitch + 8 - (m_random() & 15)).clamp(0, 255);
    } else if sfx_id != (sfx_itemup as c_int) && sfx_id != (sfx_tink as c_int) {
        pitch = (pitch + 16 - (m_random() & 31)).clamp(0, 255);
    }

    S_StopSound(origin_p);

    let mut st = sound_state().lock().unwrap();
    let Some(cnum) = st.allocate_channel(origin_p as usize, sfx_idx) else {
        return;
    };

    {
        let mut sfx_table = S_SFX.lock().unwrap();
        let s = &mut sfx_table[sfx_idx];
        if s.lumpnum < 0 {
            s.lumpnum = get_sfx_lump_num(s.name);
        }
        if s.usefulness < 0 {
            s.usefulness = 1;
        } else {
            s.usefulness += 1;
        }
    }

    st.channels[cnum].handle = I_StartSound(sfx_id, volume, sep, pitch, priority);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StartSound(origin: *mut c_void, sfx_id: c_int) {
unsafe {
S_StartSoundAtVolume(origin, sfx_id, snd_SfxVolume);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_StopSound(origin: *mut c_void) {
let origin_addr = origin as usize;
let mut st = sound_state().lock().unwrap();
for cnum in 0..st.channels.len() {
if st.channels[cnum].sfx_id.is_some() && st.channels[cnum].origin_addr == origin_addr {
st.stop_channel(cnum);
break;
}
}
}
#[unsafe(no_mangle)]
pub extern "C" fn S_PauseSound() {
let mut st = sound_state().lock().unwrap();
if let Some(m_idx) = st.mus_playing_idx {
if !st.mus_paused {
let handle = S_MUSIC.lock().unwrap()[m_idx].handle;
I_PauseSong(handle);
st.mus_paused = true;
}
}
}
#[unsafe(no_mangle)]
pub extern "C" fn S_ResumeSound() {
let mut st = sound_state().lock().unwrap();
if let Some(m_idx) = st.mus_playing_idx {
if st.mus_paused {
let handle = S_MUSIC.lock().unwrap()[m_idx].handle;
I_ResumeSong(handle);
st.mus_paused = false;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_UpdateSounds(listener_p: *mut c_void) {
let listener_opt = unsafe { (listener_p as *const mobj_t).as_ref() };
let listener_addr = listener_p as usize;
let base_sfx_vol = unsafe { snd_SfxVolume };
let cur_map = unsafe { gamemap };
let mut st = sound_state().lock().unwrap();
for cnum in 0..st.channels.len() {
let Some(sfx_idx) = st.channels[cnum].sfx_id else { continue };
let handle = st.channels[cnum].handle;
let origin_addr = st.channels[cnum].origin_addr;
if I_SoundIsPlaying(handle) != 0 {
let mut volume = base_sfx_vol;
let mut pitch = NORM_PITCH;
let mut sep = NORM_SEP;
let (has_link, sfx_pitch, sfx_vol) = {
let sfx_table = S_SFX.lock().unwrap();
let s = &sfx_table[sfx_idx];
(s.link_id.is_some(), s.pitch, s.volume)
};
if has_link {
pitch = sfx_pitch;
volume += sfx_vol;
if volume < 1 {
st.stop_channel(cnum);
continue;
} else if volume > base_sfx_vol {
volume = base_sfx_vol;
}
}
if origin_addr != 0 && listener_addr != origin_addr {
let origin_opt = unsafe { (origin_addr as *const mobj_t).as_ref() };
if let (Some(listener), Some(origin)) = (listener_opt, origin_opt) {
if let Some((adj_vol, adj_sep)) =
s_adjust_sound_params(listener, origin, volume, cur_map)
{
volume = adj_vol;
sep = adj_sep;
I_UpdateSoundParams(handle, volume, sep, pitch);
} else {
st.stop_channel(cnum);
}
}
}
} else {
st.stop_channel(cnum);
}
}
}
#[unsafe(no_mangle)]
pub extern "C" fn S_SetMusicVolume(volume: c_int) {
if !(0..=127).contains(&volume) {
doom_error(&format!("Attempt to set music volume at {}", volume));
}
I_SetMusicVolume(127);
I_SetMusicVolume(volume);
unsafe {
snd_MusicVolume = volume;
}
}
#[unsafe(no_mangle)]
pub extern "C" fn S_SetSfxVolume(volume: c_int) {
if !(0..=127).contains(&volume) {
doom_error(&format!("Attempt to set sfx volume at {}", volume));
}
unsafe {
snd_SfxVolume = volume;
}
}
#[unsafe(no_mangle)]
pub extern "C" fn S_StartMusic(m_id: c_int) {
S_ChangeMusic(m_id, 0);
}
#[unsafe(no_mangle)]
pub extern "C" fn S_ChangeMusic(musicnum: c_int, looping: c_int) {
if musicnum <= mus_None || musicnum >= NUMMUSIC {
doom_error(&format!("Bad music number {}", musicnum));
}
let m_idx = musicnum as usize;
if sound_state().lock().unwrap().mus_playing_idx == Some(m_idx) {
return;
}
S_StopMusic();
let lumpnum = {
let mut mus_table = S_MUSIC.lock().unwrap();
let music = &mut mus_table[m_idx];
if music.lumpnum == 0 {
let m_name = music.name.unwrap_or("");
let lump_name = format!("d_{}", m_name);
music.lumpnum = w_get_num_for_name(&lump_name);
}
music.lumpnum
};
let data = W_CacheLumpNum(lumpnum, PU_MUSIC);
let handle = unsafe { I_RegisterSong(data) };
S_MUSIC.lock().unwrap()[m_idx].handle = handle;
I_PlaySong(handle, looping);
sound_state().lock().unwrap().mus_playing_idx = Some(m_idx);
}
#[unsafe(no_mangle)]
pub extern "C" fn S_StopMusic() {
let mut st = sound_state().lock().unwrap();
if let Some(m_idx) = st.mus_playing_idx.take() {
let handle = S_MUSIC.lock().unwrap()[m_idx].handle;
if st.mus_paused {
I_ResumeSong(handle);
}
I_StopSong(handle);
I_UnRegisterSong(handle);
}
}
#[unsafe(no_mangle)]
pub extern "C" fn S_StopChannel(cnum: c_int) {
if cnum >= 0 {
sound_state().lock().unwrap().stop_channel(cnum as usize);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn S_AdjustSoundParams(
listener: *mut mobj_t,
source: *mut mobj_t,
vol: *mut c_int,
sep: *mut c_int,
_pitch: *mut c_int,
) -> c_int {
let (Some(l), Some(s), Some(v_out), Some(s_out)) = (
unsafe { listener.as_ref() },
unsafe { source.as_ref() },
unsafe { vol.as_mut() },
unsafe { sep.as_mut() },
) else {
return 0;
};
match s_adjust_sound_params(l, s, unsafe { snd_SfxVolume }, unsafe { gamemap }) {
Some((new_vol, new_sep)) => {
*v_out = new_vol;
*s_out = new_sep;
1
}
None => 0,
}
}
