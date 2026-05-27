use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::hid::{CONFIGURATION_DESCRIPTOR, DEVICE_DESCRIPTOR};
use crate::usbip::{
    BUSID, OP_REP_DEVLIST, OP_REP_IMPORT, OP_REQ_DEVLIST, OP_REQ_IMPORT, USBIP_VERSION,
    handler::handle_urb,
};

pub fn run_server(mut rx: mpsc::Receiver<[u8; 4]>) {
    let listener = TcpListener::bind("0.0.0.0:3240").expect("bind 0.0.0.0:3240");
    info!("listening on 0.0.0.0:3240");

    let busy = Arc::new(AtomicBool::new(false));

    for conn in listener.incoming() {
        let stream = match conn {
            Ok(s) => s,
            Err(_) => continue,
        };
        let addr = stream.peer_addr().unwrap();
        info!("client connected: {}", addr);

        if busy.load(Ordering::SeqCst) {
            warn!("rejected connection from {}: device busy", addr);
            let _ = send_import_error(stream);
            continue;
        }

        busy.store(true, Ordering::SeqCst);

        match handle_handshake(stream, addr) {
            HandshakeResult::Imported(stream) => {
                info!("device imported by {}", addr);
                handle_urb(stream, addr, &mut rx);
                // Drain channel so stale reports don't confuse next client
                while rx.try_recv().is_ok() {}
                info!("client disconnected: {}", addr);
            }
            HandshakeResult::DevlistOnly => {}
            HandshakeResult::Error => {}
        }

        busy.store(false, Ordering::SeqCst);
    }
}

enum HandshakeResult {
    Imported(TcpStream),
    DevlistOnly,
    Error,
}

fn handle_handshake(mut stream: TcpStream, _addr: std::net::SocketAddr) -> HandshakeResult {
    let mut sent_devlist = false;
    loop {
        // Read common header: version(u16), code(u16), status(u32)
        let version = match stream.read_u16::<BigEndian>() {
            Ok(v) => v,
            Err(e) if sent_devlist && is_eof(&e) => return HandshakeResult::DevlistOnly,
            Err(_) => return HandshakeResult::Error,
        };
        if version != USBIP_VERSION {
            warn!(
                "rejected handshake: client version {:#06x} != expected {:#06x}",
                version, USBIP_VERSION
            );
            return HandshakeResult::Error;
        }
        let op_code = match stream.read_u16::<BigEndian>() {
            Ok(v) => v,
            Err(_) => return HandshakeResult::Error,
        };
        let _status = match stream.read_u32::<BigEndian>() {
            Ok(v) => v,
            Err(_) => return HandshakeResult::Error,
        };

        match op_code {
            OP_REQ_DEVLIST => {
                if send_devlist(&mut stream).is_err() {
                    return HandshakeResult::Error;
                }
                sent_devlist = true;
                // loop: client may follow immediately with OP_REQ_IMPORT
            }
            OP_REQ_IMPORT => {
                let mut busid = [0u8; 32];
                if stream.read_exact(&mut busid).is_err() {
                    return HandshakeResult::Error;
                }
                let requested = std::str::from_utf8(&busid)
                    .unwrap_or("")
                    .trim_end_matches('\0');
                if requested != BUSID {
                    let _ = send_import_error(stream);
                    return HandshakeResult::Error;
                }
                if send_import_ok(&mut stream).is_err() {
                    return HandshakeResult::Error;
                }
                return HandshakeResult::Imported(stream);
            }
            _ => return HandshakeResult::Error,
        }
    }
}

fn is_eof(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
    )
}

fn send_devlist<W: Write>(w: &mut W) -> io::Result<()> {
    // OP_REP_DEVLIST header
    w.write_u16::<BigEndian>(USBIP_VERSION)?;
    w.write_u16::<BigEndian>(OP_REP_DEVLIST)?;
    w.write_u32::<BigEndian>(0)?; // status: OK
    w.write_u32::<BigEndian>(1)?; // num_exported_devices

    write_device_info(w)?;
    w.flush()
}

fn write_device_info<W: Write>(w: &mut W) -> io::Result<()> {
    // path (256 bytes)
    let mut path = [0u8; 256];
    let p = b"/sys/devices/pci0000:00/0000:00:01.0/usb1/1-1";
    path[..p.len()].copy_from_slice(p);
    w.write_all(&path)?;

    // busid (32 bytes)
    let mut busid = [0u8; 32];
    let b = BUSID.as_bytes();
    busid[..b.len()].copy_from_slice(b);
    w.write_all(&busid)?;

    w.write_u32::<BigEndian>(1)?; // busnum
    w.write_u32::<BigEndian>(1)?; // devnum
    w.write_u32::<BigEndian>(3)?; // speed: USB_SPEED_HIGH(3) → maps to USB 2.0

    // VID, PID from device descriptor (bytes 8-11, little-endian → read as LE)
    let vid = u16::from_le_bytes([DEVICE_DESCRIPTOR[8], DEVICE_DESCRIPTOR[9]]);
    let pid = u16::from_le_bytes([DEVICE_DESCRIPTOR[10], DEVICE_DESCRIPTOR[11]]);
    w.write_u16::<BigEndian>(vid)?;
    w.write_u16::<BigEndian>(pid)?;
    w.write_u16::<BigEndian>(0x0100)?; // bcdDevice

    w.write_u8(DEVICE_DESCRIPTOR[4])?; // bDeviceClass
    w.write_u8(DEVICE_DESCRIPTOR[5])?; // bDeviceSubClass
    w.write_u8(DEVICE_DESCRIPTOR[6])?; // bDeviceProtocol
    w.write_u8(CONFIGURATION_DESCRIPTOR[4])?; // bConfigurationValue
    w.write_u8(DEVICE_DESCRIPTOR[14])?; // bNumConfigurations
    w.write_u8(1)?; // bNumInterfaces

    // Interface info
    w.write_u8(CONFIGURATION_DESCRIPTOR[11])?; // bInterfaceClass (HID=0x03)
    w.write_u8(CONFIGURATION_DESCRIPTOR[12])?; // bInterfaceSubClass
    w.write_u8(CONFIGURATION_DESCRIPTOR[13])?; // bInterfaceProtocol
    w.write_u8(0)?; // padding

    Ok(())
}

fn send_import_ok<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_u16::<BigEndian>(USBIP_VERSION)?;
    w.write_u16::<BigEndian>(OP_REP_IMPORT)?;
    w.write_u32::<BigEndian>(0)?; // status: OK
    write_device_info(w)?;
    w.flush()
}

fn send_import_error(mut stream: TcpStream) -> io::Result<()> {
    stream.write_u16::<BigEndian>(USBIP_VERSION)?;
    stream.write_u16::<BigEndian>(OP_REP_IMPORT)?;
    stream.write_u32::<BigEndian>(1)?; // status: error
    stream.flush()
}
