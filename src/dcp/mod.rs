use byteorder::{ByteOrder, NetworkEndian};
mod block;

pub use block::DcpOption;
use num_enum::TryFromPrimitive;

pub static DCP_MAC_HELLO_ADDRESS: [u8; 6] = [0x01, 0x0e, 0xcf, 0x00, 0x00, 0x01];

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(TryFromPrimitive)]
#[repr(u16)]
pub enum FrameID {
    Hello = 0xfefc,
    GetSet = 0xfefd,
    Request = 0xfefe,
    Reset = 0xfeff,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum ServiceType {
    Request = 0,
    Success = 1,
    NotSupported = 5,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum ServiceID {
    Get = 3,
    Set = 4,
    Identify = 5,
    Hello = 6,
}

mod field {
    use crate::field::*;

    pub const FRAME_ID: Field = 0..2;
    pub const SERVICE_ID: SmallField = 2;
    pub const SERVICE_TYPE: SmallField = 3;
    pub const X_ID: Field = 4..8;
    pub const RESPONSE_DELAY: Field = 8..10;
    pub const DATA_LENGTH: Field = 10..12;
    pub const PAYLOAD: Rest = 12..;
}

pub const DCP_HEADER_LENGTH: usize = field::PAYLOAD.start;

pub struct DcpHeaderFrame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> DcpHeaderFrame<T> {
    pub const fn new_unchecked(buffer: T) -> DcpHeaderFrame<T> {
        DcpHeaderFrame { buffer }
    }

    pub fn new_checked(buffer: T) -> Option<DcpHeaderFrame<T>> {
        let header = Self::new_unchecked(buffer);
        match header.check_len() {
            true => Some(header),
            false => None,
        }
    }

    pub fn check_len(&self) -> bool {
        let len = self.buffer.as_ref().len();

        len > DCP_HEADER_LENGTH
    }

    pub fn frame_id(&self) -> Option<FrameID> {
        let data = self.buffer.as_ref();
        let raw = NetworkEndian::read_u16(&data[field::FRAME_ID]);
        FrameID::try_from_primitive(raw).ok()
    }

    pub fn service_id(&self) -> Option<ServiceID> {
        let data = self.buffer.as_ref();
        let raw = data[field::SERVICE_ID];
        ServiceID::try_from_primitive(raw).ok()
    }

    pub fn service_type(&self) -> Option<ServiceType> {
        let data = self.buffer.as_ref();
        let raw = data[field::SERVICE_TYPE];
        ServiceType::try_from_primitive(raw).ok()
    }

    pub fn x_id(&self) -> u32 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u32(&data[field::X_ID])
    }

    pub fn response_delay(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[field::RESPONSE_DELAY])
    }

    pub fn data_length(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[field::DATA_LENGTH])
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();
        &data[field::PAYLOAD]
    }
}

pub struct DcpBlockHeader {
    pub option: u8,
    pub suboption: u8,
    pub block_length: u16,
}

impl DcpBlockHeader {
    pub fn parse(bytes: [u8; 4]) -> Option<Self> {
        Some(Self {
            option: bytes[0],
            suboption: bytes[1],
            block_length: u16::from_le_bytes(bytes[2..4].try_into().ok()?),
        })
    }
}

pub struct DcpBlock<'a> {
    pub header: DcpBlockHeader,
    pub options: [DcpOption<'a>],
}

#[cfg(test)]
mod tests {

    use smoltcp::wire::EthernetFrame;

    use crate::dcp::{DcpHeaderFrame, FrameID, ServiceID, ServiceType};

    #[test]
    fn test_parse_dcp_header() {
        let raw_packet: [u8; 64] = [
            0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5, 0x88, 0x92,
            0xfe, 0xfe, 0x05, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0xc0, 0x00, 0x04, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = EthernetFrame::new_checked(raw_packet);

        assert!(packet.is_ok());

        let mut packet = packet.unwrap();

        // println!("Src: {}", packet.src_addr());
        // println!("Dst: {}", packet.dst_addr());
        // println!("Type: {}", packet.ethertype());
        // println!("Payload: {:?}", packet.payload_mut());

        let payload = packet.payload_mut();
        let dcp_header = DcpHeaderFrame::new_checked(payload);

        assert!(dcp_header.is_some());

        let dcp_header = dcp_header.unwrap();

        assert_eq!(dcp_header.frame_id().unwrap(), FrameID::Request);
        assert_eq!(dcp_header.service_id().unwrap(), ServiceID::Identify);
        assert_eq!(dcp_header.service_type().unwrap(), ServiceType::Request);
        assert_eq!(dcp_header.x_id(), 5);
        assert_eq!(dcp_header.response_delay(), 192);
        assert_eq!(dcp_header.data_length(), 4);

        // println!("Frame ID: {:?}", dcp_header.frame_id().unwrap());
        // println!("Service ID: {:?}", dcp_header.service_id().unwrap());
        // println!("Service Type: {:?}", dcp_header.service_type().unwrap());
        // println!("X ID: {}", dcp_header.x_id());
        // println!("Response Delay: {}", dcp_header.response_delay());
        // println!("Data Length: {}", dcp_header.data_length());
    }
}
