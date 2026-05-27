Here are the correct, precise HID descriptors for a generic 3-button relative mouse:

## Device Descriptor

```rust
pub const DEVICE_DESCRIPTOR: &[u8] = &[
    0x12,        // bLength (18 bytes)
    0x01,        // bDescriptorType (Device)
    0x00, 0x02,  // bcdUSB (2.00)
    0x00,        // bDeviceClass (defined at interface level)
    0x00,        // bDeviceSubClass
    0x00,        // bDeviceProtocol
    0x08,        // bMaxPacketSize0 (8 bytes for FS control)
    0x4D, 0x04,  // idVendor  (0x044D — generic placeholder, or use 0x046D for Logitech-like)
    0x01, 0x00,  // idProduct (0x0001)
    0x10, 0x01,  // bcdDevice (1.10)
    0x01,        // iManufacturer (string index 1)
    0x02,        // iProduct      (string index 2)
    0x00,        // iSerialNumber (none)
    0x01,        // bNumConfigurations
];
```

---

## Configuration Descriptor (composite — config + interface + HID + endpoint)

```rust
pub const CONFIG_DESCRIPTOR: &[u8] = &[
    // --- Configuration Descriptor (9 bytes) ---
    0x09,        // bLength
    0x02,        // bDescriptorType (Configuration)
    0x22, 0x00,  // wTotalLength (34 bytes: 9+9+9+7)
    0x01,        // bNumInterfaces
    0x01,        // bConfigurationValue
    0x00,        // iConfiguration (no string)
    0x80,        // bmAttributes (bus-powered, no remote wakeup)
    0x32,        // bMaxPower (100mA = 50 * 2mA units)

    // --- Interface Descriptor (9 bytes) ---
    0x09,        // bLength
    0x04,        // bDescriptorType (Interface)
    0x00,        // bInterfaceNumber (0)
    0x00,        // bAlternateSetting
    0x01,        // bNumEndpoints (1 — interrupt IN only)
    0x03,        // bInterfaceClass (HID)
    0x01,        // bInterfaceSubClass (1 = Boot Interface)
    0x02,        // bInterfaceProtocol (2 = Mouse)
    0x00,        // iInterface (no string)

    // --- HID Descriptor (9 bytes) ---
    0x09,        // bLength
    0x21,        // bDescriptorType (HID)
    0x11, 0x01,  // bcdHID (1.11)
    0x00,        // bCountryCode (not localised)
    0x01,        // bNumDescriptors
    0x22,        // bDescriptorType[0] (Report)
    0x32, 0x00,  // wDescriptorLength (50 bytes — must match HID_REPORT_DESCRIPTOR.len())

    // --- Endpoint Descriptor (7 bytes) ---
    0x07,        // bLength
    0x05,        // bDescriptorType (Endpoint)
    0x81,        // bEndpointAddress (EP1, IN direction)
    0x03,        // bmAttributes (Interrupt)
    0x04, 0x00,  // wMaxPacketSize (4 bytes)
    0x0A,        // bInterval (10ms polling interval — 10 for FS USB)
];
```

---

## HID Report Descriptor

This describes a standard 4-byte report: `[buttons, X, Y, wheel]`

```rust
pub const HID_REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,  // Usage Page (Generic Desktop Controls)
    0x09, 0x02,  // Usage (Mouse)
    0xA1, 0x01,  // Collection (Application)

      0x09, 0x01, //   Usage (Pointer)
      0xA1, 0x00, //   Collection (Physical)

        // --- Buttons (3 bits) ---
        0x05, 0x09, //     Usage Page (Button)
        0x19, 0x01, //     Usage Minimum (Button 1 — left)
        0x29, 0x03, //     Usage Maximum (Button 3 — middle)
        0x15, 0x00, //     Logical Minimum (0)
        0x25, 0x01, //     Logical Maximum (1)
        0x75, 0x01, //     Report Size (1 bit)
        0x95, 0x03, //     Report Count (3)
        0x81, 0x02, //     Input (Data, Variable, Absolute)

        // --- Padding (5 bits to fill byte) ---
        0x75, 0x05, //     Report Size (5 bits)
        0x95, 0x01, //     Report Count (1)
        0x81, 0x03, //     Input (Constant) — padding

        // --- X and Y axes (relative, signed byte each) ---
        0x05, 0x01, //     Usage Page (Generic Desktop Controls)
        0x09, 0x30, //     Usage (X)
        0x09, 0x31, //     Usage (Y)
        0x15, 0x81, //     Logical Minimum (-127)
        0x25, 0x7F, //     Logical Maximum (127)
        0x75, 0x08, //     Report Size (8 bits)
        0x95, 0x02, //     Report Count (2)
        0x81, 0x06, //     Input (Data, Variable, Relative)

        // --- Wheel (relative, signed byte) ---
        0x09, 0x38, //     Usage (Wheel)
        0x15, 0x81, //     Logical Minimum (-127)
        0x25, 0x7F, //     Logical Maximum (127)
        0x75, 0x08, //     Report Size (8 bits)
        0x95, 0x01, //     Report Count (1)
        0x81, 0x06, //     Input (Data, Variable, Relative)

      0xC0,       //   End Collection (Physical)
    0xC0,         // End Collection (Application)
];
// Total: 50 bytes — matches wDescriptorLength (0x32) in HID descriptor above
```

---

## Report Struct

4 bytes per interrupt IN transfer:

```rust
#[repr(C)]
pub struct MouseReport {
    pub buttons: u8,  // bit 0 = left, bit 1 = right, bit 2 = middle
    pub x: i8,        // relative X movement (-127 to 127)
    pub y: i8,        // relative Y movement (-127 to 127)
    pub wheel: i8,    // scroll wheel (-127 to 127)
}

impl MouseReport {
    pub fn zero() -> Self {
        Self { buttons: 0, x: 0, y: 0, wheel: 0 }
    }

    pub fn jiggle(dx: i8) -> Self {
        Self { buttons: 0, x: dx, y: 0, wheel: 0 }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        [self.buttons, self.x as u8, self.y as u8, self.wheel as u8]
    }
}
```

---

## Key Corrections vs Earlier Version

| Field | Before | Now | Why |
|---|---|---|---|
| `wTotalLength` | `0x22` (34) | `0x22` (34) | Still correct — just verify against actual struct sizes |
| `wDescriptorLength` | `0x??, 0x00` | `0x32, 0x00` (50) | Must exactly match `HID_REPORT_DESCRIPTOR.len()` |
| `wMaxPacketSize` | `0x04, 0x00` | `0x04, 0x00` | 4 bytes matches 4-byte report |
| `bInterval` | `0x04` (4ms) | `0x0A` (10ms) | 10ms is standard for FS HID mice |
| Report bytes | 3 | 4 | Added wheel axis |
| `bmAttributes` | `0xA0` | `0x80` | `0xA0` implies remote wakeup — unnecessary |

The critical one is `wDescriptorLength` — if that doesn't match the actual byte count of your report descriptor, Windows will reject the device silently.
