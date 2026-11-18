#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/doom_info_bindings.rs"));
use crate::r_segs::line_s;
use crate::r_things::player_s;

use std::ffi::{CStr, c_int, c_short};
use std::io::ErrorKind;
use std::mem::{offset_of, size_of};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::ptr;

use crate::d_net::{doomcom, netbuffer};
use crate::g_game::netgame;
use crate::m_argv::{m_check_parm, myargc, myargv};
use crate::v_video::doom_error;

const CMD_SEND: c_short = 1;
const CMD_GET: c_short = 2;
const MAX_NETNODES: usize = MAXNETNODES as usize;

#[unsafe(no_mangle)]
pub static mut DOOMPORT: c_int = 5000 + 0x1d; // IPPORT_USERRESERVED + 0x1d = 5029

#[unsafe(no_mangle)]
pub static mut sendsocket: c_int = -1;

#[unsafe(no_mangle)]
pub static mut insocket: c_int = -1;

#[unsafe(no_mangle)]
pub static mut netget: Option<unsafe extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub static mut netsend: Option<unsafe extern "C" fn()> = None;

static mut UDP_SOCKET: Option<UdpSocket> = None;
static mut NODE_ADDRS: [Option<SocketAddr>; MAX_NETNODES] = [None; MAX_NETNODES];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PacketSend() {
    unsafe {
        let Some(sock) = UDP_SOCKET.as_ref() else {
            return;
        };
        let remote = (*doomcom).remotenode as usize;
        let Some(dest_addr) = NODE_ADDRS.get(remote).and_then(|a| *a) else {
            return;
        };

        let mut sw: doomdata_t = std::mem::zeroed();
        sw.checksum = (*netbuffer).checksum.to_be();
        sw.player = (*netbuffer).player;
        sw.retransmitfrom = (*netbuffer).retransmitfrom;
        sw.starttic = (*netbuffer).starttic;
        sw.numtics = (*netbuffer).numtics;

        let n_tics = ((*netbuffer).numtics as usize).min(BACKUPTICS as usize);
        for c in 0..n_tics {
            sw.cmds[c].forwardmove = (*netbuffer).cmds[c].forwardmove;
            sw.cmds[c].sidemove = (*netbuffer).cmds[c].sidemove;
            sw.cmds[c].angleturn = (*netbuffer).cmds[c].angleturn.to_be();
            sw.cmds[c].consistancy = (*netbuffer).cmds[c].consistancy.to_be();
            sw.cmds[c].chatchar = (*netbuffer).cmds[c].chatchar;
            sw.cmds[c].buttons = (*netbuffer).cmds[c].buttons;
        }

        let len = ((*doomcom).datalength as usize).min(size_of::<doomdata_t>());
        let raw_bytes = std::slice::from_raw_parts((&raw const sw) as *const u8, len);
        let _ = sock.send_to(raw_bytes, dest_addr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PacketGet() {
    unsafe {
        let Some(sock) = UDP_SOCKET.as_ref() else {
            (*doomcom).remotenode = -1;
            return;
        };

        let mut sw: doomdata_t = std::mem::zeroed();
        let buf = std::slice::from_raw_parts_mut(
            (&raw mut sw) as *mut u8,
            size_of::<doomdata_t>(),
        );

        let (c, from_addr) = match sock.recv_from(buf) {
            Ok(res) => res,
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                (*doomcom).remotenode = -1;
                return;
            }
            Err(e) => {
                doom_error(&format!("GetPacket: {}", e));
            }
        };

        if c < offset_of!(doomdata_t, cmds) {
            (*doomcom).remotenode = -1;
            return;
        }

        let num_nodes = (*doomcom).numnodes as usize;
        let mut found_node: Option<usize> = None;
        for i in 0..num_nodes {
            if let Some(addr) = NODE_ADDRS[i] {
                if addr.ip() == from_addr.ip() {
                    found_node = Some(i);
                    break;
                }
            }
        }

        let Some(node_idx) = found_node else {
            (*doomcom).remotenode = -1;
            return;
        };

        (*doomcom).remotenode = node_idx as c_short;
        (*doomcom).datalength = c as c_short;

        (*netbuffer).checksum = u32::from_be(sw.checksum);
        (*netbuffer).player = sw.player;
        (*netbuffer).retransmitfrom = sw.retransmitfrom;
        (*netbuffer).starttic = sw.starttic;
        (*netbuffer).numtics = sw.numtics;

        let n_tics = ((*netbuffer).numtics as usize).min(BACKUPTICS as usize);
        for idx in 0..n_tics {
            (*netbuffer).cmds[idx].forwardmove = sw.cmds[idx].forwardmove;
            (*netbuffer).cmds[idx].sidemove = sw.cmds[idx].sidemove;
            (*netbuffer).cmds[idx].angleturn = i16::from_be(sw.cmds[idx].angleturn);
            (*netbuffer).cmds[idx].consistancy = i16::from_be(sw.cmds[idx].consistancy);
            (*netbuffer).cmds[idx].chatchar = sw.cmds[idx].chatchar;
            (*netbuffer).cmds[idx].buttons = sw.cmds[idx].buttons;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_InitNetwork() {
    unsafe {
        let dc = Box::leak(Box::new(std::mem::zeroed::<doomcom_t>()));
        doomcom = (dc as *mut doomcom_t) as *mut _;

        let p_dup = m_check_parm("-dup");
        if p_dup != 0 && p_dup < myargc - 1 {
            let ch = *(*myargv.add((p_dup + 1) as usize)) as u8;
            let val = (ch.wrapping_sub(b'0') as c_short).clamp(1, 9);
            (*doomcom).ticdup = val;
        } else {
            (*doomcom).ticdup = 1;
        }

        (*doomcom).extratics = if m_check_parm("-extratic") != 0 { 1 } else { 0 };

        let p_port = m_check_parm("-port");
        if p_port != 0 && p_port < myargc - 1 {
            let s = CStr::from_ptr(*myargv.add((p_port + 1) as usize)).to_string_lossy();
            DOOMPORT = s.parse::<c_int>().unwrap_or(DOOMPORT);
            println!("using alternate port {}", DOOMPORT);
        }

        let p_net = m_check_parm("-net");
        if p_net == 0 {
            netgame = 0;
            (*doomcom).id = DOOMCOM_ID as i64;
            (*doomcom).numplayers = 1;
            (*doomcom).numnodes = 1;
            (*doomcom).deathmatch = 0;
            (*doomcom).consoleplayer = 0;
            return;
        }

        netsend = Some(PacketSend);
        netget = Some(PacketGet);
        netgame = 1;

        let cp_ch = *(*myargv.add((p_net + 1) as usize)) as u8;
        (*doomcom).consoleplayer = cp_ch.wrapping_sub(b'1') as c_short;
        (*doomcom).numnodes = 1;

        let mut arg_idx = (p_net + 2) as usize;
        while arg_idx < (myargc as usize) {
            let arg_ptr = *myargv.add(arg_idx);
            let s = CStr::from_ptr(arg_ptr).to_string_lossy();
            if s.starts_with('-') {
                break;
            }

            let host = s.strip_prefix('.').unwrap_or(&s);
            let target = format!("{}:{}", host, DOOMPORT);
            let resolved = target
                .to_socket_addrs()
                .ok()
                .and_then(|mut it| it.next())
                .unwrap_or_else(|| {
                    doom_error(&format!("gethostbyname: couldn't find {}", host));
                });

            let node = (*doomcom).numnodes as usize;
            if node < MAX_NETNODES {
                NODE_ADDRS[node] = Some(resolved);
                (*doomcom).numnodes += 1;
            }
            arg_idx += 1;
        }

        (*doomcom).id = DOOMCOM_ID as i64;
        (*doomcom).numplayers = (*doomcom).numnodes;

        let bind_addr = format!("0.0.0.0:{}", DOOMPORT);
        let sock = UdpSocket::bind(&bind_addr).unwrap_or_else(|e| {
            doom_error(&format!("BindToPort: bind: {}", e));
        });
        sock.set_nonblocking(true).unwrap_or_else(|e| {
            doom_error(&format!("UDP set_nonblocking: {}", e));
        });

        UDP_SOCKET = Some(sock);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_NetCmd() {
    unsafe {
        let cmd = (*doomcom).command;
        if cmd == CMD_SEND {
            if let Some(f) = netsend {
                f();
            }
        } else if cmd == CMD_GET {
            if let Some(f) = netget {
                f();
            }
        } else {
            doom_error(&format!("Bad net cmd: {}\n", cmd));
        }
    }
}
