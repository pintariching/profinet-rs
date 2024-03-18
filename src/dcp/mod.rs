use defmt::Format;

mod block;

pub use block::DcpOption;

pub static DCP_MAC_HELLO_ADDRESS: [u8; 6] = [0x01, 0x0e, 0xcf, 0x00, 0x00, 0x01];

#[derive(Format)]
pub enum ServiceType {
    Request = 0,
    Success = 1,
    NotSupported = 5,
}

impl ServiceType {
    pub fn parse(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Request),
            1 => Some(Self::Success),
            5 => Some(Self::NotSupported),
            _ => None,
        }
    }
}

#[derive(Format)]
pub enum ServiceID {
    Get = 3,
    Set = 4,
    Identify = 5,
    Hello = 6,
}

impl ServiceID {
    pub fn parse(byte: u8) -> Option<Self> {
        match byte {
            3 => Some(Self::Get),
            4 => Some(Self::Set),
            5 => Some(Self::Identify),
            6 => Some(Self::Hello),
            _ => None,
        }
    }
}

pub struct DcpHeader {
    pub service_id: ServiceID,
    pub service_type: ServiceType,
    pub x_id: u32,
    pub response_delay: u16,
    pub data_length: u32,
}

pub struct DcpBlockHeader {
    pub option: u8,
    pub suboption: u8,
    pub block_length: u16,
}

pub struct DcpBlock<'a> {
    pub header: DcpBlockHeader,
    pub options: [DcpOption<'a>],
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_dcp_packet() {
        let raw_packet: [u8; 64] = [
            0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5, 0x88, 0x92,
            0xfe, 0xfe, 0x05, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0xc0, 0x00, 0x04, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
    }
}
