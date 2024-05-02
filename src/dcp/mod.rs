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
    Response = 0xfeff,
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
        let response_dcp_header = DcpHeader::new(
            ServiceId::Identify,
            ServiceType::Success,
            self.header.x_id,
            1,
        );
        let mut response_dcp = Dcp::new(
            self.source,
            pnet.config.ip_config.mac_address,
            response_dcp_header,
            DcpFrameId::Response,
        );

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceOptions,
        )));

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::NameOfStation(NameOfStation::new(
                pnet.config.name_of_station,
                pnet.config.name_of_station_len,
            )),
        )));

        response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
            DevicePropertiesBlock::DeviceVendor(DeviceVendor::new(
                pnet.config.device_vendor,
                pnet.config.device_vendor_len,
            )),
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
                ip_address: pnet.config.ip_config.ip_address,
                subnet_mask: pnet.config.ip_config.subnet_mask,
                gateway: pnet.config.ip_config.gateway,
                block_info: IpParameterBlockInfo::IpNotSet,
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

        defmt::debug!("Successfully parsed frame to DCP packet");

        match request_dcp.frame_id {
            DcpFrameId::Request => {
                if request_dcp.dst_is_hello() && request_dcp.number_of_blocks > 0 {
                    let Some(hello_block) = request_dcp.blocks[0] else {
                        defmt::debug!("DCP packet does not contain a Hello block");
                        return;
                    };
                    if hello_block.block == Block::All {
                        defmt::debug!("Recieved Hello DCP request, creating response");
                        let response_dcp = request_dcp.new_hello_response(pnet);
                        let mut response_buffer = [0; 255];
                        response_dcp.encode_into(&mut response_buffer);

                        let response_delay_time = request_dcp.response_delay_time();

                        defmt::debug!("Adding response DCP request to outgoing buffer");
                        pnet.queue_packet(response_buffer, current_timestamp + response_delay_time)
                    }
                }
            }
            DcpFrameId::GetSet => {
                for block in request_dcp.blocks {
                    if let Some(block) = block {
                        match block.block {
                            Block::DeviceProperties(dp) => match dp {
                                DevicePropertiesBlock::NameOfStation(ns) => {
                                    pnet.config.name_of_station = ns.name;
                                    pnet.config.name_of_station_len = ns.length;
                                }
                                _ => (),
                            },
                            Block::Ip(ip) => match ip {
                                IpBlock::IpParameter(ip) => {
                                    pnet.config.ip_config.ip_address = ip.ip_address;
                                    pnet.config.ip_config.subnet_mask = ip.subnet_mask;
                                    pnet.config.ip_config.gateway = ip.gateway;
                                    pnet.update_interface();
                                }
                                IpBlock::FullIpSuite(suite) => {
                                    pnet.config.ip_config.ip_address = suite.ip_address;
                                    pnet.config.ip_config.subnet_mask = suite.subnet_mask;
                                    pnet.config.ip_config.gateway = suite.gateway;
                                    pnet.update_interface();
                                }
                                _ => (),
                            },
                            _ => (),
                        }
                    }
                }
            }
            _ => {
                defmt::debug!("Recieved DCP packet is not a Hello packet");
                defmt::debug!(
                    "dst_is_hello = {}, num_of_blocks: {}, frame_id: {:x}",
                    request_dcp.dst_is_hello(),
                    request_dcp.number_of_blocks,
                    request_dcp.frame_id as u16
                );
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

        while block_start_index < header.data_length as usize {
            let block_frame = DCPBlockFrame::new_unchecked(&payload[block_start_index..]);
            let block_length = (block_frame.block_length() + 4) as usize; // option + suboption + block length = 4 bytes
            block_end_index = block_start_index + block_length;

            let dcp_block =
                DcpBlock::parse_block(&payload[block_start_index..block_end_index]).ok();

            blocks[block_number] = dcp_block;
            block_number += 1;
            block_start_index += block_length;

            // Check if block_length is odd
            if block_length % 2 != 0 {
                block_start_index += 1;
            }
        }

        Ok(Self {
            destination: frame.dst_address(),
            source: frame.src_address(),
            eth_type: frame.eth_type(),
            frame_id,
            header: header,
            number_of_blocks: block_number,
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
    use smoltcp::wire::Ipv4Address;
    use tests::{
        block::{Block, DevicePropertiesBlock, IpBlock, IpParameter, NameOfStation},
        header::ServiceId,
    };

    use crate::{Config, IpConfig};

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
        assert_eq!(dcp.number_of_blocks, 1);

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

        if let Err(e) = &dcp {
            println!("{:#?}", e);
        }

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
                gateway: Ipv4Address::new(0, 0, 0, 0),
                block_info: IpParameterBlockInfo::IpSetViaSetRequest
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

        let mut buffer = [0; 128];
        dcp.encode_into(&mut buffer);
        // let hexdump = print_hexdump(&buffer);
        // let mut file = File::create("hexdump").unwrap();
        // let _ = file.write_all(hexdump.as_bytes());
        // println!("{:x?}", buffer);

        assert_eq!(
            buffer,
            [
                1, 14, 207, 0, 0, 0, 0, 0, 35, 83, 78, 254, 136, 146, 254, 252, 5, 1, 0, 0, 0, 1,
                0, 0, 0, 28, 2, 5, 0, 4, 0, 0, 2, 7, 2, 2, 0, 16, 0, 0, 109, 121, 32, 99, 111, 111,
                108, 32, 100, 101, 118, 105, 99, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0
            ]
        );
    }

    #[test]
    fn test_hello_response() {
        let ip_config = IpConfig::new_not_set(EthernetAddress::from_bytes(&[
            0x00, 0x00, 0x23, 0x53, 0x4e, 0xfe,
        ]));

        let config = Config::new("test", "asd", ip_config);
        let pnet = PNet::new(config);

        let dcp_hello = Dcp::new(
            EthernetAddress::from_bytes(&DCP_MAC_HELLO_ADDRESS),
            EthernetAddress::from_bytes(&[0x02, 0x12, 0x23, 0x53, 0x4e, 0xfa]),
            DcpHeader::new(ServiceId::Identify, ServiceType::Success, 1, 0),
            DcpFrameId::Hello,
        );

        let dcp_response = dcp_hello.new_hello_response(&pnet);

        assert_eq!(
            dcp_response.destination,
            EthernetAddress::from_bytes(&[0x02, 0x12, 0x23, 0x53, 0x4e, 0xfa])
        );
        assert_eq!(
            dcp_response.source,
            EthernetAddress::from_bytes(&[0x00, 0x00, 0x23, 0x53, 0x4e, 0xfe])
        );

        assert_eq!(dcp_response.eth_type, EthType::Profinet);
        assert_eq!(dcp_response.frame_id, DcpFrameId::Response);
        assert_eq!(dcp_response.header.service_id, ServiceId::Identify);
        assert_eq!(dcp_response.header.service_type, ServiceType::Success);

        dcp_response
            .blocks
            .iter()
            .filter_map(|b| *b)
            .for_each(|b| match b.block {
                Block::Ip(ip) => match ip {
                    IpBlock::IpParameter(ip) => {
                        assert_eq!(ip.block_info, IpParameterBlockInfo::IpNotSet);
                        assert_eq!(ip.ip_address.0, [0, 0, 0, 0]);
                        assert_eq!(ip.subnet_mask.0, [0, 0, 0, 0]);
                        assert_eq!(ip.gateway.0, [0, 0, 0, 0])
                    }
                    _ => panic!("Response shouldn't contain anything but 'IpParameter' block"),
                },
                Block::All => panic!("Response shouldn't contain an 'ALL' block"),
                _ => (),
            })
    }
}
