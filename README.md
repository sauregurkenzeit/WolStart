# WolStart Service

## Description

**WolStart Service** is a dedicated Windows service crafted to awaken specific programs on your machine via Wake-on-LAN (WOL) packets. Regardless of your location, this service grants you the ability to remotely start predefined applications without the necessity for physical access.

## Features

- **Attentive to WOL Packets**: Rigorously monitors network traffic for WOL packets and springs into action when detected.

- **Configurable Execution**: Users can fine-tune settings to specify which program to launch upon a WOL packet's reception.

- **Low System Impact**: Designed to consume minimal system resources, it doesn’t compromise on system performance.

- **Seamless Windows Integration**: Crafted as a native Windows service, it offers smooth interactions using familiar Windows tools.

## Prerequisites

- **Libpcap Dependency**: 
  - You must install [Npcap](https://nmap.org/npcap/).
  - Fetch the [Npcap SDK](https://nmap.org/npcap/#download) and incorporate the SDK's `/Lib` or `/Lib/x64` directory to your `LIB` environment variable.

## Configuration

The service comes with default settings which can be altered in `main.rs`:

```rust
const DEFAULT_PROGRAM: &str = "kodi.exe";
const DEFAULT_RUN_PATH: &str = "C:\\Program Files\\Kodi\\kodi.exe";
const DEFAULT_HOST_IP: &str = "192.168.1.132";
const DEFAULT_LOG_LEVEL: &str = "warn";
```

## Installation

1. Clone the repository:
```
git clone https://github.com/YourUsername/WolStart.git
```

2. Head to the project directory and compile with Cargo:
```
cd WolStart
cargo build --release
```

3. To install the service, use:
```
wolstart.exe install
```

During installation, you can also provide specific parameters to override the default settings:

```
wolstart.exe install --program YOUR_PROGRAM.exe --run-path YOUR_PATH --host-ip YOUR_IP --log-level YOUR_LOG_LEVEL
```

## Usage

After installing the service, it will continuously listen for WOL packets on the designated interface and IP. To manage the service:

- **Start the Service**: 
  ```
  net start WolStartService
  ```

- **Stop the Service**: 
  ```
  net stop WolStartService
  ```

- **Uninstall the Service**:
  ```
  wolstart.exe uninstall
  ```

Logs pertaining to the service's operations are recorded in `/system32/wol_service.log`. The verbosity of these logs is influenced by the `LOG_LEVEL` argument.

## Contributing

Enthusiasts and contributors are the backbone of the WolStart Service project! Please see [CONTRIBUTING.md](link_to_contributing_file) for guidelines.

## License

WolStart Service is licensed under the [MIT License](LICENSE.md).

---

This adjusted documentation should better reflect your updated service's functionalities and usage. Adjust further as required.