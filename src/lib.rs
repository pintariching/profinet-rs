#![cfg_attr(not(test), no_std)]

use error::Error;
use ethernet::{setup::setup_pins, EthernetFrame, FrameId, Gpio};
use smoltcp::{
    iface::{Interface, SocketHandle, SocketSet, SocketStorage},
    socket::{
        tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer},
        udp::{self, PacketBuffer as UdpPacketBuffer, PacketMetadata, Socket as UdpSocket},
    },
    time::Instant,
    wire::{ArpPacket, EthernetAddress, Ipv4Address, Ipv4Cidr},
};
use stm32_eth::{
    dma::{EthernetDMA, RxRingEntry, TxRingEntry},
    mac::EthernetMAC,
    Parts, PartsIn,
};
use stm32f4xx_hal::rcc::Clocks;

mod dcp;
mod error;
pub mod ethernet;
mod util;

mod field {
    pub type SmallField = usize;
    pub type Field = ::core::ops::Range<usize>;
    pub type Rest = ::core::ops::RangeFrom<usize>;
}

pub use dcp::*;

pub struct Config {
    name_of_station: [u8; MAX_NAME_OF_STATION_LENGTH],
    name_of_station_len: usize,
    device_vendor: [u8; MAX_DEVICE_VENDOR_LENGTH],
    device_vendor_len: usize,
    interface: Option<Interface>,
    ip_config: IpConfig,
    last_update_timestamp: usize,
}

impl Config {
    pub fn new(name_of_station: &str, device_vendor: &str, ip_config: IpConfig) -> Self {
        let mut ns = [0; MAX_NAME_OF_STATION_LENGTH];
        let mut dv = [0; MAX_DEVICE_VENDOR_LENGTH];

        ns[0..name_of_station.len()].clone_from_slice(name_of_station.as_bytes());
        dv[0..device_vendor.len()].clone_from_slice(device_vendor.as_bytes());

        Self {
            name_of_station: ns,
            name_of_station_len: name_of_station.len(),
            device_vendor: dv,
            device_vendor_len: device_vendor.len(),
            interface: None,
            ip_config,
            last_update_timestamp: 0,
        }
    }
}

pub struct IpConfig {
    mac_address: EthernetAddress,
    ip_address: Ipv4Address,
    subnet_mask: Ipv4Address,
    gateway: Ipv4Address,
}

impl IpConfig {
    pub fn new(
        mac_address: EthernetAddress,
        ip_address: Ipv4Address,
        subnet_mask: Ipv4Address,
        gateway: Ipv4Address,
    ) -> Self {
        Self {
            mac_address,
            ip_address,
            subnet_mask,
            gateway,
        }
    }

    pub fn new_not_set(mac_address: EthernetAddress) -> Self {
        Self {
            mac_address,
            ip_address: Ipv4Address::UNSPECIFIED,
            subnet_mask: Ipv4Address::UNSPECIFIED,
            gateway: Ipv4Address::UNSPECIFIED,
        }
    }
}

pub struct PNet<'rx, 'tx> {
    config: Config,
    outgoing_packets: [Option<OutgoingPacket>; 8],
    ethernet_parts: Option<Parts<'rx, 'tx, EthernetMAC>>,
    tcp_handle: SocketHandle,
    udp_handle: SocketHandle,
}

#[derive(Clone, Copy)]
pub struct OutgoingPacket {
    pub data: [u8; 255],
    pub length: usize,
    pub send_at: usize,
}

impl<'rx, 'tx> PNet<'rx, 'tx> {
    pub fn new(config: Config) -> Self {
        let mut sockets = [SocketStorage::EMPTY];
        let mut sockets = SocketSet::new(&mut sockets[..]);

        let mut tcp_rx_buffer = [0; 1024];
        let mut tcp_tx_buffer = [0; 1024];
        let tcp_socket = TcpSocket::new(
            TcpSocketBuffer::new(&mut tcp_rx_buffer[..]),
            TcpSocketBuffer::new(&mut tcp_tx_buffer[..]),
        );

        let mut udp_rx_metadata_buf = [PacketMetadata::EMPTY; 4];
        let mut udp_rx_data_buf = [0; 1024];
        let mut udp_tx_metadata_buf = [PacketMetadata::EMPTY; 4];
        let mut udp_tx_data_buf = [0; 1024];

        let udp_rx_buffer = UdpPacketBuffer::new(
            udp_rx_metadata_buf.as_mut_slice(),
            udp_rx_data_buf.as_mut_slice(),
        );
        let upd_tx_buffer = UdpPacketBuffer::new(
            udp_tx_metadata_buf.as_mut_slice(),
            udp_tx_data_buf.as_mut_slice(),
        );
        let udp_socket = UdpSocket::new(udp_rx_buffer, upd_tx_buffer);

        let tcp_handle = sockets.add(tcp_socket);
        let udp_handle = sockets.add(udp_socket);

        Self {
            config,
            outgoing_packets: [None; 8],
            ethernet_parts: None,
            tcp_handle,
            udp_handle,
        }
    }

