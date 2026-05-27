## Plan

- Rust USB/IP server emulating a **USB HID mouse**
- Sends a small relative movement report every N seconds
- VM sees it as a real USB mouse
- Works on Windows or Linux guest

---

## HID Mouse Descriptor

The report descriptor is the key part — tells the host what kind of data to expect:

```rust
const HID_REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,  // Usage Page (Generic Desktop)
    0x09, 0x02,  // Usage (Mouse)
    0xA1, 0x01,  // Collection (Application)
    0x09, 0x01,  //   Usage (Pointer)
    0xA1, 0x00,  //   Collection (Physical)
    0x05, 0x09,  //     Usage Page (Buttons)
    0x19, 0x01,  //     Usage Minimum (1)
    0x29, 0x03,  //     Usage Maximum (3)
    0x15, 0x00,  //     Logical Minimum (0)
    0x25, 0x01,  //     Logical Maximum (1)
    0x95, 0x03,  //     Report Count (3)
    0x75, 0x01,  //     Report Size (1)
    0x81, 0x02,  //     Input (Data, Variable, Absolute)
    0x95, 0x01,  //     Report Count (1)
    0x75, 0x05,  //     Report Size (5) -- padding
    0x81, 0x03,  //     Input (Constant)
    0x05, 0x01,  //     Usage Page (Generic Desktop)
    0x09, 0x30,  //     Usage (X)
    0x09, 0x31,  //     Usage (Y)
    0x15, 0x81,  //     Logical Minimum (-127)
    0x25, 0x7F,  //     Logical Maximum (127)
    0x75, 0x08,  //     Report Size (8)
    0x95, 0x02,  //     Report Count (2)
    0x81, 0x06,  //     Input (Data, Variable, Relative)
    0xC0,        //   End Collection
    0xC0,        // End Collection
];
```

Each report is 3 bytes: `[buttons, delta_x, delta_y]`

---

## Project Structure

```
mouse-jiggler/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── usbip/
│   │   ├── mod.rs       # Protocol constants & message types
│   │   ├── server.rs    # TCP listener, device list, import
│   │   └── handler.rs   # URB handling (submit/unlink)
│   └── hid/
│       ├── mod.rs       # Descriptors (device, config, hid, report)
│       └── mouse.rs     # Jiggle logic, report generation
```

---

## Cargo.toml

```toml
[package]
name = "mouse-jiggler"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
bytes = "1"
byteorder = "1"
tracing = "1"
tracing-subscriber = "0.3"
```

---

## Core USB/IP Protocol Types

```rust
// src/usbip/mod.rs
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub const USBIP_VERSION: u16 = 0x0111;
pub const OP_REQ_DEVLIST: u16 = 0x8005;
pub const OP_REP_DEVLIST: u16 = 0x0005;
pub const OP_REQ_IMPORT: u16  = 0x8003;
pub const OP_REP_IMPORT: u16  = 0x0003;
pub const USBIP_CMD_SUBMIT: u32 = 0x00000001;
pub const USBIP_RET_SUBMIT: u32 = 0x00000003;
pub const USBIP_CMD_UNLINK: u32 = 0x00000002;
pub const USBIP_RET_UNLINK: u32 = 0x00000004;

#[derive(Debug)]
pub struct UsbipHeader {
    pub command: u32,
    pub seqnum: u32,
    pub devid: u32,
    pub direction: u32,
    pub ep: u32,
}

#[derive(Debug)]
pub struct CmdSubmit {
    pub header: UsbipHeader,
    pub transfer_flags: u32,
    pub transfer_buffer_length: i32,
    pub start_frame: i32,
    pub number_of_packets: i32,
    pub interval: i32,
    pub setup: [u8; 8],
}
```

---

## HID Descriptors

