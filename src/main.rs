use netstat::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Pid, System};
#[cfg(windows)]
mod win_net;

#[derive(Clone)]
struct SocketEntry {
    proto: String,
    local_addr: String,
    remote_addr: String,
    state: String,
    process_info: String,
    pids: Vec<u32>,
    agg_stats: Option<ProcessStats>,
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
        "-" => 1, // For UDP
        _ => 0,   // Unknown states
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

#[derive(Clone, Default)]
struct ProcessStats {
    cpu_pct: f32,
    read_rate_bps: f64,
    write_rate_bps: f64,
    net_rx_rate_bps: f64,
    net_tx_rate_bps: f64,
    total_read_bytes: u64,
    total_written_bytes: u64,
}

fn human_readable_rate(bps: f64) -> String {
    if !bps.is_finite() || bps < 0.0 {
        return "N/A".to_string();
    }
    const UNITS: [&str; 5] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    let mut v = bps;
    let mut idx = 0usize;
    while v >= 1024.0 && idx < UNITS.len() - 1 {
        v /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{:.0} {}", v, UNITS[idx])
    } else {
        format!("{:.1} {}", v, UNITS[idx])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortKeyKind { Cpu, R, W, Rx, Tx }

fn parse_args() -> (bool, u64, Option<usize>, Vec<SortKeyKind>) {
    // Returns (show_stats, sample_interval_ms, top_n, sort_keys)
    let mut show_stats = false;
    let mut sample_interval_ms: u64 = 800;
    let mut top_n: Option<usize> = None;
    let mut sort_keys: Vec<SortKeyKind> = Vec::new();

    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--full" => show_stats = true,
            "--sample-interval" => {
                if let Some(v) = args.next() && let Ok(ms) = v.parse::<u64>() {
                    sample_interval_ms = ms.max(1);
                }
            }
            "--top" => {
                if let Some(v) = args.next() && let Ok(n) = v.parse::<usize>() {
                    top_n = Some(n);
                }
            }
            "-f" => show_stats = true,
            "--sort" | "-s" => {
                if let Some(v) = args.next() {
                    let key = v.to_ascii_lowercase();
                    match key.as_str() {
                        "cpu" => sort_keys.push(SortKeyKind::Cpu),
                        "r" => sort_keys.push(SortKeyKind::R),
                        "w" => sort_keys.push(SortKeyKind::W),
                        "rx" => sort_keys.push(SortKeyKind::Rx),
                        "tx" => sort_keys.push(SortKeyKind::Tx),
                        _ => {}
                    }
                }
            }
            "-i" => {
                if let Some(v) = args.next() && let Ok(ms) = v.parse::<u64>() {
                    sample_interval_ms = ms.max(1);
                }
            }
            "-t" => {
                if let Some(v) = args.next() && let Ok(n) = v.parse::<usize>() {
                    top_n = Some(n);
                }
            }
            _ => {}
        }

        // Support attached short options like -i500 or -t3 or -scpu/-sRx
        if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2 {
            let flag = &arg[1..2];
            let rest = &arg[2..];
            match flag {
                "i" => {
                    if let Ok(ms) = rest.parse::<u64>() { sample_interval_ms = ms.max(1); }
                }
                "t" => {
                    if let Ok(n) = rest.parse::<usize>() { top_n = Some(n); }
                }
                "s" => {
                    let key = rest.to_ascii_lowercase();
                    match key.as_str() {
                        "cpu" => sort_keys.push(SortKeyKind::Cpu),
                        "r" => sort_keys.push(SortKeyKind::R),
                        "w" => sort_keys.push(SortKeyKind::W),
                        "rx" => sort_keys.push(SortKeyKind::Rx),
                        "tx" => sort_keys.push(SortKeyKind::Tx),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    // If sorting by metrics is requested, ensure stats are computed.
    if !sort_keys.is_empty() {
        show_stats = true;
    }
    (show_stats, sample_interval_ms, top_n, sort_keys)
}

fn print_help() {
    let exe = env::args().next().unwrap_or_else(|| "netstatw".to_string());
    println!("Usage: {} [OPTIONS]", exe);
    println!();
    println!("Options:");
    println!("  -h, --help                 Show this help and exit");
    println!("  -f, --full                Show CPU/Disk/IO and per-process net columns");
    println!("  -s, --sort KEY            Sort by metric (repeatable): cpu | R | W | Rx | Tx");
    println!("  -i, --sample-interval MS   Sampling interval in milliseconds (default: 800)");
    println!("  -t, --top N                Limit number of PIDs shown and included per row");
}

fn collect_process_stats(
    system: &mut System,
    pids: &HashSet<u32>,
    interval: Duration,
) -> HashMap<u32, ProcessStats> {
    // sysinfo notes:
    // - Process CPU% becomes meaningful after at least two refreshes.
    // - Disk usage totals are cumulative; compute deltas over `interval` for per-second rates.
    // - Some platforms may not expose all counters; such values may remain 0.
    // Initial refresh to capture baseline totals.
    system.refresh_processes();

    let mut base_totals: HashMap<u32, (u64, u64)> = HashMap::new();
    for &pid in pids {
        if let Some(proc_) = system.process(Pid::from(pid as usize)) {
            let du = proc_.disk_usage();
            base_totals.insert(pid, (du.total_read_bytes, du.total_written_bytes));
        }
    }

    let start = Instant::now();
    let sleep_dur = if interval.is_zero() {
        Duration::from_millis(1)
    } else {
        interval
    };
    thread::sleep(sleep_dur);

    // Second refresh to compute deltas; also makes cpu_usage meaningful.
    system.refresh_processes();

    let elapsed = start.elapsed().as_secs_f64().max(0.001);
    let mut out: HashMap<u32, ProcessStats> = HashMap::new();

    for &pid in pids {
        if let Some(proc_) = system.process(Pid::from(pid as usize)) {
            let cpu = proc_.cpu_usage();
            let du = proc_.disk_usage();
            let (base_r, base_w) = base_totals
                .get(&pid)
                .copied()
                .unwrap_or((du.total_read_bytes, du.total_written_bytes));
            let read_delta = du.total_read_bytes.saturating_sub(base_r) as f64;
            let write_delta = du.total_written_bytes.saturating_sub(base_w) as f64;
            let read_rate = read_delta / elapsed;
            let write_rate = write_delta / elapsed;
            out.insert(
                pid,
                ProcessStats {
                    cpu_pct: cpu,
                    read_rate_bps: read_rate,
                    write_rate_bps: write_rate,
                    net_rx_rate_bps: 0.0,
                    net_tx_rate_bps: 0.0,
                    total_read_bytes: du.total_read_bytes,
                    total_written_bytes: du.total_written_bytes,
                },
            );
        }
    }

out
}

fn build_socket_entries(
    sockets_info: Vec<SocketInfo>,
    system: &System,
    top_n: Option<usize>,
) -> Vec<SocketEntry> {
    let mut entries: Vec<SocketEntry> = Vec::new();
    for si in sockets_info {
        let process_info_list: Vec<String> = si
            .associated_pids
            .iter()
            .take(top_n.unwrap_or(usize::MAX))
            .map(|&pid| get_process_info(system, pid))
            .collect();
        let process_info = if process_info_list.is_empty() {
            "Unknown".to_string()
        } else {
            process_info_list.join(", ")
        };
        let pids: Vec<u32> = si
            .associated_pids
            .iter()
            .cloned()
            .take(top_n.unwrap_or(usize::MAX))
            .collect();

        match si.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp_si) => {
                let local_addr = format!("{}:{}", tcp_si.local_addr, tcp_si.local_port);
                let remote_addr = format!("{}:{}", tcp_si.remote_addr, tcp_si.remote_port);
                let state = format!("{:?}", tcp_si.state);

                entries.push(SocketEntry {
                    proto: "TCP".to_string(),
                    local_addr,
                    remote_addr,
                    state,
                    process_info,
                    pids,
                    agg_stats: None,
                });
            }
            ProtocolSocketInfo::Udp(udp_si) => {
                let local_addr = format!("{}:{}", udp_si.local_addr, udp_si.local_port);

                entries.push(SocketEntry {
                    proto: "UDP".to_string(),
                    local_addr,
                    remote_addr: "*:*".to_string(),
                    state: "-".to_string(),
                    process_info,
                    pids,
                    agg_stats: None,
                });
            }
        }
    }

    entries
}

fn main() {
    // Help flag handling
    if env::args().skip(1).any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let (show_stats, sample_interval_ms, top_n, sort_keys) = parse_args();

    let mut system = System::new_all();
    system.refresh_all();

    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let sockets_info = get_sockets_info(af_flags, proto_flags).unwrap();
    //println!("Found {} sockets", sockets_info.len());
    //println!("sockets info: {:#?}", sockets_info);

    // Collect all socket entries
    let mut socket_entries: Vec<SocketEntry> = build_socket_entries(sockets_info, &system, top_n);

    // If stats requested, sample process stats once for all involved PIDs and aggregate per row.
    // Also compute network per-process rates on Windows; on other platforms remain N/A.
    if show_stats {
        let mut pid_set: HashSet<u32> = HashSet::new();
        for e in &socket_entries {
            for &p in &e.pids {
                pid_set.insert(p);
            }
        }
        if !pid_set.is_empty() {
            // Windows-specific per-process TCP network sampling.
            #[cfg(windows)]
            let net_rates: std::collections::HashMap<u32, (f64, f64)> = {
                let dur = Duration::from_millis(sample_interval_ms);
                crate::win_net::sample_per_process_tcp_estats(dur)
            };
            #[cfg(not(windows))]
            let net_rates: std::collections::HashMap<u32, (f64, f64)> = Default::default();
            let stats_map = collect_process_stats(
                &mut system,
                &pid_set,
                Duration::from_millis(sample_interval_ms),
            );
            for entry in &mut socket_entries {
                let mut agg = ProcessStats::default();
                let mut any = false;
                let mut net_any = false;
                for &p in &entry.pids {
                    if let Some(s) = stats_map.get(&p) {
                        any = true;
                        agg.cpu_pct += s.cpu_pct;
                        agg.read_rate_bps += s.read_rate_bps;
                        agg.write_rate_bps += s.write_rate_bps;
                        agg.total_read_bytes =
                            agg.total_read_bytes.saturating_add(s.total_read_bytes);
                        agg.total_written_bytes = agg
                            .total_written_bytes
                            .saturating_add(s.total_written_bytes);
                    }
                    if let Some((rx, tx)) = net_rates.get(&p) {
                        net_any = true;
                        agg.net_rx_rate_bps += *rx;
                        agg.net_tx_rate_bps += *tx;
                    }
                }
                if !net_any {
                    // Mark network as not available so formatting shows N/A
                    agg.net_rx_rate_bps = f64::NAN;
                    agg.net_tx_rate_bps = f64::NAN;
                }
                if any {
                    entry.agg_stats = Some(agg);
                }
            }
        }
    }

    // Sort
    if !sort_keys.is_empty() {
        socket_entries.sort_by(|a, b| {
            for key in &sort_keys {
                let av = match (key, &a.agg_stats) {
                    (SortKeyKind::Cpu, Some(s)) => s.cpu_pct as f64,
                    (SortKeyKind::R, Some(s)) => s.read_rate_bps,
                    (SortKeyKind::W, Some(s)) => s.write_rate_bps,
                    (SortKeyKind::Rx, Some(s)) => s.net_rx_rate_bps,
                    (SortKeyKind::Tx, Some(s)) => s.net_tx_rate_bps,
                    _ => f64::NAN,
                };
                let bv = match (key, &b.agg_stats) {
                    (SortKeyKind::Cpu, Some(s)) => s.cpu_pct as f64,
                    (SortKeyKind::R, Some(s)) => s.read_rate_bps,
                    (SortKeyKind::W, Some(s)) => s.write_rate_bps,
                    (SortKeyKind::Rx, Some(s)) => s.net_rx_rate_bps,
                    (SortKeyKind::Tx, Some(s)) => s.net_tx_rate_bps,
                    _ => f64::NAN,
                };
                // Descending; treat NaN as smallest
                let ord = if av.is_nan() && bv.is_nan() {
                    std::cmp::Ordering::Equal
                } else if av.is_nan() {
                    std::cmp::Ordering::Greater
                } else if bv.is_nan() {
                    std::cmp::Ordering::Less
                } else {
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                };
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
            a.sort_key().cmp(&b.sort_key())
        });
    } else {
        // Default: by STATE, PROTO, LOCAL ADDRESS
        socket_entries.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }

    // Print header
    if show_stats {
        println!(
            "{:<10} {:<34} {:<27} {:<17} {:>7} {:>10} {:>10} {:>10} {:>10} {:<40}",
            "PROTO", "LOCAL ADDRESS", "REMOTE ADDRESS", "STATE", "CPU%", "R/s", "W/s", "Rx/s", "Tx/s", "PROCESS"
        );
    } else {
        println!(
            "{:<10} {:<34} {:<27} {:<17} {:<40}",
            "PROTO", "LOCAL ADDRESS", "REMOTE ADDRESS", "STATE", "PROCESS"
        );
    }

    // Create aligned separator line
    let proto_sep = "-".repeat(9);
    let local_addr_sep = "-".repeat(33);
    let remote_addr_sep = "-".repeat(26);
    let state_sep = "-".repeat(16);
    let process_sep = "-".repeat(39);
    if show_stats {
        let cpu_sep = "-".repeat(6);
        let r_sep = "-".repeat(9);
        let w_sep = "-".repeat(9);
        let rx_sep = "-".repeat(9);
        let tx_sep = "-".repeat(9);
        println!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            proto_sep, local_addr_sep, remote_addr_sep, state_sep, cpu_sep, r_sep, w_sep, rx_sep, tx_sep, process_sep
        );
    } else {
        println!(
            "{}  {}  {}  {}  {}",
            proto_sep, local_addr_sep, remote_addr_sep, state_sep, process_sep
        );
    }

    // Print sorted entries
    for entry in socket_entries {
        if show_stats {
            let (cpu_s, r_s, w_s, rx_s, tx_s) = if let Some(s) = &entry.agg_stats {
                (
                    format!("{:.1}", s.cpu_pct),
                    human_readable_rate(s.read_rate_bps),
                    human_readable_rate(s.write_rate_bps),
                    human_readable_rate(s.net_rx_rate_bps),
                    human_readable_rate(s.net_tx_rate_bps),
                )
            } else {
                (
                    "N/A".to_string(),
                    "N/A".to_string(),
                    "N/A".to_string(),
                    "N/A".to_string(),
                    "N/A".to_string(),
                )
            };
            println!(
                "{:<10} {:<34} {:<27} {:<17} {:>7} {:>10} {:>10} {:>10} {:>10} {:<40}",
                entry.proto,
                entry.local_addr,
                entry.remote_addr,
                entry.state,
                cpu_s,
                r_s,
                w_s,
                rx_s,
                tx_s,
                entry.process_info
            );
        } else {
            println!(
                "{:<10} {:<34} {:<27} {:<17} {:<40}",
                entry.proto, entry.local_addr, entry.remote_addr, entry.state, entry.process_info
            );
        }
    }
}
