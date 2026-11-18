#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

const NUM_WEAPONS: usize = NUMWEAPONS as usize;

pub static weaponinfo: [weaponinfo_t; NUM_WEAPONS] = [
    // 0: fist
    weaponinfo_t {
        ammo: am_noammo,
        upstate: S_PUNCHUP as i32,
        downstate: S_PUNCHDOWN as i32,
        readystate: S_PUNCH as i32,
        atkstate: S_PUNCH1 as i32,
        flashstate: S_NULL as i32,
    },
    // 1: pistol
    weaponinfo_t {
        ammo: am_clip,
        upstate: S_PISTOLUP as i32,
        downstate: S_PISTOLDOWN as i32,
        readystate: S_PISTOL as i32,
        atkstate: S_PISTOL1 as i32,
        flashstate: S_PISTOLFLASH as i32,
    },
    // 2: shotgun
    weaponinfo_t {
        ammo: am_shell,
        upstate: S_SGUNUP as i32,
        downstate: S_SGUNDOWN as i32,
        readystate: S_SGUN as i32,
        atkstate: S_SGUN1 as i32,
        flashstate: S_SGUNFLASH1 as i32,
    },
    // 3: chaingun
    weaponinfo_t {
        ammo: am_clip,
        upstate: S_CHAINUP as i32,
        downstate: S_CHAINDOWN as i32,
        readystate: S_CHAIN as i32,
        atkstate: S_CHAIN1 as i32,
        flashstate: S_CHAINFLASH1 as i32,
    },
    // 4: missile launcher
    weaponinfo_t {
        ammo: am_misl,
        upstate: S_MISSILEUP as i32,
        downstate: S_MISSILEDOWN as i32,
        readystate: S_MISSILE as i32,
        atkstate: S_MISSILE1 as i32,
        flashstate: S_MISSILEFLASH1 as i32,
    },
    // 5: plasma rifle
    weaponinfo_t {
        ammo: am_cell,
        upstate: S_PLASMAUP as i32,
        downstate: S_PLASMADOWN as i32,
        readystate: S_PLASMA as i32,
        atkstate: S_PLASMA1 as i32,
        flashstate: S_PLASMAFLASH1 as i32,
    },
    // 6: bfg 9000
    weaponinfo_t {
        ammo: am_cell,
        upstate: S_BFGUP as i32,
        downstate: S_BFGDOWN as i32,
        readystate: S_BFG as i32,
        atkstate: S_BFG1 as i32,
        flashstate: S_BFGFLASH1 as i32,
    },
    // 7: chainsaw
    weaponinfo_t {
        ammo: am_noammo,
        upstate: S_SAWUP as i32,
        downstate: S_SAWDOWN as i32,
        readystate: S_SAW as i32,
        atkstate: S_SAW1 as i32,
        flashstate: S_NULL as i32,
    },
    // 8: super shotgun
    weaponinfo_t {
        ammo: am_shell,
        upstate: S_DSGUNUP as i32,
        downstate: S_DSGUNDOWN as i32,
        readystate: S_DSGUN as i32,
        atkstate: S_DSGUN1 as i32,
        flashstate: S_DSGUNFLASH1 as i32,
    },
];
