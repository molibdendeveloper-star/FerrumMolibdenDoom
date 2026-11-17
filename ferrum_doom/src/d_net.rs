#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::{player_s, player_t};

use std::ffi::{c_char, c_int, c_short, c_uint};
use std::mem::{offset_of, size_of};
use std::ptr;

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

unsafe extern "C" {
    fn I_GetTime() -> c_int;
    fn I_StartTic();
    fn I_NetCmd();
    fn I_InitNetwork();
    fn I_WaitVBL(count: c_int);
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

// --- Глобальные переменные d_net.c ---
#[unsafe(no_mangle)]
pub static mut doomcom: *mut doomcom_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut netbuffer: *mut doomdata_t = ptr::null_mut();

#[unsafe(no_mangle)]
pub static mut localcmds: [ticcmd_t; BACKUP_TICS] = unsafe { std::mem::zeroed() };

#[unsafe(no_mangle)]
pub static mut netcmds: [[ticcmd_t; BACKUP_TICS]; NUM_PLAYERS] = unsafe { std::mem::zeroed() };

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
pub static mut reboundstore: doomdata_t = unsafe { std::mem::zeroed() };

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

// --- NetbufferSize / NetbufferChecksum / ExpandTics ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn NetbufferSize() -> c_int {
    unsafe {
        let header_sz = offset_of!(doomdata_t, cmds);
        let num_tics = (*netbuffer).numtics as usize;
        (header_sz + num_tics * size_of::<ticcmd_t>()) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn NetbufferChecksum() -> c_uint {
    unsafe {
        let mut c: u32 = 0x1234567;
        let retr_ofs = offset_of!(doomdata_t, retransmitfrom) as c_int;
        let l = (NetbufferSize() - retr_ofs) / 4;
        let ptr = (&raw const (*netbuffer).retransmitfrom) as *const u32;
        for i in 0..(l as usize) {
            c = c.wrapping_add((*ptr.add(i)).wrapping_mul((i as u32) + 1));
        }
        c & NCMD_CHECKSUM
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ExpandTics(low: c_int) -> c_int {
    unsafe {
        let delta = low - (maketic & 0xff);
        if (-64..=64).contains(&delta) {
            (maketic & !0xff) + low
        } else if delta > 64 {
            (maketic & !0xff) - 256 + low
        } else {
            (maketic & !0xff) + 256 + low
        }
    }
}

// --- HSendPacket / HGetPacket ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HSendPacket(node: c_int, flags: c_int) {
    unsafe {
        (*netbuffer).checksum = NetbufferChecksum() | (flags as u32);

        if node == 0 {
            ptr::copy_nonoverlapping(netbuffer, &raw mut reboundstore, 1);
            reboundpacket = 1;
            return;
        }

        if demoplayback != 0 {
            return;
        }

        if netgame == 0 {
            doom_error("Tried to transmit to another node");
        }

        (*doomcom).command = CMD_SEND as c_short;
        (*doomcom).remotenode = node as c_short;
        (*doomcom).datalength = NetbufferSize() as c_short;

        I_NetCmd();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HGetPacket() -> c_int {
    unsafe {
        if reboundpacket != 0 {
            ptr::copy_nonoverlapping(&raw const reboundstore, netbuffer, 1);
            (*doomcom).remotenode = 0;
            reboundpacket = 0;
            return 1;
        }

        if netgame == 0 || demoplayback != 0 {
            return 0;
        }

        (*doomcom).command = CMD_GET as c_short;
        I_NetCmd();

        if (*doomcom).remotenode == -1 {
            return 0;
        }

        if ((*doomcom).datalength as c_int) != NetbufferSize() {
            return 0;
        }

        if NetbufferChecksum() != ((*netbuffer).checksum & NCMD_CHECKSUM) {
            return 0;
        }

        1
    }
}

// --- GetPackets ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetPackets() {
    unsafe {
        while HGetPacket() != 0 {
            if ((*netbuffer).checksum & NCMD_SETUP) != 0 {
                continue;
            }

            let netconsole = (((*netbuffer).player) & !PL_DRONE) as usize;
            let netnode = (*doomcom).remotenode as usize;

            let realstart = ExpandTics((*netbuffer).starttic as c_int);
            let realend = realstart + ((*netbuffer).numtics as c_int);

            if ((*netbuffer).checksum & NCMD_EXIT) != 0 {
                if nodeingame[netnode] == 0 {
                    continue;
                }
                nodeingame[netnode] = 0;
                let in_game_ptr = (&raw mut playeringame) as *mut c_int;
                *in_game_ptr.add(netconsole) = 0;

                let msg = format!("Player {} left the game\0", netconsole + 1);
                let dst = (&raw mut exitmsg) as *mut c_char;
                ptr::copy_nonoverlapping(msg.as_ptr() as *const c_char, dst, msg.len());
                let players_base = (&raw mut players) as *mut player_t;
                (*players_base.add(consoleplayer as usize)).message = dst;

                if demorecording != 0 {
                    G_CheckDemoStatus();
                }
                continue;
            }

            if ((*netbuffer).checksum & NCMD_KILL) != 0 {
                doom_error("Killed by network driver");
            }

            nodeforplayer[netconsole] = netnode as c_int;

            if resendcount[netnode] <= 0 && ((*netbuffer).checksum & NCMD_RETRANSMIT) != 0 {
                resendto[netnode] = ExpandTics((*netbuffer).retransmitfrom as c_int);
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
            let start = (nettics[netnode] - realstart) as usize;
            let mut src = ((*netbuffer).cmds).as_ptr().add(start);
            let netcmds_base = (&raw mut netcmds) as *mut [ticcmd_t; BACKUP_TICS];

            while nettics[netnode] < realend {
                let slot = (nettics[netnode] as usize) % BACKUP_TICS;
                let dest = &raw mut (*netcmds_base.add(netconsole))[slot];
                nettics[netnode] += 1;
                ptr::copy_nonoverlapping(src, dest, 1);
                src = src.add(1);
            }
        }
    }
}

// --- NetUpdate ---
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

        (*netbuffer).player = consoleplayer as u8;

        let gameticdiv = gametic / ticdup;
        let local_base = (&raw mut localcmds) as *mut ticcmd_t;
        for _ in 0..newtics {
            I_StartTic();
            D_ProcessEvents();
            if maketic - gameticdiv >= ((BACKUP_TICS as c_int) / 2) - 1 {
                break;
            }
            G_BuildTiccmd(local_base.add((maketic as usize) % BACKUP_TICS) as *mut _);
            maketic += 1;
        }

        if singletics != 0 {
            return;
        }

        let num_nodes = (*doomcom).numnodes as usize;
        for i in 0..num_nodes {
            if nodeingame[i] != 0 {
                let realstart = resendto[i];
(*netbuffer).starttic = realstart as u8;
let n_tics = maketic - realstart;
(*netbuffer).numtics = n_tics as u8;
if n_tics > (BACKUP_TICS as c_int) {
doom_error("NetUpdate: netbuffer->numtics > BACKUPTICS");
}
resendto[i] = maketic - ((*doomcom).extratics as c_int);
let cmds_dst = ((*netbuffer).cmds).as_mut_ptr();
for j in 0..(n_tics as usize) {
let src_cmd = local_base.add(((realstart as usize) + j) % BACKUP_TICS);
ptr::copy_nonoverlapping(src_cmd, cmds_dst.add(j), 1);
}
if remoteresend[i] != 0 {
(*netbuffer).retransmitfrom = nettics[i] as u8;
HSendPacket(i as c_int, NCMD_RETRANSMIT as c_int);
} else {
(*netbuffer).retransmitfrom = 0;
HSendPacket(i as c_int, 0);
}
}
}
GetPackets();
}
}
// --- CheckAbort / D_ArbitrateNetStart / D_CheckNetGame / D_QuitNetGame ---
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CheckAbort() {
unsafe {
let stoptic = I_GetTime() + 2;
while I_GetTime() < stoptic {
I_StartTic();
}
I_StartTic();
let ev_base = (&raw const events) as *const event_t;
while eventtail != eventhead {
let ev = &*ev_base.add(eventtail as usize);
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
let num_nodes = (*doomcom).numnodes as usize;
if (*doomcom).consoleplayer != 0 {
println!("listening for network start info...");
loop {
CheckAbort();
if HGetPacket() == 0 {
continue;
}
if ((*netbuffer).checksum & NCMD_SETUP) != 0 {
if (*netbuffer).player != VERSION {
doom_error("Different DOOM versions cannot play a net game!");
}
let rf = (*netbuffer).retransmitfrom;
let st = (*netbuffer).starttic;
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
(*netbuffer).retransmitfrom = rf;
(*netbuffer).starttic = ((startepisode * 64 + startmap) as u8);
(*netbuffer).player = VERSION;
(*netbuffer).numtics = 0;
HSendPacket(i as c_int, NCMD_SETUP as c_int);
}
for _ in 0..10 {
if HGetPacket() == 0 {
break;
}
let idx = ((*netbuffer).player & 0x7f) as usize;
if idx < MAX_NETNODES {
gotinfo[idx] = true;
}
}
let mut all_ready = true;
for i in 1..num_nodes {
if !gotinfo[i] {
all_ready = false;
break;
}
}
if all_ready {
break;
}
}
}
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_CheckNetGame() {
unsafe {
for i in 0..MAX_NETNODES {
nodeingame[i] = 0;
nettics[i] = 0;
remoteresend[i] = 0;
resendto[i] = 0;
}
I_InitNetwork();
if (*doomcom).id != (DOOMCOM_ID as i64) {
doom_error("Doomcom buffer invalid!");
}
netbuffer = &raw mut (*doomcom).data;
let cp = (*doomcom).consoleplayer as c_int;
consoleplayer = cp;
displayplayer = cp;
if netgame != 0 {
D_ArbitrateNetStart();
}
println!(
"startskill {}  deathmatch: {}  startmap: {}  startepisode: {}",
startskill, deathmatch, startmap, startepisode
);
ticdup = (*doomcom).ticdup as c_int;
maxsend = (BACKUP_TICS as c_int) / (2 * ticdup) - 1;
if maxsend < 1 {
maxsend = 1;
}
let in_game_ptr = (&raw mut playeringame) as *mut c_int;
for i in 0..((*doomcom).numplayers as usize) {
*in_game_ptr.add(i) = 1;
}
for i in 0..((*doomcom).numnodes as usize) {
nodeingame[i] = 1;
}
println!(
"player {} of {} ({} nodes)",
consoleplayer + 1,
(*doomcom).numplayers,
(*doomcom).numnodes
);
}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn D_QuitNetGame() {
unsafe {
if netgame == 0 || usergame == 0 || consoleplayer == -1 || demoplayback != 0 {
return;
}
(*netbuffer).player = consoleplayer as u8;
(*netbuffer).numtics = 0;
let num_nodes = (*doomcom).numnodes as usize;
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
// --- TryRunTics ---
static mut OLD_ENTERTICS: c_int = 0;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn TryRunTics() {
unsafe {
let entertic = I_GetTime() / ticdup;
let realtics = entertic - OLD_ENTERTICS;
OLD_ENTERTICS = entertic;
NetUpdate();
let num_nodes = (*doomcom).numnodes as usize;
let mut lowtic = MAX_INT;
for i in 0..num_nodes {
if nodeingame[i] != 0 && nettics[i] < lowtic {
lowtic = nettics[i];
}
}
let availabletics = lowtic - gametic / ticdup;
let mut counts = if realtics < availabletics - 1 {
realtics + 1
} else if realtics < availabletics {
realtics
} else {
availabletics
};
if counts < 1 {
counts = 1;
}
frameon += 1;
if demoplayback == 0 {
let in_game_ptr = (&raw const playeringame) as *const c_int;
let mut key_p = 0usize;
for i in 0..NUM_PLAYERS {
if *in_game_ptr.add(i) != 0 {
key_p = i;
break;
}
}
if consoleplayer != (key_p as c_int) {
let node_idx = nodeforplayer[key_p] as usize;
if nettics[0] <= nettics[node_idx] {
gametime -= 1;
}
frameskip[(frameon as usize) & 3] = (oldnettics > nettics[node_idx]) as c_int;
oldnettics = nettics[0];
                if frameskip.iter().all(|&x| x != 0)
{
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
let netcmds_base = (&raw mut netcmds) as *mut [ticcmd_t; BACKUP_TICS];
while counts > 0 {
counts -= 1;
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
for j in 0..NUM_PLAYERS {
let cmd = &raw mut (*netcmds_base.add(j))[buf];
(*cmd).chatchar = 0;
if (((*cmd).buttons as u32) & BT_SPECIAL) != 0 {
(*cmd).buttons = 0;
}
}
}
}
NetUpdate();
}
}
}