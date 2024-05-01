#![cfg_attr(not(test), no_std)]

use error::Error;
use ethernet::{eth_dma::EthernetDMA, EthernetFrame, FrameId};
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

pub struct PNet<'a, T: EthernetDMA> {
    config: PNetConfig<'a>,
    outgoing_packets: [Option<OutgoingPacket>; 8],
    dma: T,
}

#[derive(Clone, Copy)]
pub struct OutgoingPacket {
    pub data: [u8; 255],
    pub length: usize,
    pub send_at: usize,
}

impl<'a, T: EthernetDMA> PNet<'a, T> {
    pub fn new(config: PNetConfig<'a>, dma: T) -> Self {
        Self {
            config,
            outgoing_packets: [None; 8],
            dma,
        }
    }

    pub fn init(&mut self) {}

    pub fn handle_periodic(&mut self, current_timestamp: usize) {
        if let Ok(packet) = self.dma.recv_next(None) {
            defmt::debug!("Recieved packet on DMA");

            match self.handle_incoming_packet(&packet, current_timestamp) {
                Ok(_) => (),
                Err(e) => defmt::debug!("Failed to handle incomming packet: {}", e),
            }
        }

        self.send_queued_packets(current_timestamp);
    }

    pub fn handle_incoming_packet(
        &mut self,
        packet: &[u8],
        current_timestamp: usize,
    ) -> Result<(), Error> {
        let frame_in = EthernetFrame::new_checked(packet).map_err(|e| Error::EthernetError(e))?;

        if !frame_in.is_profinet() {
            defmt::debug!("Packet is not Profinet");
            return Ok(());
        }

        let frame_id = frame_in.frame_id();

        match frame_id {
            FrameId::Dcp => {
                defmt::debug!("Packet Frame ID is DCP");
                Dcp::handle_frame(self, frame_in, current_timestamp);
            }
            FrameId::Other => defmt::debug!("Packet Fr  ame ID is not DCP"),
        }

        Ok(())
    }

    pub fn queue_packet(&mut self, data: [u8; 255], send_at: usize) {
        let packet_out = OutgoingPacket {
            data,
            length: data.len(),
            send_at,
        };

        for i in 0..self.outgoing_packets.len() {
            if let None = self.outgoing_packets[i] {
                self.outgoing_packets[i] = Some(packet_out);
            }
        }
    }

    pub fn send_queued_packets(&mut self, current_timestamp: usize) {
        for i in 0..self.outgoing_packets.len() {
            if let Some(p) = self.outgoing_packets[i] {
                if current_timestamp >= p.send_at {
                    match self
                        .dma
                        .send(p.length, None, |buf| buf.copy_from_slice(&p.data))
                    {
                        Ok(_) => {
                            defmt::debug!("Successfully sent out packet");
                            self.outgoing_packets[i] = None
                        }
                        Err(e) => defmt::error!("Failed sending packet: {}", e),
                    }
                }
            }
        }
    }
}
