use byteorder::{ByteOrder, NetworkEndian};
use num_enum::{FromPrimitive, IntoPrimitive};
use smoltcp::wire::EthernetAddress;

use crate::ethernet::EthernetFrame;
use crate::field::{Field, Rest};
use crate::PNet;

mod block;
mod block_options;
mod error;
mod header;
mod parser;

pub use block::*;
pub use block_options::*;
pub use error::ParseDcpError;
pub use header::*;

pub const DCP_MAC_HELLO_ADDRESS: [u8; 6] = [0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00];
pub const MAX_DCP_BLOCK_NUMBER: usize = 32;

const DESTINATION_FIELD: Field = 0..6;
const SOURCE_FIELD: Field = 6..12;
const TYPE_FIELD: Field = 12..14;
// const VLAN: Field = 12..14;
const PAYLOAD_FIELD: Rest = 14..;
const TYPE_VLAN_FIELD: Field = 16..18;
const PAYLOAD_VLAN_FIELD: Rest = 18..;

#[derive(Debug, PartialEq, Clone, FromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum EthType {
    VLAN = 0x8100,
    Profinet = 0x8892,
    #[num_enum(default)]
    Other,
}

pub struct DcpFrame<T: AsRef<[u8]>> {
    buffer: T,
    is_vlan: bool,
}

impl<T: AsRef<[u8]>> DcpFrame<T> {
    pub fn new_unchecked(buffer: T) -> Self {
        Self {
            buffer,
            is_vlan: false,
        }
    }

    pub fn new_checked(buffer: T) -> Self {
        let mut frame = Self::new_unchecked(buffer);
        frame.is_vlan = frame.is_vlan();

        frame
    }

    pub fn is_vlan(&self) -> bool {
        match self.eth_type() {
            EthType::VLAN => true,
            EthType::Profinet => false,
            EthType::Other => false,
        }
    }

    pub fn destination(&self) -> EthernetAddress {
        let data = self.buffer.as_ref();
        EthernetAddress::from_bytes(&data[DESTINATION_FIELD])
    }

    pub fn source(&self) -> EthernetAddress {
        let data = self.buffer.as_ref();
        EthernetAddress::from_bytes(&data[SOURCE_FIELD])
    }

    pub fn eth_type(&self) -> EthType {
        let data = self.buffer.as_ref();

        let raw = if self.is_vlan {
            NetworkEndian::read_u16(&data[TYPE_VLAN_FIELD])
        } else {
            NetworkEndian::read_u16(&data[TYPE_FIELD])
        };

        EthType::from_primitive(raw)
    }

    pub fn is_profinet_dcp(&self) -> bool {
        self.eth_type() == EthType::Profinet
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();

        if self.is_vlan {
            &data[PAYLOAD_VLAN_FIELD]
        } else {
            &data[PAYLOAD_FIELD]
        }
    }
}

pub struct Dcp {
    pub destination: EthernetAddress,
    pub source: EthernetAddress,
    pub eth_type: EthType,
    pub header: DcpHeader,
    pub number_of_blocks: usize,
    pub blocks: [Option<DcpBlock>; MAX_DCP_BLOCK_NUMBER],
}

impl Dcp {
    pub fn new(destination: EthernetAddress, source: EthernetAddress, header: DcpHeader) -> Self {
        Self {
            destination,
            source,
            eth_type: EthType::Profinet,
            header,
            number_of_blocks: 0,
            blocks: [None; MAX_DCP_BLOCK_NUMBER],
        }
    }

    pub fn handle_frame<T: AsRef<[u8]>>(pnet: &mut PNet, frame: EthernetFrame<T>) {}

    pub fn add_block(&mut self, block: DcpBlock) -> &mut Self {
        self.blocks[self.number_of_blocks] = Some(block);
        self.number_of_blocks += 1;
        self.header.data_length += block.block_length;

        self
    }

