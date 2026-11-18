#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::c_int;

#[unsafe(no_mangle)]
pub static mut gamemode: GameMode_t = indetermined;

#[unsafe(no_mangle)]
pub static mut gamemission: GameMission_t = doom;

#[unsafe(no_mangle)]
pub static mut language: Language_t = english;

#[unsafe(no_mangle)]
pub static mut modifiedgame: c_int = 0;
