#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]
use crate::i_system::{I_GetTime, I_WaitVBL};
use crate::i_video::I_StartTic;

pub use crate::bindings::*;
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_short, c_uint};
use std::mem::{offset_of, size_of};
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::g_game::{
    G_BuildTiccmd, G_CheckDemoStatus, G_Ticker, consoleplayer, deathmatch, demoplayback,
    demorecording, displayplayer, gametic, netgame, playeringame, players, usergame,
};
use crate::m_menu::M_Ticker;
use crate::v_video::doom_error;

const NCMD_EXIT: u32 = 0x8000_0000;
const NCMD_RETRANSMIT: u32 = 0x4000_0000;
const NCMD_SETUP: u32 = 0x2000_0000;
const NCMD_KILL: u32 = 0x1000_0000;
const NCMD_CHECKSUM: u32 = 0x0fff_ffff;

const RESENDCOUNT: c_int = 10;
const PL_DRONE: u8 = 0x80;
const MAX_INT: c_int = 0x7fff_ffff;
const VERSION: u8 = 109;

const NUM_PLAYERS: usize = MAXPLAYERS as usize;
const MAX_NETNODES: usize = MAXNETNODES as usize;
const BACKUP_TICS: usize = BACKUPTICS as usize;
const MAX_EVENTS: usize = 64;
const CMD_SEND: c_short = 1;
const CMD_GET: c_short = 2;

const EMPTY_TICCMD: ticcmd_t = ticcmd_t {
    forwardmove: 0,
    sidemove: 0,
    angleturn: 0,
    consistancy: 0,
    chatchar: 0,
    buttons: 0,
};

unsafe extern "C" {
    
    
    fn I_NetCmd();
    fn I_InitNetwork();
    
    fn D_ProcessEvents();
    fn D_DoAdvanceDemo();

    static mut events: [event_t; MAX_EVENTS];
    static mut eventhead: c_int;
    static mut eventtail: c_int;

    static mut autostart: c_int;
    static mut startskill: skill_t;
    static mut startepisode: c_int;
    static mut startmap: c_int;
    static mut nomonsters: c_int;
    static mut respawnparm: c_int;
    static mut singletics: c_int;
    static mut advancedemo: c_int;
}

#[unsafe(no_mangle)]
pub static mut doomcom: *mut doomcom_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut netbuffer: *mut doomdata_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut localcmds: [ticcmd_t; BACKUP_TICS] = [EMPTY_TICCMD; BACKUP_TICS];

#[unsafe(no_mangle)]
pub static mut netcmds: [[ticcmd_t; BACKUP_TICS]; NUM_PLAYERS] =
    [[EMPTY_TICCMD; BACKUP_TICS]; NUM_PLAYERS];

#[unsafe(no_mangle)]
pub static mut nettics: [c_int; MAX_NETNODES] = [0; MAX_NETNODES];

#[unsafe(no_mangle)]
pub static mut nodeingame: [c_int; MAX_NETNODES] = [0; MAX_NETNODES];

#[unsafe(no_mangle)]
pub static mut remoteresend: [c_int; MAX_NETNODES] = [0; MAX_NETNODES];

#[unsafe(no_mangle)]
pub static mut resendto: [c_int; MAX_NETNODES] = [0; MAX_NETNODES];

#[unsafe(no_mangle)]
pub static mut resendcount: [c_int; MAX_NETNODES] = [0; MAX_NETNODES];

#[unsafe(no_mangle)]
pub static mut nodeforplayer: [c_int; NUM_PLAYERS] = [0; NUM_PLAYERS];

#[unsafe(no_mangle)]
pub static mut maketic: c_int = 0;

#[unsafe(no_mangle)]
pub static mut lastnettic: c_int = 0;

#[unsafe(no_mangle)]
pub static mut skiptics: c_int = 0;

#[unsafe(no_mangle)]
pub static mut ticdup: c_int = 1;

#[unsafe(no_mangle)]
pub static mut maxsend: c_int = 0;

#[unsafe(no_mangle)]
pub static mut reboundpacket: c_int = 0;

#[unsafe(no_mangle)]
pub static mut reboundstore: doomdata_t = doomdata_t {
    checksum: 0,
    retransmitfrom: 0,
    starttic: 0,
    player: 0,
    numtics: 0,
    cmds: [EMPTY_TICCMD; BACKUP_TICS],
};

#[unsafe(no_mangle)]
pub static mut exitmsg: [c_char; 80] = [0; 80];

#[unsafe(no_mangle)]
pub static mut gametime: c_int = 0;

#[unsafe(no_mangle)]
pub static mut frametics: [c_int; 4] = [0; 4];