    // TODO: rewrite parser with nom
    pub fn parse<T: AsRef<[u8]>>(frame: &DcpFrame<T>) -> Result<Self, ParseDcpError<T>> {
        let header_frame = DcpHeaderFrame::new_checked(frame.payload())
            .map_err(|e| ParseDcpError::HeaderError(e))?;

        let header = DcpHeader::parse(&header_frame).map_err(|e| ParseDcpError::HeaderError(e))?;

        let payload = header_frame.payload();

        const ARRAY_REPEAT_VALUE: Option<DcpBlock> = None;
        let mut blocks = [ARRAY_REPEAT_VALUE; MAX_DCP_BLOCK_NUMBER];
        let mut block_start_index = 0usize;
        let mut block_end_index;
        let mut block_number = 0;
        let mut odd_block_length;

        while block_start_index < header.data_length as usize {
            let block_frame = DCPBlockFrame::new_unchecked(&payload[block_start_index..]);
            let block_length = (block_frame.block_length() + 4) as usize; // option + suboption + block length = 4 bytes
            block_end_index = block_start_index + block_length;

            // Check if block_length is odd
            odd_block_length = block_length & 1 != 0;

            let dcp_block = DcpBlock::parse_block(&payload[block_start_index..block_end_index])
                .map_err(|e| ParseDcpError::BlockError(e))?;

            blocks[block_number] = Some(dcp_block);
            block_number += 1;
            block_start_index += block_length;

            if odd_block_length {
                block_start_index += 1;
            }
        }

        Ok(Self {
            destination: frame.destination(),
            source: frame.source(),
            eth_type: frame.eth_type(),
            header: header,
            number_of_blocks: block_number + 1,
            blocks: blocks,
        })
    }
    pub fn is_hello(&self) -> bool {
        self.destination.0 == DCP_MAC_HELLO_ADDRESS
    }

    pub fn encode_into(&self, buffer: &mut [u8]) {
        buffer[DESTINATION_FIELD].copy_from_slice(self.destination.as_bytes());
        buffer[SOURCE_FIELD].copy_from_slice(self.source.as_bytes());
        NetworkEndian::write_u16(&mut buffer[TYPE_FIELD], self.eth_type.clone().into());

        self.header.encode_into(&mut buffer[PAYLOAD_FIELD]);

        let mut current_block_index = 0;
        let block_start = PAYLOAD_FIELD.start + header::DCP_HEADER_LENGTH_FIELD;
        for block_opt in self.blocks {
            if let Some(block) = block_opt {
                block.encode_into(&mut buffer[block_start + current_block_index..]);
                current_block_index += block.block_length as usize;
            }
        }
    }

    pub fn length(&self) -> usize {
        self.blocks.iter().fold(26usize, |mut acc, block| {
            if let Some(b) = block {
                acc += b.block_length as usize;
            }

            acc
        })
    }
}

#[cfg(test)]
mod tests {
    use smoltcp::wire::Ipv4Address;
    use tests::{
        block::{Block, DevicePropertiesBlock, IpBlock, IpParameter, NameOfStation},
        header::ServiceId,
    };

    use super::*;

