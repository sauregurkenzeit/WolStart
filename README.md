# WolStart

## Description

**WolStart** is a utility designed to awaken and run specific programs on your machine through Wake-on-LAN (WOL) packets. It's ideal for remotely initiating applications or services without physical interaction.

## Features

- **Responsive to WOL Packets**: Efficiently scans network traffic for incoming WOL packets and acts upon detection.

- **Customizable Execution**: Modify constants to specify which program to launch upon receipt of a WOL packet.

- **Resource-friendly**: Streamlined code ensures minimal resource usage.

- **Windows-focused**: Tailored for the Windows environment with necessary dependencies.

## Prerequisites

- **Libpcap Dependency**: 
  - Install [Npcap](https://nmap.org/npcap/).
  - Download the [Npcap SDK](https://nmap.org/npcap/#download).
  - Append the SDK's `/Lib` or `/Lib/x64` folder to your `LIB` environment variable.

## Configuration

Before building, you need to set some constants in `main.rs`:

```rust
const PROGRAM: &str = "YOUR_PROGRAM_NAME.exe"; 
const RUN_PATH: &str = "PATH_TO_YOUR_PROGRAM"; 
const HOST_IP: &str = "YOUR_IP_ADDRESS";
```

For instance:
```rust
const PROGRAM: &str = "kodi.exe";
const RUN_PATH: &str = "C:\\Program Files\\Kodi\\kodi.exe";
const HOST_IP: &str = "192.168.1.132";
```

## Installation

1. Clone the repository:
```
git clone https://github.com/YourUsername/WolStart.git
```

2. Navigate to the directory and build with Cargo:
```
cd WolStart
cargo build --release
```

3. The compiled binary will be available under `target/release`.

## Usage

Simply run the compiled binary, and WolStart will begin monitoring for WOL packets on the predefined interface and IP:

```
wolstart.exe
```

## Contributing

Contributions are welcomed and appreciated! Please consult [CONTRIBUTING.md](link_to_contributing_file) for guidelines on how to contribute.

## License

This software is distributed under the [MIT License](LICENSE.md).

---

Hope this suits your needs!