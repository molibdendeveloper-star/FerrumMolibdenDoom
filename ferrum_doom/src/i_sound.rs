#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs::File;
use std::io::Cursor;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

use crate::g_game::gametic;
use crate::mus2mid::convert_mus_to_mid;
use crate::s_sound::{snd_MusicVolume, snd_SfxVolume};
use crate::sounds::{S_sfx, sfxinfo_t};
use crate::w_wad::{W_CacheLumpNum, W_CheckNumForName, W_GetNumForName, W_LumpLength};
use crate::z_zone::{PU_STATIC, Z_Free};

const NUM_CHANNELS: usize = 8;
const SAMPLERATE: i32 = 44100;
const NUM_SFX: usize = NUMSFX as usize;

const SDL_AUDIO_S16: u32 = 0x8010;
const SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK: u32 = 0xFFFF_FFFF;

#[repr(C)]
pub struct SDL_AudioStream {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SDL_AudioSpec {
    pub format: u32,
    pub channels: c_int,
    pub freq: c_int,
}

type SDL_AudioStreamCallback = unsafe extern "C" fn(
    userdata: *mut c_void,
    stream: *mut SDL_AudioStream,
    additional_amount: c_int,
    total_amount: c_int,
);

unsafe extern "C" {
    fn SDL_InitSubSystem(flags: u32) -> bool;
    fn SDL_GetError() -> *const c_char;
    fn SDL_OpenAudioDeviceStream(
        devid: u32,
        spec: *const SDL_AudioSpec,
        callback: Option<SDL_AudioStreamCallback>,
        userdata: *mut c_void,
    ) -> *mut SDL_AudioStream;
    fn SDL_SetAudioStreamGetCallback(
        stream: *mut SDL_AudioStream,
        callback: Option<SDL_AudioStreamCallback>,
        userdata: *mut c_void,
    ) -> bool;
    fn SDL_ResumeAudioStreamDevice(stream: *mut SDL_AudioStream) -> bool;
    fn SDL_PutAudioStreamData(stream: *mut SDL_AudioStream, buf: *const c_void, len: c_int) -> bool;
    fn SDL_DestroyAudioStream(stream: *mut SDL_AudioStream);
}

#[unsafe(no_mangle)]
pub static mut sndserver_filename: *mut c_char = b"sndserver\0".as_ptr() as *mut c_char;

#[derive(Clone, Default)]
struct SfxSample {
    data: Vec<u8>,
    sample_rate: u32,
}

#[derive(Default)]
struct Channel {
    active: bool,
    sfx_id: usize,
    pos_fp: u64,
    step_fp: u64,
    handle: c_int,
    start_tic: c_int,
    left_vol: i32,
    right_vol: i32,
}

struct AudioEngine {
    sfx_bank: Vec<SfxSample>,
    channels: [Channel; NUM_CHANNELS],
    pitch_mul: [f64; 256],
    next_handle: c_int,

    soundfont: Option<Arc<SoundFont>>,
    sequencer: Option<MidiFileSequencer>,
    current_midi: Option<Arc<MidiFile>>,
    music_playing: bool,
    music_looping: bool,
    music_volume: f32,

    mus_left: Vec<f32>,
    mus_right: Vec<f32>,
    mix_buf: Vec<i16>,
}

impl AudioEngine {
    fn new() -> Self {
        let mut pitch_mul = [1.0; 256];
        for i in 0..256 {
            let offset = (i as f64) - 128.0;
            pitch_mul[i] = 2.0f64.powf(offset / 64.0);
        }

        Self {
            sfx_bank: vec![SfxSample::default(); NUM_SFX],
            channels: std::array::from_fn(|_| Channel::default()),
            pitch_mul,
            next_handle: 100,
            soundfont: None,
            sequencer: None,
            current_midi: None,
            music_playing: false,
            music_looping: false,
            music_volume: 0.5,
            mus_left: Vec::with_capacity(4096),
            mus_right: Vec::with_capacity(4096),
            mix_buf: Vec::with_capacity(8192),
        }
    }

