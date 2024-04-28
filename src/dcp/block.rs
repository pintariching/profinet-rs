use core::mem;

use byteorder::{ByteOrder, NetworkEndian};
use num_enum::TryFromPrimitive;
use smoltcp::wire::{EthernetAddress, Ipv4Address};

use crate::dcp::block_options::*;
use crate::dcp::error::ParseDcpBlockError;
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
pub struct DcpBlock {
    pub block: Block,
    pub block_length: u16,
}

impl DcpBlock {
    pub fn new(block: Block) -> Self {
        let mut block_length = match block {
            Block::Ip(ip) => ip.block_length(),
            Block::DeviceProperties(dp) => dp.block_length(),
            Block::All => 0,
        };

        // Account for block header
        block_length += 4;

        if block_length % 2 == 1 {
            block_length += 1
        }

        Self {
            block,
            block_length,
        }
    }

    pub fn parse_block(buffer: &[u8]) -> Result<Self, ParseDcpBlockError> {
        let frame = DCPBlockFrame::new_unchecked(buffer);

        let option = BlockOption::try_from(frame.option())
            .map_err(|_| ParseDcpBlockError::InvalidBlockOption)?;

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
                    .map_err(|_| ParseDcpBlockError::InvalidIPSuboption)?;

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
                        .map_err(|_| ParseDcpBlockError::InvalidDevicePropertySuboption)?;

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
                    DevicePropertiesSuboption::DeviceId => {
                        DevicePropertiesBlock::DeviceId(DeviceId::parse_bytes(payload))
                    }
                    DevicePropertiesSuboption::DeviceRole => DevicePropertiesBlock::DeviceRole(
                        DeviceRole::try_from_primitive(payload[0])
                            .map_err(|_| ParseDcpBlockError::InvalidDeviceRole)?,
                    ),
                    DevicePropertiesSuboption::DeviceOptions => {
                        DevicePropertiesBlock::DeviceOptions
                    }
                    DevicePropertiesSuboption::AliasName => DevicePropertiesBlock::AliasName,
                    DevicePropertiesSuboption::DeviceInstance => {
                        DevicePropertiesBlock::DeviceInstance(DeviceInstance::parse_bytes(payload))
                    }
                    DevicePropertiesSuboption::OemDeviceId => DevicePropertiesBlock::OemDeviceId,
                    DevicePropertiesSuboption::StandardGateway => {
                        DevicePropertiesBlock::StandardGateway
                    }
                    DevicePropertiesSuboption::RsiProperties => {
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
        self.block.encode_into(buffer);
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
                buffer[OPTION_FIELD] = BlockOption::IP as u8;
                ip.encode_into(buffer);
            }
            Block::DeviceProperties(dp) => {
                buffer[OPTION_FIELD] = BlockOption::DeviceProperties as u8;
                dp.encode_into(buffer);
            }
            Block::All => {
                buffer[OPTION_FIELD] = BlockOption::All as u8;
                buffer[SUBOPTION_FIELD] = BlockOption::All as u8;
            }
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
        #[cfg(test)]
        println!("buffer: {:?}", buffer);

        NetworkEndian::write_u16(&mut buffer[BLOCK_INFO_FIELD], 1);

        match self {
            IpBlock::MacAddress(mac) => {
                buffer[SUBOPTION_FIELD] = IpSuboption::MacAddress as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 8);
                mac.encode_into(&mut buffer[PAYLOAD_FIELD]);
            }
            IpBlock::IpParameter(ip) => {
                buffer[SUBOPTION_FIELD] = IpSuboption::IpParameter as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 14);
                ip.encode_into(&mut buffer[PAYLOAD_FIELD]);
            }
            IpBlock::FullIpSuite(suite) => {
                buffer[SUBOPTION_FIELD] = IpSuboption::FullIpSuite as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 20);
                suite.encode_into(&mut buffer[PAYLOAD_FIELD]);
            }
        }
    }

    fn block_length(&self) -> u16 {
        match self {
            IpBlock::MacAddress(mac) => mac.block_length(),
            IpBlock::IpParameter(ip) => ip.block_length(),
            IpBlock::FullIpSuite(suite) => suite.block_length(),
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

    fn block_length(&self) -> u16 {
        Self::ADDRESS.end as u16 + 2
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

    fn block_length(&self) -> u16 {
        Self::GATEWAY.end as u16 + 2
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

    fn block_length(&self) -> u16 {
        Self::DNS.end as u16 + 2
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
            DevicePropertiesBlock::DeviceVendor(dv) => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::DeviceVendor as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], dv.length as u16 + 2);
                dv.encode_into(&mut buffer[PAYLOAD_FIELD]);
            }
            DevicePropertiesBlock::NameOfStation(nos) => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::NameOfStation as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], nos.length as u16 + 2);
                nos.encode_into(&mut buffer[PAYLOAD_FIELD]);
            }
            DevicePropertiesBlock::DeviceId(id) => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::DeviceId as u8;
                NetworkEndian::write_u16(
                    &mut buffer[BLOCK_LENGTH_FIELD],
                    mem::size_of::<DeviceId>() as u16 + 2,
                );
                id.encode_into(&mut buffer[PAYLOAD_FIELD]);
            }
            DevicePropertiesBlock::DeviceRole(dr) => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::DeviceRole as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 4);
                buffer[PAYLOAD_FIELD.start] = *dr as u8;
            }
            DevicePropertiesBlock::DeviceOptions => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::DeviceOptions as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 4);
                buffer[PAYLOAD_FIELD.start] = 2;
                buffer[PAYLOAD_FIELD.start + 1] = 7;
            }
            DevicePropertiesBlock::AliasName => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::AliasName as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 3);
                buffer[PAYLOAD_FIELD.start] = 0;
            }
            DevicePropertiesBlock::DeviceInstance(di) => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::DeviceInstance as u8;
                NetworkEndian::write_u16(
                    &mut buffer[BLOCK_LENGTH_FIELD],
                    mem::size_of::<DeviceInstance>() as u16 + 2,
                );
                di.encode_into(&mut buffer[PAYLOAD_FIELD]);
            }
            DevicePropertiesBlock::OemDeviceId => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::OemDeviceId as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 3);
                buffer[PAYLOAD_FIELD.start] = 0;
            }
            DevicePropertiesBlock::StandardGateway => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::StandardGateway as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 3);
                buffer[PAYLOAD_FIELD.start] = 0;
            }
            DevicePropertiesBlock::RsiProperties => {
                buffer[SUBOPTION_FIELD] = DevicePropertiesSuboption::RsiProperties as u8;
                NetworkEndian::write_u16(&mut buffer[BLOCK_LENGTH_FIELD], 3);
                buffer[PAYLOAD_FIELD.start] = 0;
            }
        }
    }

    fn block_length(&self) -> u16 {
        match self {
            DevicePropertiesBlock::DeviceVendor(dv) => dv.block_length(),
            DevicePropertiesBlock::NameOfStation(nos) => nos.block_length(),
            DevicePropertiesBlock::DeviceId(id) => id.block_length(),
            DevicePropertiesBlock::DeviceInstance(di) => di.block_length(),
            DevicePropertiesBlock::DeviceOptions => 4,
            _ => mem::size_of::<u8>() as u16 + 2,
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

    fn encode_into(&self, buffer: &mut [u8]) {
        buffer[..self.length].copy_from_slice(&self.vendor[..self.length]);
    }

    fn block_length(&self) -> u16 {
        self.length as u16 + 2
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

    fn encode_into(&self, buffer: &mut [u8]) {
        buffer[..self.length].copy_from_slice(&self.name[..self.length]);
    }

    fn block_length(&self) -> u16 {
        self.length as u16 + 2
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

    fn encode_into(&self, buffer: &mut [u8]) {
        NetworkEndian::write_u16(&mut buffer[0..2], self.vendor_id);
        NetworkEndian::write_u16(&mut buffer[2..4], self.device_id);
    }

    fn block_length(&self) -> u16 {
        mem::size_of::<Self>() as u16 + 2
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

    fn encode_into(&self, buffer: &mut [u8]) {
        buffer[0] = self.high;
        buffer[1] = self.low;
    }

    fn block_length(&self) -> u16 {
        mem::size_of::<Self>() as u16 + 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_vendor_as_bytes() {
        let device_vendor = DeviceVendor::from_str("device vendor 123");
        assert_eq!(device_vendor.length, 17);

        let mut buffer = [0; 17];
        device_vendor.encode_into(&mut buffer);

        assert_eq!(
            buffer,
            [100, 101, 118, 105, 99, 101, 32, 118, 101, 110, 100, 111, 114, 32, 49, 50, 51]
        );
    }

    #[test]
    fn test_device_instance_as_bytes() {
        let device_instance = DeviceInstance { high: 123, low: 42 };
        let mut buffer = [0; mem::size_of::<DeviceInstance>()];

        device_instance.encode_into(&mut buffer);

        assert_eq!(buffer, [123, 42])
    }
}
