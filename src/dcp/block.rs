use byteorder::{ByteOrder, NetworkEndian};
use num_enum::TryFromPrimitive;
use smoltcp::wire::{EthernetAddress, Ipv4Address};

use crate::dcp::block_options::*;
use crate::dcp::error::ParseDCPBlockError;
use crate::field::{Field, Rest, SmallField};

const MAX_DEVICE_VENDOR_LENGTH: usize = 255;
const MAX_NAME_OF_STATION_LENGTH: usize = 240;

pub struct DCPBlockFrame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> DCPBlockFrame<T> {
    const OPTION: SmallField = 0;
    const SUBOPTION: SmallField = 1;
    const BLOCK_LENGTH: Field = 2..4;
    const BLOCK_INFO: Field = 4..6;
    const PAYLOAD: Rest = 6..;

    pub fn new_unchecked(buffer: T) -> DCPBlockFrame<T> {
        DCPBlockFrame { buffer }
    }

    pub fn option(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[Self::OPTION]
    }

    pub fn suboption(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[Self::SUBOPTION]
    }

    pub fn block_length(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[Self::BLOCK_LENGTH])
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();
        &data[Self::PAYLOAD]
    }
}

pub struct DCPBlock {
    block: Block,
    block_length: u16,
}

impl DCPBlock {
    pub fn parse_block(buffer: &[u8]) -> Result<Self, ParseDCPBlockError> {
        let frame = DCPBlockFrame::new_unchecked(buffer);

        let option = BlockOption::try_from(frame.option())
            .map_err(|_| ParseDCPBlockError::InvalidBlockOption)?;

        let suboption = frame.suboption();
        let block_length = frame.block_length();
        let payload = frame.payload();
        let payload_length = (block_length - 2) as usize;

        let block = match option {
            BlockOption::IP => {
                let ip_suboption = IpSuboption::try_from_primitive(suboption)
                    .map_err(|_| ParseDCPBlockError::InvalidIPSuboption)?;

                let ip_block = match ip_suboption {
                    IpSuboption::MacAddress => IpBlock::MacAddress(MacAddress::new(payload)),
                    IpSuboption::IpParameter => IpBlock::IpParameter(IpParameter::new(payload)),
                    IpSuboption::FullIpSuite => IpBlock::FullIpSuite(FullIpSuite::new(payload)),
                };

                Block::Ip(ip_block)
            }
            BlockOption::DeviceProperties => {
                let device_prop_suboption = DevicePropertiesSuboption::try_from(suboption)
                    .map_err(|_| ParseDCPBlockError::InvalidDevicePropertySuboption)?;

                let device_block = match device_prop_suboption {
                    DevicePropertiesSuboption::DeviceVendor => {
                        DeviceProperties::DeviceVendor(DeviceVendor::new(payload, payload_length))
                    }
                    DevicePropertiesSuboption::NameOfStation => {
                        DeviceProperties::NameOfStation(NameOfStation::new(payload, payload_length))
                    }
                    DevicePropertiesSuboption::DeviceID => todo!(),
                    DevicePropertiesSuboption::DeviceRole => todo!(),
                    DevicePropertiesSuboption::DeviceOptions => todo!(),
                    DevicePropertiesSuboption::AliasName => todo!(),
                    DevicePropertiesSuboption::DeviceInstance => todo!(),
                    DevicePropertiesSuboption::OEMDeviceID => todo!(),
                    DevicePropertiesSuboption::StandardGateway => todo!(),
                    DevicePropertiesSuboption::RSIProperties => todo!(),
                };

                Block::DeviceProperties(device_block);

                todo!()
            }
            //BlockOption::DHCP => BlockSuboption::DHCPSuboption(
            //     DHCPSuboption::try_from(suboption)
            //         .map_err(|_| ParseDCPBlockError::InvalidDHCPPropertySuboption)?,
            // ),
            // BlockOption::Control => BlockSuboption::ControlSuboption(
            //     ControlSuboption::try_from(suboption)
            //         .map_err(|_| ParseDCPBlockError::InvalidControlSuboption)?,
            // ),
            // BlockOption::DeviceInitiative => BlockSuboption::DeviceInitiativeSuboption(
            //     DeviceInitiativeSuboption::try_from(suboption)
            //         .map_err(|_| ParseDCPBlockError::InvalidDeviceInitiativeSuboption)?,
            // ),
            // BlockOption::NMEDomain => BlockSuboption::NMEDomainSuboption(
            //     NMEDomainSuboption::try_from(suboption)
            //         .map_err(|_| ParseDCPBlockError::InvalidNMEDomainSuboption)?,
            // ),
            // BlockOption::ManufacturerSpecific => BlockSuboption::ManufacturerSpecific(
            //     ManufacturerSpecificSuboption::try_from(suboption)
            //         .map_err(|_| ParseDCPBlockError::InvalidManufacturerSpecificSuboption)?,
            // ),
            // BlockOption::All => BlockSuboption::AllSuboption(
            //     AllSuboption::try_from(suboption)
            //         .map_err(|_| ParseDCPBlockError::InvalidAllSuboption)?,
            // ),
            _ => todo!(),
        };

        Ok(Self {
            block,
            block_length,
        })
    }
}

