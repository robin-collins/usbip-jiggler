pub mod handler;
pub mod server;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

// Protocol version
pub const USBIP_VERSION: u16 = 0x0111;

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

pub const BUSID: &str = "1-1";

/// USBIP_CMD_SUBMIT header (after the 4-byte command word)
#[derive(Debug)]
pub struct CmdSubmit {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: u32,
    pub ep: u32,
    pub transfer_flags: u32,
    pub transfer_buffer_length: u32,
    pub start_frame: u32,
    pub number_of_packets: u32,
    pub interval: u32,
    pub setup: [u8; 8],
}

impl CmdSubmit {
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(CmdSubmit {
            seqnum: r.read_u32::<BigEndian>()?,
            devid: r.read_u32::<BigEndian>()?,
            direction: r.read_u32::<BigEndian>()?,
            ep: r.read_u32::<BigEndian>()?,
            transfer_flags: r.read_u32::<BigEndian>()?,
            transfer_buffer_length: r.read_u32::<BigEndian>()?,
            start_frame: r.read_u32::<BigEndian>()?,
            number_of_packets: r.read_u32::<BigEndian>()?,
            interval: r.read_u32::<BigEndian>()?,
            setup: {
                let mut buf = [0u8; 8];
                r.read_exact(&mut buf)?;
                buf
            },
        })
    }
}

/// USBIP_RET_SUBMIT header
pub struct RetSubmit {
    pub seqnum: u32,
    pub devid: u32,
    pub direction: u32,
    pub ep: u32,
    pub status: i32,
    pub actual_length: u32,
    pub start_frame: u32,
    pub number_of_packets: u32,
    pub error_count: u32,
    pub setup: [u8; 8],
}

impl RetSubmit {
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<BigEndian>(USBIP_RET_SUBMIT)?;
        w.write_u32::<BigEndian>(self.seqnum)?;
        w.write_u32::<BigEndian>(self.devid)?;
        w.write_u32::<BigEndian>(self.direction)?;
        w.write_u32::<BigEndian>(self.ep)?;
        w.write_i32::<BigEndian>(self.status)?;
        w.write_u32::<BigEndian>(self.actual_length)?;
        w.write_u32::<BigEndian>(self.start_frame)?;
        w.write_u32::<BigEndian>(self.number_of_packets)?;
        w.write_u32::<BigEndian>(self.error_count)?;
        w.write_all(&self.setup)?;
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
    // 24 bytes padding
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

/// USBIP_RET_UNLINK header
pub struct RetUnlink {
    pub seqnum: u32,
    pub status: i32,
}

impl RetUnlink {
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<BigEndian>(USBIP_RET_UNLINK)?;
        w.write_u32::<BigEndian>(self.seqnum)?;
        w.write_u32::<BigEndian>(0)?; // devid
        w.write_u32::<BigEndian>(0)?; // direction
        w.write_u32::<BigEndian>(0)?; // ep
        w.write_i32::<BigEndian>(self.status)?;
        // 24 bytes padding
        w.write_all(&[0u8; 24])?;
        Ok(())
    }
}
