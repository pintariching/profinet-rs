#![cfg_attr(not(test), no_std)]

use fspm::{app::App, Config};
use scheduler::{Scheduler, Task, TaskCallback};
use smoltcp::{iface::SocketHandle, wire::EthernetAddress};
use stm32_eth::{mac::EthernetMAC, Parts};

mod cmdev;
mod cmrpc;
pub mod constants;
mod cpm;
// mod dcp;
mod error;
pub mod ethernet;
mod fspm;
pub mod scheduler;
pub mod types;
mod util;

mod field {
    pub type SmallField = usize;
    pub type Field = ::core::ops::Range<usize>;
    pub type Rest = ::core::ops::RangeFrom<usize>;
}

// pub use dcp::*;

#[derive(Clone, Copy)]
pub struct OutgoingPacket {
    pub data: [u8; 255],
    pub length: usize,
    pub send_at: usize,
}

pub struct PNet<'rx, 'tx, T: App + Copy, U: TaskCallback + Copy> {
    global_alarm_enable: bool,

    // CPM
    cpm_instance_count: u32,

    // PPM
    ppm_instance_count: u32,

    // DCP
    dcp_global_block_qualifier: u16,
    dcp_sam: EthernetAddress,
    dcp_delayed_response_waiting: bool,

    dcp_led_timeout: Task<U>,
    dcp_sam_timeout: Task<U>,
    dcp_identresp_timeout: Task<U>,

    // Scheduler
    scheduler: Scheduler<U>,

    // CMDEV
    cmdev_initialised: bool,
    // cmdev_device: pf_device_t,

    // CMNIA

    //    /** Reflects what is/should be stored in NVM */
    //    pf_cmina_dcp_ase_t cmina_nonvolatile_dcp_ase;

    //    /** Reflects current settings (possibly not yet committed) */
    //    pf_cmina_dcp_ase_t cmina_current_dcp_ase;

    //    os_mutex_t * cmina_mutex;
    //    pf_cmina_state_values_t cmina_state;
    //    uint8_t cmina_error_decode;
    //    uint8_t cmina_error_code_1;
    //    uint16_t cmina_hello_count;
    //    pf_scheduler_handle_t cmina_hello_timeout;

    // Scheduler
    fspm_default_config: Config<T>,
    fspm_user_config: Config<T>,

    //    bool cmina_commit_ip_suite;
    // config: Config,
    outgoing_packets: [Option<OutgoingPacket>; 8],
    ethernet_parts: Option<Parts<'rx, 'tx, EthernetMAC>>,
    tcp_handle: SocketHandle,
    udp_handle: SocketHandle,
}

impl<'rx, 'tx, T, U> PNet<'rx, 'tx, T, U>
where
    T: App + Copy,
    U: TaskCallback + Copy,
{
    pub fn init(&mut self, config: Config<T>) {
        config.init(self);

        self.cmdev_initialised = false;
    }
}

// impl<'rx, 'tx, T> PNet<'rx, 'tx, T>
// where
//     T: App,
// {
//     pub fn new(config: Config) -> Self {
//         let mut sockets = [SocketStorage::EMPTY];
//         let mut sockets = SocketSet::new(&mut sockets[..]);

//         let mut tcp_rx_buffer = [0; 1024];
//         let mut tcp_tx_buffer = [0; 1024];
//         let tcp_socket = TcpSocket::new(
//             TcpSocketBuffer::new(tcp_rx_buffer.as_mut_slice()),
//             TcpSocketBuffer::new(tcp_tx_buffer.as_mut_slice()),
//         );

//         let mut udp_rx_metadata_buf = [PacketMetadata::EMPTY; 4];
//         let mut udp_rx_data_buf = [0; 1024];
//         let mut udp_tx_metadata_buf = [PacketMetadata::EMPTY; 4];
//         let mut udp_tx_data_buf = [0; 1024];

//         let udp_rx_buffer = UdpPacketBuffer::new(
//             udp_rx_metadata_buf.as_mut_slice(),
//             udp_rx_data_buf.as_mut_slice(),
//         );
//         let upd_tx_buffer = UdpPacketBuffer::new(
//             udp_tx_metadata_buf.as_mut_slice(),
//             udp_tx_data_buf.as_mut_slice(),
//         );
//         let udp_socket = UdpSocket::new(udp_rx_buffer, upd_tx_buffer);

//         let tcp_handle = sockets.add(tcp_socket);
//         let udp_handle = sockets.add(udp_socket);

//         // Self {
//         //     config,
//         //     outgoing_packets: [None; 8],
//         //     ethernet_parts: None,
//         //     tcp_handle,
//         //     udp_handle,
//         //     global_alarm_enable: false,
//         // }

//         todo!()
//     }

