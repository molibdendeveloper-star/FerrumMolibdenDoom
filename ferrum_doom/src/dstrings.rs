#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::c_char;
use std::ptr;

const QUIT_MESSAGES_COUNT: usize = (NUM_QUITMESSAGES as usize) + 1;

const RAW_ENDMSG: [&[u8]; 20] = [
    b"are you sure you want to\nquit this great game?\0",
    b"please don't leave, there's more\ndemons to toast!\0",
    b"let's beat it -- this is turning\ninto a bloodbath!\0",
    b"i wouldn't leave if i were you.\ndos is much worse.\0",
    b"you're trying to say you like dos\nbetter than me, right?\0",
    b"don't leave yet -- there's a\ndemon around that corner!\0",
    b"ya know, next time you come in here\ni'm gonna toast ya.\0",
    b"go ahead and leave. see if i care.you want to quit?\nthen, thou hast lost an eighth!\0",
    b"don't go now, there's a \ndimensional shambler waiting\nat the dos prompt!\0",
    b"get outta here and go back\nto your boring programs.\0",
    b"if i were your boss, i'd \n deathmatch ya in a minute!\0",
    b"look, bud. you leave now\nand you forfeit your body count!\0",
    b"just leave. when you come\nback, i'll be waiting with a bat.\0",
    b"you're lucky i don't smack\nyou for thinking about leaving.fuck you, pussy!\nget the fuck out!\0",
    b"you quit and i'll jizz\nin your cystholes!\0",
    b"if you leave, i'll make\nthe lord drink my jizz.\0",
    b"hey, ron! can we say\n'fuck' in the game?\0",
    b"i'd leave: this is just\nmore monsters and levels.\nwhat a load.\0",
    b"suck it down, asshole!\nyou're a fucking wimp!\0",
    b"don't quit now! we're \nstill spending your money!\0",
];

const LAST_DEBUG_MSG: &[u8] = b"THIS IS NO MESSAGE!\nPage intentionally left blank.\0";

#[unsafe(no_mangle)]
pub static mut endmsg: [*const c_char; QUIT_MESSAGES_COUNT] = [ptr::null(); QUIT_MESSAGES_COUNT];

extern "C" fn init_endmsg() {
    unsafe {
        let dst = (&raw mut endmsg) as *mut *const c_char;
        let copy_count = RAW_ENDMSG.len().min(QUIT_MESSAGES_COUNT.saturating_sub(2));
        for i in 0..copy_count {
            *dst.add(i) = RAW_ENDMSG[i].as_ptr() as *const c_char;
        }
        if copy_count < QUIT_MESSAGES_COUNT {
            *dst.add(copy_count) = LAST_DEBUG_MSG.as_ptr() as *const c_char;
        }
    }
}

#[used]
#[unsafe(link_section = ".init_array")]
static INIT_ENDMSG_HOOK: extern "C" fn() = init_endmsg;