#[unsafe(no_mangle)]
pub static mut frameon: c_int = 0;

#[unsafe(no_mangle)]
pub static mut frameskip: [c_int; 4] = [0; 4];

#[unsafe(no_mangle)]
pub static mut oldnettics: c_int = 0;

#[inline]
pub fn expand_tics(low: c_int, cur_maketic: c_int) -> c_int {
    let delta = low - (cur_maketic & 0xff);
    if (-64..=64).contains(&delta) {
        (cur_maketic & !0xff) + low
    } else if delta > 64 {
        (cur_maketic & !0xff) - 256 + low
    } else {
        (cur_maketic & !0xff) + 256 + low
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn NetbufferSize() -> c_int {
    let header_sz = offset_of!(doomdata_t, cmds);
    let num_tics = unsafe { (*netbuffer).numtics as usize };
    (header_sz + num_tics * size_of::<ticcmd_t>()) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn NetbufferChecksum() -> c_uint {
    unsafe {
        let mut c: u32 = 0x1234567;
        let retr_ofs = offset_of!(doomdata_t, retransmitfrom) as c_int;
        let words = ((NetbufferSize() - retr_ofs) / 4).max(0) as usize;
        let ptr = (&raw const (*netbuffer).retransmitfrom) as *const u32;
        let slice = std::slice::from_raw_parts(ptr, words);
        for (i, &w) in slice.iter().enumerate() {
            c = c.wrapping_add(w.wrapping_mul((i as u32) + 1));
        }
        c & NCMD_CHECKSUM
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ExpandTics(low: c_int) -> c_int {
    expand_tics(low, unsafe { maketic })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HSendPacket(node: c_int, flags: c_int) {
    unsafe {
        let nb = &mut *netbuffer;
        nb.checksum = NetbufferChecksum() | (flags as u32);

        if node == 0 {
            reboundstore = *nb;
            reboundpacket = 1;
            return;
        }

        if demoplayback != 0 {
            return;
        }

        if netgame == 0 {
            doom_error("Tried to transmit to another node");
        }

        let dc = &mut *doomcom;
        dc.command = CMD_SEND as c_short;
        dc.remotenode = node as c_short;
        dc.datalength = NetbufferSize() as c_short;

        I_NetCmd();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HGetPacket() -> c_int {
    unsafe {
        let nb = &mut *netbuffer;
        let dc = &mut *doomcom;

        if reboundpacket != 0 {
            *nb = reboundstore;
            dc.remotenode = 0;
            reboundpacket = 0;
            return 1;
        }

        if netgame == 0 || demoplayback != 0 {
            return 0;
        }

        dc.command = CMD_GET as c_short;
        I_NetCmd();

        if dc.remotenode == -1 {
            return 0;
        }

        if (dc.datalength as c_int) != NetbufferSize() {
            return 0;
        }

        if NetbufferChecksum() != (nb.checksum & NCMD_CHECKSUM) {
            return 0;
        }

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetPackets() {
    unsafe {
        let nb = &*netbuffer;
        let dc = &*doomcom;
        let in_game = (&raw mut playeringame).as_mut().unwrap();
        let players_arr = (&raw mut players).as_mut().unwrap();
        let netcmds_arr = (&raw mut netcmds).as_mut().unwrap();

        while HGetPacket() != 0 {
            if (nb.checksum & NCMD_SETUP) != 0 {
                continue;
            }

            let netconsole = (nb.player & !PL_DRONE) as usize;
            let netnode = dc.remotenode as usize;

            let realstart = ExpandTics(nb.starttic as c_int);
            let realend = realstart + (nb.numtics as c_int);

            if (nb.checksum & NCMD_EXIT) != 0 {
                if nodeingame[netnode] == 0 {
                    continue;
                }
                nodeingame[netnode] = 0;
                in_game[netconsole] = 0;

                let msg = format!("Player {} left the game\0", netconsole + 1);
                let dst = (&raw mut exitmsg).as_mut().unwrap();
                for (d, &s) in dst.iter_mut().zip(msg.as_bytes()) {
                    *d = s as c_char;
                }
                players_arr[consoleplayer as usize].message = dst.as_mut_ptr();

                if demorecording != 0 {
                    G_CheckDemoStatus();
                }
                continue;
            }

            if (nb.checksum & NCMD_KILL) != 0 {
                doom_error("Killed by network driver");
            }

            nodeforplayer[netconsole] = netnode as c_int;

            if resendcount[netnode] <= 0 && (nb.checksum & NCMD_RETRANSMIT) != 0 {
                resendto[netnode] = ExpandTics(nb.retransmitfrom as c_int);
                resendcount[netnode] = RESENDCOUNT;
            } else {
                resendcount[netnode] -= 1;
            }

            if realend <= nettics[netnode] {
                continue;
            }

            if realstart > nettics[netnode] {
                remoteresend[netnode] = 1;
                continue;
            }

            remoteresend[netnode] = 0;
            let mut src_idx = (nettics[netnode] - realstart) as usize;

            while nettics[netnode] < realend {
                let slot = (nettics[netnode] as usize) % BACKUP_TICS;
                netcmds_arr[netconsole][slot] = nb.cmds[src_idx];
                nettics[netnode] += 1;
                src_idx += 1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn NetUpdate() {
    unsafe {
        let nowtime = I_GetTime() / ticdup;
        let mut newtics = nowtime - gametime;
        gametime = nowtime;

        if newtics <= 0 {
            GetPackets();
            return;
        }

        if skiptics <= newtics {
            newtics -= skiptics;
            skiptics = 0;
        } else {
            skiptics -= newtics;
            newtics = 0;
        }

        let nb = &mut *netbuffer;
        let dc = &*doomcom;
        nb.player = consoleplayer as u8;

        let gameticdiv = gametic / ticdup;
        let local_arr = (&raw mut localcmds).as_mut().unwrap();

        for _ in 0..newtics {
            I_StartTic();
            D_ProcessEvents();
            if maketic - gameticdiv >= ((BACKUP_TICS as c_int) / 2) - 1 {
                break;
            }
            let slot = (maketic as usize) % BACKUP_TICS;
            G_BuildTiccmd((&mut local_arr[slot] as *mut ticcmd_t) as *mut _);
            maketic += 1;
        }
if singletics != 0 {
return;
}
let num_nodes = dc.numnodes as usize;
for i in 0..num_nodes {
if nodeingame[i] != 0 {
let realstart = resendto[i];
nb.starttic = realstart as u8;
let n_tics = maketic - realstart;
nb.numtics = n_tics as u8;
if n_tics > (BACKUP_TICS as c_int) {
doom_error("NetUpdate: netbuffer->numtics > BACKUPTICS");
}
resendto[i] = maketic - (dc.extratics as c_int);
for j in 0..(n_tics as usize) {
nb.cmds[j] = local_arr[((realstart as usize) + j) % BACKUP_TICS];
}
if remoteresend[i] != 0 {
nb.retransmitfrom = nettics[i] as u8;
HSendPacket(i as c_int, NCMD_RETRANSMIT as c_int);
} else {
nb.retransmitfrom = 0;
HSendPacket(i as c_int, 0);
}
}
}
GetPackets();
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CheckAbort() {
unsafe {
let stoptic = I_GetTime() + 2;
while I_GetTime() < stoptic {
I_StartTic();
}
I_StartTic();
let ev_arr = (&raw const events).as_ref().unwrap();
while eventtail != eventhead {
let ev = &ev_arr[eventtail as usize];
if ev.type_ == ev_keydown && (ev.data1 as u32) == KEY_ESCAPE {
doom_error("Network game synchronization aborted.");
}
eventtail = (eventtail + 1) & ((MAX_EVENTS as c_int) - 1);
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_ArbitrateNetStart() {
unsafe {
let mut gotinfo = [false; MAX_NETNODES];
autostart = 1;
let dc = &*doomcom;
let nb = &mut *netbuffer;
let num_nodes = dc.numnodes as usize;
if dc.consoleplayer != 0 {
println!("listening for network start info...");
loop {
CheckAbort();
if HGetPacket() == 0 {
continue;
}
if (nb.checksum & NCMD_SETUP) != 0 {
if nb.player != VERSION {
doom_error("Different DOOM versions cannot play a net game!");
}
let rf = nb.retransmitfrom;
let st = nb.starttic;
startskill = (rf & 15) as skill_t;
deathmatch = ((rf & 0xc0) >> 6) as c_int;
nomonsters = ((rf & 0x20) > 0) as c_int;
respawnparm = ((rf & 0x10) > 0) as c_int;
startmap = (st & 0x3f) as c_int;
startepisode = (st >> 6) as c_int;
return;
}
}
} else {
println!("sending network start info...");
loop {
CheckAbort();
for i in 0..num_nodes {
let mut rf = startskill as u8;
if deathmatch != 0 {
rf |= (deathmatch as u8) << 6;
}
if nomonsters != 0 {
rf |= 0x20;
}
if respawnparm != 0 {
rf |= 0x10;
}
nb.retransmitfrom = rf;
nb.starttic = (startepisode * 64 + startmap) as u8;
nb.player = VERSION;
nb.numtics = 0;
HSendPacket(i as c_int, NCMD_SETUP as c_int);
}
for _ in 0..10 {
if HGetPacket() == 0 {
break;
}
let idx = (nb.player & 0x7f) as usize;
if idx < MAX_NETNODES {
gotinfo[idx] = true;
}
}
if gotinfo[1..num_nodes].iter().all(|&ready| ready) {
break;
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_CheckNetGame() {
unsafe {
nodeingame.fill(0);
nettics.fill(0);
remoteresend.fill(0);
resendto.fill(0);
I_InitNetwork();
let dc = &mut *doomcom;
if dc.id != (DOOMCOM_ID as i64) {
doom_error("Doomcom buffer invalid!");
}
netbuffer = &raw mut dc.data;
let cp = dc.consoleplayer as c_int;
consoleplayer = cp;
displayplayer = cp;
if netgame != 0 {
D_ArbitrateNetStart();
}
println!(
"startskill {}  deathmatch: {}  startmap: {}  startepisode: {}",
startskill, deathmatch, startmap, startepisode
);
ticdup = dc.ticdup as c_int;
maxsend = ((BACKUP_TICS as c_int) / (2 * ticdup) - 1).max(1);
let in_game = (&raw mut playeringame).as_mut().unwrap();
in_game[..(dc.numplayers as usize)].fill(1);
nodeingame[..(dc.numnodes as usize)].fill(1);
println!(
"player {} of {} ({} nodes)",
consoleplayer + 1,
dc.numplayers,
dc.numnodes
);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_QuitNetGame() {
unsafe {
if netgame == 0 || usergame == 0 || consoleplayer == -1 || demoplayback != 0 {
return;
}
let nb = &mut *netbuffer;
let dc = &*doomcom;
nb.player = consoleplayer as u8;
nb.numtics = 0;
let num_nodes = dc.numnodes as usize;
for _ in 0..4 {
for j in 1..num_nodes {
if nodeingame[j] != 0 {
HSendPacket(j as c_int, NCMD_EXIT as c_int);
}
}
I_WaitVBL(1);
}
}
}
static OLD_ENTERTICS: AtomicI32 = AtomicI32::new(0);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn TryRunTics() {
unsafe {
let entertic = I_GetTime() / ticdup;
let prev_enter = OLD_ENTERTICS.swap(entertic, Ordering::Relaxed);
let realtics = entertic - prev_enter;
NetUpdate();
let dc = &*doomcom;
let num_nodes = dc.numnodes as usize;
let mut lowtic = MAX_INT;
for i in 0..num_nodes {
if nodeingame[i] != 0 && nettics[i] < lowtic {
lowtic = nettics[i];
}
}
let availabletics = lowtic - gametic / ticdup;
let counts = if realtics < availabletics - 1 {
realtics + 1
} else if realtics < availabletics {
realtics
} else {
availabletics
}
.max(1);
frameon += 1;
if demoplayback == 0 {
let in_game = (&raw const playeringame).as_ref().unwrap();
let key_p = in_game.iter().position(|&v| v != 0).unwrap_or(0);
if consoleplayer != (key_p as c_int) {
let node_idx = nodeforplayer[key_p] as usize;
if nettics[0] <= nettics[node_idx] {
gametime -= 1;
}
frameskip[(frameon as usize) & 3] = (oldnettics > nettics[node_idx]) as c_int;
oldnettics = nettics[0];
if frameskip.iter().all(|&x| x != 0) {
skiptics = 1;
}
}
}
while lowtic < gametic / ticdup + counts {
NetUpdate();
lowtic = MAX_INT;
for i in 0..num_nodes {
if nodeingame[i] != 0 && nettics[i] < lowtic {
lowtic = nettics[i];
}
}
if lowtic < gametic / ticdup {
doom_error("TryRunTics: lowtic < gametic");
}
if I_GetTime() / ticdup - entertic >= 20 {
M_Ticker();
return;
}
}
let netcmds_arr = (&raw mut netcmds).as_mut().unwrap();
for _ in 0..counts {
for i in 0..ticdup {
if gametic / ticdup > lowtic {
doom_error("gametic>lowtic");
}
if advancedemo != 0 {
D_DoAdvanceDemo();
}
M_Ticker();
G_Ticker();
gametic += 1;
if i != ticdup - 1 {
let buf = ((gametic / ticdup) as usize) % BACKUP_TICS;
for p_cmds in netcmds_arr.iter_mut() {
let cmd = &mut p_cmds[buf];
cmd.chatchar = 0;
if ((cmd.buttons as u32) & BT_SPECIAL) != 0 {
cmd.buttons = 0;
}
}
}
}
NetUpdate();
}
}
}
