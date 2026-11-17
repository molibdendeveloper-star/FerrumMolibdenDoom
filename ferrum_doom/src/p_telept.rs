#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::player_s;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_Teleport(
    line: *mut line_t,
    side: c_int,
    thing: *mut mobj_t,
) -> c_int {
    unsafe {
        // Не телепортируем снаряды (ракеты, фаерболы)
        if ((*thing).flags as u32 & MF_MISSILE) != 0 {
            return 0;
        }

        // Не телепортируем при выходе с обратной стороны линии
        if side == 1 {
            return 0;
        }

        let tag = (*line).tag;
        let mobj_thinker_addr = P_MobjThinker as *const ();
        let cap = &raw mut thinkercap;

        for i in 0..(numsectors as usize) {
            let sec_i = sectors.add(i);
            if (*sec_i).tag == tag {
                let mut thinker = (*cap).next;

                while thinker != cap {
                    let is_mobj = match (*thinker).function.acp1 {
                        Some(func) => (func as *const ()) == mobj_thinker_addr,
                        None => false,
                    };

                    if !is_mobj {
                        thinker = (*thinker).next;
                        continue;
                    }

                    let m = thinker as *mut mobj_t;
                    if (*m).type_ != MT_TELEPORTMAN {
                        thinker = (*thinker).next;
                        continue;
                    }

                    let sub = (*m).subsector as *mut subsector_t;
                    let sector = (*sub).sector as *mut sector_t;

                    if sector.offset_from(sectors) as usize != i {
                        thinker = (*thinker).next;
                        continue;
                    }

                    let oldx = (*thing).x;
                    let oldy = (*thing).y;
                    let oldz = (*thing).z;

                    if P_TeleportMove(thing, (*m).x, (*m).y) == 0 {
                        return 0;
                    }

                    (*thing).z = (*thing).floorz;
                    let player = (*thing).player;
                    if !player.is_null() {
                        (*player).viewz = (*thing).z + (*player).viewheight;
                    }

                    // Туман телепортации в исходной точке
                    let fog1 = P_SpawnMobj(oldx, oldy, oldz, MT_TFOG);
                    S_StartSound(fog1 as *mut c_void, sfx_telept as c_int);

                    // Туман телепортации чуть впереди точки назначения
                    let an = ((*m).angle >> ANGLETOFINESHIFT) as usize;
                    let dest_x = (*m).x + 20 * finesine[an + QUARTER_FINE];
                    let dest_y = (*m).y + 20 * finesine[an];
                    let fog2 = P_SpawnMobj(dest_x, dest_y, (*thing).z, MT_TFOG);
                    S_StartSound(fog2 as *mut c_void, sfx_telept as c_int);

                    // Небольшая задержка управления для игрока после телепорта
                    if !player.is_null() {
                        (*thing).reactiontime = 18;
                    }

                    (*thing).angle = (*m).angle;
                    (*thing).momx = 0;
                    (*thing).momy = 0;
                    (*thing).momz = 0;

                    return 1;
                }
            }
        }

        0
    }
}
