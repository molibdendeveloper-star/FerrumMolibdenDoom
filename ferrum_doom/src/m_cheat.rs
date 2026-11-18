#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

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

static mut FIRST_TIME: bool = true;
static mut CHEAT_XLATE_TABLE: [u8; 256] = [0; 256];

// --- cht_CheckCheat ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cht_CheckCheat(cht: *mut cheatseq_t, key: c_char) -> c_int {
    unsafe {
        let mut rc = 0;

        if FIRST_TIME {
            FIRST_TIME = false;
            let table_ptr = (&raw mut CHEAT_XLATE_TABLE) as *mut u8;
            for i in 0..256usize {
                *table_ptr.add(i) = scramble(i as u8);
            }
        }

        if (*cht).p.is_null() {
            (*cht).p = (*cht).sequence;
        }

        if *(*cht).p == 0 {
            *(*cht).p = key as u8;
            (*cht).p = (*cht).p.add(1);
        } else {
            let table_ptr = (&raw const CHEAT_XLATE_TABLE) as *const u8;
            if *table_ptr.add((key as u8) as usize) == *(*cht).p {
                (*cht).p = (*cht).p.add(1);
            } else {
                (*cht).p = (*cht).sequence;
            }
        }

        if *(*cht).p == 1 {
            (*cht).p = (*cht).p.add(1);
        } else if *(*cht).p == 0xff {
            (*cht).p = (*cht).sequence;
            rc = 1;
        }

        rc
    }
}

// --- cht_GetParam ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cht_GetParam(cht: *mut cheatseq_t, mut buffer: *mut c_char) {
    unsafe {
        let mut p = (*cht).sequence;
        while *p != 1 {
            p = p.add(1);
        }
        p = p.add(1);

        loop {
            let c = *p;
            *buffer = c as c_char;
            buffer = buffer.add(1);
            *p = 0;
            p = p.add(1);

            if c == 0 || *p == 0xff {
                break;
            }
        }

        if *p == 0xff {
            *buffer = 0;
        }
    }
}
