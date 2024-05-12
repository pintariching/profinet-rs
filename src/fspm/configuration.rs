use smoltcp::wire::Ipv4Address;

use crate::constants::{
    MAX_LOCATION_SIZE, MAX_ORDER_ID_LENGTH, MAX_PHYSICAL_PORTS, MAX_SERIAL_NUMBER_LENGTH,
};

#[derive(Clone)]
pub struct IM0 {
    pub vendor_id_hi: u8,
    pub vendor_id_lo: u8,

    pub order_id: [u8; MAX_ORDER_ID_LENGTH],
    pub serial_number: [u8; MAX_SERIAL_NUMBER_LENGTH],

    pub hw_rev: u16,
    pub sw_rev_prefx: char,
    pub sw_rev_functional_enhancment: u8,
    pub sw_rev_bug_fix: u8,
    pub sw_rev_internal_change: u8,
    pub revision_counter: u16,
    pub profile_id: u16,
    pub profile_specific_type: u16,

    pub version_major: u8,
    pub version_minor: u8,

    pub supported: u16,
}

#[derive(Clone)]
pub struct IM1 {
    pub tag_function: [u8; 32],
    pub tag_location: [u8; MAX_LOCATION_SIZE],
}

#[derive(Clone)]
pub struct IM2 {
    /// format "YYYY-MM-DD HH:MM"
    pub date: [u8; 16],
}

#[derive(Clone)]
pub struct IM3 {
    pub descriptor: [u8; 54],
}

#[derive(Clone)]
pub struct IM4 {
    pub signatire: [u8; 54],
}

#[derive(Clone)]
pub struct DeviceIdConfig {
    pub vendor_id_hi: u8,
    pub vendor_id_lo: u8,
    pub device_id_hi: u8,
    pub device_id_lo: u8,
}

#[derive(Clone)]
pub struct IpConfig {
    pub ip_address: Ipv4Address,
    pub subnet_mask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub enable_dhcp: bool,
}

#[derive(Clone)]
pub struct PortConfig {
    pub netif_name: &'static str,
    pub default_mau_type: u16,
}

#[derive(Clone)]
pub struct InterfaceConfig {
    pub network_interface_name: &'static str,
    pub ip_config: IpConfig,
    pub port_config: [PortConfig; MAX_PHYSICAL_PORTS],
}
