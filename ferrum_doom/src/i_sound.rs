#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, c_char, c_double, c_float, c_int, c_short, c_uint, c_void};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::g_game::gametic;
use crate::s_sound::{snd_MusicVolume, snd_SfxVolume};
use crate::sounds::{S_sfx, sfxinfo_t};
use crate::w_wad::{W_CacheLumpNum, W_CheckNumForName, W_GetNumForName, W_LumpLength};
use crate::z_zone::{PU_STATIC, Z_Free};

const NUM_CHANNELS: usize = 8;
const SAMPLERATE: c_int = 44100;
const NUM_SFX: usize = NUMSFX as usize;

const SDL_AUDIO_S16: u32 = 0x8010;
const SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK: u32 = 0xFFFF_FFFF;

const TML_PROGRAM_CHANGE: u8 = 0xC0;
const TML_NOTE_ON: u8 = 0x90;
const TML_NOTE_OFF: u8 = 0x80;
const TML_PITCH_BEND: u8 = 0xE0;
const TML_CONTROL_CHANGE: u8 = 0xB0;
const TSF_STEREO_INTERLEAVED: c_int = 0;

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

#[repr(C)]
pub struct tsf {
    _private: [u8; 0],
}

type SDL_AudioStreamCallback = unsafe extern "C" fn(
    userdata: *mut c_void,
    stream: *mut SDL_AudioStream,
    additional_amount: c_int,
    total_amount: c_int,
);

unsafe extern "C" {
    fn free(ptr: *mut c_void);
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

    fn tsf_load_filename(filename: *const c_char) -> *mut tsf;
    fn tsf_close(f: *mut tsf);
    fn tsf_reset(f: *mut tsf);
    fn tsf_set_output(f: *mut tsf, outputmode: c_int, samplerate: c_int, global_gain_db: c_float);
    fn tsf_set_volume(f: *mut tsf, global_volume: c_float);
    fn tsf_channel_set_bank_preset(f: *mut tsf, channel: c_int, bank: c_int, preset_number: c_int) -> c_int;
    fn tsf_channel_set_presetnumber(f: *mut tsf, channel: c_int, preset_number: c_int, flag_mididrums: c_int) -> c_int;
    fn tsf_channel_note_on(f: *mut tsf, channel: c_int, key: c_int, vel: c_float) -> c_int;
    fn tsf_channel_note_off(f: *mut tsf, channel: c_int, key: c_int);
    fn tsf_channel_set_pitchwheel(f: *mut tsf, channel: c_int, pitch_wheel: c_int) -> c_int;
    fn tsf_channel_midi_control(f: *mut tsf, channel: c_int, controller: c_int, control_value: c_int) -> c_int;
    fn tsf_render_short(f: *mut tsf, buffer: *mut c_short, samples: c_int, flag_mixing: c_int);

    fn tml_load_memory(buffer: *const c_void, size: c_int) -> *mut tml_message;
    fn tml_free(f: *mut tml_message);

    #[link_name = "rust_mus2mid_convert"]
    fn mus2mid_convert(
        mus_data: *const u8,
        midi_out: *mut *mut u8,
        midi_len_out: *mut usize,
    ) -> c_int;
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
    pos_fp: u64,   // позиция в формате 48.16 fixed-point
    step_fp: u64,  // шаг в формате 48.16 fixed-point
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

    tsf: *mut tsf,
    tml_head: *mut tml_message,
    tml_curr: *mut tml_message,
    msec: f64,
    music_playing: bool,
    music_looping: bool,
    music_volume: f32,

    mix_buf: Vec<i16>,
    mus_buf: Vec<i16>,
}

