#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;

use std::ffi::{c_int, c_short, c_void};
use std::mem::size_of;
use std::ptr;

use crate::m_random::p_random;
use crate::p_setup::{numsectors, sectors};
use crate::p_spec::{P_FindMinSurroundingLight, P_FindSectorFromLineTag, getNextSector};
use crate::p_tick::P_AddThinker;
use crate::z_zone::{PU_LEVSPEC, Z_Malloc};

unsafe fn alloc_light_thinker<T>(
    thinker_fn: unsafe extern "C" fn(*mut T),
    get_thinker: impl FnOnce(&mut T) -> &mut crate::p_tick::thinker_t,
) -> &'static mut T {
    unsafe {
        let ptr = Z_Malloc(size_of::<T>() as c_int, PU_LEVSPEC, ptr::null_mut()) as *mut T;
        let obj = &mut *ptr;
        let thinker = get_thinker(obj);
        P_AddThinker(thinker as *mut crate::p_tick::thinker_t);
        let acp1_fn: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(thinker_fn);
        thinker.function.acp1 = Some(acp1_fn);
        obj
    }
}

pub fn t_fire_flicker(flick: &mut fireflicker_t) {
    flick.count -= 1;
    if flick.count != 0 {
        return;
    }

    let Some(sec) = (unsafe { flick.sector.as_mut() }) else {
        return;
    };

    let amount = ((p_random() & 3) * 16) as c_short;
    let min_l = flick.minlight as c_short;
    let max_l = flick.maxlight as c_short;

    if sec.lightlevel - amount < min_l {
        sec.lightlevel = min_l;
    } else {
        sec.lightlevel = max_l - amount;
    }

    flick.count = 4;
}

pub fn t_light_flash(flash: &mut lightflash_t) {
    flash.count -= 1;
    if flash.count != 0 {
        return;
    }

    let Some(sec) = (unsafe { flash.sector.as_mut() }) else {
        return;
    };

    let max_l = flash.maxlight as c_short;
    let min_l = flash.minlight as c_short;

    if sec.lightlevel == max_l {
        sec.lightlevel = min_l;
        flash.count = (p_random() & flash.mintime) + 1;
    } else {
        sec.lightlevel = max_l;
        flash.count = (p_random() & flash.maxtime) + 1;
    }
}

pub fn t_strobe_flash(flash: &mut strobe_t) {
    flash.count -= 1;
    if flash.count != 0 {
        return;
    }

    let Some(sec) = (unsafe { flash.sector.as_mut() }) else {
        return;
    };

    let min_l = flash.minlight as c_short;
    let max_l = flash.maxlight as c_short;

    if sec.lightlevel == min_l {
        sec.lightlevel = max_l;
        flash.count = flash.brighttime;
    } else {
        sec.lightlevel = min_l;
        flash.count = flash.darktime;
    }
}

