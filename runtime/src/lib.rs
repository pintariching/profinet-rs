#![no_std]

use byteorder::{ByteOrder, NetworkEndian};
use smoltcp::wire::{EthernetAddress, Ipv4Address};

use profinet_rs_lib::{
    Block, Dcp, DcpBlock, DcpFrame, DcpHeader, DeviceId, DeviceInstance, DevicePropertiesBlock,
    DeviceRole, DeviceVendor, FrameID, IpBlock, IpParameter, NameOfStation, ServiceID, ServiceType,
};

pub struct PNetConfig<'a> {
    mac_address: EthernetAddress,
    name_of_station: &'a str,
    device_vendor: &'a str,
    last_update_timestamp: usize,
    ip_address: Ipv4Address,
}

impl<'a> PNetConfig<'a> {
    pub fn new(
        mac_address: EthernetAddress,
        name_of_station: &'a str,
        device_vendor: &'a str,
    ) -> Self {
        Self {
            mac_address,
            name_of_station,
            device_vendor,
            last_update_timestamp: 0,
            ip_address: Ipv4Address::new(0, 0, 0, 0),
        }
    }
}

pub struct PNet<'a> {
    config: PNetConfig<'a>,
}

pub struct OutgoingPacket {
    pub data: [u8; 255],
    pub length: usize,
    pub send_at: usize,
}

impl<'a> PNet<'a> {
    pub fn new(config: PNetConfig<'a>) -> Self {
        Self { config }
    }

    pub fn init(&mut self) {}

    pub fn handle_periodic(
        &mut self,
        packet_in: &[u8],
        current_timestamp: usize,
    ) -> Option<OutgoingPacket> {
        let dcp_frame = DcpFrame::new_checked(packet_in);

        if dcp_frame.is_profinet_dcp() {
            // handle dcp
            defmt::info!("Recieved DCP frame");

            if let Ok(dcp) = Dcp::parse(&dcp_frame) {
                defmt::info!("Parsing DCP frame is successfull");
                defmt::info!(
                    "Source MAC: {}, Destination MAC: {}",
                    dcp.source,
                    dcp.destination
                );

                if dcp.is_hello() {
                    defmt::info!("DCP frame is hello");

                    let response_dcp_header = DcpHeader::new(
                        FrameID::Reset,
                        ServiceID::Identify,
                        ServiceType::Success,
                        dcp.header.x_id,
                        0,
                    );

                    let mut response_dcp =
                        Dcp::new(dcp.source, self.config.mac_address, response_dcp_header);

                    response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
                        DevicePropertiesBlock::DeviceOptions,
                    )));

                    response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
                        DevicePropertiesBlock::NameOfStation(NameOfStation::from_str(
                            &self.config.name_of_station,
                        )),
                    )));

                    response_dcp.add_block(DcpBlock::new(Block::DeviceProperties(
                        DevicePropertiesBlock::DeviceVendor(DeviceVendor::from_str(
                            &self.config.device_vendor,
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
                            high: 0,
                            low: 0x69,
                        }),
                    )));

                    response_dcp.add_block(DcpBlock::new(Block::Ip(IpBlock::IpParameter(
                        IpParameter {
                            ip_address: Ipv4Address::new(0, 0, 0, 0),
                            subnet_mask: Ipv4Address::new(255, 255, 255, 0),
                            gateway: Ipv4Address::new(0, 0, 0, 0),
                        },
                    ))));

                    let mut response_buffer = [0; 255];
                    response_dcp.encode_into(&mut response_buffer);

                    let delay_factor = NetworkEndian::read_u16(&self.config.mac_address.0[4..6]);
                    let response_delay = dcp.header.response_delay % delay_factor;

                    return Some(OutgoingPacket {
                        data: response_buffer,
                        length: response_dcp.length(),
                        send_at: current_timestamp + response_delay as usize,
                    });
                }
            }
        }

        None
    }
}
