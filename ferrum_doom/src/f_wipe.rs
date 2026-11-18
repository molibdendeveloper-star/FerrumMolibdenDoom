#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

use std::ffi::c_int;
use std::sync::{Mutex, OnceLock};

use crate::i_video::I_ReadScreen;
use crate::m_random::m_random;
use crate::v_video::{V_DrawBlock, V_MarkRect, screens};

struct WipeState {
    go: bool,
    scr_start: Vec<u8>,
    scr_end: Vec<u8>,
    y_offsets: Vec<i32>,
}

impl WipeState {
    const fn new() -> Self {
        Self {
            go: false,
            scr_start: Vec::new(),
            scr_end: Vec::new(),
            y_offsets: Vec::new(),
        }
    }

    /// Преобразование массива пар пикселей (u16) из построчного формата в поколоночный.
    fn col_major_xform(buf: &mut [u8], half_width: usize, height: usize) {
        let total_pairs = half_width * height;
        if buf.len() < total_pairs * 2 {
            return;
        }
        let mut dest = vec![0u8; total_pairs * 2];
        for y in 0..height {
            for x in 0..half_width {
                let src_idx = (y * half_width + x) * 2;
                let dst_idx = (x * height + y) * 2;
                dest[dst_idx] = buf[src_idx];
                dest[dst_idx + 1] = buf[src_idx + 1];
            }
        }
        buf[..total_pairs * 2].copy_from_slice(&dest);
    }

    fn init_color_xform(&mut self, dst_scr: &mut [u8]) {
        let len = dst_scr.len().min(self.scr_start.len());
        dst_scr[..len].copy_from_slice(&self.scr_start[..len]);
    }

    fn do_color_xform(&self, dst_scr: &mut [u8], ticks: i32) -> bool {
        let mut changed = false;
        for (w, &ev_u8) in dst_scr.iter_mut().zip(self.scr_end.iter()) {
            let wv = *w as i32;
            let ev = ev_u8 as i32;
            if wv != ev {
                let newval = if wv > ev {
                    (wv - ticks).max(ev)
                } else {
                    (wv + ticks).min(ev)
                };
                *w = newval as u8;
                changed = true;
            }
        }
        !changed
    }

    fn init_melt(&mut self, dst_scr: &mut [u8], width: usize, height: usize) {
        let len = dst_scr.len().min(self.scr_start.len());
        dst_scr[..len].copy_from_slice(&self.scr_start[..len]);

        let half_w = width / 2;
        Self::col_major_xform(&mut self.scr_start, half_w, height);
        Self::col_major_xform(&mut self.scr_end, half_w, height);

        self.y_offsets.resize(width, 0);
        if !self.y_offsets.is_empty() {
            self.y_offsets[0] = -(m_random() % 16);
            for i in 1..width {
                let r = (m_random() % 3) - 1;
                let mut val = self.y_offsets[i - 1] + r;
                if val > 0 {
                    val = 0;
                } else if val == -16 {
                    val = -15;
                }
                self.y_offsets[i] = val;
            }
        }
    }

    fn do_melt(&mut self, dst_scr: &mut [u8], width: usize, height: usize, mut ticks: i32) -> bool {
        let mut done = true;
        let half_w = width / 2;
        let h_i32 = height as i32;

        while ticks > 0 {
            ticks -= 1;
            for i in 0..half_w {
                let yi = self.y_offsets[i];
                if yi < 0 {
                    self.y_offsets[i] = yi + 1;
                    done = false;
                } else if yi < h_i32 {
                    let mut dy = if yi < 16 { yi + 1 } else { 8 };
                    if yi + dy >= h_i32 {
                        dy = h_i32 - yi;
                    }

                    let yi_u = yi as usize;
                    let dy_u = dy as usize;

                    for k in 0..dy_u {
                        let s_idx = (i * height + yi_u + k) * 2;
                        let d_idx = ((yi_u + k) * half_w + i) * 2;
                        dst_scr[d_idx] = self.scr_end[s_idx];
                        dst_scr[d_idx + 1] = self.scr_end[s_idx + 1];
                    }

                    let new_yi = yi + dy;
                    self.y_offsets[i] = new_yi;

                    let new_yi_u = new_yi as usize;
                    let rem = (h_i32 - new_yi).max(0) as usize;
                    for k in 0..rem {
                        let s_idx = (i * height + k) * 2;
                        let d_idx = ((new_yi_u + k) * half_w + i) * 2;
                        dst_scr[d_idx] = self.scr_start[s_idx];
                        dst_scr[d_idx + 1] = self.scr_start[s_idx + 1];
                    }

                    done = false;
                }
            }
        }

        done
    }

    fn exit_melt(&mut self) {
        self.y_offsets.clear();
    }
}

static WIPE: OnceLock<Mutex<WipeState>> = OnceLock::new();

fn wipe_state() -> &'static Mutex<WipeState> {
    WIPE.get_or_init(|| Mutex::new(WipeState::new()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_StartScreen(
    _x: c_int,
    _y: c_int,
    width: c_int,
    height: c_int,
) -> c_int {
    let size = (width * height).max(0) as usize;
    let mut st = wipe_state().lock().unwrap();
    st.scr_start.resize(size, 0);
    unsafe {
        let scr_ptr = (&raw mut screens) as *mut *mut u8;
        let scr2 = *scr_ptr.add(2);
        if !scr2.is_null() {
            I_ReadScreen(scr2);
            st.scr_start
                .copy_from_slice(std::slice::from_raw_parts(scr2, size));
        } else {
            I_ReadScreen(st.scr_start.as_mut_ptr());
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_EndScreen(x: c_int, y: c_int, width: c_int, height: c_int) -> c_int {
    let size = (width * height).max(0) as usize;
    let mut st = wipe_state().lock().unwrap();
    st.scr_end.resize(size, 0);
    unsafe {
        let scr_ptr = (&raw mut screens) as *mut *mut u8;
        let scr3 = *scr_ptr.add(3);
        if !scr3.is_null() {
            I_ReadScreen(scr3);
            st.scr_end
                .copy_from_slice(std::slice::from_raw_parts(scr3, size));
        } else {
            I_ReadScreen(st.scr_end.as_mut_ptr());
        }
        V_DrawBlock(x, y, 0, width, height, st.scr_start.as_mut_ptr());
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wipe_ScreenWipe(
    wipeno: c_int,
    _x: c_int,
    _y: c_int,
    width: c_int,
    height: c_int,
    ticks: c_int,
) -> c_int {
    let w = width.max(0) as usize;
    let h = height.max(0) as usize;
    let size = w * h;

    let dst_scr = unsafe {
        let scr0 = *(&raw const screens as *const *mut u8);
        if scr0.is_null() {
            return 1;
        }
        std::slice::from_raw_parts_mut(scr0, size)
    };

    let mut st = wipe_state().lock().unwrap();
    if !st.go {
        st.go = true;
        if wipeno == 0 {
            st.init_color_xform(dst_scr);
        } else {
            st.init_melt(dst_scr, w, h);
        }
    }

    unsafe {
        V_MarkRect(0, 0, width, height);
    }

    let finished = if wipeno == 0 {
        st.do_color_xform(dst_scr, ticks)
    } else {
        st.do_melt(dst_scr, w, h, ticks)
    };

    if finished {
        st.go = false;
        if wipeno != 0 {
            st.exit_melt();
        }
    }

    (!st.go) as c_int
}
