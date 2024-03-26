use smoltcp::wire::Ipv4Address;

#[repr(u8)]
pub enum DcpOption<'a> {
    Reserved = 0,
    Ip(IpSuboptions),
    DeviceProperties(DevicePropertiesSuboptions<'a>),
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

pub struct DeviceVendorValue<'a>(&'a str);
pub struct NameOfStation<'a>(&'a str);

pub struct DeviceId {
    pub vendor_id: u16,
    pub device_id: u16,
}

pub struct DeviceRole(u8);

pub struct DeviceInstance {
    pub high: u8,
    pub low: u8,
}

#[repr(u8)]
pub enum DevicePropertiesSuboptions<'a> {
    DeviceVendorValue(DeviceVendorValue<'a>) = 1,
    NameOfStation(NameOfStation<'a>),
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
