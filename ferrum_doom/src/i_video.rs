#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, c_char, c_float, c_int, c_void};
use std::ptr;

use crate::d_main::D_PostEvent;
use crate::v_video::{doom_error, gammatable, screens, usegamma};

const SDL_WINDOW_FULLSCREEN: u64 = 0x0000_0000_0000_0001;
const SDL_WINDOW_RESIZABLE: u64 = 0x0000_0000_0000_0020;
const SDL_LOGICAL_PRESENTATION_LETTERBOX: u32 = 2;
const SDL_PIXELFORMAT_ARGB8888: u32 = 0x1636_2004;
const SDL_TEXTUREACCESS_STREAMING: u32 = 1;
const SDL_SCALEMODE_NEAREST: u32 = 0;
const SDL_BUTTON_LMASK: u32 = 1 << 0;
const SDL_BUTTON_MMASK: u32 = 1 << 1;
const SDL_BUTTON_RMASK: u32 = 1 << 2;
const SCR_WIDTH: usize = SCREENWIDTH as usize;
const SCR_HEIGHT: usize = SCREENHEIGHT as usize;
const SCR_PIXELS: usize = SCR_WIDTH * SCR_HEIGHT;

#[repr(C)]
pub struct SDL_Window {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SDL_Renderer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SDL_Texture {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn I_Quit();

    fn SDL_Init(flags: u32) -> bool;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;

    fn SDL_CreateWindow(title: *const c_char, w: c_int, h: c_int, flags: u64) -> *mut SDL_Window;
    fn SDL_DestroyWindow(window: *mut SDL_Window);
    fn SDL_ShowWindow(window: *mut SDL_Window) -> bool;
    fn SDL_GetWindowFlags(window: *mut SDL_Window) -> u64;
    fn SDL_SetWindowFullscreen(window: *mut SDL_Window, fullscreen: bool) -> bool;
    fn SDL_SetWindowRelativeMouseMode(window: *mut SDL_Window, enabled: bool) -> bool;

    fn SDL_CreateRenderer(window: *mut SDL_Window, name: *const c_char) -> *mut SDL_Renderer;
    fn SDL_DestroyRenderer(renderer: *mut SDL_Renderer);
    fn SDL_SetRenderLogicalPresentation(
        renderer: *mut SDL_Renderer,
        w: c_int,
        h: c_int,
        mode: u32,
    ) -> bool;
    fn SDL_RenderClear(renderer: *mut SDL_Renderer) -> bool;
    fn SDL_RenderTexture(
        renderer: *mut SDL_Renderer,
        texture: *mut SDL_Texture,
        srcrect: *const c_void,
        dstrect: *const c_void,
    ) -> bool;
    fn SDL_RenderPresent(renderer: *mut SDL_Renderer) -> bool;

    fn SDL_CreateTexture(
        renderer: *mut SDL_Renderer,
        format: u32,
        access: u32,
        w: c_int,
        h: c_int,
    ) -> *mut SDL_Texture;
    fn SDL_DestroyTexture(texture: *mut SDL_Texture);
    fn SDL_SetTextureScaleMode(texture: *mut SDL_Texture, scale_mode: u32) -> bool;
    fn SDL_UpdateTexture(
        texture: *mut SDL_Texture,
        rect: *const c_void,
        pixels: *const c_void,
        pitch: c_int,
    ) -> bool;

