#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;

use std::cell::RefCell;
use std::ffi::c_int;

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::render::{ScaleMode, Texture, TextureCreator, WindowCanvas};
use sdl3::sys::render::{SDL_RendererLogicalPresentation, SDL_SetRenderLogicalPresentation};
use sdl3::video::{FullscreenType, WindowContext};
use sdl3::{EventPump, Sdl};

use crate::d_main::D_PostEvent;
use crate::i_system::I_Quit;
use crate::v_video::{doom_error, gammatable, screens, usegamma};

const SCR_WIDTH: usize = SCREENWIDTH as usize;
const SCR_HEIGHT: usize = SCREENHEIGHT as usize;
const SCR_PIXELS: usize = SCR_WIDTH * SCR_HEIGHT;

struct VideoState {
    sdl: Sdl,
    canvas: WindowCanvas,
    texture: Texture<'static>,
    event_pump: EventPump,
    palette_argb: [u32; 256],
    frame_buffer: Vec<u8>,
    mouse_grabbed: bool,
}

thread_local! {
    static VIDEO: RefCell<Option<VideoState>> = const { RefCell::new(None) };
}

impl VideoState {
    fn new() -> Result<Self, String> {
        let sdl = sdl3::init().map_err(|e| format!("SDL_Init failed: {e}"))?;
        let video_subsys = sdl
            .video()
            .map_err(|e| format!("SDL_InitSubSystem(VIDEO) failed: {e}"))?;

        let window = video_subsys
            .window(
                "FerrumMolibdenDOOM",
                (SCR_WIDTH * 3) as u32,
                (SCR_HEIGHT * 3) as u32,
            )
            .resizable()
            .position_centered()
            .build()
            .map_err(|e| format!("SDL_CreateWindow failed: {e}"))?;

        let canvas = window.into_canvas();

        unsafe {
            SDL_SetRenderLogicalPresentation(
                canvas.raw(),
                SCR_WIDTH as c_int,
                SCR_HEIGHT as c_int,
                SDL_RendererLogicalPresentation::LETTERBOX,
            );
        }

        let creator: &'static TextureCreator<WindowContext> =
            Box::leak(Box::new(canvas.texture_creator()));

        let mut texture = creator
            .create_texture_streaming(
                PixelFormat::from(sdl3::sys::pixels::SDL_PIXELFORMAT_ARGB8888.0 as i64),
                SCR_WIDTH as u32,
                SCR_HEIGHT as u32,
            )
            .map_err(|e| format!("SDL_CreateTexture failed: {e}"))?;

        texture.set_scale_mode(ScaleMode::Nearest);

        let event_pump = sdl
            .event_pump()
            .map_err(|e| format!("SDL_EventPump failed: {e}"))?;

        let mut state = Self {
            sdl,
            canvas,
            texture,
            event_pump,
            palette_argb: [0xFF00_0000; 256],
            frame_buffer: vec![0u8; SCR_PIXELS * 4],
            mouse_grabbed: true,
        };
        state.set_mouse_grab(true);
        Ok(state)
    }

    fn set_mouse_grab(&mut self, grab: bool) {
        self.mouse_grabbed = grab;
        self.sdl
            .mouse()
            .set_relative_mouse_mode(self.canvas.window(), grab);
    }

    fn toggle_fullscreen(&mut self) {
        let is_fs = self.canvas.window().fullscreen_state() != FullscreenType::Off;
        let _ = self.canvas.window_mut().set_fullscreen(!is_fs);
    }

    fn update_palette(&mut self, pal_bytes: &[u8], gamma_idx: usize) {
        let g_table = &gammatable[gamma_idx.min(4)];
        for (i, rgb) in pal_bytes.chunks_exact(3).take(256).enumerate() {
            let r = g_table[rgb[0] as usize] as u32;
            let g = g_table[rgb[1] as usize] as u32;
            let b = g_table[rgb[2] as usize] as u32;
            self.palette_argb[i] = 0xFF00_0000u32 | (r << 16) | (g << 8) | b;
        }
    }

    fn render_indexed(&mut self, scr_slice: &[u8]) {
        for (dst_px, &idx) in self.frame_buffer.chunks_exact_mut(4).zip(scr_slice.iter()) {
            let argb = self.palette_argb[idx as usize];
            dst_px.copy_from_slice(&argb.to_ne_bytes());
        }

        let _ = self.texture.update(None, &self.frame_buffer, SCR_WIDTH * 4);
        self.canvas.clear();
        let _ = self.canvas.copy(&self.texture, None, None);
        self.canvas.present();
    }
}

#[inline]
fn post_event(ev_type: evtype_t, data1: c_int, data2: c_int) {
    let mut doom_ev = event_t {
        type_: ev_type,
        data1,
        data2,
        data3: 0,
    };
    unsafe {
        D_PostEvent((&raw mut doom_ev) as *mut _);
    }
}

