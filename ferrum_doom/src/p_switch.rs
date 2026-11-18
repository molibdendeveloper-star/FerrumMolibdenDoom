#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CString, c_char, c_int, c_void};
use std::ptr;

use crate::r_data::R_CheckTextureNumForName;


pub type bwhere_e = u32;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct button_t {
    pub line: *mut line_t,
    pub where_: bwhere_e,
    pub btexture: c_int,
    pub btimer: c_int,
    pub soundorg: *mut mobj_t,
}
const BWHERE_TOP: u32 = 0;
const BWHERE_MIDDLE: u32 = 1;
const BWHERE_BOTTOM: u32 = 2;

const NUM_SWITCHES: usize = MAXSWITCHES as usize;
const NUM_BUTTONS: usize = MAXBUTTONS as usize;

const fn make_sw(n1: &[u8; 9], n2: &[u8; 9], ep: i16) -> switchlist_t {
    let mut a = [0 as c_char; 9];
    let mut b = [0 as c_char; 9];
    let mut i = 0;
    while i < 9 {
        a[i] = n1[i] as c_char;
        b[i] = n2[i] as c_char;
        i += 1;
    }
    switchlist_t {
        name1: a,
        name2: b,
        episode: ep,
    }
}

unsafe extern "C" {
    fn I_Error(error: *const c_char, ...);
    fn S_StartSound(origin: *mut c_void, sound_id: c_int);

    fn EV_VerticalDoor(line: *mut line_t, thing: *mut mobj_t);
    fn EV_BuildStairs(line: *mut line_t, stair_type: stair_e) -> c_int;
    fn EV_DoDonut(line: *mut line_t) -> c_int;
    fn EV_DoPlat(line: *mut line_t, plat_type: plat_e, amount: c_int) -> c_int;
    fn EV_DoFloor(line: *mut line_t, floortype: floor_e) -> c_int;
    fn EV_DoDoor(line: *mut line_t, door_type: vldoor_e) -> c_int;
    fn EV_DoCeiling(line: *mut line_t, ceil_type: ceiling_e) -> c_int;
    fn EV_DoLockedDoor(line: *mut line_t, door_type: vldoor_e, thing: *mut mobj_t) -> c_int;
    fn EV_LightTurnOn(line: *mut line_t, bright: c_int);

    fn G_ExitLevel();
    fn G_SecretExitLevel();

    static mut gamemode: GameMode_t;
    static mut sides: *mut side_t;
}

fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Switch error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