    pub fn init(
        &mut self,
        ethernet: PartsIn,
        clocks: Clocks,
        gpio: Gpio,
        rx_ring: &'rx mut [RxRingEntry; 2],
        tx_ring: &'tx mut [TxRingEntry; 2],
    ) {
        defmt::info!("Enabling ethernet...");

        let eth_pins = setup_pins(gpio);

        let mut parts = stm32_eth::new(
            ethernet,
            &mut rx_ring[..],
            &mut tx_ring[..],
            clocks,
            eth_pins,
        )
        .unwrap();
        parts.dma.enable_interrupt();

        let config = smoltcp::iface::Config::new(self.config.ip_config.mac_address.into());
        let iface = smoltcp::iface::Interface::new(config, &mut &mut parts.dma, Instant::ZERO);
        self.config.interface = Some(iface);

        self.update_interface();

        defmt::info!(
            "Enabled internet with IP and MAC: {}, {:x}",
            self.config.ip_config.ip_address,
            self.config.ip_config.mac_address
        );

        self.ethernet_parts = Some(parts);
    }

    pub fn dma(&mut self) -> &mut EthernetDMA<'rx, 'tx> {
        if let Some(parts) = &mut self.ethernet_parts {
            &mut parts.dma
        } else {
            panic!("PNet not initialised, ethernet_parts is None");
        }
    }

    pub fn update_interface(&mut self) {
        if let Some(iface) = &mut self.config.interface {
            iface.update_ip_addrs(|addr| {
                addr.clear();
                addr.push(smoltcp::wire::IpCidr::Ipv4(Ipv4Cidr::new(
                    self.config.ip_config.ip_address,
                    24,
                )))
                .ok();
            });

            defmt::info!(
                "Update ethernet interface with IP: {}",
                self.config.ip_config.ip_address
            );
        } else {
            panic!("PNet not yet initialised, interface is None");
        }
    }

    pub fn handle_periodic(&mut self, current_timestamp: usize) {
        // defmt::debug!("Handling incoming packets");
        let _ = self.handle_incoming_packet(current_timestamp);

        self.send_queued_packets(current_timestamp);
    }

    pub fn handle_incoming_packet(&mut self, current_timestamp: usize) -> Result<(), Error> {
        let mut packet_buf = [0; 1024];

        match self.dma().recv_next(None) {
            Ok(p) => {
                let packet_len = p.len();
                packet_buf[0..packet_len].copy_from_slice(&*p)
            }
            Err(_) => return Ok(()),
        };

        let frame_in =
            EthernetFrame::new_checked(&packet_buf).map_err(|e| Error::EthernetError(e))?;

        if frame_in.dst_address().0 != self.config.ip_config.mac_address.0
            && frame_in.dst_address().0 != DCP_MAC_HELLO_ADDRESS
        {
            // defmt::debug!(
            //     "Packet was not meant for us, dst_address: {}",
            //     frame_in.dst_address()
            // );
            return Ok(());
        }

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
            FrameId::Other => defmt::debug!("Packet Frame ID is not DCP"),
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
                break;
            }
        }
    }

    pub fn send_queued_packets(&mut self, current_timestamp: usize) {
        for i in 0..self.outgoing_packets.len() {
            if let Some(p) = self.outgoing_packets[i] {
                if current_timestamp >= p.send_at {
                    match self
                        .dma()
                        .send(p.length, None, |buf| buf.copy_from_slice(&p.data))
                    {
                        Ok(_) => {
                            defmt::debug!("Successfully sent out packet");
                            self.outgoing_packets[i] = None
                        }
                        Err(_) => defmt::error!("Failed sending packet"),
                    }
                }
            }
        }
    }
}