    fn SDL_PollEvent(event: *mut SDL_Event) -> bool;
    fn SDL_GetMouseState(x: *mut c_float, y: *mut c_float) -> u32;
}

static mut window: *mut SDL_Window = ptr::null_mut();
static mut renderer: *mut SDL_Renderer = ptr::null_mut();
static mut texture: *mut SDL_Texture = ptr::null_mut();

static mut palette_argb: [u32; 256] = [0; 256];
static mut frame_buffer: [u32; SCR_PIXELS] = [0; SCR_PIXELS];
static mut mouse_grabbed: c_int = 1;

unsafe fn set_mouse_grab(grab: c_int) {
    unsafe {
        mouse_grabbed = grab;
        if !window.is_null() {
            SDL_SetWindowRelativeMouseMode(window, grab != 0);
        }
    }
}

fn translate_key(key: u32) -> c_int {
    match key {
        SDLK_LEFT => KEY_LEFTARROW as c_int,
        SDLK_RIGHT => KEY_RIGHTARROW as c_int,
        SDLK_DOWN => KEY_DOWNARROW as c_int,
        SDLK_UP => KEY_UPARROW as c_int,
        SDLK_ESCAPE => KEY_ESCAPE as c_int,
        SDLK_RETURN | SDLK_KP_ENTER => KEY_ENTER as c_int,
        SDLK_TAB => KEY_TAB as c_int,
        SDLK_F1 => KEY_F1 as c_int,
        SDLK_F2 => KEY_F2 as c_int,
        SDLK_F3 => KEY_F3 as c_int,
        SDLK_F4 => KEY_F4 as c_int,
        SDLK_F5 => KEY_F5 as c_int,
        SDLK_F6 => KEY_F6 as c_int,
        SDLK_F7 => KEY_F7 as c_int,
        SDLK_F8 => KEY_F8 as c_int,
        SDLK_F9 => KEY_F9 as c_int,
        SDLK_F10 => KEY_F10 as c_int,
        SDLK_F11 => KEY_F11 as c_int,
        SDLK_F12 => KEY_F12 as c_int,
        SDLK_BACKSPACE | SDLK_DELETE => KEY_BACKSPACE as c_int,
        SDLK_PAUSE => KEY_PAUSE as c_int,
        SDLK_EQUALS => KEY_EQUALS as c_int,
        SDLK_MINUS => KEY_MINUS as c_int,
        SDLK_LSHIFT | SDLK_RSHIFT => KEY_RSHIFT as c_int,
        SDLK_LCTRL | SDLK_RCTRL => KEY_RCTRL as c_int,
        SDLK_LALT | SDLK_RALT => KEY_RALT as c_int,
        _ => {
            if (b'A' as u32..=b'Z' as u32).contains(&key) {
                (key + (b'a' as u32 - b'A' as u32)) as c_int
            } else if (32..=126).contains(&key) {
                key as c_int
            } else {
                0
            }
        }
    }
}

static mut FIRST_TIME: bool = true;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_InitGraphics() {
    unsafe {
        if !FIRST_TIME {
            return;
        }
        FIRST_TIME = false;

        if !SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS) {
            let err = CStr::from_ptr(SDL_GetError()).to_string_lossy();
            doom_error(&format!("SDL_Init failed: {}", err));
        }

        window = SDL_CreateWindow(
            b"FerrumMolibdenDOOM\0".as_ptr() as *const c_char,
            (SCR_WIDTH * 3) as c_int,
            (SCR_HEIGHT * 3) as c_int,
            SDL_WINDOW_RESIZABLE,
        );
        if window.is_null() {
            let err = CStr::from_ptr(SDL_GetError()).to_string_lossy();
            doom_error(&format!("SDL_CreateWindow failed: {}", err));
        }

        renderer = SDL_CreateRenderer(window, ptr::null());
        if renderer.is_null() {
            let err = CStr::from_ptr(SDL_GetError()).to_string_lossy();
            doom_error(&format!("SDL_CreateRenderer failed: {}", err));
        }

        SDL_SetRenderLogicalPresentation(
            renderer,
            SCR_WIDTH as c_int,
            SCR_HEIGHT as c_int,
            SDL_LOGICAL_PRESENTATION_LETTERBOX,
        );

        texture = SDL_CreateTexture(
            renderer,
            SDL_PIXELFORMAT_ARGB8888,
            SDL_TEXTUREACCESS_STREAMING,
            SCR_WIDTH as c_int,
            SCR_HEIGHT as c_int,
        );
        if texture.is_null() {
            let err = CStr::from_ptr(SDL_GetError()).to_string_lossy();
            doom_error(&format!("SDL_CreateTexture failed: {}", err));
        }

        SDL_SetTextureScaleMode(texture, SDL_SCALEMODE_NEAREST);
        SDL_ShowWindow(window);

        let scr_ptr = (&raw mut screens) as *mut *mut u8;
        if (*scr_ptr).is_null() {
            let buf = Box::leak(vec![0u8; SCR_PIXELS].into_boxed_slice());
            *scr_ptr = buf.as_mut_ptr();
        }

        set_mouse_grab(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ShutdownGraphics() {
    unsafe {
        if !texture.is_null() {
            SDL_DestroyTexture(texture);
            texture = ptr::null_mut();
        }
        if !renderer.is_null() {
            SDL_DestroyRenderer(renderer);
            renderer = ptr::null_mut();
        }
        if !window.is_null() {
            SDL_DestroyWindow(window);
            window = ptr::null_mut();
        }
        SDL_Quit();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_SetPalette(mut palette: *mut u8) {
    unsafe {
        let gamma_idx = usegamma as usize;
        let pal_dst = (&raw mut palette_argb) as *mut u32;
        for i in 0..256usize {
            let r_in = *palette as usize;
            palette = palette.add(1);
            let g_in = *palette as usize;
            palette = palette.add(1);
            let b_in = *palette as usize;
            palette = palette.add(1);

            let r = gammatable[gamma_idx][r_in] as u32;
            let g = gammatable[gamma_idx][g_in] as u32;
            let b = gammatable[gamma_idx][b_in] as u32;
            *pal_dst.add(i) = 0xFF00_0000u32 | (r << 16) | (g << 8) | b;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_UpdateNoBlit() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_FinishUpdate() {
    unsafe {
        let scr0 = *(&raw const screens as *const *mut u8);
        if texture.is_null() || scr0.is_null() {
            return;
        }

        let pal_ptr = (&raw const palette_argb) as *const u32;
        let fb_ptr = (&raw mut frame_buffer) as *mut u32;

        for i in 0..SCR_PIXELS {
            let idx = *scr0.add(i) as usize;
            *fb_ptr.add(i) = *pal_ptr.add(idx);
        }

        SDL_UpdateTexture(
            texture,
            ptr::null(),
            fb_ptr as *const c_void,
            (SCR_WIDTH * 4) as c_int,
        );
        SDL_RenderClear(renderer);
        SDL_RenderTexture(renderer, texture, ptr::null(), ptr::null());
        SDL_RenderPresent(renderer);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_ReadScreen(scr: *mut u8) {
    unsafe {
        let scr0 = *(&raw const screens as *const *mut u8);
        ptr::copy_nonoverlapping(scr0, scr, SCR_PIXELS);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_StartTic() {
    unsafe {
        let mut sdl_event: SDL_Event = std::mem::zeroed();
        let mut doom_event = event_t {
            type_: ev_keydown,
            data1: 0,
            data2: 0,
            data3: 0,
        };

        while SDL_PollEvent(&raw mut sdl_event) {
            match sdl_event.type_ {
                SDL_EVENT_QUIT => {
                    I_Quit();
                }
                SDL_EVENT_WINDOW_FOCUS_LOST => {
                    set_mouse_grab(0);
                }
                SDL_EVENT_WINDOW_FOCUS_GAINED => {
                    set_mouse_grab(1);
                }
                SDL_EVENT_KEY_DOWN => {
                    let key = sdl_event.key.key;
                    if key == SDLK_F11 {
                        let is_fs = (SDL_GetWindowFlags(window) & SDL_WINDOW_FULLSCREEN) != 0;
                        SDL_SetWindowFullscreen(window, !is_fs);
                        continue;
                    }
                    doom_event.type_ = ev_keydown;
                    doom_event.data1 = translate_key(key);
                    if doom_event.data1 != 0 {
                        D_PostEvent((&raw mut doom_event) as *mut _);
                    }
                }
                SDL_EVENT_KEY_UP => {
                    doom_event.type_ = ev_keyup;
                    doom_event.data1 = translate_key(sdl_event.key.key);
if doom_event.data1 != 0 {
D_PostEvent((&raw mut doom_event) as *mut _);
}
}
SDL_EVENT_MOUSE_BUTTON_DOWN => {
if mouse_grabbed == 0 {
set_mouse_grab(1);
}
let buttons = SDL_GetMouseState(ptr::null_mut(), ptr::null_mut());
doom_event.type_ = ev_mouse;
doom_event.data1 = (((buttons & SDL_BUTTON_LMASK) != 0) as c_int)
| ((((buttons & SDL_BUTTON_RMASK) != 0) as c_int) << 1)
| ((((buttons & SDL_BUTTON_MMASK) != 0) as c_int) << 2);
doom_event.data2 = 0;
doom_event.data3 = 0;
D_PostEvent((&raw mut doom_event) as *mut _);
}
SDL_EVENT_MOUSE_BUTTON_UP => {
let buttons = SDL_GetMouseState(ptr::null_mut(), ptr::null_mut());
doom_event.type_ = ev_mouse;
doom_event.data1 = (((buttons & SDL_BUTTON_LMASK) != 0) as c_int)
| ((((buttons & SDL_BUTTON_RMASK) != 0) as c_int) << 1)
| ((((buttons & SDL_BUTTON_MMASK) != 0) as c_int) << 2);
doom_event.data2 = 0;
doom_event.data3 = 0;
D_PostEvent((&raw mut doom_event) as *mut _);
}
SDL_EVENT_MOUSE_MOTION => {
if mouse_grabbed == 0 {
continue;
}
let state = sdl_event.motion.state;
doom_event.type_ = ev_mouse;
doom_event.data1 = (((state & SDL_BUTTON_LMASK) != 0) as c_int)
| ((((state & SDL_BUTTON_RMASK) != 0) as c_int) << 1)
| ((((state & SDL_BUTTON_MMASK) != 0) as c_int) << 2);
doom_event.data2 = (sdl_event.motion.xrel * 3.0f32) as c_int;
doom_event.data3 = 0;
D_PostEvent((&raw mut doom_event) as *mut _);
}
_ => {}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_StartFrame() {}