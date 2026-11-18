#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;

use std::ffi::{c_int, c_void};

use crate::p_mobj::{P_MobjThinker, P_SpawnMobj};
use crate::p_tick::thinkercap;
use crate::r_bsp::subsector_t;
use crate::tables::{ANGLETOFINESHIFT, FINEANGLES, finesine};

const QUARTER_FINE: usize = FINEANGLES / 4;

unsafe extern "C" {
    fn P_TeleportMove(thing: *mut mobj_t, x: i32, y: i32) -> c_int;
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);

    static mut numsectors: c_int;
    static mut sectors: *mut sector_t;
}

/// Безопасная логика телепортации объекта `thing` через линию `line`.
pub fn ev_teleport(line: &line_t, side: c_int, thing: &mut mobj_t) -> bool {
    // Не телепортируем снаряды (ракеты, фаерболы) и при выходе с обратной стороны линии
    if (thing.flags as u32 & MF_MISSILE) != 0 || side == 1 {
        return false;
    }

    let tag = line.tag;
    let mobj_thinker_addr = P_MobjThinker as *const ();

    let secs = unsafe {
        if sectors.is_null() || numsectors <= 0 {
            return false;
        }
        std::slice::from_raw_parts(sectors, numsectors as usize)
    };

    for (i, sec_i) in secs.iter().enumerate() {
        if sec_i.tag != tag {
            continue;
        }

        let cap = &raw mut thinkercap;
        let mut thinker = unsafe { (*cap).next };

        while thinker != cap {
            let th_ref = unsafe { &*thinker };
            let next_thinker = th_ref.next;

            let is_mobj = match unsafe { th_ref.function.acp1 } {
                Some(func) => (func as *const ()) == mobj_thinker_addr,
                None => false,
            };

            if !is_mobj {
                thinker = next_thinker;
                continue;
            }

            let m = unsafe { &*(thinker as *const mobj_t) };
            if m.type_ != MT_TELEPORTMAN {
                thinker = next_thinker;
                continue;
            }

            let sec_idx_match = unsafe {
                (m.subsector as *const subsector_t)
                    .as_ref()
                    .and_then(|sub| (sub.sector as *const sector_t).as_ref())
                    .map(|sec| (sec as *const sector_t).offset_from(secs.as_ptr()) as usize == i)
                    .unwrap_or(false)
            };

            if !sec_idx_match {
                thinker = next_thinker;
                continue;
            }

            let oldx = thing.x;
            let oldy = thing.y;
            let oldz = thing.z;

            if unsafe { P_TeleportMove(thing as *mut mobj_t, m.x, m.y) } == 0 {
                return false;
            }

            thing.z = thing.floorz;
            if let Some(player) = unsafe { thing.player.as_mut() } {
                player.viewz = thing.z + player.viewheight;
                thing.reactiontime = 18;
            }

            unsafe {
                let fog1 = P_SpawnMobj(oldx, oldy, oldz, MT_TFOG);
                S_StartSound(fog1 as *mut c_void, sfx_telept as c_int);

                let an = (m.angle >> ANGLETOFINESHIFT) as usize;
                let dest_x = m.x + 20 * finesine[an + QUARTER_FINE];
                let dest_y = m.y + 20 * finesine[an];
                let fog2 = P_SpawnMobj(dest_x, dest_y, thing.z, MT_TFOG);
                S_StartSound(fog2 as *mut c_void, sfx_telept as c_int);
            }

            thing.angle = m.angle;
            thing.momx = 0;
            thing.momy = 0;
            thing.momz = 0;

            return true;
        }
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_Teleport(
    line: *mut line_t,
    side: c_int,
    thing: *mut mobj_t,
) -> c_int {
    let (Some(ln), Some(th)) = (unsafe { line.as_ref() }, unsafe { thing.as_mut() }) else {
        return 0;
    };
    ev_teleport(ln, side, th) as c_int
}
