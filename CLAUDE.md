# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a simple Rust CLI application that displays network socket information (similar to the `netstat` command). It uses the `netstat` crate to retrieve information about TCP and UDP sockets, including local/remote addresses, ports, associated process IDs, and connection states.

## Commands

### Build and Run
```bash
cargo build          # Build the project
cargo run            # Build and run the application
cargo build --release # Build optimized release version
```

### Development
```bash
cargo check          # Quick compile check without producing binary
cargo clippy         # Run Rust linter
cargo fmt            # Format code
```

### Testing
```bash
cargo test           # Run tests (currently no tests exist)
```

## Architecture

The application has a simple single-file architecture:

- **src/main.rs**: Contains the entire application logic
  - Uses `netstat::get_sockets_info()` to retrieve socket information
  - Filters for both IPv4/IPv6 address families and TCP/UDP protocols
  - Iterates through socket information and formats output differently for TCP vs UDP connections
  - TCP connections show: local address/port → remote address/port, PIDs, and connection state
  - UDP connections show: local address/port → *:*, PIDs (no remote info or state for UDP)

## Dependencies

- **netstat v0.7.0**: Provides cross-platform network socket information retrieval

The application is straightforward with no complex architecture patterns - it's a direct CLI utility that fetches and displays network socket data.