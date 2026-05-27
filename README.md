# usbip-jiggler

A Rust binary that presents a virtual USB HID mouse to a VM via the USB/IP protocol. It periodically sends small, randomised mouse movements to prevent the guest OS from activating its screensaver or lock screen. Each movement is net-zero — the cursor returns to its origin every cycle and is imperceptible during normal use.

## Features

- Full USB/IP protocol implementation (v0.111, port 3240)
- HID boot-protocol mouse — no guest drivers required
- Diagonal jiggle every 30 seconds with random ±1–20 px displacement
- Net-zero movement per cycle; cursor never drifts
- Single static binary **and** Docker container
- Zero runtime configuration
- Works with Linux and Windows VM guests

## Quick Start

### Docker (recommended)

Pull the pre-built image from the GitHub Container Registry and run it:

```bash
docker run -d --name usbip-jiggler -p 3240:3240 ghcr.io/robin-collins/usbip-jiggler:latest
```

To follow logs:

```bash
docker logs -f usbip-jiggler
```

To stop:

```bash
docker stop usbip-jiggler && docker rm usbip-jiggler
```

Available tags:

| Tag | Description |
|-----|-------------|
| `latest` | Most recent build from `main` |
| `sha-<hash>` | Pinned to a specific commit |
| `v1.2.3` | Pinned to a release version |

### Binary

```bash
cargo build --release
./target/release/mouse-jiggler
```

The server listens on `0.0.0.0:3240`.

### Build Docker image locally

```bash
docker build -t usbip-jiggler .
docker run -p 3240:3240 usbip-jiggler
```

## Attaching from the VM

### Linux guest

```bash
sudo modprobe vhci-hcd
sudo usbip attach -r <host-ip> -b 1-1
```

### Windows guest

Install [usbip-win](https://github.com/cezanne/usbip-win), then:

```cmd
usbip attach -r <host-ip> -b 1-1
```

The device enumerates as a standard HID mouse. No additional drivers are required.

## Requirements

- Rust 1.85+ (edition 2024)
- Docker (optional, for container builds)

## Building from Source

```bash
git clone https://github.com/robin-collins/usbip-jiggler.git
cd usbip-jiggler
cargo build --release
```

## Logging

Set `RUST_LOG` to control verbosity (default: `info`):

```bash
RUST_LOG=debug ./target/release/mouse-jiggler
```

Connection events are logged; per-jiggle noise is suppressed.

## Project Structure

```
usbip-jiggler/
├── Cargo.toml
├── Dockerfile
└── src/
    ├── main.rs           # Entry point, tokio runtime, jiggle task
    ├── usbip/
    │   ├── mod.rs        # Protocol constants and message structs
    │   ├── server.rs     # TCP listener, OP_REQ_DEVLIST, OP_REQ_IMPORT
    │   └── handler.rs    # URB submit/unlink handling
    └── hid/
        ├── mod.rs        # Device, config, HID, and report descriptors
        └── mouse.rs      # Jiggle task, MouseReport type
```

## USB Device Identity

| Field               | Value                  |
|---------------------|------------------------|
| USB version         | 2.0                    |
| Vendor ID           | `0x0627` (QEMU)        |
| Product ID          | `0x0001`               |
| Interface class     | 0x03 (HID)             |
| Interface subclass  | 0x01 (Boot Interface)  |
| Interface protocol  | 0x02 (Mouse)           |
| Endpoint            | 0x81 (Interrupt IN)    |
| Bus ID              | `1-1`                  |

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.
