use std::ffi::{c_int, c_long, c_short};
use std::net::{SocketAddr, UdpSocket};
use std::sync::OnceLock;

use crate::d_net::{doomcom, doomcom_t, doomdata_t, netbuffer};
use crate::v_video::doom_error;

const DOOMPORT: u16 = 5029;
const DOOMCOM_ID: c_long = 0x12345678;
const MAXNETNODES: usize = 8;

const CMD_SEND: c_short = 1;
const CMD_GET: c_short = 2;

struct NetState {
    socket: UdpSocket,
    send_addresses: [Option<SocketAddr>; MAXNETNODES],
}

static NET_STATE: OnceLock<NetState> = OnceLock::new();

unsafe extern "C" {
    static mut netgame: i32;
}

fn get_args() -> Vec<String> {
    (0..crate::m_argv::args_len())
        .filter_map(crate::m_argv::get_arg)
        .collect()
}

fn parse_node_addr(arg: &str) -> SocketAddr {
    if arg.contains(':') {
        arg.parse().expect("Invalid IP:PORT address in -net")
    } else {
        format!("{}:{}", arg, DOOMPORT)
            .parse()
            .expect("Invalid IP address in -net")
    }
}

fn init_network_safe(dc: &mut doomcom_t) -> bool {
    dc.id = DOOMCOM_ID;
    dc.ticdup = 1;
    dc.extratics = 0;

    let args = get_args();

    if let Some(idx) = args.iter().position(|a| a == "-dup") {
        if let Some(val) = args.get(idx + 1).and_then(|v| v.parse::<c_short>().ok()) {
            dc.ticdup = val.clamp(1, 9);
        }
    }

    if args.iter().any(|a| a == "-extratic") {
        dc.extratics = 1;
    }

    let local_port = args
        .windows(2)
        .find(|w| w[0] == "-port")
        .and_then(|w| w[1].parse::<u16>().ok())
        .unwrap_or(DOOMPORT);

    let Some(net_idx) = args.iter().position(|a| a == "-net") else {
        // singleplayer
        dc.numplayers = 1;
        dc.numnodes = 1;
        dc.deathmatch = 0;
        dc.consoleplayer = 0;
        return false;
    };

    // netgame

    let player_num: c_short = args
        .get(net_idx + 1)
        .and_then(|s| s.parse::<c_short>().ok())
        .expect("Missing or invalid player number after -net");

    dc.consoleplayer = player_num - 1;

    let mut send_addresses = [None; MAXNETNODES];
    let mut numnodes: usize = 1;

    for arg in &args[net_idx + 2..] {
        if arg.starts_with('-') {
            break;
        }
        if numnodes < MAXNETNODES {
            send_addresses[numnodes] = Some(parse_node_addr(arg));
            numnodes += 1;
        }
    }

    dc.numnodes = numnodes as c_short;
    dc.numplayers = numnodes as c_short;

    let socket = UdpSocket::bind(("0.0.0.0", local_port))
        .unwrap_or_else(|e| panic!("I_InitNetwork: failed to bind port {}: {}", local_port, e));
    socket
        .set_nonblocking(true)
        .expect("I_InitNetwork: failed to set non-blocking socket");

    let _ = NET_STATE.set(NetState {
        socket,
        send_addresses,
    });
    true
}

fn packet_send(dc: &doomcom_t, data_bytes: &[u8]) {
    let Some(state) = NET_STATE.get() else { return };
    let node = dc.remotenode as usize;
    let Some(addr) = state.send_addresses.get(node).and_then(|a| *a) else {
        return;
    };

    let len = (dc.datalength.max(0) as usize).min(data_bytes.len());
    let _ = state.socket.send_to(&data_bytes[..len], addr);
}

fn packet_get(dc: &mut doomcom_t, data_bytes: &mut [u8]) {
    let Some(state) = NET_STATE.get() else {
        dc.remotenode = -1;
        return;
    };

    let mut buf = [0u8; std::mem::size_of::<doomdata_t>()];
    let Ok((len, src_addr)) = state.socket.recv_from(&mut buf) else {
        dc.remotenode = -1;
        return;
    };

    let found_node = state
        .send_addresses
        .iter()
        .position(|addr| addr.as_ref() == Some(&src_addr));

    let Some(node_idx) = found_node else {
        dc.remotenode = -1;
        return;
    };

    let copy_len = len.min(data_bytes.len());
    dc.remotenode = node_idx as c_short;
    dc.datalength = copy_len as c_short;
    data_bytes[..copy_len].copy_from_slice(&buf[..copy_len]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_InitNetwork() {
    unsafe {
        let dc = Box::leak(Box::new(doomcom_t::default()));
        doomcom = dc as *mut doomcom_t;
        netbuffer = (&raw mut dc.data) as *mut doomdata_t;
        netgame = init_network_safe(dc) as i32;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_NetCmd() {
    unsafe {
        let Some(dc) = doomcom.as_mut() else { return };
        let data_size = std::mem::size_of::<doomdata_t>();
        if dc.command == CMD_SEND {
            let bytes = std::slice::from_raw_parts(netbuffer as *const u8, data_size);
            packet_send(dc, bytes);
        } else if dc.command == CMD_GET {
            let bytes = std::slice::from_raw_parts_mut(netbuffer as *mut u8, data_size);
            packet_get(dc, bytes);
        } else {
            doom_error(&format!("Bad net cmd: {}", dc.command));
        }
    }
}
