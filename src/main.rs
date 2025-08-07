extern crate netstat2;

use netstat2::*;
use sysinfo::{System, Pid};

#[derive(Clone)]
struct SocketEntry {
    proto: String,
    local_addr: String,
    remote_addr: String,
    state: String,
    process_info: String,
}

fn get_process_info(system: &System, pid: u32) -> String {
    system
        .process(Pid::from(pid as usize))
        .map(|process| {
            let full_path = process.exe().unwrap_or_else(|| process.name().as_ref());
            format!("{}: {}", pid, full_path.display())
        })
        .unwrap_or_else(|| format!("{}: Unknown", pid))
}

fn state_sort_order(state: &str) -> u8 {
    // Reverse order - higher priority states get lower numbers for reverse sorting
    match state {
        "TimeWait" => 2,
        "LastAck" => 3,
        "Closing" => 4,
        "CloseWait" => 5,
        "FinWait2" => 6,
        "FinWait1" => 7,
        "SynReceived" => 8,
        "SynSent" => 9,
        "Established" => 10,
        "Listen" => 11,
        "-" => 1,  // For UDP
        _ => 0,    // Unknown states
    }
}

fn parse_addr_port(addr: &str) -> (&str, u16) {
    if let Some(last_colon) = addr.rfind(':') {
        let ip = &addr[..last_colon];
        let port_str = &addr[last_colon + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            return (ip, port);
        }
    }
    (addr, 0) // fallback
}

impl SocketEntry {
    fn sort_key(&self) -> (u8, &str, &str, u16) {
        let (ip, port) = parse_addr_port(&self.local_addr);
        (state_sort_order(&self.state), &self.proto, ip, port)
    }
}

fn main() {
    let mut system = System::new_all();
    system.refresh_all();
    
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let sockets_info = get_sockets_info(af_flags, proto_flags).unwrap();
    
    // Collect all socket entries
    let mut socket_entries: Vec<SocketEntry> = Vec::new();
    
    for si in sockets_info {
        let process_info_list: Vec<String> = si.associated_pids
            .iter()
            .map(|&pid| get_process_info(&system, pid))
            .collect();
        let process_info = if process_info_list.is_empty() {
            "Unknown".to_string()
        } else {
            process_info_list.join(", ")
        };

        match si.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp_si) => {
                let local_addr = format!("{}:{}", tcp_si.local_addr, tcp_si.local_port);
                let remote_addr = format!("{}:{}", tcp_si.remote_addr, tcp_si.remote_port);
                let state = format!("{:?}", tcp_si.state);
                
                socket_entries.push(SocketEntry {
                    proto: "TCP".to_string(),
                    local_addr,
                    remote_addr,
                    state,
                    process_info,
                });
            },
            ProtocolSocketInfo::Udp(udp_si) => {
                let local_addr = format!("{}:{}", udp_si.local_addr, udp_si.local_port);
                
                socket_entries.push(SocketEntry {
                    proto: "UDP".to_string(),
                    local_addr,
                    remote_addr: "*:*".to_string(),
                    state: "-".to_string(),
                    process_info,
                });
            },
        }
    }
    
    // Sort by STATE, PROTO, LOCAL ADDRESS
    socket_entries.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    
    // Print header
    println!("{:<10} {:<34} {:<27} {:<17} {:<40}",
        "PROTO", "LOCAL ADDRESS", "REMOTE ADDRESS", "STATE", "PROCESS");
    
    // Create aligned separator line
    let proto_sep = "-".repeat(9);
    let local_addr_sep = "-".repeat(33);
    let remote_addr_sep = "-".repeat(26);
    let state_sep = "-".repeat(16);
    let process_sep = "-".repeat(39);
    println!("{}  {}  {}  {}  {}", proto_sep, local_addr_sep, remote_addr_sep, state_sep, process_sep);
    
    // Print sorted entries
    for entry in socket_entries {
        println!("{:<10} {:<34} {:<27} {:<17} {:<40}",
            entry.proto,
            entry.local_addr,
            entry.remote_addr,
            entry.state,
            entry.process_info
        );
    }
}
