use std::collections::HashMap;
use std::mem::size_of;
use std::ptr::{null_mut};
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{BOOL, FALSE};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetPerTcpConnectionEStats, SetPerTcpConnectionEStats, MIB_TCPROW_LH,
    MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, TCP_ESTATS_DATA_ROD_v0, TCP_ESTATS_TYPE,
    TcpConnectionEstatsData, TCP_TABLE_OWNER_PID_ALL,
};
use windows_sys::Win32::Networking::WinSock::AF_INET;
type Ulong = u32;
type Pulong = *mut u32;
type Puchar = *mut u8;

unsafe fn get_tcp_owner_pid_table() -> Option<Vec<MIB_TCPROW_OWNER_PID>> {
    let mut size: Ulong = 0;
    // First call to get required size
    let ret = unsafe { GetExtendedTcpTable(
        null_mut(),
        &mut size as Pulong,
        FALSE as BOOL,
        AF_INET as u32,
        TCP_TABLE_OWNER_PID_ALL,
        0,
    ) };
    // ERROR_INSUFFICIENT_BUFFER = 122
    if ret != 122 || size == 0 {
        return None;
    }
    let mut buf: Vec<u8> = vec![0u8; size as usize];
    let ret2 = unsafe { GetExtendedTcpTable(
        buf.as_mut_ptr() as *mut _,
        &mut size as Pulong,
        FALSE as BOOL,
        AF_INET as u32,
        TCP_TABLE_OWNER_PID_ALL,
        0,
    ) };
    if ret2 != 0 {
        return None;
    }
    let table: *const MIB_TCPTABLE_OWNER_PID = buf.as_ptr() as *const _;
    if table.is_null() {
        return None;
    }
    let num = unsafe { (*table).dwNumEntries } as usize;
    let first_row = unsafe { &(*table).table as *const MIB_TCPROW_OWNER_PID };
    let slice = unsafe { std::slice::from_raw_parts(first_row, num) };
    Some(slice.to_vec())
}

#[allow(dead_code)]
unsafe fn owner_to_row(row: &MIB_TCPROW_OWNER_PID) -> MIB_TCPROW_LH {
    let mut r: MIB_TCPROW_LH = unsafe { std::mem::zeroed() };
    // MIB_TCPROW_LH has an anonymous union for State in windows-sys
    r.Anonymous.State = row.dwState as i32;
    r.dwLocalAddr = row.dwLocalAddr;
    r.dwLocalPort = row.dwLocalPort;
    r.dwRemoteAddr = row.dwRemoteAddr;
    r.dwRemotePort = row.dwRemotePort;
    r
}

pub fn sample_per_process_tcp_estats(interval: Duration) -> HashMap<u32, (f64, f64)> {
    // Returns pid -> (rx_rate_bps, tx_rate_bps)
    // Strategy: sum per-PID throughput counters at T0 and T1, compute deltas/second.
    unsafe {
        let rows = match get_tcp_owner_pid_table() {
            Some(v) => v,
            None => return HashMap::new(),
        };
        let mut base_pid: HashMap<u32, (u64, u64)> = HashMap::new();
        for row in &rows {
            let mut lwrow = owner_to_row(row);
            // Try enabling collection; if it fails, skip this connection to avoid bogus deltas.
            let rw = windows_sys::Win32::NetworkManagement::IpHelper::TCP_ESTATS_DATA_RW_v0 {
                EnableCollection: 1,
            };
            let set_res = SetPerTcpConnectionEStats(
                &mut lwrow as *mut MIB_TCPROW_LH,
                TcpConnectionEstatsData as TCP_ESTATS_TYPE,
                &rw as *const _ as Puchar,
                0,
                size_of::<windows_sys::Win32::NetworkManagement::IpHelper::TCP_ESTATS_DATA_RW_v0>()
                    as Ulong,
                0,
            );
            if set_res != 0 { continue; }

            let mut rod: TCP_ESTATS_DATA_ROD_v0 = std::mem::zeroed();
            let res = GetPerTcpConnectionEStats(
                &mut lwrow as *mut MIB_TCPROW_LH,
                TcpConnectionEstatsData as TCP_ESTATS_TYPE,
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null_mut(),
                0,
                0,
                &mut rod as *mut _ as Puchar,
                0,
                size_of::<TCP_ESTATS_DATA_ROD_v0>() as Ulong,
            );
            if res == 0 {
                let pid = row.dwOwningPid;
                let e = base_pid.entry(pid).or_insert((0, 0));
                e.0 = e.0.saturating_add(rod.ThruBytesReceived as u64);
                e.1 = e.1.saturating_add(rod.ThruBytesAcked as u64);
            }
        }

        let elapsed = if interval.is_zero() { Duration::from_millis(1) } else { interval };
        thread::sleep(elapsed);

        let rows_after = match get_tcp_owner_pid_table() {
            Some(v) => v,
            None => return HashMap::new(),
        };
        let secs = elapsed.as_secs_f64().max(0.001);
        let mut now_pid: HashMap<u32, (u64, u64)> = HashMap::new();
        for row in &rows_after {
            let mut lwrow = owner_to_row(row);
            let mut rod: TCP_ESTATS_DATA_ROD_v0 = std::mem::zeroed();
            let res = GetPerTcpConnectionEStats(
                &mut lwrow as *mut MIB_TCPROW_LH,
                TcpConnectionEstatsData as TCP_ESTATS_TYPE,
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null_mut(),
                0,
                0,
                &mut rod as *mut _ as Puchar,
                0,
                size_of::<TCP_ESTATS_DATA_ROD_v0>() as Ulong,
            );
            if res == 0 {
                let pid = row.dwOwningPid;
                let e = now_pid.entry(pid).or_insert((0, 0));
                e.0 = e.0.saturating_add(rod.ThruBytesReceived as u64);
                e.1 = e.1.saturating_add(rod.ThruBytesAcked as u64);
            }
        }

        let mut per_pid: HashMap<u32, (f64, f64)> = HashMap::new();
        for (pid, (b_rx, b_tx)) in base_pid.into_iter() {
            if let Some((n_rx, n_tx)) = now_pid.get(&pid).copied() {
                let rx = n_rx.saturating_sub(b_rx) as f64 / secs;
                let tx = n_tx.saturating_sub(b_tx) as f64 / secs;
                per_pid.insert(pid, (rx, tx));
            }
        }
        per_pid
    }
}
