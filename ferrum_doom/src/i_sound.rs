#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStreamWithCallback};

use crate::g_game::gametic;
use crate::mus2mid::convert_mus_to_mid;
use crate::s_sound::{snd_MusicVolume, snd_SfxVolume};
use crate::sounds::S_SFX;
use crate::w_wad::{W_CacheLumpNum, W_LumpLength, w_check_num_for_name, w_get_num_for_name};
use crate::z_zone::{PU_STATIC, Z_Free};

const NUM_CHANNELS: usize = 8;
const SAMPLERATE: i32 = 44100;
const NUM_SFX: usize = NUMSFX as usize;

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
        }
    }

    fn mix_into(&mut self, out: &mut [i16]) {
        let sample_count = out.len() / 2;
        if sample_count == 0 {
            return;
        }

        self.mus_left.resize(sample_count, 0.0);
        self.mus_right.resize(sample_count, 0.0);
        self.mus_left.fill(0.0);
        self.mus_right.fill(0.0);

        if self.music_playing {
            let midi_opt = self.current_midi.clone();
            if let Some(seq) = self.sequencer.as_mut() {
                seq.render(&mut self.mus_left, &mut self.mus_right);
                if seq.end_of_sequence() {
                    if self.music_looping {
                        if let Some(midi) = midi_opt.as_ref() {
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

            out[frame * 2] = dl.clamp(-0x8000, 0x7fff) as i16;
            out[frame * 2 + 1] = dr.clamp(-0x8000, 0x7fff) as i16;
        }
    }
}

struct DoomAudioCallback {
    buf: Vec<i16>,
}

impl AudioCallback<i16> for DoomAudioCallback {
    fn callback(&mut self, stream: &mut sdl3::audio::AudioStream, requested: i32) {
        if requested <= 0 {
            return;
        }
        let samples_i16 = requested as usize;
        self.buf.resize(samples_i16, 0);

        if let Ok(mut eng) = engine().lock() {
            eng.mix_into(&mut self.buf);
        } else {
            self.buf.fill(0);
        }

        let _ = stream.put_data_i16(&self.buf);
    }
}

static ENGINE: OnceLock<Mutex<AudioEngine>> = OnceLock::new();

thread_local! {
    static AUDIO_STREAM: RefCell<Option<AudioStreamWithCallback<DoomAudioCallback>>> = const { RefCell::new(None) };
}

fn engine() -> &'static Mutex<AudioEngine> {
    ENGINE.get_or_init(|| Mutex::new(AudioEngine::new()))
}

fn load_wad_sfx(sfxname: Option<&str>) -> SfxSample {
    let Some(name_str) = sfxname else {
        return SfxSample::default();
    };
    let full_name = format!("ds{}", name_str);

    let sfxlump = if w_check_num_for_name(&full_name) == -1 {
        w_get_num_for_name("dspistol")
    } else {
        w_get_num_for_name(&full_name)
    };

    let size = W_LumpLength(sfxlump) as usize;
    if size <= 8 {
        return SfxSample::default();
    }

    let raw = W_CacheLumpNum(sfxlump, PU_STATIC) as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(raw, size) };

    let mut rate = u16::from_le_bytes(bytes[2..4].try_into().unwrap()) as u32;
    if rate == 0 {
        rate = 11025;
    }
    let declared_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let pcm_len = declared_len.min(size - 8);
    let data = bytes[8..(8 + pcm_len)].to_vec();

    Z_Free(raw as *mut c_void);
    SfxSample {
        data,
        sample_rate: rate,
    }
}

pub fn get_sfx_lump_num(name: Option<&str>) -> c_int {
    let Some(name_str) = name else { return -1 };
    let full_name = format!("ds{}", name_str);
    w_get_num_for_name(&full_name)
}

pub extern "C" fn I_SetChannels() {}

pub extern "C" fn I_SetSfxVolume(volume: c_int) {
    unsafe {
        snd_SfxVolume = volume;
    }
}

pub extern "C" fn I_SetMusicVolume(volume: c_int) {
    unsafe {
        snd_MusicVolume = volume;
    }
    let mut eng = engine().lock().unwrap();
    eng.music_volume = (volume as f32) / 15.0;
}

pub extern "C" fn I_StartSound(
    id: c_int,
    vol: c_int,
    sep: c_int,
    pitch: c_int,
    _priority: c_int,
) -> c_int {
    if AUDIO_STREAM.with(|s| s.borrow().is_none()) {
        return 0;
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
    let cur_tic = unsafe { gametic };
    let mut oldest = cur_tic;
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
        start_tic: cur_tic,
        left_vol,
        right_vol,
    };

    handle
}

pub extern "C" fn I_StopSound(handle: c_int) {
    let mut eng = engine().lock().unwrap();
    for ch in &mut eng.channels {
        if ch.active && ch.handle == handle {
            ch.active = false;
            break;
        }
    }
}

pub extern "C" fn I_SoundIsPlaying(handle: c_int) -> c_int {
    let eng = engine().lock().unwrap();
    for ch in &eng.channels {
        if ch.active && ch.handle == handle {
            return 1;
        }
    }
    0
}

pub extern "C" fn I_UpdateSoundParams(_handle: c_int, _vol: c_int, _sep: c_int, _pitch: c_int) {}

pub extern "C" fn I_UpdateSound() {}

pub extern "C" fn I_SubmitSound() {}

pub extern "C" fn I_ShutdownSound() {
    AUDIO_STREAM.with(|s| {
        *s.borrow_mut() = None;
    });
}

pub extern "C" fn I_InitSound() {
    let sdl = match sdl3::init() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("I_InitSound: sdl3::init failed: {e}");
            return;
        }
    };
    let audio_subsys = match sdl.audio() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("I_InitSound: SDL_InitSubSystem(AUDIO) failed: {e}");
            return;
        }
    };

    {
        let mut eng = engine().lock().unwrap();
        let sfx_table = S_SFX.lock().unwrap();
        for i in 1..NUM_SFX {
            if sfx_table[i].link_id.is_none() {
                eng.sfx_bank[i] = load_wad_sfx(sfx_table[i].name);
}
}
for i in 1..NUM_SFX {
if let Some(link_idx) = sfx_table[i].link_id {
eng.sfx_bank[i] = eng.sfx_bank[link_idx].clone();
}
}
}
let spec = AudioSpec {
freq: Some(SAMPLERATE),
channels: Some(2),
format: Some(AudioFormat::s16_sys()),
};
let Ok(device) = audio_subsys.open_playback_device(&spec) else {
eprintln!("I_InitSound: open_playback_device failed");
return;
};
let stream = match device.open_playback_stream_with_callback(
&spec,
DoomAudioCallback {
buf: Vec::with_capacity(4096),
},
) {
Ok(st) => st,
Err(e) => {
eprintln!("I_InitSound: open_playback_stream_with_callback failed: {e}");
return;
}
};
let _ = stream.resume();
AUDIO_STREAM.with(|s| *s.borrow_mut() = Some(stream));
eprintln!("I_InitSound: safe sdl3::audio + rustySynth ready (44100Hz stereo)");
}
pub extern "C" fn I_InitMusic() {
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
pub extern "C" fn I_ShutdownMusic() {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
if let Some(seq) = eng.sequencer.as_mut() {
seq.stop();
}
eng.current_midi = None;
}
pub extern "C" fn I_PlaySong(_handle: c_int, looping: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_looping = looping != 0;
let midi_opt = eng.current_midi.clone();
if let (Some(seq), Some(midi)) = (eng.sequencer.as_mut(), midi_opt.as_ref()) {
seq.play(midi, looping != 0);
eng.music_playing = true;
}
}
pub extern "C" fn I_PauseSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
}
pub extern "C" fn I_ResumeSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
if eng.current_midi.is_some() {
eng.music_playing = true;
}
}
pub extern "C" fn I_StopSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
if let Some(seq) = eng.sequencer.as_mut() {
seq.stop();
}
}
pub extern "C" fn I_UnRegisterSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
if let Some(seq) = eng.sequencer.as_mut() {
seq.stop();
}
eng.current_midi = None;
}
pub unsafe extern "C" fn I_RegisterSong(data: *mut c_void) -> c_int {
I_UnRegisterSong(1);
if data.is_null() {
return 0;
}
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
pub extern "C" fn I_QrySongPlaying(_handle: c_int) -> c_int {
let eng = engine().lock().unwrap();
eng.music_playing as c_int
}
