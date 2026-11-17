#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_int, c_void};
use std::mem::transmute;
use std::ptr;

use crate::z_zone::Z_Free;

const NUM_PLAYERS: usize = MAXPLAYERS as usize;

unsafe extern "C" {
    fn P_PlayerThink(player: *mut player_t);
    fn P_UpdateSpecials();
    fn P_RespawnSpecials();

    static mut paused: c_int;
    static mut netgame: c_int;
    static mut menuactive: c_int;
    static mut demoplayback: c_int;
    static mut consoleplayer: c_int;
    static mut playeringame: [c_int; NUM_PLAYERS];
    static mut players: [player_t; NUM_PLAYERS];
}

// --- Глобальные переменные p_tick.c ---
#[unsafe(no_mangle)]
pub static mut leveltime: c_int = 0;

#[unsafe(no_mangle)]
pub static mut thinkercap: thinker_t = unsafe { std::mem::zeroed() };

#[inline]
fn removed_thinker_fn() -> Option<unsafe extern "C" fn()> {
    unsafe { transmute::<isize, Option<unsafe extern "C" fn()>>(-1) }
}

// --- P_InitThinkers ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InitThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        (*cap).prev = cap;
        (*cap).next = cap;
    }
}

// --- P_AddThinker ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AddThinker(thinker: *mut thinker_t) {
    unsafe {
        let cap = &raw mut thinkercap;
        let prev = (*cap).prev;
        (*prev).next = thinker;
        (*thinker).next = cap;
        (*thinker).prev = prev;
        (*cap).prev = thinker;
    }
}

// --- P_RemoveThinker ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RemoveThinker(thinker: *mut thinker_t) {
    unsafe {
        (*thinker).function.acv = removed_thinker_fn();
    }
}

// --- P_AllocateThinker ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AllocateThinker(_thinker: *mut thinker_t) {
    // NOP в оригинальном DOOM
}

// --- P_RunThinkers ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RunThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        let mut current = (*cap).next;
        let tombstone = removed_thinker_fn();

        while current != cap {
            let next = (*current).next;

            if (*current).function.acv == tombstone {
                let prev = (*current).prev;
                (*next).prev = prev;
                (*prev).next = next;
                Z_Free(current as *mut c_void);
            } else if let Some(func) = (*current).function.acp1 {
                func(current as *mut c_void);
            }

            current = next;
        }
    }
}

// --- P_Ticker ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_Ticker() {
    unsafe {
        if paused != 0 {
            return;
        }

        let players_ptr = (&raw mut players) as *mut player_t;
        if netgame == 0
            && menuactive != 0
            && demoplayback == 0
            && (*players_ptr.add(consoleplayer as usize)).viewz != 1
        {
            return;
        }

        let in_game_ptr = (&raw const playeringame) as *const c_int;
        for i in 0..NUM_PLAYERS {
            if *in_game_ptr.add(i) != 0 {
                P_PlayerThink(players_ptr.add(i));
            }
        }

        P_RunThinkers();
        P_UpdateSpecials();
        P_RespawnSpecials();

        leveltime += 1;
    }
}