pub fn t_glow(g: &mut glow_t) {
    let Some(sec) = (unsafe { g.sector.as_mut() }) else {
        return;
    };

    let speed = GLOWSPEED as c_short;
    let min_l = g.minlight as c_short;
    let max_l = g.maxlight as c_short;

    match g.direction {
        -1 => {
            sec.lightlevel -= speed;
            if sec.lightlevel <= min_l {
                sec.lightlevel += speed;
                g.direction = 1;
            }
        }
        1 => {
            sec.lightlevel += speed;
            if sec.lightlevel >= max_l {
                sec.lightlevel -= speed;
                g.direction = -1;
            }
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_FireFlicker(flick: *mut fireflicker_t) {
    if let Some(f) = unsafe { flick.as_mut() } {
        t_fire_flicker(f);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnFireFlicker(sector: *mut sector_t) {
    let Some(sec) = (unsafe { sector.as_mut() }) else { return };
    sec.special = 0;

    let flick = unsafe {
        alloc_light_thinker(T_FireFlicker, |f: &mut fireflicker_t| {
            &mut *(&raw mut f.thinker as *mut crate::p_tick::thinker_t)
        })
    };

    flick.sector = sector;
    flick.maxlight = sec.lightlevel as c_int;
    let min_surr = unsafe {
        P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            sec.lightlevel as c_int,
        )
    };
    flick.minlight = min_surr + 16;
    flick.count = 4;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_LightFlash(flash: *mut lightflash_t) {
    if let Some(f) = unsafe { flash.as_mut() } {
        t_light_flash(f);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnLightFlash(sector: *mut sector_t) {
    let Some(sec) = (unsafe { sector.as_mut() }) else { return };
    sec.special = 0;

    let flash = unsafe {
        alloc_light_thinker(T_LightFlash, |f: &mut lightflash_t| {
            &mut *(&raw mut f.thinker as *mut crate::p_tick::thinker_t)
        })
    };

    flash.sector = sector;
    flash.maxlight = sec.lightlevel as c_int;
    flash.minlight = unsafe {
        P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            sec.lightlevel as c_int,
        )
    };
    flash.maxtime = 64;
    flash.mintime = 7;
    flash.count = (p_random() & flash.maxtime) + 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_StrobeFlash(flash: *mut strobe_t) {
    if let Some(f) = unsafe { flash.as_mut() } {
        t_strobe_flash(f);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnStrobeFlash(
    sector: *mut sector_t,
    fast_or_slow: c_int,
    in_sync: c_int,
) {
    let Some(sec) = (unsafe { sector.as_mut() }) else { return };

    let flash = unsafe {
        alloc_light_thinker(T_StrobeFlash, |f: &mut strobe_t| {
            &mut *(&raw mut f.thinker as *mut crate::p_tick::thinker_t)
        })
    };

    flash.sector = sector;
    flash.darktime = fast_or_slow;
    flash.brighttime = STROBEBRIGHT as c_int;
    flash.maxlight = sec.lightlevel as c_int;

    let min_surr = unsafe {
        P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            sec.lightlevel as c_int,
        )
    };
    flash.minlight = if min_surr == flash.maxlight {
        0
    } else {
        min_surr
    };

    sec.special = 0;
    flash.count = if in_sync == 0 {
        (p_random() & 7) + 1
    } else {
        1
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_StartLightStrobing(line: *mut line_t) {
    let secs = unsafe {
        if sectors.is_null() || numsectors <= 0 {
            return;
        }
        std::slice::from_raw_parts_mut(sectors as *mut sector_t, numsectors as usize)
    };

    let mut secnum = -1;
    loop {
        secnum = unsafe { P_FindSectorFromLineTag(line as *mut crate::p_spec::line_t, secnum) };
        if secnum < 0 {
            break;
        }

        let sec = &mut secs[secnum as usize];
        if !sec.specialdata.is_null() {
            continue;
        }

        unsafe {
            P_SpawnStrobeFlash(sec as *mut sector_t, SLOWDARK as c_int, 0);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_TurnTagLightsOff(line: *mut line_t) {
    let Some(ln) = (unsafe { line.as_ref() }) else { return };
    let tag = ln.tag;

    let secs = unsafe {
        if sectors.is_null() || numsectors <= 0 {
            return;
        }
        std::slice::from_raw_parts_mut(sectors as *mut sector_t, numsectors as usize)
    };

    for sector in secs.iter_mut() {
        if sector.tag != tag {
            continue;
        }
        let mut min = sector.lightlevel;
        let sec_lines = unsafe { std::slice::from_raw_parts(sector.lines, sector.linecount as usize) };

        for &templine in sec_lines {
            let tsec = unsafe {
                getNextSector(
                    templine as *mut crate::p_spec::line_t,
                    (sector as *mut sector_t) as *mut crate::p_spec::sector_t,
                ) as *const sector_t
            };
            if let Some(adj) = unsafe { tsec.as_ref() } {
                if adj.lightlevel < min {
                    min = adj.lightlevel;
                }
            }
        }
        sector.lightlevel = min;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn EV_LightTurnOn(line: *mut line_t, mut bright: c_int) {
    let Some(ln) = (unsafe { line.as_ref() }) else { return };
    let tag = ln.tag;

    let secs = unsafe {
        if sectors.is_null() || numsectors <= 0 {
            return;
        }
        std::slice::from_raw_parts_mut(sectors as *mut sector_t, numsectors as usize)
    };

    for sector in secs.iter_mut() {
        if sector.tag != tag {
            continue;
        }
        if bright == 0 {
            let sec_lines =
                unsafe { std::slice::from_raw_parts(sector.lines, sector.linecount as usize) };
            for &templine in sec_lines {
                let temp = unsafe {
                    getNextSector(
                        templine as *mut crate::p_spec::line_t,
                        (sector as *mut sector_t) as *mut crate::p_spec::sector_t,
                    ) as *const sector_t
                };
                if let Some(adj) = unsafe { temp.as_ref() } {
                    if (adj.lightlevel as c_int) > bright {
                        bright = adj.lightlevel as c_int;
                    }
                }
            }
        }
        sector.lightlevel = bright as c_short;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn T_Glow(g: *mut glow_t) {
    if let Some(gl) = unsafe { g.as_mut() } {
        t_glow(gl);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_SpawnGlowingLight(sector: *mut sector_t) {
    let Some(sec) = (unsafe { sector.as_mut() }) else { return };

    let g = unsafe {
        alloc_light_thinker(T_Glow, |gl: &mut glow_t| {
            &mut *(&raw mut gl.thinker as *mut crate::p_tick::thinker_t)
        })
    };

    g.sector = sector;
    g.minlight = unsafe {
        P_FindMinSurroundingLight(
            sector as *mut crate::p_spec::sector_t,
            sec.lightlevel as c_int,
        )
    };
    g.maxlight = sec.lightlevel as c_int;
    g.direction = -1;
    sec.special = 0;
}
