#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_things::player_t;

use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};

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

static LEVEL_TIME: AtomicI32 = AtomicI32::new(0);

#[inline]
pub fn get_leveltime() -> c_int {
    LEVEL_TIME.load(Ordering::Relaxed)
}

#[inline]
pub fn set_leveltime(val: c_int) {
    LEVEL_TIME.store(val, Ordering::Relaxed);
}

#[inline]
pub fn inc_leveltime() {
    LEVEL_TIME.fetch_add(1, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub static mut thinkercap: thinker_t = thinker_t {
    prev: ptr::null_mut(),
    next: ptr::null_mut(),
    function: actionf_t { acv: None },
};

/// Проверяет, помечен ли thinker на удаление (без UB-трансмутации -1 в указатель на функцию).
#[inline]
pub fn is_thinker_removed(thinker: &thinker_t) -> bool {
    unsafe { thinker.function.acv.is_none() && thinker.prev.is_null() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InitThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        (*cap).prev = cap;
        (*cap).next = cap;
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RemoveThinker(thinker: *mut thinker_t) {
    unsafe {
        let next = (*thinker).next;
        let prev = (*thinker).prev;
        if !prev.is_null() && !next.is_null() {
            (*next).prev = prev;
            (*prev).next = next;
        }
        // Помечаем узел как удалённый: function = None, prev = null
        (*thinker).function.acv = None;
        (*thinker).prev = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_AllocateThinker(_thinker: *mut thinker_t) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_RunThinkers() {
    unsafe {
        let cap = &raw mut thinkercap;
        let mut current = (*cap).next;

        while current != cap {
            let next = (*current).next;

            if is_thinker_removed(&*current) {
                Z_Free(current as *mut c_void);
            } else if let Some(func) = (*current).function.acp1 {
                func(current as *mut c_void);
            }

            current = next;
        }
    }
}

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

        inc_leveltime();
    }
}