```rust
// src/hid/mod.rs

pub const DEVICE_DESCRIPTOR: &[u8] = &[
    0x12,        // bLength
    0x01,        // bDescriptorType (Device)
    0x00, 0x02,  // bcdUSB 2.0
    0x00,        // bDeviceClass
    0x00,        // bDeviceSubClass
    0x00,        // bDeviceProtocol
    0x08,        // bMaxPacketSize0
    0x27, 0x06,  // idVendor  (arbitrary)
    0x01, 0x00,  // idProduct (arbitrary)
    0x00, 0x01,  // bcdDevice
    0x01,        // iManufacturer
    0x02,        // iProduct
    0x00,        // iSerialNumber
    0x01,        // bNumConfigurations
];

// Full config: Configuration + Interface + HID + Endpoint descriptors
pub const CONFIG_DESCRIPTOR: &[u8] = &[
    // Configuration descriptor
    0x09, 0x02, 0x22, 0x00, 0x01, 0x01, 0x00, 0xA0, 0x32,
    // Interface descriptor (HID, boot protocol mouse)
    0x09, 0x04, 0x00, 0x00, 0x01, 0x03, 0x01, 0x02, 0x00,
    // HID descriptor
    0x09, 0x21, 0x11, 0x01, 0x00, 0x01, 0x22, 
    (HID_REPORT_DESCRIPTOR.len() as u8), 0x00,
    // Endpoint descriptor (interrupt IN, 4ms interval)
    0x07, 0x05, 0x81, 0x03, 0x04, 0x00, 0x04,
];

pub const HID_REPORT_DESCRIPTOR: &[u8] = &[
    // ... (as above)
];

pub fn string_descriptor(s: &str) -> Vec<u8> {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let len = 2 + utf16.len() * 2;
    let mut desc = vec![len as u8, 0x03];
    for ch in utf16 {
        desc.push(ch as u8);
        desc.push((ch >> 8) as u8);
    }
    desc
}
```

---

## The Jiggler Logic

```rust
// src/hid/mouse.rs
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone)]
pub struct MouseReport {
    pub buttons: u8,
    pub dx: i8,
    pub dy: i8,
}

impl MouseReport {
    pub fn to_bytes(&self) -> [u8; 3] {
        [self.buttons, self.dx as u8, self.dy as u8]
    }
}

pub async fn jiggle_task(tx: mpsc::Sender<MouseReport>, jiggle_px: i8, period_secs: u64) {
    let mut ticker = interval(Duration::from_secs(period_secs));
    let mut toggle = false;

    loop {
        ticker.tick().await;

        // Move right, then back left — net zero, just enough to reset idle
        let dx = if toggle { jiggle_px } else { -jiggle_px };
        toggle = !toggle;

        let report = MouseReport { buttons: 0, dx, dy: 0 };
        if tx.send(report).await.is_err() {
            break;
        }

        // Send a zero report immediately after (mouse "stops")
        let zero = MouseReport { buttons: 0, dx: 0, dy: 0 };
        let _ = tx.send(zero).await;
    }
}
```

---

## How the URB Handling Works

The interrupt IN endpoint is where the magic happens. When the VM polls the endpoint, you respond with a mouse report (or zeros if nothing to send):

```rust
// In your URB handler, ep == 1 (interrupt IN) means the host wants a report
async fn handle_submit(cmd: CmdSubmit, report_rx: &mut mpsc::Receiver<MouseReport>) 
    -> Vec<u8> 
{
    if cmd.header.ep == 1 && cmd.header.direction == 1 {
        // Interrupt IN — send pending report or zeros
        let report = report_rx.try_recv()
            .unwrap_or(MouseReport { buttons: 0, dx: 0, dy: 0 });
        
        build_ret_submit(cmd.header.seqnum, &report.to_bytes())
    } else if cmd.header.ep == 0 {
        // Control endpoint — handle GET_DESCRIPTOR etc.
        handle_control(cmd)
    } else {
        build_ret_submit_empty(cmd.header.seqnum)
    }
}
```

---

## VM Side Setup

**Linux guest:**
```bash
sudo modprobe vhci-hcd
sudo usbip attach -r <host-ip> -b 1-1
```

**Windows guest:** Install the [usbip-win](https://github.com/cezanne/usbip-win) driver, then:
```cmd
usbip attach -r <host-ip> -b 1-1
```

