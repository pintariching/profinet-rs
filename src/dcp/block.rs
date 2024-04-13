use smoltcp::wire::Ipv4Address;

const DEVICE_VENDOR_VALUE_LENGTH: usize = 255;
const NAME_OF_STATION_LENGTH: usize = 240;

mod block_header_field {
    use crate::field::*;

    pub const OPTION: SmallField = 0;
    pub const SUBOPTION: SmallField = 1;
    pub const BLOCK_LENGTH: Field = 2..4;
    pub const PAYLOAD: Rest = 4..;
}

pub const DCP_BLOCK_HEADER_LENGTH: usize = block_header_field::PAYLOAD.start;

pub struct DcpBlockFrame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> DcpBlockFrame<T> {
    pub fn new_unchecked(buffer: T) -> DcpBlockFrame<T> {
        DcpBlockFrame { buffer }
    }

    pub fn new_checked(buffer: T) -> Option<DcpBlockFrame<T>> {
        let header = Self::new_unchecked(buffer);
        match header.check_len() {
            true => Some(header),
            false => None,
        }
    }

    pub fn check_len(&self) -> bool {
        let len = self.buffer.as_ref().len();

        len > DCP_BLOCK_HEADER_LENGTH
    }

    pub fn option(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[block_header_field::OPTION]
    }
}

#[repr(u8)]
pub enum DcpOption {
    Reserved = 0,
    Ip(IpSuboptions),
    DeviceProperties(DevicePropertiesSuboptions),
    Dhcp(DhcpSuboptions),
    Reserved4,
    Control(ControlSuboptions),
    DeviceInitiative(DeviceInitiativeSuboptions),
    // Reserved 0x07 .. 0x7f
    // Manufacturer specific 0x80 .. 0xfe
    All(AllSuboptions) = 255,
}

pub struct IpParameter {
    pub ip_address: Ipv4Address,
    pub subnet_mask: Ipv4Address,
    pub standard_gateway: Ipv4Address,
}

#[repr(u8)]
pub enum IpSuboptions {
    Mac = 1,
    IpParameter(IpParameter),
    Suite,
}

pub struct DeviceVendor([char; DEVICE_VENDOR_VALUE_LENGTH]);
pub struct NameOfStation([char; NAME_OF_STATION_LENGTH]);

pub struct DeviceId {
    pub vendor_id: u16,
    pub device_id: u16,
}

pub enum DeviceRole {
    IODevice,
    IOController,
    IOMultidevice,
    IOSupervisor,
}

pub struct DeviceInstance {
    pub high: u8,
    pub low: u8,
}

#[repr(u8)]
pub enum DevicePropertiesSuboptions {
    DeviceVendorValue(DeviceVendor) = 1,
    NameOfStation(NameOfStation),
    DeviceID(DeviceId),
    DeviceRole(DeviceRole),
    Options,
    Alias,
    Instance(DeviceInstance),
    OemId,
    Gateway,
}

pub enum DhcpSuboptions {
    Hostname = 12,
    VendorSpecific = 43,
    ServerId = 54,
    ParReqList = 55,
    ClassId = 60,
    CliendId = 61,
    Fqdn = 81,
    UuidClientId = 97,
    Control = 255, // Defined as END in the DHCP spec
}

pub enum ControlSuboptions {
    Start = 1,
    Stop,
    Signal,
    Response,
    FactoryReset,
    ResetToFactory,
}

pub enum DeviceInitiativeSuboptions {
    InitiativeSupport = 1,
}

pub enum AllSuboptions {
    All = 255,
}

pub enum BlockErrorValues {
    NoError,
    OptionNotSupported,
    OptionNotSet,
    ResourceError,
    SetNotPossible,
}
