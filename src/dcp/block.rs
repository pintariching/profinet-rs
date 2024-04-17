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
    // const BLOCK_INFO: Field = 4..6;
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

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
pub struct DCPBlock {
    pub block: Block,
    pub block_length: u16,
}

impl DCPBlock {
    pub fn parse_block(buffer: &[u8]) -> Result<Self, ParseDCPBlockError> {
        let frame = DCPBlockFrame::new_unchecked(buffer);

        let option = BlockOption::try_from(frame.option())
            .map_err(|_| ParseDCPBlockError::InvalidBlockOption)?;

        let suboption = frame.suboption();
        let block_length = frame.block_length();

        if (option == BlockOption::All) && AllSuboption::try_from_primitive(suboption).is_ok() {
            return Ok(Self {
                block: Block::All,
                block_length,
            });
        }

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
                let device_prop_suboption =
                    DevicePropertiesSuboption::try_from_primitive(suboption)
                        .map_err(|_| ParseDCPBlockError::InvalidDevicePropertySuboption)?;

                let device_block = match device_prop_suboption {
                    DevicePropertiesSuboption::DeviceVendor => {
                        DeviceProperties::DeviceVendor(DeviceVendor::new(payload, payload_length))
                    }
                    DevicePropertiesSuboption::NameOfStation => {
                        DeviceProperties::NameOfStation(NameOfStation::new(payload, payload_length))
                    }
                    DevicePropertiesSuboption::DeviceID => {
                        DeviceProperties::DeviceId(DeviceId::new(payload))
                    }
                    DevicePropertiesSuboption::DeviceRole => DeviceProperties::DeviceRole(
                        DeviceRole::try_from_primitive(payload[0])
                            .map_err(|_| ParseDCPBlockError::InvalidDeviceRole)?,
                    ),
                    DevicePropertiesSuboption::DeviceOptions => DeviceProperties::DeviceOptions,
                    DevicePropertiesSuboption::AliasName => DeviceProperties::AliasName,
                    DevicePropertiesSuboption::DeviceInstance => {
                        DeviceProperties::DeviceInstance(DeviceInstance::new(payload))
                    }
                    DevicePropertiesSuboption::OEMDeviceID => DeviceProperties::OemDeviceId,
                    DevicePropertiesSuboption::StandardGateway => DeviceProperties::StandardGateway,
                    DevicePropertiesSuboption::RSIProperties => DeviceProperties::RsiProperties,
                };

                Block::DeviceProperties(device_block)
            }
            _ => todo!(),
        };

        Ok(Self {
            block,
            block_length,
        })
    }
}

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
pub enum Block {
    Ip(IpBlock),
    DeviceProperties(DeviceProperties),
    All,
}

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
pub enum IpBlock {
    MacAddress(MacAddress),
    IpParameter(IpParameter),
    FullIpSuite(FullIpSuite),
}

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
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

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
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

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
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

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
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

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
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

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
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

    pub fn from_str(str: &str) -> Self {
        let mut name_of_station = [0 as char; MAX_NAME_OF_STATION_LENGTH];

        for (i, c) in str.chars().enumerate() {
            name_of_station[i] = c
        }

        Self(name_of_station)
    }
}

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
pub struct DeviceId {
    pub vendor_id: u16,
    pub device_id: u16,
}

impl DeviceId {
    pub fn new(buffer: &[u8]) -> Self {
        let vendor_id = NetworkEndian::read_u16(&buffer[0..2]);
        let device_id = NetworkEndian::read_u16(&buffer[2..4]);

        Self {
            vendor_id,
            device_id,
        }
    }
}

#[cfg_attr(test, derive(Debug, Clone, PartialEq))]
pub struct DeviceInstance {
    pub high: u8,
    pub low: u8,
}

impl DeviceInstance {
    pub fn new(buffer: &[u8]) -> Self {
        let high = buffer[0];
        let low = buffer[1];

        Self { high, low }
    }
}