    fn mix_frame(&mut self, sample_count: usize) {
        self.mix_buf.resize(sample_count * 2, 0);
        self.mus_left.resize(sample_count, 0.0);
        self.mus_right.resize(sample_count, 0.0);
        self.mus_left.fill(0.0);
        self.mus_right.fill(0.0);

        if self.music_playing {
            if let Some(seq) = self.sequencer.as_mut() {
                seq.render(&mut self.mus_left, &mut self.mus_right);
                if seq.end_of_sequence() {
                    if self.music_looping {
                        if let Some(midi) = self.current_midi.as_ref() {
                            seq.play(midi, true);
                        }
                    } else {
                        self.music_playing = false;
                    }
                }
            }
        }

        let mus_gain = self.music_volume * 32767.0;

        for frame in 0..sample_count {
            let mut dl = (self.mus_left[frame] * mus_gain) as i32;
            let mut dr = (self.mus_right[frame] * mus_gain) as i32;

            for ch in &mut self.channels {
                if !ch.active {
                    continue;
                }
                let sample_data = &self.sfx_bank[ch.sfx_id].data;
                let idx = (ch.pos_fp >> 16) as usize;
                if idx >= sample_data.len() {
                    ch.active = false;
                    continue;
                }

                let pcm = (sample_data[idx] as i32) - 128;
                dl += (pcm * ch.left_vol * 256) / 127;
                dr += (pcm * ch.right_vol * 256) / 127;

                ch.pos_fp += ch.step_fp;
                if (ch.pos_fp >> 16) as usize >= sample_data.len() {
                    ch.active = false;
                }
            }

            self.mix_buf[frame * 2] = dl.clamp(-0x8000, 0x7fff) as i16;
            self.mix_buf[frame * 2 + 1] = dr.clamp(-0x8000, 0x7fff) as i16;
        }
    }
}

static ENGINE: OnceLock<Mutex<AudioEngine>> = OnceLock::new();
static mut AUDIO_STREAM: *mut SDL_AudioStream = ptr::null_mut();

fn engine() -> &'static Mutex<AudioEngine> {
    ENGINE.get_or_init(|| Mutex::new(AudioEngine::new()))
}

unsafe fn load_wad_sfx(sfxname: *const c_char) -> SfxSample {
    unsafe {
        if sfxname.is_null() {
            return SfxSample::default();
        }
        let name_str = CStr::from_ptr(sfxname).to_string_lossy();
        let full_name = format!("ds{}\0", name_str);

        let sfxlump = if W_CheckNumForName(full_name.as_ptr() as *mut c_char) == -1 {
            W_GetNumForName(b"dspistol\0".as_ptr() as *mut c_char)
        } else {
            W_GetNumForName(full_name.as_ptr() as *mut c_char)
        };

        let size = W_LumpLength(sfxlump) as usize;
        if size <= 8 {
            return SfxSample::default();
        }

        let raw = W_CacheLumpNum(sfxlump, PU_STATIC) as *const u8;
        let bytes = std::slice::from_raw_parts(raw, size);

        let mut rate = u16::from_le_bytes([bytes[2], bytes[3]]) as u32;
        if rate == 0 {
            rate = 11025;
        }
        let declared_len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let pcm_len = declared_len.min(size - 8);
        let data = bytes[8..(8 + pcm_len)].to_vec();

        Z_Free(raw as *mut c_void);
        SfxSample { data, sample_rate: rate }
    }
}

