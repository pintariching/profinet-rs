use smoltcp::wire::{EthernetAddress, Ipv4Address};

use profinet_rs_lib::{
    Block, DCPBlock, DCPFrame, DCPHeader, DevicePropertiesBlock, FrameID, NameOfStation, ServiceID,
    ServiceType, DCP, MAX_DEVICE_VENDOR_LENGTH, MAX_NAME_OF_STATION_LENGTH,
};

pub struct PNetConfig {
    mac_address: EthernetAddress,
    name_of_station: [char; MAX_NAME_OF_STATION_LENGTH],
    device_vendor: [char; MAX_DEVICE_VENDOR_LENGTH],
    last_update_timestamp: usize,
    ip_address: Ipv4Address,
}

pub struct PNet {
    config: PNetConfig,
}

impl PNet {
    pub fn init() {}

    pub fn handle_periodic(&mut self, packet: &[u8], current_timestamp: usize) -> Option<&[u8]> {
        let dcp_frame = DCPFrame::new_checked(packet);

        if dcp_frame.is_profinet_dcp() {
            // handle dcp

            if let Ok(dcp) = DCP::parse(&dcp_frame) {
                if dcp.is_hello() {
                    let response_dcp_header = DCPHeader::new(
                        FrameID::Reset,
                        ServiceID::Identify,
                        ServiceType::Request,
                        dcp.header.x_id,
                        dcp.header.response_delay,
                    );

                    let mut response_dcp =
                        DCP::new(dcp.source, self.config.mac_address, response_dcp_header);
                    response_dcp.add_block(DCPBlock {
                        block: Block::DeviceProperties(DevicePropertiesBlock::NameOfStation(
                            NameOfStation::from_str("kulsko ime naprave"),
                        )),
                        block_length: 0,
                    });
                }
            }
        }

        todo!()
    }
}
