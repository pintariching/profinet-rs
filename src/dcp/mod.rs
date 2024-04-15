use byteorder::{ByteOrder, NetworkEndian};
use num_enum::FromPrimitive;
use smoltcp::wire::EthernetAddress;

use crate::field::{Field, Rest};

use self::{
    block::{DCPBlock, DCPBlockFrame},
    error::ParseDCPError,
    header::{DCPHeader, DCPHeaderFrame},
};

mod block;
mod block_options;
mod error;
mod header;

pub const DCP_MAC_HELLO_ADDRESS: [u8; 6] = [0x01, 0x0e, 0xcf, 0x00, 0x00, 0x01];
pub const MAX_DCP_BLOCK_NUMBER: usize = 32;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(FromPrimitive)]
#[repr(u16)]
pub enum EthType {
    VLAN = 0x8100,
    Profinet = 0x8892,
    #[num_enum(default)]
    Other,
}

pub struct DCPFrame<T: AsRef<[u8]>> {
    buffer: T,
    is_vlan: bool,
}

impl<T: AsRef<[u8]>> DCPFrame<T> {
    const DESTINATION: Field = 0..6;
    const SOURCE: Field = 6..12;
    const TYPE: Field = 12..14;
    const VLAN: Field = 12..14;
    const PAYLOAD: Rest = 14..;
    const TYPE_VLAN: Field = 16..18;
    const PAYLOAD_VLAN: Rest = 18..;

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
        EthernetAddress::from_bytes(&data[Self::DESTINATION])
    }

    pub fn source(&self) -> EthernetAddress {
        let data = self.buffer.as_ref();
        EthernetAddress::from_bytes(&data[Self::SOURCE])
    }

    pub fn eth_type(&self) -> EthType {
        let data = self.buffer.as_ref();

        let raw = if self.is_vlan {
            NetworkEndian::read_u16(&data[Self::TYPE_VLAN])
        } else {
            NetworkEndian::read_u16(&data[Self::TYPE])
        };

        EthType::from_primitive(raw)
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();

        if self.is_vlan {
            &data[Self::PAYLOAD_VLAN]
        } else {
            &data[Self::PAYLOAD]
        }
    }
}

pub struct DCP {
    pub destination: EthernetAddress,
    pub source: EthernetAddress,
    pub eth_type: EthType,
    pub header: DCPHeader,
    pub number_of_blocks: usize,
    pub blocks: [Option<DCPBlock>; MAX_DCP_BLOCK_NUMBER],
}

impl DCP {
    pub fn parse<T: AsRef<[u8]>>(frame: &DCPFrame<T>) -> Result<Self, ParseDCPError> {
        let header_frame = DCPHeaderFrame::new_checked(frame.payload())
            .map_err(|e| ParseDCPError::HeaderError(e))?;

        let header = DCPHeader::parse(&header_frame).map_err(|e| ParseDCPError::HeaderError(e))?;

        let payload = header_frame.payload();

        const ARRAY_REPEAT_VALUE: Option<DCPBlock> = None;
        let mut blocks = [ARRAY_REPEAT_VALUE; MAX_DCP_BLOCK_NUMBER];
        let mut block_start_index = 0usize;
        let mut block_number = 0;

        while block_start_index < header.data_length as usize {
            let block_frame = DCPBlockFrame::new_unchecked(payload);
            let block_length = (block_frame.block_length() + 4) as usize; // option + suboption + block length = 4 bytes
            let dcp_block = DCPBlock::parse_block(&payload[block_start_index..block_length])
                .map_err(|e| ParseDCPError::BlockError(e))?;

            blocks[block_number] = Some(dcp_block);
            block_number += 1;
            block_start_index += block_length;
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
}

#[cfg(test)]
mod tests {
    use tests::header::ServiceID;

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

        let frame = DCPFrame::new_checked(raw_packet);

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

        let frame = DCPFrame::new_checked(raw_packet);

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
    fn test_dcp() {
        let raw_packet: [u8; 64] = [
            0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5, 0x88, 0x92,
            0xfe, 0xfe, 0x05, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0xc0, 0x00, 0x04, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let frame = DCPFrame::new_checked(raw_packet);
        let dcp = DCP::parse(&frame);

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

        assert_eq!(dcp.header.service_id, ServiceID::Identify);
    }
}