unsafe impl Send for AudioEngine {}

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
            tsf: ptr::null_mut(),
            tml_head: ptr::null_mut(),
            tml_curr: ptr::null_mut(),
            msec: 0.0,
            music_playing: false,
            music_looping: false,
            music_volume: 0.5,
            mix_buf: Vec::with_capacity(8192),
            mus_buf: Vec::with_capacity(8192),
        }
    }

    fn stop_song_and_reset(&mut self) {
        self.music_playing = false;
        if !self.tsf.is_null() {
            unsafe {
                tsf_reset(self.tsf);
                tsf_channel_set_bank_preset(self.tsf, 9, 128, 0);
            }
        }
    }

    fn render_music(&mut self, sample_count: usize) {
        self.mus_buf.resize(sample_count * 2, 0);
        self.mus_buf.fill(0);

        if self.tsf.is_null() || !self.music_playing || self.tml_curr.is_null() {
            return;
        }

        let mut rendered = 0usize;
        while rendered < sample_count {
            unsafe {
                while !self.tml_curr.is_null() && self.msec >= ((*self.tml_curr).time as c_double) {
                    let msg = &*self.tml_curr;
                    let ch = msg.channel as c_int;
                    match msg.type_ {
                        TML_PROGRAM_CHANGE => {
                            let prog = msg.__bindgen_anon_1.__bindgen_anon_1.__bindgen_anon_1.program as c_int;
                            tsf_channel_set_presetnumber(self.tsf, ch, prog, (ch == 9) as c_int);
                        }
                        TML_NOTE_ON => {
                            let key = msg.__bindgen_anon_1.__bindgen_anon_1.__bindgen_anon_1.key as c_int;
                            let vel = (msg.__bindgen_anon_1.__bindgen_anon_1.__bindgen_anon_2.velocity as f32) / 127.0;
                            tsf_channel_note_on(self.tsf, ch, key, vel);
                        }
                        TML_NOTE_OFF => {
                            let key = msg.__bindgen_anon_1.__bindgen_anon_1.__bindgen_anon_1.key as c_int;
                            tsf_channel_note_off(self.tsf, ch, key);
                        }
                        TML_PITCH_BEND => {
                            let bend = msg.__bindgen_anon_1.__bindgen_anon_2.pitch_bend as c_int;
                            tsf_channel_set_pitchwheel(self.tsf, ch, bend);
                        }
                        TML_CONTROL_CHANGE => {
                            let ctrl = msg.__bindgen_anon_1.__bindgen_anon_1.__bindgen_anon_1.control as c_int;
                            let val = msg.__bindgen_anon_1.__bindgen_anon_1.__bindgen_anon_2.control_value as c_int;
                            tsf_channel_midi_control(self.tsf, ch, ctrl, val);
                        }
                        _ => {}
                    }
                    self.tml_curr = msg.next;
                }
            }

            if self.tml_curr.is_null() {
                if self.music_looping && !self.tml_head.is_null() {
                    self.tml_curr = self.tml_head;
                    self.msec = 0.0;
                } else {
                    self.music_playing = false;
                    break;
                }
            }

            let chunk = (sample_count - rendered).min(64);
            unsafe {
                tsf_render_short(
                    self.tsf,
                    self.mus_buf.as_mut_ptr().add(rendered * 2),
                    chunk as c_int,
                    0,
                );
            }
            self.msec += ((chunk as f64) * 1000.0) / (SAMPLERATE as f64);
            rendered += chunk;
        }
    }

    fn mix_frame(&mut self, sample_count: usize) {
        self.mix_buf.resize(sample_count * 2, 0);
        self.render_music(sample_count);

        for frame in 0..sample_count {
            let mut dl: i32 = self.mus_buf[frame * 2] as i32;
            let mut dr: i32 = self.mus_buf[frame * 2 + 1] as i32;

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

                // Преобразуем беззнаковый 8-битный PCM (128 = тишина) в знаковый [-128..127]
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
// Заголовок DMX SFX: [u16 format=3][u16 sample_rate][u32 sample_count]
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
// --- C ABI: SFX ---
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
if !eng.tsf.is_null() {
unsafe {
tsf_set_volume(eng.tsf, eng.music_volume);
}
}
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
// Останавливаем дублирующиеся звуки пилы и пистолета
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
let mut found_free = false;
for (i, ch) in eng.channels.iter().enumerate() {
if !ch.active {
slot = i;
found_free = true;
break;
}
if ch.start_tic < oldest {
oldest = ch.start_tic;
slot = i;
}
}
let _ = found_free;
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
eprintln!("I_InitSound: clean Rust SDL3 audio ready (44100Hz 16-bit stereo)");
}
}
// --- C ABI: MUSIC ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_InitMusic() {
let sf_paths: [&[u8]; 4] = [
b"soundfont.sf2\0",
b"/usr/share/soundfonts/FluidR3_GM.sf2\0",
b"/usr/share/sounds/sf2/FluidR3_GM.sf2\0",
b"/usr/share/soundfonts/default.sf2\0",
];
let mut eng = engine().lock().unwrap();
for path in sf_paths {
let tsf_ptr = unsafe { tsf_load_filename(path.as_ptr() as *const c_char) };
if !tsf_ptr.is_null() {
unsafe {
tsf_channel_set_bank_preset(tsf_ptr, 9, 128, 0);
tsf_set_output(tsf_ptr, TSF_STEREO_INTERLEAVED, SAMPLERATE, 0.0);
tsf_set_volume(tsf_ptr, eng.music_volume);
}
eng.tsf = tsf_ptr;
let name = CStr::from_bytes_with_nul(path).unwrap().to_string_lossy();
eprintln!("I_InitMusic: loaded SoundFont '{}'", name);
return;
}
}
eprintln!("I_InitMusic: no SoundFont (.sf2) found, music disabled");
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ShutdownMusic() {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
if !eng.tml_head.is_null() {
unsafe { tml_free(eng.tml_head); }
eng.tml_head = ptr::null_mut();
eng.tml_curr = ptr::null_mut();
}
if !eng.tsf.is_null() {
unsafe { tsf_close(eng.tsf); }
eng.tsf = ptr::null_mut();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_PlaySong(_handle: c_int, looping: c_int) {
let mut eng = engine().lock().unwrap();
if eng.tml_head.is_null() {
return;
}
eng.stop_song_and_reset();
eng.tml_curr = eng.tml_head;
eng.msec = 0.0;
eng.music_looping = looping != 0;
eng.music_playing = true;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_PauseSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.music_playing = false;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ResumeSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
if !eng.tml_head.is_null() {
eng.music_playing = true;
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_StopSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.stop_song_and_reset();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_UnRegisterSong(_handle: c_int) {
let mut eng = engine().lock().unwrap();
eng.stop_song_and_reset();
if !eng.tml_head.is_null() {
unsafe { tml_free(eng.tml_head); }
eng.tml_head = ptr::null_mut();
eng.tml_curr = ptr::null_mut();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_RegisterSong(data: *mut c_void) -> c_int {
unsafe { I_UnRegisterSong(1); }
if data.is_null() {
return 0;
}
let mut midi_bytes: *mut u8 = ptr::null_mut();
let mut midi_len: usize = 0;
let converted = unsafe {
mus2mid_convert(data as *const u8, &raw mut midi_bytes, &raw mut midi_len)
};
if converted == 0 || midi_bytes.is_null() {
eprintln!("I_RegisterSong: failed to convert MUS to MIDI");
return 0;
}
let head = unsafe { tml_load_memory(midi_bytes as *const c_void, midi_len as c_int) };
unsafe { free(midi_bytes as *mut c_void); }
if head.is_null() {
eprintln!("I_RegisterSong: tml_load_memory failed");
return 0;
}
let mut eng = engine().lock().unwrap();
eng.tml_head = head;
eng.tml_curr = head;
eng.msec = 0.0;
1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_QrySongPlaying(_handle: c_int) -> c_int {
let eng = engine().lock().unwrap();
eng.music_playing as c_int
}