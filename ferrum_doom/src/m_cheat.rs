#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

pub use crate::bindings::*;
use std::ffi::{c_char, c_int};

const fn scramble(a: u8) -> u8 {
    ((a & 1) << 7)
        .wrapping_add((a & 2) << 5)
        .wrapping_add(a & 4)
        .wrapping_add((a & 8) << 1)
        .wrapping_add((a & 16) >> 1)
        .wrapping_add(a & 32)
        .wrapping_add((a & 64) >> 5)
        .wrapping_add((a & 128) >> 7)
}

const CHEAT_XLATE_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = scramble(i as u8);
        i += 1;
    }
    table
};

/// Безопасная реализация проверки чит-кода поверх среза байтов.
pub fn check_cheat_bytes(seq: &mut [u8], pos: &mut usize, key: u8) -> bool {
    if *pos >= seq.len() {
        *pos = 0;
    }

    if seq[*pos] == 0 {
        seq[*pos] = key;
        *pos += 1;
    } else if CHEAT_XLATE_TABLE[key as usize] == seq[*pos] {
        *pos += 1;
    } else {
        *pos = 0;
    }

    if *pos < seq.len() {
        if seq[*pos] == 1 {
            *pos += 1;
        } else if seq[*pos] == 0xff {
            *pos = 0;
            return true;
        }
    }

    false
}

/// Безопасное извлечение параметра чит-кода (например, номера уровня для IDCLEV).
pub fn get_cheat_param_bytes(seq: &mut [u8], out: &mut [u8]) {
    let Some(marker_idx) = seq.iter().position(|&b| b == 1) else {
        return;
    };

    let mut p_idx = marker_idx + 1;
    let mut out_idx = 0;

    while p_idx < seq.len() && out_idx < out.len() {
        let c = seq[p_idx];
        out[out_idx] = c;
        out_idx += 1;
        seq[p_idx] = 0;
        p_idx += 1;

        if c == 0 || (p_idx < seq.len() && seq[p_idx] == 0xff) {
            break;
        }
    }

    if p_idx < seq.len() && seq[p_idx] == 0xff && out_idx < out.len() {
        out[out_idx] = 0;
    }
}

// --- Тонкий совместимый слой для st_stuff / am_map ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cht_CheckCheat(cht: *mut cheatseq_t, key: c_char) -> c_int {
    unsafe {
        if cht.is_null() || (*cht).sequence.is_null() {
            return 0;
        }
        let seq_ptr = (*cht).sequence;
        let mut len = 0usize;
        while *seq_ptr.add(len) != 0xff {
            len += 1;
        }
        len += 1; // включаем завершающий маркер 0xff

        let seq_slice = std::slice::from_raw_parts_mut(seq_ptr, len);
        let mut pos = if (*cht).p.is_null() {
            0
        } else {
            (*cht).p.offset_from(seq_ptr).max(0) as usize
        };

        let matched = check_cheat_bytes(seq_slice, &mut pos, key as u8);
        (*cht).p = seq_ptr.add(pos);
        matched as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cht_GetParam(cht: *mut cheatseq_t, buffer: *mut c_char) {
    unsafe {
        if cht.is_null() || (*cht).sequence.is_null() || buffer.is_null() {
            return;
        }
        let seq_ptr = (*cht).sequence;
        let mut len = 0usize;
        while *seq_ptr.add(len) != 0xff {
            len += 1;
        }
        len += 1;

        let seq_slice = std::slice::from_raw_parts_mut(seq_ptr, len);
        let out_slice = std::slice::from_raw_parts_mut(buffer as *mut u8, 16);
        get_cheat_param_bytes(seq_slice, out_slice);
    }
}
