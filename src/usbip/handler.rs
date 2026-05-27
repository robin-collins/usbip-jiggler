use byteorder::{BigEndian, ReadBytesExt};
use std::io::{self, Write};
use std::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::hid::{
    mouse::MouseReport, CONFIGURATION_DESCRIPTOR, DEVICE_DESCRIPTOR,
    REPORT_DESCRIPTOR, STRING_DESCRIPTOR_0, string_descriptor,
};
use crate::usbip::{
    CmdSubmit, CmdUnlink, RetSubmit, RetUnlink, NUMBER_OF_PACKETS_NON_ISOCH,
    USBIP_CMD_SUBMIT, USBIP_CMD_UNLINK,
};

pub fn handle_urb(
    mut stream: TcpStream,
    addr: std::net::SocketAddr,
    rx: &mut mpsc::Receiver<MouseReport>,
) {
    loop {
        let cmd = match stream.read_u32::<BigEndian>() {
            Ok(v) => v,
            Err(_) => {
                info!("client disconnected: {}", addr);
                break;
            }
        };

        match cmd {
            USBIP_CMD_SUBMIT => {
                let submit = match CmdSubmit::read_from(&mut stream) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("malformed CMD_SUBMIT from {}: {}", addr, e);
                        break;
                    }
                };

                // Drain OUT transfer data (ignored for our mouse)
                if submit.direction == 0 && submit.transfer_buffer_length > 0 {
                    let mut buf = vec![0u8; submit.transfer_buffer_length as usize];
                    if let Err(e) = io::Read::read_exact(&mut stream, &mut buf) {
                        error!("read transfer data failed: {}", e);
                        break;
                    }
                }

                let (data, status) = if submit.ep == 1 {
                    let report = rx.try_recv().unwrap_or([0u8; 3]);
                    (report.to_vec(), 0i32)
                } else if submit.ep == 0 {
                    handle_control(&submit)
                } else {
                    (vec![], -32i32)
                };

                let ret = RetSubmit {
                    seqnum: submit.seqnum,
                    devid: submit.devid,
                    direction: submit.direction,
                    ep: submit.ep,
                    status,
                    actual_length: data.len() as i32,
                    start_frame: 0,
                    number_of_packets: NUMBER_OF_PACKETS_NON_ISOCH,
                    error_count: 0,
                };

                let mut buf: Vec<u8> = Vec::with_capacity(48 + data.len());
                if ret.write_to(&mut buf).is_err() {
                    info!("client disconnected: {}", addr);
                    break;
                }
                buf.extend_from_slice(&data);
                if stream.write_all(&buf).is_err() {
                    info!("client disconnected: {}", addr);
                    break;
                }
            }

            USBIP_CMD_UNLINK => {
                let unlink = match CmdUnlink::read_from(&mut stream) {
                    Ok(u) => u,
                    Err(e) => {
                        error!("malformed CMD_UNLINK from {}: {}", addr, e);
                        break;
                    }
                };
                tracing::debug!("unlink request for submit seqnum {}", unlink.unlink_seqnum);
                let ret = RetUnlink {
                    seqnum: unlink.seqnum,
                    devid: unlink.devid,
                    direction: unlink.direction,
                    ep: unlink.ep,
                    status: 0,
                };
                let mut buf: Vec<u8> = Vec::with_capacity(40);
                if ret.write_to(&mut buf).is_err() || stream.write_all(&buf).is_err() {
                    info!("client disconnected: {}", addr);
                    break;
                }
            }

            other => {
                error!("unknown URB command {:#010x} from {}", other, addr);
                break;
            }
        }
    }
}

fn handle_control(submit: &CmdSubmit) -> (Vec<u8>, i32) {
    let bm_request_type = submit.setup[0];
    let b_request = submit.setup[1];
    let w_value = u16::from_le_bytes([submit.setup[2], submit.setup[3]]);
    let w_length = u16::from_le_bytes([submit.setup[6], submit.setup[7]]) as usize;

    match (bm_request_type, b_request) {
        (0x80, 0x06) => {
            let desc_type = (w_value >> 8) as u8;
            let desc_idx = (w_value & 0xFF) as u8;
            let data: Vec<u8> = match desc_type {
                0x01 => DEVICE_DESCRIPTOR.to_vec(),
                0x02 => CONFIGURATION_DESCRIPTOR.to_vec(),
                0x03 => match desc_idx {
                    0 => STRING_DESCRIPTOR_0.to_vec(),
                    1 => string_descriptor("Rust"),
                    2 => string_descriptor("Mouse Jiggler"),
                    _ => vec![],
                },
                _ => vec![],
            };
            (data[..data.len().min(w_length)].to_vec(), 0)
        }
        (0x81, 0x06) => {
            let desc_type = (w_value >> 8) as u8;
            if desc_type == 0x22 {
                (REPORT_DESCRIPTOR[..REPORT_DESCRIPTOR.len().min(w_length)].to_vec(), 0)
            } else {
                (vec![], 0)
            }
        }
        _ => (vec![], 0),
    }
}
