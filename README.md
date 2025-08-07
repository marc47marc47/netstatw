# netstatw

A Rust CLI application that displays network socket information, similar to the traditional `netstat` command. This tool provides detailed information about TCP and UDP connections, including local/remote addresses, ports, connection states, and associated process information.

## Features

- **Cross-platform**: Works on Windows, macOS, and Linux
- **Protocol support**: Displays both TCP and UDP socket information
- **IPv4 and IPv6**: Supports both address families
- **Process information**: Shows process ID and executable path for each connection
- **Sorted output**: Organized by connection state, protocol, and local address
- **Clean formatting**: Aligned columns for easy reading

## Installation

### Prerequisites

- Rust 1.70 or later

### Build from source

```bash
# Clone the repository
git clone <repository-url>
cd netstatw

# Build the project
cargo build --release

# The executable will be available at target/release/netstatw
```

## Usage

Simply run the executable to display current network connections:

```bash
cargo run
```

Or if you've built the release version:

```bash
./target/release/netstatw
```

### Sample Output

```
PROTO      LOCAL ADDRESS                      REMOTE ADDRESS        STATE             PROCESS
---------  ---------------------------------  ------------------------  ----------------  ---------------------------------------
TCP        0.0.0.0:135                        *:*                       Listen            1234: C:\Windows\System32\rpcss.exe
TCP        127.0.0.1:8080                     127.0.0.1:54321           Established       5678: C:\Program Files\MyApp\app.exe
UDP        0.0.0.0:53                         *:*                       -                 9012: C:\Windows\System32\dns.exe
```

## Output Format

The output displays the following columns:

- **PROTO**: Protocol type (TCP or UDP)
- **LOCAL ADDRESS**: Local IP address and port
- **REMOTE ADDRESS**: Remote IP address and port (or `*:*` for UDP and listening TCP)
- **STATE**: Connection state (TCP only; UDP shows `-`)
- **PROCESS**: Process ID and executable path

### Connection States

TCP connections can have the following states:
- `Listen`: Waiting for incoming connections
- `Established`: Active connection
- `SynSent`: Connection request sent
- `SynReceived`: Connection request received
- `FinWait1/FinWait2`: Connection closing phases
- `CloseWait`: Waiting to close
- `Closing`: Closing connection
- `LastAck`: Final acknowledgment
- `TimeWait`: Waiting for network cleanup

## Development

### Build Commands

```bash
# Quick compile check
cargo check

# Build debug version
cargo build

# Build optimized release version
cargo build --release

# Run the application
cargo run
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests (when available)
cargo test
```

## Dependencies

- **netstat v0.7.0**: Cross-platform network socket information retrieval
- **sysinfo v0.30**: System and process information

## Architecture

The application follows a simple single-file architecture in `src/main.rs`:

1. **Data Collection**: Uses the `netstat` crate to retrieve socket information
2. **Process Resolution**: Uses `sysinfo` to map process IDs to executable paths
3. **Sorting**: Custom sorting by connection state, protocol, and local address
4. **Formatting**: Aligned tabular output for readability

## License

This project is open source. Please refer to the LICENSE file for details.