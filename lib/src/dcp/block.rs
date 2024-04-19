use byteorder::{ByteOrder, NetworkEndian};
use num_enum::TryFromPrimitive;
use smoltcp::wire::{EthernetAddress, Ipv4Address};
use zerocopy::AsBytes;

use crate::dcp::block_options::*;
use crate::dcp::error::ParseDCPBlockError;
use crate::field::{Field, Rest, SmallField};

pub const MAX_DEVICE_VENDOR_LENGTH: usize = 255;
pub const MAX_NAME_OF_STATION_LENGTH: usize = 240;

const OPTION_FIELD: SmallField = 0;
const SUBOPTION_FIELD: SmallField = 1;
const BLOCK_LENGTH_FIELD: Field = 2..4;
const BLOCK_INFO_FIELD: Field = 4..6;
const PAYLOAD_FIELD: Rest = 6..;

pub struct DCPBlockFrame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> DCPBlockFrame<T> {
    pub fn new_unchecked(buffer: T) -> DCPBlockFrame<T> {
        DCPBlockFrame { buffer }
    }

    pub fn option(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[OPTION_FIELD]
    }

    pub fn suboption(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[SUBOPTION_FIELD]
    }

    pub fn block_length(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[BLOCK_LENGTH_FIELD])
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();
        &data[PAYLOAD_FIELD]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DCPBlock {
    pub block: Block,
    pub block_length: u16,
}

impl DCPBlock {
    pub fn new(block: Block) -> Self {
        // let slice: &[u8] = bytemuck::bytes_of(&block);

        todo!()
    }

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
                    DevicePropertiesSuboption::DeviceVendor => DevicePropertiesBlock::DeviceVendor(
                        DeviceVendor::parse_bytes(payload, payload_length),
                    ),
                    DevicePropertiesSuboption::NameOfStation => {
                        DevicePropertiesBlock::NameOfStation(NameOfStation::parse_bytes(
                            payload,
                            payload_length,
                        ))
                    }
                    DevicePropertiesSuboption::DeviceID => {
                        DevicePropertiesBlock::DeviceId(DeviceId::parse_bytes(payload))
                    }
                    DevicePropertiesSuboption::DeviceRole => DevicePropertiesBlock::DeviceRole(
                        DeviceRole::try_from_primitive(payload[0])
                            .map_err(|_| ParseDCPBlockError::InvalidDeviceRole)?,
                    ),
                    DevicePropertiesSuboption::DeviceOptions => {
                        DevicePropertiesBlock::DeviceOptions
                    }
                    DevicePropertiesSuboption::AliasName => DevicePropertiesBlock::AliasName,
                    DevicePropertiesSuboption::DeviceInstance => {
                        DevicePropertiesBlock::DeviceInstance(DeviceInstance::parse_bytes(payload))
                    }
                    DevicePropertiesSuboption::OEMDeviceID => DevicePropertiesBlock::OemDeviceId,
                    DevicePropertiesSuboption::StandardGateway => {
                        DevicePropertiesBlock::StandardGateway
                    }
                    DevicePropertiesSuboption::RSIProperties => {
                        DevicePropertiesBlock::RsiProperties
                    }
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

