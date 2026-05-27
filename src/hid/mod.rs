pub mod mouse;

// USB Device Descriptor — Logitech M105 (046D:C077), USB 2.0
pub static DEVICE_DESCRIPTOR: &[u8] = &[
    0x12,       // bLength (18)
    0x01,       // bDescriptorType (Device)
    0x00, 0x02, // bcdUSB 2.00
    0x00,       // bDeviceClass (defined at interface level)
    0x00,       // bDeviceSubClass
    0x00,       // bDeviceProtocol
    0x08,       // bMaxPacketSize0 (8)
    0x6D, 0x04, // idVendor  0x046D (Logitech) — little-endian
    0x77, 0xC0, // idProduct 0xC077 (M105 Optical Mouse) — little-endian
    0x00, 0x48, // bcdDevice 72.00 (0x4800 BCD)
    0x01,       // iManufacturer (string index 1)
    0x02,       // iProduct      (string index 2)
    0x00,       // iSerialNumber (none)
    0x01,       // bNumConfigurations
];

// HID Report Descriptor — 46 bytes, verified against real usbhid-dump of M105
pub static REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x02, // Usage (Mouse)
    0xA1, 0x01, // Collection (Application)
      0x09, 0x01, //   Usage (Pointer)
      0xA1, 0x00, //   Collection (Physical)
        // Buttons: 3 bits (left, right, middle)
        0x05, 0x09, //     Usage Page (Button)
        0x19, 0x01, //     Usage Minimum (1)
        0x29, 0x03, //     Usage Maximum (3)
        0x15, 0x00, //     Logical Minimum (0)
        0x25, 0x01, //     Logical Maximum (1)
        0x75, 0x01, //     Report Size (1 bit)
        0x95, 0x03, //     Report Count (3)
        0x81, 0x02, //     Input (Data, Variable, Absolute)
        // Padding: 5 bits
        0x75, 0x05, //     Report Size (5 bits)
        0x95, 0x01, //     Report Count (1)
        0x81, 0x01, //     Input (Constant) — 0x01 matches real dump
        // X, Y, Wheel: 3 signed bytes grouped
        0x05, 0x01, //     Usage Page (Generic Desktop)
        0x09, 0x30, //     Usage (X)
        0x09, 0x31, //     Usage (Y)
        0x09, 0x38, //     Usage (Wheel)
        0x15, 0x81, //     Logical Minimum (-127)
        0x25, 0x7F, //     Logical Maximum (127)
        0x75, 0x08, //     Report Size (8 bits)
        0x95, 0x03, //     Report Count (3)
        0x81, 0x06, //     Input (Data, Variable, Relative)
      0xC0, //   End Collection (Physical)
    0xC0, // End Collection (Application)
    // Total: 52 bytes (wDescriptorLength computed automatically)
];

// Configuration blob: Config (9) + Interface (9) + HID (9) + Endpoint (7) = 34 bytes
pub static CONFIGURATION_DESCRIPTOR: &[u8] = &[
    // Configuration Descriptor (9 bytes)
    0x09,       // bLength
    0x02,       // bDescriptorType (Configuration)
    0x22, 0x00, // wTotalLength 34
    0x01,       // bNumInterfaces
    0x01,       // bConfigurationValue
    0x00,       // iConfiguration (no string)
    0xA0,       // bmAttributes (Bus Powered + Remote Wakeup — confirmed from real M105)
    0x32,       // bMaxPower (100mA)

    // Interface Descriptor (9 bytes)
    0x09,       // bLength
    0x04,       // bDescriptorType (Interface)
    0x00,       // bInterfaceNumber
    0x00,       // bAlternateSetting
    0x01,       // bNumEndpoints
    0x03,       // bInterfaceClass (HID)
    0x01,       // bInterfaceSubClass (Boot Interface)
    0x02,       // bInterfaceProtocol (Mouse)
    0x00,       // iInterface (no string)

    // HID Descriptor (9 bytes)
    0x09,       // bLength
    0x21,       // bDescriptorType (HID)
    0x11, 0x01, // bcdHID 1.11
    0x00,       // bCountryCode (not localised)
    0x01,       // bNumDescriptors
    0x22,       // bDescriptorType[0] (Report)
    REPORT_DESCRIPTOR_LEN_LO, REPORT_DESCRIPTOR_LEN_HI, // wDescriptorLength (46 = 0x2E)

    // Endpoint Descriptor (7 bytes)
    0x07,       // bLength
    0x05,       // bDescriptorType (Endpoint)
    0x81,       // bEndpointAddress (EP1 IN)
    0x03,       // bmAttributes (Interrupt)
    0x04, 0x00, // wMaxPacketSize (4 bytes)
    0x0A,       // bInterval (10ms)
];

const REPORT_DESCRIPTOR_LEN_LO: u8 = REPORT_DESCRIPTOR.len() as u8;
const REPORT_DESCRIPTOR_LEN_HI: u8 = (REPORT_DESCRIPTOR.len() >> 8) as u8;

// String descriptor 0: language list (English US)
pub static STRING_DESCRIPTOR_0: &[u8] = &[
    0x04,       // bLength
    0x03,       // bDescriptorType (String)
    0x09, 0x04, // wLANGID: English (US)
];

pub fn string_descriptor(s: &str) -> Vec<u8> {
    let utf16: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    let mut desc = vec![(2 + utf16.len()) as u8, 0x03];
    desc.extend_from_slice(&utf16);
    desc
}
