use byteorder::{ByteOrder, NetworkEndian};
use num_enum::TryFromPrimitive;
use smoltcp::wire::EthernetAddress;

use crate::ethernet::{EthType, EthernetFrame};
use crate::field::{Field, Rest};
use crate::PNet;

mod block;
mod block_options;
mod error;
mod header;

pub use block::*;
pub use block_options::*;
pub use error::ParseDcpError;
pub use header::*;

pub const DCP_MAC_HELLO_ADDRESS: [u8; 6] = [0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00];
pub const MAX_DCP_BLOCK_NUMBER: usize = 32;

const DESTINATION_FIELD: Field = 0..6;
const SOURCE_FIELD: Field = 6..12;
const TYPE_FIELD: Field = 12..14;
const FRAME_ID_FIELD: Field = 14..16;
const PAYLOAD_FIELD: Rest = 16..;

#[derive(Debug, PartialEq, Clone, TryFromPrimitive)]
#[repr(u16)]
pub enum DcpFrameId {
    Hello = 0xfefc,
    GetSet = 0xfefd,
    Request = 0xfefe,
    Reset = 0xfeff,
}

pub struct Dcp {
    pub destination: EthernetAddress,
    pub source: EthernetAddress,
    pub eth_type: EthType,
    pub frame_id: DcpFrameId,
    pub header: DcpHeader,
    pub number_of_blocks: usize,
    pub blocks: [Option<DcpBlock>; MAX_DCP_BLOCK_NUMBER],
}

impl Dcp {
    pub fn new(
        destination: EthernetAddress,
        source: EthernetAddress,
        header: DcpHeader,
        frame_id: DcpFrameId,
    ) -> Self {
        Self {
            destination,
            source,
            eth_type: EthType::Profinet,
            frame_id,
            header,
            number_of_blocks: 0,
            blocks: [None; MAX_DCP_BLOCK_NUMBER],
        }
    }

