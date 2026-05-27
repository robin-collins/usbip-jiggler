# USB/IP Mouse Jiggler — Specification

## Overview

A Rust binary that presents a USB HID mouse to a virtual machine via the USB/IP protocol. It periodically sends small, randomized mouse movement reports to prevent the guest OS from activating its screensaver or lock screen. The cursor returns to its origin each full cycle — movement is net-zero and imperceptible in normal use.

---

## Goals

- Emulate a USB HID mouse over USB/IP (RFC-compliant, port 3240)
- Send a diagonal jiggle movement every 30 seconds with net-zero displacement
- Work with both Linux and Windows VM guests without additional host-side drivers
- Ship as a single static binary and a Docker container
- Zero runtime configuration — sensible defaults are baked in

---

## Non-Goals

- Multiple simultaneous VM clients (single-owner device)
- CLI flags or config file (no runtime knobs)
- Screensaver bypass on the host — only the VM guest is affected
- Active reconnection to the VM; the server waits for clients to re-attach

---

## Protocol

The server implements the USB/IP protocol version `0x0111` over TCP, listening on **port 3240** bound to **0.0.0.0** (all interfaces).

### Message flow

1. Client connects (TCP)
2. Server handles `OP_REQ_DEVLIST` → responds with one device: a HID boot-protocol mouse at bus ID `1-1`
3. Client sends `OP_REQ_IMPORT` for bus ID `1-1` → server accepts, transitions to URB mode
4. Client polls the interrupt IN endpoint (EP1) with `USBIP_CMD_SUBMIT`
5. Server responds with pending mouse report or a zero report
6. On `USBIP_CMD_UNLINK` or connection close, server tears down session and returns to listening

---

## USB Device Identity

| Field            | Value                    |
|------------------|--------------------------|
| USB version      | 2.0                      |
| Vendor ID        | `0x0627` (QEMU)          |
| Product ID       | `0x0001`                 |
| Device class     | 0x00 (per-interface)     |
| Interface class  | 0x03 (HID)               |
| Interface subclass | 0x01 (Boot Interface)  |
| Interface protocol | 0x02 (Mouse)           |
| Endpoint         | 0x81 (Interrupt IN)      |
| Poll interval    | 4 ms                     |
| Max packet size  | 4 bytes                  |
| Bus ID           | `1-1`                    |

String descriptors:
- Index 1 (Manufacturer): `"Rust"`
- Index 2 (Product): `"Mouse Jiggler"`

---

## HID Report Format

Each report is **3 bytes**: `[buttons, delta_x, delta_y]`

- `buttons`: always `0x00` (no buttons pressed)
- `delta_x`: signed 8-bit relative X movement
- `delta_y`: signed 8-bit relative Y movement

The HID report descriptor declares:
- 3 button bits + 5-bit pad (absolute)
- X and Y axes, logical range −127..127 (relative)

---

## Jiggle Behavior

### Timing

- Interval: **30 seconds** between jiggle events (hardcoded)
- A jiggle event consists of two reports sent in immediate succession

### Movement

Each jiggle cycle picks a **random displacement** for both axes independently:

```
dx = random integer in [−20, −1] ∪ [1, 20]
dy = random integer in [−20, −1] ∪ [1, 20]
```

The cycle is **two ticks**:

| Tick | Report sent        | Effect              |
|------|--------------------|---------------------|
| 1    | `[0, dx, dy]`      | Cursor moves        |
| 2    | `[0, −dx, −dy]`    | Cursor returns home |

New random values are drawn each cycle, so movement distance varies but displacement always cancels. The cursor never drifts.

### Channel

The jiggle task runs independently and pushes reports into a bounded `mpsc` channel (capacity 4). The URB handler drains from this channel. If the channel is full (e.g. no client attached), the jiggle task drops the report and continues.

---

## Connection Lifecycle

### Single client