// --- Глобальные переменные p_switch.c ---
#[unsafe(no_mangle)]
pub static mut alphSwitchList: [switchlist_t; 41] = [
    // Doom shareware episode 1 switches
    make_sw(b"SW1BRCOM\0", b"SW2BRCOM\0", 1),
    make_sw(b"SW1BRN1\0\0", b"SW2BRN1\0\0", 1),
    make_sw(b"SW1BRN2\0\0", b"SW2BRN2\0\0", 1),
    make_sw(b"SW1BRNGN\0", b"SW2BRNGN\0", 1),
    make_sw(b"SW1BROWN\0", b"SW2BROWN\0", 1),
    make_sw(b"SW1COMM\0\0", b"SW2COMM\0\0", 1),
    make_sw(b"SW1COMP\0\0", b"SW2COMP\0\0", 1),
    make_sw(b"SW1DIRT\0\0", b"SW2DIRT\0\0", 1),
    make_sw(b"SW1EXIT\0\0", b"SW2EXIT\0\0", 1),
    make_sw(b"SW1GRAY\0\0", b"SW2GRAY\0\0", 1),
    make_sw(b"SW1GRAY1\0", b"SW2GRAY1\0", 1),
    make_sw(b"SW1METAL\0", b"SW2METAL\0", 1),
    make_sw(b"SW1PIPE\0\0", b"SW2PIPE\0\0", 1),
    make_sw(b"SW1SLAD\0\0", b"SW2SLAD\0\0", 1),
    make_sw(b"SW1STARG\0", b"SW2STARG\0", 1),
    make_sw(b"SW1STON1\0", b"SW2STON1\0", 1),
    make_sw(b"SW1STON2\0", b"SW2STON2\0", 1),
    make_sw(b"SW1STONE\0", b"SW2STONE\0", 1),
    make_sw(b"SW1STRTN\0", b"SW2STRTN\0", 1),
    // Doom registered episodes 2&3 switches
    make_sw(b"SW1BLUE\0\0", b"SW2BLUE\0\0", 2),
    make_sw(b"SW1CMT\0\0\0", b"SW2CMT\0\0\0", 2),
    make_sw(b"SW1GARG\0\0", b"SW2GARG\0\0", 2),
    make_sw(b"SW1GSTON\0", b"SW2GSTON\0", 2),
    make_sw(b"SW1HOT\0\0\0", b"SW2HOT\0\0\0", 2),
    make_sw(b"SW1LION\0\0", b"SW2LION\0\0", 2),
    make_sw(b"SW1SATYR\0", b"SW2SATYR\0", 2),
    make_sw(b"SW1SKIN\0\0", b"SW2SKIN\0\0", 2),
    make_sw(b"SW1VINE\0\0", b"SW2VINE\0\0", 2),
    make_sw(b"SW1WOOD\0\0", b"SW2WOOD\0\0", 2),
    // Doom II switches
    make_sw(b"SW1PANEL\0", b"SW2PANEL\0", 3),
    make_sw(b"SW1ROCK\0\0", b"SW2ROCK\0\0", 3),
    make_sw(b"SW1MET2\0\0", b"SW2MET2\0\0", 3),
    make_sw(b"SW1WDMET\0", b"SW2WDMET\0", 3),
    make_sw(b"SW1BRIK\0\0", b"SW2BRIK\0\0", 3),
    make_sw(b"SW1MOD1\0\0", b"SW2MOD1\0\0", 3),
    make_sw(b"SW1ZIM\0\0\0", b"SW2ZIM\0\0\0", 3),
    make_sw(b"SW1STON6\0", b"SW2STON6\0", 3),
    make_sw(b"SW1TEK\0\0\0", b"SW2TEK\0\0\0", 3),
    make_sw(b"SW1MARB\0\0", b"SW2MARB\0\0", 3),
    make_sw(b"SW1SKULL\0", b"SW2SKULL\0", 3),
    // Терминатор списка
    make_sw(b"\0\0\0\0\0\0\0\0\0", b"\0\0\0\0\0\0\0\0\0", 0),
];

#[unsafe(no_mangle)]
pub static mut switchlist: [c_int; NUM_SWITCHES * 2] = [0; NUM_SWITCHES * 2];

#[unsafe(no_mangle)]
pub static mut numswitches: c_int = 0;

#[unsafe(no_mangle)]
pub static mut buttonlist: [button_t; NUM_BUTTONS] = [unsafe { std::mem::zeroed() }; NUM_BUTTONS];

// --- P_InitSwitchList ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_InitSwitchList() {
    unsafe {
        let mut episode: i16 = 1;
        if gamemode == registered || gamemode == retail {
            episode = 2;
        } else if gamemode == commercial {
            episode = 3;
        }

        let alph_ptr = (&raw const alphSwitchList) as *const switchlist_t;
        let sw_ptr = (&raw mut switchlist) as *mut c_int;
        let mut index = 0usize;

        for i in 0..NUM_SWITCHES {
            let entry = &*alph_ptr.add(i);
            if entry.episode == 0 {
                numswitches = (index / 2) as c_int;
                *sw_ptr.add(index) = -1;
                break;
            }

            if entry.episode <= episode {
                let tex1 = R_CheckTextureNumForName(entry.name1.as_ptr() as *mut c_char);
                let tex2 = R_CheckTextureNumForName(entry.name2.as_ptr() as *mut c_char);
                if tex1 != -1 && tex2 != -1 {
                    *sw_ptr.add(index) = tex1;
                    index += 1;
                    *sw_ptr.add(index) = tex2;
                    index += 1;
                }
            }
        }
    }
}