    pub fn new_hello_response(&self, pnet: &PNet) -> Self {
        let response_dcp_header =
            DcpHeader::new(ServiceId::Hello, ServiceType::Success, self.header.x_id, 1);
        let mut response_dcp = Dcp::new(
            self.source,
            pnet.config.mac_address,
            response_dcp_header,
            DcpFrameId::Hello,
        );

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceOptions,
        )));

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::NameOfStation(NameOfStation::from_str(
                pnet.config.name_of_station,
            )),
        )));

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceVendor(DeviceVendor::from_str(pnet.config.device_vendor)),
        )));

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceRole(DeviceRole::IODevice),
        )));

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceId(DeviceId {
                vendor_id: 0x1337,
                device_id: 0x6969,
            }),
        )));

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceInstance(DeviceInstance {
                high: 0x42,
                low: 0x69,
            }),
        )));

        response_dcp.add_block(DcpBlock::new(Block::Ip(IpBlock::IpParameter(
            IpParameter {
                ip_address: pnet.config.ip_address,
                subnet_mask: pnet.config.subnet_mask,
                gateway: pnet.config.gateway,
            },
        ))));

        response_dcp
    }

    pub fn handle_frame<T: AsRef<[u8]>>(
        pnet: &mut PNet,
        frame: EthernetFrame<T>,
        current_timestamp: usize,
    ) {
        let Ok(request_dcp) = Dcp::parse(&frame) else {
            defmt::debug!("Failed to parse DCP packet");
            return;
        };

        if request_dcp.dst_is_hello()
            && request_dcp.number_of_blocks > 0
            && request_dcp.frame_id == DcpFrameId::Hello
        {
            let Some(hello_block) = request_dcp.blocks[0] else {
                defmt::debug!("DCP packet does not contain a Hello block");
                return;
            };
            if hello_block.block == Block::All {
                // Recieved a hello request, create a response
                let response_dcp = request_dcp.new_hello_response(pnet);
                let mut response_buffer = [0; 255];
                response_dcp.encode_into(&mut response_buffer);

                let response_delay_time = request_dcp.response_delay_time();
                pnet.send_packet(response_buffer, current_timestamp + response_delay_time)
            }
        }
    }

    pub fn add_block(&mut self, block: DcpBlock) -> &mut Self {
        self.blocks[self.number_of_blocks] = Some(block);
        self.number_of_blocks += 1;
        self.header.data_length += block.block_length;

        self
    }

    pub fn parse<T: AsRef<[u8]>>(frame: &EthernetFrame<T>) -> Result<Self, ParseDcpError> {
        let frame_id = DcpFrameId::try_from_primitive(frame.frame_id_u16())
            .map_err(|_| ParseDcpError::FrameIdError)?;

        let header_frame = DcpHeaderFrame::new_checked(frame.payload())
            .map_err(|e| ParseDcpError::HeaderError(e))?;

        let header = DcpHeader::parse(&header_frame).map_err(|e| ParseDcpError::HeaderError(e))?;

        let payload = header_frame.payload();

        let mut blocks = [None; MAX_DCP_BLOCK_NUMBER];
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
            destination: frame.dst_address(),
            source: frame.src_address(),
            eth_type: frame.eth_type(),
            frame_id,
            header: header,
            number_of_blocks: block_number + 1,
            blocks: blocks,
        })
    }
    pub fn dst_is_hello(&self) -> bool {
        self.destination.0 == DCP_MAC_HELLO_ADDRESS
    }

    pub fn encode_into(&self, buffer: &mut [u8]) {
        buffer[DESTINATION_FIELD].copy_from_slice(self.destination.as_bytes());
        buffer[SOURCE_FIELD].copy_from_slice(self.source.as_bytes());
        NetworkEndian::write_u16(&mut buffer[TYPE_FIELD], self.eth_type.clone() as u16);
        NetworkEndian::write_u16(&mut buffer[FRAME_ID_FIELD], self.frame_id.clone() as u16);
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

    pub fn response_delay_time(&self) -> usize {
        match self.header.response_delay_factor {
            0..=1 => 400,
            2.. => {
                let delay = 1 + self.header.response_delay_factor as usize * 10;
                let rem = delay % 1000;
                delay + (1000 - rem)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write};

    use smoltcp::wire::Ipv4Address;
    use tests::{
        block::{Block, DevicePropertiesBlock, IpBlock, IpParameter, NameOfStation},
        header::ServiceId,
    };

    use crate::{util::print_hexdump, PNetConfig};

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

        let frame = EthernetFrame::new_checked(raw_packet).unwrap();

        assert_eq!(
            frame.dst_address(),
            EthernetAddress::from_bytes(&[0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00])
        );
        assert_eq!(
            frame.src_address(),
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

        let frame = EthernetFrame::new_checked(raw_packet).unwrap();

        assert_eq!(
            frame.dst_address(),
            EthernetAddress::from_bytes(&[0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00])
        );
        assert_eq!(
            frame.src_address(),
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

        let frame = EthernetFrame::new_checked(raw_packet).unwrap();
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

        let frame = EthernetFrame::new_checked(raw_packet).unwrap();
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
            EthernetAddress::from_bytes(&[0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00]),
            EthernetAddress::from_bytes(&[0x00, 0x00, 0x23, 0x53, 0x4e, 0xfe]),
            DcpHeader::new(ServiceId::Identify, ServiceType::Success, 1, 0),
            DcpFrameId::Hello,
        );

        dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceOptions,
        )));

        dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::NameOfStation(NameOfStation::from_str("my cool device")),
        )));

        let mut buffer = [0; 255];
        dcp.encode_into(&mut buffer);
        let hexdump = print_hexdump(&buffer);
        let mut file = File::create("hexdump").unwrap();
        let _ = file.write_all(hexdump.as_bytes());
        // println!("{:x?}", buffer);
    }

    #[test]
    fn test_hello_response() {
        let config = PNetConfig::new(
            EthernetAddress::from_bytes(&[0x00, 0x00, 0x23, 0x53, 0x4e, 0xfe]),
            "test",
            "asd",
        );
        let pnet = PNet::new(config);

        let dcp_hello = Dcp::new(
            EthernetAddress::from_bytes(&[0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00]),
            EthernetAddress::from_bytes(&[0x00, 0x00, 0x23, 0x53, 0x4e, 0xfe]),
            DcpHeader::new(ServiceId::Identify, ServiceType::Success, 1, 0),
            DcpFrameId::Hello,
        );

        let dcp_response = dcp_hello.new_hello_response(&pnet);

        for block in dcp_response.blocks {
            if let Some(block) = block {
                match block.block {
                    Block::DeviceProperties(dp) => match dp {
                        DevicePropertiesBlock::DeviceVendor(dv) => println!("{:?}", dv.vendor),
                        DevicePropertiesBlock::NameOfStation(ns) => println!("{:?}", ns.name),
                        _ => (),
                    },
                    _ => (),
                }
            }
        }

        let mut buffer = [0; 255];
        dcp_response.encode_into(&mut buffer);
        let hexdump = print_hexdump(&buffer);
        let mut file = File::create("hexdump").unwrap();
        let _ = file.write_all(hexdump.as_bytes());
    }
}