    #[test]
    fn test_non_vlan() {
        let raw_packet: [u8; 64] = [
            0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5, 0x88, 0x92,
            0xfe, 0xfe, 0x05, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0xc0, 0x00, 0x04, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let frame = DcpFrame::new_checked(raw_packet);

        assert_eq!(frame.is_vlan, false);
        assert_eq!(
            frame.destination(),
            EthernetAddress::from_bytes(&[0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00])
        );
        assert_eq!(
            frame.source(),
            EthernetAddress::from_bytes(&[0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5])
        );
        assert_eq!(frame.eth_type(), EthType::Profinet);
    }

    #[test]
    fn test_vlan() {
        let raw_packet = [
            0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0xa8, 0x5e, 0x45, 0x15, 0x85, 0x46, 0x81, 0x00,
            0x00, 0x00, 0x88, 0x92, 0xfe, 0xfe, 0x05, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x04, 0xff, 0xff, 0x00, 0x00,
        ];

        let frame = DcpFrame::new_checked(raw_packet);

        assert_eq!(frame.is_vlan, true);
        assert_eq!(
            frame.destination(),
            EthernetAddress::from_bytes(&[0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00])
        );
        assert_eq!(
            frame.source(),
            EthernetAddress::from_bytes(&[0xa8, 0x5e, 0x45, 0x15, 0x85, 0x46])
        );
        assert_eq!(frame.eth_type(), EthType::Profinet);
    }

    #[test]
    fn test_dcp_hello() {
        let raw_packet: [u8; 64] = [
            0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5, 0x88, 0x92,
            0xfe, 0xfe, 0x05, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0xc0, 0x00, 0x04, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let frame = DcpFrame::new_checked(raw_packet);
        let dcp = Dcp::parse(&frame);

        assert!(dcp.is_ok());
        let dcp = dcp.unwrap();

        assert_eq!(
            dcp.destination,
            EthernetAddress::from_bytes(&[0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00])
        );

        assert_eq!(
            dcp.source,
            EthernetAddress::from_bytes(&[0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5])
        );

        assert_eq!(dcp.eth_type, EthType::Profinet);
        assert_eq!(dcp.header.service_id, ServiceId::Identify);

        let block = dcp.blocks[0].clone().unwrap();

        assert_eq!(block.block, Block::All);
    }

    #[test]
    fn test_dcp_response() {
        let raw_packet: [u8; 112] = [
            0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5, 0x8c, 0xf3, 0x19, 0x45, 0x01, 0x63, 0x81, 0x00,
            0x00, 0x00, 0x88, 0x92, 0xfe, 0xff, 0x05, 0x01, 0x00, 0x00, 0x01, 0x66, 0x00, 0x00,
            0x00, 0x52, 0x02, 0x05, 0x00, 0x04, 0x00, 0x00, 0x02, 0x07, 0x02, 0x01, 0x00, 0x09,
            0x00, 0x00, 0x53, 0x37, 0x2d, 0x31, 0x32, 0x30, 0x30, 0x00, 0x02, 0x02, 0x00, 0x0c,
            0x00, 0x00, 0x70, 0x6c, 0x63, 0x78, 0x62, 0x31, 0x64, 0x30, 0x65, 0x64, 0x02, 0x03,
            0x00, 0x06, 0x00, 0x00, 0x00, 0x2a, 0x01, 0x0d, 0x02, 0x04, 0x00, 0x04, 0x00, 0x00,
            0x02, 0x00, 0x02, 0x07, 0x00, 0x04, 0x00, 0x00, 0x00, 0x64, 0x01, 0x02, 0x00, 0x0e,
            0x00, 0x01, 0xc0, 0xa8, 0x00, 0x01, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let frame = DcpFrame::new_checked(raw_packet);
        let dcp = Dcp::parse(&frame);

        assert!(dcp.is_ok());
        let dcp = dcp.unwrap();

        let name_of_station = dcp.blocks[2].clone().unwrap().block;

        assert_eq!(
            name_of_station,
            Block::DeviceProperties(DevicePropertiesBlock::NameOfStation(
                NameOfStation::from_str("plcxb1d0ed")
            ))
        );

        let ip = dcp.blocks[6].clone().unwrap().block;

        assert_eq!(
            ip,
            Block::Ip(IpBlock::IpParameter(IpParameter {
                ip_address: Ipv4Address::new(192, 168, 0, 1),
                subnet_mask: Ipv4Address::new(255, 255, 255, 0),
                gateway: Ipv4Address::new(0, 0, 0, 0)
            }))
        )
    }

    #[test]
    fn test_dcp_encoding() {
        let mut dcp = Dcp::new(
            EthernetAddress::from_bytes(&[0, 0, 0, 0, 0, 0]),
            EthernetAddress::from_bytes(&[1, 1, 1, 1, 1, 1]),
            DcpHeader::new(
                FrameID::Hello,
                ServiceId::Identify,
                ServiceType::Success,
                1,
                0,
            ),
        );

        dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceOptions,
        )));

        dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::NameOfStation(NameOfStation::from_str("my cool device")),
        )));

        let mut buffer = [0; 255];
        dcp.encode_into(&mut buffer);
        println!("{:x?}", buffer);
    }
}