pub enum Block {
    Ip(IpBlock),
    DeviceProperties(DeviceProperties),
}

pub enum IpBlock {
    MacAddress(MacAddress),
    IpParameter(IpParameter),
    FullIpSuite(FullIpSuite),
}

pub struct MacAddress {
    address: EthernetAddress,
}

impl MacAddress {
    const ADDRESS: Field = 0..6;

    fn new(buffer: &[u8]) -> Self {
        Self {
            address: EthernetAddress::from_bytes(&buffer[Self::ADDRESS]),
        }
    }
}

pub struct IpParameter {
    pub ip_address: Ipv4Address,
    pub subnet_mask: Ipv4Address,
    pub gateway: Ipv4Address,
}

impl IpParameter {
    const IP_ADDRESS: Field = 0..4;
    const SUBNET_MASK: Field = 4..8;
    const GATEWAY: Field = 8..12;

    fn new(buffer: &[u8]) -> Self {
        Self {
            ip_address: Ipv4Address::from_bytes(&buffer[Self::IP_ADDRESS]),
            subnet_mask: Ipv4Address::from_bytes(&buffer[Self::SUBNET_MASK]),
            gateway: Ipv4Address::from_bytes(&buffer[Self::GATEWAY]),
        }
    }
}

pub struct FullIpSuite {
    pub ip_address: Ipv4Address,
    pub subnet_mask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub dns: Ipv4Address,
}

impl FullIpSuite {
    const IP_ADDRESS: Field = 0..4;
    const SUBNET_MASK: Field = 4..8;
    const GATEWAY: Field = 8..12;
    const DNS: Field = 12..16;

    fn new(buffer: &[u8]) -> Self {
        Self {
            ip_address: Ipv4Address::from_bytes(&buffer[Self::IP_ADDRESS]),
            subnet_mask: Ipv4Address::from_bytes(&buffer[Self::SUBNET_MASK]),
            gateway: Ipv4Address::from_bytes(&buffer[Self::GATEWAY]),
            dns: Ipv4Address::from_bytes(&buffer[Self::DNS]),
        }
    }
}

pub enum DeviceProperties {
    DeviceVendor(DeviceVendor),
    NameOfStation(NameOfStation),
    DeviceId(DeviceId),
    DeviceRole(DeviceRole),
    DeviceOptions,
    AliasName,
    DeviceInstance(DeviceInstance),
    OemDeviceId,
    StandardGateway,
    RsiProperties,
}

pub struct DeviceVendor([char; 255]);

impl DeviceVendor {
    pub fn new(buffer: &[u8], data_size: usize) -> Self {
        let mut device_vendor = [0 as char; MAX_DEVICE_VENDOR_LENGTH];
        let slice = &buffer[0..data_size];

        for i in 0..data_size {
            device_vendor[i] = slice[i] as char;
        }

        Self(device_vendor)
    }
}

pub struct NameOfStation([char; MAX_NAME_OF_STATION_LENGTH]);

impl NameOfStation {
    pub fn new(buffer: &[u8], data_size: usize) -> Self {
        let mut name_of_station = [0 as char; MAX_NAME_OF_STATION_LENGTH];
        let slice = &buffer[0..data_size];

        for i in 0..data_size {
            name_of_station[i] = slice[i] as char;
        }

        Self(name_of_station)
    }
}

pub struct DeviceId {
    pub vendor_id: u16,
    pub device_id: u16,
}

pub struct DeviceInstance {
    pub high: u8,
    pub low: u8,
}