// --- P_StartButton ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_StartButton(
    line: *mut line_t,
    w: bwhere_e,
    texture: c_int,
    time: c_int,
) {
    unsafe {
        let btn_ptr = (&raw mut buttonlist) as *mut button_t;

        for i in 0..NUM_BUTTONS {
            let btn = btn_ptr.add(i);
            if (*btn).btimer != 0 && (*btn).line == line {
                return;
            }
        }

        for i in 0..NUM_BUTTONS {
            let btn = btn_ptr.add(i);
            if (*btn).btimer == 0 {
                (*btn).line = line;
                (*btn).where_ = w;
                (*btn).btexture = texture;
                (*btn).btimer = time;
                let fsec = (*line).frontsector;
                (*btn).soundorg = (&raw mut (*fsec).soundorg) as *mut mobj_t;
                return;
            }
        }

        doom_error("P_StartButton: no button slots left!");
    }
}

// --- P_ChangeSwitchTexture ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_ChangeSwitchTexture(line: *mut line_t, use_again: c_int) {
    unsafe {
        if use_again == 0 {
            (*line).special = 0;
        }

        let side0 = *(*line).sidenum.as_ptr() as usize;
        let side_ptr = sides.add(side0);

        let tex_top = (*side_ptr).toptexture as c_int;
        let tex_mid = (*side_ptr).midtexture as c_int;
        let tex_bot = (*side_ptr).bottomtexture as c_int;

        let sound = if (*line).special == 11 {
            sfx_swtchx as c_int
        } else {
            sfx_swtchn as c_int
        };

        let sw_ptr = (&raw const switchlist) as *const c_int;
        let btn_ptr = (&raw const buttonlist) as *const button_t;
        let btime = BUTTONTIME as c_int;

        for i in 0..((numswitches * 2) as usize) {
            let sw_tex = *sw_ptr.add(i);
            let opp_tex = *sw_ptr.add(i ^ 1) as i16;

            if sw_tex == tex_top {
                S_StartSound((*btn_ptr).soundorg as *mut c_void, sound);
                (*side_ptr).toptexture = opp_tex;
                if use_again != 0 {
                    P_StartButton(line, BWHERE_TOP, sw_tex, btime);
                }
                return;
            } else if sw_tex == tex_mid {
                S_StartSound((*btn_ptr).soundorg as *mut c_void, sound);
                (*side_ptr).midtexture = opp_tex;
                if use_again != 0 {
                    P_StartButton(line, BWHERE_MIDDLE, sw_tex, btime);
                }
                return;
            } else if sw_tex == tex_bot {
                S_StartSound((*btn_ptr).soundorg as *mut c_void, sound);
                (*side_ptr).bottomtexture = opp_tex;
                if use_again != 0 {
                    P_StartButton(line, BWHERE_BOTTOM, sw_tex, btime);
                }
                return;
            }
        }
    }
}

