use std::ffi::{c_int, c_long, c_short, CStr};
use std::net::{SocketAddr, UdpSocket};
use std::sync::OnceLock;

use crate::d_net::{doomcom, doomcom_t, doomdata_t, netbuffer};

const DOOMPORT: u16 = 5029;
const DOOMCOM_ID: c_long = 0x12345678;
const MAXNETNODES: usize = 8;

const CMD_SEND: c_short = 1;
const CMD_GET: c_short = 2;

static mut DOOMCOM_INSTANCE: doomcom_t = unsafe { std::mem::zeroed() };

struct NetState {
    socket: UdpSocket,
    send_addresses: [Option<SocketAddr>; MAXNETNODES],
}

static NET_STATE: OnceLock<NetState> = OnceLock::new();

unsafe extern "C" {
    static myargc: c_int;
    static myargv: *const *const i8;
    static mut netgame: i32;
    fn I_Error(error: *const i8, ...);
}

fn get_args() -> Vec<String> {
    unsafe {
        let mut args = Vec::with_capacity(myargc.max(0) as usize);
        for i in 0..myargc {
            let ptr = *myargv.add(i as usize);
            if !ptr.is_null() {
                if let Ok(s) = CStr::from_ptr(ptr).to_str() {
                    args.push(s.to_string());
                }
            }
        }
        args
    }
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_InitNetwork() {
    doomcom = &raw mut DOOMCOM_INSTANCE;
    netbuffer = &raw mut (*doomcom).data;

    (*doomcom).id = DOOMCOM_ID;
    (*doomcom).ticdup = 1;
    (*doomcom).extratics = 0;

    let args = get_args();

    // Проверка флага -dup
    if let Some(idx) = args.iter().position(|a| a == "-dup") {
        if let Some(val) = args.get(idx + 1).and_then(|v| v.parse::<c_short>().ok()) {
            (*doomcom).ticdup = val.clamp(1, 9);
        }
    }

    // Проверка флага -extratic
    if args.iter().any(|a| a == "-extratic") {
        (*doomcom).extratics = 1;
    }

    // Порт по умолчанию или через -port
    let local_port = args
        .windows(2)
        .find(|w| w[0] == "-port")
        .and_then(|w| w[1].parse::<u16>().ok())
        .unwrap_or(DOOMPORT);

    // Ищем -net
    let Some(net_idx) = args.iter().position(|a| a == "-net") else {
        // Одиночная игра
        netgame = 0;
        (*doomcom).numplayers = 1;
        (*doomcom).numnodes = 1;
        (*doomcom).deathmatch = 0;
        (*doomcom).consoleplayer = 0;
        return;
    };

    netgame = 1;

    let player_num: c_short = args
        .get(net_idx + 1)
        .and_then(|s| s.parse::<c_short>().ok())
        .expect("Missing or invalid player number after -net");

    (*doomcom).consoleplayer = player_num - 1;

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

    (*doomcom).numnodes = numnodes as c_short;
    (*doomcom).numplayers = numnodes as c_short;

    let socket = UdpSocket::bind(("0.0.0.0", local_port))
        .unwrap_or_else(|e| panic!("I_InitNetwork: failed to bind port {}: {}", local_port, e));
    socket
        .set_nonblocking(true)
        .expect("I_InitNetwork: failed to set non-blocking socket");

    let _ = NET_STATE.set(NetState {
        socket,
        send_addresses,
    });
}

unsafe fn packet_send() {
    let Some(state) = NET_STATE.get() else { return };
    let node = (*doomcom).remotenode as usize;
    let Some(addr) = state.send_addresses.get(node).and_then(|a| *a) else {
        return;
    };

    let len = ((*doomcom).datalength as usize).min(std::mem::size_of::<doomdata_t>());
    let raw_bytes = std::slice::from_raw_parts(netbuffer as *const u8, len);

    let _ = state.socket.send_to(raw_bytes, addr);
}

unsafe fn packet_get() {
    let Some(state) = NET_STATE.get() else {
        (*doomcom).remotenode = -1;
        return;
    };

    let mut buf = [0u8; std::mem::size_of::<doomdata_t>()];
    let Ok((len, src_addr)) = state.socket.recv_from(&mut buf) else {
        (*doomcom).remotenode = -1;
        return;
    };

    // Ищем отправителя среди известных адресов
    let found_node = state
        .send_addresses
        .iter()
        .position(|addr| addr.as_ref() == Some(&src_addr));

    let Some(node_idx) = found_node else {
        (*doomcom).remotenode = -1;
        return;
    };

    (*doomcom).remotenode = node_idx as c_short;
    (*doomcom).datalength = len as c_short;

    std::ptr::copy_nonoverlapping(
        buf.as_ptr(),
        netbuffer as *mut u8,
        len.min(std::mem::size_of::<doomdata_t>()),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn I_NetCmd() {
    if (*doomcom).command == CMD_SEND {
        packet_send();
    } else if (*doomcom).command == CMD_GET {
        packet_get();
    } else {
        I_Error(
            b"Bad net cmd: %i\n\0".as_ptr() as *const i8,
            (*doomcom).command as c_int,
        );
    }
}