//     pub fn init(
//         &mut self,
//         ethernet: PartsIn,
//         clocks: Clocks,
//         gpio: Gpio,
//         rx_ring: &'rx mut [RxRingEntry; 2],
//         tx_ring: &'tx mut [TxRingEntry; 2],
//     ) {
//         defmt::info!("Enabling ethernet...");

//         let eth_pins = setup_pins(gpio);

//         let mut parts = stm32_eth::new(
//             ethernet,
//             &mut rx_ring[..],
//             &mut tx_ring[..],
//             clocks,
//             eth_pins,
//         )
//         .unwrap();
//         parts.dma.enable_interrupt();

//         let config = smoltcp::iface::Config::new(self.config.ip_config.mac_address.into());
//         let iface = smoltcp::iface::Interface::new(config, &mut &mut parts.dma, Instant::ZERO);
//         self.config.interface = Some(iface);

//         self.update_interface();

//         defmt::info!(
//             "Enabled internet with IP and MAC: {}, {:x}",
//             self.config.ip_config.ip_address,
//             self.config.ip_config.mac_address
//         );

//         self.ethernet_parts = Some(parts);
//     }

//     pub fn dma(&mut self) -> &mut EthernetDMA<'rx, 'tx> {
//         if let Some(parts) = &mut self.ethernet_parts {
//             &mut parts.dma
//         } else {
//             panic!("PNet not initialised, ethernet_parts is None");
//         }
//     }

//     pub fn update_interface(&mut self) {
//         if let Some(iface) = &mut self.config.interface {
//             iface.update_ip_addrs(|addr| {
//                 addr.clear();
//                 addr.push(smoltcp::wire::IpCidr::Ipv4(Ipv4Cidr::new(
//                     self.config.ip_config.ip_address,
//                     24,
//                 )))
//                 .ok();
//             });

//             defmt::info!(
//                 "Update ethernet interface with IP: {}",
//                 self.config.ip_config.ip_address
//             );
//         } else {
//             panic!("PNet not yet initialised, interface is None");
//         }
//     }

//     pub fn handle_periodic(&mut self, current_timestamp: usize) {
//         // defmt::debug!("Handling incoming packets");
//         let _ = self.handle_incoming_packet(current_timestamp);

//         self.send_queued_packets(current_timestamp);
//     }

//     pub fn handle_incoming_packet(&mut self, current_timestamp: usize) -> Result<(), Error> {
//         let mut packet_buf = [0; 1024];

//         match self.dma().recv_next(None) {
//             Ok(p) => {
//                 let packet_len = p.len();
//                 packet_buf[0..packet_len].copy_from_slice(&*p)
//             }
//             Err(_) => return Ok(()),
//         };

//         let frame_in =
//             EthernetFrame::new_checked(&packet_buf).map_err(|e| Error::EthernetError(e))?;

//         if frame_in.dst_address().0 != self.config.ip_config.mac_address.0
//             && frame_in.dst_address().0 != DCP_MAC_HELLO_ADDRESS
//         {
//             // defmt::debug!(
//             //     "Packet was not meant for us, dst_address: {}",
//             //     frame_in.dst_address()
//             // );
//             return Ok(());
//         }

//         if !frame_in.is_profinet() {
//             defmt::debug!("Packet is not Profinet");
//             return Ok(());
//         }

//         let frame_id = frame_in.frame_id();

//         match frame_id {
//             FrameId::Dcp => {
//                 defmt::debug!("Packet Frame ID is DCP");
//                 Dcp::handle_frame(self, frame_in, current_timestamp);
//             }
//             FrameId::Other => defmt::debug!("Packet Frame ID is not DCP"),
//         }

//         Ok(())
//     }

//     pub fn queue_packet(&mut self, data: [u8; 255], send_at: usize) {
//         let packet_out = OutgoingPacket {
//             data,
//             length: data.len(),
//             send_at,
//         };

//         for i in 0..self.outgoing_packets.len() {
//             if let None = self.outgoing_packets[i] {
//                 self.outgoing_packets[i] = Some(packet_out);
//                 break;
//             }
//         }
//     }

//     pub fn send_queued_packets(&mut self, current_timestamp: usize) {
//         for i in 0..self.outgoing_packets.len() {
//             if let Some(p) = self.outgoing_packets[i] {
//                 if current_timestamp >= p.send_at {
//                     match self
//                         .dma()
//                         .send(p.length, None, |buf| buf.copy_from_slice(&p.data))
//                     {
//                         Ok(_) => {
//                             defmt::debug!("Successfully sent out packet");
//                             self.outgoing_packets[i] = None
//                         }
//                         Err(_) => defmt::error!("Failed sending packet"),
//                     }
//                 }
//             }
//         }
//     }
// }
