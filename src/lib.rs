#![cfg_attr(not(test), no_std)]

use error::Error;
use ethernet::{EthernetFrame, FrameId};
use smoltcp::wire::{EthernetAddress, Ipv4Address};

mod dcp;
mod error;
mod ethernet;
mod util;

mod field {
    pub type SmallField = usize;
    pub type Field = ::core::ops::Range<usize>;
    pub type Rest = ::core::ops::RangeFrom<usize>;
}

pub use dcp::*;

pub struct PNetConfig<'a> {
    mac_address: EthernetAddress,
    ip_address: Ipv4Address,
    subnet_mask: Ipv4Address,
    gateway: Ipv4Address,
    name_of_station: &'a str,
    device_vendor: &'a str,
    last_update_timestamp: usize,
}

impl<'a> PNetConfig<'a> {
    pub fn new(
        mac_address: EthernetAddress,
        name_of_station: &'a str,
        device_vendor: &'a str,
    ) -> Self {
        Self {
            mac_address,
            ip_address: Ipv4Address::new(0, 0, 0, 0),
            subnet_mask: Ipv4Address::new(0, 0, 0, 0),
            gateway: Ipv4Address::new(0, 0, 0, 0),
            name_of_station,
            device_vendor,
            last_update_timestamp: 0,
        }
    }
}

pub struct PNet<'a> {
    config: PNetConfig<'a>,
    outgoing_packets_num: usize,
    outgoing_packets: [Option<OutgoingPacket>; 8],
}

#[derive(Clone, Copy)]
pub struct OutgoingPacket {
    pub data: [u8; 255],
    pub length: usize,
    pub send_at: usize,
}

impl<'a> PNet<'a> {
    pub fn new(config: PNetConfig<'a>) -> Self {
        Self {
            config,
            outgoing_packets_num: 0,
            outgoing_packets: [None; 8],
        }
    }

    pub fn init(&mut self) {}

    pub fn handle_periodic(
        &mut self,
        packet_in: &[u8],
        current_timestamp: usize,
    ) -> Option<OutgoingPacket> {
        None
    }

    pub fn handle_incoming_packet(
        &mut self,
        packet_in: &[u8],
        current_timestamp: usize,
    ) -> Result<(), Error> {
        let frame_in =
            EthernetFrame::new_checked(packet_in).map_err(|e| Error::EthernetError(e))?;

        if !frame_in.is_profinet() {
            return Ok(());
        }

        let frame_id = frame_in.frame_id();

        if frame_id == FrameId::Other {
            return Ok(());
        }

        match frame_id {
            FrameId::Dcp => Dcp::handle_frame(self, frame_in, current_timestamp),
            FrameId::Other => todo!(),
        }

        todo!()
    }

    pub fn send_packet(&mut self, data: [u8; 255], send_at: usize) {
        let packet_out = OutgoingPacket {
            data,
            length: data.len(),
            send_at,
        };

        self.outgoing_packets[self.outgoing_packets_num as usize] = Some(packet_out);
        self.outgoing_packets_num += 1;
    }
}