fn translate_keycode(key: Keycode) -> c_int {
    match key {
        Keycode::Left => KEY_LEFTARROW as c_int,
        Keycode::Right => KEY_RIGHTARROW as c_int,
        Keycode::Down => KEY_DOWNARROW as c_int,
        Keycode::Up => KEY_UPARROW as c_int,
        Keycode::Escape => KEY_ESCAPE as c_int,
        Keycode::Return | Keycode::KpEnter => KEY_ENTER as c_int,
        Keycode::Tab => KEY_TAB as c_int,
        Keycode::F1 => KEY_F1 as c_int,
        Keycode::F2 => KEY_F2 as c_int,
        Keycode::F3 => KEY_F3 as c_int,
        Keycode::F4 => KEY_F4 as c_int,
        Keycode::F5 => KEY_F5 as c_int,
        Keycode::F6 => KEY_F6 as c_int,
        Keycode::F7 => KEY_F7 as c_int,
        Keycode::F8 => KEY_F8 as c_int,
        Keycode::F9 => KEY_F9 as c_int,
        Keycode::F10 => KEY_F10 as c_int,
        Keycode::F11 => KEY_F11 as c_int,
        Keycode::F12 => KEY_F12 as c_int,
        Keycode::Backspace | Keycode::Delete => KEY_BACKSPACE as c_int,
        Keycode::Pause => KEY_PAUSE as c_int,
        Keycode::Equals => KEY_EQUALS as c_int,
        Keycode::Minus => KEY_MINUS as c_int,
        Keycode::LShift | Keycode::RShift => KEY_RSHIFT as c_int,
        Keycode::LCtrl | Keycode::RCtrl => KEY_RCTRL as c_int,
        Keycode::LAlt | Keycode::RAlt => KEY_RALT as c_int,
        other => {
            let code = other as i32;
            if (b'A' as i32..=b'Z' as i32).contains(&code) {
                code + (b'a' as i32 - b'A' as i32)
            } else if (32..=126).contains(&code) {
                code
            } else {
                0
            }
        }
    }
}

pub extern "C" fn I_InitGraphics() {
    if VIDEO.with(|v| v.borrow().is_some()) {
        return;
    }

    let state = match VideoState::new() {
        Ok(s) => s,
        Err(msg) => doom_error(&msg),
    };

    unsafe {
        let scr_ptr = (&raw mut screens) as *mut *mut u8;
        if (*scr_ptr).is_null() {
            let buf = Box::leak(vec![0u8; SCR_PIXELS].into_boxed_slice());
            *scr_ptr = buf.as_mut_ptr();
        }
    }

    VIDEO.with(|v| *v.borrow_mut() = Some(state));
}

pub extern "C" fn I_ShutdownGraphics() {
    VIDEO.with(|v| {
        if let Some(state) = v.borrow_mut().as_mut() {
            state.set_mouse_grab(false);
        }
    });
}

pub unsafe extern "C" fn I_SetPalette(palette: *mut u8) {
    if palette.is_null() {
        return;
    }
    let pal_bytes = unsafe { std::slice::from_raw_parts(palette, 256 * 3) };
    let gamma_idx = unsafe { usegamma as usize };

    VIDEO.with(|v| {
        if let Some(state) = v.borrow_mut().as_mut() {
            state.update_palette(pal_bytes, gamma_idx);
        }
    });
}

pub extern "C" fn I_UpdateNoBlit() {}

pub extern "C" fn I_FinishUpdate() {
    let scr_slice = unsafe {
        let scr0 = *(&raw const screens as *const *mut u8);
        if scr0.is_null() {
            return;
        }
        std::slice::from_raw_parts(scr0, SCR_PIXELS)
    };

    VIDEO.with(|v| {
        if let Some(state) = v.borrow_mut().as_mut() {
            state.render_indexed(scr_slice);
        }
    });
}

pub unsafe extern "C" fn I_ReadScreen(scr: *mut u8) {
    unsafe {
        let scr0 = *(&raw const screens as *const *mut u8);
        if !scr0.is_null() && !scr.is_null() {
            std::ptr::copy_nonoverlapping(scr0, scr, SCR_PIXELS);
        }
    }
}

pub extern "C" fn I_StartTic() {
    VIDEO.with(|v| {
        let mut borrow = v.borrow_mut();
        let Some(state) = borrow.as_mut() else { return };

        let events: Vec<Event> = state.event_pump.poll_iter().collect();

        for event in events {
            match event {
                Event::Quit { .. } => {
                    I_Quit();
                }
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => {
                    state.set_mouse_grab(false);
                }
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => {
                    state.set_mouse_grab(true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F11),
                    ..
                } => {
                    state.toggle_fullscreen();
                }
                Event::KeyDown {
                    keycode: Some(key),
                    ..
                } => {
                    let code = translate_keycode(key);
                    if code != 0 {
                        post_event(ev_keydown, code, 0);
                    }
                }
                Event::KeyUp {
                    keycode: Some(key),
                    ..
                } => {
                    let code = translate_keycode(key);
                    if code != 0 {
                        post_event(ev_keyup, code, 0);
                    }
                }
                Event::MouseButtonDown { .. } | Event::MouseButtonUp { .. } => {
                    if matches!(event, Event::MouseButtonDown { .. }) && !state.mouse_grabbed {
                        state.set_mouse_grab(true);
                    }
                    let mouse_state = state.event_pump.mouse_state();
                    let buttons = (mouse_state.left() as c_int)
                        | ((mouse_state.right() as c_int) << 1)
                        | ((mouse_state.middle() as c_int) << 2);
post_event(ev_mouse, buttons, 0);
}
Event::MouseMotion {
mousestate, xrel, ..
} => {
if !state.mouse_grabbed {
continue;
}
let buttons = (mousestate.left() as c_int)
| ((mousestate.right() as c_int) << 1)
| ((mousestate.middle() as c_int) << 2);
post_event(ev_mouse, buttons, (xrel * 3.0) as c_int);
}
_ => {}
}
}
});
}
pub extern "C" fn I_StartFrame() {}