unsafe extern "C" fn audio_stream_callback(
    _userdata: *mut c_void,
    stream: *mut SDL_AudioStream,
    additional_amount: c_int,
    _total_amount: c_int,
) {
    if additional_amount <= 0 {
        return;
    }
    let samples = (additional_amount as usize) / 4;
    if samples == 0 {
        return;
    }

    let mut eng = engine().lock().unwrap();
    eng.mix_frame(samples);
    unsafe {
        SDL_PutAudioStreamData(
            stream,
            eng.mix_buf.as_ptr() as *const c_void,
            (samples * 4) as c_int,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_SetChannels() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_SetSfxVolume(volume: c_int) {
    unsafe {
        snd_SfxVolume = volume;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_SetMusicVolume(volume: c_int) {
    unsafe {
        snd_MusicVolume = volume;
    }
    let mut eng = engine().lock().unwrap();
    eng.music_volume = (volume as f32) / 15.0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_GetSfxLumpNum(sfx: *mut sfxinfo_t) -> c_int {
    unsafe {
        let name_str = CStr::from_ptr((*sfx).name).to_string_lossy();
        let full_name = format!("ds{}\0", name_str);
        W_GetNumForName(full_name.as_ptr() as *mut c_char)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_StartSound(
    id: c_int,
    vol: c_int,
    sep: c_int,
    pitch: c_int,
    _priority: c_int,
) -> c_int {
    unsafe {
        if AUDIO_STREAM.is_null() {
            return 0;
        }
    }
    let sfx_id = id as usize;
    if sfx_id == 0 || sfx_id >= NUM_SFX {
        return 0;
    }

    let mut eng = engine().lock().unwrap();
    if eng.sfx_bank[sfx_id].data.is_empty() {
        return 0;
    }

    if id == (sfx_sawup as c_int)
        || id == (sfx_sawidl as c_int)
        || id == (sfx_sawful as c_int)
        || id == (sfx_sawhit as c_int)
        || id == (sfx_stnmov as c_int)
        || id == (sfx_pistol as c_int)
    {
        for ch in &mut eng.channels {
            if ch.active && ch.sfx_id == sfx_id {
                ch.active = false;
                break;
            }
        }
    }

    let mut slot = 0usize;
    let mut oldest = unsafe { gametic };
    for (i, ch) in eng.channels.iter().enumerate() {
        if !ch.active {
            slot = i;
            break;
        }
        if ch.start_tic < oldest {
            oldest = ch.start_tic;
            slot = i;
        }
    }

    let pitch_idx = pitch.clamp(0, 255) as usize;
    let rate_ratio = (eng.sfx_bank[sfx_id].sample_rate as f64) / (SAMPLERATE as f64);
    let step_fp = (eng.pitch_mul[pitch_idx] * rate_ratio * 65536.0) as u64;

    let s1 = sep + 1;
    let left_vol = (vol - ((vol * s1 * s1) >> 16)).clamp(0, 127);
    let s2 = s1 - 257;
    let right_vol = (vol - ((vol * s2 * s2) >> 16)).clamp(0, 127);

    let handle = eng.next_handle;
    eng.next_handle = if eng.next_handle >= 30000 { 100 } else { eng.next_handle + 1 };

    eng.channels[slot] = Channel {
active: true,
sfx_id,
pos_fp: 0,
step_fp,
handle,
start_tic: unsafe { gametic },
left_vol,
right_vol,
};
handle
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_StopSound(handle: c_int) {
let mut eng = engine().lock().unwrap();
for ch in &mut eng.channels {
if ch.active && ch.handle == handle {
ch.active = false;
break;
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_SoundIsPlaying(handle: c_int) -> c_int {
let eng = engine().lock().unwrap();
for ch in &eng.channels {
if ch.active && ch.handle == handle {
return 1;
}
}
0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_UpdateSoundParams(_handle: c_int, _vol: c_int, _sep: c_int, _pitch: c_int) {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_UpdateSound() {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_SubmitSound() {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ShutdownSound() {
unsafe {
if !AUDIO_STREAM.is_null() {
SDL_DestroyAudioStream(AUDIO_STREAM);
AUDIO_STREAM = ptr::null_mut();
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_InitSound() {
unsafe {
if !SDL_InitSubSystem(SDL_INIT_AUDIO) {
let err = CStr::from_ptr(SDL_GetError()).to_string_lossy();
eprintln!("I_InitSound: SDL_InitSubSystem(AUDIO) failed: {}", err);
return;
}
let spec = SDL_AudioSpec {
format: SDL_AUDIO_S16,
channels: 2,
freq: SAMPLERATE,
};
AUDIO_STREAM = SDL_OpenAudioDeviceStream(
SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
&raw const spec,
None,
ptr::null_mut(),
);
if AUDIO_STREAM.is_null() {
let err = CStr::from_ptr(SDL_GetError()).to_string_lossy();
eprintln!("I_InitSound: SDL_OpenAudioDeviceStream failed: {}", err);
return;
}
let mut eng = engine().lock().unwrap();
let sfx_base = (&raw mut S_sfx) as *mut sfxinfo_t;
for i in 1..NUM_SFX {
let s = sfx_base.add(i);
if (*s).link.is_null() {
eng.sfx_bank[i] = load_wad_sfx((*s).name);
(*s).data = if eng.sfx_bank[i].data.is_empty() {
ptr::null_mut()
} else {
eng.sfx_bank[i].data.as_mut_ptr() as *mut c_void
};
}
}
for i in 1..NUM_SFX {
let s = sfx_base.add(i);
if !(*s).link.is_null() {
let link_idx = (*s).link.offset_from(sfx_base) as usize;
eng.sfx_bank[i] = eng.sfx_bank[link_idx].clone();
(*s).data = (*sfx_base.add(link_idx)).data;
}
}
drop(eng);
SDL_SetAudioStreamGetCallback(AUDIO_STREAM, Some(audio_stream_callback), ptr::null_mut());
SDL_ResumeAudioStreamDevice(AUDIO_STREAM);
eprintln!("I_InitSound: 100% pure Rust SDL3 + rustySynth audio ready (44100Hz stereo)");
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_InitMusic() {
let sf_paths = [
"soundfont.sf2",
"/usr/share/soundfonts/FluidR3_GM.sf2",
"/usr/share/sounds/sf2/FluidR3_GM.sf2",
"/usr/share/soundfonts/default.sf2",
];
let mut eng = engine().lock().unwrap();
for path in sf_paths {
if let Ok(mut file) = File::open(path) {
if let Ok(sf) = SoundFont::new(&mut file) {
let sf_arc = Arc::new(sf);
let settings = SynthesizerSettings::new(SAMPLERATE);
if let Ok(synth) = Synthesizer::new(&sf_arc, &settings) {
eng.soundfont = Some(sf_arc);
eng.sequencer = Some(MidiFileSequencer::new(synth));
eprintln!("I_InitMusic: loaded SoundFont '{}' (rustySynth)", path);
return;
}
}
}
}
eprintln!("I_InitMusic: no SoundFont (.sf2) found, music disabled");
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ShutdownMusic() {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
if let Some(seq) = eng.sequencer.as_mut() {
seq.stop();
}
eng.current_midi = None;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_PlaySong(_handle: c_int, looping: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_looping = looping != 0;
    let midi_opt = eng.current_midi.clone();
    if let (Some(seq), Some(midi)) = (eng.sequencer.as_mut(), midi_opt.as_ref()) {
seq.play(midi, looping != 0);
eng.music_playing = true;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_PauseSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ResumeSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
if eng.current_midi.is_some() {
eng.music_playing = true;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_StopSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
if let Some(seq) = eng.sequencer.as_mut() {
seq.stop();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_UnRegisterSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
if let Some(seq) = eng.sequencer.as_mut() {
seq.stop();
}
eng.current_midi = None;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_RegisterSong(data: *mut c_void) -> c_int {
unsafe { I_UnRegisterSong(1); }
if data.is_null() {
return 0;
}
// Максимальный размер MUS-лампа в DOOM не превышает 64 КБ
let mus_slice = unsafe { std::slice::from_raw_parts(data as *const u8, 65536) };
let Some(mid_bytes) = convert_mus_to_mid(mus_slice) else {
eprintln!("I_RegisterSong: failed to convert MUS to MIDI");
return 0;
};
let mut cursor = Cursor::new(mid_bytes);
let Ok(midi_file) = MidiFile::new(&mut cursor) else {
eprintln!("I_RegisterSong: rustySynth MidiFile parse failed");
return 0;
};
let mut eng = engine().lock().unwrap();
eng.current_midi = Some(Arc::new(midi_file));
1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_QrySongPlaying(_handle: c_int) -> c_int {
let eng = engine().lock().unwrap();
eng.music_playing as c_int
}