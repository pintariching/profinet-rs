use num_enum::TryFromPrimitive;

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum BlockOption {
    IP = 1,
    DeviceProperties = 2,
    DHCP = 3,
    Control = 5,
    DeviceInitiative = 6,
    NMEDomain = 7,
    #[num_enum(alternatives = [0x81..0xfe])] // 0x80..0xfe
    ManufacturerSpecific = 0x80,
    All = 255,
}
#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum IpSuboption {
    MacAddress = 1,
    IpParameter = 2,
    FullIpSuite = 3,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum DevicePropertiesSuboption {
    DeviceVendor = 1,
    NameOfStation = 2,
    DeviceID = 3,
    DeviceRole = 4,
    DeviceOptions = 5,
    AliasName = 6,
    DeviceInstance = 7,
    OEMDeviceID = 8,
    StandardGateway = 9,
    RSIProperties = 10,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum ControlSuboption {
    Start = 1,
    Stop = 2,
    Signal = 3,
    Response = 4,
    FactoryReset = 5,
    ResetToFactory = 6,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum DeviceInitiativeSuboption {
    DeviceInitiative = 1,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum NMEDomainSuboption {
    NMEDomain = 1,
    NMEPrio = 2,
    NMEParameterUUID = 3,
    NMEName = 4,
    CIMInterface = 5,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum AllSuboption {
    All = 0xff,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum ManufacturerSpecificSuboption {
    #[num_enum(alternatives = [0x01..0xff])]
    ManufacturerSpecific,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum DHCPSuboption {
    HostName = 12,
    VendorSpecific = 43,
    ServerIdentifier = 54,
    ParameterRequestList = 55,
    ClassIdentifier = 60,
    DHCPClientIdentifier = 61,
    FQDN = 81,
    UUIDBasedClient = 97,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum DeviceRole {
    IODevice,
    IOController,
    IOMultidevice,
    IOSupervisor,
}

// #[repr(u8)]
// pub enum DevicePropertiesSuboptions {
//     DeviceVendorValue(DeviceVendor) = 1,
//     NameOfStation(NameOfStation),
//     DeviceID(DeviceId),
//     DeviceRole(DeviceRole),
//     Options,
//     Alias,
//     Instance(DeviceInstance),
//     OemId,
//     Gateway,
// }

#[derive(TryFromPrimitive)]
#[repr(u8)]
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

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum ControlSuboptions {
    Start = 1,
    Stop,
    Signal,
    Response,
    FactoryReset,
    ResetToFactory,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum DeviceInitiativeSuboptions {
    InitiativeSupport = 1,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum AllSuboptions {
    All = 255,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum BlockErrorValues {
    NoError,
    OptionNotSupported,
    OptionNotSet,
    ResourceError,
    SetNotPossible,
}