    pub fn encode_into(&self, buffer: &mut [u8]) {
        // match self.block {
        //     // Block::Ip(ip) => ip.encode_into(&mut buf),
        //     // Block::DeviceProperties(dp) => dp.encode_into(but),
        //     // Block::All => {
        //     //     todo!()
        //     // }
        // }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Block {
    Ip(IpBlock),
    DeviceProperties(DevicePropertiesBlock),
    All,
}

impl Block {
    fn encode_into(&self, buffer: &mut [u8]) {
        match self {
            Block::Ip(ip) => {
                buffer[OPTION_FIELD] = BlockOption::IP.into();
                ip.encode_into(buffer);
            }
            Block::DeviceProperties(dp) => {
                buffer[OPTION_FIELD] = BlockOption::DeviceProperties.into();
                dp.encode_into(buffer);
            }
            Block::All => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IpBlock {
    MacAddress(MacAddress),
    IpParameter(IpParameter),
    FullIpSuite(FullIpSuite),
}

impl IpBlock {
    fn encode_into(&self, buffer: &mut [u8]) {
        NetworkEndian::write_u16(&mut buffer[BLOCK_INFO_FIELD], 0);

        match self {
            IpBlock::MacAddress(mac) => {
                buffer[SUBOPTION_FIELD] = IpSuboption::MacAddress.into();
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 8);
                mac.encode_into(buffer);
            }
            IpBlock::IpParameter(ip) => {
                buffer[SUBOPTION_FIELD] = IpSuboption::IpParameter.into();
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 14);
                ip.encode_into(buffer);
            }
            IpBlock::FullIpSuite(suite) => {
                buffer[SUBOPTION_FIELD] = IpSuboption::FullIpSuite.into();
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 20);
                suite.encode_into(buffer);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacAddress {
    pub address: EthernetAddress,
}

impl MacAddress {
    const ADDRESS: Field = 0..6;

    fn new(buffer: &[u8]) -> Self {
        Self {
            address: EthernetAddress::from_bytes(&buffer[Self::ADDRESS]),
        }
    }

    fn encode_into(&self, buffer: &mut [u8]) {
        buffer[Self::ADDRESS].clone_from_slice(self.address.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

    fn encode_into(&self, buffer: &mut [u8]) {
        buffer[Self::IP_ADDRESS].clone_from_slice(self.ip_address.as_bytes());
        buffer[Self::SUBNET_MASK].clone_from_slice(self.subnet_mask.as_bytes());
        buffer[Self::GATEWAY].clone_from_slice(self.gateway.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

    fn encode_into(&self, buffer: &mut [u8]) {
        buffer[Self::IP_ADDRESS].clone_from_slice(self.ip_address.as_bytes());
        buffer[Self::SUBNET_MASK].clone_from_slice(self.subnet_mask.as_bytes());
        buffer[Self::GATEWAY].clone_from_slice(self.gateway.as_bytes());
        buffer[Self::DNS].clone_from_slice(self.dns.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevicePropertiesBlock {
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

impl DevicePropertiesBlock {
    fn encode_into(&self, buffer: &mut [u8]) {
        NetworkEndian::write_u16(&mut buffer[BLOCK_INFO_FIELD], 0);

        match self {
            DevicePropertiesBlock::DeviceVendor(dv) => todo!(),
            DevicePropertiesBlock::NameOfStation(_) => todo!(),
            DevicePropertiesBlock::DeviceId(_) => todo!(),
            DevicePropertiesBlock::DeviceRole(_) => todo!(),
            DevicePropertiesBlock::DeviceOptions => todo!(),
            DevicePropertiesBlock::AliasName => todo!(),
            DevicePropertiesBlock::DeviceInstance(_) => todo!(),
            DevicePropertiesBlock::OemDeviceId => todo!(),
            DevicePropertiesBlock::StandardGateway => todo!(),
            DevicePropertiesBlock::RsiProperties => todo!(),
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        match self {
            DevicePropertiesBlock::DeviceVendor(dev) => dev.as_bytes(),
            DevicePropertiesBlock::NameOfStation(nos) => nos.as_bytes(),
            DevicePropertiesBlock::DeviceId(did) => did.as_bytes(),
            DevicePropertiesBlock::DeviceRole(drl) => todo!(),
            DevicePropertiesBlock::DeviceOptions => todo!(),
            DevicePropertiesBlock::AliasName => todo!(),
            DevicePropertiesBlock::DeviceInstance(din) => din.as_bytes(),
            DevicePropertiesBlock::OemDeviceId => todo!(),
            DevicePropertiesBlock::StandardGateway => todo!(),
            DevicePropertiesBlock::RsiProperties => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceVendor {
    pub vendor: [u8; MAX_DEVICE_VENDOR_LENGTH],
    pub length: usize,
}

impl DeviceVendor {
    pub fn parse_bytes(buffer: &[u8], data_size: usize) -> Self {
        let mut device_vendor = [0; MAX_DEVICE_VENDOR_LENGTH];

        for i in 0..data_size {
            device_vendor[i] = buffer[i];
        }

        Self {
            vendor: device_vendor,
            length: data_size,
        }
    }

    pub fn from_str(str: &str) -> Self {
        let mut device_vendor = [0; MAX_DEVICE_VENDOR_LENGTH];
        let str_bytes = str.as_bytes();

        for i in 0..str.len() {
            device_vendor[i] = str_bytes[i] as u8;
        }

        Self {
            vendor: device_vendor,
            length: str_bytes.len(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.vendor[0..self.length]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NameOfStation {
    pub name: [u8; MAX_NAME_OF_STATION_LENGTH],
    pub length: usize,
}

impl NameOfStation {
    pub fn parse_bytes(buffer: &[u8], data_size: usize) -> Self {
        let mut name_of_station = [0; MAX_NAME_OF_STATION_LENGTH];

        for i in 0..data_size {
            name_of_station[i] = buffer[i];
        }

        Self {
            name: name_of_station,
            length: data_size,
        }
    }

    pub fn from_str(str: &str) -> Self {
        let mut name_of_station = [0; MAX_NAME_OF_STATION_LENGTH];
        let name_of_station_bytes = str.as_bytes();

        for i in 0..name_of_station_bytes.len() {
            name_of_station[i] = name_of_station_bytes[i] as u8
        }

        Self {
            name: name_of_station,
            length: name_of_station_bytes.len(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.name[0..self.length]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct DeviceId {
    pub vendor_id: u16,
    pub device_id: u16,
}

impl DeviceId {
    pub fn parse_bytes(buffer: &[u8]) -> Self {
        let vendor_id = NetworkEndian::read_u16(&buffer[0..2]);
        let device_id = NetworkEndian::read_u16(&buffer[2..4]);

        Self {
            vendor_id,
            device_id,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct DeviceInstance {
    pub high: u8,
    pub low: u8,
}

impl DeviceInstance {
    pub fn parse_bytes(buffer: &[u8]) -> Self {
        let high = buffer[0];
        let low = buffer[1];

        Self { high, low }
    }

    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_vendor_as_bytes() {
        let device_vendor = DeviceVendor::from_str("device vendor 123");
        let bytes = device_vendor.as_bytes();

        assert_eq!(
            bytes,
            [100, 101, 118, 105, 99, 101, 32, 118, 101, 110, 100, 111, 114, 32, 49, 50, 51]
        );
    }

    #[test]
    fn test_device_instance_as_bytes() {
        let device_instance = DeviceInstance { high: 123, low: 42 };
        let bytes = device_instance.as_bytes();

        assert_eq!(bytes, [123, 42])
    }
}