// --- P_UseSpecialLine ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn P_UseSpecialLine(
    thing: *mut mobj_t,
    line: *mut line_t,
    side: c_int,
) -> c_int {
    unsafe {
        if side != 0 {
            match (*line).special {
                124 => {}
                _ => return 0,
            }
        }

        if (*thing).player.is_null() {
            if ((*line).flags as u32 & ML_SECRET) != 0 {
                return 0;
            }

            match (*line).special {
                1 | 32 | 33 | 34 => {}
                _ => return 0,
            }
        }

        match (*line).special {
            // MANUALS
            1 | 26 | 27 | 28 | 31 | 32 | 33 | 34 | 117 | 118 => {
                EV_VerticalDoor(line, thing);
            }

            // SWITCHES
            7 => {
                if EV_BuildStairs(line, build8) != 0 {
                    P_ChangeSwitchTexture(line, 0);
                }
            }
            9 => {
                if EV_DoDonut(line) != 0 {
                    P_ChangeSwitchTexture(line, 0);
                }
            }
            11 => {
                P_ChangeSwitchTexture(line, 0);
                G_ExitLevel();
            }
            14 => {
                if EV_DoPlat(line, raiseAndChange, 32) != 0 {
                    P_ChangeSwitchTexture(line, 0);
                }
            }
            15 => {
                if EV_DoPlat(line, raiseAndChange, 24) != 0 {
                    P_ChangeSwitchTexture(line, 0);
                }
            }
            18 => {
                if EV_DoFloor(line, raiseFloorToNearest) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
20 => {
if EV_DoPlat(line, raiseToNearestAndChange, 0) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
21 => {
if EV_DoPlat(line, downWaitUpStay, 0) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
23 => {
if EV_DoFloor(line, lowerFloorToLowest) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
29 => {
if EV_DoDoor(line, normal) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
41 => {
if EV_DoCeiling(line, lowerToFloor) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
71 => {
if EV_DoFloor(line, turboLower) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
49 => {
if EV_DoCeiling(line, crushAndRaise) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
50 => {
if EV_DoDoor(line, close) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
51 => {
P_ChangeSwitchTexture(line, 0);
G_SecretExitLevel();
}
55 => {
if EV_DoFloor(line, raiseFloorCrush) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
101 => {
if EV_DoFloor(line, raiseFloor) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
102 => {
if EV_DoFloor(line, lowerFloor) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
103 => {
if EV_DoDoor(line, open) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
111 => {
if EV_DoDoor(line, blazeRaise) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
112 => {
if EV_DoDoor(line, blazeOpen) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
113 => {
if EV_DoDoor(line, blazeClose) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
122 => {
if EV_DoPlat(line, blazeDWUS, 0) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
127 => {
if EV_BuildStairs(line, turbo16) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
131 => {
if EV_DoFloor(line, raiseFloorTurbo) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
133 | 135 | 137 => {
if EV_DoLockedDoor(line, blazeOpen, thing) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
140 => {
if EV_DoFloor(line, raiseFloor512) != 0 {
P_ChangeSwitchTexture(line, 0);
}
}
// BUTTONS
42 => {
if EV_DoDoor(line, close) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
43 => {
if EV_DoCeiling(line, lowerToFloor) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
45 => {
if EV_DoFloor(line, lowerFloor) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
60 => {
if EV_DoFloor(line, lowerFloorToLowest) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
61 => {
if EV_DoDoor(line, open) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
62 => {
if EV_DoPlat(line, downWaitUpStay, 1) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
63 => {
if EV_DoDoor(line, normal) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
64 => {
if EV_DoFloor(line, raiseFloor) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
66 => {
if EV_DoPlat(line, raiseAndChange, 24) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
67 => {
if EV_DoPlat(line, raiseAndChange, 32) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
65 => {
if EV_DoFloor(line, raiseFloorCrush) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
68 => {
if EV_DoPlat(line, raiseToNearestAndChange, 0) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
69 => {
if EV_DoFloor(line, raiseFloorToNearest) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
70 => {
if EV_DoFloor(line, turboLower) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
114 => {
if EV_DoDoor(line, blazeRaise) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
115 => {
if EV_DoDoor(line, blazeOpen) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
116 => {
if EV_DoDoor(line, blazeClose) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
123 => {
if EV_DoPlat(line, blazeDWUS, 0) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
132 => {
if EV_DoFloor(line, raiseFloorTurbo) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
99 | 134 | 136 => {
if EV_DoLockedDoor(line, blazeOpen, thing) != 0 {
P_ChangeSwitchTexture(line, 1);
}
}
138 => {
EV_LightTurnOn(line, 255);
P_ChangeSwitchTexture(line, 1);
}
139 => {
EV_LightTurnOn(line, 35);
P_ChangeSwitchTexture(line, 1);
}
_ => {}
}
1
}
}