Only one client may be attached at a time. If a second TCP connection arrives while a client is already in URB mode, the server sends `OP_REP_IMPORT` with `status = 1` (error) and closes the new connection.

### Disconnect and re-attach

When the client disconnects (TCP FIN, RST, or VM reboot), the server:
1. Logs the disconnect
2. Clears any buffered reports from the channel
3. Returns to the listening state

The client (VM) can re-attach at any time with `usbip attach` without restarting the server.

---

## Logging

Minimal — connection events only. No per-jiggle noise.

| Event                  | Log level | Message format                         |
|------------------------|-----------|----------------------------------------|
| Server start           | INFO      | `listening on 0.0.0.0:3240`            |
| Client connected       | INFO      | `client connected: <addr>`             |
| Device imported        | INFO      | `device imported by <addr>`            |
| Client disconnected    | INFO      | `client disconnected: <addr>`          |
| Second client rejected | WARN      | `rejected connection from <addr>: device busy` |

Use `tracing` + `tracing-subscriber` with `RUST_LOG` env var support (default level: `info`).

---

## Error Handling

| Scenario                        | Behavior                                              |
|---------------------------------|-------------------------------------------------------|
| TCP write fails mid-session     | Log disconnect, return to listening                   |
| Malformed USB/IP message        | Log error, close connection, return to listening      |
| Unknown URB endpoint            | Respond with `USBIP_RET_SUBMIT`, `status = -EPIPE`, zero data |
| Unknown control request         | Respond with empty data, `status = 0`                 |
| Channel send fails (no client)  | Silently drop the report; jiggle task continues       |

The server never panics on client misbehavior. Only unrecoverable startup errors (e.g. `bind()` fails) cause process exit.

---

## Project Structure

```
mouse-jiggler/
├── Cargo.toml
├── Dockerfile
├── src/
│   ├── main.rs          # Startup, tokio runtime, jiggle task spawn
│   ├── usbip/
│   │   ├── mod.rs       # Protocol constants & message structs
│   │   ├── server.rs    # TCP listener, OP_REQ_DEVLIST, OP_REQ_IMPORT
│   │   └── handler.rs   # URB submit/unlink handling
│   └── hid/
│       ├── mod.rs       # Device, config, HID, report descriptors
│       └── mouse.rs     # Jiggle task, MouseReport type
```

---

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
bytes = "1"
byteorder = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
rand = "0.8"
```

---

## Docker

The container image should:
- Use a multi-stage build: `rust:slim` builder → `debian:slim` runtime
- Expose port 3240/tcp
- Run as a non-root user
- Have no entrypoint flags (no config to pass)

```dockerfile
# Build stage
FROM rust:slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN useradd -m jiggler
COPY --from=builder /app/target/release/mouse-jiggler /usr/local/bin/
USER jiggler
EXPOSE 3240
CMD ["mouse-jiggler"]
```

---

## VM Setup

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

The device will enumerate as a standard HID mouse. No additional drivers required.

---

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Config surface | Hardcoded defaults | No runtime knobs needed; reduces surface area and deployment complexity |
| Multi-client | Reject second client | USB devices are single-owner; simplifies state machine considerably |
| Bind address | 0.0.0.0 | VMs on the host network need to reach the server without extra flags |
| Jiggle pattern | Diagonal (X + Y) | Harder for aggressive screensavers to ignore than axis-only movement |
| Movement range | Random ±1–20px each axis | Varies each cycle to avoid pattern detection; net-zero per cycle |
| Zero report | Yes, sent immediately after movement | Cursor snaps back; no accumulation over time |
| Disconnect behavior | Server re-listens | VM reboots are common; not requiring a server restart is ergonomic |
| Logging | Connection events only | Jiggle fires every 30s; per-tick logs would flood long-running sessions |
| Port | 3240 | Standard USB/IP port; clients attach with no extra flags |
| Deployment | Binary + Docker | Covers bare-metal host and containerized host environments |
