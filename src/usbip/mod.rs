pub mod handler;
pub mod server;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

pub const USBIP_VERSION: u16 = 0x0111; // protocol v1.1.1

// OP codes (used during handshake phase)
pub const OP_REQ_DEVLIST: u16 = 0x8005;
pub const OP_REP_DEVLIST: u16 = 0x0005;
pub const OP_REQ_IMPORT: u16 = 0x8003;
pub const OP_REP_IMPORT: u16 = 0x0003;

// CMD codes (used during URB phase)
pub const USBIP_CMD_SUBMIT: u32 = 0x00000001;
pub const USBIP_CMD_UNLINK: u32 = 0x00000002;
pub const USBIP_RET_SUBMIT: u32 = 0x00000003;
pub const USBIP_RET_UNLINK: u32 = 0x00000004;

// Sentinel used in number_of_packets for non-isochronous transfers
pub const NUMBER_OF_PACKETS_NON_ISOCH: i32 = -1;

pub const BUSID: &str = "1-1";

/// USBIP_CMD_SUBMIT header (fields after the 4-byte command word, per usbip proto)
#[derive(Debug)]
pub struct CmdSubmit {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: u32,
    pub ep: u32,
    pub transfer_buffer_length: i32, // INT32 in protocol
    pub setup: [u8; 8],
}

impl CmdSubmit {
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let seqnum = r.read_u32::<BigEndian>()?;
        let devid = r.read_u32::<BigEndian>()?;
        let direction = r.read_u32::<BigEndian>()?;
        let ep = r.read_u32::<BigEndian>()?;
        let _transfer_flags = r.read_u32::<BigEndian>()?;  // URB flags; irrelevant for interrupt IN
        let transfer_buffer_length = r.read_i32::<BigEndian>()?;
        let _start_frame = r.read_i32::<BigEndian>()?;     // always 0 for non-isochronous
        let _number_of_packets = r.read_i32::<BigEndian>()?; // always -1 for non-isochronous
        let _interval = r.read_i32::<BigEndian>()?;         // polling hint; we use our own schedule
        let mut setup = [0u8; 8];
        r.read_exact(&mut setup)?;
        Ok(CmdSubmit { seqnum, devid, direction, ep, transfer_buffer_length, setup })
    }
}

/// USBIP_RET_SUBMIT header — total 48 bytes (header_basic 20 + payload 20 + padding 8)
pub struct RetSubmit {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: u32,
    pub ep: u32,
    pub status: i32,
    pub actual_length: i32,      // INT32 in protocol
    pub start_frame: i32,        // INT32 in protocol
    pub number_of_packets: i32,  // INT32; -1 for non-isochronous
    pub error_count: i32,        // INT32 in protocol
}

impl RetSubmit {
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<BigEndian>(USBIP_RET_SUBMIT)?;
        w.write_u32::<BigEndian>(self.seqnum)?;
        w.write_u32::<BigEndian>(self.devid)?;
        w.write_u32::<BigEndian>(self.direction)?;
        w.write_u32::<BigEndian>(self.ep)?;
        w.write_i32::<BigEndian>(self.status)?;
        w.write_i32::<BigEndian>(self.actual_length)?;
        w.write_i32::<BigEndian>(self.start_frame)?;
        w.write_i32::<BigEndian>(self.number_of_packets)?;
        w.write_i32::<BigEndian>(self.error_count)?;
        w.write_all(&[0u8; 8])?; // padding to reach 48-byte header size
        Ok(())
    }
}

/// USBIP_CMD_UNLINK header
#[derive(Debug)]
pub struct CmdUnlink {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: u32,
    pub ep: u32,
    pub unlink_seqnum: u32,
}

impl CmdUnlink {
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let seqnum = r.read_u32::<BigEndian>()?;
        let devid = r.read_u32::<BigEndian>()?;
        let direction = r.read_u32::<BigEndian>()?;
        let ep = r.read_u32::<BigEndian>()?;
        let unlink_seqnum = r.read_u32::<BigEndian>()?;
        let mut _pad = [0u8; 24];
        r.read_exact(&mut _pad)?;
        Ok(CmdUnlink { seqnum, devid, direction, ep, unlink_seqnum })
    }
}

/// USBIP_RET_UNLINK header — total 48 bytes (header_basic 20 + status 4 + padding 24)
pub struct RetUnlink {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: u32,
    pub ep: u32,
    pub status: i32,
}

impl RetUnlink {
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<BigEndian>(USBIP_RET_UNLINK)?;
        w.write_u32::<BigEndian>(self.seqnum)?;
        w.write_u32::<BigEndian>(self.devid)?;
        w.write_u32::<BigEndian>(self.direction)?;
        w.write_u32::<BigEndian>(self.ep)?;
        w.write_i32::<BigEndian>(self.status)?;
        w.write_all(&[0u8; 24])?; // padding
        Ok(())
    }
}
